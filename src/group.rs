//! `Group`: ek rect (capsule) ke andar kai buttons, beech mein patli lakeer (separator).
//! Har hissa apna hover/click alag leta hai: cursor jis hisse par ho wahi chamakta hai.
//!
//! ```ignore
//! let run = Group("run")
//!     .button(Button(None).image(Image(PLAY).size(18.0)).size((36.0, 28.0)))
//!     .separator()                                                    // lakeer: PLAY ke baad
//!     .button(Button(None).image(Image(STOP).size(18.0)).size((36.0, 28.0)))
//!     .show(ui);
//! if run.clicked(0) { /* play */ }
//! if run.clicked(1) { /* stop */ }
//!
//! Group("nav").glass().blur(40).saturation(1.6).radius(14.0).padding(3.0, 3.0).button(back).button(forward).show(ui);
//! ```
//!
//! Rect (capsule) ke saare rang-roop wahi hain jo `Button` ke: `.fill`, `.fill_opacity`, `.gradient`,
//! `.multi_gradient`, `.animate`, `.border_*`, `.border_glow*`, `.glow`, `.shadow`, `.glass`, `.blur`,
//! `.saturation`, `.brightness`, `.tint`, `.opacity`. Andar ke buttons ka hover/pressed `.hover(..)` /
//! `.pressed(..)` se, sab buttons ka ek saath.

use crate::button::{bundle_slot, BundleItem, Button, Hover, Image, IntoSize, Label, Measured, Placeable, Spot, State};
use crate::paint::Glow;
use crate::panel::{Fill, Opacity};
use crate::theme::{is_dark, light, rgba, Color};
use egui::{pos2, vec2, Id, Rect, Response, Stroke, Ui, Vec2};

/// Ek rect ke andar kai buttons. `Group("id")` se banao, `.button(..)` se bharo, `.show(ui)` se dikhao.
#[must_use = "Group ke aakhir mein .show(ui) likho"]
#[derive(Clone)]
pub struct Group {
    bundle: Option<String>,
    lead: Option<Button>,
    id: Option<String>,
    buttons: Vec<Button>,
    /// `sep_after[i]`: `buttons[i]` ke baad lakeer.
    sep_after: Vec<bool>,
    all_sep: bool,
    sep_width: f32,
    sep_length: Option<f32>,
    /// Har button ke apne corner (jahan `.button_corners(..)` se diye).
    part_ids: Vec<Option<String>>,
    id_corners: Vec<(String, [f32; 4])>,
    scale_states: Vec<State>,
    /// Peeche ka rect: ek khali `Button`, jiski saari styling Group ke methods se aati hai.
    frame: Button,
    radius: Option<f32>,
    corners: Option<[f32; 4]>,
    seam_radius: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
    styled: bool,
    padding: (f32, f32),
    gap: f32,
    separator_color: Option<Fill>,
    vertical: bool,
    hover: Option<Hover>,
    pressed: Option<Hover>,
    transition: Option<f32>,
}

/// Naya group. `id` har group ka alag hona chahiye.
#[allow(non_snake_case)]
pub fn Group(id: impl Into<String>) -> Group {
    Group {
        id: Some(id.into()),
        buttons: Vec::new(),
        sep_after: Vec::new(),
        all_sep: false,
        sep_width: 1.0,
        sep_length: None,
        bundle: None,
        lead: None,
        part_ids: Vec::new(),
        id_corners: Vec::new(),
        scale_states: Vec::new(),
        frame: Button(None),
        radius: None,
        corners: None,
        seam_radius: None,
        width: None,
        height: None,
        styled: false,
        padding: (0.0, 0.0),
        gap: 0.0,
        separator_color: None,
        vertical: false,
        hover: None,
        pressed: None,
        transition: None,
    }
}

struct Plan {
    fixed: Vec<Option<Vec2>>,
    measured: Vec<Measured>,
    want_main: Option<f32>,
    main: f32,
    cross: f32,
    inner: Vec2,
    outer: Vec2,
}

