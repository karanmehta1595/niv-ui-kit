//! `Panel("id")`: side/top/bottom/center panels. egui ka `Panel` + `CentralPanel` ek jagah, rang
//! aur glass (blur) ke saath.
//!
//! ```ignore
//! Panel("sidebar").left().width(220.0)
//!     .fill(rgba(28, 28, 30, 120))
//!     .glass().blur(40)
//!     .show_in(ui, |ui| { ui.text("Hello"); });
//!
//! Panel("main").center().show_in(ui, |ui| { });
//! ```

use crate::theme::{is_dark, Color, ThemeValue, Themed, DARK_BG, LIGHT_BG};
use egui::{Frame, InnerResponse, Margin, Ui, Visuals};
use crate::paint::{
    clean_stops, paint_border, paint_gradient, two_stops, Anim, Glow, GlowCfg, Grad, Stops,
};
use egui_bg_layer::{presets, BG_Panel};

/// `Full` = us side ki bachi hui poori jagah (left/right mein chaudai, top/bottom mein unchai).
/// `Full - 100.0` = poori jagah se 100 px kam. `.range(120.0, Full - 100.0)` jaisa likho.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    /// Bachi jagah mein ye jodo (`Full - 100.0` mein -100).
    offset: f32,
}

/// Bachi hui poori jagah (dekho [`Room`]).
#[allow(non_upper_case_globals)]
pub const Full: Room = Room { offset: 0.0 };

impl std::ops::Sub<f32> for Room {
    type Output = Room;
    fn sub(self, px: f32) -> Room {
        Room { offset: self.offset - px }
    }
}

/// Size ya hadd jo pixel (`120.0`) ya `Full` / `Full - 100.0` ho sakti hai.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extent {
    /// Seedhe pixel.
    Px(f32),
    /// Bachi hui poori jagah mein ye jodo (`Full - 100.0` mein -100).
    Room(f32),
}

/// `.width(..)`, `.range(..)` jaisi jagah jo `f32` aur `Full` dono leti hain.
pub trait IntoExtent {
    /// Extent mein badlo.
    fn into_extent(self) -> Extent;
}
impl IntoExtent for f32 {
    fn into_extent(self) -> Extent {
        Extent::Px(self)
    }
}
impl IntoExtent for Room {
    fn into_extent(self) -> Extent {
        Extent::Room(self.offset)
    }
}
impl Extent {
    /// Asli pixel, `room` = abhi bachi hui jagah.
    fn px(self, room: f32) -> f32 {
        match self {
            Extent::Px(px) => px,
            Extent::Room(offset) => (room + offset).max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// Ek theme ka rang.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shade {
    /// Theme ke saath badle: dark mein 28,28,30, light mein 245,245,247.
    Default,
    /// Hamesha dark default (28,28,30).
    DefaultDark,
    /// Hamesha light default (245,245,247).
    DefaultLight,
    /// Default ka ulta: dark theme mein light default (245,245,247), light theme mein dark default (28,28,30).
    Inverse,
    /// Apna rang.
    Color(Color),
}

impl Shade {
    fn resolve(self, dark_theme: bool) -> Color {
        match self {
            Shade::Default if dark_theme => DARK_BG,
            Shade::Default => LIGHT_BG,
            Shade::DefaultDark => DARK_BG,
            Shade::DefaultLight => LIGHT_BG,
            Shade::Inverse if dark_theme => LIGHT_BG,
            Shade::Inverse => DARK_BG,
            Shade::Color(color) => color,
        }
    }
}

/// Panel ka rang. Ye sab chalte hain:
///
/// ```ignore
/// .fill(default)                                  // theme ke saath: dark #1C1C1E, light #F5F5F7
/// .fill(default_dark)                             // hamesha dark default
/// .fill(default_light)                            // hamesha light default
/// .fill(hex("#FFFFFF"))                           // dono themes mein wahi rang (rgb, rgba bhi)
/// .fill(light(hex("#FFFFFF")))                    // sirf light badla, dark default
/// .fill(dark(rgb(0, 0, 0)))                       // sirf dark badla, light default
/// .fill(light(hex("#FFFFFF")).dark(rgb(0, 0, 0))) // dono alag-alag
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    light: Shade,
    dark: Shade,
}

impl Fill {
    const fn same(shade: Shade) -> Self {
        Fill { light: shade, dark: shade }
    }

