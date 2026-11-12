//! Button: text, image, gradient, border, gol corners, aur glass (blur + glow + drop shadow).
//!
//! ```ignore
//! if Button("Save").radius(8.0).show(ui).clicked() { }
//! Button(Label("Save").bold().italic()).show(ui);      // text ko style dena ho to Label
//!
//! Button(None).size(vec2(200.0, 40.0))
//!     .fill(light(hex("#1c1c1e")).dark(hex("#F5F5F7")))
//!     .image(Image("assets/a.svg").size(20.0).left(12.0))      // left kinare se 12 px andar
//!     .text(Label("Save").bold().size(14.0).center())          // beech mein
//!     .image(Image("assets/b.svg").size(20.0).at(100.0, 0.0))  // top-right kona
//!     .show(ui);
//!
//! Button("Glass").glass().blur(30).glow(hex("#7f5af0"), 16.0).shadow(14.0).show(ui);
//! ```
//!
//! Button ko parent ke andar kahin bithana: `Locate(Button("Save")).at(80.0, 5.0).show(ui)`.
//! Jagah ki kami par buttons ek `>>` button mein simat jayein: `Button("Cut").bundle("toolbar")` aur
//! `Bundle("toolbar").image("assets/more.svg").set(ui)`.
//!
//! Image aur text apni apni jagah (`.left(px)`, `.at(x, y)`, `.center()`) bata sakte hain. Jagah na batao
//! to item text ke bagal mein baithta hai (sab aisi cheezein ek row mein, jis ka group beech mein).

use crate::keys::KeyCombo;
use crate::paint::{
    clean_stops, gradient_fn, paint_border, paint_shape, paint_sides, thin, two_stops, Anim, Glow, GlowCfg, Grad, Stops,
};
use crate::panel::{brightness, Fill, Opacity};
use crate::shadow::ShadowSpec;
use crate::theme::{is_dark, Color, ThemeValue, Themed};
use egui::{
    pos2,
    text::{LayoutJob, TextFormat},
    vec2, CornerRadius, Galley, Id, Pos2, Rect, Response, Sense, Shadow, Stroke, StrokeKind,
    TextStyle, Ui, Vec2,
};
use egui_bg_layer::{presets, BG_Panel};
use std::sync::Arc;

/// Flow mein item aur item ke beech ki khali jagah.
const GAP: f32 = 6.0;

/// `Button(None)` ke liye: is type ki koi value ban hi nahi sakti, isliye sirf `None` likha ja sakta hai.
#[derive(Clone, Copy, Debug)]
pub enum NoLabel {}

/// `Button("text")`, `Button(String)`, `Button(Label("text").bold())` ya `Button(None)` (bina text).
pub trait ButtonLabel {
    /// Text, agar hai.
    fn into_label(self) -> Option<Label>;
}

impl ButtonLabel for &str {
    fn into_label(self) -> Option<Label> {
        Some(Label(self))
    }
}
impl ButtonLabel for String {
    fn into_label(self) -> Option<Label> {
        Some(Label(self))
    }
}
impl ButtonLabel for Label {
    fn into_label(self) -> Option<Label> {
        Some(self)
    }
}
impl ButtonLabel for Option<NoLabel> {
    fn into_label(self) -> Option<Label> {
        None
    }
}

/// `.size(..)` mein: `20.0` (chaukor), `(24.0, 16.0)`, `[24.0, 16.0]` ya `vec2(24.0, 16.0)`.
pub trait IntoSize {
    /// Size.
    fn into_size(self) -> Vec2;
}
impl IntoSize for f32 {
    fn into_size(self) -> Vec2 {
        vec2(self, self)
    }
}
impl IntoSize for (f32, f32) {
    fn into_size(self) -> Vec2 {
        vec2(self.0, self.1)
    }
}
impl IntoSize for [f32; 2] {
    fn into_size(self) -> Vec2 {
        vec2(self[0], self[1])
    }
}
impl IntoSize for Vec2 {
    fn into_size(self) -> Vec2 {
        self
    }
}

/// Item (image ya text) button ke andar kahan baithe.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spot {
    left: Option<f32>,
    right: Option<f32>,
    top: Option<f32>,
    bottom: Option<f32>,
    at: Option<(f32, f32)>,
    center: bool,
}

impl Spot {
    /// Koi jagah nahi bataya: item text ke bagal mein (flow) baithega.
    fn is_flow(&self) -> bool {
        *self == Spot::default()
    }

    /// Item (`size`) ka top-left kona, button `rect` ke andar.
    fn place(&self, rect: Rect, size: Vec2) -> Pos2 {
        let free = rect.size() - size;
        let base_x = match (self.at, self.center) {
            (Some((x, _)), _) => Some(free.x * x.clamp(0.0, 100.0) / 100.0),
            (None, true) => Some(free.x / 2.0),
            _ => None,
        };
        let base_y = match (self.at, self.center) {
            (Some((_, y)), _) => Some(free.y * y.clamp(0.0, 100.0) / 100.0),
            (None, true) => Some(free.y / 2.0),
            _ => None,
        };
        let x = match (self.left, self.right, base_x) {
            (Some(l), _, None) => l,
            (None, Some(r), None) => free.x - r,
            (Some(l), _, Some(base)) => base + l,
            (None, Some(r), Some(base)) => base - r,
            (None, None, Some(base)) => base,
            (None, None, None) => free.x / 2.0,
        };
        let y = match (self.top, self.bottom, base_y) {
            (Some(t), _, None) => t,
            (None, Some(b), None) => free.y - b,
            (Some(t), _, Some(base)) => base + t,
            (None, Some(b), Some(base)) => base - b,
            (None, None, Some(base)) => base,
            (None, None, None) => free.y / 2.0,
        };
        pos2(rect.min.x + x, rect.min.y + y)
    }
}

/// Jagah batane ke tareeke, `Image` aur `Label` dono mein.
macro_rules! spot_methods {
    () => {
        /// Left kinare se `px` andar (minus: bahar). Upar-neeche beech mein, jab tak
        /// `.top(..)` / `.bottom(..)` na do. `.left(8.0).top(8.0)` = top-left kona.
        pub fn left(mut self, px: f32) -> Self {
            self.spot.left = Some(px);
            self
        }

        /// Right kinare se `px` andar (minus: bahar).
        pub fn right(mut self, px: f32) -> Self {
            self.spot.right = Some(px);
            self
        }

        /// Upar ke kinare se `px` andar (minus: bahar).
        pub fn top(mut self, px: f32) -> Self {
            self.spot.top = Some(px);
            self
        }

        /// Neeche ke kinare se `px` andar (minus: bahar).
        pub fn bottom(mut self, px: f32) -> Self {
            self.spot.bottom = Some(px);
            self
        }

        /// Bilkul beech mein.
        pub fn center(mut self) -> Self {
            self.spot.center = true;
            self
        }

        /// Percent se jagah: `x` 0 (left kinare se shuru) se 100 (right kinare par khatam), `y` 0 (upar)
        /// se 100 (neeche). Item hamesha apne container ke andar rehta hai. `(50.0, 50.0)` = beech mein.
        /// Iske baad `.left(px)` wagairah is jagah se aur hilate hain.
        pub fn at(mut self, x: f32, y: f32) -> Self {
            self.spot.at = Some((x, y));
            self
        }
    };
}

/// Image ka rasta, theme ke hisaab se: `"a.svg"` ya `light("a_light.svg").dark("a_dark.svg")`.
/// Jis theme ka na likho wahan image nahi dikhti.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagePath {
    light: Option<String>,
    dark: Option<String>,
}

fn image_uri(path: String) -> String {
    if path.contains("://") { path } else { format!("file://{path}") }
}

impl ImagePath {
    fn same(path: String) -> Self {
        let uri = image_uri(path);
        ImagePath { light: Some(uri.clone()), dark: Some(uri) }
    }

    /// The path written for the light theme, as the user wrote it (None: none for light).
    pub fn light_path(&self) -> Option<&str> {
        self.light.as_deref().map(|p| p.strip_prefix("file://").unwrap_or(p))
    }

    /// The path written for the dark theme, as the user wrote it (None: none for dark).
    pub fn dark_path(&self) -> Option<&str> {
        self.dark.as_deref().map(|p| p.strip_prefix("file://").unwrap_or(p))
    }

    /// Is theme ki image (agar likhi ho).
    fn resolve(&self, dark_theme: bool) -> Option<&str> {
        if dark_theme { self.dark.as_deref() } else { self.light.as_deref() }
    }
}

impl From<&str> for ImagePath {
    fn from(path: &str) -> Self {
        ImagePath::same(path.to_owned())
    }
}
impl From<String> for ImagePath {
    fn from(path: String) -> Self {
        ImagePath::same(path)
    }
}
impl From<Themed<ImagePath>> for ImagePath {
    fn from(themed: Themed<ImagePath>) -> Self {
        ImagePath { light: themed.light.and_then(|p| p.light), dark: themed.dark.and_then(|p| p.dark) }
    }
}
impl ThemeValue for &str {
    type Slot = ImagePath;
    fn into_slot(self) -> ImagePath {
        ImagePath::same(self.to_owned())
    }
}
impl ThemeValue for String {
    type Slot = ImagePath;
    fn into_slot(self) -> ImagePath {
        ImagePath::same(self)
    }
}

/// Flow mein item kahan baithe: text se pehle, jahan likha wahan (default), ya text ke baad.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Order {
    Before,
    #[default]
    AsWritten,
    After,
}

/// Flow ki jagah ke tareeke, `Image` aur `Label` dono mein.
macro_rules! order_methods {
    () => {
        /// Text bagal mein ho to is item ko sabse pehle rakho (likhne ke order ki parwah kiye bina).
        pub fn before(mut self) -> Self {
            self.order = Order::Before;
            self
        }

        /// Is item ko sabse baad mein rakho.
        pub fn after(mut self) -> Self {
            self.order = Order::After;
            self
        }
    };
}

/// Button ke andar ki image (`Image("assets/a.svg")`). Image dikhne ke liye `all_loaders` feature chahiye.
#[derive(Clone, Debug)]
pub struct Image {
    path: ImagePath,
    size: Vec2,
    fit: bool,
    order: Order,
    spot: Spot,
    shadow: Option<ShadowSpec>,
    shadow_opacity: Option<f32>,
    shadow_color: Option<Color>,
}

/// `Image("assets/a.svg")`, `Image("https://..")` ya `Image(light("a_l.svg").dark("a_d.svg"))`.
#[allow(non_snake_case)]
pub fn Image(path: impl Into<ImagePath>) -> Image {
    Image { path: path.into(), size: vec2(18.0, 18.0), fit: false, order: Order::default(), spot: Spot::default(), shadow: None, shadow_opacity: None, shadow_color: None }
}

impl Image {
    /// Image ka size: `20.0` (chaukor), `(w, h)`, `[w, h]` ya `vec2(w, h)`. Default 18.
    /// Diye hue size mein image poori bhar jati hai (kheench kar); aspect ratio bachane ke liye `.fit()`.
    pub fn size(mut self, size: impl IntoSize) -> Self {
        self.size = size.into_size();
        self
    }

    /// Image apna aspect ratio rakhe aur `.size(..)` ke box ke andar beech mein baithe (kheeche nahi).
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// A soft shadow in the shape of the image itself (a PNG / SVG with a transparent background gets a shadow of its
    /// outline): `Image("assets/folder.svg").size(48.0).shadow(drop(0.0, 6.0))`, with `.blur(..)`, `.color(..)`,
    /// `.opacity(..)` (see `niv_ui_kit::shadow`). Only the drop shadow is made (an `.inner(..)` is left out). Only images from
    /// a file path get one (a web address is drawn without). The shadow is made once per size and kept.
    pub fn shadow(mut self, shadow: impl Into<ShadowSpec>) -> Self {
        self.shadow = Some(shadow.into());
        self
    }

    /// The shadow's opacity, 0 to 100 percent.
    pub fn shadow_opacity(mut self, percent: f32) -> Self {
        self.shadow_opacity = Some(percent.clamp(0.0, 100.0));
        self
    }

    /// The shadow's colour (`"#1c1c1e"` or a `Color`).
    pub fn shadow_color(mut self, color: impl crate::shadow::ShadowColor) -> Self {
        self.shadow_color = Some(color.shadow_color());
        self
    }

    order_methods!();
    spot_methods!();
}

impl From<&str> for Image {
    fn from(path: &str) -> Self {
        Image(path)
    }
}
impl From<String> for Image {
    fn from(path: String) -> Self {
        Image(path)
    }
}
impl From<Themed<ImagePath>> for Image {
    fn from(themed: Themed<ImagePath>) -> Self {
        Image(themed)
    }
}

