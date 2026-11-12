//! Andar ka rang-saaz: gradient, animation, border. Panel aur Button dono isse chalte hain.

use crate::theme::Color;
use egui::{pos2, vec2, Mesh, Painter, Pos2, Rect, Shape, Ui};
use std::f32::consts::PI;

/// Chalte hue gradient ki setting (`.animate(..)` aur uske saathi).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Anim {
    pub seconds: f32,
    pub reverse: bool,
    pub ping_pong: bool,
    pub turn_at: Option<f32>,
}

/// Rang aur uski jagah (0..=100 percent), line par.
pub(crate) type Stops = Vec<(Color, f32)>;

/// Gradient: rang+jagah, points (x0, x1, y0, y1; 0..=100) aur (optional) animation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Grad {
    pub stops: Stops,
    pub points: (f32, f32, f32, f32),
    pub anim: Option<Anim>,
}

/// Do rangon se stops: `a` 0% par, `b` 100% par.
pub(crate) fn two_stops(a: Color, b: Color) -> Stops {
    vec![(a, 0.0), (b, 100.0)]
}

/// Stops ko jagah ke hisaab se sajao (sort) aur jagah 0..=100 mein rakho. Ek rang ho to dohra do.
pub(crate) fn clean_stops(stops: impl IntoIterator<Item = (Color, f32)>) -> Stops {
    let mut stops: Stops = stops.into_iter().map(|(c, at)| (c, at.clamp(0.0, 100.0))).collect();
    stops.sort_by(|x, y| x.1.total_cmp(&y.1));
    if let [(c, _)] = stops[..] {
        stops = two_stops(c, c);
    }
    stops
}

/// Chalte gradient/border ke liye: stops aadhe raaste mein simat jate hain aur aakhir mein pehla rang
/// wapas aata hai, taki ghoomte waqt jod dikhe nahi.
pub(crate) fn cyclic(stops: &[(Color, f32)]) -> Stops {
    let mut cycle: Stops = stops.iter().map(|&(c, at)| (c, at * 0.5)).collect();
    if let Some(&(first, _)) = stops.first() {
        cycle.push((first, 100.0));
    }
    cycle
}

/// Animation ki progress (0..1 range ke raaste par), time `time` seconds par.
pub(crate) fn anim_progress(time: f64, anim: &Anim) -> f32 {
    let travels = time as f32 / anim.seconds.max(0.01); // kitne poore safar ho chuke
    let progress = if let Some(turn) = anim.turn_at {
        let turn = (turn / 100.0).clamp(0.01, 1.0);
        let phase = (travels % (2.0 * turn)) / turn; // 0..2
        if phase < 1.0 { phase * turn } else { (2.0 - phase) * turn }
    } else if anim.ping_pong {
        let phase = travels % 2.0;
        if phase < 1.0 { phase } else { 2.0 - phase }
    } else {
        travels.fract()
    };
    if anim.reverse { -progress } else { progress }
}

/// `stops` (jagah ke hisaab se sorted) ke beech `t` (0..=1) par rang.
pub(crate) fn color_at(stops: &[(Color, f32)], t: f32) -> Color {
    let at = t.clamp(0.0, 1.0) * 100.0;
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return Color::TRANSPARENT;
    };
    if at <= first.1 {
        return first.0;
    }
    for pair in stops.windows(2) {
        let ((c0, p0), (c1, p1)) = (pair[0], pair[1]);
        if at <= p1 {
            let span = p1 - p0;
            return if span <= f32::EPSILON { c1 } else { c0.lerp_to_gamma(c1, (at - p0) / span) };
        }
    }
    last.0
}

/// Gradient ka rang har point par (`rect` ke hisaab se), time `time` par.
pub(crate) fn gradient_fn(rect: Rect, grad: &Grad, time: f64) -> impl Fn(Pos2) -> Color {
    let (x0, x1, y0, y1) = grad.points;
    let at = move |x: f32, y: f32| rect.min + vec2(rect.width() * x / 100.0, rect.height() * y / 100.0);
    let (start, end) = (at(x0, y0), at(x1, y1));
    let axis = end - start;
    let length_sq = axis.length_sq();
    let (palette, shift) = match grad.anim {
        Some(anim) => (cyclic(&grad.stops), Some(anim_progress(time, &anim))),
        None => (grad.stops.clone(), None),
    };
    move |pos| {
        if length_sq <= f32::EPSILON {
            return palette[0].0; // shuru aur aakhir ek hi point: ek hi rang
        }
        let along = (pos - start).dot(axis) / length_sq;
        match shift {
            Some(shift) => color_at(&palette, (along - shift).rem_euclid(1.0)),
            None => color_at(&palette, along),
        }
    }
}

