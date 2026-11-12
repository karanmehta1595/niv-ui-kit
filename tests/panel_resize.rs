//! Panel resize: khali (chhote content wala) panel bhi pointer ke saath bada-chhota ho.

use niv_ui_kit::egui::{self, Event, PointerButton, Pos2, RawInput, Rect, Vec2};
use niv_ui_kit::prelude::*;

fn frame(ctx: &egui::Context, t: &mut f64, events: Vec<Event>) -> f32 {
    *t += 0.05;
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
        time: Some(*t),
        events,
        ..Default::default()
    };
    let mut x = 0.0;
    let out = ctx.run_ui(input, |ui| {
        Panel("Left").left().width(220.0).resizable(true).range(180.0, 280.0).show_in(ui, |ui| {
            ui.label("hi");
        });
        let c = Panel("Main").center().show_in(ui, |_| {});
        x = c.response.rect.min.x;
    });
    std::mem::forget(out);
    x
}

#[test]
fn resize_follows_pointer() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut t = 0.0;
    let mut w = 0.0;
    for _ in 0..3 { w = frame(&ctx, &mut t, vec![]); }
    println!("start {w}");
    assert_eq!(w, 220.0);
    let edge = Pos2::new(w, 300.0);
    for _ in 0..2 { frame(&ctx, &mut t, vec![Event::PointerMoved(edge)]); }
    for _ in 0..2 { frame(&ctx, &mut t, vec![Event::PointerButton { pos: edge, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }]); }
    let mut got = vec![];
    for x in [200.0, 150.0, 260.0, 400.0] {
        let mut r = 0.0;
        for _ in 0..3 { r = frame(&ctx, &mut t, vec![Event::PointerMoved(Pos2::new(x, 300.0))]); }
        println!("pointer x={x} -> panel edge {r}");
        got.push(r);
    }
    assert_eq!(got, vec![200.0, 180.0, 260.0, 280.0]);
}

#[test]
fn fill_light_dark_rules() {
    use niv_ui_kit::prelude::{dark, default, default_dark, default_light, hex, light, rgb, rgba, Fill};
    let (dark_bg, light_bg) = (rgb(28, 28, 30), rgb(245, 245, 247));
    let at = |fill: Fill| (fill.resolve(false), fill.resolve(true)); // (light, dark)

    assert_eq!(hex("f5f5f7"), light_bg);
    assert_eq!(hex("#F5F5F7"), light_bg);
    assert_eq!(at(default), (light_bg, dark_bg));
    assert_eq!(at(default_dark), (dark_bg, dark_bg));
    assert_eq!(at(default_light), (light_bg, light_bg));
    assert_eq!(at(rgb(1, 2, 3).into()), (rgb(1, 2, 3), rgb(1, 2, 3)));
    assert_eq!(at(rgba(1, 2, 3, 4).into()), (rgba(1, 2, 3, 4), rgba(1, 2, 3, 4)));
    // sirf light badla, dark default
    assert_eq!(at(light(hex("#FFFFFF")).into()), (rgb(255, 255, 255), dark_bg));
    // sirf dark badla, light default
    assert_eq!(at(dark(rgb(0, 0, 0)).into()), (light_bg, rgb(0, 0, 0)));
    // dono, kisi bhi order mein
    assert_eq!(at(light(rgb(1, 1, 1)).dark(rgb(2, 2, 2)).into()), (rgb(1, 1, 1), rgb(2, 2, 2)));
    assert_eq!(at(dark(rgb(2, 2, 2)).light(rgb(1, 1, 1)).into()), (rgb(1, 1, 1), rgb(2, 2, 2)));
    // light mein bhi dark default
    assert_eq!(at(light(default_dark).into()), (dark_bg, dark_bg));
    assert_eq!(at(light(default_dark).dark(default_light).into()), (dark_bg, light_bg));
    // rang aur default mix
    assert_eq!(at(light(hex("#FFFFFF")).dark(default).into()), (rgb(255, 255, 255), dark_bg));
    assert_eq!(at(light(rgb(9, 9, 9)).dark(default_light).into()), (rgb(9, 9, 9), light_bg));
}

#[test]
fn fill_opacity_builds() {
    use niv_ui_kit::prelude::{dark, default, light};
    let _a = Panel("a").fill(default).fill_opacity(60);
    let _b = Panel("b").fill(default).fill_opacity(light(40).dark(70));
    let _c = Panel("c").fill(default).fill_opacity(dark(70)).frame(None);
}

#[test]
fn size_matches_width_and_height() {
    // .size(..) har side par chalta hai, aur .width/.height ke saath bhi bina error ke.
    let _a = Panel("s1").left().size(220.0).range(140.0, 280.0);
    let _b = Panel("s2").bottom().size(80.0).range(60.0, 200.0);
    let _c = Panel("s3").left().width(200.0);
    let _d = Panel("s4").top().height(60.0);
}

#[test]
fn gradient_and_border_paint_without_panic() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut t = 0.0;
    for _ in 0..4 {
        t += 0.3;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            time: Some(t),
            ..Default::default()
        };
        let mut shapes = 0;
        let out = ctx.run_ui(input, |ui| {
            Panel("g").right().width(300.0)
                .multi_gradient([(hex("#ff0000"), 0.0), (hex("#00ff00"), 20.0), (hex("#0000ff"), 100.0)])
                .points(0.0, 100.0, 0.0, 100.0).animate(2.0).ping_pong()
                .border_gradient(hex("#7f5af0"), hex("#2cb67d")).border_width(3.0).border_glow(10.0).border_animate(2.0)
                .show_in(ui, |ui| { ui.label("x"); });
            for at in [Glow::Inside, Glow::Middle, Glow::Outside] {
                Button("Glow").radius(10.0).border_gradient(hex("#7f5af0"), hex("#2cb67d")).border_width(2.0)
                    .border_glow(12.0).border_glow_at(at).border_glow_intensity(70.0)
                    .glow(hex("#7f5af0"), 10.0).glow_intensity(40.0).show(ui);
            }
            Panel("pg").top().height(60.0).border_gradient(hex("#f00"), hex("#00f")).border_glow(10.0)
                .border_glow_at(Glow::Middle).border_glow_intensity(500.0).show_in(ui, |_| {});
            // bahut chhota panel: border ki motai usse badi
            Panel("tiny").bottom().height(6.0).border_gradient(hex("#fff"), hex("#000")).border_width(20.0)
                .show_in(ui, |_| {});
            shapes += 1;
        });
        assert!(shapes > 0);
        assert!(!out.shapes.is_empty());
        std::mem::forget(out);
    }
}

