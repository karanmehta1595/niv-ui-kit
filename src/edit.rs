//! CodeEdit: code / lambe text ka editor (ek ya kai line), aur uske cursor ke helpers.
//!
//! ```ignore
//! let out = CodeEdit(&mut doc.text).id(id).font(font).lock_focus().full_width()
//!     .min_size(vec2(400.0, 300.0)).layouter(&mut layouter).show(ui);
//! if out.response.changed() { /* text badla */ }
//! let caret = out.cursor_range;                 // Option<CCursorRange>
//! let galley = out.galley;                      // har row ki jagah
//! CodeEdit(&mut name).single_line().id(id).align(Align2::LEFT_CENTER).show(ui); // ek line
//!
//! selection(ctx, id)            // Some((start, end)) chars mein
//! set_selection(ctx, id, 0, 4)  // 0..4 chars chuno
//! caret_ends(ctx, id)           // (anchor, head): sirf seedha dikha, direction ke saath
//! ```
//! Background nahi banta (aap khud banao), kyunki code editor ka apna rang/gutter hota hai.
//! Chhote search/naam wale box ke liye `TextField` hai.

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditOutput;
use egui::{Align2, Context, FontId, Frame, Galley, Id, TextBuffer, TextEdit, Ui, Vec2};
use std::sync::Arc;

/// Row banane wala: text ke tukde ko `Galley` mein badalta hai.
pub type Layouter<'a> = dyn FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> + 'a;

/// `CodeEdit(&mut String)`: dekho module docs.
#[allow(non_snake_case)]
pub fn CodeEdit(text: &mut String) -> CodeEdit<'_> {
    CodeEdit { text, id: None, font: None, lock_focus: false, full_width: false, min_size: None, single: false, align: None, layouter: None }
}

/// Text editor builder.
#[must_use = "aakhir mein .show(ui) likho"]
pub struct CodeEdit<'a> {
    text: &'a mut String,
    id: Option<Id>,
    font: Option<FontId>,
    lock_focus: bool,
    full_width: bool,
    min_size: Option<Vec2>,
    single: bool,
    align: Option<Align2>,
    layouter: Option<&'a mut Layouter<'a>>,
}

impl<'a> CodeEdit<'a> {
    /// Pehchaan (cursor ke helpers isi se chalte hain).
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
    /// Font.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    /// Tab key focus na chhode (code mein Tab likhna).
    pub fn lock_focus(mut self) -> Self {
        self.lock_focus = true;
        self
    }
    /// Poori chaudai lo.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
    /// Kam se kam itna bada.
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = Some(size);
        self
    }
    /// Sirf ek line.
    pub fn single_line(mut self) -> Self {
        self.single = true;
        self
    }
    /// Text ka alignment (ek line mein).
    pub fn align(mut self, align: Align2) -> Self {
        self.align = Some(align);
        self
    }
    /// Text kaise rang/rakha jaye (syntax colours): har row ka `Galley` banao.
    pub fn layouter(mut self, layouter: &'a mut Layouter<'a>) -> Self {
        self.layouter = Some(layouter);
        self
    }
    /// Dikhao. Natija: `response`, `galley`, `galley_pos`, `cursor_range` ...
    pub fn show(self, ui: &mut Ui) -> TextEditOutput {
        let mut edit = if self.single { TextEdit::singleline(self.text) } else { TextEdit::multiline(self.text) };
        edit = edit.frame(Frame::NONE).lock_focus(self.lock_focus);
        if let Some(id) = self.id {
            edit = edit.id(id);
        }
        if let Some(font) = self.font {
            edit = edit.font(font);
        }
        if self.full_width {
            edit = edit.desired_width(f32::INFINITY);
        }
        if let Some(size) = self.min_size {
            edit = edit.min_size(size);
        }
        if let Some(align) = self.align {
            edit = edit.align(align);
        }
        if let Some(layouter) = self.layouter {
            edit = edit.layouter(layouter);
        }
        edit.show(ui)
    }
}

/// Chuna hua hissa `(start, end)` chars mein (start <= end). Cursor ho par kuch chuna na ho to `start == end`.
pub fn selection(ctx: &Context, id: Id) -> Option<(usize, usize)> {
    let (a, b) = caret_ends(ctx, id)?;
    Some((a.min(b), a.max(b)))
}

/// `(anchor, head)`: jahan se chunna shuru kiya aur jahan cursor hai (direction bachi rehti hai).
pub fn caret_ends(ctx: &Context, id: Id) -> Option<(usize, usize)> {
    let range = TextEdit::load_state(ctx, id)?.cursor.char_range()?;
    Some((range.secondary.index.into(), range.primary.index.into()))
}

/// `start..end` chars chuno (`start == end`: sirf cursor wahan).
pub fn set_selection(ctx: &Context, id: Id, start: usize, end: usize) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(start), CCursor::new(end))));
    state.store(ctx, id);
}
