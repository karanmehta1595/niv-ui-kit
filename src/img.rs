//! Img: tasveer (file, bytes ya decode hue pixels) screen par dikhana.
//!
//! ```ignore
//! let a = Img::file("/tmp/a.png");                       // file se (png, jpg, svg ...)
//! let b = Img::bytes("logo", include_bytes!("logo.png")); // memory se, `"logo"` = naam
//! let c = Img::rgba(w, h, pixels);                        // decode hue RGBA pixels
//! a.show(ui, vec2(200.0, 120.0));                         // itni jagah mein poori dikhao
//! a.fit(vec2(200.0, 200.0)).radius(8.0).show(ui);         // ratio bacha ke andar fit
//! a.zoom(2.0).pan(vec2(10.0, 0.0)).size(rect.size()).show(ui); // viewer jaisa
//! if let Some(size) = a.size(ctx) { /* asli size, jab load ho gayi */ }
//! ```
//! Load hone mein time lage to ye khud `repaint` maangta rahta hai. Na mile to kuch nahi banta.

use crate::shadow::ShadowColor;
use crate::theme::Color;
use egui::{
    load::{SizedTexture, TexturePoll},
    pos2, vec2, ColorImage, Context, Rect, Response, Sense, SizeHint, TextureHandle, TextureId, TextureOptions, Ui, Vec2,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
enum Source {
    Uri(String),
    Bytes { name: String, bytes: Arc<[u8]> },
    Pixels { size: [usize; 2], rgba: Arc<[u8]>, handle: Arc<Mutex<Option<TextureHandle>>> },
    Texture(SizedTexture),
}

/// `Img::load` ka natija.
#[derive(Clone, Debug)]
pub enum Load {
    /// Taiyaar: GPU texture ka id aur size.
    Ready {
        /// Texture id.
        id: TextureId,
        /// Size (px).
        size: Vec2,
    },
    /// Abhi load ho rahi hai (khud repaint maangti hai).
    Pending,
    /// Nahi khuli: wajah.
    Failed(String),
}

/// Ek tasveer. `Img::file / bytes / rgba` se banao, saste mein clone hota hai.
#[derive(Clone)]
pub struct Img {
    source: Source,
    options: TextureOptions,
}

impl Img {
    /// File ke path se.
    pub fn file(path: impl AsRef<str>) -> Img {
        let path = path.as_ref();
        let uri = if path.contains("://") { path.to_owned() } else { format!("file://{path}") };
        Img { source: Source::Uri(uri), options: TextureOptions::LINEAR }
    }

    /// Memory ke bytes se (`name` unique hona chahiye, jaise `"logo"`).
    pub fn bytes(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Img {
        Img { source: Source::Bytes { name: name.into(), bytes: bytes.into() }, options: TextureOptions::LINEAR }
    }

    /// Decode hue RGBA pixels se (`rgba.len() == w * h * 4`).
    pub fn rgba(width: usize, height: usize, rgba: impl Into<Arc<[u8]>>) -> Img {
        Img { source: Source::Pixels { size: [width, height], rgba: rgba.into(), handle: Arc::new(Mutex::new(None)) }, options: TextureOptions::LINEAR }
    }

    fn poll(&self, ctx: &Context) -> Option<SizedTexture> {
        match self.load_hint(ctx, SizeHint::default()) {
            Load::Ready { id, size } => Some(SizedTexture::new(id, size)),
            _ => None,
        }
    }

    fn load_hint(&self, ctx: &Context, hint: SizeHint) -> Load {
        let options = self.options;
        let poll_uri = |uri: &str| match ctx.try_load_texture(uri, options, hint) {
            Ok(TexturePoll::Ready { texture }) => Load::Ready { id: texture.id, size: texture.size },
            Ok(TexturePoll::Pending { .. }) => {
                ctx.request_repaint();
                Load::Pending
            }
            Err(e) => Load::Failed(e.to_string()),
        };
        match &self.source {
            Source::Uri(uri) => poll_uri(uri),
            Source::Texture(t) => Load::Ready { id: t.id, size: t.size },
            Source::Bytes { name, bytes } => {
                let uri = if name.contains("://") { name.clone() } else { format!("bytes://{name}") };
                ctx.include_bytes(uri.clone(), bytes.clone());
                poll_uri(&uri)
            }
            Source::Pixels { size, rgba, handle } => {
                let Ok(mut slot) = handle.lock() else { return Load::Failed("locked".into()) };
                if slot.is_none() && rgba.len() == size[0] * size[1] * 4 {
                    let image = ColorImage::from_rgba_unmultiplied(*size, rgba);
                    *slot = Some(ctx.load_texture("niv_img", image, options));
                }
                match slot.as_ref() {
                    Some(h) => Load::Ready { id: h.id(), size: h.size_vec2() },
                    None => Load::Failed("pixels do not match the size".into()),
                }
            }
        }
    }

    /// Ek texture se (jaise CodeMe ke apne bane `TextureHandle` se).
    pub fn texture(handle: &TextureHandle) -> Img {
        Img { source: Source::Texture(SizedTexture::from_handle(handle)), options: TextureOptions::LINEAR }
    }

    /// Texture ke options seedha (nearest/linear/repeat).
    pub fn options(mut self, options: TextureOptions) -> Img {
        self.options = options;
        self
    }

    /// Bade karne par pixels dikhen (nearest), chikne nahi.
    pub fn sharp(mut self) -> Img {
        self.options = TextureOptions::NEAREST;
        self
    }

    /// Is tasveer ka address (file/bytes ke liye), texture/pixels ke liye `None`.
    pub fn uri(&self) -> Option<String> {
        match &self.source {
            Source::Uri(u) => Some(u.clone()),
            Source::Bytes { name, .. } => Some(if name.contains("://") { name.clone() } else { format!("bytes://{name}") }),
            _ => None,
        }
    }

    /// `size` ke hisaab se load karo (SVG ko itne size par banata hai). Ready / Pending / Failed.
    pub fn load(&self, ctx: &Context, size: Vec2) -> Load {
        let hint = SizeHint::Size { width: size.x.max(1.0) as u32, height: size.y.max(1.0) as u32, maintain_aspect_ratio: true };
        self.load_hint(ctx, hint)
    }

    /// Decode hue pixels (SVG ko `side` x `side` mein), jab taiyaar ho.
    pub fn pixels(&self, ctx: &Context, side: u32) -> Option<ColorImage> {
        let hint = SizeHint::Size { width: side, height: side, maintain_aspect_ratio: true };
        let uri = self.uri()?;
        if let Source::Bytes { bytes, .. } = &self.source {
            ctx.include_bytes(uri.clone(), bytes.clone());
        }
        match ctx.try_load_image(&uri, hint) {
            Ok(egui::load::ImagePoll::Ready { image }) => Some((*image).clone()),
            _ => None,
        }
    }

    /// `rect` mein seedha banao (ratio nahi bachata, rect bhar deta hai).
    pub fn paint_at(&self, ui: &Ui, rect: Rect) {
        if let Load::Ready { id, .. } = self.load(ui.ctx(), rect.size()) {
            ui.painter().image(id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color::WHITE);
        }
    }

    /// Asli size (pixels), jab load ho chuki ho.
    pub fn size(&self, ctx: &Context) -> Option<Vec2> {
        self.poll(ctx).map(|t| t.size)
    }

    /// GPU texture ka id (jab load ho chuki ho).
    pub fn texture_id(&self, ctx: &Context) -> Option<TextureId> {
        self.poll(ctx).map(|t| t.id)
    }

    /// `size` jitni jagah mein poori tasveer dikhao.
    pub fn show(&self, ui: &mut Ui, size: Vec2) -> Response {
        ImgShow::new(self.clone()).size(size).show(ui)
    }

    /// Ratio bacha ke `max` ke andar fit karo.
    pub fn fit(&self, max: Vec2) -> ImgShow {
        ImgShow::new(self.clone()).fit(max)
    }

    /// Bade karo (1.0 = fit/size jitna): `show` se pehle `.size()` ya `.fit()` bhi do.
    pub fn zoom(&self, zoom: f32) -> ImgShow {
        ImgShow::new(self.clone()).zoom(zoom)
    }
}

/// `img.fit(..)` / `img.zoom(..)` ke baad ke options.
pub struct ImgShow {
    img: Img,
    size: Option<Vec2>,
    fit: Option<Vec2>,
    zoom: f32,
    pan: Vec2,
    radius: f32,
    tint: Color,
    sense: Sense,
}

impl ImgShow {
    fn new(img: Img) -> Self {
        ImgShow { img, size: None, fit: None, zoom: 1.0, pan: Vec2::ZERO, radius: 0.0, tint: Color::WHITE, sense: Sense::hover() }
    }
    /// Itni jagah lo.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    /// Ratio bacha ke `max` ke andar.
    pub fn fit(mut self, max: Vec2) -> Self {
        self.fit = Some(max);
        self
    }
    /// 1.0 = normal, 2.0 = do guna bada (frame mein kat jata hai).
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom.max(0.01);
        self
    }
    /// Zoom ke baad kitna khisko (screen px).
    pub fn pan(mut self, pan: Vec2) -> Self {
        self.pan = pan;
        self
    }
    /// Kone ki golayi.
    pub fn radius(mut self, px: f32) -> Self {
        self.radius = px;
        self
    }
    /// Rang chadhao.
    pub fn tint(mut self, color: impl ShadowColor) -> Self {
        self.tint = color.shadow_color();
        self
    }
    /// Click bhi suno.
    pub fn clickable(mut self) -> Self {
        self.sense = Sense::click();
        self
    }
    /// Dikhao.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tex = self.img.poll(ui.ctx());
        let frame = match (self.size, self.fit, tex) {
            (Some(s), _, _) => s,
            (None, Some(max), Some(t)) => fit_into(t.size, max),
            (None, Some(max), None) => max,
            (None, None, Some(t)) => t.size,
            (None, None, None) => Vec2::ZERO,
        };
        let (rect, response) = ui.allocate_exact_size(frame, self.sense);
        if let (Some(t), true) = (tex, ui.is_rect_visible(rect)) {
            let drawn = fit_into(t.size, rect.size()) * self.zoom;
            let image_rect = Rect::from_center_size(rect.center() + self.pan, drawn);
            let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            if self.radius > 0.5 {
                // golayi: image ko rounded rect ki mesh par lagao
                let clipped = image_rect.intersect(rect);
                painter.add(egui::Shape::mesh(rounded_mesh(clipped, self.radius, image_rect, t.id, self.tint)));
            } else {
                painter.image(t.id, image_rect, uv, self.tint);
            }
        }
        response
    }
}

