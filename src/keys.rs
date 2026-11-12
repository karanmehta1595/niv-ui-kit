//! Keyboard: key aur modifier ka jod, bool mein.
//!
//! ```ignore
//! if Key::Enter.pressed() { }                       // is frame mein dabi
//! if Key::Backspace.held() { }                      // dabi hui hai
//! if Key::S.with(Cmd).with(Shift).pressed() { }     // Cmd+Shift+S
//! if clear.clicked() || Key::Backspace.pressed() { }
//! Button("5").key(Key::Num5).show(ui);              // key dabane par button click
//! ```
//!
//! Modifier **exact** hote hain: `Key::Num5` tab chalta hai jab koi modifier na dabi ho (Shift ke saath nahi),
//! `Key::Num5.with(Shift)` sirf Shift ke saath. `Cmd` Mac par Command, baaki jagah Ctrl hai.
//! `.pressed()` / `.held()` ke liye `mainkit` har frame keyboard yaad rakhta hai (`ui` nahi chahiye).
//! Bina `mainkit` ke ya sirf ek `ui` ke liye `.pressed_in(ui)` / `.held_in(ui)` hain: ye us `ui` se padhte hain
//! aur jab koi text box keyboard pakde hue ho to `false` dete hain.

use egui::{Context, Event, Key, Modifiers, Ui};
use std::cell::RefCell;

/// Modifier key: `Shift`, `Alt`, `Ctrl`, `Cmd`. `Key::S.with(Cmd)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mod {
    /// Shift.
    Shift,
    /// Alt (Mac par Option).
    Alt,
    /// Ctrl (Mac par asli Control; baaki jagah `Cmd` jaisa hi).
    Ctrl,
    /// Command (Mac) / Ctrl (Windows, Linux).
    Cmd,
}

/// Ek key aur uske saath dabe modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyCombo {
    key: Key,
    shift: bool,
    alt: bool,
    ctrl: bool,
    cmd: bool,
}

impl From<Key> for KeyCombo {
    fn from(key: Key) -> Self {
        KeyCombo { key, shift: false, alt: false, ctrl: false, cmd: false }
    }
}

/// Is frame ka keyboard (`begin_frame` bharta hai).
#[derive(Default)]
struct KeyFrame {
    pressed: Vec<(Key, Modifiers)>,
    down: Vec<Key>,
    modifiers: Modifiers,
    text_focus: bool,
}

thread_local! {
    static FRAME: RefCell<KeyFrame> = RefCell::new(KeyFrame::default());
}

/// Har frame ke shuru mein ek baar (`mainkit` ye khud karta hai): `.pressed()` / `.held()` ke liye keyboard yaad rakho.
pub fn begin_frame(ctx: &Context) {
    let mut frame = KeyFrame::default();
    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key { key, pressed: true, modifiers, .. } = event {
                frame.pressed.push((*key, *modifiers));
            }
        }
        frame.down = i.keys_down.iter().copied().collect();
        frame.modifiers = i.modifiers;
    });
    frame.text_focus = ctx.egui_wants_keyboard_input();
    FRAME.with(|f| *f.borrow_mut() = frame);
}

impl KeyCombo {
    /// Ek aur modifier jodo: `Key::S.with(Cmd).with(Shift)`.
    pub fn with(mut self, modifier: Mod) -> Self {
        match modifier {
            Mod::Shift => self.shift = true,
            Mod::Alt => self.alt = true,
            Mod::Ctrl => self.ctrl = true,
            Mod::Cmd => self.cmd = true,
        }
        self
    }

    /// Dabe hue modifiers theek wahi hain jo is combo ne maange (na kam, na zyada)?
    fn matches(&self, m: &Modifiers) -> bool {
        let mac = cfg!(target_os = "macos");
        let (want_cmd, want_ctrl) = if mac { (self.cmd, self.ctrl) } else { (self.cmd || self.ctrl, false) };
        let have_ctrl = mac && m.ctrl;
        m.shift == self.shift && m.alt == self.alt && m.command == want_cmd && have_ctrl == want_ctrl
    }

    /// Is frame mein dabi (`mainkit` app mein, bina `ui` ke).
    pub fn pressed(self) -> bool {
        FRAME.with(|f| {
            let f = f.borrow();
            !f.text_focus && f.pressed.iter().any(|(key, m)| *key == self.key && self.matches(m))
        })
    }

    /// Dabi hui hai (`mainkit` app mein, bina `ui` ke).
    pub fn held(self) -> bool {
        FRAME.with(|f| {
            let f = f.borrow();
            !f.text_focus && f.down.contains(&self.key) && self.matches(&f.modifiers)
        })
    }

    /// Is frame mein dabi, `ui` se padhi (text box keyboard pakde ho to `false`).
    pub fn pressed_in(self, ui: &Ui) -> bool {
        !ui.ctx().egui_wants_keyboard_input()
            && ui.input(|i| {
                i.events.iter().any(|event| {
                    matches!(event, Event::Key { key, pressed: true, modifiers, .. }
                        if *key == self.key && self.matches(modifiers))
                })
            })
    }

    /// Dabi hui hai, `ui` se padhi (text box keyboard pakde ho to `false`).
    pub fn held_in(self, ui: &Ui) -> bool {
        !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_down(self.key) && self.matches(&i.modifiers))
    }
}

/// `Key` par seedha `.with(..)`, `.pressed()`, `.held()`: `Key::Enter.pressed()`, `Key::S.with(Cmd).pressed()`.
pub trait KeyExt {
    /// Modifier jodo.
    fn with(self, modifier: Mod) -> KeyCombo;
    /// Is frame mein dabi (bina `ui`, `mainkit` app mein).
    fn pressed(self) -> bool;
    /// Dabi hui hai (bina `ui`, `mainkit` app mein).
    fn held(self) -> bool;
    /// Is frame mein dabi, `ui` se padhi.
    fn pressed_in(self, ui: &Ui) -> bool;
    /// Dabi hui hai, `ui` se padhi.
    fn held_in(self, ui: &Ui) -> bool;
}

impl KeyExt for Key {
    fn with(self, modifier: Mod) -> KeyCombo {
        KeyCombo::from(self).with(modifier)
    }
    fn pressed(self) -> bool {
        KeyCombo::from(self).pressed()
    }
    fn held(self) -> bool {
        KeyCombo::from(self).held()
    }
    fn pressed_in(self, ui: &Ui) -> bool {
        KeyCombo::from(self).pressed_in(ui)
    }
    fn held_in(self, ui: &Ui) -> bool {
        KeyCombo::from(self).held_in(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(shift: bool, command: bool) -> Modifiers {
        Modifiers { shift, command, mac_cmd: command && cfg!(target_os = "macos"), ctrl: command && !cfg!(target_os = "macos"), ..Modifiers::NONE }
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let five = KeyCombo::from(Key::Num5);
        assert!(five.matches(&Modifiers::NONE));
        assert!(!five.matches(&mods(true, false)), "Shift dabi ho to plain 5 nahi");
        let percent = five.with(Mod::Shift);
        assert!(percent.matches(&mods(true, false)));
        assert!(!percent.matches(&Modifiers::NONE));
        let save = KeyCombo::from(Key::S).with(Mod::Cmd).with(Mod::Shift);
        assert!(save.matches(&mods(true, true)));
        assert!(!save.matches(&mods(false, true)));
    }
}
