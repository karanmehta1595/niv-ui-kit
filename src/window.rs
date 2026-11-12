//! Window: a small floating window you can drag by its title. It has one button, the close `x`
//! (and `.esc()` lets the Esc key close it too). Nothing else: no minimise, no resize.
//!
//! ```ignore
//! Window("find").title("Find and Replace").size(vec2(380.0, 128.0)).esc()
//!     .show(ui, &mut self.find_open, |ui| { /* the content */ });
//! ```
//! It stays where the user dragged it. Clicking outside does not close it (use a Dropdown for that).

use crate::button::{Button, Label};
use crate::dropdown::Dropdown;
use crate::panel::inverse;
use egui::{pos2, vec2, Color32, Event, Id, Key, Layout, Order, Pos2, Rect, Response, Sense, Ui, UiBuilder, Vec2};

/// `Window("id")`: see the module docs.
#[allow(non_snake_case)]
pub fn Window(id: impl Into<String>) -> Window {
    Window {
        id: id.into(),
        title: String::new(),
        size: vec2(360.0, 200.0),
        at: None,
        bounds: None,
        padding: 12.0,
        esc: false,
        separate: false,
        attached: None,
        blur: None,
        hide_titlebar: false,
        minimize: true,
        maximize: true,
        child: false,
    }
}

/// Which edge of the app window an attached panel grows out of: `Outside::left().size(240.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outside {
    edge: Edge,
    size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Outside {
    /// The panel grows out of the left edge.
    pub fn left() -> Self {
        Self { edge: Edge::Left, size: 240.0 }
    }
    /// The panel grows out of the right edge.
    pub fn right() -> Self {
        Self { edge: Edge::Right, size: 240.0 }
    }
    /// The panel grows out of the top edge.
    pub fn top() -> Self {
        Self { edge: Edge::Top, size: 160.0 }
    }
    /// The panel grows out of the bottom edge.
    pub fn bottom() -> Self {
        Self { edge: Edge::Bottom, size: 160.0 }
    }
    /// How far the app window grows (width for left / right, height for top / bottom).
    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }
}

/// A floating, draggable window made with `Window("id")`.
#[must_use = "end Window with .show(ui, &mut open, |ui| ..)"]
pub struct Window {
    id: String,
    title: String,
    size: Vec2,
    at: Option<Pos2>,
    bounds: Option<Rect>,
    padding: f32,
    esc: bool,
    separate: bool,
    attached: Option<Outside>,
    blur: Option<u8>,
    hide_titlebar: bool,
    minimize: bool,
    maximize: bool,
    child: bool,
}

/// How tall the strip at the top of a window without a title bar is (drag it to move the window); the traffic
/// light buttons sit inside it.
const DRAG_STRIP: f32 = 36.0;

#[derive(Clone, Copy, Default)]
struct State {
    pos: Option<Pos2>,
    /// A widget inside had the keyboard last frame (so Esc is meant for this window).
    focus_inside: bool,
}