#[test]
fn buttons_paint_and_click() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut t = 0.0;
    let mut clicked = false;
    let mut sizes = vec![];
    for step in 0..5 {
        t += 0.1;
        // 3rd aur 4th frame mein click (dabao, chhodo)
        let mut events = vec![Event::PointerMoved(Pos2::new(30.0, 20.0))];
        if step == 2 || step == 3 {
            events.push(Event::PointerButton { pos: Pos2::new(30.0, 20.0), button: PointerButton::Primary, pressed: step == 2, modifiers: Default::default() });
        }
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            time: Some(t),
            events,
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            ui.add_space(5.0);
            let plain = Button("Save").show(ui);
            sizes.push(plain.rect.size());
            clicked |= plain.clicked();
            Button(Label("Fancy").italic().underline().strike().mono().opacity(0.8).size(18.0)).radius(12.0).corners(2.0, 20.0, 2.0, 20.0)
                .multi_gradient([(hex("#f00"), 0.0), (hex("#0f0"), 30.0), (hex("#00f"), 100.0)]).animate(2.0)
                .border_widths(1.0, 2.0, 3.0, 4.0).border_colors(hex("#fff"), hex("#f00"), hex("#0f0"), hex("#00f"))
                .shadow(12.0).glow(hex("#7f5af0"), 14.0).show(ui);
            Button(None).size(vec2(200.0, 40.0))
                .fill(light(hex("#1c1c1e")).dark(hex("#F5F5F7")))
                .image(Image("icons/a.png").size(vec2(16.0, 16.0)).left(12.0))
                .text(Label("Mid").bold().color(light(hex("#000")).dark(Color::WHITE)).center())
                .image(Image("icons/b.png").size(10.0).at(100.0, 0.0))
                .image("icons/c.png")
                .image(Image(light("icons/l.svg").dark("icons/d.svg")).size(vec2(30.0, 10.0)).fit().before())
                .glass().blur(40).show(ui);
            Button("Ring").border_multi_gradient([(hex("#f00"), 0.0), (hex("#00f"), 100.0)]).border_animate(2.0).border_glow(6.0).show(ui);
            Button("tiny").width(4.0).height(4.0).radius(50.0).border_width(30.0).border_gradient(hex("#fff"), hex("#000")).show(ui);
        });
        std::mem::forget(out);
    }
    assert!(clicked, "button click nahi pakda");
    assert!(sizes[0].x > 20.0 && sizes[0].y >= 24.0, "size {:?}", sizes[0]);
}

fn run_frame(ctx: &egui::Context, t: f64, events: Vec<Event>, add: impl FnMut(&mut egui::Ui)) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
        time: Some(t),
        events,
        ..Default::default()
    };
    let out = ctx.run_ui(input, add);
    std::mem::forget(out);
}

#[test]
fn locate_places_button_inside_parent() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = vec![];
    for step in 0..3 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let parent = ui.max_rect();
            let a = Locate(Button("Save")).at(100.0, 100.0).show(ui); // neeche-right kona
            let b = Locate(Button("Save")).left(10.0).top(20.0).show(ui);
            let c = Locate(Button("Save")).center().show(ui);
            got = vec![(parent, a.rect), (parent, b.rect), (parent, c.rect)];
        });
    }
    let (parent, a) = got[0];
    assert!((a.max.x - parent.max.x).abs() < 0.5 && (a.max.y - parent.max.y).abs() < 0.5, "{a:?} {parent:?}");
    let (parent, b) = got[1];
    assert!((b.min.x - (parent.min.x + 10.0)).abs() < 0.5 && (b.min.y - (parent.min.y + 20.0)).abs() < 0.5);
    let (parent, c) = got[2];
    assert!((c.center().x - parent.center().x).abs() < 0.5 && (c.center().y - parent.center().y).abs() < 0.5);
}

#[test]
fn bundle_hides_overflowing_buttons() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut visible = vec![];
    for step in 0..5 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            // Sirf 160 px chaudi jagah: teen 80 px ke buttons nahi samayenge.
            let area = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(160.0, 40.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                Bundle("tools").text("More").set(ui);
                visible.clear();
                ui.horizontal(|ui| {
                    for name in ["Cut", "Copy", "Paste", "Delete"] {
                        let r = Button(name).width(70.0).bundle("tools").show(ui);
                        visible.push(r.rect.width() > 1.0);
                    }
                });
            });
        });
    }
    println!("visible: {visible:?}");
    assert!(visible[0], "pehla button dikhna chahiye");
    assert!(!visible[3], "aakhri button chhupna chahiye");
    // Jo chhupa, uske baad sab chhupe.
    let first_hidden = visible.iter().position(|v| !v).expect("koi hidden nahi");
    assert!(visible[first_hidden..].iter().all(|v| !v));
}

#[test]
fn bundle_shows_everything_when_there_is_room() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut visible = vec![];
    for step in 0..4 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            visible.clear();
            ui.horizontal(|ui| {
                for name in ["Cut", "Copy", "Paste"] {
                    visible.push(Button(name).bundle("roomy").show(ui).rect.width() > 1.0);
                }
            });
        });
    }
    assert_eq!(visible, vec![true, true, true]);
}

