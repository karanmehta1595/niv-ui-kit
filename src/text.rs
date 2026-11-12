//! Step 2: Text.
//!
//! ```ignore
//! ui.text("Hello").bold().italic().underline().strike().color(Color::RED).size(18.0);
//! ui.text("Hello ").bold().then("world").italic();
//! if ui.text("Open").response().clicked() { }
//! ```
//!
//! Text statement khatam hote hi apne aap draw ho jata hai (`.show()` ki zaroorat nahi).

use crate::theme::Color;
use egui::{text::LayoutJob, FontId, Label, Response, Stroke, TextFormat, TextStyle, TextWrapMode, Ui};

#[derive(Clone)]
struct Segment {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    mono: bool,
    color: Option<Color>,
    size: Option<f32>,
    opacity: Option<f32>,
}

impl Segment {
    fn new(text: String) -> Self {
        Self {
            text,
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            mono: false,
            color: None,
            size: None,
            opacity: None,
        }
    }
}

/// Text builder. `ui.text("..")` se milta hai, chain mein styles lagao.
pub struct Text<'a> {
    ui: &'a mut Ui,
    segments: Vec<Segment>,
    selectable: bool,
    wrap: Option<TextWrapMode>,
    drawn: bool,
}

impl<'a> Text<'a> {
    fn new(ui: &'a mut Ui, text: String) -> Self {
        Self {
            ui,
            segments: vec![Segment::new(text)],
            selectable: false,
            wrap: None,
            drawn: false,
        }
    }

    fn edit(mut self, change: impl FnOnce(&mut Segment)) -> Self {
        if let Some(segment) = self.segments.last_mut() {
            change(segment);
        }
        self
    }

    /// Bold. System ka bold font (macOS Arial Bold, Windows Arial Bold, Linux DejaVu/Liberation Bold)
    /// mil jaye to asli bold, warna gehra (strong) color.
    pub fn bold(self) -> Self {
        self.edit(|s| s.bold = true)
    }

    /// Italic.
    pub fn italic(self) -> Self {
        self.edit(|s| s.italic = true)
    }

    /// Neeche line.
    pub fn underline(self) -> Self {
        self.edit(|s| s.underline = true)
    }

    /// Beech se kati hui line.
    pub fn strike(self) -> Self {
        self.edit(|s| s.strike = true)
    }

    /// Monospace (code jaisa) font.
    pub fn mono(self) -> Self {
        self.edit(|s| s.mono = true)
    }

    /// Text ka color.
    pub fn color(self, color: Color) -> Self {
        self.edit(|s| s.color = Some(color))
    }

    /// Font size (points).
    pub fn size(self, size: f32) -> Self {
        self.edit(|s| s.size = Some(size))
    }

    /// Opacity, 0.0 (gayab) se 1.0 (poora).
    pub fn opacity(self, opacity: f32) -> Self {
        self.edit(|s| s.opacity = Some(opacity))
    }

    /// Text select/copy ho sake (default: nahi).
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// Lambi line ho to agli line mein jaye.
    pub fn wrap(mut self) -> Self {
        self.wrap = Some(TextWrapMode::Wrap);
        self
    }

    /// Lambi line ho to ... lagakar kaat de.
    pub fn truncate(mut self) -> Self {
        self.wrap = Some(TextWrapMode::Truncate);
        self
    }

    /// Ek hi line mein agla hissa, alag style ke saath: `.then("world").italic()`.
    pub fn then(mut self, text: impl Into<String>) -> Self {
        self.segments.push(Segment::new(text.into()));
        self
    }

    /// Abhi draw karo aur click/hover wala `Response` do.
    pub fn response(mut self) -> Response {
        self.drawn = true;
        self.draw()
    }

    fn draw(&mut self) -> Response {
        let mut job = LayoutJob::default();
        for segment in &self.segments {
            let style = self.ui.style();
            let visuals = &style.visuals;
            let text_style = if segment.mono { TextStyle::Monospace } else { TextStyle::Body };
            let mut font_id: FontId = text_style.resolve(style);
            if let Some(size) = segment.size {
                font_id.size = size;
            }
            let real_bold = segment.bold && !segment.mono && crate::fonts::has_bold();
            if real_bold {
                font_id = FontId::new(font_id.size, crate::fonts::bold_family());
            }
            let mut color = segment.color.unwrap_or(if segment.bold && !real_bold {
                visuals.strong_text_color()
            } else {
                visuals.text_color()
            });
            if let Some(opacity) = segment.opacity {
                color = color.gamma_multiply(opacity.clamp(0.0, 1.0));
            }
            let line = Stroke::new(1.0, color);
            job.append(
                &segment.text,
                0.0,
                TextFormat {
                    font_id,
                    color,
                    italics: segment.italic,
                    underline: if segment.underline { line } else { Stroke::NONE },
                    strikethrough: if segment.strike { line } else { Stroke::NONE },
                    ..Default::default()
                },
            );
        }
        let mut label = Label::new(job).selectable(self.selectable);
        if let Some(mode) = self.wrap {
            label = label.wrap_mode(mode);
        }
        self.ui.add(label)
    }
}

impl Drop for Text<'_> {
    fn drop(&mut self) {
        if !self.drawn {
            let _ = self.draw();
        }
    }
}

/// `ui.text("..")` ke liye extension.
pub trait UiExt {
    /// Styled text shuru karo.
    fn text(&mut self, text: impl Into<String>) -> Text<'_>;
    /// Itna scroll karo ki `rect` scroll area ke beech mein aa jaye.
    fn scroll_to_center(&self, rect: egui::Rect);
}

impl UiExt for Ui {
    fn text(&mut self, text: impl Into<String>) -> Text<'_> {
        Text::new(self, text.into())
    }

    fn scroll_to_center(&self, rect: egui::Rect) {
        self.scroll_to_rect(rect, Some(egui::Align::Center));
    }
}

/// Text ko `width` px mein fit karo: lamba ho to peeche "…" laga ke chhota karta hai.
pub fn fit(ui: &Ui, text: &str, size: f32, width: f32) -> String {
    let font = FontId::proportional(size);
    let measure = |s: &str| ui.painter().layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    if measure(text) <= width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0usize, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect::<String>() + "\u{2026}";
        if measure(&candidate) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    chars[..low].iter().collect::<String>().trim_end().to_owned() + "\u{2026}"
}

/// Text ko shabdon par tod kar `width` px ki lines mein baanto (`\n` bhi maanta hai).
pub fn wrap_lines(ui: &Ui, text: &str, size: f32, width: f32) -> Vec<String> {
    let font = FontId::proportional(size);
    let measure = |s: &str| ui.painter().layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    let mut lines: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let tentative = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
            if current.is_empty() || measure(&tentative) <= width {
                current = tentative;
            } else {
                lines.push(std::mem::take(&mut current));
                current = word.to_owned();
            }
        }
        lines.push(current);
    }
    lines
}

/// Text clipboard mein copy karo.
pub fn copy_text(ui: &Ui, text: impl Into<String>) {
    ui.ctx().copy_text(text.into());
}

/// Text ki chaudai (px). `mono`: monospace font mein.
pub fn text_width(ui: &Ui, text: &str, size: f32, mono: bool) -> f32 {
    let font = if mono { FontId::monospace(size) } else { FontId::proportional(size) };
    ui.painter().layout_no_wrap(text.to_owned(), font, egui::Color32::WHITE).size().x
}