    /// Is theme ka rang, sirf tab jab user ne seedha rang diya ho (`default` wagairah par `None`).
    pub(crate) fn explicit(self, dark_theme: bool) -> Option<Color> {
        match if dark_theme { self.dark } else { self.light } {
            Shade::Color(color) => Some(color),
            Shade::Inverse => Some(Shade::Inverse.resolve(dark_theme)),
            _ => None,
        }
    }

    /// Is theme (dark ya light) ke liye asli rang.
    pub fn resolve(self, dark_theme: bool) -> Color {
        if dark_theme { self.dark } else { self.light }.resolve(dark_theme)
    }
}

impl From<Color> for Fill {
    fn from(color: Color) -> Self {
        Fill::same(Shade::Color(color))
    }
}

impl From<Themed<Fill>> for Fill {
    fn from(themed: Themed<Fill>) -> Self {
        Fill {
            light: themed.light.map_or(Shade::Default, |fill| fill.light),
            dark: themed.dark.map_or(Shade::Default, |fill| fill.dark),
        }
    }
}

impl ThemeValue for Fill {
    type Slot = Fill;
    fn into_slot(self) -> Fill {
        self
    }
}

impl ThemeValue for Color {
    type Slot = Fill;
    fn into_slot(self) -> Fill {
        self.into()
    }
}

/// `.fill(default)`: theme ke saath badalta default rang (dark: 28,28,30, light: 245,245,247).
#[allow(non_upper_case_globals)]
pub const default: Fill = Fill::same(Shade::Default);

/// `.fill(default_dark)`: hamesha dark default (28,28,30), theme kuch bhi ho.
#[allow(non_upper_case_globals)]
pub const default_dark: Fill = Fill::same(Shade::DefaultDark);

/// `.fill(inverse)`: `default` ka ulta. Dark theme mein safed-sa (245,245,247), light theme mein kaala-sa (28,28,30).
/// Button ke liye kaam ka hai: page dark ho to button safed, page light ho to button kaala.
#[allow(non_upper_case_globals)]
pub const inverse: Fill = Fill::same(Shade::Inverse);

/// `.fill(default_light)`: hamesha light default (245,245,247), theme kuch bhi ho.
#[allow(non_upper_case_globals)]
pub const default_light: Fill = Fill::same(Shade::DefaultLight);

/// Rang ki opacity, 0..=100. `.fill_opacity(40)` dono themes par, `.fill_opacity(light(40).dark(70))`
/// alag-alag. Jis theme ka na likho, wahan rang ki apni alpha rehti hai.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opacity {
    light: Option<u8>,
    dark: Option<u8>,
}

impl Opacity {
    /// Is theme ki opacity (0..=100), agar likhi ho.
    pub(crate) fn resolve(self, dark_theme: bool) -> Option<u8> {
        if dark_theme { self.dark } else { self.light }
    }
}

impl From<u8> for Opacity {
    fn from(percent: u8) -> Self {
        Opacity { light: Some(percent.min(100)), dark: Some(percent.min(100)) }
    }
}

impl From<Themed<u8>> for Opacity {
    fn from(themed: Themed<u8>) -> Self {
        Opacity { light: themed.light.map(|p| p.min(100)), dark: themed.dark.map(|p| p.min(100)) }
    }
}

/// `Panel(..).frame(None)` ke liye: is type ki koi value ban hi nahi sakti, isliye `.frame(..)` mein
/// sirf `None` likha ja sakta hai (matlab: frame nahi).
#[derive(Clone, Copy, Debug)]
pub enum NoFrame {}

/// Ek panel. `Panel("id")` se banao, aakhir mein `.show_in(ui, |ui| { .. })`.
#[must_use = "Panel ke aakhir mein .show_in(ui, |ui| { .. }) likho"]
pub struct Panel {
    id: String,
    side: Side,
    size: Option<Extent>,
    min: Option<Extent>,
    max: Option<Extent>,
    fill: Fill,
    fill_opacity: Opacity,
    gradient: Stops,
    gradient_points: (f32, f32, f32, f32),
    anim_seconds: Option<f32>,
    anim_reverse: bool,
    anim_ping_pong: bool,
    anim_turn_at: Option<f32>,
    border_stops: Stops,
    border_width: f32,
    border_glow: f32,
    border_glow_at: Glow,
    border_glow_intensity: f32,
    border_anim_seconds: Option<f32>,
    border_reverse: bool,
    border_ping_pong: bool,
    transparent: bool,
    padding: i8,
    resizable: bool,
    glass: bool,
    blur: Option<u8>,
    kawase: bool,
    shadow: Option<crate::shadow::ShadowSpec>,
    shadow_opacity: Option<f32>,
    shadow_color: Option<Color>,
}

