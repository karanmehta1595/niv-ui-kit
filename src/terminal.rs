//! Terminal: ek asli shell (PTY) ke saath chalne wala terminal widget.
//!
//! ```ignore
//! let mut term = Terminal::spawn(ctx.clone(), Some(project_dir), None)?;
//! // har frame:
//! term.pump(ui.ctx());   // shell ne jo likha wo screen mein lo
//! term.show(ui);         // bachi hui poori jagah mein dikhao
//! term.send_line("cargo run");
//! ```
//! Right-click menu (Copy, Paste, Select All, Clear, Split, Kill) apna hai; Split/Kill `take_req()` se milte hain.
//! Cmd/Ctrl+click par output ki line `take_click()` se milti hai.
//!
//! Layering (so the same core can be reused for NivOS later):
//!   * `term_core::Emulator`  - pure VT parser + screen grid (no egui / no OS)
//!   * this file              - PTY backend (`portable-pty`) + egui front-end
//!
//! The widget always fills the space the panel gives it and derives the
//! terminal size (cols x rows) from that space, so resizing the panel resizes
//! the shell (SIGWINCH) and the panel can shrink freely.

use egui::{
    Align2, Color32, Context, Event, FontId, Key, Modifiers, MouseWheelUnit, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use crate::menu::{Menu, MenuLine};
use crate::term_core::{attr, Cell, Color, Emulator};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

const FONT_SIZE: f32 = 13.0;
const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 4.0;
/// Max bytes parsed per frame, so a huge `cat` can't freeze the UI.
const MAX_FEED_PER_FRAME: usize = 512 * 1024;

enum PtyMsg {
    Data(Vec<u8>),
    Closed,
}

/// Something the terminal's own right-click menu asks the panel to do.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TermReq {
    Split,
    Kill,
}

pub struct Terminal {
    /// "zsh": the name the panel shows for this terminal
    name: String,
    /// "/bin/zsh": what was started
    shell: String,
    /// had the keyboard focus when it was drawn last
    focused: bool,
    req: Option<TermReq>,
    /// the line of text and column the user Cmd/Ctrl+clicked in the output
    link_click: Option<(String, usize)>,
    /// where the right-click menu is open
    menu: Option<Pos2>,
    uid: u64,
    emu: Emulator,
    receiver: Receiver<PtyMsg>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send>,
    exited: bool,

    /// How many lines the view is scrolled back into history (0 = live).
    scroll_back: f32,
    /// Selection in absolute line / column coordinates.
    sel_anchor: Option<(usize, usize)>,
    sel_head: Option<(usize, usize)>,

    last_evicted: u64,
    last_alt: bool,
    focus_done: bool,
}

