//! Shadows for everything: `.shadow(drop(x, y))` on Button, Image, Group, Panel and Card.
//!
//! ```ignore
//! Button("Save").shadow(drop(0.0, 6.0))                                        // a soft shadow below
//! Button("Save").shadow(drop(0.0, 6.0).blur(18.0).color("#1c1c1e").opacity(45.0))
//! Card().shadow(drop(0.0, 8.0).inner(0.0, 2.0))                                // outside and inside
//! Image("assets/folder.svg").size(48.0).shadow(drop(0.0, 6.0))                // the shape of the image itself
//! ```
//! The old `.shadow(10.0)` (a number of pixels) still works.
//!
//! Ranges (everything is in points):
//! * `drop(x, y)` and `.inner(x, y)`: `x` right, `y` down, each -100.0 to 100.0 (negative goes left / up).
//! * `.blur(n)`: softness, 0.0 to 100.0. Not written: it follows the distance (about twice the larger of x and y, 8 at least).
//! * `.color("#rrggbb")` (or any `Color`): default black.
//! * `.opacity(n)`: percent, 0.0 (invisible) to 100.0 (solid), default 35.
//!
//! `.blur`, `.color` and `.opacity` change the layer that was written last: `drop(0.0, 6.0).inner(0.0, 2.0).blur(4.0)` blurs
//! the inner one only. An Image has the drop shadow only (the inner one is left out), a web address is drawn without a shadow.
//! A Panel inside a window shows its drop shadow only where nothing is painted over it by its neighbours.

use crate::theme::Color;
use egui::{epaint::Shadow as EguiShadow, vec2, CornerRadius, Rect, Shape, Stroke, StrokeKind};

/// One layer of a shadow.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    x: f32,
    y: f32,
    blur: Option<f32>,
    color: Color,
    opacity: f32,
}

impl Layer {
    fn new(x: f32, y: f32) -> Self {
        Layer { x: x.clamp(-100.0, 100.0), y: y.clamp(-100.0, 100.0), blur: None, color: Color::BLACK, opacity: 35.0 }
    }

    /// Softness in points: written, or from the distance.
    fn softness(&self) -> f32 {
        self.blur.unwrap_or_else(|| (self.x.abs().max(self.y.abs()) * 2.0).max(8.0)).clamp(0.0, 100.0)
    }

    /// The colour with the opacity in it.
    fn tint(&self) -> Color {
        self.color.gamma_multiply(self.opacity.clamp(0.0, 100.0) / 100.0)
    }
}

/// `drop(x, y)`: see the module docs.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadow {
    drop: Layer,
    inner: Option<Layer>,
    /// `.blur` / `.color` / `.opacity` go to the inner layer once it is written.
    on_inner: bool,
}

/// A shadow below (and to the side of) the thing: `drop(0.0, 6.0)`.
pub fn drop(x: f32, y: f32) -> Shadow {
    Shadow { drop: Layer::new(x, y), inner: None, on_inner: false }
}

/// A colour for a shadow: `"#1c1c1e"` or a `Color`.
pub trait ShadowColor {
    /// The colour (black when the text is not a hex colour).
    fn shadow_color(self) -> Color;
}

impl ShadowColor for &str {
    fn shadow_color(self) -> Color {
        Color::from_hex(self).unwrap_or(Color::BLACK)
    }
}

impl ShadowColor for Color {
    fn shadow_color(self) -> Color {
        self
    }
}

impl Shadow {
    fn layer(&mut self) -> &mut Layer {
        match (&mut self.inner, self.on_inner) {
            (Some(inner), true) => inner,
            _ => &mut self.drop,
        }
    }

    /// A shadow inside too: the edges inside get darker, `.inner(0.0, 2.0)`.
    pub fn inner(mut self, x: f32, y: f32) -> Self {
        self.inner = Some(Layer::new(x, y));
        self.on_inner = true;
        self
    }

    /// Softness, 0.0 to 100.0.
    pub fn blur(mut self, blur: f32) -> Self {
        self.layer().blur = Some(blur.clamp(0.0, 100.0));
        self
    }

    /// Colour: `"#1c1c1e"` or a `Color`. Default black.
    pub fn color(mut self, color: impl ShadowColor) -> Self {
        self.layer().color = color.shadow_color();
        self
    }

    /// Opacity in percent, 0.0 to 100.0 (default 35).
    pub fn opacity(mut self, percent: f32) -> Self {
        self.layer().opacity = percent.clamp(0.0, 100.0);
        self
    }
}