/// Naya panel. `id` har panel ka alag hona chahiye. Side na bolo to left.
#[allow(non_snake_case)]
pub fn Panel(id: impl Into<String>) -> Panel {
    Panel {
        id: id.into(),
        side: Side::Left,
        size: None,
        min: None,
        max: None,
        fill: default,
        fill_opacity: Opacity::default(),
        gradient: Vec::new(),
        gradient_points: (0.0, 0.0, 0.0, 100.0),
        anim_seconds: None,
        anim_reverse: false,
        anim_ping_pong: false,
        anim_turn_at: None,
        border_stops: Vec::new(),
        border_width: 2.0,
        border_glow: 0.0,
        border_glow_at: Glow::Inside,
        border_glow_intensity: GlowCfg::DEFAULT_INTENSITY,
        border_anim_seconds: None,
        border_reverse: false,
        border_ping_pong: false,
        transparent: false,
        padding: 8,
        resizable: false,
        glass: false,
        blur: None,
        kawase: false,
        shadow: None,
        shadow_opacity: None,
        shadow_color: None,
    }
}

impl Panel {
    /// Window ke left mein.
    pub fn left(mut self) -> Self {
        self.side = Side::Left;
        self
    }

    /// Window ke right mein.
    pub fn right(mut self) -> Self {
        self.side = Side::Right;
        self
    }

    /// Window ke upar.
    pub fn top(mut self) -> Self {
        self.side = Side::Top;
        self
    }

    /// Window ke neeche.
    pub fn bottom(mut self) -> Self {
        self.side = Side::Bottom;
        self
    }

    /// Bachi hui poori jagah (baaki panels ke baad likho).
    pub fn center(mut self) -> Self {
        self.side = Side::Center;
        self
    }

    /// Size, side ke hisab se: left/right mein chaudai, top/bottom mein unchai (`.range(..)` jaisa).
    /// `120.0` ya `Full` / `Full - 100.0`. `.width(..)` / `.height(..)` bhi wahi kaam karte hain,
    /// jo naam pasand ho wo likho.
    pub fn size(mut self, size: impl IntoExtent) -> Self {
        self.size = Some(size.into_extent());
        self
    }

    /// Chaudai, left/right panel ke liye (`220.0` ya `Full`).
    pub fn width(self, width: impl IntoExtent) -> Self {
        self.size(width)
    }

    /// Unchai, top/bottom panel ke liye (`80.0` ya `Full`).
    pub fn height(self, height: impl IntoExtent) -> Self {
        self.size(height)
    }

    /// Bachi hui poori chaudai (`.width(Full)`).
    pub fn full_width(self) -> Self {
        self.size(Full)
    }

    /// Bachi hui poori unchai (`.height(Full)`).
    pub fn full_height(self) -> Self {
        self.size(Full)
    }

    /// Mouse se size badli ja sake (default: nahi).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Resize ki hadd: `.range(180.0, 240.0)` matlab sabse chhota 180, sabse bada 240.
    /// Left/right mein ye chaudai hai, top/bottom mein unchai. Ye `.resizable(true)` ke saath
    /// kaam ka hai: bina resizable ke `.width()` / `.height()` wali size hi fix rehti hai.
    /// Dono mein pixel ya `Full` chalta hai: `.range(120.0, Full)`, `.range(120.0, Full - 100.0)`.
    pub fn range(mut self, min: impl IntoExtent, max: impl IntoExtent) -> Self {
        self.min = Some(min.into_extent());
        self.max = Some(max.into_extent());
        self
    }

    /// `min` se shuru, aur bachi hui poori jagah tak (`.range(min, Full)`). Kis taraf badhna hai wo
    /// panel ki side se pata chalta hai (bottom mein upar tak, left mein right tak, ...).
    pub fn range_full(self, min: impl IntoExtent) -> Self {
        self.range(min, Full)
    }

    /// Sirf sabse chhota size (upar koi hadd nahi): `.min(180.0)`.
    pub fn min(mut self, min: impl IntoExtent) -> Self {
        self.min = Some(min.into_extent());
        self
    }

