//! Row aur Column: cheezein ek line mein (left se right) ya ek ke neeche ek rakhne ke liye.
//!
//! ```ignore
//! Row().gap(8.0).show(ui, |ui| {
//!     Button("A").show(ui);
//!     Button("B").show(ui);
//! });
//!
//! Column().gap(8.0).align(Align::Center).show(ui, |ui| { /* ek ke neeche ek, beech mein */ });
//! Row().reverse().show(ui, |ui| { /* right se left */ });
//! Column().reverse().show(ui, |ui| { /* neeche se upar */ });
//! ```

use egui::{vec2, Direction, Id, InnerResponse, Layout, Ui};

/// Row ya Column ke andar cheezein kis taraf sateen (cross direction mein).
/// Row mein `Top`/`Center`/`Bottom` (upar-neeche), Column mein `Left`/`Center`/`Right` (aadi-tirchhi).
/// Dusri taraf ka naam dene par wahi jagah ka matlab mana jata hai (`Left` = `Top`, `Right` = `Bottom`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Upar (Row mein) ya left (Column mein).
    Top,
    /// Beech mein.
    #[default]
    Center,
    /// Neeche (Row mein) ya right (Column mein).
    Bottom,
    /// Left (Column mein) ya upar (Row mein).
    Left,
    /// Right (Column mein) ya neeche (Row mein).
    Right,
}

impl Align {
    fn to_egui(self) -> egui::Align {
        match self {
            Align::Top | Align::Left => egui::Align::Min,
            Align::Center => egui::Align::Center,
            Align::Bottom | Align::Right => egui::Align::Max,
        }
    }
}


/// Poora group (row ya column) apni line mein kahan baithe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Place {
    /// Left (Row) / upar (Column).
    #[default]
    Start,
    Center,
    /// Right (Row) / neeche (Column).
    End,
}

/// Group ko `place` par laane ke liye shuru mein kitni khali jagah chhodni hai. `extent` pichhli frame ki
/// naap hai (None ho to pehli frame, kuch nahi chhodte). `from_start`: layout apne shuru ke kinare (left/upar)
/// se bharta hai ya ulta (right/neeche se).
fn lead_space(place: Place, from_start: bool, available: f32, extent: Option<f32>) -> f32 {
    let Some(extent) = extent else {
        return 0.0;
    };
    let spare = (available - extent).max(0.0);
    match (place, from_start) {
        (Place::Center, _) => spare / 2.0,
        (Place::Start, true) | (Place::End, false) => 0.0,
        (Place::Start, false) | (Place::End, true) => spare,
    }
}

/// Bina egui ke beech-ki-doori (`item_spacing`) jode, ek hi dishaa mein `amount` khali jagah chhodo.
fn pad(ui: &mut Ui, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    let spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    ui.add_space(amount);
    ui.spacing_mut().item_spacing = spacing;
}

/// (shuru ki khali jagah, aakhir ki khali jagah) `gap_start` / `gap_end` / `gap_around` se.
/// `main_gap`: cheezon ke beech ki asli doori (is dishaa mein).
fn edge_gaps(start: f32, end: f32, around: bool, main_gap: f32) -> (f32, f32) {
    let (start, end) = if around { (start.max(main_gap), end.max(main_gap)) } else { (start, end) };
    // Aakhri cheez ke baad egui ki beech-ki-doori pehle se hoti hai, wo is jagah mein gini jati hai.
    (start, (end - main_gap).max(0.0))
}

/// Pichhli frame ki naap yaad rakhne ki jagah.
fn extent_id(ui: &Ui, kind: &str) -> Id {
    Id::unique((kind, ui.next_auto_id()))
}

fn stored_extent(ui: &Ui, id: Id) -> Option<f32> {
    ui.ctx().data_mut(|d| d.get_temp::<f32>(id))
}

