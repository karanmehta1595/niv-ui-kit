//! # niv-ui-kit
//!
//! Swift-style, easy-to-use UI kit for Rust, built on top of egui.
//! Ye egui wala hissa hai (text, colors, theme, widgets). Window chalane wala hissa `mainkit` hai.
//!
//! Apni alag `egui` dependency mat jodo. Egui chahiye to `use niv_ui_kit::egui;`.

#![deny(warnings)]
#![warn(missing_docs)]

#[cfg(all(feature = "egui-0-36", not(feature = "egui-0-37")))]
compile_error!("egui-0-36 abhi supported nahi hai. Phase 1 mein sirf egui-0-37 chalta hai.");

#[cfg(all(feature = "egui-0-36", feature = "egui-0-37"))]
compile_error!("Do egui versions ek saath on hain (egui-0-36 aur egui-0-37). Sirf ek chuno.");

#[cfg(not(feature = "egui-0-37"))]
compile_error!("niv-ui-kit ko ek egui version feature chahiye: egui-0-37 (default).");

#[cfg(feature = "egui-0-37")]
mod fonts;

#[cfg(feature = "egui-0-37")]
pub mod theme;

#[cfg(feature = "egui-0-37")]
pub mod text;

#[cfg(feature = "egui-0-37")]
mod paint;

#[cfg(feature = "egui-0-37")]
pub mod panel;

#[cfg(feature = "egui-0-37")]
pub mod button;

#[cfg(feature = "egui-0-37")]
pub mod keys;

#[cfg(feature = "egui-0-37")]
pub mod dropdown;
pub mod window;
pub mod mac_window;
#[cfg(feature = "egui-0-37")]
pub mod scroll;
pub mod group;
#[cfg(feature = "egui-0-37")]
pub mod card;
#[cfg(feature = "egui-0-37")]
/// `.shadow(drop(x, y))` for Button, Image, Group, Panel and Card.
pub mod shadow;
#[cfg(feature = "egui-0-37")]
#[allow(missing_docs, dead_code, clippy::all)]
pub mod term_core;
#[cfg(feature = "egui-0-37")]
#[allow(missing_docs, clippy::all)]
pub mod terminal;
#[cfg(feature = "egui-0-37")]
pub mod menu;
#[cfg(feature = "egui-0-37")]
pub mod switch;
#[cfg(feature = "egui-0-37")]
pub mod textfield;
#[cfg(feature = "egui-0-37")]
pub mod layout;

#[cfg(feature = "egui-0-37")]
pub mod canvas;
#[cfg(feature = "egui-0-37")]
pub mod edit;
#[cfg(feature = "egui-0-37")]
pub mod img;
#[cfg(feature = "egui-0-37")]
pub mod input;
#[cfg(feature = "egui-0-37")]
pub mod mem;
#[cfg(feature = "egui-0-37")]
pub mod popup;

/// Poora egui, niv-ui-kit se hi (alag dependency ki zaroorat nahi).
#[cfg(feature = "egui-0-37")]
pub use egui;

/// UI ka `Ui` type: `use niv_ui_kit::Ui;` ya seedha `niv_ui_kit::Ui` likho.
#[cfg(feature = "egui-0-37")]
pub use egui::Ui;

/// Blur / liquid-glass crate.
#[cfg(feature = "egui-0-37")]
pub use egui_bg_layer;

/// Shuru mein ek baar call karo (`mainkit` ye khud kar deta hai): theme ke default colors,
/// bold font, aur (agar `all_loaders` feature on hai) image loaders.
#[cfg(feature = "egui-0-37")]
pub fn init(ctx: &egui::Context) {
    theme::install(ctx);
    fonts::install(ctx);
    #[cfg(feature = "all_loaders")]
    egui_extras::install_image_loaders(ctx);
}

/// Ek hi import mein zaroori cheezein.
#[cfg(feature = "egui-0-37")]
pub mod prelude {
    pub use crate::button::{button_feel, glass_button, Bundle, Button, ButtonFeel, Hover, Image, ImagePath, Label, Locate, Placeable, Pressed, State};
    pub use crate::dropdown::Dropdown;
    pub use crate::window::{close_dot, Outside, Window};
    pub use crate::group::{Group, GroupResponse};
    pub use crate::keys::Mod::{Alt, Cmd, Ctrl, Shift};
    pub use crate::keys::{KeyCombo, KeyExt, Mod};
    pub use crate::layout::{Align, Column, Row};
    pub use crate::paint::Glow;
    pub use crate::scroll::{Bottom, Scroll};
    pub use crate::card::Card;
    pub use crate::shadow::{drop, Shadow};
    pub use crate::layout::put;
    pub use crate::menu::{Menu, MenuLine};
    pub use crate::text::{copy_text, fit, text_width, wrap_lines};
    pub use crate::switch::Switch;
    pub use crate::terminal::{TermReq, Terminal};
    pub use crate::textfield::TextField;
    pub use crate::panel::{default, default_dark, default_light, inverse, Fill, Full, Opacity, Panel};
    pub use crate::text::{Text, UiExt};
    pub use crate::theme::{dark, hex, light, rgb, rgba, set_theme, Color, ColorExt, ThemePreference};
    pub use crate::canvas::Canvas;
    pub use crate::edit::{caret_ends, selection, set_selection, CodeEdit};
    pub use crate::img::{Img, Load};
    pub use crate::input::Input;
    pub use crate::mem::Mem;
    pub use crate::popup::{Popup, Tip};
    pub use egui::text::{LayoutJob, TextFormat};
    pub use egui::{
        pos2, vec2, Align2, Area, ColorImage, Context, CornerRadius, CursorIcon, Event, FontId, Id, Key, Layout, Mesh, Modifiers,
        Order, Painter, PointerButton, Pos2, RawInput, Rect, Response, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions,
        Theme, Ui, UiBuilder, Vec2, Visuals,
    };
    pub use egui::emath::GuiRounding;
    pub use egui::{MouseWheelUnit, TouchPhase};
    pub use egui::MouseWheelSource as WheelSource;
    /// `Layout::left_to_right(LayoutAlign::Center)` ke liye (`Row`/`Column` ke `Align` se alag cheez).
    pub use egui::Align as LayoutAlign;
    pub use egui::scroll_area::ScrollBarVisibility;
    pub use egui::text::{CCursor, CCursorRange};
    pub use egui::{Galley, TextBuffer};
    /// Dark ya Light mode (`ThemeMode::Dark`).
    pub type ThemeMode = egui::Theme;
    /// Purana egui naam (`Color` ke saath ek hi cheez).
    pub type Color32 = egui::Color32;
    /// `Point` = `Pos2` (screen par ek jagah).
    pub type Point = egui::Pos2;
    /// `Size` = `Vec2` (chaudai, unchai).
    pub type Size = egui::Vec2;
}