/// `ui.max_rect()` par gradient (seedha rect, 40x40 mesh).
pub(crate) fn paint_gradient(ui: &Ui, grad: &Grad) {
    let rect = ui.max_rect();
    if grad.anim.is_some() {
        ui.ctx().request_repaint();
    }
    let color_for = gradient_fn(rect, grad, ui.input(|i| i.time));

    const CELLS: usize = 40;
    let mut mesh = Mesh::default();
    for row in 0..=CELLS {
        for col in 0..=CELLS {
            let pos = pos2(
                rect.min.x + rect.width() * col as f32 / CELLS as f32,
                rect.min.y + rect.height() * row as f32 / CELLS as f32,
            );
            mesh.colored_vertex(pos, color_for(pos));
        }
    }
    let stride = (CELLS + 1) as u32;
    for row in 0..CELLS as u32 {
        for col in 0..CELLS as u32 {
            let a = row * stride + col;
            mesh.add_triangle(a, a + 1, a + stride);
            mesh.add_triangle(a + 1, a + stride + 1, a + stride);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
}

/// Har corner par kitne tukde (gol corner ke liye).
const ARC: usize = 8;

/// Gol corners wale rect ka kinara (ghadi ki disha mein, top-left corner se). Radii:
/// top-left, top-right, bottom-right, bottom-left. Hamesha `4 * (ARC + 1)` points.
pub(crate) fn rounded_path(rect: Rect, radii: [f32; 4]) -> Vec<Pos2> {
    let max_r = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let r = radii.map(|v| v.clamp(0.0, max_r));
    let centres = [
        pos2(rect.min.x + r[0], rect.min.y + r[0]),
        pos2(rect.max.x - r[1], rect.min.y + r[1]),
        pos2(rect.max.x - r[2], rect.max.y - r[2]),
        pos2(rect.min.x + r[3], rect.max.y - r[3]),
    ];
    let start = [PI, 1.5 * PI, 0.0, 0.5 * PI];
    let mut out = Vec::with_capacity(4 * (ARC + 1));
    for k in 0..4 {
        for s in 0..=ARC {
            let a = start[k] + (s as f32 / ARC as f32) * 0.5 * PI;
            out.push(centres[k] + vec2(a.cos(), a.sin()) * r[k]);
        }
    }
    out
}

/// Poori shape (gol corners ke saath) ko gradient se bharo.
pub(crate) fn paint_shape(painter: &Painter, rect: Rect, radii: [f32; 4], color: &dyn Fn(Pos2) -> Color) {
    let path = rounded_path(rect, radii);
    let centre = rect.center();
    const RINGS: u32 = 12;
    let n = path.len() as u32;
    let mut mesh = Mesh::default();
    for k in 0..=RINGS {
        let s = k as f32 / RINGS as f32;
        for p in &path {
            let pos = centre + (*p - centre) * s;
            mesh.colored_vertex(pos, color(pos));
        }
    }
    for k in 0..RINGS {
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (k * n + i, k * n + j);
            mesh.add_triangle(a, b, a + n);
            mesh.add_triangle(b, b + n, a + n);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// egui jaisa patla stroke: 1 px se patli line 1 px chaudi banti hai aur halki (uski motai jitni) dikhti hai.
/// Wapas: (motai, rang ko kitna dikhana hai 0..=1).
pub(crate) fn thin(width: f32) -> (f32, f32) {
    if width > 0.0 && width < 1.0 {
        (1.0, width)
    } else {
        (width, 1.0)
    }
}

/// Border ki motai (top, right, bottom, left) ko rect ke andar rakho.
fn inner_rect(rect: Rect, widths: [f32; 4]) -> Rect {
    let w = widths.map(|v| v.max(0.0));
    let (hx, hy) = (rect.width() / 2.0, rect.height() / 2.0);
    let (top, bottom) = (w[0].min(hy), w[2].min(hy));
    let (right, left) = (w[1].min(hx), w[3].min(hx));
    Rect::from_min_max(rect.min + vec2(left, top), rect.max - vec2(right, bottom))
}

/// Andar ke kinare ki radii: bahar ki radii se border ki motai ghata kar.
fn inner_radii(radii: [f32; 4], widths: [f32; 4]) -> [f32; 4] {
    [
        radii[0] - widths[3].max(widths[0]),
        radii[1] - widths[0].max(widths[1]),
        radii[2] - widths[1].max(widths[2]),
        radii[3] - widths[2].max(widths[3]),
    ]
    .map(|v| v.max(0.0))
}

/// Do kinaron ke beech patti, har vertex par `tint(u)` (u: 0..1, kinare ke gird) se rang.
fn ring(painter: &Painter, outer: &[Pos2], inner: &[Pos2], tint: &dyn Fn(f32) -> (Color, Color)) {
    let n = outer.len();
    if n < 3 {
        return;
    }
    let mut cumulative = vec![0.0_f32; n + 1];
    for i in 0..n {
        cumulative[i + 1] = cumulative[i] + (outer[(i + 1) % n] - outer[i]).length();
    }
    let total = cumulative[n];
    if total <= f32::EPSILON {
        return;
    }
    let mut mesh = Mesh::default();
    for (i, along) in cumulative.iter().enumerate() {
        let (out_color, in_color) = tint(along / total);
        mesh.colored_vertex(outer[i % n], out_color);
        mesh.colored_vertex(inner[i % n], in_color);
    }
    for i in 0..n as u32 {
        let a = 2 * i;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
}

/// Border ka glow border ke kis taraf phaile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Glow {
    /// Border se andar ki taraf (default).
    #[default]
    Inside,
    /// Border ki line ke dono taraf barabar (aadha andar, aadha bahar).
    Middle,
    /// Border se bahar ki taraf (parent ke area ke bahar nahi jata, wahin kat jata hai).
    Outside,
}

/// Border glow ki setting: kitna phaile (px), kahan, aur kitna tez (1..=100).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlowCfg {
    pub px: f32,
    pub at: Glow,
    pub intensity: f32,
}

impl GlowCfg {
    /// Default tezi (percent).
    pub(crate) const DEFAULT_INTENSITY: f32 = 45.0;
}

/// Gradient border (aur uska glow), kinare ke gird; `anim` ho to rang ghoomta hai.
/// `widths`: top, right, bottom, left. `radii`: top-left, top-right, bottom-right, bottom-left.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_border(
    painter: &Painter,
    rect: Rect,
    radii: [f32; 4],
    widths: [f32; 4],
    stops: &[(Color, f32)],
    glow: GlowCfg,
    anim: Option<Anim>,
    time: f64,
) {
    let shift = match anim {
        Some(anim) => {
            painter.ctx().request_repaint();
            anim_progress(time, &anim)
        }
        None => 0.0,
    };
    let palette = cyclic(stops);
    // Patli (1 px se kam) motai: 1 px par halka rang, jaise egui ka stroke.
    let strength = widths.iter().copied().filter(|w| *w > 0.0).map(|w| thin(w).1).fold(1.0, f32::min);
    let widths = widths.map(|w| thin(w).0);
    let base = |u: f32| color_at(&palette, (u - shift).rem_euclid(1.0)).gamma_multiply(strength);

    let inner = inner_rect(rect, widths);
    let inner_r = inner_radii(radii, widths);
    let limit = (inner.width().min(inner.height()) / 2.0).max(0.0);
    let tone = glow.intensity.clamp(1.0, 100.0) / 100.0;
    let px = glow.px.max(0.0);
    let (inward, outward) = match glow.at {
        Glow::Inside => (px, 0.0),
        Glow::Middle => (px / 2.0, px / 2.0),
        Glow::Outside => (0.0, px),
    };
    let inward = inward.min(limit);
    if inward > 0.0 {
        let deeper = inner.shrink(inward);
        let deeper_r = inner_radii(inner_r, [inward; 4]);
        ring(
            painter,
            &rounded_path(inner, inner_r),
            &rounded_path(deeper, deeper_r),
            &|u| (base(u).gamma_multiply(tone), Color::TRANSPARENT),
        );
    }
    if outward > 0.0 {
        let expanded = rect.expand(outward);
        let expanded_r = radii.map(|r| r + outward);
        ring(
            painter,
            &rounded_path(expanded, expanded_r),
            &rounded_path(rect, radii),
            &|u| (Color::TRANSPARENT, base(u).gamma_multiply(tone)),
        );
    }
    if widths.iter().any(|w| *w > 0.0) {
        ring(painter, &rounded_path(rect, radii), &rounded_path(inner, inner_r), &|u| (base(u), base(u)));
    }
}

/// Border, har side ka alag (seedha) rang. `colors` aur `widths`: top, right, bottom, left.
pub(crate) fn paint_sides(painter: &Painter, rect: Rect, radii: [f32; 4], widths: [f32; 4], colors: [Color; 4]) {
    let colors = [0, 1, 2, 3].map(|i| colors[i].gamma_multiply(thin(widths[i]).1));
    let widths = widths.map(|w| thin(w).0);
    let outer = rounded_path(rect, radii);
    let inner_box = inner_rect(rect, widths);
    let inner = rounded_path(inner_box, inner_radii(radii, widths));
    let n = outer.len();
    let mut mesh = Mesh::default();
    for j in 0..n {
        let next = (j + 1) % n;
        let (corner, step) = (j / (ARC + 1), j % (ARC + 1));
        // Corner ka pehla aadha pichhli side ka rang, doosra aadha agli side ka; seedhi patti apni side ka.
        let side = if step == ARC || step >= ARC / 2 { corner } else { (corner + 3) % 4 };
        let color = colors[side];
        let base = mesh.vertices.len() as u32;
        for p in [outer[j], inner[j], outer[next], inner[next]] {
            mesh.colored_vertex(p, color);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
    painter.add(Shape::mesh(mesh));
}
