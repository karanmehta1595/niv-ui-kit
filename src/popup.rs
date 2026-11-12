//! Popup aur Tip: floating cheezein.
//!
//! ```ignore
//! // Tip: mouse upar rakhne par chhota text (bold, glass jaisa)
//! Tip("Naya folder").on(button_response);
//!
//! // Popup: kisi jagah par bina style ka floating box (completion list, context menu ke liye)
//! Popup("complete").at(pos).width(260.0).show(ui, &mut open, |ui| { /* rows */ });
//! ```
//! `Popup` par koi background nahi hota: andar `Card` / `Panel` / `Button` jo chaho wo lagao.
//! `open` ko bahar click ya `Esc` band kar deta hai (`.keep_open()` likho to nahi).

use crate::text::UiExt;
use egui::{Area, Id, Key, Order, Pos2, Response, Sense, Ui, UiBuilder};

/// `Tip("text")`.
#[allow(non_snake_case)]
pub fn Tip(text: impl Into<String>) -> Tip {
    Tip { text: text.into() }
}

/// Hover par chhota text.
pub struct Tip {
    text: String,
}

impl Tip {
    /// `response` par mouse rakhne par dikhao; wahi response wapas milta hai.
    pub fn on(self, response: Response) -> Response {
        let text = self.text;
        response.on_hover_ui(move |ui| {
            ui.text(text).size(12.0).bold();
        })
    }
}

/// `Popup("id")`.
#[allow(non_snake_case)]
pub fn Popup(id: impl Into<String>) -> Popup {
    Popup { id: id.into(), at: Pos2::ZERO, width: None, keep_open: false }
}

/// Floating, bina background ka box.
#[must_use = "aakhir mein .show(ui, &mut open, ..) likho"]
pub struct Popup {
    id: String,
    at: Pos2,
    width: Option<f32>,
    keep_open: bool,
}

impl Popup {
    /// Top-left kona yahan.
    pub fn at(mut self, pos: Pos2) -> Self {
        self.at = pos;
        self
    }
    /// Chaudai (px).
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }
    /// Bahar click / Esc par band na karo.
    pub fn keep_open(mut self) -> Self {
        self.keep_open = true;
        self
    }
    /// `open` true ho tab dikhao. Content ka rect aur `add` ka natija milta hai; band ho to `None`.
    pub fn show<R>(self, ui: &Ui, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        if !*open {
            return None;
        }
        let ctx = ui.ctx();
        let id = Id::unique(("niv_popup", self.id.as_str()));
        let width = self.width;
        let out = Area::new(id)
            .order(Order::Foreground)
            .fixed_pos(self.at)
            .sense(Sense::hover())
            .show(ctx, |ui| {
                if let Some(w) = width {
                    ui.set_width(w);
                }
                ui.scope_builder(UiBuilder::new(), add).inner
            });
        if !self.keep_open {
            let clicked_outside = ctx.input(|i| i.pointer.any_pressed())
                && !out.response.rect.contains(ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::ZERO));
            if clicked_outside || ui.input(|i| i.key_pressed(Key::Escape)) {
                *open = false;
            }
        }
        Some(out.inner)
    }
}