/// Button ke andar ka text (`Label("Save").bold()`).
#[derive(Clone, Debug)]
pub struct Label {
    text: String,
    size: Option<f32>,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    mono: bool,
    opacity: Option<f32>,
    color: Option<Fill>,
    order: Order,
    spot: Spot,
}

/// `Label("Save")`.
#[allow(non_snake_case)]
pub fn Label(text: impl Into<String>) -> Label {
    Label {
        text: text.into(),
        size: None,
        bold: false,
        italic: false,
        underline: false,
        strike: false,
        mono: false,
        opacity: None,
        color: None,
        order: Order::default(),
        spot: Spot::default(),
    }
}

impl Label {
    /// Bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Neeche line.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Beech se kati hui line.
    pub fn strike(mut self) -> Self {
        self.strike = true;
        self
    }

    /// Monospace (code jaisa) font.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// Opacity, 0.0 (gayab) se 1.0 (poora).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Text ka rang: `.color(Color::WHITE)` ya `.color(light(hex("#000")).dark(Color::WHITE))`.
    /// Jis theme ka na likho wahan theme ka apna text rang.
    pub fn color(mut self, color: impl Into<Fill>) -> Self {
        self.color = Some(color.into());
        self
    }

    order_methods!();
    spot_methods!();
}

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Label(text)
    }
}
impl From<String> for Label {
    fn from(text: String) -> Self {
        Label(text)
    }
}

#[derive(Clone, Debug)]
enum Item {
    Image(Image),
    Label(Label),
}

impl Item {
    fn order(&self) -> Order {
        match self {
            Item::Image(image) => image.order,
            Item::Label(label) => label.order,
        }
    }

    fn spot(&self) -> Spot {
        match self {
            Item::Image(image) => image.spot,
            Item::Label(label) => label.spot,
        }
    }
}

/// Apple jaisa glass button, bina kuch set kiye: peeche ka blur, halka safed rang, patla chamakta
/// kinara, naram saaya. Baaki sab (`.size`, `.radius`, `.image`, ...) pehle jaisa lagta hai.
///
/// ```ignore
/// glass_button().image(Image(PLAY).size(18.0)).size(40.0).show(ui);
/// ```
pub fn glass_button() -> Button {
    Button(None).glass().blur(30).radius(12.0).shadow(10.0)
}

/// Ek button. `Button("text")` se banao, aakhir mein `.show(ui)` (ye `Response` deta hai).
#[must_use = "Button ke aakhir mein .show(ui) likho"]
#[derive(Clone)]
pub struct Button {
    bundle: Option<String>,
    /// In the `>>` list this button is not drawn alone (see [`Button::bundle_quiet`]).
    quiet: bool,
    items: Vec<Item>,
    /// `Button("text")` ka text abhi tak apne aap ka hai; pehla `.text(..)` use badal deta hai.
    implicit_label: bool,
    pub(crate) id: Option<String>,
    pub(crate) scale_states: Vec<State>,
    /// A member of a Group: the app-wide `button_feel` does not apply (a Group has its own `.scale(..)`).
    pub(crate) no_feel: bool,
    width: Option<f32>,
    height: Option<f32>,
    /// Bachi hui poori chaudai / unchai tak failao (`.full_width()` / `.full_height()`).
    full_width: bool,
    draggable: bool,
    full_height: bool,
    /// `.full_width_gap(left, right)` / `.full_height_gap(top, bottom)`.
    gap_x: (f32, f32),
    gap_y: (f32, f32),
    padding: (f32, f32),
    fill: Option<Fill>,
    fill_opacity: Option<Opacity>,
    gradient: Stops,
    gradient_points: (f32, f32, f32, f32),
    anim: Option<Anim>,
    radii: [f32; 4],
    border_width: Option<f32>,
    border_widths: Option<[f32; 4]>,
    border_color: Option<Fill>,
    border_colors: Option<[Fill; 4]>,
    border_stops: Stops,
    border_glow: f32,
    border_glow_at: Glow,
    border_glow_intensity: f32,
    border_anim: Option<Anim>,
    glass: bool,
    blur: Option<u8>,
    /// `.blur_kawase(..)`: the Kawase blur instead of the Gaussian one.
    kawase: bool,
    /// Glass ke peeche ka rang-gaadhapan (default 1.4), roshni aur halka rang.
    glass_saturation: f32,
    glass_brightness: Option<f32>,
    glass_tint: Option<(Color, f32)>,
    glow: Option<(Color, f32)>,
    glow_intensity: f32,
    shadow: Option<ShadowSpec>,
    shadow_opacity: Option<f32>,
    shadow_color: Option<Color>,
    opacity: Option<f32>,
    hover: Option<Hover>,
    pressed: Option<Hover>,
    transition: f32,
    keys: Vec<KeyCombo>,
}

/// `Button("text")`, `Button(Label("text").bold())` ya `Button(None)`.
#[allow(non_snake_case)]
pub fn Button(label: impl ButtonLabel) -> Button {
    let label = label.into_label();
    let implicit_label = label.is_some();
    Button {
        bundle: None,
        quiet: false,
        items: label.map(Item::Label).into_iter().collect(),
        implicit_label,
        id: None,
        scale_states: Vec::new(),
        no_feel: false,
        width: None,
        height: None,
        full_width: false,
        draggable: false,
        full_height: false,
        gap_x: (0.0, 0.0),
        gap_y: (0.0, 0.0),
        padding: (12.0, 6.0),
        fill: None,
        fill_opacity: None,
        gradient: Vec::new(),
        gradient_points: (0.0, 0.0, 0.0, 100.0),
        anim: None,
        radii: [6.0; 4],
        border_width: None,
        border_widths: None,
        border_color: None,
        border_colors: None,
        border_stops: Vec::new(),
        border_glow: 0.0,
        border_glow_at: Glow::Inside,
        border_glow_intensity: GlowCfg::DEFAULT_INTENSITY,
        border_anim: None,
        glass: false,
        blur: None,
        kawase: false,
        glass_saturation: 1.4,
        glass_brightness: None,
        glass_tint: None,
        glow: None,
        glow_intensity: 100.0,
        shadow: None,
        shadow_opacity: None,
        shadow_color: None,
        opacity: None,
        hover: None,
        pressed: None,
        transition: 0.12,
        keys: Vec::new(),
    }
}

impl Button {
    // ---- text aur image ----

    /// Aur text jodo: `.text("Save")` ya `.text(Label("Save").bold())`. Kitne bhi ho sakte hain.
    /// `Button(..)` mein diya text pehle `.text(..)` se badal jata hai.
    pub fn text(mut self, label: impl Into<Label>) -> Self {
        let item = Item::Label(label.into());
        if self.implicit_label && !self.items.is_empty() {
            self.items[0] = item;
        } else {
            self.items.push(item);
        }
        self.implicit_label = false;
        self
    }

    /// Image jodo: `.image("assets/a.svg")` ya `.image(Image("assets/a.svg").size(20.0).left(8.0))`.
    pub fn image(mut self, image: impl Into<Image>) -> Self {
        self.items.push(Item::Image(image.into()));
        self
    }

    /// Is button ko `id` wale bundle mein daalo: jagah kam padne par (window chhoti hone par) bundle ke
    /// buttons ek `>>` button mein simat jate hain, aur us par click se list mein khulte hain.
    /// `>>` ka style `Bundle("id")` se.
    pub fn bundle(mut self, id: impl Into<String>) -> Self {
        self.bundle = Some(id.into());
        self
    }

    /// Bundle ka member jo `>>` ki list mein akela nahi dikhta: uski jagah wahi bundle ke Group ka
    /// `Group::bundle_lead(..)` button lega (Group ke frame ke andar, sabse upar). Bar mein ye normal button hai.
    pub fn bundle_quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    // ---- size aur jagah ----

    /// Button ka size: `vec2(120.0, 40.0)`, `(120.0, 40.0)` ya `[120.0, 40.0]`.
    pub fn size(mut self, size: impl IntoSize) -> Self {
        let size = size.into_size();
        self.width = Some(size.x);
        self.height = Some(size.y);
        self
    }

    /// Button ki chaudai (default: content ke hisaab se).
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// Button ki unchai (default: content ke hisaab se).
    pub fn height(mut self, px: f32) -> Self {
        self.height = Some(px);
        self
    }

    /// Button ko pakad kar kheencha ja sake (`response.drag_started()`, `dragged()`, `drag_stopped()`). Click bhi pehle jaisa chalta hai.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// Jitni chaudai bachi hai (row/panel mein) utni poori: button khinchkar poori chaudai le leta hai.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Poori bachi chaudai, par left se `left` aur right se `right` px chhodkar:
    /// `.full_width_gap(12.0, 12.0)`.
    pub fn full_width_gap(mut self, left: f32, right: f32) -> Self {
        self.full_width = true;
        self.gap_x = (left, right);
        self
    }

    /// Poori bachi unchai, par upar se `top` aur neeche se `bottom` px chhodkar:
    /// `.full_height_gap(8.0, 8.0)`.
    pub fn full_height_gap(mut self, top: f32, bottom: f32) -> Self {
        self.full_height = true;
        self.gap_y = (top, bottom);
        self
    }

    /// Jitni unchai bachi hai utni poori: button khinchkar poori unchai le leta hai.
    pub fn full_height(mut self) -> Self {
        self.full_height = true;
        self
    }