    /// Sirf sabse bada size (neeche koi hadd nahi): `.max(300.0)` ya `.max(Full - 100.0)`.
    pub fn max(mut self, max: impl IntoExtent) -> Self {
        self.max = Some(max.into_extent());
        self
    }

    /// Panel ka rang: `default`, `default_dark`, `default_light`, `hex(..)`, `rgb(..)`, `rgba(..)`,
    /// ya theme ke hisab se `light(..)` / `dark(..)` (dekho [`Fill`]).
    pub fn fill(mut self, fill: impl Into<Fill>) -> Self {
        self.fill = fill.into();
        self
    }

    /// Rang ki opacity, 0 (dikhta nahi) se 100 (poora). `.fill_opacity(40)` dono themes par,
    /// `.fill_opacity(light(40).dark(70))` alag-alag. Jis theme ka na likho, wahan rang ki apni
    /// alpha (jaise `rgba(..)` ki) rehti hai.
    pub fn fill_opacity(mut self, opacity: impl Into<Opacity>) -> Self {
        self.fill_opacity = opacity.into();
        self
    }

    /// Do rangon ka gradient (`a` se `b`), `.fill(..)` ke upar. Rang `hex(..)`, `rgb(..)`, `rgba(..)`.
    /// Default: upar se neeche. Direction ke liye `.points(..)`, chalane ke liye `.animate(..)`.
    pub fn gradient(mut self, a: Color, b: Color) -> Self {
        self.gradient = two_stops(a, b);
        self
    }

