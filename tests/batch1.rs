//! Canvas, Img, Mem, Input, Popup, Tip: ek frame chala ke dekho ki ban raha hai.

use niv_ui_kit::prelude::*;

fn frame<R>(mut build: impl FnMut(&mut Ui) -> R) -> (usize, R) {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))), ..Default::default() };
    let mut result = None;
    let out = ctx.run_ui(input, |ui| result = Some(build(ui)));
    let n = out.shapes.len();
    std::mem::forget(out);
    (n, result.unwrap())
}

#[test]
fn canvas_draws_when_the_statement_ends() {
    let (empty, _) = frame(|_| ());
    let (n, _) = frame(|ui| {
        let r = Rect::from_min_size(pos2(10.0, 10.0), vec2(50.0, 50.0));
        let c = Canvas(ui);
        c.rect(r).fill("#222222").stroke(1.0, "#ffffff").radius(8.0);
        c.line(pos2(0.0, 0.0), pos2(40.0, 40.0)).width(2.0).color("#ff0000");
        c.circle(pos2(30.0, 30.0), 5.0).fill(Color::BLUE);
        c.polygon(&[pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(5.0, 9.0)]).fill("#00aa00");
        c.text(pos2(5.0, 5.0), "hi").size(12.0).bold().color("#ffffff");
    });
    assert!(n >= empty + 6, "{n} shapes, empty frame {empty}");
}

#[test]
fn canvas_skips_invisible_and_degenerate_shapes() {
    let (empty, _) = frame(|_| ());
    let (n, _) = frame(|ui| {
        let c = Canvas(ui);
        c.rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(5.0, 5.0)));
        c.line(pos2(0.0, 0.0), pos2(1.0, 1.0)).width(0.0);
        c.polygon(&[pos2(0.0, 0.0), pos2(1.0, 1.0)]).fill("#fff");
    });
    assert_eq!(n, empty);
}

#[test]
fn img_from_pixels_takes_its_own_size_and_fits_inside_the_box() {
    let pixels = vec![255u8; 8 * 4 * 4]; // 8 x 4
    let img = Img::rgba(8, 4, pixels);
    let (_, (size, shown)) = frame(|ui| {
        let size = img.size(ui.ctx());
        let shown = img.fit(vec2(80.0, 80.0)).show(ui).rect.size();
        (size, shown)
    });
    assert_eq!(size, Some(vec2(8.0, 4.0)));
    assert_eq!(shown, vec2(80.0, 40.0), "ratio 2:1 kept inside 80x80");
    let (_, zoomed) = frame(|ui| img.zoom(2.0).size(vec2(40.0, 40.0)).radius(6.0).show(ui).rect.size());
    assert_eq!(zoomed, vec2(40.0, 40.0), "zoom crops inside the frame, it never grows it");
}

#[test]
fn a_missing_file_draws_nothing_and_does_not_crash() {
    let img = Img::file("/definitely/not/here.png");
    let (_, id) = frame(|ui| {
        img.zoom(1.0).size(vec2(20.0, 20.0)).show(ui);
        img.texture_id(ui.ctx())
    });
    assert!(id.is_none());
}

#[test]
fn mem_keeps_a_value_between_frames() {
    let ctx = egui::Context::default();
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0))), ..Default::default() };
    let mut seen = Vec::new();
    for _ in 0..3 {
        let out = ctx.run_ui(input(), |ui| {
            let mut tab = Mem::<usize>::new(ui, "tab");
            seen.push(tab.get());
            tab.update(|t| *t += 1);
            let zoom = Mem::with(ui, "zoom", 1.5_f32);
            assert_eq!(zoom.get(), 1.5);
        });
        std::mem::forget(out);
    }
    assert_eq!(seen, vec![0, 1, 2]);
}

#[test]
fn input_reads_typed_text_modifiers_and_scroll() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0))),
        events: vec![egui::Event::Text("ab".into()), egui::Event::Text("c".into()), egui::Event::Paste("zz".into())],
        ..Default::default()
    };
    let mut got = None;
    let out = ctx.run_ui(input, |ui| got = Some(Input(ui)));
    std::mem::forget(out);
    let i = got.unwrap();
    assert_eq!(i.typed(), "abc");
    assert_eq!(i.pasted(), Some("zz"));
    assert!(!i.shift() && !i.alt() && !i.cmd());
    assert!(i.dropped().is_empty());
}

#[test]
fn popup_shows_only_while_open_and_tip_keeps_the_response() {
    let (_, shown) = frame(|ui| {
        let mut closed = false;
        let a = Popup("p").at(pos2(10.0, 10.0)).width(120.0).show(ui, &mut closed, |_| 1);
        let mut open = true;
        let b = Popup("q").at(pos2(10.0, 10.0)).show(ui, &mut open, |ui| {
            ui.text("row").size(12.0);
            2
        });
        let r = ui.allocate_response(vec2(10.0, 10.0), Sense::hover());
        let same = Tip("hello").on(r.clone()).rect == r.rect;
        (a, b, same)
    });
    assert_eq!(shown, (None, Some(2), true));
}

#[test]
fn a_shrinking_scroll_leaves_room_for_what_comes_after_it() {
    // a tab strip: a horizontal scroll with something below it
    let (_, below) = frame(|ui| {
        Scroll::horizontal().id("tabs").hide_bar().shrink().show(ui, |ui| ui.label("tab"));
        ui.cursor().min.y
    });
    assert!(below < 60.0, "what follows the strip starts at y = {below}");
    // the same strip without `.shrink()` pushes it down to the bottom of the free space
    let (_, pushed) = frame(|ui| {
        Scroll::horizontal().id("tabs").hide_bar().show(ui, |ui| ui.label("tab"));
        ui.cursor().min.y
    });
    assert!(pushed > 200.0, "without shrink: y = {pushed}");
    // without `.shrink()` it takes all the free height (what a scroll area for a whole panel should do)
    let (_, (full, _)) = frame(|ui| {
        let full = Scroll::vertical().id("page").show(ui, |ui| ui.label("x")).viewport;
        (full, 0.0)
    });
    assert!(full.height() > 200.0, "{}", full.height());
}