    /// Content ke charon taraf khali jagah: `(left-right, upar-neeche)`, default `(12.0, 6.0)`.
    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding = (x, y);
        self
    }

    // ---- rang ----

    /// Button ka rang: `.fill(hex("#1c1c1e"))` ya `.fill(light(hex("#fff")).dark(hex("#1c1c1e")))`.
    /// Jis theme ka na likho wahan theme ka apna button rang. Gradient ke neeche rehta hai.
    pub fn fill(mut self, color: impl Into<Fill>) -> Self {
        self.fill = Some(color.into());
        self
    }

    /// Button ke rang ki opacity, 0 (dikhta nahi) se 100 (poora): `.fill_opacity(40)` ya
    /// `.fill_opacity(light(40).dark(70))`. Jis theme ka na likho wahan rang ki apni alpha (jaise `rgba(..)` ki).
    /// Sirf rang par lagti hai, text, image aur gradient par nahi.
    pub fn fill_opacity(mut self, opacity: impl Into<Opacity>) -> Self {
        self.fill_opacity = Some(opacity.into());
        self
    }

    /// Do rangon ka gradient (Panel jaisa), `.points(..)` se direction.
    pub fn gradient(mut self, a: Color, b: Color) -> Self {
        self.gradient = two_stops(a, b);
        self
    }

    /// Kai rangon ka gradient, jagah ke saath: `[(red, 0.0), (blue, 50.0), (green, 100.0)]`.
    pub fn multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.gradient = clean_stops(stops);
        self
    }

    /// Gradient ki direction (x0, x1, y0, y1; 0 se 100 percent), Panel jaisi.
    pub fn points(mut self, x0: f32, x1: f32, y0: f32, y1: f32) -> Self {
        let clamp = |v: f32| v.clamp(0.0, 100.0);
        self.gradient_points = (clamp(x0), clamp(x1), clamp(y0), clamp(y1));
        self
    }

    fn anim_mut(&mut self) -> &mut Anim {
        self.anim.get_or_insert(Anim { seconds: 3.0, reverse: false, ping_pong: false, turn_at: None })
    }

    /// Gradient ko `seconds` mein ek safar chalao, baar-baar.
    pub fn animate(mut self, seconds: f32) -> Self {
        self.anim_mut().seconds = seconds;
        self
    }

    /// `.animate(..)` ulti disha mein.
    pub fn reverse(mut self) -> Self {
        self.anim_mut().reverse = true;
        self
    }

    /// `.animate(..)` aage-peeche.
    pub fn ping_pong(mut self) -> Self {
        self.anim_mut().ping_pong = true;
        self
    }

    /// `.animate(..)` raaste ke `percent` par mud jaye.
    pub fn turn_at(mut self, percent: f32) -> Self {
        self.anim_mut().turn_at = Some(percent);
        self
    }

    // ---- gol corner aur border ----

    /// Button bada/chhota hota hai: `.scale(State::pop(0.1).hovered())` (hover par 10% ubhre),
    /// `.scale(State::push(0.05).clicked())` (dabane par 5% dabe), `.scale(State::scale(1.08).hovered())` (seedha factor).
    /// Kai baar likh sakte ho (har ek apni halat par chalta hai).
    pub fn scale(mut self, state: State) -> Self {
        self.scale_states.push(state);
        self
    }

    /// Button ka naam: Group ke andar isi naam se is button ko kuch bhi diya ja sakta hai,
    /// jaise `Group("run").button_corners("hammer", 32.0, 0.0, 0.0, 32.0)`.
    pub fn id(mut self, name: impl Into<String>) -> Self {
        self.id = Some(name.into());
        self
    }

    /// Sab corners ek jaise gol, pixel mein.
    pub fn radius(mut self, px: f32) -> Self {
        self.radii = [px.max(0.0); 4];
        self
    }

    /// Har corner alag: top-left, top-right, bottom-right, bottom-left.
    pub fn corners(mut self, top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        self.radii = [top_left, top_right, bottom_right, bottom_left].map(|v| v.max(0.0));
        self
    }

    /// Border ki motai (default 1.0). 1 se patli ho (jaise 0.1) to line halki dikhti hai, egui ke stroke jaisi.
    pub fn border_width(mut self, px: f32) -> Self {
        self.border_width = Some(px);
        self
    }

    /// Har side ki alag motai: top, right, bottom, left.
    pub fn border_widths(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.border_widths = Some([top, right, bottom, left]);
        self
    }

    /// Border ka ek rang; `light(..).dark(..)` bhi chalta hai (jis theme ka na likho wahan border nahi).
    pub fn border_color(mut self, color: impl Into<Fill>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Har side ka alag rang: top, right, bottom, left.
    pub fn border_colors(
        mut self,
        top: impl Into<Fill>,
        right: impl Into<Fill>,
        bottom: impl Into<Fill>,
        left: impl Into<Fill>,
    ) -> Self {
        self.border_colors = Some([top.into(), right.into(), bottom.into(), left.into()]);
        self
    }

    /// Border ka gradient (kinare ke gird).
    pub fn border_gradient(mut self, a: Color, b: Color) -> Self {
        self.border_stops = two_stops(a, b);
        self
    }

    /// Border mein kai rang, jagah ke saath (0 se 100 percent, kinare ke gird).
    pub fn border_multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.border_stops = clean_stops(stops);
        self
    }

    /// Border ke andar ki taraf `px` tak halka glow.
    pub fn border_glow(mut self, px: f32) -> Self {
        self.border_glow = px;
        self
    }

    /// Border glow kahan phaile: `Glow::Inside` (default), `Glow::Middle` ya `Glow::Outside`.
    /// Bahar ka glow parent ke area ke bahar nahi jata, wahin kat jata hai.
    pub fn border_glow_at(mut self, at: Glow) -> Self {
        self.border_glow_at = at;
        self
    }

    /// Border glow kitna tez, 1.0 se 100.0 (default 45.0).
    pub fn border_glow_intensity(mut self, intensity: f32) -> Self {
        self.border_glow_intensity = intensity.clamp(1.0, 100.0);
        self
    }

    fn border_anim_mut(&mut self) -> &mut Anim {
        self.border_anim.get_or_insert(Anim { seconds: 3.0, reverse: false, ping_pong: false, turn_at: None })
    }

    /// Border ka rang `seconds` mein kinare ke gird ek chakkar lagaye.
    pub fn border_animate(mut self, seconds: f32) -> Self {
        self.border_anim_mut().seconds = seconds;
        self
    }

    /// `.border_animate(..)` ulti disha mein.
    pub fn border_reverse(mut self) -> Self {
        self.border_anim_mut().reverse = true;
        self
    }

    /// `.border_animate(..)` aage-peeche.
    pub fn border_ping_pong(mut self) -> Self {
        self.border_anim_mut().ping_pong = true;
        self
    }

    // ---- glass, glow, shadow ----

    /// Glass button: peeche ka blur + halka safed rang + patla chamakta kinara.
    /// Rang `.fill(rgba(..))` se badlo (translucent rakho).
    pub fn glass(mut self) -> Self {
        self.glass = true;
        self
    }

    /// Glass ke peeche ke rang kitne gaadhe dikhen (default 1.4, 1.0 = jaisa hai, 0 = safed-kaala); glass chalu kar deta hai.
    pub fn saturation(mut self, amount: f32) -> Self {
        self.glass = true;
        self.glass_saturation = amount.max(0.0);
        self
    }

    /// Glass ke peeche ki roshni (1.0 = jaisi hai, 1.2 = thodi tez, 0.8 = thodi dhimi); glass chalu kar deta hai.
    pub fn brightness(mut self, amount: f32) -> Self {
        self.glass = true;
        self.glass_brightness = Some(amount.max(0.0));
        self
    }

    /// Glass ke peeche ka halka rang: `.tint(hex("#7f5af0"), 0.2)`, `amount` 0 se 1; glass chalu kar deta hai.
    pub fn tint(mut self, color: Color, amount: f32) -> Self {
        self.glass = true;
        self.glass_tint = Some((color, amount.clamp(0.0, 1.0)));
        self
    }

    /// Blur ki taakat (0 se 100); glass bhi chalu kar deta hai. Default 20.
    pub fn blur(mut self, intensity: u8) -> Self {
        self.glass = true;
        self.blur = Some(intensity.min(100));
        self.kawase = false;
        self
    }

    #[cfg(test)]
    pub(crate) fn kawase_for_test(&self) -> bool {
        self.kawase
    }

    /// Jaise `.blur(..)`, par Kawase blur: bade blur par Gaussian se sasta, thoda zyada naram. 0 se 100.
    pub fn blur_kawase(mut self, intensity: u8) -> Self {
        self.glass = true;
        self.blur = Some(intensity.min(100));
        self.kawase = true;
        self
    }

    /// Button ke bahar ka chamak (glow): rang aur kitna phaile (pixel).
    pub fn glow(mut self, color: Color, px: f32) -> Self {
        self.glow = Some((color, px.max(0.0)));
        self
    }

    /// `.glow(..)` kitna tez, 1.0 se 100.0 (default 100.0, yani rang jaisa diya waisa).
    pub fn glow_intensity(mut self, intensity: f32) -> Self {
        self.glow_intensity = intensity.clamp(1.0, 100.0);
        self
    }

    /// Parchhai: `.shadow(10.0)` (neeche ki taraf, itne px phaili) ya `.shadow(drop(0.0, 6.0))` (x, y, aur `.blur(..)`,
    /// `.color(..)`, `.opacity(..)`, `.inner(x, y)`; dekho `niv_ui_kit::shadow`).
    pub fn shadow(mut self, shadow: impl Into<ShadowSpec>) -> Self {
        self.shadow = Some(shadow.into());
        self
    }

    /// Parchhai kitni gaadhi, 0 se 100 percent (jaise `.fill_opacity`). `.shadow(..)` ki saari layers par lagta hai.
    pub fn shadow_opacity(mut self, percent: f32) -> Self {
        self.shadow_opacity = Some(percent.clamp(0.0, 100.0));
        self
    }

    /// Parchhai ka rang (`"#1c1c1e"` ya `Color`). `.shadow(..)` ki saari layers par lagta hai.
    pub fn shadow_color(mut self, color: impl crate::shadow::ShadowColor) -> Self {
        self.shadow_color = Some(color.shadow_color());
        self
    }

    /// Poore button (rang, text, image, border sab) ki opacity, 0 se 100 (default 100).
    pub fn opacity(mut self, percent: u8) -> Self {
        self.opacity = Some(f32::from(percent.min(100)) / 100.0);
        self
    }

    /// Is key (ya combo) se bhi button click ho: `.key(Key::Enter)`, `.key(Key::Num5.with(Shift))`.
    /// Kai baar likh sakte ho. Key dabe rehne par button dabta hua dikhta hai (`.pressed(..)`), aur
    /// `.show(ui).clicked()` true hota hai.
    pub fn key(mut self, key: impl Into<KeyCombo>) -> Self {
        self.keys.push(key.into());
        self
    }

    // ---- hover aur dabana ----

    /// Mouse upar aane par kya badle: `.hover(Hover().fill(..).scale(1.05).tooltip("Save"))`.
    /// Jo `Hover()` mein nahi likha wo jaisa tha waisa hi rahta hai.
    pub fn hover(mut self, hover: Hover) -> Self {
        self.hover = Some(hover);
        self
    }

    /// Button dabe hone par kya badle: `.pressed(Pressed().scale(0.97))`. Hover ke upar lagta hai.
    pub fn pressed(mut self, pressed: Hover) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Hover/pressed ka badlav kitne second mein ho (default 0.12). `0.0` matlab ek dum.
    pub fn transition(mut self, seconds: f32) -> Self {
        self.transition = seconds.max(0.0);
        self
    }

    // ---- dikhao ----

    /// Text ka rang jab user ne na diya ho: button ka rang (opaque) ho to uske hisaab se kaala/safed,
    /// taaki `.fill(inverse)` jaise rang par text dikhe. Gradient, glass ya translucent rang par theme ka rang.
    fn auto_text_color(&self, dark: bool) -> Option<Color> {
        if self.glass || !self.gradient.is_empty() {
            return None;
        }
        let fill = self.fill.and_then(|fill| fill.explicit(dark))?;
        let percent = self.fill_opacity.and_then(|opacity| opacity.resolve(dark));
        let alpha = percent.map_or(fill.a(), |p| (u16::from(p) * 255 / 100) as u8);
        if alpha < 128 {
            return None;
        }
        Some(if brightness(fill) < 140.0 { Color::WHITE } else { Color::BLACK })
    }

    fn layout_label(ui: &Ui, label: &Label, dark: bool, auto: Option<Color>) -> Arc<Galley> {
        let style_font = if label.mono { TextStyle::Monospace } else { TextStyle::Button }.resolve(ui.style());
        let size = label.size.unwrap_or(style_font.size);
        let real_bold = label.bold && !label.mono && crate::fonts::has_bold();
        let font_id = if real_bold {
            egui::FontId::new(size, crate::fonts::bold_family())
        } else {
            egui::FontId::new(size, style_font.family.clone())
        };
        let mut color = label.color.and_then(|fill| fill.explicit(dark)).or(auto).unwrap_or_else(|| {
            if label.bold && !real_bold { ui.visuals().strong_text_color() } else { ui.visuals().text_color() }
        });
        if let Some(opacity) = label.opacity {
            color = color.gamma_multiply(opacity.clamp(0.0, 1.0));
        }
        let line = Stroke::new(1.0, color);
        let mut job = LayoutJob::default();
        job.append(
            &label.text,
            0.0,
            TextFormat {
                font_id,
                color,
                italics: label.italic,
                underline: if label.underline { line } else { Stroke::NONE },
                strikethrough: if label.strike { line } else { Stroke::NONE },
                ..Default::default()
            },
        );
        ui.painter().layout_job(job)
    }

    /// Is button ko do hisson mein baanto: (1) sirf uska rang-roop (khali rect), (2) sirf uska content
    /// (text/image, size, padding, hover, pressed, keys). `Group` ke liye.
    pub(crate) fn split(&self) -> (Button, Button) {
        let mut frame = self.clone();
        frame.items.clear();
        frame.implicit_label = false;
        frame.bundle = None;
        frame.width = None;
        frame.height = None;
        frame.full_width = false;
        frame.full_height = false;
        frame.hover = None;
        frame.pressed = None;
        frame.keys.clear();
        let mut part = Button(None);
        part.items = self.items.clone();
        part.implicit_label = self.implicit_label;
        part.id = self.id.clone();
        part.scale_states = self.scale_states.clone();
        part.no_feel = self.no_feel;
        part.width = self.width;
        part.height = self.height;
        part.full_width = self.full_width;
        part.full_height = self.full_height;
        part.gap_x = self.gap_x;
        part.gap_y = self.gap_y;
        part.padding = self.padding;
        part.hover = self.hover.clone();
        part.pressed = self.pressed.clone();
        part.transition = self.transition;
        part.keys = self.keys.clone();
        part.opacity = self.opacity;
        (frame, part)
    }

    /// Width aur height dono likhi hon to wahi size.
    pub(crate) fn fixed_size(&self) -> Option<Vec2> {
        Some(vec2(self.width?, self.height?))
    }

    /// Is button ne apna koi rang/glass/gradient diya hai?
    pub(crate) fn has_look(&self) -> bool {
        self.fill.is_some() || !self.gradient.is_empty() || self.glass
    }

    /// Corners (top-left, top-right, bottom-right, bottom-left).
    pub(crate) fn radii(&self) -> [f32; 4] {
        self.radii
    }

    /// Is button ke baad patli lakeer, aur uske baad dusra button: ek hi rect mein do hisse
    /// (image | image). Is button ka rang-roop (glass, border, radius ...) poore rect ka hota hai,
    /// aur uska content/hover pehle hisse ka. `.button(..)` se aur hisse jodo.
    ///
    /// ```ignore
    /// let both = Button(None).glass().radius(16.0).image(Image(PLAY)).size(36.0)
    ///     .separator()
    ///     .button(Button(None).image(Image(STOP)).size(36.0))
    ///     .show(ui);
    /// if both.clicked(0) { } if both.clicked(1) { }
    /// ```
    pub fn separator(self) -> crate::group::Group {
        crate::group::Group::from_button(self).separator()
    }

    /// Is button ke baad dusra button, bina lakeer ke, ek hi rect mein (dekho [`Button::separator`]).
    pub fn button(self, next: Button) -> crate::group::Group {
        crate::group::Group::from_button(self).button(next)
    }

    /// `Group` ke andar ka hissa: jis cheez ko user ne nahi chuna (rang, border, hover, pressed) wo
    /// group ke hisaab se: rang nahi, border nahi, hover par halka rang.
    pub(crate) fn into_segment(mut self, radii: [f32; 4], hover: Option<&Hover>, pressed: Option<&Hover>, transition: Option<f32>) -> Self {
        if self.fill.is_none() && self.gradient.is_empty() && !self.glass {
            self.fill = Some(Color::TRANSPARENT.into());
        }
        if self.border_width.is_none() && self.border_widths.is_none() {
            self.border_width = Some(0.0);
        }
        if self.hover.is_none() {
            self.hover = Some(hover.cloned().unwrap_or_else(|| {
                Hover::default().fill(crate::theme::light(crate::theme::rgba(0, 0, 0, 22)).dark(crate::theme::rgba(255, 255, 255, 30)))
            }));
        }
        if self.pressed.is_none() {
            self.pressed = Some(pressed.cloned().unwrap_or_else(|| {
                Hover::default().fill(crate::theme::light(crate::theme::rgba(0, 0, 0, 44)).dark(crate::theme::rgba(255, 255, 255, 56)))
            }));
        }
        // Group ke andar corner Group tay karta hai: hover/pressed ka `.radius(..)` yahan nahi chalta,
        // warna hover par hissa poora gol hokar capsule se bahar nikal jata hai.
        if let Some(h) = self.hover.as_mut() {
            h.radius = None;
        }
        if let Some(p) = self.pressed.as_mut() {
            p.radius = None;
        }
        if let Some(seconds) = transition {
            self.transition = seconds;
        }
        self.radii = radii.map(|v| v.max(0.0));
        self
    }

    /// Har item ka size, flow ki row aur button ka size.
    pub(crate) fn measure(&self, ui: &Ui) -> Measured {
        let dark = is_dark(ui);
        let auto = self.auto_text_color(dark);
        let galleys: Vec<Option<Arc<Galley>>> = self
            .items
            .iter()
            .map(|item| match item {
                Item::Label(label) => Some(Self::layout_label(ui, label, dark, auto)),
                Item::Image(_) => None,
            })
            .collect();
        let sizes: Vec<Vec2> = self
            .items
            .iter()
            .zip(&galleys)
            .map(|(item, galley)| match (item, galley) {
                (Item::Image(image), _) => image.size,
                (_, Some(galley)) => galley.size(),
                _ => Vec2::ZERO,
            })
            .collect();

        // Jin items ne jagah nahi batai wo ek row mein, group beech mein.
        let mut flow: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].spot().is_flow()).collect();
        flow.sort_by_key(|&i| self.items[i].order()); // stable: ek jaise order mein likhne ka krama bachta hai
        let content = vec2(
            flow.iter().map(|&i| sizes[i].x).sum::<f32>() + GAP * flow.len().saturating_sub(1) as f32,
            flow.iter().map(|&i| sizes[i].y).fold(0.0, f32::max),
        );
        let size = vec2(
            self.width.unwrap_or((content.x + 2.0 * self.padding.0).max(24.0)),
            self.height.unwrap_or((content.y + 2.0 * self.padding.1).max(24.0)),
        );
        Measured { galleys, sizes, flow, content, size }
    }

    /// Button dikhao. Click ke liye `.show(ui).clicked()`.
    pub fn show(self, ui: &mut Ui) -> Response {
        self.show_placed(ui, None, None)
    }

    /// `spot` ho to parent ke andar us jagah (layout mein jagah nahi leta), warna normal flow mein.
    pub(crate) fn show_placed(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> Response {
        let mut measured = self.measure(ui);
        // `.full_width()` / `.full_height()`: bachi hui jagah (free area ho to wahi) tak.
        let room = area.unwrap_or_else(|| ui.available_rect_before_wrap());
        if self.full_width && room.width().is_finite() {
            measured.size.x = (room.width() - self.gap_x.0 - self.gap_x.1).max(0.0);
        }
        if self.full_height && room.height().is_finite() {
            measured.size.y = (room.height() - self.gap_y.0 - self.gap_y.1).max(0.0);
        }
        match self.bundle.clone() {
            None => {
                let (id, rect) = Self::claim(ui, measured.size, spot, area, self.gaps(spot.is_some()));
                self.paint(ui, id, rect, &measured)
            }
            Some(group) => self.show_bundled(ui, &group, spot, area, &measured),
        }
    }

    /// `.full_width_gap(..)` / `.full_height_gap(..)` ka (shuruat ki taraf ka gap, kul gap). Sirf
    /// normal flow ke button ke liye; `placed` button apni jagah se hi shuru hota hai.
    fn gaps(&self, placed: bool) -> (Vec2, Vec2) {
        if placed {
            return (Vec2::ZERO, Vec2::ZERO);
        }
        let x = if self.full_width { self.gap_x } else { (0.0, 0.0) };
        let y = if self.full_height { self.gap_y } else { (0.0, 0.0) };
        (vec2(x.0, y.0), vec2(x.0 + x.1, y.0 + y.1))
    }

    pub(crate) fn claim(ui: &mut Ui, size: Vec2, spot: Option<Spot>, area: Option<Rect>, gaps: (Vec2, Vec2)) -> (Id, Rect) {
        match spot {
            Some(spot) => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                (id, Rect::from_min_size(spot.place(area.unwrap_or_else(|| ui.max_rect()), size), size))
            }
            None => {
                let (id, outer) = ui.allocate_space(size + gaps.1);
                (id, Rect::from_min_size(outer.min + gaps.0, size))
            }
        }
    }

    /// Bundle ka member: jagah ho to normal button, warna chhup kar `>>` ki list mein.
    fn show_bundled(self, ui: &mut Ui, group: &str, spot: Option<Spot>, area: Option<Rect>, m: &Measured) -> Response {
        let gaps = self.gaps(spot.is_some());
        bundle_slot(
            ui,
            group,
            spot,
            area,
            m.size,
            gaps,
            self.quiet,
            |ui, id, rect| self.paint(ui, id, rect, m),
            |popup_ui, _| {
                if self.quiet {
                    let (popup_id, popup_rect) = popup_ui.allocate_space(Vec2::ZERO);
                    return <Response as BundleItem>::hidden(popup_ui, popup_id, popup_rect);
                }
                let (popup_id, popup_rect) = popup_ui.allocate_space(m.size);
                self.paint(popup_ui, popup_id, popup_rect, m)
            },
        )
    }

    /// Button `rect` mein banao aur click/hover pakdo (hover/pressed ka badlav yahin lagta hai).
    pub(crate) fn paint(&self, ui: &Ui, id: Id, rect: Rect, m: &Measured) -> Response {
        let mut response = ui.interact(rect, id, if self.draggable { Sense::click_and_drag() } else { Sense::click() });
        let key_pressed = self.keys.iter().any(|key| key.pressed_in(ui));
        let key_held = self.keys.iter().any(|key| key.held_in(ui));
        if key_pressed {
            response.flags.set(egui::response::Flags::FAKE_PRIMARY_CLICKED, true);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let ctx = ui.ctx();
        let hovered = response.hovered();
        let th = ctx.animate_bool_with_time(id.with("niv_hover"), hovered && self.hover.is_some(), self.transition);
        let tp = ctx.animate_bool_with_time(
            id.with("niv_pressed"),
            (response.is_pointer_button_down_on() || key_held) && self.pressed.is_some(),
            self.transition,
        );
        let down = response.is_pointer_button_down_on() || key_held;
        let mut state_scale = 0.0_f32;
        let mut state_px = 0.0_f32;
        // no `.scale(..)` of its own: the app-wide feel (see `button_feel`) decides by the button's size
        let auto = if self.scale_states.is_empty() && !self.no_feel { auto_states(ctx, rect.size()) } else { Vec::new() };
        for (i, s) in self.scale_states.iter().chain(auto.iter()).enumerate() {
            let on = if s.when == When::Clicked { down } else { hovered };
            let t = ctx.animate_bool_with_time(id.with(("niv_scale", i)), on, self.transition);
            state_scale += (s.factor() - 1.0) * t;
            state_px += s.px() * t;
        }
        if th <= 0.0 && tp <= 0.0 && state_scale == 0.0 && state_px == 0.0 {
            self.draw(ui, rect, m);
            return response;
        }
        let mut eff = self.clone();
        let (mut scale, mut lift) = (1.0_f32 + state_scale, 0.0_f32);
        if let Some(h) = self.hover.as_ref().filter(|_| th > 0.0) {
            eff = eff.with_effect(h, th, ui);
            scale += (h.scale.unwrap_or(1.0) - 1.0) * th;
            lift += h.lift.unwrap_or(0.0) * th;
        }
        if let Some(p) = self.pressed.as_ref().filter(|_| tp > 0.0) {
            eff = eff.with_effect(p, tp, ui);
            scale += (p.scale.unwrap_or(1.0) - 1.0) * tp;
            lift += p.lift.unwrap_or(0.0) * tp;
        }
        // Naap wahi (layout nahi hilta), sirf text/image/rang badal sakte hain.
        let measured = Measured { size: m.size, ..eff.measure(ui) };
        let drawn = Rect::from_center_size(rect.center(), (rect.size() * scale + vec2(2.0 * state_px, 2.0 * state_px)).max(Vec2::ZERO)).translate(vec2(0.0, -lift));
        eff.draw(ui, drawn, &measured);

        if hovered {
            if let Some(h) = &self.hover {
                if let Some(cursor) = h.cursor {
                    ctx.set_cursor_icon(cursor);
                }
                if let Some(tip) = &h.tooltip {
                    response = response.on_hover_text(tip);
                }
            }
        }
        response
    }

    /// Button ki jagah `hover` ke badlav `t` (0 se 1) tak lagakar naya Button.
    /// Rang, motai, opacity wagairah dheere dheere badalte hain; text, image, gradient `t >= 0.5` par ek dum.
    fn with_effect(&self, h: &Hover, t: f32, ui: &Ui) -> Button {
        let dark = is_dark(ui);
        let on = t >= 0.5;
        let lerp = |from: f32, to: f32| from + (to - from) * t;
        let mut b = self.clone();

        if let Some(target) = h.fill.and_then(|fill| fill.explicit(dark)) {
            let from = self.fill.and_then(|fill| fill.explicit(dark)).unwrap_or(ui.visuals().widgets.inactive.weak_bg_fill);
            b.fill = Some(from.lerp_to_gamma(target, t).into());
        }
        if let Some(target) = h.fill_opacity.and_then(|o| o.resolve(dark)) {
            let from = self.fill_opacity.and_then(|o| o.resolve(dark)).unwrap_or(100);
            b.fill_opacity = Some(Opacity::from(lerp(f32::from(from), f32::from(target)).round() as u8));
        }
        if let Some(target) = h.opacity {
            b.opacity = Some(lerp(self.opacity.unwrap_or(1.0), target));
        }
        if on && !h.gradient.is_empty() {
            b.gradient = h.gradient.clone();
        }
        if on {
            if let Some(text) = &h.text {
                if let Some(Item::Label(label)) = b.items.iter_mut().find(|item| matches!(item, Item::Label(_))) {
                    let plain = text.size.is_none()
                        && !(text.bold || text.italic || text.underline || text.strike || text.mono)
                        && text.opacity.is_none()
                        && text.color.is_none();
                    if plain {
                        label.text = text.text.clone();
                    } else {
                        let (order, spot) = (label.order, label.spot);
                        *label = Label { order, spot, ..text.clone() };
                    }
                }
            }
            if let Some(path) = &h.image {
                if let Some(Item::Image(image)) = b.items.iter_mut().find(|item| matches!(item, Item::Image(_))) {
                    image.path = path.clone();
                }
            }
            if let Some(color) = h.text_color {
                for item in &mut b.items {
                    if let Item::Label(label) = item {
                        label.color = Some(color);
                    }
                }
            }
            if let Some(blur) = h.blur {
                b.blur = Some(blur);
                if let Some(kawase) = h.kawase {
                    b.kawase = kawase;
                }
            }
        }
        if let Some(target) = h.border_color.and_then(|fill| fill.explicit(dark)) {
            let from = self
                .border_color
                .and_then(|fill| fill.explicit(dark))
                .unwrap_or_else(|| target.gamma_multiply(0.0));
            b.border_color = Some(from.lerp_to_gamma(target, t).into());
            b.border_colors = None;
            if on {
                b.border_stops.clear();
            }
        }
        if let Some(width) = h.border_width {
            b.border_width = Some(lerp(self.border_width.unwrap_or(1.0), width));
            b.border_widths = None;
        }
        if let Some(px) = h.border_glow {
            b.border_glow = lerp(self.border_glow, px);
        }
        if let Some(intensity) = h.border_glow_intensity {
            b.border_glow_intensity = lerp(self.border_glow_intensity, intensity);
        }
        if let Some(radius) = h.radius {
            b.radii = self.radii.map(|r| lerp(r, radius));
        }
        if let Some((color, px)) = h.glow {
            let (from_px, from_intensity) = self.glow.map_or((0.0, 0.0), |(_, px)| (px, self.glow_intensity));
            b.glow = Some((color, lerp(from_px, px)));
            b.glow_intensity = lerp(from_intensity, h.glow_intensity.unwrap_or(100.0));
        } else if let (Some(intensity), true) = (h.glow_intensity, self.glow.is_some()) {
            b.glow_intensity = lerp(self.glow_intensity, intensity);
        }
        if let Some(px) = h.shadow {
            b.shadow = Some(ShadowSpec::Px(lerp(self.shadow.as_ref().map_or(0.0, ShadowSpec::px), px)));
        }
        b
    }

    /// Button `rect` mein ban kar dikhta hai.
    fn draw(&self, ui: &Ui, rect: Rect, m: &Measured) {
        let dark = is_dark(ui);
        let (galleys, sizes, flow, content) = (&m.galleys, &m.sizes, &m.flow, m.content);
        let mut painter = ui.painter().clone();
        if let Some(opacity) = self.opacity {
            painter.set_opacity(opacity);
        }
        let tint = Color::WHITE.gamma_multiply(self.opacity.unwrap_or(1.0));
        let time = ui.input(|i| i.time);
        let radii = self.radii;
        let corner = CornerRadius {
            nw: radii[0].round().clamp(0.0, 255.0) as u8,
            ne: radii[1].round().clamp(0.0, 255.0) as u8,
            se: radii[2].round().clamp(0.0, 255.0) as u8,
            sw: radii[3].round().clamp(0.0, 255.0) as u8,
        };

        // 1. parchhai aur bahar ka glow
        let shadow = self.shadow.as_ref().map(|spec| spec.styled(self.shadow_opacity, self.shadow_color));
        if let Some(spec) = &shadow {
            for shape in spec.outer(rect, corner) {
                painter.add(shape);
            }
        }
        if let Some((color, px)) = self.glow {
            let shadow = Shadow {
                offset: [0, 0],
                blur: px.round().min(255.0) as u8,
                spread: (px / 4.0).round().min(255.0) as u8,
                color: color.gamma_multiply(self.glow_intensity / 100.0),
            };
            painter.add(shadow.as_shape(rect, corner));
        }

        // 2. rang: glass (blur + overlay), ya seedha fill, phir gradient
        let percent = self.fill_opacity.and_then(|opacity| opacity.resolve(dark));
        let with_opacity = |color: Color| match percent {
            Some(percent) => {
                let [r, g, b, _] = color.to_srgba_unmultiplied();
                Color::from_rgba_unmultiplied(r, g, b, (u16::from(percent) * 255 / 100) as u8)
            }
            None => color,
        };
        let user_fill = self.fill.and_then(|fill| fill.explicit(dark)).map(with_opacity);
        if self.glass {
            let overlay = user_fill.unwrap_or_else(|| with_opacity(Color::from_white_alpha(28)));
            let radius = f32::from(self.blur.unwrap_or(20));
            let blur = if self.kawase { presets::kawase_blur(radius) } else { presets::gaussian_blur(radius) };
            let mut chain = blur.then(presets::saturation(self.glass_saturation));
            if let Some(amount) = self.glass_brightness {
                chain = chain.then(presets::brightness(amount));
            }
            if let Some((color, amount)) = self.glass_tint {
                chain = chain.then(presets::tint(color, amount));
            }
            BG_Panel::new(chain)
                .overlay(overlay)
                .corner_radius(corner)
                .paint_rect(&painter, rect);
        } else {
            let fill = match user_fill {
                Some(color) => Some(color),
                None if self.gradient.is_empty() => Some(with_opacity(ui.visuals().widgets.inactive.weak_bg_fill)),
                None => None,
            };
            if let Some(fill) = fill {
                painter.rect_filled(rect, corner, fill);
            }
        }
        if self.gradient.len() >= 2 {
            let anim = self.anim;
            if anim.is_some() {
                ui.ctx().request_repaint();
            }
            let grad = Grad { stops: self.gradient.clone(), points: self.gradient_points, anim };
            paint_shape(&painter, rect, radii, &gradient_fn(rect, &grad, time));
        }
        // 3. border
        let width = self.border_width.unwrap_or(1.0);
        let widths = self.border_widths.unwrap_or([width; 4]);
        if self.border_stops.len() >= 2 {
            let glow =
                GlowCfg { px: self.border_glow, at: self.border_glow_at, intensity: self.border_glow_intensity };
            paint_border(&painter, rect, radii, widths, &self.border_stops, glow, self.border_anim, time);
        } else {
            let sides: Option<[Option<Color>; 4]> = match (self.border_colors, self.border_color) {
                (Some(colors), _) => Some(colors.map(|fill| fill.explicit(dark))),
                (None, Some(fill)) => Some([fill.explicit(dark); 4]),
                (None, None) if self.glass => Some([Some(Color::from_white_alpha(50)); 4]),
                (None, None) => None,
            };
            match sides {
                Some([Some(a), Some(b), Some(c), Some(d)])
                    if a == b && b == c && c == d && widths.iter().all(|w| *w == widths[0]) =>
                {
                    // Ek hi rang aur ek hi motai: egui ka apna (smooth) stroke.
                    if widths[0] > 0.0 {
                        let (w, strength) = thin(widths[0]);
                        painter.rect_stroke(rect, corner, Stroke::new(w, a.gamma_multiply(strength)), StrokeKind::Inside);
                    }
                }
                Some(sides) if sides.iter().any(Option::is_some) => {
                    paint_sides(&painter, rect, radii, widths, sides.map(|c| c.unwrap_or(Color::TRANSPARENT)));
                }
                _ => {}
            }
        }

        // 3b. the shadow inside (over the colour and the border, under the content)
        if let Some(spec) = &shadow {
            let inside = painter.with_clip_rect(rect);
            for shape in spec.inside(rect, corner) {
                inside.add(shape);
            }
        }

        // 4. content: pehle flow wale (row, beech mein), phir jagah bataye hue
        let mut x = rect.center().x - content.x / 2.0;
        let draw = |i: usize, top_left: Pos2| {
            let item_rect = Rect::from_min_size(top_left, sizes[i]);
            match (&self.items[i], &galleys[i]) {
                (Item::Image(image), _) => {
                    if let Some(uri) = image.path.resolve(dark) {
                        let picture = egui::Image::new(uri.to_owned()).fit_to_exact_size(sizes[i]).tint(tint);
                        let target = if image.fit {
                            Rect::from_center_size(item_rect.center(), picture.load_and_calc_size(ui, sizes[i]).unwrap_or(sizes[i]))
                        } else {
                            item_rect
                        };
                        // with a shadow the picture and its shadow are drawn together (no tint then)
                        let shadow = image.shadow.as_ref().map(|spec| spec.styled(image.shadow_opacity, image.shadow_color));
                        if !shadow.as_ref().is_some_and(|spec| paint_shadow(ui, uri, spec, target)) {
                            picture.paint_at(ui, target);
                        }
                    }
                }
                (_, Some(galley)) => {
                    painter.galley(item_rect.min, galley.clone(), Color::PLACEHOLDER);
                }
                _ => {}
            }
        };
        for &i in flow {
            draw(i, pos2(x, rect.center().y - sizes[i].y / 2.0));
            x += sizes[i].x + GAP;
        }
        for (i, item) in self.items.iter().enumerate() {
            let spot = item.spot();
            if !spot.is_flow() {
                draw(i, spot.place(rect, sizes[i]));
            }
        }
    }
}