#[test]
fn hover_and_pressed_keep_layout_and_work() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let make = || {
        Button("Save")
            .size(vec2(100.0, 40.0))
            .fill(rgb(10, 10, 10))
            .hover(Hover().fill(rgb(200, 0, 0)).opacity(80).text("Save now").scale(1.1).lift(2.0).glow(rgb(0, 255, 0), 10.0).tooltip("tip"))
            .pressed(Pressed().scale(0.95))
            .transition(0.1)
    };
    let (mut rect, mut hovered, mut still) = (Rect::NOTHING, false, Rect::NOTHING);
    for step in 0..6 {
        let events = if step >= 1 { vec![Event::PointerMoved(Pos2::new(20.0, 20.0))] } else { vec![] };
        run_frame(&ctx, 0.05 * f64::from(step), events, |ui| {
            let r = make().show(ui);
            if step == 0 {
                still = r.rect;
            }
            rect = r.rect;
            hovered = r.hovered();
        });
    }
    assert!(hovered, "pointer upar hai to hovered");
    assert_eq!(rect, still, "hover se layout nahi hilna chahiye");
}

#[test]
fn button_click_zones_in_panels() {
    // (panel, x% , button ka kaun sa hissa dabaya: 0.0 = left kinara .. 1.0 = right kinara)
    for central in [false, true] {
        for frac in [0.1_f32, 0.5, 0.9, 0.98] {
            let ctx = egui::Context::default();
            niv_ui_kit::init(&ctx);
            let mut t = 0.0;
            let mut rect = Rect::NOTHING;
            let mut clicks = 0;
            let mut run = |events: Vec<Event>, rect: &mut Rect, clicks: &mut i32| {
                t += 0.05;
                let input = RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    time: Some(t),
                    events,
                    ..Default::default()
                };
                let out = ctx.run_ui(input, |ui| {
                    Panel("Left").left().resizable(true).range(140.0, 280.0).show_in(ui, |ui| {
                        if !central {
                            let r = Locate(Button(None).size(vec2(55.0, 30.0)).hover(Hover().scale(1.03))).at(99.0, 0.5).show(ui);
                            *rect = r.rect;
                            *clicks += i32::from(r.clicked());
                        }
                    });
                    Panel("ctx").center().frame(None).show_in(ui, |ui| {
                        if central {
                            let r = Locate(Button(None).size(vec2(55.0, 30.0)).hover(Hover().scale(1.03))).at(1.0, 1.0).show(ui);
                            *rect = r.rect;
                            *clicks += i32::from(r.clicked());
                        }
                    });
                });
                std::mem::forget(out);
            };
            for _ in 0..3 {
                run(vec![], &mut rect, &mut clicks);
            }
            let p = Pos2::new(rect.min.x + rect.width() * frac, rect.center().y);
            for _ in 0..3 {
                run(vec![Event::PointerMoved(p)], &mut rect, &mut clicks);
            }
            run(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }], &mut rect, &mut clicks);
            run(vec![], &mut rect, &mut clicks);
            run(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Default::default() }], &mut rect, &mut clicks);
            run(vec![], &mut rect, &mut clicks);
            println!("central={central} frac={frac} rect={rect:?} clicks={clicks}");
        }
    }
}
#[test]
fn button_inside_resizable_panel_gets_clicks() {
    for (label, resizable, x_pct) in [("resizable 99%", true, 99.0_f32), ("fixed 99%", false, 99.0), ("resizable 50%", true, 50.0)] {
        let ctx = egui::Context::default();
        niv_ui_kit::init(&ctx);
        let mut t = 0.0;
        let mut rect = Rect::NOTHING;
        let mut clicks = 0;
        let mut run = |events: Vec<Event>, rect: &mut Rect, clicks: &mut i32| {
            t += 0.05;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                time: Some(t),
                events,
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                Panel("Left").left().resizable(resizable).range(140.0, 280.0).show_in(ui, |ui| {
                    let r = Locate(Button(None).size(vec2(55.0, 30.0))).at(x_pct, 50.0).show(ui);
                    *rect = r.rect;
                    if r.clicked() {
                        *clicks += 1;
                    }
                });
                Panel("ctx").center().show_in(ui, |_| {});
            });
            std::mem::forget(out);
        };
        for _ in 0..3 {
            run(vec![], &mut rect, &mut clicks);
        }
        let p = rect.center();
        println!("{label}: button rect {rect:?}");
        run(vec![Event::PointerMoved(p)], &mut rect, &mut clicks);
        run(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }], &mut rect, &mut clicks);
        run(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Default::default() }], &mut rect, &mut clicks);
        run(vec![], &mut rect, &mut clicks);
        println!("{label}: clicks = {clicks}");
        assert_eq!(clicks, 1, "{label}: click nahi pakda");
    }
}

#[test]
fn free_and_claim_stack_from_every_edge() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut r = vec![];
    for step in 0..3 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let b = |w: f32, h: f32| Button(None).size(vec2(w, h));
            let top = Locate(b(100.0, 40.0)).left(0.0).top(10.0).claim().show(ui).rect;
            let under_top = Locate(b(30.0, 20.0)).left(5.0).top(0.0).free().show(ui).rect;
            let bottom = Locate(b(100.0, 30.0)).left(0.0).bottom(10.0).claim().show(ui).rect;
            let above_bottom = Locate(b(30.0, 20.0)).left(5.0).bottom(0.0).free().show(ui).rect;
            let left = Locate(b(50.0, 20.0)).left(8.0).top(200.0).claim_side().show(ui).rect;
            let beside_left = Locate(b(30.0, 20.0)).left(0.0).top(200.0).free().show(ui).rect;
            let right = Locate(b(60.0, 20.0)).right(8.0).top(250.0).claim().show(ui).rect;
            let beside_right = Locate(b(30.0, 20.0)).right(0.0).top(250.0).free().show(ui).rect;
            // free + claim ke bina: parent ke kone se (purana tareeka)
            let plain = Locate(b(30.0, 20.0)).left(0.0).top(0.0).show(ui).rect;
            r = vec![top, under_top, bottom, above_bottom, left, beside_left, right, beside_right, plain];
        });
    }
    let [top, under_top, bottom, above_bottom, left, beside_left, right, beside_right, plain] = r[..] else { panic!() };
    assert!(under_top.min.y >= top.max.y, "top claim ke neeche: {under_top:?} vs {top:?}");
    assert!(above_bottom.max.y <= bottom.min.y, "bottom claim ke upar: {above_bottom:?} vs {bottom:?}");
    // sirf left/right wale claim ka kolam (corner mein .top hai, to patti): claim_side left kolam leta hai
    assert!(beside_left.min.x >= left.max.x, "left kolam ke bagal: {beside_left:?} vs {left:?}");
    assert!(beside_right.max.x <= right.min.x || beside_right.min.y >= right.max.y, "{beside_right:?} vs {right:?}");
    assert!(plain.min.y < top.max.y, "free nahi to parent ke kone se: {plain:?}");
}

