//! App trait, `run()` aur `app("..")` launcher.

use egui::{Context, Ui, ViewportBuilder};

/// Tumhari app yahi trait implement karti hai.
///
/// ```ignore
/// impl App for MyApp {
///     fn ui(&mut self, ui: &mut Ui) { }
/// }
/// ```
pub trait App {
    /// Har frame pe call hota hai. Yahin saara UI banta hai.
    fn ui(&mut self, ui: &mut Ui);
}

struct Adapter<A: App>(A);

impl<A: App> eframe::App for Adapter<A> {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        crate::theme::paint_background(ui);
        self.0.ui(ui);
    }
}

/// Window ke options. `app("Title")` se shuru karo, aakhir mein `.run(MyApp)`.
///
/// ```ignore
/// fn main() {
///     app("My App").size(1000.0, 700.0).min_size(600.0, 400.0).run(MyApp::default());
/// }
/// ```
pub struct AppBuilder {
    title: String,
    size: Option<[f32; 2]>,
    min_size: Option<[f32; 2]>,
    resizable: bool,
    transparent: bool,
    decorations: bool,
    maximized: bool,
    hide_titlebar: bool,
    icon: Option<&'static [u8]>,
    setup: Option<SetupFn>,
}

type SetupFn = Box<dyn FnOnce(&Context)>;

/// Naya app launcher banao, window title ke saath.
pub fn app(title: impl Into<String>) -> AppBuilder {
    AppBuilder {
        title: title.into(),
        size: None,
        min_size: None,
        resizable: true,
        transparent: false,
        decorations: true,
        maximized: false,
        hide_titlebar: false,
        icon: None,
        setup: None,
    }
}

impl AppBuilder {
    /// Window ki shuruaati size (width, height).
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Some([width, height]);
        self
    }

    /// Window ki sabse chhoti size.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Some([width, height]);
        self
    }

    /// Window ki size badli ja sake ya nahi (default: haan).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Transparent window (blur / glass ke liye zaroori).
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Title bar aur border dikhao ya nahi (default: haan).
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Maximized shuru ho.
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Title bar aur title chhupao, content poori window tak (macOS style).
    /// Ye teen egui options ek saath lagata hai: fullsize content, titlebar band, title band.
    pub fn hide_titlebar(mut self) -> Self {
        self.hide_titlebar = true;
        self
    }

    /// App icon, PNG bytes se: `.icon(include_bytes!("../assets/icons/app.png"))`.
    pub fn icon(mut self, png_bytes: &'static [u8]) -> Self {
        self.icon = Some(png_bytes);
        self
    }

    /// Start mein ek baar chalne wala setup (fonts, style wagairah). `ctx` egui ka Context hai.
    pub fn setup(mut self, setup: impl FnOnce(&Context) + 'static) -> Self {
        self.setup = Some(Box::new(setup));
        self
    }

    /// App chalao. Error aaye to stderr pe likhkar band ho jata hai, isliye `main()` mein
    /// return type likhne ki zaroorat nahi.
    pub fn run<A: App + 'static>(self, app: A) {
        if let Err(error) = self.try_run(app) {
            eprintln!("niv-ui-kit: {error}");
            std::process::exit(1);
        }
    }

    /// App chalao aur `Result` wapas do (apne error handling ke liye).
    pub fn try_run<A: App + 'static>(self, app: A) -> eframe::Result {
        let mut viewport = ViewportBuilder::default()
            .with_title(self.title.clone())
            .with_resizable(self.resizable)
            .with_transparent(self.transparent)
            .with_decorations(self.decorations)
            .with_maximized(self.maximized);
        if self.hide_titlebar {
            viewport = viewport
                .with_fullsize_content_view(true)
                .with_titlebar_shown(false)
                .with_title_shown(false);
        }
        if let Some(bytes) = self.icon {
            match eframe::icon_data::from_png_bytes(bytes) {
                Ok(icon) => viewport = viewport.with_icon(icon),
                Err(error) => eprintln!("niv-ui-kit: icon load nahi hua: {error}"),
            }
        }
        if let Some(size) = self.size {
            viewport = viewport.with_inner_size(size);
        }
        if let Some(size) = self.min_size {
            viewport = viewport.with_min_inner_size(size);
        }
        let options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };
        let setup = self.setup;
        eframe::run_native(
            &self.title,
            options,
            Box::new(move |cc| {
                crate::theme::install(&cc.egui_ctx);
                crate::fonts::install(&cc.egui_ctx);
                #[cfg(feature = "all_loaders")]
                egui_extras::install_image_loaders(&cc.egui_ctx);
                if let Some(gl) = cc.gl.as_ref() {
                    if let Err(error) = egui_bg_layer::install_glow(&cc.egui_ctx, gl) {
                        eprintln!("egui_bg_layer: {error}");
                    }
                }
                let _ = egui_bg_layer::install_macos(cc);
                if let Some(setup) = setup {
                    setup(&cc.egui_ctx);
                }
                Ok(Box::new(Adapter(app)))
            }),
        )
    }
}

/// Window kholo aur app chalao (title: "niv-ui-kit"). Chhoti apps ke liye.
pub fn run<A: App + 'static>(app: A) -> eframe::Result {
    run_titled("niv-ui-kit", app)
}

/// Window kholo aur app chalao, apne title ke saath.
pub fn run_titled<A: App + 'static>(title: &str, application: A) -> eframe::Result {
    app(title).try_run(application)
}