/// The picture with its shadow behind it, from a file (read once and kept). False: could not (not a file, unreadable), the
/// caller then draws the plain picture.
fn paint_shadow(ui: &Ui, uri: &str, shadow: &ShadowSpec, rect: Rect) -> bool {
    use egui_bg_layer::ImageShadowExt;
    let Some(path) = uri.strip_prefix("file://") else {
        return false;
    };
    let id = egui::Id::unique(("niv-shadow-bytes", uri));
    let bytes = ui.ctx().data_mut(|d| d.get_temp_mut_or_insert_with::<Option<std::sync::Arc<[u8]>>>(id, || std::fs::read(path).ok().map(Into::into)).clone());
    let Some(bytes) = bytes else {
        return false;
    };
    let source = egui::ImageSource::Bytes { uri: uri.to_owned().into(), bytes: egui::load::Bytes::from(bytes) };
    let (x, y, blur, color) = shadow.for_image();
    // (egui_bg_layer takes the colour and the opacity apart; the opacity is already in the colour's alpha here)
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let tint = egui_bg_layer::ShadowColor::rgba(r, g, b, f32::from(a) / 2.55);
    source.dropshadow((x, y), blur, tint).paint_at(ui, rect);
    true
}

#[derive(Clone, Copy, PartialEq)]
enum When {
    Hovered,
    Clicked,
}

