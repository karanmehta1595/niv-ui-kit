use niv_ui_kit::prelude::*;

struct TextDemo {
    clicks: u32,
}

impl App for TextDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.text("Step 2: Text").bold().size(22.0);
        ui.text("Normal text");
        ui.text("Bold").bold();
        ui.text("Italic").italic();
        ui.text("Underline").underline();
        ui.text("Strike").strike();
        ui.text("Mono: let x = 5;").mono();
        ui.text("Red, size 18").color(Color::RED).size(18.0);
        ui.text("Hex color").color(Color::hex("#34c759"));
        ui.text("Opacity 40%").opacity(0.4);
        ui.text("Sab ek saath").bold().italic().underline().strike().color(rgb(0, 122, 255));
        ui.text("Aadha ").bold().then("aadha").italic().color(Color::RED).then(" aur normal");
        ui.text("Is text ko select/copy karo").selectable();
        ui.text("Bahut lambi line jo window chhoti karne par kat jayegi aur ... lag jayega").truncate();
        if ui.text(format!("Click karo ({} baar)", self.clicks)).underline().response().clicked() {
            self.clicks += 1;
        }
    }
}

fn main() -> niv_ui_kit::eframe::Result {
    run_titled("Text demo", TextDemo { clicks: 0 })
}