/// `Group::show` ka jawab: poore rect ka `Response` aur har hisse ka alag.
pub struct GroupResponse {
    /// Poore rect ka response.
    pub response: Response,
    parts: Vec<Response>,
    lead: Option<Response>,
}

impl GroupResponse {
    /// `index` wale button par click hua?
    pub fn clicked(&self, index: usize) -> bool {
        self.parts.get(index).is_some_and(Response::clicked)
    }

    /// How many buttons the group itself has (the `bundle_lead` button is not counted).
    pub fn parts_len(&self) -> usize {
        self.parts.len()
    }

    /// `bundle_lead` button par click hua (sirf `>>` ki list mein)?
    pub fn lead_clicked(&self) -> bool {
        self.lead.as_ref().is_some_and(Response::clicked)
    }

    /// Cursor `index` wale button par hai?
    pub fn hovered(&self, index: usize) -> bool {
        self.parts.get(index).is_some_and(Response::hovered)
    }

    /// Kis button par click hua (agar kisi par hua).
    pub fn which(&self) -> Option<usize> {
        self.parts.iter().position(Response::clicked)
    }

    /// `index` wale button ka poora `Response`.
    pub fn part(&self, index: usize) -> Option<&Response> {
        self.parts.get(index)
    }
}

impl Group {
    // ---- buttons aur bandish ----

    /// Button se shuru: uska rang-roop poore rect ka, content pehle hisse ka (dekho [`Button::separator`]).
    pub(crate) fn from_button(first: Button) -> Group {
        let (frame, part) = first.split();
        let mut group = Group("");
        group.id = None;
        group.styled = first.has_look();
        let radii = first.radii();
        group.corners = Some(radii);
        group.frame = frame;
        group.part_ids.push(first.id.clone());
        group.buttons.push(part);
        group.sep_after.push(false);
        group
    }

    /// Ek aur hissa, pichhle ke baad. Button ka content (text/image), size, padding, hover, pressed
    /// aur keys is hisse ke hote hain; rang-roop poore rect ka (Group ka) rehta hai.
    pub fn button(mut self, button: Button) -> Self {
        self.part_ids.push(button.id.clone());
        self.buttons.push(button);
        self.sep_after.push(false);
        self
    }

    /// Bundle mein: jab `>>` khule aur same bundle ka koi `Button::bundle_quiet()` chhup chuka ho, to ye button is
    /// Group ke frame ke andar sabse upar jud jata hai (jaise Terminal Build/Stop/Run ke saath). Bar mein ye nahi
    /// dikhta. Iska click `GroupResponse::lead_clicked()` se, `clicked(i)` ke index nahi badalte.
    pub fn bundle_lead(mut self, button: Button) -> Self {
        self.lead = Some(button);
        self
    }

    /// Pichhle button ke baad patli lakeer (agla `.button(..)` uske baad aayega).
    pub fn separator(mut self) -> Self {
        if let Some(last) = self.sep_after.last_mut() {
            *last = true;
        }
        self
    }

    /// Har do buttons ke beech lakeer (pehle likho ya baad mein, sab buttons ke liye chalti hai).
    pub fn separators(mut self) -> Self {
        self.all_sep = true;
        self
    }

    /// Lakeer ki motai (default 1.0 px).
    pub fn separator_width(mut self, px: f32) -> Self {
        self.sep_width = px.max(0.0);
        self
    }

    /// Lakeer ki lambai px mein (default: rect ki aadhi unchai).
    pub fn separator_length(mut self, px: f32) -> Self {
        self.sep_length = Some(px.max(0.0));
        self
    }

    /// `id` wale button ke chaaron corner ek jaise: `.button_radius("hammer", 32.0)`.
    pub fn button_radius(self, id: &str, px: f32) -> Self {
        self.button_corners(id, px, px, px, px)
    }

    /// Group ka scale: `.hovered()` wale state se peeche ka frame bada hota hai (buttons nahi), `.clicked()` wale
    /// state se dabaya hua button dabta hai (frame hover wala hi rehta hai):
    /// `.scale(State::scale(1.1).hovered()).scale(State::push(0.4).clicked())`; in pixels:
    /// `.scale(State::pop_px(3.0).hovered()).scale(State::push_px(2.0).clicked())`.
    pub fn scale(mut self, state: State) -> Self {
        self.scale_states.push(state);
        self
    }