#[test]
fn free_top_offset_counts_from_free_area() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut out = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING, f32::NAN);
    for step in 0..3 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let gap = ui.spacing().item_spacing.y;
            let a = Locate(Button(None).size(vec2(100.0, 40.0))).left(0.0).top(10.0).claim().show(ui).rect;
            let b = Locate(Button(None).size(vec2(30.0, 20.0))).left(5.0).top(4.0).free().show(ui).rect;
            let c = Locate(Button(None).size(vec2(30.0, 20.0))).left(50.0).top(4.0).free().show(ui).rect;
            out = (a, b, c, gap);
        });
    }
    let (a, b, c, gap) = out;
    println!("a={a:?} b={b:?} c={c:?} gap={gap}");
    assert_eq!(b.min.y, a.max.y + gap + 4.0);
    assert_eq!(c.min.y, a.max.y + gap + 4.0, "free area b se nahi badalni (b ne claim nahi kiya)");
}

#[test]
fn row_and_column_lay_out_buttons() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (vec![], vec![], vec![], vec![]);
    for step in 0..3 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let b = |w: f32, h: f32| Button(None).size(vec2(w, h));
            let mut row = vec![];
            Row().gap(10.0).show(ui, |ui| {
                row = vec![b(40.0, 40.0).show(ui).rect, b(40.0, 20.0).show(ui).rect, b(40.0, 40.0).show(ui).rect];
            });
            let mut rev = vec![];
            Row().reverse().gap(10.0).show(ui, |ui| {
                rev = vec![b(40.0, 40.0).show(ui).rect, b(40.0, 40.0).show(ui).rect];
            });
            let mut col = vec![];
            Column().gap(7.0).show(ui, |ui| {
                col = vec![b(40.0, 30.0).show(ui).rect, b(40.0, 30.0).show(ui).rect];
            });
            let mut cen = vec![];
            Column().align(Align::Center).show(ui, |ui| {
                cen = vec![b(40.0, 30.0).show(ui).rect];
            });
            got = (row, rev, col, cen);
        });
    }
    let (row, rev, col, cen) = got;
    // Row: left se right, beech mein 10 px, chhota button upar-neeche beech mein
    assert_eq!(row[1].min.x - row[0].max.x, 10.0);
    assert_eq!(row[2].min.x - row[1].max.x, 10.0);
    assert_eq!(row[1].center().y, row[0].center().y, "Align::Center default");
    // reverse: pehla button right mein
    assert!(rev[0].min.x > rev[1].min.x, "{rev:?}");
    // Column: ek ke neeche ek, 7 px beech mein
    assert_eq!(col[1].min.y - col[0].max.y, 7.0);
    assert_eq!(col[0].min.x, col[1].min.x);
    // Column Center: parent ke beech mein
    assert!((cen[0].center().x - 400.0).abs() < 1.0, "{cen:?}");
}

#[test]
fn row_and_column_center_and_right() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (vec![], vec![], vec![], vec![]);
    for step in 0..6 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let b = || Button(None).size(vec2(40.0, 30.0));
            let (mut centered, mut right, mut col_center, mut col_bottom) = (vec![], vec![], vec![], vec![]);
            Row().center().gap(10.0).show(ui, |ui| {
                centered = vec![b().show(ui).rect, b().show(ui).rect];
            });
            Row().right().gap(10.0).show(ui, |ui| {
                right = vec![b().show(ui).rect, b().show(ui).rect];
            });
            Column().center().show(ui, |ui| {
                col_center = vec![b().show(ui).rect];
            });
            Column().bottom().show(ui, |ui| {
                col_bottom = vec![b().show(ui).rect];
            });
            got = (centered, right, col_center, col_bottom);
        });
    }
    let (centered, right, col_center, col_bottom) = got;
    let mid = (centered[0].min.x + centered[1].max.x) / 2.0;
    assert!((mid - 400.0).abs() < 1.5, "row beech mein: {centered:?}");
    assert!((right[1].max.x - 800.0).abs() < 1.5, "row right mein: {right:?}");
    assert!(centered[1].min.x - centered[0].max.x == 10.0, "gap bacha rehna chahiye");
    assert!(col_center[0].min.y > 100.0, "column neeche khiska hua: {col_center:?}");
    assert!(col_bottom[0].max.y > col_center[0].max.y, "bottom, center se neeche: {col_bottom:?} {col_center:?}");
}