/// What `.shadow(..)` takes: a number of pixels (the old way) or `drop(..)`.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowSpec {
    /// `.shadow(10.0)`.
    Px(f32),
    /// `.shadow(drop(0.0, 6.0))`.
    Layers(Shadow),
}

impl From<f32> for ShadowSpec {
    fn from(px: f32) -> Self {
        ShadowSpec::Px(px.max(0.0))
    }
}

impl From<Shadow> for ShadowSpec {
    fn from(shadow: Shadow) -> Self {
        ShadowSpec::Layers(shadow)
    }
}

impl ShadowSpec {
    /// The same shadow with a widget's own `.shadow_opacity(..)` / `.shadow_color(..)` put over every layer of it.
    pub(crate) fn styled(&self, opacity: Option<f32>, color: Option<Color>) -> ShadowSpec {
        if opacity.is_none() && color.is_none() {
            return self.clone();
        }
        let mut shadow = match self {
            ShadowSpec::Px(px) => drop(0.0, px / 3.0).blur(*px).opacity(31.0),
            ShadowSpec::Layers(shadow) => shadow.clone(),
        };
        for layer in std::iter::once(&mut shadow.drop).chain(shadow.inner.as_mut()) {
            if let Some(opacity) = opacity {
                layer.opacity = opacity.clamp(0.0, 100.0);
            }
            if let Some(color) = color {
                layer.color = color;
            }
        }
        ShadowSpec::Layers(shadow)
    }

    /// How far it spreads (for the hover change of a number-of-pixels shadow).
    pub(crate) fn px(&self) -> f32 {
        match self {
            ShadowSpec::Px(px) => *px,
            ShadowSpec::Layers(shadow) => shadow.drop.softness(),
        }
    }

    /// The drop layer as `(x, y, blur, colour with opacity)`, for an image (whose shadow is made from its own shape).
    pub(crate) fn for_image(&self) -> (f32, f32, f32, Color) {
        match self {
            ShadowSpec::Px(px) => (0.0, px / 3.0, *px, Color::from_black_alpha(80)),
            ShadowSpec::Layers(shadow) => (shadow.drop.x, shadow.drop.y, shadow.drop.softness(), shadow.drop.tint()),
        }
    }

    /// The shadow outside the rect, to paint *under* the thing.
    pub(crate) fn outer(&self, rect: Rect, corner: CornerRadius) -> Vec<Shape> {
        let (offset, blur, color) = match self {
            ShadowSpec::Px(px) => ([0.0, px / 3.0], *px, Color::from_black_alpha(80)),
            ShadowSpec::Layers(shadow) => ([shadow.drop.x, shadow.drop.y], shadow.drop.softness(), shadow.drop.tint()),
        };
        let shadow = EguiShadow {
            offset: offset.map(|v| v.round().clamp(-127.0, 127.0) as i8),
            blur: blur.round().clamp(0.0, 255.0) as u8,
            spread: 0,
            color,
        };
        vec![Shape::Rect(shadow.as_shape(rect, corner))]
    }

    /// The shadow inside the rect (the shapes spill outside it: paint them with a painter clipped to `rect`), to paint *over*
    /// the fill. Nothing for a number-of-pixels shadow.
    pub(crate) fn inside(&self, rect: Rect, corner: CornerRadius) -> Vec<Shape> {
        let ShadowSpec::Layers(Shadow { inner: Some(layer), .. }) = self else {
            return Vec::new();
        };
        let blur = layer.softness();
        let tint = layer.tint();
        let shifted = rect.translate(vec2(layer.x, layer.y));
        let mut shapes = Vec::new();
        // the part the shifted rect leaves open: solid
        let cover = layer.x.abs() + layer.y.abs() + 2.0;
        shapes.push(Shape::rect_stroke(shifted, corner, Stroke::new(cover, tint), StrokeKind::Outside));
        // then it fades inwards from the edge of the shifted rect
        let steps = (blur / 2.0).ceil().clamp(1.0, 16.0) as usize;
        let step = blur / steps as f32;
        for i in 0..steps {
            let t = (i as f32 + 0.5) / steps as f32;
            let color = tint.gamma_multiply((1.0 - t) * (1.0 - t));
            let ring = shifted.shrink(i as f32 * step);
            shapes.push(Shape::rect_stroke(ring, corner, Stroke::new(step + 0.5, color), StrokeKind::Inside));
        }
        shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Rect};