impl Window {
    /// The text in the title strip (drag by it).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Width and height of the window.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    /// Where the top-left corner starts (the user can drag it away; it is then remembered).
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.at = Some(pos2(x, y));
        self
    }

    /// The window cannot be dragged out of this rect (default: the whole screen).
    pub fn bounds(mut self, rect: Rect) -> Self {
        self.bounds = Some(rect);
        self
    }

    /// Space between the card edge and the content.
    pub fn padding(mut self, px: f32) -> Self {
        self.padding = px;
        self
    }

    /// The Esc key closes this window too (when the keyboard is inside it).
    pub fn esc(mut self) -> Self {
        self.esc = true;
        self
    }

    /// A real window of its own (own title bar, can leave the app window), still inside the same process.
    /// Closing it with the red dot, Cmd+W or Esc sets `open` to false. `at` and `bounds` do not apply.
    /// Look: `.blur(60).hide_titlebar().minimize_button(false).maximize_button(false)`.
    pub fn separate(mut self) -> Self {
        self.separate = true;
        self
    }

    /// (With `.separate()`.) Blur of what lies behind the window, `0` (none) to `100` (a lot), like mainkit's
    /// `.panel_blur(..)`. It makes the window transparent and paints a translucent base, so the blur shows. macOS only
    /// (elsewhere the window just stays translucent).
    pub fn blur(mut self, amount: u8) -> Self {
        self.blur = Some(amount.min(100));
        self
    }

    /// (With `.separate()`.) No title bar and no title: the content reaches the top of the window (macOS style, like
    /// mainkit's `.hide_titlebar()`). The top strip stays draggable, and the close button stays.
    pub fn hide_titlebar(mut self) -> Self {
        self.hide_titlebar = true;
        self
    }

    /// (With `.separate()`.) A child of the app window: it always stays in front of it, goes away and comes back
    /// with it when the app is minimised and restored, and moves with it. macOS only (other systems: a plain window).
    pub fn child(mut self) -> Self {
        self.child = true;
        self
    }

    /// (With `.separate()`.) Show the minimise (yellow) button or not (default: yes).
    pub fn minimize_button(mut self, show: bool) -> Self {
        self.minimize = show;
        self
    }

    /// (With `.separate()`.) Show the maximise (green) button or not (default: yes).
    pub fn maximize_button(mut self, show: bool) -> Self {
        self.maximize = show;
        self
    }

    /// Instead of floating, the content becomes a panel on one side of the app window and the app window
    /// grows by that much (like the history of the Calculator); `open = false` shrinks it back. Call it
    /// before the rest of the app's UI, so the rest gets the space that is left:
    /// `Window("history").attached(Outside::left().size(240.0)).show(ui, &mut open, |ui| ..)`.
    pub fn attached(mut self, outside: Outside) -> Self {
        self.attached = Some(outside);
        self
    }

    /// Shows the window while `open` is true; the close button (or Esc) sets it false.
    pub fn show<R>(self, ui: &mut Ui, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        if let Some(outside) = self.attached {
            return self.show_attached(ui, outside, open, add);
        }
        self.show_ctx(ui.ctx(), open, add)
    }

    fn show_attached<R>(self, ui: &mut Ui, outside: Outside, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let ctx = ui.ctx().clone();
        let sid = Id::unique(("niv_window_attached", self.id.as_str()));
        let grown: bool = ctx.data_mut(|d| d.get_temp(sid)).unwrap_or(false);
        let delta = match outside.edge {
            Edge::Left | Edge::Right => vec2(outside.size, 0.0),
            Edge::Top | Edge::Bottom => vec2(0.0, outside.size),
        };
        // Window grows (or shrinks back) once, when `open` changes; left / top also move the window so the app stays where it was.
        if *open != grown {
            let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
            if let (Some(inner), Some(outer)) = (inner, outer) {
                let k = if *open { 1.0 } else { -1.0 };
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(inner.size() + delta * k));
                if matches!(outside.edge, Edge::Left | Edge::Top) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(outer.min - delta * k));
                }
                ctx.data_mut(|d| d.insert_temp(sid, *open));
            }
        }
        if !*open {
            return None;
        }
        let id = format!("niv_attached_{}", self.id);
        let frame = egui::Frame::NONE.inner_margin(self.padding);
        let panel = match outside.edge {
            Edge::Left => egui::Panel::left(id).exact_size(outside.size),
            Edge::Right => egui::Panel::right(id).exact_size(outside.size),
            Edge::Top => egui::Panel::top(id).exact_size(outside.size),
            Edge::Bottom => egui::Panel::bottom(id).exact_size(outside.size),
        };
        Some(panel.frame(frame).resizable(false).show(ui, add).inner)
    }

    fn show_separate<R>(self, ctx: &egui::Context, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let mut builder = egui::ViewportBuilder::default()
            .with_title(self.title.clone())
            .with_inner_size(self.size)
            .with_min_inner_size(self.size * 0.8)
            .with_minimize_button(self.minimize)
            .with_maximize_button(self.maximize);
        if self.hide_titlebar {
            builder = builder.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
        }
        if self.blur.is_some() {
            builder = builder.with_transparent(true);
        }
        let (pad, esc, blur, hide, child) = (self.padding + 8.0, self.esc, self.blur, self.hide_titlebar, self.child);
        let (title, id) = (self.title.clone(), self.id.clone());
        let mut add = Some(add);
        let mut out = None;
        let mut close = false;
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of(("niv_separate_window", self.id.as_str())), builder, |ui, class| {
            if let Some(add) = add.take() {
                if class != egui::ViewportClass::EmbeddedWindow {
                    match blur {
                        // transparent window: a translucent base (dark / light), the blur shows through it
                        Some(amount) => {
                            let base = if crate::theme::is_dark(ui) { Color32::from_rgba_unmultiplied(28, 28, 30, 155) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 195) };
                            ui.painter().rect_filled(ui.max_rect(), 0.0, base);
                            crate::mac_window::set_blur(&title, amount);
                        }
                        None => crate::theme::paint_background(ui),
                    }
                    if child {
                        crate::mac_window::attach_to_app_window(&title);
                    }
                    if hide {
                        // made before the content, so a button on the strip still gets its own click
                        let screen = ui.ctx().content_rect();
                        let strip = Rect::from_min_size(screen.min, vec2(screen.width(), DRAG_STRIP));
                        let drag = ui.interact(strip, Id::unique(("niv_window_drag", id.as_str())), Sense::click_and_drag());
                        if drag.drag_started() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                }
                let mut inner = ui.max_rect().shrink(pad);
                if hide {
                    inner.min.y = ui.max_rect().min.y + DRAG_STRIP;
                }
                out = Some(ui.scope_builder(UiBuilder::new().max_rect(inner), add).inner);
            }
            close = ui.ctx().input(|i| i.viewport().close_requested() || (esc && i.key_pressed(Key::Escape)));
        });
        if close {
            *open = false;
        }
        out
    }

    /// Like `show`, from a `Context`.
    pub fn show_ctx<R>(self, ctx: &egui::Context, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        if !*open {
            return None;
        }
        if self.separate {
            return self.show_separate(ctx, open, add);
        }
        let ctx = ctx.clone();
        let sid = Id::unique(("niv_window_state", self.id.as_str()));
        let mut st: State = ctx.data_mut(|d| d.get_temp(sid)).unwrap_or_default();
        // Esc: read from the raw events, because other code may already have used the key this frame.
        let esc = self.esc
            && st.focus_inside
            && ctx.input(|i| i.raw.events.iter().any(|e| matches!(e, Event::Key { key: Key::Escape, pressed: true, repeat: false, .. })));
        if esc {
            *open = false;
            st.focus_inside = false;
            ctx.data_mut(|d| d.insert_temp(sid, st));
            return None;
        }
        let bounds = self.bounds.unwrap_or_else(|| ctx.content_rect());
        let start = st.pos.or(self.at).unwrap_or_else(|| pos2(bounds.center().x - self.size.x / 2.0, bounds.min.y + 60.0));
        let clamp = |p: Pos2| {
            pos2(
                p.x.clamp(bounds.min.x, (bounds.max.x - self.size.x).max(bounds.min.x)),
                p.y.clamp(bounds.min.y, (bounds.max.y - self.size.y).max(bounds.min.y)),
            )
        };
        let pos = clamp(start);
        let rect = Rect::from_min_size(pos, self.size);
        let id = Id::unique(("niv_window", self.id.as_str()));
        let frame = Dropdown::default_frame(ctx.global_style().visuals.dark_mode).size(self.size);
        let padding = self.padding;
        let mut result = None;
        let mut moved = Vec2::ZERO;
        let mut close = false;
        let title = self.title.clone();
        egui::Area::new(id).order(Order::Foreground).fixed_pos(pos).show(&ctx, |area_ui| {
            // click + drag: the window's body takes the pointer, so a click on it never reaches what lies under the window
            let (card, _) = area_ui.allocate_exact_size(self.size, Sense::click_and_drag());
            let measured = frame.measure(area_ui);
            let _ = frame.paint(area_ui, id.with("frame"), card, &measured);
            let mut inner = area_ui.new_child(UiBuilder::new().max_rect(card.shrink(padding)).layout(Layout::top_down(egui::Align::Min)));
            inner.set_clip_rect(card.shrink(padding / 2.0).intersect(inner.clip_rect()));
            let ui = &mut inner;
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            // macOS style: the red close dot on the left, the title in the middle
            let w = ui.available_width();
            let mut strip = Rect::NOTHING;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if close_dot(ui, &format!("{}-close", self.id)).clicked() {
                    close = true;
                }
                let grip = Button(None)
                    .id(format!("{}-grip", self.id))
                    .draggable()
                    .size(vec2(w - 44.0, 22.0))
                    .radius(6.0)
                    .fill(inverse)
                    .fill_opacity(0)
                    .text(Label(title.clone()).size(12.0).bold().center().color(inverse).opacity(0.75))
                    .show(ui);
                if grip.dragged() {
                    moved = grip.drag_delta();
                }
                strip = grip.rect;
            });
            let _ = strip;
            result = Some(add(ui));
        });
        // does the keyboard sit in a widget inside this window? (for Esc next frame)
        // (not `Context::read_response`: reading a response there drops the text cursor of a field that was just clicked)
        st.focus_inside = ctx
            .memory(|m| m.focused())
            .and_then(|f| ctx.viewport(|v| v.this_pass.widgets.get(f).map(|w| w.interact_rect)))
            .is_some_and(|r| rect.contains(r.center()));
        if moved != Vec2::ZERO {
            st.pos = Some(clamp(pos + moved));
        } else if st.pos.is_none() {
            st.pos = Some(pos);
        }
        ctx.data_mut(|d| d.insert_temp(sid, st));
        if close {
            *open = false;
        }
        result
    }
}

/// A small red close button like the one in a macOS window: a plain red dot, and the x only while the pointer is on it
/// (`close_dot(ui, "id").clicked()`). Windows use it; put it in a dropdown's corner for a close button.
pub fn close_dot(ui: &mut Ui, id: &str) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let response = ui.interact(rect, Id::unique(("niv_close_dot", id)), Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let fill = if pressed { Color32::from_rgb(191, 73, 67) } else { Color32::from_rgb(255, 95, 87) };
    let radius = 6.5;
    ui.painter().circle_filled(rect.center(), radius, fill);
    ui.painter().circle_stroke(rect.center(), radius, egui::Stroke::new(0.5, Color32::from_rgb(224, 68, 62)));
    if response.hovered() {
        let ink = egui::Stroke::new(1.5, Color32::from_rgb(77, 0, 0));
        let d = 2.4;
        let c = rect.center();
        ui.painter().line_segment([c + vec2(-d, -d), c + vec2(d, d)], ink);
        ui.painter().line_segment([c + vec2(-d, d), c + vec2(d, -d)], ink);
    }
    response
}