#[test]
fn row_and_column_edge_gaps() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (vec![], vec![], vec![], vec![], Rect::NOTHING, Rect::NOTHING);
    for step in 0..5 {
        run_frame(&ctx, 0.1 * f64::from(step), vec![], |ui| {
            let b = || Button(None).size(vec2(40.0, 30.0));
            let (mut plain, mut start, mut around, mut rev) = (vec![], vec![], vec![], vec![]);
            let top0 = ui.cursor().min;
            Column().gap(10.0).show(ui, |ui| {
                plain = vec![b().show(ui).rect];
            });
            let after_plain = ui.cursor().min;
            Row().gap(10.0).gap_start(18.0).show(ui, |ui| {
                start = vec![b().show(ui).rect, b().show(ui).rect];
            });
            Row().gap(10.0).gap_around().show(ui, |ui| {
                around = vec![b().show(ui).rect];
            });
            Row().gap(10.0).gap_start(18.0).reverse().show(ui, |ui| {
                rev = vec![b().show(ui).rect];
            });
            got = (plain, start, around, rev, Rect::from_min_max(top0, top0), Rect::from_min_max(after_plain, after_plain));
        });
    }
    let (plain, start, around, rev, _, _) = got;
    assert_eq!(plain[0].min.x, 0.0, "gap_start nahi to kinare se chipka: {plain:?}");
    assert_eq!(start[0].min.x, 18.0, "gap_start 18: {start:?}");
    assert_eq!(start[1].min.x - start[0].max.x, 10.0, "beech ki gap wahi");
    assert_eq!(around[0].min.x, 10.0, "gap_around: shuru mein gap jitna: {around:?}");
    assert_eq!(800.0 - rev[0].max.x, 18.0, "reverse mein start right se: {rev:?}");
}

fn key_event(key: egui::Key, shift: bool) -> Event {
    let modifiers = egui::Modifiers { shift, ..egui::Modifiers::NONE };
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

#[test]
fn keys_click_buttons_and_read_as_bool() {
    use niv_ui_kit::egui::Key as K;
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    // (events, plain-5 button, shift-5 button, global plain 5, global shift 5, Backspace)
    let run = |events: Vec<Event>| {
        let mut r = (false, false, false, false, false);
        run_frame(&ctx, 0.1, events, |ui| {
            niv_ui_kit::keys::begin_frame(ui.ctx());
            r.0 = Button("5").key(K::Num5).show(ui).clicked();
            r.1 = Button("%").key(K::Num5.with(Shift)).show(ui).clicked();
            r.2 = K::Num5.pressed();
            r.3 = K::Num5.with(Shift).pressed();
            r.4 = K::Backspace.pressed() || K::Backspace.pressed_in(ui);
        });
        r
    };
    run(vec![]);
    assert_eq!(run(vec![key_event(K::Num5, false)]), (true, false, true, false, false), "plain 5");
    assert_eq!(run(vec![key_event(K::Num5, true)]), (false, true, false, true, false), "shift+5");
    assert_eq!(run(vec![key_event(K::Backspace, false)]), (false, false, false, false, true), "backspace");
    assert_eq!(run(vec![]), (false, false, false, false, false), "kuch nahi dabi");
}

#[test]
fn full_width_and_height_take_the_room_left() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (0.0, 0.0, 0.0);
    for _ in 0..2 {
        run_frame(&ctx, 0.1, vec![], |ui| {
            ui.set_max_size(Vec2::new(300.0, 200.0));
            got.0 = Button("a").full_width().show(ui).rect.width();
            got.1 = Button("b").width(80.0).full_height().show(ui).rect.height();
            got.2 = Button("c").show(ui).rect.width();
        });
    }
    assert_eq!(got.0, 300.0, "full_width: poori chaudai");
    assert!(got.1 > 24.0 && got.1 < 200.0, "full_height: bachi unchai, mili {}", got.1);
    assert!(got.2 < 100.0, "bina full ke content jitna");
}

#[test]
fn full_sizes_and_ranges_follow_the_room_left() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut sizes = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..3 {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            time: Some(1.0),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            // 600 unchai mein: Full - 100 = 500
            sizes.0 = Panel("a").bottom().height(Full - 100.0).show_in(ui, |_| {}).response.rect.height();
            // ab bachi 100: range(40, Full) resizable, default size
            sizes.1 = Panel("b").top().height(60.0).resizable(true).range_full(40.0).show_in(ui, |_| {}).response.rect.height();
            sizes.2 = Panel("c").left().full_width().show_in(ui, |_| {}).response.rect.width();
            sizes.3 = Panel("d").center().show_in(ui, |_| {}).response.rect.width();
        });
        std::mem::forget(out);
    }
    assert_eq!(sizes.0, 500.0, "height(Full - 100)");
    assert_eq!(sizes.1, 60.0, "range_full(40) resizable, size 60");
    assert_eq!(sizes.2, 800.0, "full_width: poori chaudai");
    assert!(sizes.3 <= 16.0, "centre ko sirf padding bachi, mili {}", sizes.3);
    // baaki forms bina error ke
    let _ = Panel("e").left().range(120.0, 220.0).min(100.0).max(Full - 50.0).size(Full);
}

#[test]
fn button_full_width_gap_leaves_room_on_both_sides() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut r = (Rect::ZERO, Rect::ZERO);
    for _ in 0..2 {
        run_frame(&ctx, 0.1, vec![], |ui| {
            ui.set_max_size(Vec2::new(300.0, 200.0));
            let start = ui.cursor().min;
            let a = Button("a").full_width_gap(12.0, 20.0).show(ui).rect;
            r.0 = a;
            r.1 = Rect::from_min_size(start, Vec2::ZERO);
            let _ = Button("b").full_height_gap(8.0, 8.0).show(ui);
        });
    }
    assert_eq!(r.0.min.x - r.1.min.x, 12.0, "left se 12");
    assert_eq!(r.0.width(), 300.0 - 12.0 - 20.0, "chaudai = poori - 12 - 20");
}

#[test]
fn panel_size_is_remembered_after_restart() {
    // Pehli baar: start size 240. App band karke dobara: code mein 100 likha ho tab bhi 240 hi.
    let width = |ctx: &egui::Context, start: f32| {
        let mut w = 0.0;
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                time: Some(1.0),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                w = Panel("side").left().width(start).resizable(true).show_in(ui, |_| {}).response.rect.width();
                Panel("main").center().show_in(ui, |_| {});
            });
            std::mem::forget(out);
        }
        w
    };
    let first = egui::Context::default();
    niv_ui_kit::init(&first);
    assert_eq!(width(&first, 240.0), 240.0, "pehli baar start size");
    // egui ki yaaddasht file mein likhi jaati hai (eframe yahi karta hai) aur agli baar padhi jaati hai.
    let saved = first.memory(|m| ron::ser::to_string(m).unwrap());
    let second = egui::Context::default();
    niv_ui_kit::init(&second);
    second.memory_mut(|m| *m = ron::from_str(&saved).unwrap());
    assert_eq!(width(&second, 100.0), 240.0, "dobara kholne par wahi size");
}