impl Terminal {
    /// A terminal running `shell` (a path like /bin/bash); None = the user's own shell ($SHELL).
    pub fn spawn(ctx: Context, cwd: Option<std::path::PathBuf>, shell: Option<&str>) -> Result<Self, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = shell.map(|s| s.to_string()).or_else(|| std::env::var("SHELL").ok()).unwrap_or_else(|| "/bin/zsh".to_string());
        let name = shell.rsplit('/').next().unwrap_or("shell").to_string();
        let shell_path = shell.clone();
        let mut command = CommandBuilder::new(shell);
        command.arg("-l"); // login shell: loads PATH etc. like Terminal.app does
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        // Start in the folder opened in the explorer; fall back to HOME.
        match cwd.filter(|p| p.is_dir()) {
            Some(dir) => command.cwd(dir),
            None => {
                if let Ok(home) = std::env::var("HOME") {
                    command.cwd(home);
                }
            }
        }

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;

        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| e.to_string())?;

        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let (sender, receiver) = mpsc::channel::<PtyMsg>();

        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if sender.send(PtyMsg::Data(buffer[..n].to_vec())).is_err() {
                            return;
                        }
                        // wake the UI as soon as the shell printed something
                        ctx.request_repaint();
                    }
                }
            }
            let _ = sender.send(PtyMsg::Closed);
            ctx.request_repaint();
        });

        Ok(Self {
            name,
            shell: shell_path,
            focused: false,
            req: None,
            emu: Emulator::new(100, 24),
            receiver,
            writer,
            master: pair.master,
            child,
            exited: false,
            scroll_back: 0.0,
            link_click: None,
            menu: None,
            uid: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            sel_anchor: None,
            sel_head: None,
            last_evicted: 0,
            last_alt: false,
            focus_done: false,
        })
    }

    /// Everything that is on the screen now, as text (for tests and for "copy the whole screen").
    pub fn screen_text(&self) -> String {
        let first = self.emu.scrollback_len();
        (first..first + self.emu.rows())
            .filter_map(|l| self.emu.line(l))
            .map(|l| l.iter().map(|c| c.ch).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The shell's name ("zsh").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The program that was started ("/bin/zsh").
    pub fn shell_path(&self) -> &str {
        &self.shell
    }

    /// Did the keyboard focus sit in this terminal when it was drawn last?
    pub fn focused(&self) -> bool {
        self.focused
    }

    /// Give this terminal the keyboard focus when it is drawn next.
    pub fn focus_next(&mut self) {
        self.focus_done = false;
    }

    /// What the right-click menu asked for (consumed once).
    pub fn take_req(&mut self) -> Option<TermReq> {
        self.req.take()
    }

    /// The text of the line and the column that the user Cmd/Ctrl+clicked in the output (consumed once).
    /// The app decides what it means (for example `src/main.rs:12:5` opens a file).
    pub fn take_click(&mut self) -> Option<(String, usize)> {
        self.link_click.take()
    }

    /// True once the shell process has ended (e.g. the user typed `exit`).
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Type a line into the shell and press Enter (used by Run / Build).
    pub fn send_line(&mut self, line: &str) {
        self.write(line.as_bytes());
        self.write(b"\r");
    }

    /// Wipes the screen and the history (the panel's "Clear Terminal").
    pub fn clear_all(&mut self) {
        self.emu.feed(b"\x1b[3J");
        self.clear_selection();
        self.scroll_back = 0.0;
        self.clear_screen();
    }

    /// Ctrl+C: stop whatever is running in the shell.
    pub fn interrupt(&mut self) {
        self.write(&[0x03]);
    }

    /// Clear the terminal screen (Ctrl+L).
    pub fn clear_screen(&mut self) {
        self.write(&[0x0c]);
    }

    fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    // -----------------------------------------------------------------------
    // Selection helpers
    // -----------------------------------------------------------------------

    fn clear_selection(&mut self) {
        self.sel_anchor = None;
        self.sel_head = None;
    }

    fn shift_selection(&mut self, d: usize) {
        if let (Some(a), Some(h)) = (self.sel_anchor, self.sel_head) {
            if a.0 >= d && h.0 >= d {
                self.sel_anchor = Some((a.0 - d, a.1));
                self.sel_head = Some((h.0 - d, h.1));
            } else {
                self.clear_selection();
            }
        }
    }

    fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.sel_anchor?;
        let h = self.sel_head?;
        if a == h {
            return None;
        }
        Some(if a <= h { (a, h) } else { (h, a) })
    }

    /// Select the word under the cell (double click): everything around it that is not a space or a bracket / quote.
    /// A path like `src/main.rs:12:5` is one word, so it can be copied in one go.
    fn select_word(&mut self, abs: usize, col: usize) {
        let Some(line) = self.emu.line(abs) else {
            return;
        };
        let chars: Vec<char> = line.iter().map(|c| c.ch).collect();
        if let Some((a, b)) = word_span(&chars, col) {
            self.sel_anchor = Some((abs, a));
            self.sel_head = Some((abs, b));
        }
    }

    /// Select the whole row (triple click).
    fn select_row(&mut self, abs: usize, cols: usize) {
        self.sel_anchor = Some((abs, 0));
        self.sel_head = Some((abs, cols.saturating_sub(1)));
    }

    /// Select everything: history and screen.
    fn select_all(&mut self, rows: usize, cols: usize) {
        let total = self.emu.scrollback_len() + rows;
        self.sel_anchor = Some((0, 0));
        self.sel_head = Some((total.saturating_sub(1), cols.saturating_sub(1)));
    }

    fn selection_text(&self) -> Option<String> {
        let (s, e) = self.selection_range()?;
        let mut out = String::new();
        for l in s.0..=e.0 {
            let line = match self.emu.line(l) {
                Some(x) => x,
                None => continue,
            };
            if line.is_empty() {
                continue;
            }
            let last = line.len() - 1;
            let from = if l == s.0 { s.1 } else { 0 };
            let to = if l == e.0 { e.1.min(last) } else { last };
            let mut text = String::new();
            for c in from..=to {
                if let Some(cell) = line.get(c) {
                    text.push(cell.ch);
                }
            }
            out.push_str(text.trim_end());
            if l != e.0 {
                out.push('\n');
            }
        }
        Some(out)
    }

    // -----------------------------------------------------------------------
    // The widget
    // -----------------------------------------------------------------------

    /// Takes what the shell printed so far into the screen. The panel calls this for every terminal on every frame,
    /// also for the ones that are not shown (a hidden terminal keeps running, and its output must not pile up).
    pub fn pump(&mut self, ctx: &Context) {
        let sb_before = self.emu.scrollback_len();
        let mut budget = MAX_FEED_PER_FRAME;
        loop {
            if budget == 0 {
                ctx.request_repaint();
                break;
            }
            match self.receiver.try_recv() {
                Ok(PtyMsg::Data(d)) => {
                    budget = budget.saturating_sub(d.len());
                    self.emu.feed(&d);
                }
                Ok(PtyMsg::Closed) => self.exited = true,
                Err(_) => break,
            }
        }

        let replies = self.emu.take_responses();
        if !replies.is_empty() {
            self.write(&replies);
        }

        // keep a scrolled-back view steady while new lines arrive
        let sb_after = self.emu.scrollback_len();
        if self.scroll_back > 0.0 && sb_after > sb_before {
            self.scroll_back += (sb_after - sb_before) as f32;
        }
        let ev = self.emu.evicted();
        if ev != self.last_evicted {
            let d = (ev - self.last_evicted) as usize;
            self.last_evicted = ev;
            self.shift_selection(d);
        }
        if self.emu.in_alt() != self.last_alt {
            self.last_alt = self.emu.in_alt();
            self.clear_selection();
            self.scroll_back = 0.0;
        }
    }


    /// Terminal dikhao: panel ki bachi hui poori jagah leta hai, aur jagah ke hisaab se shell ka size badalta hai.
    pub fn show(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let default_fg = ui.visuals().text_color();
        let default_bg = ui.visuals().panel_fill;
        let font = FontId::monospace(FONT_SIZE);

        // --- cell metrics (measure 16 glyphs so rounding doesn't drift) -----
        let probe = ui
            .painter()
            .layout_no_wrap("M".repeat(16), font.clone(), default_fg);
        let cw = (probe.size().x / 16.0).max(1.0);
        let ch = probe.size().y.max(1.0);

        // --- take ALL the space the panel gives us ---------------------------
        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        let inner = rect.shrink2(Vec2::new(PAD_X, PAD_Y));

        let cols = ((inner.width() / cw).floor() as usize).max(2);
        let rows = ((inner.height() / ch).floor() as usize).max(1);

        if cols != self.emu.cols() || rows != self.emu.rows() {
            self.emu.resize(cols, rows);
            let _ = self.master.resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        self.pump(ui.ctx());

        // --- focus ------------------------------------------------------------
        if !self.focus_done {
            response.request_focus();
            self.focus_done = true;
        }
        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        // keep Tab / arrows / Esc for the shell instead of moving egui focus
        let filter = egui::EventFilter {
            tab: true,
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: true,
            ..Default::default()
        };
        ui.ctx()
            .memory_mut(|m| m.set_focus_lock_filter(response.id, filter));
        let focused = response.has_focus();
        self.focused = focused;

        if response.hovered() {
            ui.set_cursor_icon(egui::CursorIcon::Text);
        }

        let events: Vec<Event> = ui.input(|i| i.events.clone());

        // --- mouse wheel scrolls the history ------------------------------------
        if response.hovered() && !self.emu.in_alt() {
            for ev in &events {
                if let Event::MouseWheel { unit, delta, .. } = ev {
                    let lines = match unit {
                        MouseWheelUnit::Point => delta.y / ch,
                        MouseWheelUnit::Line => delta.y,
                        MouseWheelUnit::Page => delta.y * rows as f32,
                    };
                    self.scroll_back += lines;
                }
            }
        }

        let max_back = if self.emu.in_alt() {
            0.0
        } else {
            self.emu.scrollback_len() as f32
        };
        self.scroll_back = self.scroll_back.clamp(0.0, max_back);

        // --- mouse selection -----------------------------------------------------
        {
            let sb_len = self.emu.scrollback_len();
            let offset = self.scroll_back.floor() as usize;
            let cell_at = |pos: Pos2| -> (usize, usize) {
                let col =
                    (((pos.x - inner.min.x) / cw).floor().max(0.0) as usize).min(cols - 1);
                let row =
                    (((pos.y - inner.min.y) / ch).floor().max(0.0) as usize).min(rows - 1);
                (sb_len.saturating_sub(offset) + row, col)
            };

            if response.drag_started_by(egui::PointerButton::Primary) {
                // the selection starts where the button went down, not where the drag was noticed
                // (a few pixels later: the first character would be missed)
                let press = ui.input(|i| i.pointer.press_origin()).or_else(|| response.interact_pointer_pos());
                if let Some(pos) = press {
                    let p = cell_at(pos);
                    self.sel_anchor = Some(p);
                    self.sel_head = Some(p);
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.sel_head = Some(cell_at(pos));
                    if !self.emu.in_alt() {
                        // auto-scroll while selecting: faster the further the pointer is out
                        let edge = 14.0;
                        if pos.y < rect.min.y + edge {
                            self.scroll_back += ((rect.min.y + edge - pos.y) / 20.0).clamp(0.2, 3.0);
                        } else if pos.y > rect.max.y - edge {
                            self.scroll_back -= ((pos.y - (rect.max.y - edge)) / 20.0).clamp(0.2, 3.0);
                        }
                    }
                }
                ui.ctx().request_repaint();
            } else if response.triple_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (abs, _) = cell_at(pos);
                    self.select_row(abs, cols);
                }
            } else if response.double_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (abs, col) = cell_at(pos);
                    self.select_word(abs, col);
                }
            } else if response.clicked() {
                let cmd = ui.input(|i| i.modifiers.command);
                if cmd {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let (abs, col) = cell_at(pos);
                        if let Some(line) = self.emu.line(abs) {
                            self.link_click = Some((line.iter().map(|c| c.ch).collect(), col));
                        }
                    }
                }
                self.clear_selection();
            }
        }
        self.scroll_back = self.scroll_back.clamp(0.0, max_back);

        // --- keyboard -------------------------------------------------------------
        if focused {
            let mut out: Vec<u8> = Vec::new();
            for ev in &events {
                match ev {
                    Event::Text(s) => {
                        for c in s.chars() {
                            if !c.is_control() {
                                let mut b = [0u8; 4];
                                out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                            }
                        }
                    }
                    Event::Paste(s) => {
                        let text = s.replace("\r\n", "\r").replace('\n', "\r");
                        if self.emu.bracketed_paste() {
                            out.extend_from_slice(b"\x1b[200~");
                            out.extend_from_slice(text.as_bytes());
                            out.extend_from_slice(b"\x1b[201~");
                        } else {
                            out.extend_from_slice(text.as_bytes());
                        }
                    }
                    Event::Copy | Event::Cut => {
                        if let Some(t) = self.selection_text() {
                            ui.ctx().copy_text(t);
                        }
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        let combo = modifiers.mac_cmd || (modifiers.ctrl && modifiers.shift);
                        if combo && *key == Key::C {
                            if let Some(t) = self.selection_text() {
                                ui.ctx().copy_text(t);
                            }
                        } else if combo && *key == Key::A {
                            // select everything (history + screen)
                            self.select_all(rows, cols);
                            ui.ctx().request_repaint();
                        } else if *key == Key::PageUp && modifiers.shift {
                            self.scroll_back += rows.saturating_sub(1) as f32;
                        } else if *key == Key::PageDown && modifiers.shift {
                            self.scroll_back -= rows.saturating_sub(1) as f32;
                        } else if let Some(bytes) =
                            key_bytes(*key, *modifiers, self.emu.app_cursor())
                        {
                            out.extend_from_slice(&bytes);
                        }
                    }
                    _ => {}
                }
            }
            self.scroll_back = self.scroll_back.clamp(0.0, max_back);
            if !out.is_empty() {
                self.scroll_back = 0.0;
                self.clear_selection();
                self.write(&out);
            }
        }

        // --- right click: Copy / Paste / Select All / Clear / Split / Kill --------------------------
        {
            #[derive(Clone, Copy)]
            enum Act {
                Copy,
                Paste,
                All,
                Clear,
                Split,
                Kill,
            }
            if response.secondary_clicked() {
                self.menu = ui.input(|i| i.pointer.interact_pos());
            }
            let mut picked: Option<Act> = None;
            if let Some(at) = self.menu {
                let has_sel = self.selection_range().is_some();
                let lines = vec![
                    MenuLine::item_if("Copy", "", Act::Copy, has_sel),
                    MenuLine::item("Paste", "", Act::Paste),
                    MenuLine::Sep,
                    MenuLine::item("Select All", "", Act::All),
                    MenuLine::item("Clear", "", Act::Clear),
                    MenuLine::Sep,
                    MenuLine::item("Split Terminal", "", Act::Split),
                    MenuLine::item("Kill Terminal", "", Act::Kill),
                ];
                let mut open = true;
                picked = Menu(format!("niv-terminal-menu-{}", self.uid)).at(at.x, at.y).width(180.0).show(ui, &mut open, &lines);
                if !open {
                    self.menu = None;
                }
            }
            let mut touched = true;
            match picked {
                Some(Act::Copy) => {
                    if let Some(t) = self.selection_text() {
                        ui.ctx().copy_text(t);
                    }
                }
                // the text arrives as an `Event::Paste` in the next frame, which the terminal takes when it has the focus
                Some(Act::Paste) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                Some(Act::All) => self.select_all(rows, cols),
                Some(Act::Clear) => {
                    self.emu.feed(b"\x1b[3J");
                    self.clear_selection();
                    self.scroll_back = 0.0;
                    self.clear_screen();
                }
                Some(Act::Split) => self.req = Some(TermReq::Split),
                Some(Act::Kill) => self.req = Some(TermReq::Kill),
                None => touched = false,
            }
            if touched {
                response.request_focus();
                ui.ctx().request_repaint();
            }
        }

        // --- drawing -----------------------------------------------------------------
        let painter = ui.painter_at(rect);
        let sb_len = self.emu.scrollback_len();
        let offset = self.scroll_back.floor() as usize;
        let first_abs = sb_len.saturating_sub(offset);
        let sel = self.selection_range();
        let sel_color = Color32::from_rgba_unmultiplied(64, 128, 255, 90);

        for r in 0..rows {
            let abs = first_abs + r;
            let line = match self.emu.line(abs) {
                Some(l) => l,
                None => continue,
            };
            let y = inner.min.y + r as f32 * ch;
            let n = cols.min(line.len());

            // 1) cell backgrounds, merged into runs
            let mut c = 0;
            while c < n {
                if let Some(color) = cell_colors(&line[c], dark, default_fg, default_bg).1 {
                    let start = c;
                    while c + 1 < n
                        && cell_colors(&line[c + 1], dark, default_fg, default_bg).1
                            == Some(color)
                    {
                        c += 1;
                    }
                    let x0 = inner.min.x + start as f32 * cw;
                    let x1 = inner.min.x + (c as f32 + 1.0) * cw;
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + ch)),
                        0.0,
                        color,
                    );
                }
                c += 1;
            }

            // 2) selection highlight
            if let Some((s, e)) = sel {
                if abs >= s.0 && abs <= e.0 {
                    let from = if abs == s.0 { s.1 } else { 0 };
                    let to = if abs == e.0 { e.1 } else { cols - 1 };
                    if from <= to && from < cols {
                        let to = to.min(cols - 1);
                        let x0 = inner.min.x + from as f32 * cw;
                        let x1 = inner.min.x + (to as f32 + 1.0) * cw;
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + ch)),
                            0.0,
                            sel_color,
                        );
                    }
                }
            }

            // 3) text, in runs of ASCII with the same colour
            let draw_run = |run: &mut String, start: usize, fg: Color32| {
                let t = run.trim_end();
                if !t.is_empty() {
                    painter.text(
                        Pos2::new(inner.min.x + start as f32 * cw, y),
                        Align2::LEFT_TOP,
                        t,
                        font.clone(),
                        fg,
                    );
                }
                run.clear();
            };

            let mut run = String::new();
            let mut run_start = 0usize;
            let mut run_fg = default_fg;

            for c in 0..n {
                let cell = &line[c];
                let (fg, _) = cell_colors(cell, dark, default_fg, default_bg);
                let x = inner.min.x + c as f32 * cw;

                if cell.attrs & attr::UNDERLINE != 0 {
                    let yy = y + ch - 2.0;
                    painter.line_segment(
                        [Pos2::new(x, yy), Pos2::new(x + cw, yy)],
                        Stroke::new(1.0, fg),
                    );
                }
                if cell.attrs & attr::STRIKE != 0 {
                    let yy = y + ch * 0.5;
                    painter.line_segment(
                        [Pos2::new(x, yy), Pos2::new(x + cw, yy)],
                        Stroke::new(1.0, fg),
                    );
                }

                if cell.ch == ' ' {
                    if !run.is_empty() {
                        run.push(' ');
                    }
                    continue;
                }

                if !cell.ch.is_ascii() {
                    // non-ASCII glyphs may have a different advance: pin each to its cell
                    draw_run(&mut run, run_start, run_fg);
                    painter.text(
                        Pos2::new(x, y),
                        Align2::LEFT_TOP,
                        cell.ch.to_string(),
                        font.clone(),
                        fg,
                    );
                    continue;
                }

                if run.is_empty() {
                    run_start = c;
                    run_fg = fg;
                } else if run_fg != fg {
                    draw_run(&mut run, run_start, run_fg);
                    run_start = c;
                    run_fg = fg;
                }
                run.push(cell.ch);
            }
            draw_run(&mut run, run_start, run_fg);
        }

        // --- cursor (only while looking at the live screen) ----------------------------
        if offset == 0 && self.emu.cursor_visible() && !self.exited {
            let (cx, cy) = self.emu.cursor();
            if cx < cols && cy < rows {
                let x = inner.min.x + cx as f32 * cw;
                let y = inner.min.y + cy as f32 * ch;
                let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cw, ch));
                if focused {
                    painter.rect_filled(r, 0.0, default_fg);
                    if let Some(line) = self.emu.line(sb_len + cy) {
                        if let Some(cell) = line.get(cx) {
                            if cell.ch != ' ' {
                                painter.text(
                                    Pos2::new(x, y),
                                    Align2::LEFT_TOP,
                                    cell.ch.to_string(),
                                    font.clone(),
                                    default_bg,
                                );
                            }
                        }
                    }
                } else {
                    // hollow cursor when the terminal does not have focus
                    let s = Stroke::new(1.0, default_fg);
                    painter.line_segment([r.left_top(), r.right_top()], s);
                    painter.line_segment([r.right_top(), r.right_bottom()], s);
                    painter.line_segment([r.right_bottom(), r.left_bottom()], s);
                    painter.line_segment([r.left_bottom(), r.left_top()], s);
                }
            }
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