#[derive(Clone, Copy)]
enum Kind {
    Push(f32),
    Pop(f32),
    Scale(f32),
    /// Pixels each edge moves in.
    PushPx(f32),
    /// Pixels each edge moves out.
    PopPx(f32),
}

/// Button ka size kab, kitna badle: `Button.scale(State::pop(0.1).hovered())`.
/// `State::push(0.1)` andar dabta hai, `State::pop(0.1)` bahar ubharta hai, `State::scale(1.08)` seedha factor.
/// Kab: `.hovered()` (default) ya `.clicked()` (dabe rehne tak).
#[derive(Clone, Copy)]
pub struct State {
    kind: Kind,
    when: When,
}

impl State {
    /// Andar dabe, `intensity` jitna: `State::push(0.1)` yani 10% chhota.
    pub fn push(intensity: f32) -> Self {
        Self { kind: Kind::Push(intensity.clamp(0.0, 0.9)), when: When::Hovered }
    }

    /// Bahar ubhre, `intensity` jitna: `State::pop(0.1)` yani 10% bada.
    pub fn pop(intensity: f32) -> Self {
        Self { kind: Kind::Pop(intensity.max(0.0)), when: When::Hovered }
    }

    /// In pixels: every edge moves `px` inward, `State::push_px(2.0)`.
    pub fn push_px(px: f32) -> Self {
        Self { kind: Kind::PushPx(px.max(0.0)), when: When::Hovered }
    }

    /// In pixels: every edge moves `px` outward, `State::pop_px(3.0)`.
    pub fn pop_px(px: f32) -> Self {
        Self { kind: Kind::PopPx(px.max(0.0)), when: When::Hovered }
    }

    /// Seedha factor: `State::scale(1.04)` 4% bada, `0.95` chhota.
    pub fn scale(factor: f32) -> Self {
        Self { kind: Kind::Scale(factor.max(0.0)), when: When::Hovered }
    }

    /// Mouse upar aane par (default).
    pub fn hovered(mut self) -> Self {
        self.when = When::Hovered;
        self
    }

    /// Click ke dauran (button dabe rehne tak).
    pub fn clicked(mut self) -> Self {
        self.when = When::Clicked;
        self
    }

    /// Is it a "while pressed" state (`.clicked()`)?
    pub(crate) fn is_clicked(self) -> bool {
        self.when == When::Clicked
    }

    pub(crate) fn factor(self) -> f32 {
        match self.kind {
            Kind::Push(v) => 1.0 - v,
            Kind::Pop(v) => 1.0 + v,
            Kind::Scale(f) => f,
            Kind::PushPx(_) | Kind::PopPx(_) => 1.0,
        }
    }

    /// Pixels every edge moves (outward positive) for the pixel states, 0 for the others.
    pub(crate) fn px(self) -> f32 {
        match self.kind {
            Kind::PushPx(v) => -v,
            Kind::PopPx(v) => v,
            _ => 0.0,
        }
    }
}

/// How every button without its own `.scale(..)` moves: `big_hover` bigger on hover and `push` smaller while pressed
/// for buttons whose short side is above `big_from` px; smaller buttons only get the push.
/// Buttons wider/taller than `skip_above` px (rows, cards) are left alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonFeel {
    /// Short side above this (px) = a big button.
    pub big_from: f32,
    /// Hover factor of big buttons (1.1 = 10% bigger).
    pub big_hover: f32,
    /// How much a pressed button shrinks (0.6 = 60%).
    pub push: f32,
    /// A button longer than this (px) in any direction gets no movement at all.
    pub skip_above: f32,
}

impl Default for ButtonFeel {
    fn default() -> Self {
        Self { big_from: 30.0, big_hover: 1.1, push: 0.6, skip_above: 120.0 }
    }
}

