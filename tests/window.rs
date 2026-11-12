use niv_ui_kit::prelude::*;
use niv_ui_kit::egui::{self, Event, Key, Modifiers, RawInput};

fn frame(ctx: &egui::Context, events: Vec<Event>, f: impl FnMut(&mut egui::Ui)) {
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
    let out = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, f);
    std::mem::forget(out);
}

#[test]
fn a_window_shows_and_esc_closes_it_when_the_keyboard_is_inside() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut open = true;
    let mut text = String::new();
    for i in 0..3 {
        frame(&ctx, vec![], |ui| {
            Window("w1").title("Test").size(vec2(300.0, 120.0)).at(100.0, 100.0).esc().show(ui, &mut open, |ui| {
                let mut f = TextField(&mut text).id("w1-field");
                if i == 0 {
                    f = f.focus();
                }
                f.show(ui);
            });
        });
    }
    assert!(open);
    let esc = Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
    frame(&ctx, vec![esc], |ui| {
        Window("w1").title("Test").size(vec2(300.0, 120.0)).at(100.0, 100.0).esc().show(ui, &mut open, |_| {});
    });
    assert!(!open, "Esc did not close the window");
}

#[test]
fn without_esc_the_window_ignores_the_key_and_the_x_closes_it() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut open = true;
    let esc = Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
    for _ in 0..2 {
        frame(&ctx, vec![], |ui| {
            Window("w2").size(vec2(300.0, 120.0)).at(100.0, 100.0).show(ui, &mut open, |_| {});
        });
    }
    frame(&ctx, vec![esc], |ui| {
        Window("w2").size(vec2(300.0, 120.0)).at(100.0, 100.0).show(ui, &mut open, |_| {});
    });
    assert!(open);
    // click the x (top left of the card, inside its padding)
    let x = egui::pos2(100.0 + 12.0 + 8.0, 100.0 + 12.0 + 11.0);
    let press = |pressed| Event::PointerButton { pos: x, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    for events in [vec![Event::PointerMoved(x)], vec![press(true)], vec![press(false)]] {
        frame(&ctx, events, |ui| {
            Window("w2").size(vec2(300.0, 120.0)).at(100.0, 100.0).show(ui, &mut open, |_| {});
        });
    }
    assert!(!open, "the x button did not close the window");
}

#[test]
fn a_click_on_a_field_inside_a_window_keeps_the_cursor() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let (mut open, mut a, mut b) = (true, String::new(), String::new());
    let mut rect = egui::Rect::NOTHING;
    let run = |events: Vec<Event>, a: &mut String, b: &mut String, open: &mut bool, rect: &mut egui::Rect| {
        frame(&ctx, events, |ui| {
            Window("w3").title("Test").size(vec2(300.0, 160.0)).at(100.0, 100.0).show(ui, open, |ui| {
                TextField(a).id("w3-a").show(ui);
                *rect = TextField(b).id("w3-b").show(ui).rect;
            });
        });
    };
    for _ in 0..3 {
        run(vec![], &mut a, &mut b, &mut open, &mut rect);
    }
    let at = rect.center();
    let press = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    for events in [vec![Event::PointerMoved(at)], vec![press(true)], vec![press(false)], vec![]] {
        run(events, &mut a, &mut b, &mut open, &mut rect);
    }
    run(vec![Event::Text("hi".into())], &mut a, &mut b, &mut open, &mut rect);
    assert_eq!((a.as_str(), b.as_str()), ("", "hi"));
}

#[test]
fn an_attached_window_takes_a_side_of_the_app_and_leaves_the_rest() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut open = true;
    let mut rest = Rect::NOTHING;
    for _ in 0..2 {
        frame(&ctx, vec![], |ui| {
            Window("history").attached(Outside::left().size(200.0)).show(ui, &mut open, |ui| {
                ui.label("History");
            });
            rest = ui.available_rect_before_wrap();
        });
    }
    assert!((rest.min.x - 200.0).abs() < 2.0, "the app got {rest:?}");
    open = false;
    frame(&ctx, vec![], |ui| {
        Window("history").attached(Outside::left().size(200.0)).show(ui, &mut open, |_| {});
        rest = ui.available_rect_before_wrap();
    });
    assert!(rest.min.x < 2.0, "closed attached window still takes room: {rest:?}");
}

#[test]
fn a_separate_window_with_blur_and_no_titlebar_shows_its_content() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut open = true;
    let mut shown = false;
    frame(&ctx, vec![], |ui| {
        Window("sep")
            .title("Settings")
            .size(vec2(300.0, 200.0))
            .separate()
            .blur(60)
            .hide_titlebar()
            .minimize_button(false)
            .maximize_button(false)
            .child()
            .show(ui, &mut open, |_| shown = true);
    });
    assert!(shown, "the content of the separate window was not built");
    assert!(open);
}
