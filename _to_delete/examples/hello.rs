use niv_ui_kit::prelude::*;

#[derive(Default)]
struct MyApp {
    clicks: u32,
}

impl App for MyApp {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Hello from niv-ui-kit");
        if ui.button("Click me").clicked() {
            self.clicks += 1;
        }
        ui.label(format!("Clicks: {}", self.clicks));
    }
}

fn main() -> niv_ui_kit::eframe::Result {
    run(MyApp::default())
}