#[test]
fn group_splits_clicks_and_hover_per_button() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let make = |sep: bool| {
        let group = Group("g")
            .button(Button(None).size((40.0, 30.0)))
            .button(Button(None).size((60.0, 20.0)))
            .button(glass_button().size((40.0, 30.0)));
        if sep { group.separators() } else { group }
    };
    let click = |pos: Pos2, pressed: bool| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() };

    // size: 40 + 60 + 40 chaudai, sab ki unchai barabar (30)
    let mut rect = Rect::NOTHING;
    run_frame(&ctx, 0.0, vec![], |ui| rect = make(true).show(ui).response.rect);
    assert_eq!(rect.size(), Vec2::new(140.0, 30.0), "buttons ka kul size");

    let at = |x: f32| Pos2::new(rect.min.x + x, rect.min.y + 15.0);
    // dusre button (x = 40..100) par hover: sirf wahi
    let mut hov = (false, false, false);
    for i in 0..3 {
        run_frame(&ctx, 0.05 * f64::from(i), vec![Event::PointerMoved(at(70.0))], |ui| {
            let r = make(true).show(ui);
            hov = (r.hovered(0), r.hovered(1), r.hovered(2));
        });
    }
    assert_eq!(hov, (false, true, false), "cursor sirf beech wale par");

    // click pehle button par: sirf use mile
    let mut clicked = (false, false, false);
    for (i, events) in [vec![Event::PointerMoved(at(10.0))], vec![click(at(10.0), true)], vec![click(at(10.0), false)]].into_iter().enumerate() {
        run_frame(&ctx, 0.3 + 0.05 * i as f64, events, |ui| {
            let r = make(true).show(ui);
            if r.which() == Some(0) {
                clicked.0 = true;
            }
            clicked.1 |= r.clicked(1);
            clicked.2 |= r.clicked(2);
        });
    }
    assert_eq!(clicked, (true, false, false), "click sirf pehle button ko");

    // vertical + padding + gap bina panic ke
    run_frame(&ctx, 1.0, vec![], |ui| {
        let r = Group("v").vertical().padding(3.0, 3.0).gap(2.0).radius(10.0).glass().blur(40).shadow(8.0)
            .separator_color(rgb(1, 2, 3))
            .saturation(1.8).brightness(1.1).tint(hex("#7f5af0"), 0.2)
            .fill(rgba(255, 255, 255, 40)).fill_opacity(60).border(rgb(9, 9, 9), 1.0).border_glow(6.0)
            .glow(hex("#7f5af0"), 12.0).glow_intensity(60.0).opacity(90)
            .gradient(hex("#111111"), hex("#eeeeee")).animate(2.0)
            .hover(Hover().fill(rgb(5, 5, 5))).pressed(Pressed().scale(0.95)).transition(0.2)
            .button(Button("a")).button(Button("b")).show(ui);
        assert!(r.response.rect.height() > 40.0, "vertical group lamba");
    });
}

#[test]
fn group_takes_image_text_corners_and_sides() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    run_frame(&ctx, 0.0, vec![], |ui| {
        let r = Group("c")
            .corners(20.0, 4.0, 20.0, 4.0)
            .padding(2.0, 2.0)
            .image(Image("icons/a.png").size(14.0).left(8.0))
            .text(Label("Title").bold().center())
            .border_widths(1.0, 0.0, 2.0, 0.0)
            .border_colors(rgb(1, 1, 1), rgb(2, 2, 2), rgb(3, 3, 3), rgb(4, 4, 4))
            .button(Button("a").size((40.0, 24.0)))
            .button(Button("b").size((40.0, 24.0)))
            .show(ui);
        assert_eq!(r.response.rect.size(), Vec2::new(84.0, 28.0));
    });
}

#[test]
fn button_separator_button_makes_one_capsule() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let make = || {
        Button(None).glass().blur(30).radius(16.0).size((36.0, 30.0))
            .separator()
            .button(Button(None).fill(rgb(200, 0, 0)).size((40.0, 30.0)))
            .separator()
            .button(Button(None).size((24.0, 30.0)))
    };
    let mut rect = Rect::NOTHING;
    let mut hov = (false, false, false);
    for i in 0..3 {
        run_frame(&ctx, 0.05 * f64::from(i), vec![Event::PointerMoved(Pos2::new(50.0, 15.0))], |ui| {
            let r = make().show(ui);
            rect = r.response.rect;
            hov = (r.hovered(0), r.hovered(1), r.hovered(2));
        });
    }
    assert_eq!(rect.size(), Vec2::new(100.0, 30.0), "36 + 40 + 24");
    assert_eq!(hov, (false, true, false), "cursor (50,15) dusre hisse par");
    // bina lakeer ka seedha jodna bhi chalta hai
    run_frame(&ctx, 1.0, vec![], |ui| {
        let r = Button("a").button(Button("b")).show(ui);
        assert!(r.part(1).is_some());
    });
}