    /// Kai rangon ka gradient, har rang ke saath uski jagah (0 se 100 percent, line par):
    /// `.multi_gradient([(red, 0.0), (blue, 20.0), (green, 100.0)])`.
    pub fn multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.gradient = clean_stops(stops);
        self
    }

    /// Gradient ki direction: `x0` left, `x1` right, `y0` top, `y1` bottom, sab 0.0 se 100.0
    /// (percent). Rang `a` point (x0, y0) se shuru hota hai aur `b` point (x1, y1) par khatam.
    /// Jaise upar se neeche `(0.0, 0.0, 0.0, 100.0)`, left se right `(0.0, 100.0, 0.0, 0.0)`.
    pub fn points(mut self, x0: f32, x1: f32, y0: f32, y1: f32) -> Self {
        let clamp = |v: f32| v.clamp(0.0, 100.0);
        self.gradient_points = (clamp(x0), clamp(x1), clamp(y0), clamp(y1));
        self
    }

    /// Gradient ko chalao: `seconds` mein `.points(..)` ke raaste par ek safar, phir loop. Chalne par
    /// gradient ghoomta hai (shuru aur aakhir ka rang jud jata hai). Animation chalne tak hi screen
    /// dobara draw hoti hai.
    pub fn animate(mut self, seconds: f32) -> Self {
        self.anim_seconds = Some(seconds);
        self
    }

    /// `.animate(..)` ulti disha mein chale.
    pub fn reverse(mut self) -> Self {
        self.anim_reverse = true;
        self
    }

    /// `.animate(..)` aage jaye, phir wapas aaye, baar-baar. `seconds` ek taraf ke safar ke hain.
    pub fn ping_pong(mut self) -> Self {
        self.anim_ping_pong = true;
        self
    }

    /// `.animate(..)` raaste ke `percent` (0..=100) par pahunch kar wapas mud jaye.
    pub fn turn_at(mut self, percent: f32) -> Self {
        self.anim_turn_at = Some(percent);
        self
    }

    /// Border ka gradient (`a` se `b`, kinare ke gird). Motai `.border_width(..)` se, chalane ke liye
    /// `.border_animate(..)`.
    pub fn border_gradient(mut self, a: Color, b: Color) -> Self {
        self.border_stops = two_stops(a, b);
        self
    }

    /// Border mein kai rang, jagah ke saath (0 se 100 percent, kinare ke gird):
    /// `.border_multi_gradient([(red, 0.0), (blue, 50.0), (green, 100.0)])`.
    pub fn border_multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.border_stops = clean_stops(stops);
        self
    }

    /// Border ki motai pixel mein (default 2.0).
    pub fn border_width(mut self, px: f32) -> Self {
        self.border_width = px;
        self
    }

    /// Border ke andar ki taraf `px` tak halka chamakta glow (default 0, yani nahi).
    pub fn border_glow(mut self, px: f32) -> Self {
        self.border_glow = px;
        self
    }

    /// Border glow kahan phaile: `Glow::Inside` (default), `Glow::Middle` ya `Glow::Outside`.
    /// Panel ke bahar kuch dikhta nahi, isliye yahan `Inside` hi kaam ka hai.
    pub fn border_glow_at(mut self, at: Glow) -> Self {
        self.border_glow_at = at;
        self
    }

    /// Border glow kitna tez, 1.0 se 100.0 (default 45.0).
    pub fn border_glow_intensity(mut self, intensity: f32) -> Self {
        self.border_glow_intensity = intensity.clamp(1.0, 100.0);
        self
    }

    /// Border ka rang `seconds` mein kinare ke gird ek chakkar lagata hai, baar-baar.
    pub fn border_animate(mut self, seconds: f32) -> Self {
        self.border_anim_seconds = Some(seconds);
        self
    }

    /// `.border_animate(..)` ulti disha mein.
    pub fn border_reverse(mut self) -> Self {
        self.border_reverse = true;
        self
    }

    /// `.border_animate(..)` aage jaye, phir wapas, baar-baar.
    pub fn border_ping_pong(mut self) -> Self {
        self.border_ping_pong = true;
        self
    }

    /// `.frame(None)`: andar ka 8 px padding hat jata hai, content kinare tak jata hai.
    /// Rang `.fill(..)` / `.transparent(..)` se hi tay hota hai.
    pub fn frame(mut self, frame: Option<NoFrame>) -> Self {
        if frame.is_none() {
            self.padding = 0;
        }
        self
    }

    /// Panel ka rang bilkul nahi (poora transparent). `.fill(..)` ko bhi dabaa deta hai.
    /// Window ka transparent hona zaroori hai taki peeche ka hissa dikhe (`.transparent(true)` ya `.panel_blur(..)`).
    /// Andar ke text/widgets ka rang window ke theme ke hisab se rehta hai.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Glass look: peeche ka blur + halka gradient. Rang translucent rakho, jaise `rgba(28, 28, 30, 120)`.
    pub fn glass(mut self) -> Self {
        self.glass = true;
        self
    }

    /// Glass ka blur, 0 se 100 (default 20). `.glass()` ke saath chalta hai, aur ise likhne par
    /// `.glass()` apne aap on ho jata hai.
    pub fn blur(mut self, amount: u8) -> Self {
        self.glass = true;
        self.blur = Some(amount.min(100));
        self.kawase = false;
        self
    }

    /// Jaise `.blur(..)`, par Kawase blur: bade blur par Gaussian se sasta, thoda zyada naram. 0 se 100.
    pub fn blur_kawase(mut self, amount: u8) -> Self {
        self.glass = true;
        self.blur = Some(amount.min(100));
        self.kawase = true;
        self
    }

    /// Parchhai: `.shadow(drop(0.0, 6.0))`, `.shadow(drop(0.0, 6.0).inner(0.0, 2.0))` (dekho `niv_ui_kit::shadow`).
    /// Andar wali (`.inner`) hamesha dikhti hai; bahar wali sirf wahan jahan padosi panel use dhak nahi dete.
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

    /// Panel dikhao aur andar ka content `add_contents` mein banao.
    pub fn show_in<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let page_dark = is_dark(ui);
        let fill = self.fill.resolve(page_dark);
        let fill = match if page_dark { self.fill_opacity.dark } else { self.fill_opacity.light } {
            Some(percent) => {
                let [r, g, b, _] = fill.to_srgba_unmultiplied();
                Color::from_rgba_unmultiplied(r, g, b, (u16::from(percent) * 255 / 100) as u8)
            }
            None => fill,
        };
        let (fill, content_dark) = if self.transparent {
            (Color::TRANSPARENT, page_dark)
        } else {
            (fill, brightness(fill) < 140.0)
        };
        let grad = (self.gradient.len() >= 2).then(|| Grad {
            stops: self.gradient.clone(),
            points: self.gradient_points,
            anim: self.anim_seconds.map(|seconds| Anim {
                seconds,
                reverse: self.anim_reverse,
                ping_pong: self.anim_ping_pong,
                turn_at: self.anim_turn_at,
            }),
        });
        let border = (self.border_stops.len() >= 2).then(|| {
            let anim = self.border_anim_seconds.map(|seconds| Anim {
                seconds,
                reverse: self.border_reverse,
                ping_pong: self.border_ping_pong,
                turn_at: None,
            });
            let glow = GlowCfg { px: self.border_glow, at: self.border_glow_at, intensity: self.border_glow_intensity };
            (self.border_stops.clone(), self.border_width, glow, anim)
        });
        let glass = self.glass;
        let blur = f32::from(self.blur.unwrap_or(20));
        let kawase = self.kawase;
        let shadow = self.shadow.as_ref().map(|spec| spec.styled(self.shadow_opacity, self.shadow_color));
        let room_rect = ui.available_rect_before_wrap();
        let ui_room = if matches!(self.side, Side::Top | Side::Bottom) { room_rect.height() } else { room_rect.width() };

        let padding = self.padding;
        let fill_height = matches!(self.side, Side::Top | Side::Bottom);
        let body = move |ui: &mut Ui| {
            if let Some(spec) = &shadow {
                // outside: over the window, not only the panel (so the panel's own clip does not cut it)
                let outside = ui.painter().with_clip_rect(ui.ctx().content_rect());
                for shape in spec.outer(ui.max_rect(), egui::CornerRadius::ZERO) {
                    outside.add(shape);
                }
            }
            if glass {
                let filter = if kawase { presets::kawase_blur(blur) } else { presets::gaussian_blur(blur) };
                BG_Panel::new(filter.then(presets::saturation(1.4)))
                    .overlay(fill)
                    .paint(ui);
                gradient(ui);
            }
            if let Some(grad) = &grad {
                paint_gradient(ui, grad);
            }
            if let Some((stops, width, glow, anim)) = &border {
                paint_border(ui.painter(), ui.max_rect(), [0.0; 4], [*width; 4], stops, *glow, *anim, ui.input(|i| i.time));
            }
            if let Some(spec) = &shadow {
                let inside = ui.painter().with_clip_rect(ui.max_rect());
                for shape in spec.inside(ui.max_rect(), egui::CornerRadius::ZERO) {
                    inside.add(shape);
                }
            }
            // Andar ke text/widgets ka rang panel ke rang ke hisab se, na ki window ke theme ke.
            if content_dark != page_dark {
                *ui.visuals_mut() = if content_dark { Visuals::dark() } else { Visuals::light() };
            }
            Frame::NONE
                .inner_margin(Margin::same(padding))
                .show(ui, |ui| {
                    // Content chhota ho tab bhi panel apni poori jagah le (egui mein khali panel
                    // content jitna sikud jata hai aur resize nahi hota).
                    if fill_height {
                        ui.set_min_height(ui.available_height());
                    } else {
                        ui.set_min_width(ui.available_width());
                    }
                    add_contents(ui)
                })
                .inner
        };

        let frame = if glass { Frame::NONE } else { Frame::NONE.fill(fill) };

        if self.side == Side::Center {
            return egui::CentralPanel::default().frame(frame).show(ui, body);
        }
        let mut panel = match self.side {
            Side::Left => egui::Panel::left(self.id),
            Side::Right => egui::Panel::right(self.id),
            Side::Top => egui::Panel::top(self.id),
            _ => egui::Panel::bottom(self.id),
        }
        .frame(frame)
        .resizable(self.resizable);
        // `Full` = is waqt is side mein bachi hui jagah.
        let available = ui_room;
        if self.min.is_some() || self.max.is_some() {
            let lo = self.min.map_or(20.0, |e| e.px(available));
            let hi = self.max.map_or(f32::INFINITY, |e| e.px(available));
            panel = panel.size_range(lo.min(hi)..=lo.max(hi));
        }
        if let Some(size) = self.size {
            let size = size.px(available);
            panel = if self.resizable { panel.default_size(size) } else { panel.exact_size(size) };
        }
        panel.show(ui, body)
    }
}

