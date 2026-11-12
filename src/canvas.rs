//! Canvas: seedha screen par shape banana (egui ke `Painter` ki jagah).
//!
//! ```ignore
//! let c = Canvas(ui);                                   // is ui ki jagah par
//! c.rect(r).fill("#222").stroke(1.0, "#fff").radius(8.0);
//! c.line(a, b).width(2.0).color("#f00");
//! c.circle(p, 6.0).fill(Color::BLUE);
//! c.polygon(&[a, b, c]).fill("#0a0");
//! c.text(p, "hi").size(12.0).bold().color("#fff");     // p = text ka left-top
//! c.image(&img, r).tint("#fff");
//! Canvas::at(ui, rect)                                  // sirf is rect ke andar (clip)
//! ```
//! Har `c.rect(..)` / `c.line(..)` ... jaise hi statement khatam hota hai, apne aap ban jata hai
//! (`.show()` likhne ki zaroorat nahi). Colors `"#rrggbb"` / `"#rrggbbaa"` text ya `Color` dono chalte hain.

use crate::img::Img;
use crate::shadow::ShadowColor;
use crate::theme::Color;
use egui::{epaint::PathStroke, pos2, Align2, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui};

/// `Canvas(ui)`: dekho module docs.
#[allow(non_snake_case)]
pub fn Canvas(ui: &Ui) -> Canvas {
    Canvas { painter: ui.painter().clone() }
}

/// Drawing surface. `Canvas(ui)` ya `Canvas::at(ui, rect)` se banao.
#[derive(Clone)]
pub struct Canvas {
    painter: Painter,
}

impl Canvas {
    /// Sirf `rect` ke andar draw hoga (bahar ka kata jayega).
    pub fn at(ui: &Ui, rect: Rect) -> Canvas {
        Canvas { painter: ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())) }
    }

    /// Rectangle: `.fill() .stroke() .radius()`.
    pub fn rect(&self, rect: Rect) -> RectDraw {
        RectDraw { painter: self.painter.clone(), rect, fill: Color::TRANSPARENT, stroke: Stroke::NONE, radius: 0.0 }
    }

    /// Seedhi line: `.width() .color()`.
    pub fn line(&self, a: Pos2, b: Pos2) -> LineDraw {
        LineDraw { painter: self.painter.clone(), points: vec![a, b], width: 1.0, color: Color::WHITE }
    }

    /// Kai points se judi line (polyline): `.width() .color()`.
    pub fn path(&self, points: &[Pos2]) -> LineDraw {
        LineDraw { painter: self.painter.clone(), points: points.to_vec(), width: 1.0, color: Color::WHITE }
    }

    /// Gol: `.fill() .stroke()`.
    pub fn circle(&self, center: Pos2, radius: f32) -> CircleDraw {
        CircleDraw { painter: self.painter.clone(), center, radius, fill: Color::TRANSPARENT, stroke: Stroke::NONE }
    }

    /// Bhara hua polygon (convex): `.fill() .stroke()`.
    pub fn polygon(&self, points: &[Pos2]) -> PolygonDraw {
        PolygonDraw { painter: self.painter.clone(), points: points.to_vec(), fill: Color::TRANSPARENT, stroke: Stroke::NONE }
    }

    /// Text, `pos` = text ka left-top: `.size() .bold() .mono() .color() .anchor()`.
    pub fn text(&self, pos: Pos2, text: impl Into<String>) -> TextDraw {
        TextDraw { painter: self.painter.clone(), pos, text: text.into(), size: 13.0, bold: false, mono: false, color: Color::WHITE, anchor: Align2::LEFT_TOP }
    }

    /// Tasveer: `.tint()`. (Bytes/file abhi load na hui ho to kuch nahi banta.)
    pub fn image<'a>(&self, img: &'a Img, rect: Rect) -> ImageDraw<'a> {
        ImageDraw { painter: self.painter.clone(), img, rect, tint: Color::WHITE }
    }

    /// Koi bhi purani/khaas egui shape seedha daalna ho to.
    pub fn shape(&self, shape: impl Into<Shape>) {
        self.painter.add(shape);
    }

    /// Ye canvas jis rect tak draw karta hai.
    pub fn clip(&self) -> Rect {
        self.painter.clip_rect()
    }
}

/// `c.rect(..)`.
pub struct RectDraw {
    painter: Painter,
    rect: Rect,
    fill: Color,
    stroke: Stroke,
    radius: f32,
}

impl RectDraw {
    /// Bharne ka color.
    pub fn fill(mut self, color: impl ShadowColor) -> Self {
        self.fill = color.shadow_color();
        self
    }
    /// Kinare ki line (andar ki taraf).
    pub fn stroke(mut self, width: f32, color: impl ShadowColor) -> Self {
        self.stroke = Stroke::new(width, color.shadow_color());
        self
    }
    /// Kone ki golayi.
    pub fn radius(mut self, px: f32) -> Self {
        self.radius = px;
        self
    }
}