#[test]
fn locate_places_a_group_with_its_own_size() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
    for _ in 0..2 {
        run_frame(&ctx, 0.1, vec![], |ui| {
            let r = Locate(Group("g").size(vec2(80.0, 40.0)).radius(12.0).fill(inverse).opacity(80))
                .button(Button(None)) // size group ka (button par size likhte to wahi chalta)
                .separator()
                .button(Button(None))
                .right(10.0)
                .top(5.0)
                .show(ui);
            got = (r.response.rect, r.part(0).unwrap().rect, r.part(1).unwrap().rect, Rect::NOTHING);
        });
    }
    assert_eq!(got.0.size(), Vec2::new(80.0, 40.0), "group ka tay size");
    assert_eq!(got.1.width(), 40.0, "aadhi chaudai");
    assert_eq!(got.2.width(), 40.0, "aadhi chaudai");
    assert_eq!(got.0.max.x, 800.0 - 10.0, "right se 10");
    assert_eq!(got.0.min.y, 5.0, "top se 5");
}

#[test]
fn group_separator_radius_builds() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    run_frame(&ctx, 0.0, vec![], |ui| {
        let r = Group("s").size(vec2(80.0, 40.0)).radius(20.0).separator_radius(20.0)
            .button(Button("a")).separator().button(Button("b")).show(ui);
        assert_eq!(r.response.rect.size(), Vec2::new(80.0, 40.0));
    });
}

#[test]
fn group_separator_order_corners_and_hover_radius_build() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    for i in 0..3 {
        run_frame(&ctx, 0.05 * f64::from(i), vec![Event::PointerMoved(Pos2::new(60.0, 15.0))], |ui| {
            // .separators() pehle likho, buttons baad mein: phir bhi chalna chahiye
            let r = Group("c").separators().separator_width(2.0).separator_length(12.0)
                .radius(32.0)
                .button_corners("l", 32.0, 0.0, 0.0, 32.0)
                .button(Button(None).id("l").size((40.0, 30.0)).hover(Hover().radius(32.0)))
                .button_corners("r", 0.0, 32.0, 32.0, 0.0)
                .button(Button(None).id("r").size((40.0, 30.0)).hover(Hover().radius(32.0)))
                .show(ui);
            assert!(r.part(1).is_some());
        });
    }
}

#[test]
fn group_button_size_by_id_and_scale_states() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut sizes = (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO);
    for i in 0..4 {
        run_frame(&ctx, 0.05 * f64::from(i), vec![Event::PointerMoved(Pos2::new(10.0, 10.0))], |ui| {
            let r = Group("s")
                .button(Button("a").id("a").size((50.0, 30.0)).scale(State::pop(0.1).hovered()).scale(State::push(0.05).clicked()))
                .button(Button("b").size((20.0, 30.0)))
                .scale(State::scale(1.02))
                .show(ui);
            sizes = (r.response.rect.size(), r.part(0).map_or(Vec2::ZERO, |p| p.rect.size()), r.part(1).map_or(Vec2::ZERO, |p| p.rect.size()));
        });
    }
    assert_eq!(sizes.1, Vec2::new(50.0, 30.0));
    assert_eq!(sizes.2.x, 20.0);
    assert_eq!(sizes.0.x, 70.0);
}

#[test]
fn button_size_keeps_group_rect_and_sits_inside() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
    for _ in 0..2 {
        run_frame(&ctx, 0.1, vec![], |ui| {
            let r = Group("k").size(vec2(120.0, 50.0)).separators()
                .button(Button(None).size((30.0, 30.0))).button(Button(None).size((30.0, 30.0)))
                .show(ui);
            got = (r.response.rect, r.part(0).unwrap().rect, r.part(1).unwrap().rect);
        });
    }
    assert_eq!(got.0.size(), Vec2::new(120.0, 50.0), "group rect badla nahi");
    assert_eq!(got.1.size(), Vec2::new(30.0, 30.0));
    assert_eq!(got.2.size(), Vec2::new(30.0, 30.0));
    assert_eq!(got.1.center().y, got.0.center().y, "beech mein");
    assert!(got.1.min.x > got.0.min.x && got.2.max.x < got.0.max.x, "rect ke andar");
}

#[test]
fn group_gap_spreads_fixed_buttons_evenly_and_centered() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut got = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
    for _ in 0..2 {
        run_frame(&ctx, 0.1, vec![], |ui| {
            let r = Group("g").size(vec2(120.0, 40.0)).gap(20.0)
                .button(Button(None).size((30.0, 30.0))).button(Button(None).size((30.0, 30.0))).show(ui);
            got = (r.response.rect, r.part(0).unwrap().rect, r.part(1).unwrap().rect);
        });
    }
    assert_eq!(got.2.min.x - got.1.max.x, 20.0);
    assert_eq!(got.1.min.x - got.0.min.x, got.0.max.x - got.2.max.x, "beech mein");
}

#[test]
fn group_button_keeps_its_own_look() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    for i in 0..3 {
        run_frame(&ctx, 0.05 * f64::from(i), vec![Event::PointerMoved(Pos2::new(10.0, 10.0))], |ui| {
            let r = Group("l").size(vec2(100.0, 40.0))
                .button(Button(None).size(30.0).fill(rgb(255, 255, 255)).fill_opacity(20).blur(40)
                    .hover(Hover().fill(rgb(0, 255, 0)).fill_opacity(40).blur(40)).pressed(Pressed().scale(0.9)))
                .button(Button(None).size(30.0))
                .show(ui);
            assert!(r.part(1).is_some());
        });
    }
}

#[test]
fn dropdown_opens_draws_content_and_closes_outside() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut open = true;
    let mut ran = false;
    run_frame(&ctx, 0.0, vec![], |ui| {
        let _ = Dropdown("d").size(vec2(200.0, 100.0)).show(ui, &mut open, |_| ran = true);
    });
    assert!(ran && open);
    // bahar click: band
    run_frame(&ctx, 0.1, vec![
        Event::PointerMoved(Pos2::new(5.0, 590.0)),
        Event::PointerButton { pos: Pos2::new(5.0, 590.0), button: PointerButton::Primary, pressed: true, modifiers: Default::default() },
    ], |ui| {
        let _ = Dropdown("d").size(vec2(200.0, 100.0)).show(ui, &mut open, |_| {});
    });
    assert!(!open);
    // band ho to content nahi chalta
    ran = false;
    run_frame(&ctx, 0.2, vec![], |ui| {
        let _ = Dropdown("d").show(ui, &mut open, |_| ran = true);
    });
    assert!(!ran);
}

