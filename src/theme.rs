//! Step 1: Colors + Theme.
//!
//! Defaults: dark `28, 28, 30`, light `245, 245, 247`. Theme system ke saath apne aap badalti hai.

use egui::{Context, Theme, Ui};

/// Color type. egui ke `Color32` ka hi naam, to `Color::RED`, `Color::WHITE` sab chalte hain.
pub type Color = egui::Color32;

/// Theme choose karne ka tareeka (system / dark / light).
pub use egui::ThemePreference;

/// Dark mode ka default panel/window color: `rgb(28, 28, 30)`.
pub const DARK_BG: Color = Color::from_rgb(28, 28, 30);

/// Light mode ka default panel/window color: `rgb(245, 245, 247)`.
pub const LIGHT_BG: Color = Color::from_rgb(245, 245, 247);

/// `Color::hex("#1c1c1e")` jaisa shortcut. `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` chalte hain.
///
/// Galat hex likhne par `Color::MAGENTA` milta hai, taaki galti screen pe turant dikhe.
pub trait ColorExt {
    /// Hex string se color.
    fn hex(hex: &str) -> Self;
}

impl ColorExt for Color {
    fn hex(hex: &str) -> Self {
        hex_color(hex)
    }
}

/// Hex se color: `hex("#F5F5F7")` ya `hex("f5f5f7")` (# ho ya na ho). `#RRGGBBAA` bhi chalta hai.
/// Galat hex par `Color::MAGENTA` milta hai.
pub fn hex(hex: &str) -> Color {
    hex_color(hex)
}

fn hex_color(text: &str) -> Color {
    let text = text.trim();
    let text = text.strip_prefix('#').unwrap_or(text);
    Color::from_hex(&format!("#{text}")).unwrap_or(Color::MAGENTA)
}

/// `rgb(r, g, b)` se color banao.
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

/// `rgba(r, g, b, a)` se color banao (a = 0..=255, un-premultiplied).
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba_unmultiplied(r, g, b, a)
}

/// Is waqt theme dark hai ya nahi.
pub fn is_dark(ui: &Ui) -> bool {
    ui.visuals().dark_mode
}

/// Is waqt ke theme ka default background color.
pub fn background(ui: &Ui) -> Color {
    if is_dark(ui) {
        DARK_BG
    } else {
        LIGHT_BG
    }
}

/// Theme preference badlo: `set_theme(&ctx, ThemePreference::Dark)`.
pub fn set_theme(ctx: &Context, preference: ThemePreference) {
    ctx.set_theme(preference);
}

/// Dono themes ke default colors lagao (run() shuru mein ek baar call karta hai).
pub(crate) fn install(ctx: &Context) {
    for (theme, bg) in [(Theme::Dark, DARK_BG), (Theme::Light, LIGHT_BG)] {
        ctx.style_mut_of(theme, |style| {
            style.visuals.panel_fill = bg;
            style.visuals.window_fill = bg;
        });
    }
}

/// Poori window ka background theme ke hisaab se paint karo (`mainkit` har frame ye call karta hai).
pub fn paint_background(ui: &Ui) {
    ui.painter().rect_filled(ui.max_rect(), 0.0, background(ui));
}

/// Ek value jo light aur dark theme ke liye alag ho sakti hai. `light(..)` / `dark(..)` se banao:
///
/// ```ignore
/// .fill(light(hex("#FFFFFF")))              // sirf light badla, dark default
/// .fill(light(hex("#FFFFFF")).dark(rgb(0, 0, 0)))   // dono alag
/// .fill_opacity(light(40).dark(70))
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Themed<T> {
    pub(crate) light: Option<T>,
    pub(crate) dark: Option<T>,
}

/// Wo cheezein jo `light(..)` / `dark(..)` ke andar likhi ja sakti hain (rang, `default`, opacity).
/// Isse `light(rgb(..)).dark(default)` jaisa mix chalta hai.
pub trait ThemeValue {
    /// `light(..)` / `dark(..)` ke andar jis type mein ye store hota hai.
    type Slot;
    /// Slot ke type mein badlo.
    fn into_slot(self) -> Self::Slot;
}

impl ThemeValue for u8 {
    type Slot = u8;
    fn into_slot(self) -> u8 {
        self
    }
}

/// Sirf light theme ke liye value. Dark ke liye `.dark(..)` jodo, ya na jodo to default rehta hai.
pub fn light<V: ThemeValue>(value: V) -> Themed<V::Slot> {
    Themed { light: Some(value.into_slot()), dark: None }
}

/// Sirf dark theme ke liye value. Light ke liye `.light(..)` jodo, ya na jodo to default rehta hai.
pub fn dark<V: ThemeValue>(value: V) -> Themed<V::Slot> {
    Themed { light: None, dark: Some(value.into_slot()) }
}

impl<T> Themed<T> {
    /// Light theme ki value (jod ke chain karne ke liye: `dark(a).light(b)`).
    pub fn light<V: ThemeValue<Slot = T>>(mut self, value: V) -> Self {
        self.light = Some(value.into_slot());
        self
    }

    /// Dark theme ki value (jod ke chain karne ke liye: `light(a).dark(b)`).
    pub fn dark<V: ThemeValue<Slot = T>>(mut self, value: V) -> Self {
        self.dark = Some(value.into_slot());
        self
    }
}
