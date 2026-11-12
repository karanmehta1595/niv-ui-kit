use niv_ui_kit::prelude::*;

#[derive(Default)]
struct MyApp {
    clicks: u32,
}

impl App for MyApp {
    fn ui(&mut self, ui: &mut Ui) {
        ui.text("app() launcher").bold().size(22.0);
        ui.text("main() mein return type ya Box::new nahi chahiye");
        if ui.button(format!("Clicks: {}", self.clicks)).clicked() {
            self.clicks += 1;
        }
    }
}

fn main() {
    app("Window demo")
        .size(900.0, 600.0)
        .min_size(500.0, 300.0)
        .run(MyApp::default());
}