#[test]
fn bundle_hides_group_when_no_room_and_until_limits() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let run = |limit: f32| {
        let mut shown = (false, false);
        for i in 0..4 {
            run_frame(&ctx, 0.1 * f64::from(i), vec![], |ui| {
                Bundle("tb").until(limit).set(ui);
                ui.horizontal(|ui| {
                    let a = Button(None).bundle("tb").size((60.0, 30.0)).show(ui);
                    let g = Group("bg").size(vec2(100.0, 30.0)).bundle("tb")
                        .button(Button(None).size(30.0)).button(Button(None).size(30.0)).show(ui);
                    shown = (a.rect.width() > 0.0, g.part(0).is_some());
                });
            });
        }
        shown
    };
    assert_eq!(run(1000.0), (true, true), "jagah hai: dono dikhte hain");
    assert_eq!(run(120.0), (true, false), "limit 120: Group chhup gaya");
}

#[test]
fn scroll_space_adds_room_after_the_content() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))), ..Default::default() };
    let mut sizes = Vec::new();
    for px in [0.0, 36.0 * 8.0] {
        for _ in 0..2 {
            let out = ctx.run_ui(input.clone(), |ui| {
                let r = Scroll::vertical().id(format!("s{px}")).scroll_space(Bottom::topdown_space(px)).show(ui, |ui| {
                    ui.allocate_space(Vec2::new(100.0, 500.0));
                });
                sizes.push((px, r.content_size.y, r.space.rect.height()));
            });
            std::mem::forget(out);
        }
    }
    let (_, plain, _) = sizes[1];
    let (px, more, extra) = sizes[3];
    assert_eq!(extra, px);
    assert!((more - plain - 288.0).abs() < 1.0, "{plain} -> {more}");
}

#[test]
fn bundle_lead_joins_the_group_frame_in_the_list() {
    // Quiet button + Group with a lead: when both are hidden the list shows ONE frame with the lead on top.
    let run = |quiet: bool| {
        let ctx = egui::Context::default();
        niv_ui_kit::init(&ctx);
        let mut pos = Pos2::new(20.0, 20.0);
        let mut seen = (0, 0.0_f32, false);
        for i in 0..12 {
            let click = i == 4;
            let events = if click {
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Default::default() },
                    Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Default::default() },
                ]
            } else {
                vec![]
            };
            run_frame(&ctx, 0.1 * f64::from(i), events, |ui| {
                Bundle("ld").until(10.0).set(ui);
                ui.horizontal(|ui| {
                    Button(None).bundle("ld").size((20.0, 30.0)).show(ui);
                    let mut first = Button(None).bundle("ld").size((60.0, 30.0));
                    if quiet {
                        first = first.bundle_quiet();
                    }
                    let a = first.show(ui);
                    pos = a.rect.min + vec2(5.0, 5.0);
                    let g = Group("ldg").gap(0.0).bundle("ld")
                        .bundle_lead(Button(None).id("lead").size(30.0))
                        .button(Button(None).id("a").size(30.0))
                        .button(Button(None).id("b").size(30.0))
                        .show(ui);
                    if g.part(0).is_some() {
                        seen = (g.parts_len(), g.response.rect.height(), g.lead_clicked());
                    }
                });
            });
        }
        seen
    };
    let (parts, height, _) = run(true);
    assert_eq!(parts, 2, "the lead is not counted in the parts");
    assert!(height >= 89.0, "three buttons (lead + two) in one frame: {height}");
    let (_, plain, _) = run(false);
    assert!(plain < 70.0, "no quiet member hidden: no lead: {plain}");
}

#[test]
fn right_bundle_puts_the_arrows_at_the_right_end_and_hides_the_left_ones_first() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let run = |limit: f32| {
        let mut seen = (Rect::NOTHING, Rect::NOTHING);
        for i in 0..6 {
            run_frame(&ctx, 0.1 * f64::from(i), vec![], |ui| {
                Bundle("rb").right().until(limit).set(ui);
                let a = Locate(Button(None).bundle("rb").size((60.0, 30.0))).right(100.0).top(4.0).show(ui);
                let b = Locate(Button(None).bundle("rb").size((60.0, 30.0))).right(4.0).top(4.0).show(ui);
                seen = (a.rect, b.rect);
            });
        }
        seen
    };
    let (a, b) = run(10.0);
    assert!(a.width() > 0.0 && b.width() > 0.0, "room: both are shown");
    assert!((b.max.x - 796.0).abs() < 1.5, "the right one stays at the right edge: {b:?}");
    let (a, b) = run(690.0);
    assert_eq!(a.width(), 0.0, "the left one went into the list");
    assert!(b.width() > 0.0, "the right one is still shown");
    assert!(b.max.x < 790.0, "it made room for the arrows at the right end: {b:?}");
}

#[test]
fn one_right_bundle_can_hold_members_on_both_sides_of_a_middle_zone() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let run = |zone: (f32, f32)| {
        let mut seen = (Rect::NOTHING, Rect::NOTHING);
        for i in 0..6 {
            run_frame(&ctx, 0.1 * f64::from(i), vec![], |ui| {
                Bundle("both").right().avoid(zone.0, zone.1).set(ui);
                let l = Locate(Button(None).bundle("both").size((60.0, 30.0))).left(100.0).top(4.0).show(ui);
                let r = Locate(Button(None).bundle("both").size((60.0, 30.0))).right(4.0).top(4.0).show(ui);
                seen = (l.rect, r.rect);
            });
        }
        seen
    };
    let (l, r) = run((300.0, 500.0));
    assert!(l.width() > 0.0 && r.width() > 0.0, "room: both are shown");
    let (l, r) = run((120.0, 500.0));
    assert_eq!(l.width(), 0.0, "the left one runs into the zone: into the list");
    assert!(r.width() > 0.0, "the right one stays");
}