    /// `id` wale button ke corner (px): top-left, top-right, bottom-right, bottom-left.
    /// Button ka naam `Button.id("hammer")` se aata hai; kab likha (buttons se pehle/baad) farak nahi padta.
    /// `(_____|` jaisa pehla: `.button_corners("hammer", 32.0, 0.0, 0.0, 32.0)`,
    /// aakhri `|_____)`: `.button_corners("stop", 0.0, 32.0, 32.0, 0.0)`.
    /// Jis button par na likho wo Group ke hisaab se apne aap tay hota hai (bahar gol, lakeer par seedha).
    pub fn button_corners(mut self, id: &str, top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        let c = [top_left, top_right, bottom_right, bottom_left].map(|v| v.max(0.0));
        self.id_corners.retain(|(name, _)| name != id);
        self.id_corners.push((id.to_owned(), c));
        self
    }

    /// Lakeer ke paas wale corner ki gol-ai (px). `0.0`: seedha, aadha kata hua (default jab padding nahi);
    /// bada: har hissa lakeer par bhi gol, jaise alag-alag goliyan. Na likho to padding ke hisaab se apne aap.
    pub fn separator_radius(mut self, px: f32) -> Self {
        self.seam_radius = Some(px.max(0.0));
        self
    }

    /// Rect ka size: `.size(vec2(80.0, 40.0))`, `(80.0, 40.0)` ya `40.0`. Buttons is rect ko barabar
    /// baant lete hain (apna size nahi chalta). Na likho to rect buttons ke size se banta hai.
    pub fn size(mut self, size: impl IntoSize) -> Self {
        let size = size.into_size();
        self.width = Some(size.x);
        self.height = Some(size.y);
        self
    }

    /// Sirf chaudai tay karo (unchai buttons se).
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// Sirf unchai tay karo (chaudai buttons se).
    pub fn height(mut self, px: f32) -> Self {
        self.height = Some(px);
        self
    }