fn ansi256(i: u8, dark: bool) -> Color32 {
    const DARK: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];
    const LIGHT: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (0, 160, 0),
        (148, 152, 0),
        (4, 81, 165),
        (188, 5, 188),
        (5, 152, 188),
        (85, 85, 85),
        (102, 102, 102),
        (205, 49, 49),
        (20, 170, 20),
        (181, 186, 0),
        (4, 81, 165),
        (188, 5, 188),
        (5, 152, 188),
        (130, 130, 130),
    ];

    let i = i as usize;
    if i < 16 {
        let (r, g, b) = if dark { DARK[i] } else { LIGHT[i] };
        return Color32::from_rgb(r, g, b);
    }
    if i < 232 {
        let n = i - 16;
        let level = |v: usize| -> u8 {
            if v == 0 {
                0
            } else {
                (55 + v * 40) as u8
            }
        };
        return Color32::from_rgb(level(n / 36), level((n / 6) % 6), level(n % 6));
    }
    let g = (8 + (i - 232) * 10) as u8;
    Color32::from_rgb(g, g, g)
}

fn to_color32(c: Color, dark: bool) -> Color32 {
    match c {
        Color::Default => Color32::WHITE,
        Color::Indexed(i) => ansi256(i, dark),
        Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

/// Resolve a cell to (foreground, optional background). `None` = transparent.
fn cell_colors(cell: &Cell, dark: bool, dfg: Color32, dbg: Color32) -> (Color32, Option<Color32>) {
    let mut fg_c = cell.fg;
    // bold + one of the 8 base colours = bright variant
    if cell.attrs & attr::BOLD != 0 {
        if let Color::Indexed(i) = fg_c {
            if i < 8 {
                fg_c = Color::Indexed(i + 8);
            }
        }
    }

    let mut fg = match fg_c {
        Color::Default => dfg,
        other => to_color32(other, dark),
    };
    let mut bg = match cell.bg {
        Color::Default => None,
        other => Some(to_color32(other, dark)),
    };

    if cell.attrs & attr::REVERSE != 0 {
        let new_fg = bg.unwrap_or(dbg);
        let new_bg = fg;
        fg = new_fg;
        bg = Some(new_bg);
    }
    if cell.attrs & attr::DIM != 0 {
        fg = fg.gamma_multiply(0.6);
    }
    if cell.attrs & attr::HIDDEN != 0 {
        fg = bg.unwrap_or(Color32::TRANSPARENT);
    }
    (fg, bg)
}

// ---------------------------------------------------------------------------
// Keyboard -> bytes
// ---------------------------------------------------------------------------

fn key_bytes(key: Key, m: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    // macOS: a few Cmd shortcuts edit the command line, every other Cmd
    // combination belongs to the app (Cmd+J, Cmd+C, Cmd+V, ...).
    if m.mac_cmd {
        return match key {
            Key::ArrowLeft => Some(vec![0x01]),  // line start
            Key::ArrowRight => Some(vec![0x05]), // line end
            Key::Backspace => Some(vec![0x15]),  // kill line
            Key::K => Some(vec![0x0c]),          // clear screen
            _ => None,
        };
    }

    let ctrl = m.ctrl;
    let alt = m.alt;
    let shift = m.shift;
    // xterm modifier parameter: 1 + shift(1) + alt(2) + ctrl(4)
    let xmod = 1 + (shift as u8) + 2 * (alt as u8) + 4 * (ctrl as u8);

    let csi = |c: char| -> Vec<u8> {
        if xmod > 1 {
            format!("\x1b[1;{}{}", xmod, c).into_bytes()
        } else if app_cursor {
            format!("\x1bO{}", c).into_bytes()
        } else {
            format!("\x1b[{}", c).into_bytes()
        }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if xmod > 1 {
            format!("\x1b[{};{}~", n, xmod).into_bytes()
        } else {
            format!("\x1b[{}~", n).into_bytes()
        }
    };

    match key {
        Key::Enter => Some(vec![b'\r']),
        Key::Backspace => Some(if alt { vec![0x1b, 0x7f] } else { vec![0x7f] }),
        Key::Tab => Some(if shift { b"\x1b[Z".to_vec() } else { vec![b'\t'] }),
        Key::Escape => Some(vec![0x1b]),

        // Option+Left / Option+Right = move by word (what zsh expects on macOS)
        Key::ArrowLeft if alt && !ctrl && !shift => Some(b"\x1bb".to_vec()),
        Key::ArrowRight if alt && !ctrl && !shift => Some(b"\x1bf".to_vec()),

        Key::ArrowUp => Some(csi('A')),
        Key::ArrowDown => Some(csi('B')),
        Key::ArrowRight => Some(csi('C')),
        Key::ArrowLeft => Some(csi('D')),
        Key::Home => Some(csi('H')),
        Key::End => Some(csi('F')),

        Key::PageUp => Some(tilde(5)),
        Key::PageDown => Some(tilde(6)),
        Key::Delete => Some(tilde(3)),
        Key::Insert => Some(tilde(2)),

        Key::F1 => Some(b"\x1bOP".to_vec()),
        Key::F2 => Some(b"\x1bOQ".to_vec()),
        Key::F3 => Some(b"\x1bOR".to_vec()),
        Key::F4 => Some(b"\x1bOS".to_vec()),
        Key::F5 => Some(tilde(15)),
        Key::F6 => Some(tilde(17)),
        Key::F7 => Some(tilde(18)),
        Key::F8 => Some(tilde(19)),
        Key::F9 => Some(tilde(20)),
        Key::F10 => Some(tilde(21)),
        Key::F11 => Some(tilde(23)),
        Key::F12 => Some(tilde(24)),

        _ => {
            if ctrl && !alt {
                match key {
                    Key::OpenBracket => return Some(vec![0x1b]),
                    Key::Backslash => return Some(vec![0x1c]),
                    Key::CloseBracket => return Some(vec![0x1d]),
                    Key::Space => return Some(vec![0x00]),
                    _ => {}
                }
                // Ctrl+A .. Ctrl+Z  ->  0x01 .. 0x1a  (Ctrl+C = SIGINT, Ctrl+D = EOF ...)
                let name = key.name();
                let b = name.as_bytes();
                if b.len() == 1 && b[0].is_ascii_uppercase() {
                    return Some(vec![b[0] - b'A' + 1]);
                }
            }
            None
        }
    }
}

/// Characters that end a word for double click (the same as VS Code's terminal): spaces and brackets / quotes.
fn is_word_break(c: char) -> bool {
    c.is_whitespace() || c == '\0' || "()[]{}'\"`|<>;,".contains(c)
}

/// The first and last column of the word at `col`, or None when `col` is on a separator / past the line.
fn word_span(chars: &[char], col: usize) -> Option<(usize, usize)> {
    if col >= chars.len() || is_word_break(chars[col]) {
        return None;
    }
    let mut a = col;
    while a > 0 && !is_word_break(chars[a - 1]) {
        a -= 1;
    }
    let mut b = col;
    while b + 1 < chars.len() && !is_word_break(chars[b + 1]) {
        b += 1;
    }
    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::word_span;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn double_click_takes_a_whole_path() {
        let l = chars("error at src/main.rs:12:5 now");
        assert_eq!(word_span(&l, 12), Some((9, 24)), "inside the path");
        assert_eq!(word_span(&l, 9), Some((9, 24)), "its first char");
        assert_eq!(word_span(&l, 24), Some((9, 24)), "its last char");
        assert_eq!(word_span(&l, 0), Some((0, 4)));
    }

    #[test]
    fn brackets_quotes_and_spaces_end_a_word() {
        let l = chars("foo(\"bar baz\")");
        assert_eq!(word_span(&l, 5), Some((5, 7)));
        assert_eq!(word_span(&l, 3), None, "on the bracket");
        assert_eq!(word_span(&l, 8), None, "on the space");
        assert_eq!(word_span(&l, 40), None, "past the end of the line");
    }
}
