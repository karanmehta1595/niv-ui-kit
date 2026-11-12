//! Card: halka sa rounded dabba jisme cheezein rakhte hain (command, notice, list ka item).
//!
//! ```ignore
//! Card().show(ui, |ui| {
//!     ui.text("Title").bold();
//!     ui.text("kuch likha hai").opacity(0.6);
//! });
//! Card().border(rgb(229, 107, 107)).padding(12.0).show(ui, |ui| { /* laal kinare wala notice */ });
//! ```
//! Chaudai poori bachi hui jagah ki hoti hai.

use crate::theme::Color;
use egui::{Frame, InnerResponse, Margin, Stroke, Ui};

/// `Card()`: dekho module docs.
#[allow(non_snake_case)]
pub fn Card() -> Card {
    Card { padding: 10.0, radius: 10.0, alpha: 14, border: None, shadow: None, shadow_opacity: None, shadow_color: None }
}

/// Dabba, `Card()` se banao.
#[must_use = "Card ke aakhir mein .show(ui, |ui| ..) likho"]
pub struct Card {
    padding: f32,
    radius: f32,
    alpha: u8,
    border: Option<Color>,
    shadow: Option<crate::shadow::ShadowSpec>,
    shadow_opacity: Option<f32>,
    shadow_color: Option<Color>,
}

impl Card {
    /// Kinare se andar tak ki jagah (px, default 10).
    pub fn padding(mut self, px: f32) -> Self {
        self.padding = px;
        self
    }

    /// Kone ki golai (default 10).
    pub fn radius(mut self, px: f32) -> Self {
        self.radius = px;
        self
    }

    /// Bharav kitna gehra (0 se 100 percent, default ~5).
    pub fn fill_opacity(mut self, percent: u8) -> Self {
        self.alpha = (percent.min(100) as f32 * 2.55) as u8;
        self
    }

    /// Rangeen kinara (patla).
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// Parchhai: `.shadow(drop(0.0, 6.0))` ya `.shadow(drop(0.0, 6.0).inner(0.0, 2.0))` (dekho `niv_ui_kit::shadow`), ya purana
    /// `.shadow(10.0)`.
    pub fn shadow(mut self, shadow: impl Into<crate::shadow::ShadowSpec>) -> Self {
        self.shadow = Some(shadow.into());
        self
    }

    /// Parchhai kitni gaadhi, 0 se 100 percent.
    pub fn shadow_opacity(mut self, percent: f32) -> Self {
        self.shadow_opacity = Some(percent.clamp(0.0, 100.0));
        self
    }

    /// Parchhai ka rang (`"#1c1c1e"` ya `Color`).
    pub fn shadow_color(mut self, color: impl crate::shadow::ShadowColor) -> Self {
        self.shadow_color = Some(color.shadow_color());
        self
    }

    /// Dikhao.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let dark = ui.visuals().dark_mode;
        let ink = if dark { Color::WHITE } else { Color::BLACK };
        let fill = Color::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), self.alpha);
        let stroke = match self.border {
            Some(c) => Stroke::new(1.0, c),
            None => Stroke::new(0.5, Color::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 40)),
        };
        let corner = egui::CornerRadius::same(self.radius.round().clamp(0.0, 255.0) as u8);
        let shadow = self.shadow.as_ref().map(|spec| spec.styled(self.shadow_opacity, self.shadow_color));
        // places in the paint order, filled when the size is known: the outside shadow under the card, the inside one over its colour
        let outside = shadow.as_ref().map(|_| ui.painter().add(egui::Shape::Noop));
        let mut prepared = Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(self.radius)
            .inner_margin(Margin::same(self.padding as i8))
            .begin(ui);
        let inside = shadow.as_ref().map(|_| ui.painter().add(egui::Shape::Noop));
        let inner = {
            let content = &mut prepared.content_ui;
            content.set_width(content.available_width());
            add(content)
        };
        let response = prepared.end(ui);
        if let (Some(spec), Some(outside), Some(inside)) = (&shadow, outside, inside) {
            let rect = response.rect;
            ui.painter().set(outside, egui::Shape::Vec(spec.outer(rect, corner)));
            ui.painter().with_clip_rect(rect).set(inside, egui::Shape::Vec(spec.inside(rect, corner)));
        }
        InnerResponse::new(inner, response)
    }
}