fn feel_id() -> egui::Id {
    egui::Id::unique("niv_button_feel")
}

/// Turns the app-wide button movement on (`Some(ButtonFeel::default())`) or off (`None`), for every button that has
/// no `.scale(..)` of its own. Call it once at start, for example in `mainkit::app(..).setup(|ctx| ..)`.
pub fn button_feel(ctx: &egui::Context, feel: Option<ButtonFeel>) {
    ctx.data_mut(|d| match feel {
        Some(f) => {
            d.insert_temp(feel_id(), f);
        }
        None => {
            d.remove::<ButtonFeel>(feel_id());
        }
    });
}

fn auto_states(ctx: &egui::Context, size: Vec2) -> Vec<State> {
    let Some(f) = ctx.data(|d| d.get_temp::<ButtonFeel>(feel_id())) else { return Vec::new() };
    if size.x.max(size.y) > f.skip_above {
        return Vec::new();
    }
    let mut out = Vec::new();
    if size.x.min(size.y) > f.big_from {
        out.push(State::scale(f.big_hover).hovered());
    }
    out.push(State::push(f.push).clicked());
    out
}

/// Hover (ya dabe hone) par Button mein kya badle. `Hover()` se shuru karo, jo badalna ho wahi likho.
#[derive(Clone, Default)]
pub struct Hover {
    fill: Option<Fill>,
    fill_opacity: Option<Opacity>,
    opacity: Option<f32>,
    gradient: Stops,
    text: Option<Label>,
    text_color: Option<Fill>,
    image: Option<ImagePath>,
    border_color: Option<Fill>,
    border_width: Option<f32>,
    border_glow: Option<f32>,
    border_glow_intensity: Option<f32>,
    radius: Option<f32>,
    glow: Option<(Color, f32)>,
    glow_intensity: Option<f32>,
    shadow: Option<f32>,
    blur: Option<u8>,
    kawase: Option<bool>,
    scale: Option<f32>,
    lift: Option<f32>,
    cursor: Option<egui::CursorIcon>,
    tooltip: Option<String>,
}

/// `Hover().fill(..).scale(1.05)`: `Button::hover` ke liye.
#[allow(non_snake_case)]
pub fn Hover() -> Hover {
    Hover::default()
}

/// Dabe hone par badlav: `Button::pressed` ke liye. Options `Hover()` wale hi hain.
#[allow(non_snake_case)]
pub fn Pressed() -> Hover {
    Hover::default()
}

impl Hover {
    /// Button ka rang (light/dark alag bhi ho sakta hai).
    pub fn fill(mut self, color: impl Into<Fill>) -> Self {
        self.fill = Some(color.into());
        self
    }

    /// Rang ki opacity, 0 se 100.
    pub fn fill_opacity(mut self, opacity: impl Into<Opacity>) -> Self {
        self.fill_opacity = Some(opacity.into());
        self
    }

    /// Poore button ki opacity, 0 se 100.
    pub fn opacity(mut self, percent: u8) -> Self {
        self.opacity = Some(f32::from(percent.min(100)) / 100.0);
        self
    }

    /// Do rang ka gradient.
    pub fn gradient(mut self, a: Color, b: Color) -> Self {
        self.gradient = two_stops(a, b);
        self
    }

    /// Kai rang ka gradient: `(rang, percent)`.
    pub fn multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.gradient = clean_stops(stops);
        self
    }

    /// Button ka pehla text badal jata hai: `.text("Save now")` (style wahi rehta hai)
    /// ya `.text(Label("Save").bold())` (naya style).
    pub fn text(mut self, label: impl Into<Label>) -> Self {
        self.text = Some(label.into());
        self
    }

    /// Button ke saare text ka rang.
    pub fn text_color(mut self, color: impl Into<Fill>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Button ki pehli image badal jati hai (size aur jagah wahi): `.image("assets/b.svg")`.
    pub fn image(mut self, path: impl Into<ImagePath>) -> Self {
        self.image = Some(path.into());
        self
    }

    /// Border ka rang.
    pub fn border_color(mut self, color: impl Into<Fill>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Border ki motai (px).
    pub fn border_width(mut self, px: f32) -> Self {
        self.border_width = Some(px.max(0.0));
        self
    }

    /// Border ke glow ki chaudai (px).
    pub fn border_glow(mut self, px: f32) -> Self {
        self.border_glow = Some(px.max(0.0));
        self
    }

    /// Border glow kitna tez, 1.0 se 100.0.
    pub fn border_glow_intensity(mut self, intensity: f32) -> Self {
        self.border_glow_intensity = Some(intensity.clamp(1.0, 100.0));
        self
    }

    /// Gol corner (px).
    pub fn radius(mut self, px: f32) -> Self {
        self.radius = Some(px.max(0.0));
        self
    }

    /// Bahar ka chamak: rang aur kitna phaile (px).
    pub fn glow(mut self, color: Color, px: f32) -> Self {
        self.glow = Some((color, px.max(0.0)));
        self
    }

    /// Bahar ka chamak kitna tez, 1.0 se 100.0.
    pub fn glow_intensity(mut self, intensity: f32) -> Self {
        self.glow_intensity = Some(intensity.clamp(1.0, 100.0));
        self
    }

    /// Parchhai (px).
    pub fn shadow(mut self, px: f32) -> Self {
        self.shadow = Some(px.max(0.0));
        self
    }

    /// Glass button ka blur, 0 se 100.
    pub fn blur(mut self, intensity: u8) -> Self {
        self.blur = Some(intensity.min(100));
        self.kawase = Some(false);
        self
    }

    /// Glass button ka Kawase blur, 0 se 100.
    pub fn blur_kawase(mut self, intensity: u8) -> Self {
        self.blur = Some(intensity.min(100));
        self.kawase = Some(true);
        self
    }

    /// Button bada/chhota: `1.05` yani 5% bada. Button ka dabba badhta hai, andar ka text/image apni size par.
    pub fn scale(mut self, factor: f32) -> Self {
        self.scale = Some(factor.max(0.0));
        self
    }

    /// Button itne px upar uthta hai (negative ho to neeche).
    pub fn lift(mut self, px: f32) -> Self {
        self.lift = Some(px);
        self
    }

    /// Mouse ka nishan, jaise `egui::CursorIcon::PointingHand`.
    pub fn cursor(mut self, cursor: egui::CursorIcon) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Hover par chhota text box.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
}

/// Button ka naapa hua size (text layout ke baad).
pub(crate) struct Measured {
    galleys: Vec<Option<Arc<Galley>>>,
    sizes: Vec<Vec2>,
    flow: Vec<usize>,
    content: Vec2,
    pub(crate) size: Vec2,
}

/// Bundle ka member (Button ya Group): `>>` mein chhupne wali cheez.
pub(crate) trait BundleItem {
    fn item_clicked(&self) -> bool;
    fn item_rect(&self) -> Rect;
    /// Chhupe hue member ka (bina jagah liya) khali jawab.
    fn hidden(ui: &mut Ui, id: Id, rect: Rect) -> Self;
}

impl BundleItem for Response {
    fn item_clicked(&self) -> bool {
        self.clicked()
    }
    fn item_rect(&self) -> Rect {
        self.rect
    }
    fn hidden(ui: &mut Ui, id: Id, rect: Rect) -> Self {
        ui.interact(Rect::from_min_size(rect.min, Vec2::ZERO), id, Sense::hover())
    }
}

/// Bundle mein ek member ki jagah: jagah ho to `visible` se wahin banao, warna chhupao (pehla chhupa member
/// `>>` banata hai) aur `>>` khula ho to `popup` se list mein dikhao.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bundle_slot<R: BundleItem>(
    ui: &mut Ui,
    group: &str,
    spot: Option<Spot>,
    area: Option<Rect>,
    size: Vec2,
    gaps: (Vec2, Vec2),
    quiet: bool,
    visible: impl FnOnce(&mut Ui, Id, Rect) -> R,
    popup: impl FnOnce(&mut Ui, bool) -> R,
) -> R {
    let ctx = ui.ctx().clone();
    let state_id = Id::unique(("niv_bundle", group));
    let mut st: BundleState = ctx.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();
    let right: bool = ctx.data_mut(|d| d.get_temp(Id::unique(("niv_bundle_right", group)))).unwrap_or(false);
    let pass = ctx.cumulative_pass_nr();
    if st.pass != pass {
        let was_hidden = st.hidden_from.is_some();
        // `right()`: chhupna shuru / band hone par agli frame mein `>>` ki jagah badalti hai: ek frame aur chalao.
        if right && was_hidden != st.right_hidden_prev {
            ctx.request_repaint();
        }
        st = BundleState { pass, count_prev: st.count_cur, open: st.open && was_hidden, right_hidden_prev: was_hidden, ..BundleState::default() };
    }
    let index = st.count_cur;
    st.count_cur += 1;

    let overflow: Button = ctx
        .data_mut(|d| d.get_temp(Id::unique(("niv_bundle_style", group))))
        .unwrap_or_else(Bundle::default_button);
    let overflow_m = overflow.measure(ui);
    let mut parent = area.unwrap_or_else(|| ui.max_rect());
    let right_end = parent.max.x;
    // `Bundle(..).until(x)`: member is x se aage nahi jaa sakte (jaise beech ke button se pehle ruk jao).
    // `right()` mein ulta: member x se peeche (left) nahi jaa sakte.
    if let Some(limit) = ctx.data_mut(|d| d.get_temp::<f32>(Id::unique(("niv_bundle_limit", group)))) {
        if right {
            parent.min.x = parent.min.x.max(limit.min(parent.max.x));
        } else {
            parent.max.x = parent.max.x.min(limit.max(parent.min.x));
        }
    }
    // `right()`: `>>` right kinare par baithta hai, baaki members uske left mein khisakte hain.
    let mut claim_area = area;
    if right && st.right_hidden_prev {
        let shift = overflow_m.size.x + RIGHT_END;
        parent.max.x -= shift;
        let mut moved = area.unwrap_or_else(|| ui.max_rect());
        moved.max.x -= shift;
        claim_area = Some(moved);
    }
    let already_hidden = st.hidden_from.is_some();
    let (id, rect) = if already_hidden && spot.is_none() && !right {
        // Chhupe hue flow member layout mein jagah nahi lete.
        (Id::unique(("niv_bundle_hidden", group, index)), Rect::from_min_size(parent.min, size))
    } else {
        Button::claim(ui, size, spot, claim_area, gaps)
    };

    let avoid = ctx.data_mut(|d| d.get_temp::<(f32, f32)>(Id::unique(("niv_bundle_avoid", group))));
    let fits = if right {
        st.fits_right(parent, rect) && avoid.is_none_or(|(a, b)| rect.max.x <= a + 0.5 || rect.min.x >= b - 0.5)
    } else {
        !already_hidden && st.fits(parent, rect, overflow_m.size, ui.spacing().item_spacing, index, spot.is_some())
    };
    let response = if fits {
        st.rects.push(rect);
        visible(ui, id, rect)
    } else {
        if quiet {
            st.quiet_hidden = true;
        }
        if !already_hidden {
            st.hidden_from = Some(index);
            let pos = if right {
                pos2(right_end - RIGHT_END - overflow_m.size.x, (rect.center().y - overflow_m.size.y / 2.0).clamp(parent.min.y, (parent.max.y - overflow_m.size.y).max(parent.min.y)))
            } else {
                pos2(
                    rect.min.x.min(parent.max.x - overflow_m.size.x).max(parent.min.x),
                    rect.min.y.min(parent.max.y - overflow_m.size.y).max(parent.min.y),
                )
            };
            let overflow_rect = Rect::from_min_size(pos, overflow_m.size);
            let response = overflow.paint(ui, Id::unique(("niv_bundle_btn", group)), overflow_rect, &overflow_m);
            let over_popup = ctx
                .pointer_latest_pos()
                .and_then(|p| ctx.layer_id_at(p))
                .is_some_and(|layer| layer.order == egui::Order::Foreground);
            if response.clicked() {
                st.open = !st.open;
            } else if st.open && ctx.input(|i| i.pointer.any_pressed()) && !response.hovered() && !over_popup {
                st.open = false;
            }
            st.anchor = Some(overflow_rect);
            st.popup_y = overflow_rect.max.y + 4.0;
        }
        if st.open {
            let anchor = st.anchor.unwrap_or(rect);
            let mut inner = None;
            let lead_wanted = st.quiet_hidden;
            let (pivot, at_x) = if right { (egui::Align2::RIGHT_TOP, anchor.max.x) } else { (egui::Align2::LEFT_TOP, anchor.min.x) };
            egui::Area::new(Id::unique(("niv_bundle_popup", group, index)))
                .order(egui::Order::Foreground)
                .pivot(pivot)
                .fixed_pos(pos2(at_x, st.popup_y))
                .show(&ctx, |popup_ui| {
                    inner = Some(popup(popup_ui, lead_wanted));
                });
            match inner {
                Some(response) => {
                    let height = response.item_rect().height();
                    if height > 0.0 {
                        st.popup_y += height + 4.0;
                    }
                    if response.item_clicked() {
                        st.open = false;
                    }
                    response
                }
                None => R::hidden(ui, id, rect),
            }
        } else {
            R::hidden(ui, id, rect)
        }
    };
    ctx.data_mut(|d| d.insert_temp(state_id, st));
    response
}

