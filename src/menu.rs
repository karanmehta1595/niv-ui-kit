//! Menu: right-click / "..." / gear wala menu, liquid-glass card mein.
//!
//! ```ignore
//! let lines = vec![
//!     MenuLine::item("Copy", "Cmd+C", Act::Copy),
//!     MenuLine::Sep,
//!     MenuLine::check("Auto Update", "", Act::Auto, on),
//!     MenuLine::item_if("Uninstall", "", Act::Remove, can_remove),
//! ];
//! if let Some(act) = Menu("gear").at(x, y).width(260.0).ignore(button.rect).show(ui, &mut open, &lines) {
//!     /* act = jo line chuni gayi; menu apne aap band ho jata hai */
//! }
//! ```
//! Bahar click ya `Esc` par band. Disabled line dhundhli dikhti hai aur click nahi leti.

use crate::button::{Button, Hover, Label, Pressed};
use crate::dropdown::Dropdown;
use crate::panel::inverse;
use egui::{vec2, Rect, Ui};

/// Menu ki ek line.
pub enum MenuLine<T> {
    /// Ek item. `check`: `None` = seedha item, `Some(on)` = on/off wala item (on ho to tick).
    Item {
        /// Line ka text.
        label: String,
        /// Right side ka chhota text (shortcut).
        hint: String,
        /// Chunne par ye value wapas milti hai.
        act: T,
        /// `false` ho to dhundhli, click nahi leti.
        enabled: bool,
        /// Tick ki halat.
        check: Option<bool>,
    },
    /// Patli lakeer.
    Sep,
}

impl<T> MenuLine<T> {
    /// Seedha item.
    pub fn item(label: impl Into<String>, hint: impl Into<String>, act: T) -> Self {
        MenuLine::Item { label: label.into(), hint: hint.into(), act, enabled: true, check: None }
    }

    /// Item jo `enabled` false ho to dhundhla rehta hai.
    pub fn item_if(label: impl Into<String>, hint: impl Into<String>, act: T, enabled: bool) -> Self {
        MenuLine::Item { label: label.into(), hint: hint.into(), act, enabled, check: None }
    }

    /// On/off item: `on` ho to aage tick dikhta hai.
    pub fn check(label: impl Into<String>, hint: impl Into<String>, act: T, on: bool) -> Self {
        MenuLine::Item { label: label.into(), hint: hint.into(), act, enabled: true, check: Some(on) }
    }

    /// Disabled on/off item.
    pub fn check_if(label: impl Into<String>, act: T, on: bool, enabled: bool) -> Self {
        MenuLine::Item { label: label.into(), hint: String::new(), act, enabled, check: Some(on) }
    }
}

/// Shuru, aakhir aur do ek saath wali lakeeron ko hatao (optional groups chhodne par bachi lakeer na dikhe).
pub fn tidy<T>(lines: Vec<MenuLine<T>>) -> Vec<MenuLine<T>> {
    let mut out: Vec<MenuLine<T>> = Vec::with_capacity(lines.len());
    for line in lines {
        if matches!(line, MenuLine::Sep) && matches!(out.last(), None | Some(MenuLine::Sep)) {
            continue;
        }
        out.push(line);
    }
    while matches!(out.last(), Some(MenuLine::Sep)) {
        out.pop();
    }
    out
}

const ROW: f32 = 26.0;
const SEP: f32 = 9.0;
const PAD: f32 = 6.0;

/// `Menu("id")`: dekho module docs.
#[allow(non_snake_case)]
pub fn Menu(id: impl Into<String>) -> Menu {
    Menu { id: id.into(), at: (0.0, 0.0), width: 240.0, ignore: Vec::new(), bold: false }
}

/// Glass menu, `Menu("id")` se banao.
#[must_use = "Menu ke aakhir mein .show(ui, &mut open, &lines) likho"]
pub struct Menu {
    id: String,
    at: (f32, f32),
    width: f32,
    ignore: Vec<Rect>,
    bold: bool,
}

impl Menu {
    /// Menu ka top-left kona.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.at = (x, y);
        self
    }

    /// Menu ki chaudai (px).
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// Lines ke naam bold mein.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Is rect par click ko bahar ka click na maano (kholne wala button).
    pub fn ignore(mut self, rect: Rect) -> Self {
        self.ignore.push(rect);
        self
    }

    /// Menu dikhao. Line chuni gayi to uski `act` milti hai aur `open` false ho jata hai.
    pub fn show<T: Clone>(self, ui: &Ui, open: &mut bool, lines: &[MenuLine<T>]) -> Option<T> {
        if !*open {
            return None;
        }
        let height = PAD * 2.0
            + lines.iter().map(|l| if matches!(l, MenuLine::Sep) { SEP } else { ROW }).sum::<f32>();
        let mut dropdown = Dropdown(format!("niv_menu_{}", self.id))
            .at(self.at.0, self.at.1)
            .size(vec2(self.width, height))
            .padding(PAD);
        for rect in &self.ignore {
            dropdown = dropdown.ignore(*rect);
        }
        let picked = dropdown
            .show(ui, open, |ui| {
                let mut picked: Option<T> = None;
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, line) in lines.iter().enumerate() {
                    match line {
                        MenuLine::Sep => {
                            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), SEP), egui::Sense::hover());
                            let y = rect.center().y;
                            let color = ui.visuals().text_color().gamma_multiply(0.18);
                            ui.painter().line_segment(
                                [egui::pos2(rect.min.x + 8.0, y), egui::pos2(rect.max.x - 8.0, y)],
                                egui::Stroke::new(1.0, color),
                            );
                        }
                        MenuLine::Item { label, hint, act, enabled, check } => {
                            let mut row = Button(None).full_width().height(ROW).radius(8.0).fill(inverse).fill_opacity(0).id(format!("row{index}"));
                            if *enabled {
                                row = row
                                    .hover(Hover().fill(inverse).fill_opacity(20))
                                    .pressed(Pressed().fill(inverse).fill_opacity(32));
                            }
                            let shade = if *enabled { 1.0 } else { 0.38 };
                            if *check == Some(true) {
                                row = row.text(Label("\u{2714}").size(11.0).left(9.0).color(inverse).opacity(shade));
                            }
                            let x = if check.is_some() { 28.0 } else { 12.0 };
                            let mut name = Label(label.as_str()).size(13.0).left(x).color(inverse).opacity(shade);
                            if self.bold {
                                name = name.bold();
                            }
                            row = row.text(name);
                            if !hint.is_empty() {
                                row = row.text(Label(hint.as_str()).size(11.0).right(12.0).color(inverse).opacity(0.5 * shade));
                            }
                            if row.show(ui).clicked() && *enabled {
                                picked = Some(act.clone());
                            }
                        }
                    }
                }
                picked
            })
            .flatten();
        if picked.is_some() {
            *open = false;
        }
        picked
    }
}