    #[test]
    fn drop_and_inner_layers_take_their_own_blur_colour_and_opacity() {
        let s = drop(0.0, 6.0).blur(18.0).color("#ff0000").opacity(45.0).inner(0.0, 2.0).blur(4.0).opacity(10.0);
        assert_eq!(s.drop.blur, Some(18.0));
        assert_eq!(s.drop.opacity, 45.0);
        assert_eq!(s.drop.color, Color::from_rgb(255, 0, 0));
        let inner = s.inner.unwrap();
        assert_eq!((inner.x, inner.y, inner.blur, inner.opacity), (0.0, 2.0, Some(4.0), 10.0));
        assert_eq!(inner.color, Color::BLACK);
    }

    #[test]
    fn the_defaults_and_ranges() {
        let l = Layer::new(500.0, -500.0);
        assert_eq!((l.x, l.y), (100.0, -100.0), "x and y stop at 100");
        assert_eq!(Layer::new(0.0, 2.0).softness(), 8.0, "at least 8");
        assert_eq!(Layer::new(0.0, 10.0).softness(), 20.0, "twice the distance");
        assert_eq!(drop(0.0, 1.0).blur(500.0).drop.softness(), 100.0);
        assert_eq!(drop(0.0, 1.0).drop.opacity, 35.0);
        assert_eq!(drop(0.0, 1.0).opacity(250.0).drop.opacity, 100.0);
        assert_eq!(drop(0.0, 1.0).color("not a colour").drop.color, Color::BLACK);
    }

    #[test]
    fn a_widgets_own_opacity_and_colour_go_over_every_layer() {
        let red = Color::from_rgb(255, 0, 0);
        let spec: ShadowSpec = drop(0.0, 6.0).inner(0.0, 2.0).opacity(10.0).into();
        let ShadowSpec::Layers(styled) = spec.styled(Some(80.0), Some(red)) else { panic!("layers") };
        assert_eq!((styled.drop.opacity, styled.drop.color), (80.0, red));
        let inner = styled.inner.unwrap();
        assert_eq!((inner.opacity, inner.color), (80.0, red));
        assert_eq!(spec.styled(None, None), spec, "nothing to put over: the same");
        let old: ShadowSpec = 12.0.into();
        let ShadowSpec::Layers(from_px) = old.styled(Some(50.0), None) else { panic!("layers") };
        assert_eq!((from_px.drop.y, from_px.drop.blur, from_px.drop.opacity), (4.0, Some(12.0), 50.0));
    }

    #[test]
    fn a_number_keeps_the_old_shadow_and_inner_only_comes_with_layers() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 40.0));
        let old: ShadowSpec = 10.0.into();
        assert_eq!(old, ShadowSpec::Px(10.0));
        assert_eq!(old.outer(rect, CornerRadius::same(8)).len(), 1);
        assert!(old.inside(rect, CornerRadius::same(8)).is_empty());
        let plain: ShadowSpec = drop(0.0, 4.0).into();
        assert!(plain.inside(rect, CornerRadius::same(8)).is_empty(), "no .inner: nothing inside");
        let both: ShadowSpec = drop(0.0, 4.0).inner(0.0, 2.0).blur(6.0).into();
        assert_eq!(both.inside(rect, CornerRadius::same(8)).len(), 1 + 3, "the solid part and 3 fading rings");
        assert_eq!(ShadowSpec::from(-5.0), ShadowSpec::Px(0.0));
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::button::Button;
    use crate::card::Card;
    use egui::{pos2, Rect};

    /// How many shapes a frame painted.
    fn shapes_of(mut build: impl FnMut(&mut egui::Ui)) -> usize {
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))), ..Default::default() };
        let out = ctx.run_ui(input, &mut build);
        let count = out.shapes.len();
        std::mem::forget(out);
        count
    }

    #[test]
    fn a_shadow_adds_shapes_to_a_button_and_a_card_and_blur_kawase_is_kept() {
        let plain = shapes_of(|ui| {
            Button("A").show(ui);
            Card().show(ui, |ui| ui.label("x"));
        });
        let shadowed = shapes_of(|ui| {
            Button("A").shadow(drop(0.0, 6.0).inner(0.0, 2.0)).shadow_opacity(60.0).shadow_color("#112233").show(ui);
            Card().shadow(drop(0.0, 6.0).inner(0.0, 2.0)).show(ui, |ui| ui.label("x"));
        });
        assert!(shadowed > plain, "{shadowed} should be more than {plain}");
        assert!(Button("A").blur_kawase(40).kawase_for_test());
        assert!(!Button("A").blur_kawase(40).blur(30).kawase_for_test());
    }
}