/// Ek bundle ki is frame ki haalat (egui ki memory mein, har frame naye sire se).
#[derive(Clone, Default)]
struct BundleState {
    pass: u64,
    /// Pichhli frame mein kitne member the (pata chale ki ye aakhri hai ya nahi).
    count_prev: usize,
    count_cur: usize,
    /// Pehla chhupa hua member (uske baad ke sab chhupe).
    hidden_from: Option<usize>,
    /// A `bundle_quiet` member is hidden (its Group in the list carries it).
    quiet_hidden: bool,
    /// Dikhne wale members ki jagah.
    rects: Vec<Rect>,
    open: bool,
    anchor: Option<Rect>,
    popup_y: f32,
    /// `Bundle::right()`: pichhli frame mein koi member chhupa tha (to `>>` ke liye jagah chhodni hai).
    right_hidden_prev: bool,
}

/// `Bundle::right()`: `>>` aur window ke right kinare ke beech ka fasla (px).
const RIGHT_END: f32 = 4.0;

impl BundleState {
    /// `Bundle::right()` mein member `rect` parent ke andar baithe (left limit se aage) aur kisi se na takraye?
    fn fits_right(&self, parent: Rect, rect: Rect) -> bool {
        rect.min.x >= parent.min.x - 0.5
            && rect.max.x <= parent.max.x + 0.5
            && rect.min.y >= parent.min.y - 0.5
            && rect.max.y <= parent.max.y + 0.5
            && !self.rects.iter().any(|other| other.intersects(rect))
    }

    /// Member `rect` par bina kinare se bahar nikle aur bina kisi pichhle member se chhue baith sakta hai?
    /// Aakhri nahi hai to `>>` ke liye jagah bhi chhodni padti hai.
    fn fits(&self, parent: Rect, rect: Rect, overflow: Vec2, spacing: Vec2, index: usize, located: bool) -> bool {
        let reserve = index + 1 < self.count_prev; // aakhri nahi hai to `>>` ke liye jagah chahiye
        let x_ok = |right: f32| rect.min.x >= parent.min.x - 0.5 && rect.max.x <= right + 0.5;
        let y_ok = |bottom: f32| rect.min.y >= parent.min.y - 0.5 && rect.max.y <= bottom + 0.5;
        let inside = match self.rects.last() {
            // Row mein agla button: sirf chaudai dekho (unchai layout ka mamla hai).
            Some(prev) if rect.min.x >= prev.max.x - 0.5 => {
                x_ok(parent.max.x - if reserve { overflow.x + spacing.x } else { 0.0 })
            }
            // Column mein agla button: sirf unchai dekho.
            Some(prev) if rect.min.y >= prev.max.y - 0.5 => {
                y_ok(parent.max.y - if reserve { overflow.y + spacing.y } else { 0.0 })
            }
            Some(_) => x_ok(parent.max.x) && y_ok(parent.max.y),
            None if located => x_ok(parent.max.x) && y_ok(parent.max.y),
            None => true,
        };
        inside && !self.rects.iter().any(|other| other.intersects(rect))
    }
}

/// Bundle ka `>>` button: `Bundle("toolbar").image("assets/more.svg").set(ui)`. Ye khud kuch nahi dikhata,
/// sirf `>>` button ka style batata hai (har frame ya jab bhi badle). Na likho to text `>>` wala button.
#[must_use = "Bundle ke aakhir mein .set(ui) likho"]
pub struct Bundle {
    id: String,
    button: Button,
    limit: Option<f32>,
    right: bool,
    avoid: Option<(f32, f32)>,
}

/// `Bundle("toolbar")`: wahi id jo buttons ke `.bundle("toolbar")` mein hai.
#[allow(non_snake_case)]
pub fn Bundle(id: impl Into<String>) -> Bundle {
    Bundle { id: id.into(), button: Bundle::default_button(), limit: None, right: false, avoid: None }
}

impl Bundle {
    fn default_button() -> Button {
        Button(">>")
    }

    /// `>>` ki jagah image (text hat jata hai): `.image("assets/more.svg")` ya `.image(Image(..).size(18.0))`.
    pub fn image(mut self, image: impl Into<Image>) -> Self {
        if self.button.implicit_label {
            self.button.items.clear();
            self.button.implicit_label = false;
        }
        self.button.items.push(Item::Image(image.into()));
        self
    }

    /// `>>` button ka text (badalna ho to).
    pub fn text(mut self, label: impl Into<Label>) -> Self {
        self.button = self.button.text(label);
        self
    }

    /// Poora `>>` button apne style mein: `.button(Button(None).image("a.svg").radius(20.0).fill(..))`.
    pub fn button(mut self, button: Button) -> Self {
        self.button = button;
        self
    }

    /// Bundle ke members screen ke is `x` (px, window ke left se) se aage nahi jayenge: wahan se pehle hi
    /// `>>` mein simat jate hain. Jaise beech ke button se pehle ruknaa ho: `.until(beech_ka_left - 12.0)`.
    pub fn until(mut self, x: f32) -> Self {
        self.limit = Some(x);
        self
    }

    /// Right kinare ka bundle: `>>` bilkul right end par aata hai aur members uske left mein rehte hain. Jagah kam
    /// padne par left wale (beech ke paas wale) pehle `>>` mein jate hain. `.until(x)` yahan "x se left nahi jayenge"
    /// hota hai (jaise beech ke project button ka right kinara + fasla).
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }

    /// Ek hi bundle left aur right dono taraf ke members ke liye (`right()` ke saath): members `[left, right]` x-patti
    /// (jaise beech ka project button) par nahi baith sakte, jo takraye woh `>>` mein jata hai. `>>` right end par rehta hai.
    pub fn avoid(mut self, left: f32, right: f32) -> Self {
        self.avoid = Some((left, right));
        self
    }

    /// Style yaad rakho (button dikhta nahi, `.bundle(id)` wale buttons use dikhate hain).
    pub fn set(self, ui: &Ui) {
        let limit_id = Id::unique(("niv_bundle_limit", self.id.as_str()));
        let right_id = Id::unique(("niv_bundle_right", self.id.as_str()));
        ui.ctx().data_mut(|d| {
            if self.right {
                d.insert_temp(right_id, true);
            } else {
                d.remove::<bool>(right_id);
            }
            let avoid_id = Id::unique(("niv_bundle_avoid", self.id.as_str()));
            match self.avoid {
                Some(range) => {
                    d.insert_temp(avoid_id, range);
                }
                None => {
                    d.remove::<(f32, f32)>(avoid_id);
                }
            }
            d.insert_temp(Id::unique(("niv_bundle_style", self.id.as_str())), self.button);
            match self.limit {
                Some(x) if x.is_finite() => {
                    d.insert_temp(limit_id, x);
                }
                _ => d.remove::<f32>(limit_id),
            }
        });
    }
}

/// Locate ke `.claim()` / `.claim_side()` ka tareeka.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Claim {
    /// Upar/neeche ki patti (corner mein bhi).
    Band,
    /// Left/right ka kolam.
    Side,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Parent ka "bacha hua khali hissa" (is frame mein): `Locate(..).claim()` isse ghatata hai,
/// `Locate(..).free()` isi ke andar naapta hai.
#[derive(Clone, Copy)]
struct FreeArea {
    pass: u64,
    rect: Rect,
}

fn free_area_id(ui: &Ui) -> Id {
    Id::unique(("niv_free_area", ui.scope_id()))
}

/// Ab tak ka bacha hua hissa (claim kiye hue patti/kolam ghatake).
fn free_area(ui: &Ui) -> Rect {
    let ctx = ui.ctx();
    let pass = ctx.cumulative_pass_nr();
    let stored: Option<FreeArea> = ctx.data_mut(|d| d.get_temp(free_area_id(ui)));
    match stored {
        Some(area) if area.pass == pass => area.rect,
        _ => ui.max_rect(),
    }
}

/// Button ke bagal ka kinara jo `.claim()` ke liye gina jata hai.
fn claim_edge(claim: Claim, spot: &Spot) -> Option<Edge> {
    let top = spot.top.map(|_| Edge::Top);
    let bottom = spot.bottom.map(|_| Edge::Bottom);
    let left = spot.left.map(|_| Edge::Left);
    let right = spot.right.map(|_| Edge::Right);
    match claim {
        Claim::Band => top.or(bottom).or(left).or(right),
        Claim::Side => left.or(right).or(top).or(bottom),
    }
}

/// `placed` button jitni jagah leta hai wo bacha hua hissa se hata do (aur upar ki patti ho to
/// egui ke normal widgets bhi uske neeche shuru hon).
fn claim_space(ui: &mut Ui, edge: Edge, placed: Rect) {
    let gap = ui.spacing().item_spacing;
    let mut area = free_area(ui);
    match edge {
        Edge::Top => {
            area.min.y = area.min.y.max(placed.max.y + gap.y);
            let flow_top = ui.available_rect_before_wrap().min.y;
            if ui.layout().main_dir() == egui::Direction::TopDown && placed.max.y + gap.y > flow_top {
                ui.add_space(placed.max.y + gap.y - flow_top);
            }
        }
        Edge::Bottom => area.max.y = area.max.y.min(placed.min.y - gap.y),
        Edge::Left => area.min.x = area.min.x.max(placed.max.x + gap.x),
        Edge::Right => area.max.x = area.max.x.min(placed.min.x - gap.x),
    }
    let ctx = ui.ctx();
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(free_area_id(ui), FreeArea { pass, rect: area }));
}

/// Button ko parent ke andar kahin bithao: `Locate(Button("Save")).at(80.0, 5.0).show(ui)`.
/// Jagah na batao to button normal flow mein rehta hai.
///
/// Ab tak ka bacha hua hissa dekhna ho to `.free()`, aur apni jagah kharch karni ho to `.claim()`:
/// ```ignore
/// Locate(display).top(50.0).claim().show(ui);              // upar ki patti kharch
/// Locate(Button("AC")).left(12.0).free().show(ui);         // display ke neeche se naapega
/// ```
#[must_use = "Locate ke aakhir mein .show(ui) likho"]
pub struct Locate<T = Button> {
    item: T,
    spot: Spot,
    free: bool,
    claim: Option<Claim>,
}

/// Jo cheez `Locate(..)` ke andar jaa sakti hai: `Button` ya `Group`.
pub trait Placeable: Sized {
    /// `.show(ui)` ka jawab.
    type Output;
    #[doc(hidden)]
    fn place(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> Self::Output;
    #[doc(hidden)]
    fn rect(out: &Self::Output) -> Rect;
}

impl Placeable for Button {
    type Output = Response;
    fn place(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> Response {
        self.show_placed(ui, spot, area)
    }
    fn rect(out: &Response) -> Rect {
        out.rect
    }
}

/// `Locate(Button(..))` ya `Locate(Group(..))`.
#[allow(non_snake_case)]
pub fn Locate<T: Placeable>(item: T) -> Locate<T> {
    Locate { item, spot: Spot::default(), free: false, claim: None }
}

impl<T: Placeable> Locate<T> {
    spot_methods!();

    /// `.left/.right/.top/.bottom/.center/.at` poore parent ki jagah, ab tak ke **bacha hua khali hissa**
    /// (pichhle `.claim()` aur egui ke normal widgets ke baad) dekh ke naapo.
    pub fn free(mut self) -> Self {
        self.free = true;
        self
    }

    /// Ye button apni jagah kharch kare: jis kinare se laga hai (`.top` ho to upar ki patti, `.bottom` ho
    /// to neeche ki, sirf `.left`/`.right` ho to kolam) wahan se bacha hua hissa chhota ho jata hai.
    /// Corner (`.left` + `.top`) mein upar/neeche ki patti. `.center()` / `.at(..)` mein kuch nahi.
    pub fn claim(mut self) -> Self {
        self.claim = Some(Claim::Band);
        self
    }

    /// Jaise `.claim()`, par corner mein upar/neeche ki jagah left/right ka kolam kharch hota hai.
    pub fn claim_side(mut self) -> Self {
        self.claim = Some(Claim::Side);
        self
    }