    /// Corner ki gol-ai. Na likho to poori gol capsule.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius.max(0.0));
        self
    }

    /// Har corner alag: top-left, top-right, bottom-right, bottom-left (px).
    pub fn corners(mut self, top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        self.corners = Some([top_left, top_right, bottom_right, bottom_left].map(|v| v.max(0.0)));
        self
    }

    /// Rect mein tasveer (buttons ke peeche): `.image(Image(BG).size(20.0).left(8.0))`. Jagah/size
    /// `Image` ke methods se, jaise `Button` mein. Kai baar likh sakte ho.
    pub fn image(mut self, image: impl Into<Image>) -> Self {
        self.frame = self.frame.image(image);
        self
    }

    /// Rect mein text (buttons ke peeche), jaise `Button` mein: `.text(Label("Title").bold().center())`.
    pub fn text(mut self, label: impl Into<Label>) -> Self {
        self.frame = self.frame.text(label);
        self
    }

    /// Har side ki alag motai: top, right, bottom, left.
    pub fn border_widths(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.frame = self.frame.border_widths(top, right, bottom, left);
        self
    }

    /// Har side ka alag rang: top, right, bottom, left.
    pub fn border_colors(
        mut self,
        top: impl Into<Fill>,
        right: impl Into<Fill>,
        bottom: impl Into<Fill>,
        left: impl Into<Fill>,
    ) -> Self {
        self.frame = self.frame.border_colors(top, right, bottom, left);
        self
    }

    /// Buttons ke charon taraf khali jagah: `(left-right, upar-neeche)`. Default `(0.0, 0.0)`: buttons
    /// kinare tak jate hain aur hover poore hisse par aata hai. `(3.0, 3.0)` se har hover capsule ke andar tairta hai.
    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding = (x.max(0.0), y.max(0.0));
        self
    }

    /// Do buttons ke beech khali jagah (default 0).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Lakeer ka rang (`light(..).dark(..)` bhi chalta hai).
    pub fn separator_color(mut self, color: impl Into<Fill>) -> Self {
        self.separator_color = Some(color.into());
        self
    }

    /// Is Group ko `id` wale bundle mein daalo: jagah kam padne par poora Group `>>` mein simat jata hai
    /// aur wahan buttons upar se neeche khulte hain (dekho `Button::bundle`, `Bundle`).
    pub fn bundle(mut self, id: impl Into<String>) -> Self {
        self.bundle = Some(id.into());
        self
    }

    /// Buttons upar se neeche (default: left se right).
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Andar ke sab buttons ka hover (jin ka apna `.hover(..)` nahi). Options `Button` ke `Hover()` wale hi.
    pub fn hover(mut self, hover: Hover) -> Self {
        self.hover = Some(hover);
        self
    }

    /// Andar ke sab buttons ka pressed (jin ka apna `.pressed(..)` nahi).
    pub fn pressed(mut self, pressed: Hover) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Hover/pressed ka badlav kitni der mein (seconds).
    pub fn transition(mut self, seconds: f32) -> Self {
        self.transition = Some(seconds);
        self
    }

    // ---- rect ka rang-roop (Button ke jaisa) ----

    /// Rect ka rang: `hex(..)`, `rgb(..)`, `rgba(..)`, `light(..).dark(..)`. Na likho to Apple jaisa halka dhusar.
    pub fn fill(mut self, fill: impl Into<Fill>) -> Self {
        self.styled = true;
        self.frame = self.frame.fill(fill);
        self
    }

    /// Rang ki opacity, 0 se 100.
    pub fn fill_opacity(mut self, opacity: impl Into<Opacity>) -> Self {
        self.frame = self.frame.fill_opacity(opacity);
        self
    }

    /// Poore rect ki opacity, 0 se 100.
    pub fn opacity(mut self, percent: u8) -> Self {
        self.frame = self.frame.opacity(percent);
        self
    }

    /// Do rang ka gradient.
    pub fn gradient(mut self, a: Color, b: Color) -> Self {
        self.styled = true;
        self.frame = self.frame.gradient(a, b);
        self
    }

    /// Kai rang ka gradient: `[(rang, percent), ..]`.
    pub fn multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.styled = true;
        self.frame = self.frame.multi_gradient(stops);
        self
    }

    /// Gradient ke do sire: `(x0, x1, y0, y1)` percent mein.
    pub fn points(mut self, x0: f32, x1: f32, y0: f32, y1: f32) -> Self {
        self.frame = self.frame.points(x0, x1, y0, y1);
        self
    }

    /// Gradient chale (seconds mein ek chakkar).
    pub fn animate(mut self, seconds: f32) -> Self {
        self.frame = self.frame.animate(seconds);
        self
    }

    /// Gradient ulta chale.
    pub fn reverse(mut self) -> Self {
        self.frame = self.frame.reverse();
        self
    }

    /// Gradient aage-peeche chale.
    pub fn ping_pong(mut self) -> Self {
        self.frame = self.frame.ping_pong();
        self
    }

    /// Gradient ka mod (percent).
    pub fn turn_at(mut self, percent: f32) -> Self {
        self.frame = self.frame.turn_at(percent);
        self
    }

    /// Kinare ki motai (px).
    pub fn border_width(mut self, px: f32) -> Self {
        self.frame = self.frame.border_width(px);
        self
    }

    /// Kinare ka rang; `light(..).dark(..)` bhi chalta hai.
    pub fn border_color(mut self, color: impl Into<Fill>) -> Self {
        self.frame = self.frame.border_color(color);
        self
    }

    /// Kinara (rang, motai) ek saath.
    pub fn border(mut self, color: impl Into<Fill>, width: f32) -> Self {
        self.frame = self.frame.border_color(color).border_width(width);
        self
    }

    /// Kinare ka do rang ka gradient.
    pub fn border_gradient(mut self, a: Color, b: Color) -> Self {
        self.frame = self.frame.border_gradient(a, b);
        self
    }

    /// Kinare ka kai rang ka gradient.
    pub fn border_multi_gradient(mut self, stops: impl IntoIterator<Item = (Color, f32)>) -> Self {
        self.frame = self.frame.border_multi_gradient(stops);
        self
    }

    /// Kinare ki chamak (px).
    pub fn border_glow(mut self, px: f32) -> Self {
        self.frame = self.frame.border_glow(px);
        self
    }

    /// Kinare ki chamak kahan: andar, bahar ya beech mein.
    pub fn border_glow_at(mut self, at: Glow) -> Self {
        self.frame = self.frame.border_glow_at(at);
        self
    }

    /// Kinare ki chamak kitni tez (0 se 100).
    pub fn border_glow_intensity(mut self, intensity: f32) -> Self {
        self.frame = self.frame.border_glow_intensity(intensity);
        self
    }

    /// Kinare ka gradient chale.
    pub fn border_animate(mut self, seconds: f32) -> Self {
        self.frame = self.frame.border_animate(seconds);
        self
    }

    /// Kinare ka gradient ulta chale.
    pub fn border_reverse(mut self) -> Self {
        self.frame = self.frame.border_reverse();
        self
    }

    /// Kinare ka gradient aage-peeche chale.
    pub fn border_ping_pong(mut self) -> Self {
        self.frame = self.frame.border_ping_pong();
        self
    }

    /// Glass: peeche ka blur + halka rang + patla chamakta kinara. Baaki sab neeche ke methods se.
    pub fn glass(mut self) -> Self {
        self.styled = true;
        self.frame = self.frame.glass();
        self
    }

    /// Glass ka blur, 0 se 100 (default 20); glass bhi chalu kar deta hai.
    pub fn blur(mut self, intensity: u8) -> Self {
        self.styled = true;
        self.frame = self.frame.blur(intensity);
        self
    }

    /// Jaise `.blur(..)`, par Kawase blur: bade blur par Gaussian se sasta, thoda zyada naram.
    pub fn blur_kawase(mut self, intensity: u8) -> Self {
        self.styled = true;
        self.frame = self.frame.blur_kawase(intensity);
        self
    }

    /// Glass ke peeche ke rang kitne gaadhe (default 1.4).
    pub fn saturation(mut self, amount: f32) -> Self {
        self.styled = true;
        self.frame = self.frame.saturation(amount);
        self
    }

    /// Glass ke peeche ki roshni (1.0 = jaisi hai).
    pub fn brightness(mut self, amount: f32) -> Self {
        self.styled = true;
        self.frame = self.frame.brightness(amount);
        self
    }

    /// Glass ke peeche ka halka rang: `.tint(hex("#7f5af0"), 0.2)`.
    pub fn tint(mut self, color: Color, amount: f32) -> Self {
        self.styled = true;
        self.frame = self.frame.tint(color, amount);
        self
    }

    /// Rect ke gird rangeen chamak: `.glow(hex("#7f5af0"), 16.0)`.
    pub fn glow(mut self, color: Color, px: f32) -> Self {
        self.frame = self.frame.glow(color, px);
        self
    }

    /// Chamak kitni tez (0 se 100).
    pub fn glow_intensity(mut self, intensity: f32) -> Self {
        self.frame = self.frame.glow_intensity(intensity);
        self
    }

    /// Parchhai: `.shadow(10.0)` ya `.shadow(drop(0.0, 6.0).blur(18.0).inner(0.0, 2.0))` (dekho `niv_ui_kit::shadow`).
    pub fn shadow(mut self, shadow: impl Into<crate::shadow::ShadowSpec>) -> Self {
        self.frame = self.frame.shadow(shadow);
        self
    }

    /// Parchhai kitni gaadhi, 0 se 100 percent.
    pub fn shadow_opacity(mut self, percent: f32) -> Self {
        self.frame = self.frame.shadow_opacity(percent);
        self
    }

    /// Parchhai ka rang (`"#1c1c1e"` ya `Color`).
    pub fn shadow_color(mut self, color: impl crate::shadow::ShadowColor) -> Self {
        self.frame = self.frame.shadow_color(color);
        self
    }

    /// Group dikhao.
    pub fn show(self, ui: &mut Ui) -> GroupResponse {
        self.show_placed(ui, None, None)
    }

    /// Har button ka size aur poore rect ka size (`ui` mein kuch banaye bina).
    fn plan(&self, ui: &Ui) -> Plan {
        let n = self.buttons.len();
        let (px, py) = self.padding;
        let gap = self.gap;
        // 1. har button ka size. Group ka size diya ho to wahi baant lete hain, warna buttons ka apna;
        //    cross-axis par sab ek barabar.
        // Jis button ki apni size (width aur height dono) likhi hai wo wahi rakhta hai.
        let fixed: Vec<Option<Vec2>> = self.buttons.iter().map(Button::fixed_size).collect();
        let mut measured: Vec<_> = self
            .buttons
            .iter()
            .zip(&fixed)
            .map(|(b, _)| b.measure(ui))
            .collect();
        let natural_cross = measured
            .iter()
            .map(|m| if self.vertical { m.size.x } else { m.size.y })
            .fold(0.0, f32::max);
        let natural_main: f32 = measured.iter().map(|m| if self.vertical { m.size.y } else { m.size.x }).sum::<f32>()
            + gap * n.saturating_sub(1) as f32;
        let (want_main, want_cross, pad_main, pad_cross) = if self.vertical {
            (self.height, self.width, py, px)
        } else {
            (self.width, self.height, px, py)
        };
        let cross = want_cross.map_or(natural_cross, |v| (v - 2.0 * pad_cross).max(0.0));
        let main = want_main.map_or(natural_main, |v| (v - 2.0 * pad_main).max(0.0));
        let fixed_main: f32 = fixed.iter().flatten().map(|s| if self.vertical { s.y } else { s.x }).sum();
        let loose = fixed.iter().filter(|f| f.is_none()).count().max(1) as f32;
        let each = (main - gap * n.saturating_sub(1) as f32 - fixed_main).max(0.0) / loose;
        for (m, f) in measured.iter_mut().zip(&fixed) {
            // `.size(..)` wala button apni hi size rakhta hai (rect ke upar baithta hai, rect ko nahi badalta).
            if self.vertical {
                if f.is_none() {
                    m.size.x = cross;
                }
                if want_main.is_some() && f.is_none() {
                    m.size.y = each;
                }
            } else {
                if f.is_none() {
                    m.size.y = cross;
                }
                if want_main.is_some() && f.is_none() {
                    m.size.x = each;
                }
            }
        }
        let inner = if self.vertical { vec2(cross, main) } else { vec2(main, cross) };
        let outer = inner + vec2(2.0 * px, 2.0 * py);
        Plan { fixed, measured, want_main, main, cross, inner, outer }
    }

    /// `spot` ho to parent ke andar us jagah (`Locate` se), warna normal flow mein.
    fn show_placed(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> GroupResponse {
        let Some(bundle) = self.bundle.clone() else {
            return self.show_unbundled(ui, spot, area);
        };
        // Bundle ka member: jagah ho to wahin, warna `>>` mein (wahan seedha khada hokar).
        let outer = self.plan(ui).outer;
        let mut popup_group = self.clone();
        let lead = popup_group.lead.take();
        popup_group.bundle = None;
        popup_group.vertical = true;
        popup_group.width = None;
        popup_group.height = None;
        bundle_slot(
            ui,
            &bundle,
            spot,
            area,
            outer,
            (Vec2::ZERO, Vec2::ZERO),
            false,
            |ui, _, rect| {
                let mut unbundled = self;
                unbundled.bundle = None;
                let plan = unbundled.plan(ui);
                unbundled.paint_at(ui, rect, plan)
            },
            |popup_ui, lead_wanted| {
                let lead = lead.filter(|_| lead_wanted);
                let has_lead = lead.is_some();
                if let Some(button) = lead {
                    popup_group.part_ids.insert(0, button.id.clone());
                    popup_group.buttons.insert(0, button);
                    popup_group.sep_after.insert(0, false);
                }
                let mut shown = popup_group.show_unbundled(popup_ui, None, None);
                if has_lead && !shown.parts.is_empty() {
                    shown.lead = Some(shown.parts.remove(0));
                }
                shown
            },
        )
    }

    fn show_unbundled(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> GroupResponse {
        let plan = self.plan(ui);
        let (_, rect) = Button::claim(ui, plan.outer, spot, area, (Vec2::ZERO, Vec2::ZERO));
        self.paint_at(ui, rect, plan)
    }

    /// `rect` mein poora Group banao.
    fn paint_at(self, ui: &mut Ui, rect: Rect, plan: Plan) -> GroupResponse {
        let n = self.buttons.len();
        let is_dark_theme = is_dark(ui);
        // Har group ki apni pehchaan: naam diya ho to wahi, warna jagah se.
        let base = match &self.id {
            Some(name) => Id::unique(("niv_group", name.as_str())),
            None => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        };
        let (px, py) = self.padding;
        let gap = self.gap;
        let Plan { fixed, measured, want_main, main, cross, inner, outer } = plan;

        // 2. peeche ka rect: Button wali poori painting (rang, glass, border, glow, saaya ...).
        let max_radius = rect.width().min(rect.height()) / 2.0;
        let radius = self.radius.unwrap_or(max_radius).min(max_radius);
        let corners = self.corners.map_or([radius; 4], |c| c.map(|r| r.min(max_radius)));
        let mut frame = self.frame.clone().size(outer).corners(corners[0], corners[1], corners[2], corners[3]);
        if !self.styled {
            frame = frame.fill(if is_dark_theme { Color::from_rgb(58, 58, 60) } else { Color::from_rgb(229, 229, 234) });
        }
        frame.no_feel = true;
        let frame_measured = frame.measure(ui);
        // `.scale(..hovered())`: the background grows while the pointer is on the group (the buttons do not grow)
        let over = ui.rect_contains_pointer(rect) && ui.is_enabled();
        let t = ui.ctx().animate_bool_with_time(base.with("bg_hover"), over, self.transition.unwrap_or(0.12));
        let grow: f32 = self.scale_states.iter().filter(|s| !s.is_clicked()).map(|s| (s.factor() - 1.0) * t).sum();
        let grow_px: f32 = self.scale_states.iter().filter(|s| !s.is_clicked()).map(|s| s.px() * t).sum();
        let drawn = Rect::from_center_size(rect.center(), (rect.size() * (1.0 + grow) + vec2(2.0 * grow_px, 2.0 * grow_px)).max(Vec2::ZERO));
        let _ = frame.paint(ui, base.with("bg"), drawn, &frame_measured);

        // 3. buttons: rect ke bahar wale corner rect jaise (padding jitne kam), andar ke corner seedhe
        //    (ya padding ho to gol).
        let pad = px.min(py);
        let inner_corners = corners.map(|r| (r - pad).max(0.0));
        let seam = self.seam_radius.unwrap_or(if px > 0.0 || py > 0.0 {
            inner_corners.iter().copied().fold(0.0, f32::max)
        } else {
            0.0
        });
        let mut parts = Vec::with_capacity(n);
        let mut cursor = rect.min + vec2(px, py);
        // Sab buttons ki apni size ho aur rect bada ho: buttons beech mein baithte hain.
        if want_main.is_some() && fixed.iter().all(Option::is_some) {
            let used: f32 = measured.iter().map(|m| if self.vertical { m.size.y } else { m.size.x }).sum::<f32>()
                + gap * n.saturating_sub(1) as f32;
            let extra = ((main - used) / 2.0).max(0.0);
            if self.vertical {
                cursor.y += extra;
            } else {
                cursor.x += extra;
            }
        }
        let mut seams: Vec<f32> = Vec::new();
        for (i, (button, m)) in self.buttons.iter().zip(&measured).enumerate() {
            let first = i == 0;
            let last = i + 1 == n;
            // top-left, top-right, bottom-right, bottom-left
            let [tl, tr, br, bl] = inner_corners;
            let pick = |outer: f32, edge: bool| if edge { outer } else { seam };
            let auto = if self.vertical {
                [pick(tl, first), pick(tr, first), pick(br, last), pick(bl, last)]
            } else {
                [pick(tl, first), pick(tr, last), pick(br, last), pick(bl, first)]
            };
            let radii = self.part_ids.get(i).and_then(Option::as_ref)
                .and_then(|id| self.id_corners.iter().find(|(name, _)| name == id))
                .map_or(auto, |(_, c)| c.map(|r| r.min(max_radius)));
            // cross-axis par chhota button rect ke beech mein
            let slack = if self.vertical { cross - m.size.x } else { cross - m.size.y }.max(0.0) / 2.0;
            let origin = if self.vertical { cursor + vec2(slack, 0.0) } else { cursor + vec2(0.0, slack) };
            let part = Rect::from_min_size(origin, m.size);
            let id = base.with(("part", i));
            let mut button = button.clone();
            button.no_feel = true;
            if button.scale_states.is_empty() {
                // only the "while pressed" states go to the buttons: the pressed one shrinks, the background stays
                button.scale_states = self.scale_states.iter().copied().filter(|s| s.is_clicked()).collect();
            }
            let segment = button.into_segment(radii, self.hover.as_ref(), self.pressed.as_ref(), self.transition);
            parts.push(segment.paint(ui, id, part, m));
            if self.vertical {
                cursor.y += m.size.y + gap;
                seams.push(part.max.y + gap / 2.0);
            } else {
                cursor.x += m.size.x + gap;
                seams.push(part.max.x + gap / 2.0);
            }
        }

        // 4. beech ki lakeer: aadhi unchai ki.
        if (self.all_sep || self.sep_after.iter().any(|s| *s)) && ui.is_rect_visible(rect) {
            let fill: Fill = self.separator_color.unwrap_or_else(|| light(rgba(0, 0, 0, 80)).dark(rgba(255, 255, 255, 110)).into());
            let color = fill.resolve(is_dark_theme);
            for (&at, _) in seams.iter().zip(&self.sep_after).take(n.saturating_sub(1)).filter(|(_, sep)| self.all_sep || **sep) {
                let (from, to) = if self.vertical {
                    let half = self.sep_length.unwrap_or_else(|| (inner.x * 0.5).max(4.0)) / 2.0;
                    (pos2(rect.center().x - half, at), pos2(rect.center().x + half, at))
                } else {
                    let half = self.sep_length.unwrap_or_else(|| (inner.y * 0.5).max(4.0)) / 2.0;
                    (pos2(at, rect.center().y - half), pos2(at, rect.center().y + half))
                };
                ui.painter().line_segment([from, to], Stroke::new(self.sep_width, color));
            }
        }

        let response = ui.interact(rect, base.with("rect"), egui::Sense::hover());
        GroupResponse { response, parts, lead: None }
    }
}

impl BundleItem for GroupResponse {
    fn item_clicked(&self) -> bool {
        self.parts.iter().any(Response::clicked) || self.lead_clicked()
    }
    fn item_rect(&self) -> Rect {
        self.response.rect
    }
    fn hidden(ui: &mut Ui, id: Id, rect: Rect) -> Self {
        GroupResponse { response: <Response as BundleItem>::hidden(ui, id, rect), parts: Vec::new(), lead: None }
    }
}

impl Placeable for Group {
    type Output = GroupResponse;
    fn place(self, ui: &mut Ui, spot: Option<Spot>, area: Option<Rect>) -> GroupResponse {
        self.show_placed(ui, spot, area)
    }
    fn rect(out: &GroupResponse) -> Rect {
        out.response.rect
    }
}


#[cfg(test)]
mod corner_tests {
    use super::*;

    #[test]
    fn button_corners_go_to_button_with_that_id() {
        let g = Group("c")
            .button_corners("b", 0.0, 32.0, 32.0, 0.0)
            .button(Button("a").id("a"))
            .button(Button("b").id("b"));
        assert_eq!(g.part_ids, vec![Some("a".to_owned()), Some("b".to_owned())]);
        assert_eq!(g.id_corners, vec![("b".to_owned(), [0.0, 32.0, 32.0, 0.0])]);
    }
}