/// 0 (kaala) se 255 (safed).
pub(crate) fn brightness(color: Color) -> f32 {
    // Alpha ke bina (un-premultiplied), warna translucent safed rang kaala samjha jata.
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)
}

/// Upar se neeche halka safed gradient, glass ko sheesha jaisa dikhane ke liye.
fn gradient(ui: &Ui) {
    let rect = ui.max_rect();
    let top = Color::from_white_alpha(22);
    let bottom = Color::TRANSPARENT;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{anim_progress, color_at, cyclic};
    use crate::theme::rgb;

    const A: Color = rgb(0, 0, 0);
    const B: Color = rgb(200, 100, 50);

    fn anim(seconds: f32) -> Anim {
        Anim { seconds, reverse: false, ping_pong: false, turn_at: None }
    }

    #[test]
    fn color_at_blends_and_clamps() {
        let ab = two_stops(A, B);
        assert_eq!(color_at(&ab, 0.0), A);
        assert_eq!(color_at(&ab, 1.0), B);
        assert_eq!(color_at(&ab, -5.0), A);
        assert_eq!(color_at(&ab, 5.0), B);
        let mid = color_at(&ab, 0.5);
        assert_eq!((mid.r(), mid.g(), mid.b()), (100, 50, 25));
        // jagah ke saath: 20% tak A, phir B tak
        let c = rgb(10, 20, 30);
        let stops = clean_stops([(B, 100.0), (A, 0.0), (c, 20.0)]); // bina sort ke bhi theek
        assert_eq!(color_at(&stops, 0.2), c);
        assert_eq!(color_at(&stops, 0.0), A);
        assert_eq!(color_at(&stops, 1.0), B);
        // pehle stop se pehle pehla rang, aakhri ke baad aakhri rang
        let late = clean_stops([(A, 30.0), (B, 60.0)]);
        assert_eq!(color_at(&late, 0.1), A);
        assert_eq!(color_at(&late, 0.9), B);
    }

    #[test]
    fn cyclic_returns_to_first_color() {
        let cycle = cyclic(&two_stops(A, B));
        assert_eq!(cycle.len(), 3);
        assert_eq!(color_at(&cycle, 0.0), color_at(&cycle, 1.0));
        assert_eq!(color_at(&cycle, 0.5), B);
    }

    #[test]
    fn animation_progress_modes() {
        // loop: 3 sec mein ek safar
        let a = anim(3.0);
        assert!(anim_progress(0.0, &a).abs() < 1e-5);
        assert!((anim_progress(1.5, &a) - 0.5).abs() < 1e-5);
        assert!((anim_progress(3.0 + 0.75, &a) - 0.25).abs() < 1e-4); // loop: dobara shuru
        // reverse
        let r = Anim { reverse: true, ..a };
        assert!((anim_progress(1.5, &r) + 0.5).abs() < 1e-5);
        // ping-pong: 3 sec aage, 3 sec peeche
        let p = Anim { ping_pong: true, ..a };
        assert!((anim_progress(3.0, &p) - 1.0).abs() < 1e-5);
        assert!((anim_progress(4.5, &p) - 0.5).abs() < 1e-5);
        assert!(anim_progress(6.0, &p).abs() < 1e-5);
        // turn_at 60: 60% tak jaye aur wapas
        let t = Anim { turn_at: Some(60.0), ..a };
        assert!((anim_progress(1.8, &t) - 0.6).abs() < 1e-5); // 3 sec * 0.6
        assert!(anim_progress(3.6, &t).abs() < 1e-5); // wapas shuru par
        assert!(anim_progress(10.0, &t) <= 0.6 + 1e-5);
    }

    #[test]
    fn builder_collects_gradient_settings() {
        let panel = Panel("g")
            .gradient(A, B)
            .points(0.0, 100.0, 0.0, 100.0)
            .animate(3.0)
            .reverse()
            .ping_pong()
            .turn_at(60.0);
        assert_eq!(panel.gradient, two_stops(A, B));
        assert_eq!(panel.gradient_points, (0.0, 100.0, 0.0, 100.0));
        assert_eq!(panel.anim_seconds, Some(3.0));
        assert!(panel.anim_reverse && panel.anim_ping_pong);
        assert_eq!(panel.anim_turn_at, Some(60.0));
        // bahar ki value clamp hoti hai
        let clamped = Panel("c").points(-5.0, 150.0, 0.0, 100.0);
        assert_eq!(clamped.gradient_points, (0.0, 100.0, 0.0, 100.0));
        // multi_gradient
        let multi = Panel("m").multi_gradient([(A, 0.0), (B, 20.0), (A, 100.0)]);
        assert_eq!(multi.gradient.len(), 3);
        // border
        let b = Panel("b").border_gradient(A, B).border_width(3.0).border_glow(8.0).border_animate(2.0);
        assert_eq!(b.border_stops, two_stops(A, B));
        assert_eq!((b.border_width, b.border_glow, b.border_anim_seconds), (3.0, 8.0, Some(2.0)));
    }
}
