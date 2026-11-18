use egui::{epaint::Mesh, Color32, Rect, Shape};

#[derive(Clone, Copy, Debug)]
pub struct TextBacking {
    pub opacity: f32,
    pub feather: f32,
    pub whole_line: bool,
    pub invert: bool,
    /// Custom colors instead of the pane background (or text, inverted).
    pub palette: Option<BackingPalette>,
    /// Draw text on the default background in this color instead of the
    /// theme's (ignored while inverted).
    pub text_color: Option<Color32>,
    /// Also recolor text that has its own (ANSI) color.
    pub recolor_all: bool,
    /// WCAG contrast ratio every glyph must reach against what is drawn
    /// behind it (1 = off). Text below it is moved toward white or black.
    pub min_contrast: f32,
}

impl Default for TextBacking {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            feather: 0.0,
            whole_line: false,
            invert: false,
            palette: None,
            text_color: None,
            recolor_all: false,
            min_contrast: 1.0,
        }
    }
}

/// Largest feather in points.
pub const MAX_FEATHER: f32 = 30.0;

/// sRGB channel to linear light, as WCAG 2 defines relative luminance.
fn linear_channel(value: u8) -> f32 {
    static TABLE: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let c = index as f32 / 255.0;
            *slot = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        table
    })[value as usize]
}

/// WCAG 2 relative luminance (0 black .. 1 white).
pub fn relative_luminance(color: Color32) -> f32 {
    0.2126 * linear_channel(color.r())
        + 0.7152 * linear_channel(color.g())
        + 0.0722 * linear_channel(color.b())
}

/// WCAG 2 contrast ratio, 1 (same) .. 21 (black on white).
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `fg` moved toward white or black just far enough to reach `target`
/// against `bg` (like xterm.js `minimumContrastRatio`). Keeps the hue as
/// long as possible; returns the better extreme when the target cannot be
/// reached at all.
pub fn ensure_contrast(fg: Color32, bg: Color32, target: f32) -> Color32 {
    let fg = Color32::from_rgb(fg.r(), fg.g(), fg.b());
    if !(target > 1.0) || contrast_ratio(fg, bg) >= target {
        return fg;
    }
    let lighter = relative_luminance(fg) >= relative_luminance(bg);
    let (first, second) = if lighter {
        (Color32::WHITE, Color32::BLACK)
    } else {
        (Color32::BLACK, Color32::WHITE)
    };
    for toward in [first, second] {
        if contrast_ratio(toward, bg) < target {
            continue;
        }
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..10 {
            let mid = (low + high) * 0.5;
            if contrast_ratio(lerp(fg, toward, mid), bg) >= target {
                high = mid;
            } else {
                low = mid;
            }
        }
        return lerp(fg, toward, high);
    }
    if contrast_ratio(Color32::WHITE, bg) >= contrast_ratio(Color32::BLACK, bg)
    {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// The color a glyph at `pos` sits on: the backing (`run` color or the
/// palette color there) blended at `opacity` over `base`. Rings keep the
/// run color under the text itself.
pub(crate) fn backdrop_at(
    options: &TextBacking,
    run: Color32,
    base: Color32,
    pos: egui::Pos2,
    row: usize,
    bounds: Rect,
) -> Color32 {
    let opacity = options.opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return base;
    }
    let backing =
        match options.palette {
            Some(palette) => match palette.style {
                BackingStyle::Rings => run,
                BackingStyle::Lines => {
                    let colors = palette.colors();
                    colors[row % colors.len()]
                },
                BackingStyle::Horizontal => palette
                    .at((pos.x - bounds.left()) / bounds.width().max(1.0)),
                BackingStyle::Vertical => palette
                    .at((pos.y - bounds.top()) / bounds.height().max(1.0)),
            },
            None => run,
        };
    lerp(base, backing, opacity)
}

/// How custom backing colors are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackingStyle {
    /// A gradient from the first color (top of the pane) to the last (bottom).
    #[default]
    Vertical,
    /// A gradient from the first color (left) to the last (right).
    Horizontal,
    /// Each text row takes the next color, cycling.
    Lines,
    /// Outline rings around the text (first color innermost) on the usual
    /// backing color.
    Rings,
}

