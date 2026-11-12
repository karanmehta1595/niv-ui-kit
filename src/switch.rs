//! Switch: on/off ka gol-goli wala button (jaise iPhone settings mein).
//!
//! ```ignore
//! if Switch(&mut self.dark_mode).show(ui).changed() { /* badal gaya */ }
//! Switch(&mut on).size(vec2(40.0, 22.0)).accent(rgb(52, 199, 89)).show(ui);
//! ```
//! Apple jaisa: neeli patti par `|` (on), dhusar patti par `O` (off).
//! Click karne par `value` ulta ho jata hai aur `response.changed()` true milta hai.

use crate::theme::{rgb, Color};
use egui::{vec2, Id, Response, Sense, Ui, Vec2};

/// `Switch(&mut bool)`: dekho module docs.
#[allow(non_snake_case)]
pub fn Switch(value: &mut bool) -> Switch<'_> {
    Switch { value, size: vec2(36.0, 20.0), accent: rgb(10, 132, 255), id: None }
}

/// On/off switch, `Switch(&mut bool)` se banao.
#[must_use = "Switch ke aakhir mein .show(ui) likho"]
pub struct Switch<'a> {
    value: &'a mut bool,
    size: Vec2,
    accent: Color,
    id: Option<Id>,
}

impl Switch<'_> {
    /// Chaudai x unchai (default 36 x 20).
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// On hone par patti ka rang (default neela).
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    /// Ek hi jagah ek se zyada switch ho to har ek ko alag `id` do.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(Id::unique(id.into()));
        self
    }

    /// Dikhao. `response.changed()` = ulta hua.
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = ui.allocate_exact_size(self.size, Sense::click());
        let id = self.id.unwrap_or(response.id);
        if response.clicked() {
            *self.value = !*self.value;
            response.mark_changed();
        }
        if ui.is_rect_visible(rect) {
            let dark = ui.visuals().dark_mode;
            let t = ui.ctx().animate_bool_with_time(id.with("niv_switch"), *self.value, 0.12);
            let ink = if dark { Color::WHITE } else { Color::BLACK };
            let off = Color::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), if dark { 60 } else { 45 });
            let track = off.lerp_to_gamma(self.accent, t);
            let r = rect.height() / 2.0;
            ui.painter().rect_filled(rect, r, track);
            // Apple's on/off marks (Accessibility > Display > On/Off Labels): a thin bar "|" shows on the blue track when it is on,
            // a ring "O" on the grey track when it is off; each one sits on the side the knob has left.
            let h = rect.height();
            let bar = egui::Stroke::new((h * 0.09).max(1.0), Color::WHITE.gamma_multiply(t));
            let on_at = egui::pos2(rect.min.x + r * 0.95, rect.center().y);
            ui.painter().line_segment([on_at - vec2(0.0, h * 0.2), on_at + vec2(0.0, h * 0.2)], bar);
            let ring = egui::Stroke::new((h * 0.09).max(1.0), ink.gamma_multiply(0.55 * (1.0 - t)));
            ui.painter().circle_stroke(egui::pos2(rect.max.x - r * 0.95, rect.center().y), h * 0.19, ring);
            let pad = 2.0;
            let knob_r = r - pad;
            let x = egui::lerp((rect.min.x + r)..=(rect.max.x - r), t);
            let center = egui::pos2(x, rect.center().y);
            ui.painter().circle_filled(center + vec2(0.0, 0.8), knob_r, Color::from_black_alpha(40));
            ui.painter().circle_filled(center, knob_r, Color::WHITE);
        }
        response
    }
}

