//! Switch aur TextField: click se badalte hain, likhne par text aata hai.

use niv_ui_kit::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use niv_ui_kit::prelude::*;

fn frame(ctx: &egui::Context, t: &mut f64, events: Vec<Event>, mut f: impl FnMut(&mut Ui)) {
    *t += 0.05;
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
        time: Some(*t),
        events,
        ..Default::default()
    };
    let out = ctx.run_ui(input, &mut f);
    std::mem::forget(out);
}

fn click(at: Pos2) -> Vec<Event> {
    let b = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    vec![Event::PointerMoved(at), b(true), b(false)]
}

#[test]
fn switch_flips_on_click() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let (mut t, mut on, mut rect) = (0.0, false, Rect::NOTHING);
    frame(&ctx, &mut t, vec![], |ui| rect = Switch(&mut on).show(ui).rect);
    assert!(!on);
    let mut changed = false;
    frame(&ctx, &mut t, click(rect.center()), |ui| changed = Switch(&mut on).show(ui).changed());
    assert!(on && changed);
    frame(&ctx, &mut t, click(rect.center()), |ui| {
        Switch(&mut on).show(ui);
    });
    assert!(!on);
}

#[test]
fn text_field_takes_typing_and_enter() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let (mut t, mut text, mut rect) = (0.0, String::new(), Rect::NOTHING);
    frame(&ctx, &mut t, vec![], |ui| rect = TextField(&mut text).hint("Search").id("q").show(ui).rect);
    // click inside: the caret comes in the box
    frame(&ctx, &mut t, click(rect.center()), |ui| {
        TextField(&mut text).hint("Search").id("q").show(ui);
    });
    let mut changed = false;
    frame(&ctx, &mut t, vec![Event::Text("rust".into())], |ui| changed = TextField(&mut text).hint("Search").id("q").show(ui).changed());
    assert_eq!(text, "rust");
    assert!(changed);
    let enter = Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
    let mut lost = false;
    frame(&ctx, &mut t, vec![enter], |ui| lost = TextField(&mut text).hint("Search").id("q").show(ui).lost_focus());
    assert!(lost);
}

#[test]
fn text_field_can_start_focused() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let (mut t, mut text) = (0.0, String::new());
    frame(&ctx, &mut t, vec![], |ui| {
        TextField(&mut text).id("f").focus().show(ui);
    });
    frame(&ctx, &mut t, vec![Event::Text("x".into())], |ui| {
        TextField(&mut text).id("f").show(ui);
    });
    assert_eq!(text, "x");
}

#[test]
fn menu_gives_back_the_chosen_act_and_closes() {
    use niv_ui_kit::menu::{Menu, MenuLine};
    let ctx = egui::Context::default();
    let mut open = true;
    let mut got: Option<u8> = None;
    let lines = vec![MenuLine::item("One", "", 1u8), MenuLine::Sep, MenuLine::check("Two", "K", 2u8, true), MenuLine::item_if("Three", "", 3u8, false)];
    let frame = |ctx: &egui::Context, events: Vec<egui::Event>, t: f64, open: &mut bool, got: &mut Option<u8>| {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 500.0))),
            time: Some(t),
            events,
            ..Default::default()
        };
        let out = ctx.run_ui(raw, |ui| {
            if let Some(a) = Menu("t").at(100.0, 100.0).width(200.0).show(ui, open, &lines) {
                *got = Some(a);
            }
        });
        std::mem::forget(out);
    };
    frame(&ctx, vec![], 0.0, &mut open, &mut got);
    frame(&ctx, vec![], 0.1, &mut open, &mut got);
    // "One" row: card top 100 + pad 6 -> centre y = 119
    let p = egui::pos2(150.0, 119.0);
    frame(&ctx, vec![egui::Event::PointerMoved(p)], 0.2, &mut open, &mut got);
    frame(&ctx, vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }], 0.3, &mut open, &mut got);
    frame(&ctx, vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], 0.4, &mut open, &mut got);
    assert_eq!(got, Some(1));
    assert!(!open);
}

#[test]
fn text_field_keeps_the_cursor_when_press_and_release_come_in_different_frames() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let (mut t, mut text, mut rect) = (0.0, String::new(), Rect::NOTHING);
    frame(&ctx, &mut t, vec![], |ui| rect = TextField(&mut text).id("q2").show(ui).rect);
    let at = rect.center();
    let b = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    for events in [vec![Event::PointerMoved(at)], vec![b(true)], vec![b(false)], vec![]] {
        frame(&ctx, &mut t, events, |ui| {
            TextField(&mut text).id("q2").show(ui);
        });
    }
    frame(&ctx, &mut t, vec![Event::Text("ok".into())], |ui| {
        TextField(&mut text).id("q2").show(ui);
    });
    assert_eq!(text, "ok");
}

#[test]
fn a_long_multiline_text_stays_inside_the_box() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut text = (0..60).map(|i| format!("line number {i}")).collect::<Vec<_>>().join("\n");
    let (mut top, mut bottom) = (0.0, 0.0);
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))), time: Some(0.1), ..Default::default() };
    let mut out = ctx.run_ui(input, |ui| {
        top = ui.cursor().top();
        TextField(&mut text).id("body").multiline().height(100.0).show(ui);
        bottom = ui.cursor().top();
    });
    out.textures_delta.clear();
    let rect = Rect::from_min_max(Pos2::new(0.0, top), Pos2::new(400.0, bottom));
    assert!((rect.height() - 100.0).abs() < 8.0, "the box keeps its height: {}", rect.height());
    let mut texts = 0;
    for clipped in &out.shapes {
        if matches!(clipped.shape, egui::epaint::Shape::Text(_)) {
            texts += 1;
            assert!(clipped.clip_rect.bottom() <= rect.bottom() + 1.0, "text is clipped to the box: {:?}", clipped.clip_rect);
        }
    }
    assert!(texts > 0);
}
