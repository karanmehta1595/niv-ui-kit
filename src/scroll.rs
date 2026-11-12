//! `Scroll`: scroll area jisme aakhir ke baad extra khali jagah (`scroll_space`) mil sakti hai.

use egui::{Rect, Response, ScrollArea, Sense, Ui, Vec2};

/// Scroll ke aakhir ki extra jagah. `Bottom::topdown_space(36.0 * 8.0)`: content ke neeche 8 rows jitni jagah.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bottom {
    px: f32,
}

impl Bottom {
    /// Content ke neeche `px` pixel khali jagah (hisaab chalta hai: `36.0 * 8.0`).
    pub fn topdown_space(px: f32) -> Self {
        Self { px }
    }
}

/// Scroll area (bachi hui poori jagah leta hai). `.scroll_space(..)` se aakhir ke baad bhi scroll hota rehta hai.
pub struct Scroll {
    area: ScrollArea,
    space: Bottom,
    shrink: bool,
}

/// `Scroll::show` ka jawab.
pub struct ScrollOutput<R> {
    /// Tumhare closure ka return.
    pub inner: R,
    /// Aakhir ki extra khali jagah (click pakadne ke liye).
    pub space: Response,
    /// Poore content ki naap (extra jagah samet).
    pub content_size: Vec2,
    /// Dikhne wala hissa.
    pub viewport: Rect,
}

impl Scroll {
    fn new(area: ScrollArea) -> Self {
        Self { area, space: Bottom::default(), shrink: false }
    }

    /// Upar-neeche scroll.
    pub fn vertical() -> Self {
        Self::new(ScrollArea::vertical())
    }

    /// Left-right scroll.
    pub fn horizontal() -> Self {
        Self::new(ScrollArea::horizontal())
    }

    /// Dono taraf scroll.
    pub fn both() -> Self {
        Self::new(ScrollArea::both())
    }

    /// Ek scroll ko doosre se alag pehchaanne ke liye naam (ek hi jagah kai scroll hon to).
    pub fn id(mut self, name: impl Into<String>) -> Self {
        self.area = self.area.id_salt(name.into());
        self
    }

    /// Scroll bar kabhi na dikhe (scroll phir bhi hota hai).
    pub fn hide_bar(mut self) -> Self {
        self.area = self.area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        self
    }

    /// Content jitna hi bada ho (bachi hui poori jagah na le): tabs ki patti jaisi cheezon ke liye, jinke neeche kuch aur aana hai.
    /// Iske bina `Scroll` bachi hui saari jagah le leta hai aur uske baad ka sab neeche chala jata hai.
    pub fn shrink(mut self) -> Self {
        self.shrink = true;
        self
    }

    /// Content ke aakhir ke baad itni khali jagah aur scroll hoti hai: `.scroll_space(Bottom::topdown_space(36.0 * 8.0))`.
    pub fn scroll_space(mut self, space: Bottom) -> Self {
        self.space = space;
        self
    }

    /// Content dikhao.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> ScrollOutput<R> {
        let space = self.space;
        let mut extra = None;
        let out = self.area.auto_shrink(self.shrink).show(ui, |ui| {
            let inner = add_contents(ui);
            extra = Some(ui.allocate_response(Vec2::new(0.0, space.px.max(0.0)), Sense::click()));
            inner
        });
        ScrollOutput {
            inner: out.inner,
            space: extra.expect("scroll content ran"),
            content_size: out.content_size,
            viewport: out.inner_rect,
        }
    }
}
