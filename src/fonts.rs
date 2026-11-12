//! Asli bold font (system se), taaki `.bold()` sach mein bold dikhe.

use egui::{Context, FontData, FontDefinitions, FontFamily};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

static BOLD_READY: AtomicBool = AtomicBool::new(false);

/// System mein bold fonts ki jagah (pehla jo mile wahi use hota hai).
const BOLD_PATHS: [&str; 5] = [
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "C:\\Windows\\Fonts\\arialbd.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
];

/// Bold font ka family naam.
pub(crate) fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Bold font mil gaya ya nahi.
pub(crate) fn has_bold() -> bool {
    BOLD_READY.load(Ordering::Relaxed)
}

/// Bold font dhundh kar egui mein lagao. Na mile to `.bold()` gehre color par chalta rahega.
pub(crate) fn install(ctx: &Context) {
    let Some(bytes) = BOLD_PATHS.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("niv_bold".to_owned(), Arc::new(FontData::from_owned(bytes)));
    let mut chain = vec!["niv_bold".to_owned()];
    chain.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts.families.insert(bold_family(), chain);
    ctx.set_fonts(fonts);
    BOLD_READY.store(true, Ordering::Relaxed);
}
