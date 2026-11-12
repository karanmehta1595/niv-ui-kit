//! Asli shell chala kar terminal widget ko test karta hai.
#![cfg(unix)]

use niv_ui_kit::terminal::Terminal;
use std::time::{Duration, Instant};

fn run_frame(ctx: &egui::Context, term: &mut Terminal) {
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 400.0))),
        ..Default::default()
    };
    let out = ctx.run_ui(raw, |ui| {
        term.pump(ui.ctx());
        term.show(ui);
    });
    std::mem::forget(out);
}

#[test]
fn a_shell_runs_a_line_and_exits() {
    let ctx = egui::Context::default();
    niv_ui_kit::init(&ctx);
    let mut term = Terminal::spawn(ctx.clone(), None, Some("/bin/sh")).expect("a shell");
    run_frame(&ctx, &mut term);
    term.send_line("echo niv_terminal_works");
    let start = Instant::now();
    let mut seen = false;
    while start.elapsed() < Duration::from_secs(5) {
        run_frame(&ctx, &mut term);
        if term.screen_text().contains("niv_terminal_works") {
            seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen, "the output of echo is on the screen");
    term.send_line("exit");
    let start = Instant::now();
    while !term.exited() && start.elapsed() < Duration::from_secs(5) {
        run_frame(&ctx, &mut term);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(term.exited());
}