    /// Dikhao (`Button` ka `Response`, `Group` ka `GroupResponse`).
    pub fn show(self, ui: &mut Ui) -> T::Output {
        let spot = (!self.spot.is_flow()).then_some(self.spot);
        let area = (self.free && spot.is_some()).then(|| free_area(ui).intersect(ui.available_rect_before_wrap()));
        let out = self.item.place(ui, spot, area);
        if let (Some(claim), Some(spot)) = (self.claim, spot) {
            let rect = T::rect(&out);
            // Bundle mein chhupe member ka size zero hota hai, wo jagah nahi leta.
            if rect.size() != Vec2::ZERO {
                if let Some(edge) = claim_edge(claim, &spot) {
                    claim_space(ui, edge, rect);
                }
            }
        }
        out
    }
}

impl Locate<crate::group::Group> {
    /// Group mein ek aur button (content/size/hover uska, rang-roop Group ka).
    pub fn button(mut self, button: Button) -> Self {
        self.item = self.item.button(button);
        self
    }

    /// Pichhle button ke baad lakeer.
    pub fn separator(mut self) -> Self {
        self.item = self.item.separator();
        self
    }

    /// `id` wale button ke chaaron corner ek jaise (dekho `Group::button_radius`).
    pub fn button_radius(mut self, id: &str, px: f32) -> Self {
        self.item = self.item.button_radius(id, px);
        self
    }

    /// Group ke buttons ka scale (dekho `Group::scale`).
    pub fn scale(mut self, state: State) -> Self {
        self.item = self.item.scale(state);
        self
    }

    /// `id` wale button ke corner (dekho `Group::button_corners`).
    pub fn button_corners(mut self, id: &str, top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        self.item = self.item.button_corners(id, top_left, top_right, bottom_right, bottom_left);
        self
    }

    /// Group ko bundle mein daalo (dekho `Group::bundle`).
    pub fn bundle(mut self, id: impl Into<String>) -> Self {
        self.item = self.item.bundle(id);
        self
    }

    /// Bundle ki list mein sabse upar judne wala button (dekho `Group::bundle_lead`).
    pub fn bundle_lead(mut self, button: Button) -> Self {
        self.item = self.item.bundle_lead(button);
        self
    }

    /// Buttons ke beech barabar khali jagah (px); kam/zyada karke buttons ko door ya paas karo.
    pub fn gap(mut self, px: f32) -> Self {
        self.item = self.item.gap(px);
        self
    }

    /// Har do buttons ke beech lakeer.
    pub fn separators(mut self) -> Self {
        self.item = self.item.separators();
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pixel_states_move_the_edges_by_pixels() {
        assert_eq!(State::pop_px(3.0).px(), 3.0);
        assert_eq!(State::push_px(2.0).clicked().px(), -2.0);
        assert!(State::push_px(2.0).clicked().is_clicked());
        assert_eq!(State::pop_px(3.0).factor(), 1.0);
        assert_eq!(State::scale(1.1).px(), 0.0);
    }

    #[test]
    fn the_app_wide_feel_depends_on_the_button_size() {
        let ctx = egui::Context::default();
        assert!(auto_states(&ctx, vec2(40.0, 40.0)).is_empty());
        button_feel(&ctx, Some(ButtonFeel::default()));
        let big = auto_states(&ctx, vec2(40.0, 40.0));
        assert_eq!(big.len(), 2);
        assert!((big[0].factor() - 1.1).abs() < 1e-6 && (big[1].factor() - 0.4).abs() < 1e-6);
        let small = auto_states(&ctx, vec2(26.0, 26.0));
        assert_eq!(small.len(), 1);
        assert!((small[0].factor() - 0.4).abs() < 1e-6);
        // rows and cards are left alone
        assert!(auto_states(&ctx, vec2(260.0, 26.0)).is_empty());
        button_feel(&ctx, None);
        assert!(auto_states(&ctx, vec2(40.0, 40.0)).is_empty());
    }

    use super::*;
    use crate::panel::inverse;
    use crate::theme::{dark, hex, light, rgb};

    #[test]
    fn spot_places_inside_button() {
        let rect = Rect::from_min_size(pos2(100.0, 50.0), vec2(200.0, 40.0));
        let item = vec2(20.0, 20.0);
        let at = |s: Spot| s.place(rect, item);
        // left(12): kinare se 12 andar, upar-neeche beech mein
        let p = at(Spot { left: Some(12.0), ..Spot::default() });
        assert_eq!((p.x, p.y), (112.0, 60.0));
        // minus: bahar
        assert_eq!(at(Spot { left: Some(-6.0), ..Spot::default() }).x, 94.0);
        // right(10): right kinare se 10 andar
        assert_eq!(at(Spot { right: Some(10.0), ..Spot::default() }).x, 270.0);
        // kone: left+top
        let corner = at(Spot { left: Some(8.0), top: Some(8.0), ..Spot::default() });
        assert_eq!((corner.x, corner.y), (108.0, 58.0));
        // percent: 0 shuru, 100 aakhir (andar hi), 50 beech
        assert_eq!(at(Spot { at: Some((0.0, 0.0)), ..Spot::default() }), pos2(100.0, 50.0));
        assert_eq!(at(Spot { at: Some((100.0, 100.0)), ..Spot::default() }), pos2(280.0, 70.0));
        assert_eq!(at(Spot { at: Some((50.0, 50.0)), ..Spot::default() }), pos2(190.0, 60.0));
        // percent ke baad left(px) aur hilata hai
        assert_eq!(at(Spot { at: Some((50.0, 50.0)), left: Some(5.0), ..Spot::default() }).x, 195.0);
        assert_eq!(at(Spot { center: true, ..Spot::default() }), pos2(190.0, 60.0));
        assert!(Spot::default().is_flow());
        assert!(!Spot { center: true, ..Spot::default() }.is_flow());
    }

    #[test]
    fn an_image_drop_shadow_is_drawn_from_a_file_and_skipped_for_the_rest() {
        let dir = std::env::temp_dir().join(format!("niv_shadow_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("dot.svg");
        std::fs::write(&png, r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><circle cx="16" cy="16" r="8" fill="#c81e1e"/></svg>"##).unwrap();
        let image = Image(png.to_string_lossy().into_owned()).size(32.0).shadow(crate::shadow::drop(0.0, 4.0).blur(10.0)).shadow_opacity(50.0);
        assert!(matches!(image.shadow, Some(ShadowSpec::Layers(_))) && image.shadow_opacity == Some(50.0));
        assert!(Image("a.svg").shadow.is_none());
        let ctx = egui::Context::default();
        let uri = format!("file://{}", png.display());
        let shadow = image.shadow.clone().unwrap().styled(image.shadow_opacity, None);
        let out = ctx.run_ui(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(200.0, 200.0))), ..Default::default() }, |ui| {
            let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(32.0, 32.0));
            assert!(paint_shadow(ui, &uri, &shadow, rect), "a file gets its shadow");
            assert!(!paint_shadow(ui, "https://x/a.png", &shadow, rect), "a web address is drawn plain");
            assert!(!paint_shadow(ui, "file:///no/such/file.png", &shadow, rect), "a missing file is drawn plain");
        });
        std::mem::forget(out);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn items_and_implicit_label() {
        // Button("..") ka text pehle .text(..) se badalta hai, doosra jud jata hai.
        let b = Button("Save").text(Label("Open").bold());
        assert_eq!(b.items.len(), 1);
        let b = b.text("Two").image(Image("assets/a.svg").size(vec2(20.0, 10.0)).left(4.0));
        assert_eq!(b.items.len(), 3);
        let Item::Image(image) = &b.items[2] else { panic!("image nahi mili") };
        assert_eq!(image.size, vec2(20.0, 10.0));
        assert_eq!(image.spot.left, Some(4.0));
        assert!(image.path.resolve(false).is_some_and(|p| p.starts_with("file://")));
        // Button(Label(..)): text ko seedha style
        let b = Button(Label("Bold one").bold());
        assert_eq!(b.items.len(), 1);
        let Item::Label(label) = &b.items[0] else { panic!("label nahi mila") };
        assert!(label.bold && label.text == "Bold one");
        // inverse: dark mein safed-sa, light mein kaala-sa; text uske hisaab se
        let inv: Fill = inverse;
        assert_eq!(inv.explicit(true), Some(rgb(245, 245, 247)));
        assert_eq!(inv.explicit(false), Some(rgb(28, 28, 30)));
        let b = Button("x").fill(inverse);
        assert_eq!(b.auto_text_color(true), Some(Color::BLACK)); // safed button par kaala text
        assert_eq!(b.auto_text_color(false), Some(Color::WHITE)); // kaale button par safed text
        assert_eq!(Button("x").auto_text_color(true), None); // fill nahi: theme ka text
        assert_eq!(Button("x").fill(inverse).fill_opacity(20).auto_text_color(true), None); // translucent
        // glow ki jagah aur tezi (tezi 1..=100 mein simat jati hai)
        let g = Button("x").border_glow(8.0).border_glow_at(Glow::Outside).border_glow_intensity(500.0).glow_intensity(0.0);
        assert_eq!(g.border_glow_at, Glow::Outside);
        assert_eq!((g.border_glow_intensity, g.glow_intensity), (100.0, 1.0));
        assert_eq!(Button("x").border_glow_at, Glow::Inside);
        // fill_opacity
        let b = Button("x").fill_opacity(light(40).dark(70));
        assert_eq!(b.fill_opacity.and_then(|o| o.resolve(false)), Some(40));
        assert_eq!(b.fill_opacity.and_then(|o| o.resolve(true)), Some(70));
        // Button(None) mein text nahi
        assert!(Button(None).items.is_empty());
        // button ka size
        let b = Button(None).size(vec2(120.0, 40.0));
        assert_eq!((b.width, b.height), (Some(120.0), Some(40.0)));
    }

    #[test]
    fn themed_images_and_order() {
        let img = Image(light("a_light.svg").dark("b_dark.svg"));
        assert_eq!(img.path.resolve(false), Some("file://a_light.svg"));
        assert_eq!(img.path.resolve(true), Some("file://b_dark.svg"));
        // sirf light likha: dark mein image nahi
        let only_light = Image(light("a.svg"));
        assert_eq!(only_light.path.resolve(true), None);
        // seedha string dono themes mein
        assert!(Image("a.svg").path.resolve(true).is_some());
        // https wala rasta jaisa hai waisa
        assert_eq!(Image("https://x.y/a.svg").path.resolve(false), Some("https://x.y/a.svg"));
        // .image(light(..).dark(..)) seedha
        let b = Button(None).image(light("l.svg").dark("d.svg"));
        assert_eq!(b.items.len(), 1);
        // before/after
        let b = Button(Label("Save")).image(Image("a.svg").before()).image(Image("b.svg").after());
        let m_order: Vec<Order> = b.items.iter().map(Item::order).collect();
        assert_eq!(m_order, vec![Order::AsWritten, Order::Before, Order::After]);
        assert!(Image("a.svg").fit().fit);
    }

    #[test]
    fn theme_colors_resolve() {
        let c = light(hex("#112233")).dark(rgb(1, 2, 3));
        let fill: Fill = c.into();
        assert_eq!(fill.explicit(false), Some(rgb(0x11, 0x22, 0x33)));
        assert_eq!(fill.explicit(true), Some(rgb(1, 2, 3)));
        // sirf light likha: dark mein default (None)
        let only_light: Fill = light(rgb(9, 9, 9)).into();
        assert_eq!(only_light.explicit(true), None);
        let _ = dark(rgb(0, 0, 0));
    }

    #[test]
    fn builder_collects_style() {
        let a = rgb(1, 2, 3);
        let b = Button("x")
            .radius(8.0)
            .corners(1.0, 2.0, 3.0, 4.0)
            .border_widths(1.0, 2.0, 3.0, 4.0)
            .border_colors(a, a, a, a)
            .glass()
            .blur(30)
            .glow(a, 12.0)
            .shadow(10.0)
            .animate(2.0)
            .ping_pong();
        assert_eq!(b.radii, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(b.border_widths, Some([1.0, 2.0, 3.0, 4.0]));
        assert!(b.glass && b.blur == Some(30) && b.glow.is_some() && b.shadow == Some(ShadowSpec::Px(10.0)));
        assert!(b.anim.is_some_and(|x| x.ping_pong && x.seconds == 2.0));
    }

    #[test]
    fn hover_builder_collects() {
        let a = rgb(1, 2, 3);
        let b = Button("x")
            .opacity(50)
            .transition(0.3)
            .hover(Hover().fill(a).scale(1.1).lift(2.0).text("y").tooltip("t").glow(a, 8.0).border_width(2.0))
            .pressed(Pressed().scale(0.9));
        assert_eq!(b.opacity, Some(0.5));
        assert_eq!(b.transition, 0.3);
        let h = b.hover.as_ref().map(|h| (h.scale, h.lift, h.tooltip.clone(), h.border_width));
        assert_eq!(h, Some((Some(1.1), Some(2.0), Some("t".to_string()), Some(2.0))));
        assert_eq!(b.pressed.and_then(|p| p.scale), Some(0.9));
    }
}