/// Up to eight custom backing colors. `Copy`, so the widget stays cheap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackingPalette {
    pub colors: [Color32; BackingPalette::MAX],
    pub count: usize,
    pub style: BackingStyle,
    /// Width of each ring in points (Rings only).
    pub ring_width: f32,
    /// How the rings are drawn (Rings only).
    pub rings: RingStyle,
}

/// Options for `BackingStyle::Rings`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingStyle {
    /// 0 = hard bands; 1 = each ring fades linearly into the next one, so
    /// the rings form one gradient that follows the text outline.
    pub blend: f32,
    /// How many times the color list repeats outward (1..=4).
    pub repeat: u8,
    /// 0..0.9: how much narrower the outermost ring is than the first.
    pub taper: f32,
    /// Points of plain backing between the text and the first ring.
    pub gap: f32,
    /// Round the ring corners instead of square ones.
    pub round: bool,
    /// Last color innermost.
    pub reverse: bool,
    /// Rings the colors have moved outward (animation); 0 = still.
    pub phase: f32,
    /// Points over which the text's own backing color fades into the
    /// rings; 0 = a hard edge around the text panel.
    pub fade: f32,
    /// Follow the shapes of the letters instead of a box around them.
    pub letters: bool,
}

impl Default for RingStyle {
    fn default() -> Self {
        Self {
            blend: 1.0,
            repeat: 1,
            taper: 0.0,
            gap: 0.0,
            round: false,
            reverse: false,
            phase: 0.0,
            fade: 0.0,
            letters: false,
        }
    }
}

impl BackingPalette {
    pub const MAX: usize = 8;

    /// `None` when `colors` is empty; extra colors are ignored.
    pub fn new(
        colors: &[Color32],
        style: BackingStyle,
        ring_width: f32,
    ) -> Option<Self> {
        if colors.is_empty() {
            return None;
        }
        let mut palette = Self {
            colors: [Color32::BLACK; Self::MAX],
            count: colors.len().min(Self::MAX),
            style,
            ring_width: if ring_width.is_finite() {
                ring_width.clamp(1.0, 24.0)
            } else {
                3.0
            },
            rings: RingStyle::default(),
        };
        palette.colors[..palette.count]
            .copy_from_slice(&colors[..palette.count]);
        Some(palette)
    }

    pub fn with_rings(mut self, rings: RingStyle) -> Self {
        self.rings = rings;
        self
    }

    fn colors(&self) -> &[Color32] {
        &self.colors[..self.count.clamp(1, Self::MAX)]
    }

    /// The gradient color at `t` (0..1) through every color in order.
    fn at(&self, t: f32) -> Color32 {
        let colors = self.colors();
        if colors.len() == 1 || !t.is_finite() {
            return colors[0];
        }
        let scaled = t.clamp(0.0, 1.0) * (colors.len() - 1) as f32;
        let index = (scaled.floor() as usize).min(colors.len() - 2);
        lerp(colors[index], colors[index + 1], scaled - index as f32)
    }
}

pub(crate) fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix =
        |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