fn store_extent(ui: &Ui, id: Id, extent: f32, previous: Option<f32>) {
    ui.ctx().data_mut(|d| d.insert_temp(id, extent));
    if previous.is_none_or(|p| (p - extent).abs() > 0.5) {
        ui.ctx().request_repaint();
    }
}

/// Cheezein ek line mein: left se right (`.reverse()` se right se left).
#[must_use = "Row ke aakhir mein .show(ui, |ui| { .. }) likho"]
#[derive(Clone, Copy, Debug)]
pub struct Row {
    gap: Option<f32>,
    gap_start: f32,
    gap_end: f32,
    gap_around: bool,
    align: Align,
    reverse: bool,
    place: Place,
}

/// `Row()`.
#[allow(non_snake_case)]
pub fn Row() -> Row {
    Row { gap: None, gap_start: 0.0, gap_end: 0.0, gap_around: false, align: Align::Center, reverse: false, place: Place::Start }
}

impl Row {
    /// Cheezon ke beech ki doori (px).
    pub fn gap(mut self, px: f32) -> Self {
        self.gap = Some(px.max(0.0));
        self
    }

    /// Pehli cheez se pehle itni khali jagah (left kinare se). `.reverse()` ho to jahan se layout shuru hota hai wahin se.
    pub fn gap_start(mut self, px: f32) -> Self {
        self.gap_start = px.max(0.0);
        self
    }

    /// Aakhri cheez ke baad itni khali jagah (right kinare tak). `.gap(..)` se kam nahi ho sakti.
    pub fn gap_end(mut self, px: f32) -> Self {
        self.gap_end = px.max(0.0);
        self
    }

    /// `.gap(..)` wali doori shuru aur aakhir mein bhi lagao (beech mein to lagti hi hai).
    pub fn gap_around(mut self) -> Self {
        self.gap_around = true;
        self
    }


    /// Cheezein row mein upar/beech/neeche: `Align::Top`, `Align::Center` (default), `Align::Bottom`.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Right se left.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Poori row left kinare par (default).
    pub fn left(mut self) -> Self {
        self.place = Place::Start;
        self
    }

    /// Poori row left-right beech mein.
    pub fn center(mut self) -> Self {
        self.place = Place::Center;
        self
    }

    /// Poori row right kinare par.
    pub fn right(mut self) -> Self {
        self.place = Place::End;
        self
    }

    /// Row banao aur uske andar ki cheezein `add` se jodo.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let align = self.align.to_egui();
        let layout = if self.reverse { Layout::right_to_left(align) } else { Layout::left_to_right(align) };
        let gap = self.gap;
        // Unchai kam se kam ek widget jitni: lambi cheezein row ko khud badha dete hain.
        let available = ui.available_width();
        let size = vec2(available, ui.spacing().interact_size.y);
        let place = self.place;
        let reverse = self.reverse;
        let (gap_start, gap_end, gap_around) = (self.gap_start, self.gap_end, self.gap_around);
        let id = extent_id(ui, "niv_row_extent");
        let previous = stored_extent(ui, id);
        let inner = ui.allocate_ui_with_layout(size, layout, |ui| {
            if let Some(gap) = gap {
                ui.spacing_mut().item_spacing = vec2(gap, gap);
            }
            if place != Place::Start || previous.is_some() {
                pad(ui, lead_space(place, !reverse, available, previous));
            }
            let edge = if reverse { ui.cursor().max.x } else { ui.cursor().min.x };
            let (start, end) = edge_gaps(gap_start, gap_end, gap_around, ui.spacing().item_spacing.x);
            pad(ui, start);
            let result = add(ui);
            pad(ui, end);
            let used = ui.min_rect();
            let extent = if reverse { edge - used.min.x } else { used.max.x - edge };
            (result, extent)
        });
        let (result, extent) = inner.inner;
        if place != Place::Start {
            store_extent(ui, id, extent, previous);
        }
        InnerResponse::new(result, inner.response)
    }
}

