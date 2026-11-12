//! Input: mouse, text, scroll, files (egui ke `ctx.input` ki jagah).
//!
//! ```ignore
//! let i = Input(ui);
//! let typed: String = i.typed();            // is frame mein jo likha gaya
//! let wheel: Vec2 = i.scroll();             // trackpad/mouse scroll (px)
//! let pinch: f32 = i.pinch();               // 1.0 = koi zoom nahi
//! if let Some(p) = i.pointer() { }          // mouse kahan hai
//! if i.shift() || i.cmd() { }               // modifiers dabe hain
//! for path in i.dropped() { }               // file chhodi gayi
//! if let Some(text) = i.pasted() { }        // Cmd+V
//! let dt = i.dt();                          // pichle frame se seconds
//! ```
//! Keyboard shortcut ke liye `Key::S.with(Cmd).pressed()` (module `keys`) hai.

use egui::{Event, Pos2, Ui, Vec2};
use std::path::PathBuf;

/// Is frame ka input, `Input(ui)` se lo.
#[allow(non_snake_case)]
pub fn Input(ui: &Ui) -> Input {
    ui.input(|i| Input {
        typed: i
            .events
            .iter()
            .filter_map(|e| if let Event::Text(t) = e { Some(t.as_str()) } else { None })
            .collect(),
        pasted: i.events.iter().find_map(|e| if let Event::Paste(t) = e { Some(t.clone()) } else { None }),
        scroll: i.smooth_scroll_delta,
        pinch: i.zoom_delta(),
        pointer: i.pointer.hover_pos(),
        moved: i.pointer.delta(),
        dropped: i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect(),
        shift: i.modifiers.shift,
        alt: i.modifiers.alt,
        cmd: i.modifiers.command,
        dt: i.stable_dt,
        time: i.time,
        focused: i.focused,
        down: i.pointer.primary_down(),
    })
}

/// `Input(ui)` ka natija.
#[derive(Clone, Debug, Default)]
pub struct Input {
    typed: String,
    pasted: Option<String>,
    scroll: Vec2,
    pinch: f32,
    pointer: Option<Pos2>,
    moved: Vec2,
    dropped: Vec<PathBuf>,
    shift: bool,
    alt: bool,
    cmd: bool,
    dt: f32,
    time: f64,
    focused: bool,
    down: bool,
}

impl Input {
    /// Is frame mein likha hua text.
    pub fn typed(&self) -> &str {
        &self.typed
    }
    /// Cmd+V se aaya text.
    pub fn pasted(&self) -> Option<&str> {
        self.pasted.as_deref()
    }
    /// Scroll (px), smooth.
    pub fn scroll(&self) -> Vec2 {
        self.scroll
    }
    /// Pinch / Cmd+scroll zoom: 1.0 matlab koi badlav nahi.
    pub fn pinch(&self) -> f32 {
        self.pinch
    }
    /// Mouse ki jagah (window ke andar ho to).
    pub fn pointer(&self) -> Option<Pos2> {
        self.pointer
    }
    /// Is frame mein mouse kitna hila.
    pub fn moved(&self) -> Vec2 {
        self.moved
    }
    /// Mouse ka left button dabaa hai.
    pub fn mouse_down(&self) -> bool {
        self.down
    }
    /// Chhodi gayi files.
    pub fn dropped(&self) -> &[PathBuf] {
        &self.dropped
    }
    /// Shift dabi hai.
    pub fn shift(&self) -> bool {
        self.shift
    }
    /// Alt (Mac par Option) dabi hai.
    pub fn alt(&self) -> bool {
        self.alt
    }
    /// Cmd (Mac) / Ctrl dabi hai.
    pub fn cmd(&self) -> bool {
        self.cmd
    }
    /// Pichle frame se kitne seconds.
    pub fn dt(&self) -> f32 {
        self.dt
    }
    /// App chalu hone ke baad ke seconds.
    pub fn time(&self) -> f64 {
        self.time
    }
    /// Window focus mein hai.
    pub fn focused(&self) -> bool {
        self.focused
    }
}