/// `src` ko ratio bacha ke `max` mein fit karne ka size (chhote ko bhi `max` tak bada karta hai).
pub(crate) fn fit_into(src: Vec2, max: Vec2) -> Vec2 {
    if src.x <= 0.0 || src.y <= 0.0 {
        return Vec2::ZERO;
    }
    let scale = (max.x / src.x).min(max.y / src.y);
    vec2(src.x * scale, src.y * scale)
}

/// Rounded rect `r` ki mesh, jisme `uv` `image_rect` ke hisaab se lagte hain.
fn rounded_mesh(r: Rect, radius: f32, image_rect: Rect, tex: TextureId, tint: Color) -> egui::Mesh {
    let radius = radius.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    let mut out = egui::Mesh::with_texture(tex);
    let mut ring: Vec<egui::Pos2> = Vec::new();
    let corners = [
        (pos2(r.max.x - radius, r.min.y + radius), -std::f32::consts::FRAC_PI_2),
        (pos2(r.max.x - radius, r.max.y - radius), 0.0),
        (pos2(r.min.x + radius, r.max.y - radius), std::f32::consts::FRAC_PI_2),
        (pos2(r.min.x + radius, r.min.y + radius), std::f32::consts::PI),
    ];
    for (center, start) in corners {
        for i in 0..=6 {
            let a = start + std::f32::consts::FRAC_PI_2 * (i as f32 / 6.0);
            ring.push(pos2(center.x + radius * a.cos(), center.y + radius * a.sin()));
        }
    }
    let uv_of = |p: egui::Pos2| {
        pos2(
            ((p.x - image_rect.min.x) / image_rect.width().max(1.0)).clamp(0.0, 1.0),
            ((p.y - image_rect.min.y) / image_rect.height().max(1.0)).clamp(0.0, 1.0),
        )
    };
    let c = r.center();
    out.vertices.push(egui::epaint::Vertex { pos: c, uv: uv_of(c), color: tint });
    for p in &ring {
        out.vertices.push(egui::epaint::Vertex { pos: *p, uv: uv_of(*p), color: tint });
    }
    let n = ring.len() as u32;
    for i in 0..n {
        out.indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % n]);
    }
    out
}