/// Cheezein ek ke neeche ek: upar se neeche (`.reverse()` se neeche se upar).
#[must_use = "Column ke aakhir mein .show(ui, |ui| { .. }) likho"]
#[derive(Clone, Copy, Debug)]
pub struct Column {
    gap: Option<f32>,
    gap_start: f32,
    gap_end: f32,
    gap_around: bool,
    align: Align,
    reverse: bool,
    place: Place,
}

/// `Column()`.
#[allow(non_snake_case)]
pub fn Column() -> Column {
    Column { gap: None, gap_start: 0.0, gap_end: 0.0, gap_around: false, align: Align::Left, reverse: false, place: Place::Start }
}

impl Column {
    /// Cheezon ke beech ki doori (px).
    pub fn gap(mut self, px: f32) -> Self {
        self.gap = Some(px.max(0.0));
        self
    }

    /// Pehli cheez se pehle itni khali jagah (upar kinare se). `.reverse()` ho to jahan se layout shuru hota hai wahin se.
    pub fn gap_start(mut self, px: f32) -> Self {
        self.gap_start = px.max(0.0);
        self
    }

    /// Aakhri cheez ke baad itni khali jagah (neeche kinare tak). `.gap(..)` se kam nahi ho sakti.
    pub fn gap_end(mut self, px: f32) -> Self {
        self.gap_end = px.max(0.0);
        self
    }

    /// `.gap(..)` wali doori shuru aur aakhir mein bhi lagao (beech mein to lagti hi hai).
    pub fn gap_around(mut self) -> Self {
        self.gap_around = true;
        self
    }


    /// Cheezein column mein left/beech/right: `Align::Left` (default), `Align::Center`, `Align::Right`.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Neeche se upar.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Poora column upar (default).
    pub fn top(mut self) -> Self {
        self.place = Place::Start;
        self
    }

    /// Poora column upar-neeche beech mein.
    pub fn center(mut self) -> Self {
        self.place = Place::Center;
        self
    }

    /// Poora column neeche.
    pub fn bottom(mut self) -> Self {
        self.place = Place::End;
        self
    }

    /// Column banao aur uske andar ki cheezein `add` se jodo.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let align = self.align.to_egui();
        let layout = if self.reverse { Layout::bottom_up(align) } else { Layout::top_down(align) };
        let gap = self.gap;
        let available = ui.available_height();
        let place = self.place;
        let reverse = self.reverse;
        let (gap_start, gap_end, gap_around) = (self.gap_start, self.gap_end, self.gap_around);
        let id = extent_id(ui, "niv_column_extent");
        let previous = stored_extent(ui, id);
        let inner = ui.with_layout(layout, |ui| {
            if let Some(gap) = gap {
                ui.spacing_mut().item_spacing = vec2(gap, gap);
            }
            if place != Place::Start || previous.is_some() {
                pad(ui, lead_space(place, !reverse, available, previous));
            }
            debug_assert!(matches!(ui.layout().main_dir(), Direction::TopDown | Direction::BottomUp));
            let edge = if reverse { ui.cursor().max.y } else { ui.cursor().min.y };
            let (start, end) = edge_gaps(gap_start, gap_end, gap_around, ui.spacing().item_spacing.y);
            pad(ui, start);
            let result = add(ui);
            pad(ui, end);
            let used = ui.min_rect();
            let extent = if reverse { edge - used.min.y } else { used.max.y - edge };
            (result, extent)
        });
        let (result, extent) = inner.inner;
        if place != Place::Start {
            store_extent(ui, id, extent, previous);
        }
        InnerResponse::new(result, inner.response)
    }
}

/// `rect` ke andar widgets rakho (layout mein jagah nahi leta): `put(ui, rect, |ui| Button("x").show(ui))`.
pub fn put<R>(ui: &mut Ui, rect: egui::Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(rect).layout(Layout::top_down(egui::Align::Min)),
    );
    add(&mut child)
}
