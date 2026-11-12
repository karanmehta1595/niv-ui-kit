//! TextField: ek line ka text likhne ka dabba (search box, naam, ...).
//!
//! ```ignore
//! let r = TextField(&mut self.query).hint("Search").show(ui);
//! if r.changed() { /* likha gaya */ }
//! TextField(&mut name).width(240.0).focus().show(ui); // `.focus()`: pehli baar cursor yahin
//! ```
//! Enter dabane par `response.lost_focus()` true hota hai (`ui.input(|i| i.key_pressed(Key::Enter))` se pakdo).

use crate::theme::{rgb, Color};
use egui::{vec2, Align2, ScrollArea, FontId, Frame, Id, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder};

/// `TextField(&mut String)`: dekho module docs.
#[allow(non_snake_case)]
pub fn TextField(text: &mut String) -> TextField<'_> {
    TextField {
        text,
        hint: String::new(),
        width: None,
        height: 32.0,
        radius: 10.0,
        size: 13.0,
        accent: rgb(10, 132, 255),
        id: None,
        focus: false,
        multiline: false,
    }
}

/// Ek line ka text box, `TextField(&mut String)` se banao.
#[must_use = "TextField ke aakhir mein .show(ui) likho"]
pub struct TextField<'a> {
    text: &'a mut String,
    hint: String,
    width: Option<f32>,
    height: f32,
    radius: f32,
    size: f32,
    accent: Color,
    id: Option<Id>,
    focus: bool,
    multiline: bool,
}

impl TextField<'_> {
    /// Khali hone par dikhne wala halka text.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// Chaudai (default: bachi hui poori chaudai).
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// Unchai (default 32).
    pub fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    /// Kone kitne gol (default 10).
    pub fn radius(mut self, px: f32) -> Self {
        self.radius = px;
        self
    }

    /// Text ka size (default 13).
    pub fn text_size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }

    /// Cursor ka / focus ke gher ka rang (default neela).
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    /// Ek hi jagah ek se zyada TextField ho to har ek ko alag `id` do.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(Id::unique(id.into()));
        self
    }

    /// Kai lines ka dabba: Enter nayi line deta hai, text upar se shuru hota hai (`.height(..)` se unchai do).
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Cursor isi dabbe mein rakho (jab ye dikhna shuru ho, tab ek baar).
    pub fn focus(mut self) -> Self {
        self.focus = true;
        self
    }

    /// Dikhao. `response.changed()` = text badla, `lost_focus()` = Enter / bahar click.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width()).max(40.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, self.height), Sense::hover());
        let dark = ui.visuals().dark_mode;
        let ink = if dark { Color::WHITE } else { Color::BLACK };
        let fill = Color::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), if dark { 20 } else { 14 });
        let pad_y = if self.multiline { 6.0 } else { 0.0 };
        let inner = Rect::from_min_max(rect.min + vec2(10.0, pad_y), rect.max - vec2(10.0, pad_y));
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let edit = if self.multiline { TextEdit::multiline(self.text) } else { TextEdit::singleline(self.text) };
        let edit = edit
            .id(id)
            .frame(Frame::NONE)
            .font(FontId::proportional(self.size))
            .text_color(ink)
            .hint_text(egui::RichText::new(self.hint).color(Color::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 110)))
            .align(if self.multiline { Align2::LEFT_TOP } else { Align2::LEFT_CENTER })
            .min_size(inner.size())
            .desired_width(inner.width());
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        // Kai lines ka dabba: lamba text dabbe ke andar hi rehta hai aur scroll hota hai (bahar nahi nikalta).
        let response = if self.multiline {
            ScrollArea::vertical().id_salt(id.with("niv_scroll")).auto_shrink([false, false]).max_height(inner.height()).show(&mut child, |ui| ui.add(edit)).inner
        } else {
            child.add_sized(inner.size(), edit)
        };
        if self.focus {
            response.request_focus();
        }
        // Dabbe mein kahin bhi click ho, text mein cursor aa jaye.
        let hit = ui.interact(rect, id.with("niv_field"), Sense::click());
        if hit.clicked() && !response.has_focus() {
            response.request_focus();
        }
        let focused = response.has_focus();
        ui.painter().rect_filled(rect, self.radius, fill);
        if focused {
            ui.painter().rect_stroke(rect, self.radius, Stroke::new(1.5, self.accent), StrokeKind::Inside);
        }
        if hit.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        response
    }
}
