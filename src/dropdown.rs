//! Dropdown: kisi button ke neeche ya screen ke upar-beech mein khulne wala liquid-glass card.
//!
//! ```ignore
//! if search.clicked() { self.search_open = !self.search_open; }
//! Dropdown("search").top_center(50.0).size(vec2(480.0, 340.0)).ignore(search.rect)
//!     .show(ui, &mut self.search_open, |ui| { /* andar ka content */ });
//! ```
//! Bahar click ya `Esc` par band ho jata hai (`open` false). `.ignore(rect)`: us rect (jaise kholne wala
//! button) par click ko "bahar" na maano, warna button se band-khol ek saath ho jayega.

use crate::button::{Button, IntoSize};
use crate::paint::Glow;
use crate::panel::inverse;
use crate::theme::{light, rgb, rgba};
use egui::{pos2, vec2, Id, Key, Layout, Order, Rect, Sense, Ui, UiBuilder, Vec2};

/// `Dropdown("id")`: dekho module docs.
#[allow(non_snake_case)]
pub fn Dropdown(id: impl Into<String>) -> Dropdown {
    Dropdown {
        id: id.into(),
        size: vec2(420.0, 320.0),
        place: Place::TopCenter(46.0),
        ignore: Vec::new(),
        padding: 10.0,
        frame: None,
        min_margin: 8.0,
    }
}

enum Place {
    TopCenter(f32),
    At(egui::Pos2),
}

/// Dropdown card, `Dropdown("id")` se banao.
#[must_use = "Dropdown ke aakhir mein .show(ui, &mut open, ..) likho"]
pub struct Dropdown {
    id: String,
    size: Vec2,
    place: Place,
    ignore: Vec<Rect>,
    padding: f32,
    frame: Option<Button>,
    min_margin: f32,
}

impl Dropdown {
    /// Card ka size (px): `vec2(480.0, 340.0)`.
    pub fn size(mut self, size: impl IntoSize) -> Self {
        self.size = size.into_size();
        self
    }

    /// Window ke upar ke beech mein, upar se `y` px neeche (default 46: top bar ke neeche).
    pub fn top_center(mut self, y: f32) -> Self {
        self.place = Place::TopCenter(y);
        self
    }

    /// Card ka top-left kona seedha yahan.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.place = Place::At(pos2(x, y));
        self
    }

    /// Is rect par click ko bahar ka click na maano (kholne wale button ka rect). Kai baar likh sakte ho.
    pub fn ignore(mut self, rect: Rect) -> Self {
        self.ignore.push(rect);
        self
    }

    /// Card ke kinare se andar ke content tak khali jagah (default 10).
    pub fn padding(mut self, px: f32) -> Self {
        self.padding = px.max(0.0);
        self
    }

    /// Card ka apna roop (Button ka rang, blur, border, radius ...). Na likho to liquid glass.
    pub fn frame(mut self, frame: Button) -> Self {
        self.frame = Some(frame);
        self
    }

    pub(crate) fn default_frame(dark: bool) -> Button {
        Button(None)
            // light mein safed (kaala nahi), dark mein halka dhundhla grey
            .fill(light(rgb(255, 255, 255)).dark(rgb(70, 70, 74)))
            .fill_opacity(55)
            // dark theme: a light black tint, darker towards the bottom, so white text stays readable on the glass.
            // light theme: only a whisper of it (a stronger black tint makes the card look like a black box behind black text)
            .gradient(rgba(0, 0, 0, if dark { 18 } else { 0 }), rgba(0, 0, 0, if dark { 78 } else { 10 }))
            .blur(40)
            .radius(18.0)
            .border_width(0.4)
            .border_glow(1.0)
            .border_color(inverse)
            .border_glow_at(Glow::Middle)
            .border_glow_intensity(1.0)
            .shadow(24.0)
    }

    /// `open` true ho tab card dikhao; `add` andar ka content bharta hai. Band hone par `None`.
    pub fn show<R>(self, ui: &Ui, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        self.show_ctx(ui.ctx(), open, add)
    }

    /// Jaise `.show(..)`, par `Ui` ki jagah seedha `Context` (jahan `Ui` haath mein na ho).
    pub fn show_ctx<R>(self, ctx: &egui::Context, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        if !*open {
            return None;
        }
        let ctx = ctx.clone();
        let screen = ctx.content_rect();
        let pos = match self.place {
            Place::TopCenter(y) => pos2(screen.center().x - self.size.x / 2.0, screen.min.y + y),
            Place::At(p) => p,
        };
        let min = pos2(
            pos.x.clamp(screen.min.x + self.min_margin, (screen.max.x - self.size.x - self.min_margin).max(screen.min.x)),
            pos.y.max(screen.min.y),
        );
        let rect = Rect::from_min_size(min, self.size);
        let id = Id::unique(("niv_dropdown", self.id.as_str()));
        let dark = ctx.global_style().visuals.dark_mode;
        let frame = self.frame.unwrap_or_else(|| Self::default_frame(dark)).size(self.size);
        let padding = self.padding;
        let mut result = None;
        egui::Area::new(id).order(Order::Foreground).fixed_pos(rect.min).show(&ctx, |area_ui| {
            let (card, _) = area_ui.allocate_exact_size(self.size, Sense::hover());
            let measured = frame.measure(area_ui);
            let _ = frame.paint(area_ui, id.with("frame"), card, &measured);
            let mut inner = area_ui.new_child(
                UiBuilder::new().max_rect(card.shrink(padding)).layout(Layout::top_down(egui::Align::Min)),
            );
            inner.set_clip_rect(card.shrink(padding / 2.0).intersect(inner.clip_rect()));
            result = Some(add(&mut inner));
        });
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer.interact_pos().is_some_and(|p| {
                    !rect.contains(p) && !self.ignore.iter().any(|r| r.contains(p))
                })
        });
        if escape || outside {
            *open = false;
        }
        result
    }
}
