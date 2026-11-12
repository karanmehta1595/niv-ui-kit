//! term_core — a small, UI-independent terminal emulator core.
//!
//! No egui, no PTY, no OS calls in here: bytes go in through `feed()`,
//! a grid of cells (plus scrollback) comes out. Anything the terminal must
//! answer back to the shell (cursor position reports etc.) is collected in
//! `take_responses()`. Because of this the same file can later be reused
//! unchanged for NivOS (it only needs `std`; porting to `no_std + alloc`
//! is a matter of swapping `String`/`Vec`/`VecDeque` imports).

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Cell model
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub mod attr {
    pub const BOLD: u8 = 1;
    pub const DIM: u8 = 2;
    pub const ITALIC: u8 = 4;
    pub const UNDERLINE: u8 = 8;
    pub const REVERSE: u8 = 16;
    pub const STRIKE: u8 = 32;
    pub const HIDDEN: u8 = 64;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: u8,
}

impl Cell {
    pub fn blank() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Pen {
    fg: Color,
    bg: Color,
    attrs: u8,
}

impl Pen {
    fn new() -> Self {
        Pen {
            fg: Color::Default,
            bg: Color::Default,
            attrs: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    /// ESC ( B, ESC # 8 ... : swallow exactly one more byte
    EscInter,
    Csi,
    Osc,
    /// DCS / APC / PM / SOS: ignored until ST or BEL
    StringIgnore,
}

const MAX_PARAMS: usize = 32;
const MAX_OSC: usize = 4096;

pub struct Emulator {
    cols: usize,
    rows: usize,

    /// Active screen buffer (always `rows` lines of `cols` cells).
    screen: Vec<Vec<Cell>>,
    /// The inactive buffer (primary while the alt screen is shown, and vice versa).
    alt: Vec<Vec<Cell>>,
    in_alt: bool,

    scrollback: VecDeque<Vec<Cell>>,
    max_scrollback: usize,
    evicted: u64,

    cx: usize,
    cy: usize,
    wrap_pending: bool,
    pen: Pen,
    saved_cursor: Option<(usize, usize, Pen)>,
    alt_saved_cursor: Option<(usize, usize, Pen)>,

    top: usize,
    bottom: usize,
    tabs: Vec<bool>,
    last_char: Option<char>,

    // modes
    autowrap: bool,
    cursor_visible: bool,
    app_cursor: bool,
    bracketed_paste: bool,

    // parser
    state: State,
    params: Vec<u32>,
    cur_param: Option<u32>,
    private: u8,
    inter: u8,
    osc_buf: Vec<u8>,
    utf8_buf: [u8; 4],
    utf8_len: usize,
    utf8_need: usize,

    responses: Vec<u8>,
}

fn blank_grid(rows: usize, cols: usize) -> Vec<Vec<Cell>> {
    vec![vec![Cell::blank(); cols]; rows]
}

impl Emulator {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(1);
        let mut e = Emulator {
            cols,
            rows,
            screen: blank_grid(rows, cols),
            alt: blank_grid(rows, cols),
            in_alt: false,
            scrollback: VecDeque::new(),
            max_scrollback: 10_000,
            evicted: 0,
            cx: 0,
            cy: 0,
            wrap_pending: false,
            pen: Pen::new(),
            saved_cursor: None,
            alt_saved_cursor: None,
            top: 0,
            bottom: rows - 1,
            tabs: Vec::new(),
            last_char: None,
            autowrap: true,
            cursor_visible: true,
            app_cursor: false,
            bracketed_paste: false,
            state: State::Ground,
            params: Vec::new(),
            cur_param: None,
            private: 0,
            inter: 0,
            osc_buf: Vec::new(),
            utf8_buf: [0; 4],
            utf8_len: 0,
            utf8_need: 0,
            responses: Vec::new(),
        };
        e.reset_tabs();
        e
    }

    // -----------------------------------------------------------------------
    // Public read API (used by the renderer)
    // -----------------------------------------------------------------------

    pub fn cols(&self) -> usize {
        self.cols
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cursor(&self) -> (usize, usize) {
        (self.cx, self.cy)
    }
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }
    pub fn app_cursor(&self) -> bool {
        self.app_cursor
    }
    pub fn bracketed_paste(&self) -> bool {
        self.bracketed_paste
    }
    pub fn in_alt(&self) -> bool {
        self.in_alt
    }
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }
    /// Total number of lines dropped from the top of the scrollback so far.
    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// Line by absolute index: `0..scrollback_len()` is history,
    /// the following `rows()` lines are the live screen.
    pub fn line(&self, abs: usize) -> Option<&Vec<Cell>> {
        let sb = self.scrollback.len();
        if abs < sb {
            self.scrollback.get(abs)
        } else {
            self.screen.get(abs - sb)
        }
    }

