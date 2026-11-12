use niv_ui_kit::egui::{self, Event, Modifiers, PointerButton, RawInput};
use niv_ui_kit::prelude::*;

#[test]
fn a_group_grows_its_background_on_hover_and_pushes_the_pressed_button() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0));
    let mut clicked = None;
    let at = pos2(40.0, 30.0);
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    for events in [vec![], vec![Event::PointerMoved(at)], vec![Event::PointerMoved(at)], vec![press(true)], vec![press(false)]] {
        let out = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
            let g = Locate(Group("g").size(vec2(120.0, 36.0)).button(Button(None).id("a").size(vec2(30.0, 30.0))).button(Button(None).id("b").size(vec2(30.0, 30.0)))
                .scale(State::pop_px(3.0).hovered())
                .scale(State::push_px(2.0).clicked()))
            .left(10.0)
            .top(10.0)
            .show(ui);
            if g.clicked(0) {
                clicked = Some(0);
            }
        });
        std::mem::forget(out);
    }
    assert_eq!(clicked, Some(0));
}