impl Drop for RectDraw {
    fn drop(&mut self) {
        let r = self.radius.clamp(0.0, 255.0) as u8;
        if self.fill.a() > 0 {
            self.painter.rect_filled(self.rect, r, self.fill);
        }
        if self.stroke.width > 0.0 && self.stroke.color.a() > 0 {
            self.painter.rect_stroke(self.rect, r, self.stroke, StrokeKind::Inside);
        }
    }
}

/// `c.line(..)` / `c.path(..)`.
pub struct LineDraw {
    painter: Painter,
    points: Vec<Pos2>,
    width: f32,
    color: Color,
}

impl LineDraw {
    /// Moti.
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }
    /// Rang.
    pub fn color(mut self, color: impl ShadowColor) -> Self {
        self.color = color.shadow_color();
        self
    }
}

impl Drop for LineDraw {
    fn drop(&mut self) {
        if self.points.len() >= 2 && self.width > 0.0 && self.color.a() > 0 {
            let stroke = PathStroke::new(self.width, self.color);
            self.painter.add(Shape::Path(egui::epaint::PathShape {
                points: std::mem::take(&mut self.points),
                closed: false,
                fill: Color::TRANSPARENT,
                stroke,
            }));
        }
    }
}

/// `c.circle(..)`.
pub struct CircleDraw {
    painter: Painter,
    center: Pos2,
    radius: f32,
    fill: Color,
    stroke: Stroke,
}

impl CircleDraw {
    /// Bharne ka color.
    pub fn fill(mut self, color: impl ShadowColor) -> Self {
        self.fill = color.shadow_color();
        self
    }
    /// Kinare ki line.
    pub fn stroke(mut self, width: f32, color: impl ShadowColor) -> Self {
        self.stroke = Stroke::new(width, color.shadow_color());
        self
    }
}

impl Drop for CircleDraw {
    fn drop(&mut self) {
        self.painter.circle(self.center, self.radius, self.fill, self.stroke);
    }
}

/// `c.polygon(..)`.
pub struct PolygonDraw {
    painter: Painter,
    points: Vec<Pos2>,
    fill: Color,
    stroke: Stroke,
}

impl PolygonDraw {
    /// Bharne ka color.
    pub fn fill(mut self, color: impl ShadowColor) -> Self {
        self.fill = color.shadow_color();
        self
    }
    /// Kinare ki line.
    pub fn stroke(mut self, width: f32, color: impl ShadowColor) -> Self {
        self.stroke = Stroke::new(width, color.shadow_color());
        self
    }
}

impl Drop for PolygonDraw {
    fn drop(&mut self) {
        if self.points.len() >= 3 {
            self.painter.add(Shape::convex_polygon(std::mem::take(&mut self.points), self.fill, self.stroke));
        }
    }
}

/// `c.text(..)`.
pub struct TextDraw {
    painter: Painter,
    pos: Pos2,
    text: String,
    size: f32,
    bold: bool,
    mono: bool,
    color: Color,
    anchor: Align2,
}

impl TextDraw {
    /// Size px mein.
    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }
    /// Mota.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    /// Barabar chaudai wala font.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
    /// Rang.
    pub fn color(mut self, color: impl ShadowColor) -> Self {
        self.color = color.shadow_color();
        self
    }
    /// `pos` text ke kis kone/beech par ho (default left-top).
    pub fn anchor(mut self, anchor: Align2) -> Self {
        self.anchor = anchor;
        self
    }
    /// Text ko jahan anchor hai wahan ke hisaab se kitna jagah lega.
    pub fn center(self) -> Self {
        self.anchor(Align2::CENTER_CENTER)
    }
}

impl Drop for TextDraw {
    fn drop(&mut self) {
        let font = if self.mono {
            FontId::monospace(self.size)
        } else if self.bold && crate::fonts::has_bold() {
            FontId::new(self.size, crate::fonts::bold_family())
        } else {
            FontId::proportional(self.size)
        };
        self.painter.text(self.pos, self.anchor, std::mem::take(&mut self.text), font, self.color);
    }
}

/// `c.image(..)`.
pub struct ImageDraw<'a> {
    painter: Painter,
    img: &'a Img,
    rect: Rect,
    tint: Color,
}

impl ImageDraw<'_> {
    /// Rang chadhao (white = jaisi hai waisi).
    pub fn tint(mut self, color: impl ShadowColor) -> Self {
        self.tint = color.shadow_color();
        self
    }
}

impl Drop for ImageDraw<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.img.texture_id(self.painter.ctx()) {
            self.painter.image(id, self.rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), self.tint);
        }
    }
}