    /// Bytes the terminal wants to send back to the shell.
    pub fn take_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.responses)
    }

    // -----------------------------------------------------------------------
    // Resize
    // -----------------------------------------------------------------------

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(2);
        let rows = rows.max(1);
        if cols == self.cols && rows == self.rows {
            return;
        }

        // width: truncate / pad every line (no reflow)
        for line in self.screen.iter_mut() {
            line.resize(cols, Cell::blank());
        }
        for line in self.alt.iter_mut() {
            line.resize(cols, Cell::blank());
        }

        // height of the active screen
        if rows < self.screen.len() {
            let mut remove = self.screen.len() - rows;
            // 1) drop empty lines below the cursor first
            while remove > 0
                && self.screen.len() > self.cy + 1
                && self.screen.last().map(|l| line_is_blank(l)).unwrap_or(false)
            {
                self.screen.pop();
                remove -= 1;
            }
            // 2) then push the top lines into the scrollback
            while remove > 0 {
                let line = self.screen.remove(0);
                if !self.in_alt {
                    self.push_scrollback(line);
                }
                self.cy = self.cy.saturating_sub(1);
                remove -= 1;
            }
        } else if rows > self.screen.len() {
            let add = rows - self.screen.len();
            for _ in 0..add {
                if !self.in_alt {
                    if let Some(mut line) = self.scrollback.pop_back() {
                        line.resize(cols, Cell::blank());
                        self.screen.insert(0, line);
                        self.cy += 1;
                        continue;
                    }
                }
                self.screen.push(vec![Cell::blank(); cols]);
            }
        }

        // height of the inactive screen
        self.alt.truncate(rows);
        while self.alt.len() < rows {
            self.alt.push(vec![Cell::blank(); cols]);
        }

        self.cols = cols;
        self.rows = rows;
        self.top = 0;
        self.bottom = rows - 1;
        self.cx = self.cx.min(cols - 1);
        self.cy = self.cy.min(rows - 1);
        self.wrap_pending = false;
        self.reset_tabs();
    }

    // -----------------------------------------------------------------------
    // Feeding bytes
    // -----------------------------------------------------------------------

    pub fn feed(&mut self, data: &[u8]) {
        for &b in data {
            self.byte(b);
        }
    }

    fn byte(&mut self, b: u8) {
        match self.state {
            State::Ground => self.ground(b),
            State::Escape => self.escape(b),
            State::EscInter => {
                self.state = State::Ground;
            }
            State::Csi => self.csi_byte(b),
            State::Osc => self.osc_byte(b),
            State::StringIgnore => self.string_byte(b),
        }
    }

    fn ground(&mut self, b: u8) {
        match b {
            0x1b => {
                self.utf8_need = 0;
                self.utf8_len = 0;
                self.state = State::Escape;
            }
            0x00..=0x1f => {
                self.utf8_need = 0;
                self.utf8_len = 0;
                self.execute(b);
            }
            0x20..=0x7e => {
                self.utf8_need = 0;
                self.utf8_len = 0;
                self.print(b as char);
            }
            0x7f => {}
            _ => self.utf8_byte(b),
        }
    }

    fn utf8_byte(&mut self, b: u8) {
        if self.utf8_need == 0 {
            let need = if b & 0xE0 == 0xC0 {
                1
            } else if b & 0xF0 == 0xE0 {
                2
            } else if b & 0xF8 == 0xF0 {
                3
            } else {
                0
            };
            if need == 0 {
                self.print('\u{FFFD}');
                return;
            }
            self.utf8_buf[0] = b;
            self.utf8_len = 1;
            self.utf8_need = need;
        } else if b & 0xC0 == 0x80 {
            self.utf8_buf[self.utf8_len] = b;
            self.utf8_len += 1;
            self.utf8_need -= 1;
            if self.utf8_need == 0 {
                let buf = self.utf8_buf;
                let len = self.utf8_len;
                self.utf8_len = 0;
                let ch = std::str::from_utf8(&buf[..len])
                    .ok()
                    .and_then(|s| s.chars().next())
                    .unwrap_or('\u{FFFD}');
                self.print(ch);
            }
        } else {
            // broken sequence: emit a replacement char and reprocess this byte
            self.utf8_need = 0;
            self.utf8_len = 0;
            self.print('\u{FFFD}');
            self.ground(b);
        }
    }

    fn escape(&mut self, b: u8) {
        self.state = State::Ground;
        match b {
            b'[' => {
                self.params.clear();
                self.cur_param = None;
                self.private = 0;
                self.inter = 0;
                self.state = State::Csi;
            }
            b']' => {
                self.osc_buf.clear();
                self.state = State::Osc;
            }
            b'P' | b'X' | b'^' | b'_' => self.state = State::StringIgnore,
            b'7' => self.save_cursor(),
            b'8' => self.restore_cursor(),
            b'D' => {
                self.wrap_pending = false;
                self.linefeed();
            }
            b'E' => {
                self.cx = 0;
                self.wrap_pending = false;
                self.linefeed();
            }
            b'M' => {
                self.wrap_pending = false;
                self.reverse_index();
            }
            b'H' => {
                if self.cx < self.tabs.len() {
                    self.tabs[self.cx] = true;
                }
            }
            b'c' => self.full_reset(),
            b'=' | b'>' => {}
            b'(' | b')' | b'*' | b'+' | b'#' | b'%' | b' ' => self.state = State::EscInter,
            0x1b => self.state = State::Escape,
            0x18 | 0x1a => {}
            0x00..=0x1f => {
                self.execute(b);
                self.state = State::Escape;
            }
            _ => {}
        }
    }

    fn csi_byte(&mut self, b: u8) {
        match b {
            0x18 | 0x1a => self.state = State::Ground,
            0x1b => self.state = State::Escape,
            0x00..=0x1f => self.execute(b),
            b'0'..=b'9' => {
                let d = (b - b'0') as u32;
                let v = self
                    .cur_param
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(d)
                    .min(65535);
                self.cur_param = Some(v);
            }
            b';' | b':' => {
                if self.params.len() < MAX_PARAMS {
                    let v = self.cur_param.take().unwrap_or(0);
                    self.params.push(v);
                }
                self.cur_param = None;
            }
            b'<' | b'=' | b'>' | b'?' => self.private = b,
            0x20..=0x2f => self.inter = b,
            0x40..=0x7e => {
                if self.cur_param.is_some() || !self.params.is_empty() {
                    if self.params.len() < MAX_PARAMS {
                        let v = self.cur_param.take().unwrap_or(0);
                        self.params.push(v);
                    }
                }
                self.cur_param = None;
                self.state = State::Ground;
                self.csi_dispatch(b);
            }
            _ => self.state = State::Ground,
        }
    }

    fn osc_byte(&mut self, b: u8) {
        match b {
            0x07 => {
                self.finish_osc();
                self.state = State::Ground;
            }
            0x1b => {
                // ESC \ (ST): finish now, the '\' is swallowed by the Escape state
                self.finish_osc();
                self.state = State::Escape;
            }
            0x18 | 0x1a => self.state = State::Ground,
            _ => {
                if self.osc_buf.len() < MAX_OSC {
                    self.osc_buf.push(b);
                }
            }
        }
    }

    fn finish_osc(&mut self) {
        // window titles and the other OSC commands are not used: the text is just dropped
        self.osc_buf.clear();
    }

    fn string_byte(&mut self, b: u8) {
        match b {
            0x07 | 0x18 | 0x1a => self.state = State::Ground,
            0x1b => self.state = State::Escape,
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // C0 controls and printing
    // -----------------------------------------------------------------------

    fn execute(&mut self, b: u8) {
        match b {
            0x08 => {
                // BS
                self.wrap_pending = false;
                self.cx = self.cx.saturating_sub(1);
            }
            0x09 => {
                // HT
                self.wrap_pending = false;
                self.forward_tab(1);
            }
            0x0a | 0x0b | 0x0c => {
                // LF VT FF
                self.wrap_pending = false;
                self.linefeed();
            }
            0x0d => {
                // CR
                self.wrap_pending = false;
                self.cx = 0;
            }
            _ => {} // BEL, SO, SI, ...
        }
    }

    fn print(&mut self, c: char) {
        // zero-width / combining marks: dropped (no grapheme clusters yet)
        if ('\u{300}'..='\u{36f}').contains(&c)
            || ('\u{fe00}'..='\u{fe0f}').contains(&c)
            || c == '\u{200d}'
            || c == '\u{200b}'
        {
            return;
        }

        if self.wrap_pending {
            if self.autowrap {
                self.cx = 0;
                self.linefeed();
            }
            self.wrap_pending = false;
        }

        let cell = Cell {
            ch: c,
            fg: self.pen.fg,
            bg: self.pen.bg,
            attrs: self.pen.attrs,
        };
        if self.cy < self.screen.len() && self.cx < self.screen[self.cy].len() {
            self.screen[self.cy][self.cx] = cell;
        }
        self.last_char = Some(c);

        if self.cx + 1 < self.cols {
            self.cx += 1;
        } else if self.autowrap {
            self.wrap_pending = true;
        }
    }

    fn linefeed(&mut self) {
        if self.cy == self.bottom {
            self.scroll_up(1);
        } else if self.cy + 1 < self.rows {
            self.cy += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.cy == self.top {
            self.scroll_down(1);
        } else if self.cy > 0 {
            self.cy -= 1;
        }
    }

    fn blank_cell(&self) -> Cell {
        Cell {
            ch: ' ',
            fg: Color::Default,
            bg: self.pen.bg,
            attrs: 0,
        }
    }

    fn blank_row(&self) -> Vec<Cell> {
        vec![self.blank_cell(); self.cols]
    }

    fn push_scrollback(&mut self, line: Vec<Cell>) {
        if self.scrollback.len() >= self.max_scrollback {
            self.scrollback.pop_front();
            self.evicted += 1;
        }
        self.scrollback.push_back(line);
    }

    fn scroll_up(&mut self, n: usize) {
        let n = n.min(self.bottom + 1 - self.top);
        for _ in 0..n {
            let line = self.screen.remove(self.top);
            if self.top == 0 && !self.in_alt {
                self.push_scrollback(line);
            }
            let blank = self.blank_row();
            self.screen.insert(self.bottom, blank);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        let n = n.min(self.bottom + 1 - self.top);
        for _ in 0..n {
            self.screen.remove(self.bottom);
            let blank = self.blank_row();
            self.screen.insert(self.top, blank);
        }
    }

    // -----------------------------------------------------------------------
    // Tabs / cursor save
    // -----------------------------------------------------------------------

    fn reset_tabs(&mut self) {
        self.tabs = (0..self.cols).map(|i| i % 8 == 0 && i != 0).collect();
    }

    fn forward_tab(&mut self, n: usize) {
        for _ in 0..n {
            let mut x = self.cx + 1;
            while x < self.cols - 1 && !self.tabs.get(x).copied().unwrap_or(false) {
                x += 1;
            }
            self.cx = x.min(self.cols - 1);
        }
    }

    fn backward_tab(&mut self, n: usize) {
        for _ in 0..n {
            let mut x = self.cx.saturating_sub(1);
            while x > 0 && !self.tabs.get(x).copied().unwrap_or(false) {
                x -= 1;
            }
            self.cx = x;
        }
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = Some((self.cx, self.cy, self.pen));
    }

    fn restore_cursor(&mut self) {
        if let Some((x, y, pen)) = self.saved_cursor {
            self.cx = x.min(self.cols - 1);
            self.cy = y.min(self.rows - 1);
            self.pen = pen;
        } else {
            self.cx = 0;
            self.cy = 0;
        }
        self.wrap_pending = false;
    }

    // -----------------------------------------------------------------------
    // CSI dispatch
    // -----------------------------------------------------------------------

    /// n-th parameter, with 0 / missing replaced by `default`.
    fn p(&self, i: usize, default: usize) -> usize {
        match self.params.get(i) {
            Some(&v) if v != 0 => v as usize,
            _ => default,
        }
    }

    /// n-th parameter, raw (0 stays 0, missing is 0).
    fn raw(&self, i: usize) -> u32 {
        self.params.get(i).copied().unwrap_or(0)
    }

    fn csi_dispatch(&mut self, fin: u8) {
        if self.inter != 0 {
            if self.inter == b'!' && fin == b'p' {
                self.soft_reset();
            }
            return;
        }

        if self.private == b'?' {
            match fin {
                b'h' => self.set_private_modes(true),
                b'l' => self.set_private_modes(false),
                _ => {}
            }
            return;
        }

        if self.private == b'>' {
            if fin == b'c' {
                self.responses.extend_from_slice(b"\x1b[>0;0;0c");
            }
            return;
        }

        if self.private != 0 {
            return;
        }

        let rows = self.rows;
        let cols = self.cols;

        match fin {
            b'A' => {
                let n = self.p(0, 1);
                let lim = if self.cy >= self.top { self.top } else { 0 };
                self.cy = self.cy.saturating_sub(n).max(lim);
                self.wrap_pending = false;
            }
            b'B' | b'e' => {
                let n = self.p(0, 1);
                let lim = if self.cy <= self.bottom { self.bottom } else { rows - 1 };
                self.cy = (self.cy + n).min(lim);
                self.wrap_pending = false;
            }
            b'C' | b'a' => {
                let n = self.p(0, 1);
                self.cx = (self.cx + n).min(cols - 1);
                self.wrap_pending = false;
            }
            b'D' => {
                let n = self.p(0, 1);
                self.cx = self.cx.saturating_sub(n);
                self.wrap_pending = false;
            }
            b'E' => {
                let n = self.p(0, 1);
                let lim = if self.cy <= self.bottom { self.bottom } else { rows - 1 };
                self.cy = (self.cy + n).min(lim);
                self.cx = 0;
                self.wrap_pending = false;
            }
            b'F' => {
                let n = self.p(0, 1);
                let lim = if self.cy >= self.top { self.top } else { 0 };
                self.cy = self.cy.saturating_sub(n).max(lim);
                self.cx = 0;
                self.wrap_pending = false;
            }
            b'G' | b'`' => {
                self.cx = (self.p(0, 1) - 1).min(cols - 1);
                self.wrap_pending = false;
            }
            b'd' => {
                self.cy = (self.p(0, 1) - 1).min(rows - 1);
                self.wrap_pending = false;
            }
            b'H' | b'f' => {
                self.cy = (self.p(0, 1) - 1).min(rows - 1);
                self.cx = (self.p(1, 1) - 1).min(cols - 1);
                self.wrap_pending = false;
            }
            b'I' => {
                let n = self.p(0, 1);
                self.forward_tab(n);
                self.wrap_pending = false;
            }
            b'Z' => {
                let n = self.p(0, 1);
                self.backward_tab(n);
                self.wrap_pending = false;
            }
            b'J' => self.erase_display(self.raw(0)),
            b'K' => self.erase_line(self.raw(0)),
            b'L' => {
                if self.cy >= self.top && self.cy <= self.bottom {
                    let n = self.p(0, 1).min(self.bottom + 1 - self.cy);
                    for _ in 0..n {
                        self.screen.remove(self.bottom);
                        let blank = self.blank_row();
                        self.screen.insert(self.cy, blank);
                    }
                    self.cx = 0;
                    self.wrap_pending = false;
                }
            }
            b'M' => {
                if self.cy >= self.top && self.cy <= self.bottom {
                    let n = self.p(0, 1).min(self.bottom + 1 - self.cy);
                    for _ in 0..n {
                        self.screen.remove(self.cy);
                        let blank = self.blank_row();
                        self.screen.insert(self.bottom, blank);
                    }
                    self.cx = 0;
                    self.wrap_pending = false;
                }
            }
            b'@' => {
                let n = self.p(0, 1).min(cols - self.cx);
                let blank = self.blank_cell();
                let line = &mut self.screen[self.cy];
                for _ in 0..n {
                    line.insert(self.cx, blank);
                    line.truncate(cols);
                }
                self.wrap_pending = false;
            }
            b'P' => {
                let n = self.p(0, 1).min(cols - self.cx);
                let blank = self.blank_cell();
                let line = &mut self.screen[self.cy];
                for _ in 0..n {
                    line.remove(self.cx);
                    line.push(blank);
                }
                self.wrap_pending = false;
            }
            b'X' => {
                let n = self.p(0, 1).min(cols - self.cx);
                let blank = self.blank_cell();
                let line = &mut self.screen[self.cy];
                for i in 0..n {
                    line[self.cx + i] = blank;
                }
            }
            b'S' => {
                let n = self.p(0, 1);
                self.scroll_up(n);
            }
            b'T' => {
                let n = self.p(0, 1);
                self.scroll_down(n);
            }
            b'b' => {
                if let Some(c) = self.last_char {
                    let n = self.p(0, 1).min(10_000);
                    for _ in 0..n {
                        self.print(c);
                    }
                }
            }
            b'm' => self.sgr(),
            b'r' => {
                let top = self.p(0, 1) - 1;
                let bottom = self.p(1, rows) - 1;
                let bottom = bottom.min(rows - 1);
                if top < bottom {
                    self.top = top;
                    self.bottom = bottom;
                    self.cx = 0;
                    self.cy = 0;
                    self.wrap_pending = false;
                }
            }
            b's' => self.save_cursor(),
            b'u' => self.restore_cursor(),
            b'g' => match self.raw(0) {
                0 => {
                    if self.cx < self.tabs.len() {
                        self.tabs[self.cx] = false;
                    }
                }
                3 => {
                    for t in self.tabs.iter_mut() {
                        *t = false;
                    }
                }
                _ => {}
            },
            b'n' => match self.raw(0) {
                5 => self.responses.extend_from_slice(b"\x1b[0n"),
                6 => {
                    let s = format!("\x1b[{};{}R", self.cy + 1, self.cx + 1);
                    self.responses.extend_from_slice(s.as_bytes());
                }
                _ => {}
            },
            b'c' => {
                if self.raw(0) == 0 {
                    self.responses.extend_from_slice(b"\x1b[?1;2c");
                }
            }
            _ => {}
        }
    }

    fn set_private_modes(&mut self, on: bool) {
        let params = self.params.clone();
        for m in params {
            match m {
                1 => self.app_cursor = on,
                7 => self.autowrap = on,
                25 => self.cursor_visible = on,
                47 | 1047 => {
                    if on {
                        self.enter_alt(false);
                    } else {
                        self.leave_alt(false);
                    }
                }
                1048 => {
                    if on {
                        self.save_cursor();
                    } else {
                        self.restore_cursor();
                    }
                }
                1049 => {
                    if on {
                        self.enter_alt(true);
                    } else {
                        self.leave_alt(true);
                    }
                }
                2004 => self.bracketed_paste = on,
                _ => {}
            }
        }
    }

    fn enter_alt(&mut self, save: bool) {
        if self.in_alt {
            return;
        }
        if save {
            self.alt_saved_cursor = Some((self.cx, self.cy, self.pen));
        }
        std::mem::swap(&mut self.screen, &mut self.alt);
        self.in_alt = true;
        self.screen = blank_grid(self.rows, self.cols);
        self.top = 0;
        self.bottom = self.rows - 1;
        self.wrap_pending = false;
    }

    fn leave_alt(&mut self, restore: bool) {
        if !self.in_alt {
            return;
        }
        std::mem::swap(&mut self.screen, &mut self.alt);
        self.in_alt = false;
        self.top = 0;
        self.bottom = self.rows - 1;
        self.wrap_pending = false;
        if restore {
            if let Some((x, y, pen)) = self.alt_saved_cursor.take() {
                self.cx = x.min(self.cols - 1);
                self.cy = y.min(self.rows - 1);
                self.pen = pen;
            }
        }
    }

    fn erase_display(&mut self, mode: u32) {
        let blank = self.blank_cell();
        match mode {
            0 => {
                for x in self.cx..self.cols {
                    self.screen[self.cy][x] = blank;
                }
                for y in (self.cy + 1)..self.rows {
                    for c in self.screen[y].iter_mut() {
                        *c = blank;
                    }
                }
            }
            1 => {
                for y in 0..self.cy {
                    for c in self.screen[y].iter_mut() {
                        *c = blank;
                    }
                }
                for x in 0..=self.cx.min(self.cols - 1) {
                    self.screen[self.cy][x] = blank;
                }
            }
            2 | 3 => {
                for y in 0..self.rows {
                    for c in self.screen[y].iter_mut() {
                        *c = blank;
                    }
                }
                if mode == 3 && !self.in_alt {
                    self.scrollback.clear();
                }
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u32) {
        let blank = self.blank_cell();
        let cx = self.cx;
        let cols = self.cols;
        let line = &mut self.screen[self.cy];
        match mode {
            0 => {
                for x in cx..cols {
                    line[x] = blank;
                }
            }
            1 => {
                for x in 0..=cx.min(cols - 1) {
                    line[x] = blank;
                }
            }
            2 => {
                for c in line.iter_mut() {
                    *c = blank;
                }
            }
            _ => {}
        }
    }

    fn sgr(&mut self) {
        if self.params.is_empty() {
            self.pen = Pen::new();
            return;
        }
        let params = self.params.clone();
        let mut i = 0;
        while i < params.len() {
            let p = params[i];
            match p {
                0 => self.pen = Pen::new(),
                1 => self.pen.attrs |= attr::BOLD,
                2 => self.pen.attrs |= attr::DIM,
                3 => self.pen.attrs |= attr::ITALIC,
                4 => self.pen.attrs |= attr::UNDERLINE,
                7 => self.pen.attrs |= attr::REVERSE,
                8 => self.pen.attrs |= attr::HIDDEN,
                9 => self.pen.attrs |= attr::STRIKE,
                22 => self.pen.attrs &= !(attr::BOLD | attr::DIM),
                23 => self.pen.attrs &= !attr::ITALIC,
                24 => self.pen.attrs &= !attr::UNDERLINE,
                27 => self.pen.attrs &= !attr::REVERSE,
                28 => self.pen.attrs &= !attr::HIDDEN,
                29 => self.pen.attrs &= !attr::STRIKE,
                30..=37 => self.pen.fg = Color::Indexed((p - 30) as u8),
                38 => {
                    if let Some(c) = parse_ext_color(&params, &mut i) {
                        self.pen.fg = c;
                    }
                }
                39 => self.pen.fg = Color::Default,
                40..=47 => self.pen.bg = Color::Indexed((p - 40) as u8),
                48 => {
                    if let Some(c) = parse_ext_color(&params, &mut i) {
                        self.pen.bg = c;
                    }
                }
                49 => self.pen.bg = Color::Default,
                90..=97 => self.pen.fg = Color::Indexed((p - 90 + 8) as u8),
                100..=107 => self.pen.bg = Color::Indexed((p - 100 + 8) as u8),
                _ => {}
            }
            i += 1;
        }
    }

    fn soft_reset(&mut self) {
        self.pen = Pen::new();
        self.top = 0;
        self.bottom = self.rows - 1;
        self.autowrap = true;
        self.cursor_visible = true;
        self.app_cursor = false;
        self.bracketed_paste = false;
        self.wrap_pending = false;
    }

    fn full_reset(&mut self) {
        self.soft_reset();
        if self.in_alt {
            self.leave_alt(false);
        }
        self.screen = blank_grid(self.rows, self.cols);
        self.cx = 0;
        self.cy = 0;
        self.saved_cursor = None;
        self.reset_tabs();
    }
}

fn parse_ext_color(params: &[u32], i: &mut usize) -> Option<Color> {
    match params.get(*i + 1) {
        Some(&5) => {
            let n = *params.get(*i + 2)?;
            *i += 2;
            Some(Color::Indexed(n.min(255) as u8))
        }
        Some(&2) => {
            let r = *params.get(*i + 2)?;
            let g = *params.get(*i + 3)?;
            let b = *params.get(*i + 4)?;
            *i += 4;
            Some(Color::Rgb(r.min(255) as u8, g.min(255) as u8, b.min(255) as u8))
        }
        _ => None,
    }
}

fn line_is_blank(line: &[Cell]) -> bool {
    line.iter()
        .all(|c| c.ch == ' ' && c.bg == Color::Default)
}

// ---------------------------------------------------------------------------
// Tests (run with `cargo test`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn row_text(e: &Emulator, abs: usize) -> String {
        e.line(abs)
            .map(|l| l.iter().map(|c| c.ch).collect::<String>().trim_end().to_string())
            .unwrap_or_default()
    }

    #[test]
    fn prints_and_wraps() {
        let mut e = Emulator::new(5, 3);
        e.feed(b"abcdefg");
        assert_eq!(row_text(&e, 0), "abcde");
        assert_eq!(row_text(&e, 1), "fg");
    }

    #[test]
    fn crlf_and_scroll_into_history() {
        let mut e = Emulator::new(10, 2);
        e.feed(b"one\r\ntwo\r\nthree");
        assert_eq!(e.scrollback_len(), 1);
        assert_eq!(row_text(&e, 0), "one");
        assert_eq!(row_text(&e, 1), "two");
        assert_eq!(row_text(&e, 2), "three");
    }

    #[test]
    fn cursor_movement_and_erase() {
        let mut e = Emulator::new(10, 3);
        e.feed(b"hello\x1b[3D\x1b[K");
        assert_eq!(row_text(&e, 0), "he");
        e.feed(b"\x1b[2;4Hx");
        assert_eq!(e.cursor(), (4, 1));
    }

    #[test]
    fn sgr_colors() {
        let mut e = Emulator::new(10, 2);
        e.feed(b"\x1b[31mR\x1b[0mN\x1b[38;2;1;2;3mT");
        let l = e.line(0).unwrap();
        assert_eq!(l[0].fg, Color::Indexed(1));
        assert_eq!(l[1].fg, Color::Default);
        assert_eq!(l[2].fg, Color::Rgb(1, 2, 3));
    }

    #[test]
    fn utf8_and_responses() {
        let mut e = Emulator::new(10, 2);
        e.feed("héy".as_bytes());
        assert_eq!(row_text(&e, 0), "héy");
        e.feed(b"\x1b[6n");
        assert_eq!(e.take_responses(), b"\x1b[1;4R".to_vec());
    }

    #[test]
    fn alt_screen_roundtrip() {
        let mut e = Emulator::new(10, 2);
        e.feed(b"main");
        e.feed(b"\x1b[?1049h\x1b[HALT");
        assert_eq!(row_text(&e, 0), "ALT");
        e.feed(b"\x1b[?1049l");
        assert_eq!(row_text(&e, 0), "main");
    }

    #[test]
    fn resize_keeps_content() {
        let mut e = Emulator::new(10, 4);
        e.feed(b"a\r\nb\r\nc");
        e.resize(10, 2);
        assert_eq!(e.rows(), 2);
        assert_eq!(row_text(&e, e.scrollback_len() + 1), "c");
        e.resize(10, 4);
        assert_eq!(e.rows(), 4);
    }
}
