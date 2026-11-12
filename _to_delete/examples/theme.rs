use niv_ui_kit::prelude::*;

struct ThemeDemo;

impl App for ThemeDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Step 1: Colors + Theme");
        ui.horizontal(|ui| {
            if ui.button("System").clicked() {
                set_theme(ui.ctx(), ThemePreference::System);
            }
            if ui.button("Dark").clicked() {
                set_theme(ui.ctx(), ThemePreference::Dark);
            }
            if ui.button("Light").clicked() {
                set_theme(ui.ctx(), ThemePreference::Light);
            }
        });
        ui.colored_label(Color::RED, "Color::RED");
        ui.colored_label(rgb(0, 122, 255), "rgb(0, 122, 255)");
        ui.colored_label(rgba(255, 149, 0, 160), "rgba(255, 149, 0, 160)");
        ui.colored_label(Color::hex("#34c759"), "Color::hex(\"#34c759\")");
        let dark = niv_ui_kit::theme::is_dark(ui);
        ui.label(if dark { "Dark: 28, 28, 30" } else { "Light: 245, 245, 247" });
    }
}

fn main() -> niv_ui_kit::eframe::Result {
    run_titled("Theme demo", ThemeDemo)
}