fn with_alpha(color: Color32, opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// A feathered quad: solid inside `rect`, transparent at `rect.expand(feather)`.
/// `fill` picks the solid color at each inner vertex position.
fn feathered_rect(
    mesh: &mut Mesh,
    rect: Rect,
    feather: f32,
    fill: impl Fn(egui::Pos2) -> Color32,
) {
    let outer = rect.expand(feather);
    let xs = [outer.left(), rect.left(), rect.right(), outer.right()];
    let ys = [outer.top(), rect.top(), rect.bottom(), outer.bottom()];
    let base = mesh.vertices.len() as u32;
    for (y, &py) in ys.iter().enumerate() {
        for (x, &px) in xs.iter().enumerate() {
            let edge = x == 0 || x == 3 || y == 0 || y == 3;
            let pos = egui::pos2(px, py);
            mesh.colored_vertex(
                pos,
                if edge {
                    Color32::TRANSPARENT
                } else {
                    fill(pos)
                },
            );
        }
    }
    for y in 0..3 {
        for x in 0..3 {
            let a = base + y * 4 + x;
            mesh.add_triangle(a, a + 1, a + 4);
            mesh.add_triangle(a + 1, a + 5, a + 4);
        }
    }
}

/// Most rings drawn around one run (colors x repeats).
const MAX_RINGS: usize = 32;
/// Points per rounded corner.
const CORNER_POINTS: usize = 4;

/// The outline `distance` points outside `rect`, clockwise from the top
/// left. Always the same number of points for one `round` value, so two
/// outlines can be joined point by point.
fn outline(rect: Rect, distance: f32, round: bool, out: &mut Vec<egui::Pos2>) {
    if !round {
        let grown = rect.expand(distance);
        out.extend([
            grown.left_top(),
            grown.right_top(),
            grown.right_bottom(),
            grown.left_bottom(),
        ]);
        return;
    }
    let corners = [
        (rect.left_top(), 2.0),
        (rect.right_top(), 3.0),
        (rect.right_bottom(), 0.0),
        (rect.left_bottom(), 1.0),
    ];
    for (corner, quarter) in corners {
        for step in 0..CORNER_POINTS {
            let angle = (quarter + step as f32 / (CORNER_POINTS - 1) as f32)
                * std::f32::consts::FRAC_PI_2;
            out.push(corner + distance * egui::vec2(angle.cos(), angle.sin()));
        }
    }
}

/// Distance from the text, color, and share of the text's own backing
/// color (1 = all backing, 0 = all ring) at each place the rings change,
/// innermost first. Colors between two stops are a straight blend.
pub(crate) fn ring_stops(palette: &BackingPalette) -> Vec<(f32, Color32, f32)> {
    let style = palette.rings;
    let mut colors = palette.colors().to_vec();
    if style.reverse {
        colors.reverse();
    }
    let count = colors.len() as i64;
    let rings =
        (colors.len() * usize::from(style.repeat.clamp(1, 4))).min(MAX_RINGS);
    let finite = |value: f32, low: f32, high: f32| {
        if value.is_finite() {
            value.clamp(low, high)
        } else {
            low
        }
    };
    let taper = finite(style.taper, 0.0, 0.9);
    let mut edges = vec![finite(style.gap, 0.0, 24.0)];
    for ring in 0..rings {
        let outward = ring as f32 / (rings.max(2) - 1) as f32;
        let width = (palette.ring_width * (1.0 - taper * outward)).max(0.5);
        edges.push(edges[ring] + width);
    }
    let narrowest = edges[rings] - edges[rings - 1];
    // Half the blend zone, in rings. Hard bands keep half a point of
    // blending so their edges are not jagged.
    let half = (finite(style.blend, 0.0, 1.0) * 0.5)
        .max(0.5 / narrowest)
        .min(0.5);
    // Moving colors wrap around; still ones end on the first and last color.
    let phase = if style.phase.is_finite() {
        style.phase.rem_euclid(count as f32)
    } else {
        0.0
    };
    let moving = phase != 0.0;
    let pick = |index: i64| {
        let index = if moving {
            index.rem_euclid(count)
        } else {
            index.clamp(0, rings as i64 - 1) % count
        };
        colors[index as usize]
    };
    let color = |at: f32| {
        let shifted = at - phase;
        let index = shifted.floor();
        let part = shifted - index;
        let index = index as i64;
        if part < half {
            lerp(pick(index - 1), pick(index), 0.5 + part / (2.0 * half))
        } else if part > 1.0 - half {
            lerp(
                pick(index),
                pick(index + 1),
                (part - (1.0 - half)) / (2.0 * half),
            )
        } else {
            pick(index)
        }
    };
    let distance = |at: f32| {
        let ring = (at.floor() as usize).min(rings - 1);
        edges[ring] + (at - ring as f32) * (edges[ring + 1] - edges[ring])
    };
    // The text's backing color fades out over the first `fade` points.
    let fade = finite(style.fade, 0.0, 48.0).min(edges[rings] - edges[0]);
    let backing = |at: f32| {
        if fade > 0.0 {
            (1.0 - (distance(at) - edges[0]) / fade).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    // Ring positions where the width, the color blend or the fade changes.
    let mut places: Vec<f32> = (0..=rings).map(|ring| ring as f32).collect();
    if let Some(ring) =
        (0..rings).find(|ring| edges[ring + 1] >= edges[0] + fade)
    {
        places.push(
            ring as f32
                + (edges[0] + fade - edges[ring])
                    / (edges[ring + 1] - edges[ring]),
        );
    }
    for boundary in -1..=rings as i64 + count {
        for side in [-half, half] {
            let at = boundary as f32 + phase + side;
            if at > 0.0 && at < rings as f32 {
                places.push(at);
            }
        }
    }
    places.sort_by(f32::total_cmp);
    places.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
    // Where two runs overlap, the later one wins inside a step, so blends
    // are cut into steps too thin to show a seam.
    let step = (edges[rings] / 24.0).max(2.0);
    let stop = |at: f32| (distance(at), color(at), backing(at));
    let mut stops = vec![stop(places[0])];
    for pair in places.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let parts = if color(from) == color(to) && backing(from) == backing(to)
        {
            1
        } else {
            ((distance(to) - distance(from)) / step).ceil().max(1.0) as usize
        };
        for part in 1..=parts {
            let at = from + (to - from) * part as f32 / parts as f32;
            stops.push(stop(at));
        }
    }
    stops
}

/// Outline rings around every run: one gradient that follows the text.
/// Below full opacity the bands of neighboring rows show through each other
/// where they overlap; that layered look is wanted, so it is not hidden.
fn ring_mesh(
    mesh: &mut Mesh,
    runs: &[(Rect, Color32)],
    palette: &BackingPalette,
    opacity: f32,
    feather: f32,
) {
    let round = palette.rings.round;
    let mut stops = ring_stops(palette);
    let (reach, ..) = stops[stops.len() - 1];
    stops.push((reach + feather.max(0.75), Color32::TRANSPARENT, 0.0));
    let points = if round { 4 * CORNER_POINTS } else { 4 };
    let mut ring = Vec::with_capacity(points);
    let first = mesh.vertices.len() as u32;
    for (rect, backing) in runs {
        for (distance, color, share) in &stops {
            let color = if *share > 0.0 {
                lerp(*color, *backing, *share)
            } else {
                *color
            };
            // The last stop is the transparent outer edge.
            let color = if color.a() == 0 {
                color
            } else {
                with_alpha(color, opacity)
            };
            ring.clear();
            outline(*rect, *distance, round, &mut ring);
            for pos in &ring {
                mesh.colored_vertex(*pos, color);
            }
        }
    }
    // Outermost band first for every run, then the next one in: where runs
    // overlap, the band nearest to any text is the one that shows.
    let points = points as u32;
    let per_run = stops.len() as u32 * points;
    for band in (0..stops.len() as u32 - 1).rev() {
        for run in 0..runs.len() as u32 {
            let inner = first + run * per_run + band * points;
            let outer = inner + points;
            for point in 0..points {
                let next = (point + 1) % points;
                mesh.add_triangle(inner + point, inner + next, outer + next);
                mesh.add_triangle(inner + point, outer + next, outer + point);
            }
        }
    }
    // The text sits on its usual backing color, so it stays readable
    // whatever the ring colors are.
    let (gap, ..) = stops[0];
    for (rect, color) in runs {
        let color = with_alpha(*color, opacity);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(rect.center(), color);
        for (distance, color) in
            [(gap, color), (gap + 0.75, with_alpha(color, 0.0))]
        {
            ring.clear();
            outline(*rect, distance, round, &mut ring);
            for pos in &ring {
                mesh.colored_vertex(*pos, color);
            }
        }
        for point in 0..points {
            let next = (point + 1) % points;
            let (inner, outer) = (base + 1, base + 1 + points);
            mesh.add_triangle(base, inner + point, inner + next);
            mesh.add_triangle(inner + point, inner + next, outer + next);
            mesh.add_triangle(inner + point, outer + next, outer + point);
        }
    }
}

/// Join neighboring glyphs before feathering, so letters never acquire dark seams.
fn runs(
    cells: impl Iterator<Item = (Rect, Color32)>,
    bounds: Rect,
    gap: f32,
    whole_line: bool,
) -> Vec<(Rect, Color32)> {
    let mut runs: Vec<(Rect, Color32)> = Vec::new();
    for (mut rect, color) in cells {
        if !rect.intersects(bounds) {
            continue;
        }
        if whole_line {
            rect.min.x = bounds.left();
            rect.max.x = bounds.right();
        }
        if let Some((last, last_color)) = runs.last_mut() {
            if *last_color == color
                && last.top() == rect.top()
                && rect.left() <= last.right() + gap
            {
                *last = last.union(rect);
                continue;
            }
        }
        runs.push((rect, color));
    }
    runs
}

pub(crate) fn shapes(
    cells: impl Iterator<Item = (Rect, Color32)>,
    bounds: Rect,
    cell_width: f32,
    options: TextBacking,
) -> Vec<Shape> {
    if !options.opacity.is_finite() || options.opacity <= 0.0 {
        return Vec::new();
    }
    let feather = if options.feather.is_finite() {
        options.feather.clamp(0.0, MAX_FEATHER)
    } else {
        0.0
    };
    let opacity = options.opacity.min(1.0);
    let runs = runs(cells, bounds, cell_width * 1.01, options.whole_line);
    let mut mesh = Mesh::default();
    match options.palette {
        None => {
            for (rect, color) in runs {
                let color = with_alpha(color, opacity);
                feathered_rect(&mut mesh, rect, feather, |_| color);
            }
        },
        Some(palette) if palette.style == BackingStyle::Rings => {
            ring_mesh(&mut mesh, &runs, &palette, opacity, feather);
        },
        Some(palette) => {
            let height = bounds.height().max(1.0);
            let width = bounds.width().max(1.0);
            for (rect, _) in runs {
                match palette.style {
                    BackingStyle::Lines => {
                        let row = ((rect.top() - bounds.top())
                            / rect.height().max(1.0))
                        .round()
                        .max(0.0) as usize;
                        let colors = palette.colors();
                        let color =
                            with_alpha(colors[row % colors.len()], opacity);
                        feathered_rect(&mut mesh, rect, feather, |_| color);
                    },
                    BackingStyle::Horizontal => {
                        feathered_rect(&mut mesh, rect, feather, |pos| {
                            with_alpha(
                                palette.at((pos.x - bounds.left()) / width),
                                opacity,
                            )
                        });
                    },
                    _ => {
                        feathered_rect(&mut mesh, rect, feather, |pos| {
                            with_alpha(
                                palette.at((pos.y - bounds.top()) / height),
                                opacity,
                            )
                        });
                    },
                }
            }
        },
    }
    if mesh.is_empty() {
        Vec::new()
    } else {
        vec![Shape::mesh(mesh)]
    }
}

/// One feathered backing behind `rect` (a label outside the terminal grid),
/// drawn the same way as terminal text backing.
pub fn backing_shapes(
    rect: Rect,
    color: Color32,
    opacity: f32,
    feather: f32,
) -> Vec<Shape> {
    let options = TextBacking {
        opacity,
        feather,
        ..TextBacking::default()
    };
    shapes(
        std::iter::once((rect, color)),
        rect.expand(feather.max(0.0) + 1.0),
        rect.width().max(1.0),
        options,
    )
}

/// Records a copy to the clipboard at `pos`, for the app's "Copied" toast.
pub fn note_copied(ctx: &egui::Context, pos: Option<egui::Pos2>) {
    let at = ctx.input(|input| input.time);
    let pos = pos.or_else(|| ctx.pointer_latest_pos());
    ctx.data_mut(|data| {
        data.insert_temp(copied_id(), (at, pos));
    });
}

/// The time and place of the latest copy recorded by `note_copied`.
pub fn last_copied(ctx: &egui::Context) -> Option<(f64, Option<egui::Pos2>)> {
    ctx.data(|data| data.get_temp(copied_id()))
}

fn copied_id() -> egui::Id {
    egui::Id::new("egui_term-copied-to-clipboard")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f32, y: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(8.0, 16.0))
    }
    #[test]
    fn text_backing_joins_spaces_but_preserves_blank_rows_and_large_gaps() {
        let bounds =
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(200.0, 100.0));
        let cells = [
            rect(0.0, 0.0),
            rect(16.0, 0.0),
            rect(80.0, 0.0),
            rect(0.0, 32.0),
        ];
        assert_eq!(
            runs(
                cells.into_iter().map(|r| (r, Color32::BLACK)),
                bounds,
                8.1,
                false
            )
            .len(),
            3
        );
        let lines = runs(
            cells.into_iter().map(|r| (r, Color32::BLACK)),
            bounds,
            8.1,
            true,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0.width(), 200.0);
    }

    #[test]
    fn contrast_matches_wcag_and_ensure_contrast_reaches_the_target() {
        assert!(
            (contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs()
                < 0.01
        );
        assert!(
            (contrast_ratio(Color32::RED, Color32::RED) - 1.0).abs() < 0.001
        );
        // #767676 on white is the classic 4.54:1 AA boundary.
        let grey = Color32::from_rgb(0x76, 0x76, 0x76);
        assert!((contrast_ratio(grey, Color32::WHITE) - 4.54).abs() < 0.02);
        let bg = Color32::from_rgb(40, 40, 60);
        for fg in [
            Color32::from_rgb(60, 60, 90),
            Color32::from_rgb(200, 30, 30),
            Color32::from_rgb(20, 20, 20),
        ] {
            let fixed = ensure_contrast(fg, bg, 4.5);
            assert!(contrast_ratio(fixed, bg) >= 4.5, "{fg:?} -> {fixed:?}");
        }
        // Already readable text is left alone; ratio 1 turns it off.
        assert_eq!(ensure_contrast(Color32::WHITE, bg, 4.5), Color32::WHITE);
        let dim = Color32::from_rgb(50, 50, 70);
        assert_eq!(ensure_contrast(dim, bg, 1.0), dim);
        // A mid grey cannot reach 21: it gets the better extreme.
        let mid = Color32::from_rgb(128, 128, 128);
        assert_eq!(ensure_contrast(mid, mid, 21.0), Color32::BLACK);
    }

    #[test]
    fn text_backing_feathers_to_transparent_and_zero_opacity_skips_work() {
        let bounds = rect(0.0, 0.0).expand(30.0);
        let mut options = TextBacking {
            opacity: 0.75,
            feather: 6.0,
            whole_line: false,
            invert: false,
            palette: None,
            ..Default::default()
        };
        let result = shapes(
            [(rect(0.0, 0.0), Color32::BLACK)].into_iter(),
            bounds,
            8.0,
            options,
        );
        let Shape::Mesh(mesh) = &result[0] else {
            panic!("expected mesh")
        };
        assert_eq!(mesh.vertices[0].color.a(), 0);
        assert_eq!(mesh.vertices[5].color.a(), 191);
        assert_eq!(mesh.vertices[0].pos, egui::pos2(-6.0, -6.0));
        options.opacity = 0.0;
        assert!(shapes(
            [(rect(0.0, 0.0), Color32::BLACK)].into_iter(),
            bounds,
            8.0,
            options
        )
        .is_empty());
    }

    #[test]
    fn palette_gradients_and_rings_draw_every_color() {
        let red = Color32::from_rgb(255, 0, 0);
        let blue = Color32::from_rgb(0, 0, 255);
        let palette =
            BackingPalette::new(&[red, blue], BackingStyle::Vertical, 3.0)
                .unwrap();
        assert_eq!(palette.at(0.0), red);
        assert_eq!(palette.at(1.0), blue);
        assert_eq!(palette.at(0.5), Color32::from_rgb(128, 0, 128));
        assert!(BackingPalette::new(&[], BackingStyle::Rings, 3.0).is_none());

        let bounds = rect(0.0, 0.0).expand(60.0);
        let draw = |rings: RingStyle| {
            let mut palette = palette.with_rings(rings);
            palette.style = BackingStyle::Rings;
            let options = TextBacking {
                opacity: 1.0,
                feather: 4.0,
                whole_line: false,
                invert: false,
                palette: Some(palette),
                ..Default::default()
            };
            let mut result = shapes(
                [(rect(0.0, 0.0), Color32::BLACK)].into_iter(),
                bounds,
                8.0,
                options,
            );
            let Some(Shape::Mesh(mesh)) = result.pop() else {
                panic!("expected a mesh");
            };
            mesh
        };
        let color_at = |mesh: &Mesh, x: f32| {
            mesh.vertices
                .iter()
                .find(|vertex| (vertex.pos.x - x).abs() < 1e-3)
                .map(|vertex| vertex.color)
        };
        let purple = Color32::from_rgb(128, 0, 128);

        // Blended: red at the text, blue at the outside, and the shade
        // between them where the two rings meet (3 points out).
        let soft = draw(RingStyle::default());
        assert_eq!(color_at(&soft, 0.0), Some(red));
        assert_eq!(color_at(&soft, -3.0), Some(purple));
        assert_eq!(color_at(&soft, -6.0), Some(blue));
        // The outer edge feathers out; the text core keeps its own color.
        assert_eq!(color_at(&soft, -10.0), Some(Color32::TRANSPARENT));
        assert!(soft.vertices.iter().any(|v| v.color == Color32::BLACK));
        assert!(soft
            .indices
            .iter()
            .all(|i| (*i as usize) < soft.vertices.len()));

        // Hard bands: plain colors up to half a point from the boundary.
        let hard = draw(RingStyle {
            blend: 0.0,
            ..RingStyle::default()
        });
        assert_eq!(color_at(&hard, -2.5), Some(red));
        assert_eq!(color_at(&hard, -3.5), Some(blue));

        // Options: repeats and the gap move the outer edge; reverse swaps
        // the ends; rounded corners use more points per outline.
        let wide = draw(RingStyle {
            repeat: 2,
            gap: 2.0,
            reverse: true,
            ..RingStyle::default()
        });
        assert_eq!(color_at(&wide, -2.0), Some(blue));
        assert_eq!(color_at(&wide, -14.0), Some(red));
        let tapered = draw(RingStyle {
            taper: 0.5,
            ..RingStyle::default()
        });
        assert_eq!(color_at(&tapered, -4.5), Some(blue));
        let round = draw(RingStyle {
            round: true,
            ..RingStyle::default()
        });
        assert!(round.vertices.len() > soft.vertices.len() * 3);
        assert!(round
            .indices
            .iter()
            .all(|i| (*i as usize) < round.vertices.len()));

        // Fade: the text's own color (black here) at the text, mostly
        // red 2.5 points out, all ring color from 4 points on.
        let faded = draw(RingStyle {
            fade: 4.0,
            blend: 0.0,
            ..RingStyle::default()
        });
        assert_eq!(color_at(&faded, 0.0), Some(Color32::BLACK));
        assert_eq!(color_at(&faded, -2.5), Some(Color32::from_rgb(159, 0, 0)));
        assert_eq!(color_at(&faded, -4.0), Some(blue));

        // Moving colors wrap around: half a ring out, the boundary shade
        // sits on the text edge.
        let moved = draw(RingStyle {
            phase: 1.0,
            ..RingStyle::default()
        });
        assert_eq!(color_at(&moved, 0.0), Some(purple));
        assert_eq!(color_at(&moved, -1.5), Some(blue));
        for bad in [f32::NAN, f32::INFINITY] {
            draw(RingStyle {
                fade: bad,
                blend: bad,
                taper: bad,
                gap: bad,
                phase: bad,
                ..RingStyle::default()
            });
        }
    }
}
