//! Outline rings that follow the letters themselves, not their cells.
//!
//! The glyphs on screen are stamped into a mask from egui's font atlas, a
//! distance map is grown from that mask, and each pixel takes the ring
//! color for its distance from the nearest ink. The map is rebuilt only
//! when the text on screen changes; moving colors only recolor it.

use crate::text_backing::{lerp, ring_stops, BackingPalette};
use egui::{
    epaint::{text::Fonts, Mesh},
    Color32, ColorImage, Context, FontId, Id, Pos2, Rect, Shape, TextureHandle,
    TextureOptions, Vec2,
};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Distance units per map pixel (5-7-11 chamfer weights).
const UNIT: u16 = 5;
const FAR: u16 = u16::MAX / 2;
/// Largest map; bigger panes use a coarser map, stretched smoothly.
const MAX_PIXELS: f32 = 900_000.0;

/// One glyph on screen: where it is drawn (points) and where its picture
/// sits in the font atlas (texels).
pub(crate) struct Stamp {
    min: Pos2,
    size: Vec2,
    uv_min: [u16; 2],
    uv_max: [u16; 2],
    /// The backing color this text would normally sit on.
    backing: Color32,
}

/// Adds the glyphs of `text`, drawn centered below `center_top` the way
/// the terminal draws a cell.
pub(crate) fn stamps_for(
    fonts: &Fonts,
    center_top: Pos2,
    text: String,
    font: FontId,
    backing: Color32,
    out: &mut Vec<Stamp>,
) {
    let galley = fonts.layout_no_wrap(text, font, Color32::WHITE);
    let origin = center_top - egui::vec2(galley.size().x / 2.0, 0.0);
    for row in &galley.rows {
        for glyph in &row.glyphs {
            let uv = glyph.uv_rect;
            if uv.is_nothing() {
                continue;
            }
            out.push(Stamp {
                min: origin + glyph.pos.to_vec2() + uv.offset,
                size: uv.size,
                uv_min: uv.min,
                uv_max: uv.max,
                backing,
            });
        }
    }
}

#[derive(Default)]
struct Field {
    key: u64,
    width: usize,
    height: usize,
    /// Map pixels per point.
    scale: f32,
    distance: Vec<u16>,
    /// Index into `backings` of the nearest glyph's backing color.
    nearest: Vec<u8>,
    backings: Vec<Color32>,
}

#[derive(Default)]
struct Cache {
    field: Field,
    painted: u64,
    texture: Option<TextureHandle>,
}

fn hash_f32(hasher: &mut impl Hasher, values: &[f32]) {
    for value in values {
        value.to_bits().hash(hasher);
    }
}

/// The 5-7-11 chamfer neighbors already visited in a top-to-bottom pass.
const NEIGHBORS: [(isize, isize, u16); 8] = [
    (-1, 0, 5),
    (-1, -1, 7),
    (0, -1, 5),
    (1, -1, 7),
    (-2, -1, 11),
    (2, -1, 11),
    (-1, -2, 11),
    (1, -2, 11),
];

impl Field {
    fn build(
        key: u64,
        fonts: &Fonts,
        stamps: &[Stamp],
        bounds: Rect,
        scale: f32,
        reach: f32,
    ) -> Self {
        let width = ((bounds.width() * scale).ceil() as usize).max(1);
        let height = ((bounds.height() * scale).ceil() as usize).max(1);
        let mut field = Self {
            key,
            width,
            height,
            scale,
            distance: vec![FAR; width * height],
            nearest: vec![0; width * height],
            backings: Vec::new(),
        };
        // Rows and columns that hold ink, so empty areas cost nothing.
        let (mut left, mut top, mut right, mut bottom) =
            (width, height, 0usize, 0usize);
        {
            let atlas = fonts.texture_atlas();
            let atlas = atlas.lock();
            let image = atlas.image();
            for stamp in stamps {
                if stamp.size.x <= 0.0 || stamp.size.y <= 0.0 {
                    continue;
                }
                let backing = match field
                    .backings
                    .iter()
                    .position(|color| *color == stamp.backing)
                {
                    Some(index) => index as u8,
                    None if field.backings.len() < 255 => {
                        field.backings.push(stamp.backing);
                        (field.backings.len() - 1) as u8
                    },
                    None => 0,
                };
                let local = stamp.min - bounds.min;
                let pixel = |value: f32, limit: usize| {
                    (value.max(0.0) as usize).min(limit)
                };
                let x0 = pixel((local.x * scale).floor(), width);
                let y0 = pixel((local.y * scale).floor(), height);
                let x1 =
                    pixel(((local.x + stamp.size.x) * scale).ceil(), width);
                let y1 =
                    pixel(((local.y + stamp.size.y) * scale).ceil(), height);
                let texels = [
                    f32::from(stamp.uv_max[0].saturating_sub(stamp.uv_min[0])),
                    f32::from(stamp.uv_max[1].saturating_sub(stamp.uv_min[1])),
                ];
                for y in y0..y1 {
                    let v = ((y as f32 + 0.5) / scale - local.y) / stamp.size.y;
                    if !(0.0..1.0).contains(&v) {
                        continue;
                    }
                    let ty =
                        usize::from(stamp.uv_min[1]) + (v * texels[1]) as usize;
                    for x in x0..x1 {
                        let u =
                            ((x as f32 + 0.5) / scale - local.x) / stamp.size.x;
                        if !(0.0..1.0).contains(&u) {
                            continue;
                        }
                        let tx = usize::from(stamp.uv_min[0])
                            + (u * texels[0]) as usize;
                        let Some(coverage) =
                            image.pixels.get(ty * image.size[0] + tx)
                        else {
                            continue;
                        };
                        if *coverage < 0.05 {
                            continue;
                        }
                        // Half-covered edge pixels start half a pixel
                        // out, which keeps the outlines smooth.
                        let start = ((1.0 - coverage.min(1.0))
                            * f32::from(UNIT))
                        .round() as u16;
                        let index = y * width + x;
                        if start < field.distance[index] {
                            field.distance[index] = start;
                            field.nearest[index] = backing;
                            left = left.min(x);
                            right = right.max(x);
                            top = top.min(y);
                            bottom = bottom.max(y);
                        }
                    }
                }
            }
        }
        if left > right {
            return field;
        }
        let margin = (reach * scale).ceil() as usize + 2;
        let x0 = left.saturating_sub(margin);
        let y0 = top.saturating_sub(margin);
        let x1 = (right + margin + 1).min(width);
        let y1 = (bottom + margin + 1).min(height);
        let limit =
            (margin as u32 * u32::from(UNIT)).min(u32::from(FAR)) as u16;
        // Down then up: every pixel takes the shortest way to any ink.
        let distance = &mut field.distance;
        let nearest = &mut field.nearest;
        for direction in [1isize, -1] {
            let offsets = NEIGHBORS.map(|(dx, dy, cost)| {
                ((dy * width as isize + dx) * direction, cost)
            });
            for row in 0..y1 - y0 {
                let y = if direction == 1 {
                    y0 + row
                } else {
                    y1 - 1 - row
                };
                let inner_row = y >= y0 + 2 && y + 2 < y1;
                for column in 0..x1 - x0 {
                    let x = if direction == 1 {
                        x0 + column
                    } else {
                        x1 - 1 - column
                    };
                    let index = y * width + x;
                    let mut best = distance[index];
                    if best == 0 {
                        continue;
                    }
                    let mut from = usize::MAX;
                    if inner_row && x >= x0 + 2 && x + 2 < x1 {
                        // Every neighbor is inside: no edge checks.
                        for (offset, cost) in offsets {
                            let other = (index as isize + offset) as usize;
                            let candidate = distance[other] + cost;
                            if candidate < best {
                                best = candidate;
                                from = other;
                            }
                        }
                    } else {
                        for (dx, dy, cost) in NEIGHBORS {
                            let nx = x as isize + dx * direction;
                            let ny = y as isize + dy * direction;
                            if nx < x0 as isize
                                || ny < y0 as isize
                                || nx >= x1 as isize
                                || ny >= y1 as isize
                            {
                                continue;
                            }
                            let other = ny as usize * width + nx as usize;
                            let candidate = distance[other] + cost;
                            if candidate < best {
                                best = candidate;
                                from = other;
                            }
                        }
                    }
                    if from != usize::MAX && best <= limit {
                        distance[index] = best;
                        nearest[index] = nearest[from];
                    }
                }
            }
        }
        field
    }

    /// One color per distance unit, for text on `backing`.
    fn colors(
        &self,
        stops: &[(f32, Color32, f32)],
        backing: Color32,
        opacity: f32,
        feather: f32,
    ) -> Vec<Color32> {
        let (gap, ..) = stops[0];
        let (reach, last, _) = stops[stops.len() - 1];
        let per_point = self.scale * f32::from(UNIT);
        let count = ((reach + feather) * per_point).ceil() as usize + 1;
        let mut stop = 0;
        (0..count)
            .map(|unit| {
                let at = unit as f32 / per_point;
                let (color, alpha) = if at < gap {
                    (backing, 1.0)
                } else if at >= reach {
                    (last, 1.0 - (at - reach) / feather)
                } else {
                    while stop + 2 < stops.len() && stops[stop + 1].0 <= at {
                        stop += 1;
                    }
                    let (from, from_color, from_share) = stops[stop];
                    let (to, to_color, to_share) = stops[stop + 1];
                    let part = if to > from {
                        ((at - from) / (to - from)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let share = from_share + (to_share - from_share) * part;
                    (
                        lerp(lerp(from_color, to_color, part), backing, share),
                        1.0,
                    )
                };
                Color32::from_rgba_unmultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    (alpha.clamp(0.0, 1.0) * opacity * 255.0).round() as u8,
                )
            })
            .collect()
    }

    fn image(
        &self,
        palette: &BackingPalette,
        opacity: f32,
        feather: f32,
    ) -> ColorImage {
        let stops = ring_stops(palette);
        let tables: Vec<Vec<Color32>> = self
            .backings
            .iter()
            .map(|backing| self.colors(&stops, *backing, opacity, feather))
            .collect();
        let pixels = self
            .distance
            .iter()
            .zip(&self.nearest)
            .map(|(distance, nearest)| {
                tables
                    .get(usize::from(*nearest))
                    .and_then(|table| table.get(usize::from(*distance)))
                    .copied()
                    .unwrap_or(Color32::TRANSPARENT)
            })
            .collect();
        ColorImage {
            size: [self.width, self.height],
            pixels,
        }
    }
}

/// The ring picture for the glyphs in `stamps`, as one textured quad over
/// `bounds`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn shapes(
    ctx: &Context,
    id: Id,
    fonts: &Fonts,
    stamps: &[Stamp],
    bounds: Rect,
    palette: &BackingPalette,
    opacity: f32,
    feather: f32,
) -> Vec<Shape> {
    let area = bounds.width() * bounds.height();
    if stamps.is_empty()
        || !opacity.is_finite()
        || opacity <= 0.0
        || !area.is_finite()
        || area < 1.0
    {
        return Vec::new();
    }
    let opacity = opacity.min(1.0);
    let feather = if feather.is_finite() {
        feather.clamp(0.75, 20.0)
    } else {
        0.75
    };
    let stops = ring_stops(palette);
    let reach = stops[stops.len() - 1].0 + feather;
    let scale = ctx.pixels_per_point().min((MAX_PIXELS / area).sqrt());

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(
        &mut hasher,
        &[
            bounds.min.x,
            bounds.min.y,
            bounds.width(),
            bounds.height(),
            scale,
            reach,
        ],
    );
    for stamp in stamps {
        hash_f32(
            &mut hasher,
            &[stamp.min.x, stamp.min.y, stamp.size.x, stamp.size.y],
        );
        (stamp.uv_min, stamp.uv_max, stamp.backing).hash(&mut hasher);
    }
    let field_key = hasher.finish();
    hash_f32(&mut hasher, &[opacity, feather, palette.ring_width]);
    let rings = palette.rings;
    hash_f32(
        &mut hasher,
        &[rings.blend, rings.taper, rings.gap, rings.phase, rings.fade],
    );
    (rings.repeat, rings.reverse, palette.count, palette.colors)
        .hash(&mut hasher);
    let paint_key = hasher.finish();

    let cache = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Arc<Mutex<Cache>>>(
            id.with("letter-rings"),
        )
        .clone()
    });
    let Ok(mut cache) = cache.lock() else {
        return Vec::new();
    };
    if cache.field.key != field_key || cache.field.distance.is_empty() {
        cache.field =
            Field::build(field_key, fonts, stamps, bounds, scale, reach);
        cache.painted = !paint_key;
    }
    if cache.painted != paint_key || cache.texture.is_none() {
        let image = cache.field.image(palette, opacity, feather);
        match cache.texture.as_mut() {
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => {
                cache.texture = Some(ctx.load_texture(
                    "egui_term-letter-rings",
                    image,
                    TextureOptions::LINEAR,
                ));
            },
        }
        cache.painted = paint_key;
    }
    let Some(texture) = cache.texture.as_ref() else {
        return Vec::new();
    };
    let field = &cache.field;
    let mut mesh = Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(
        Rect::from_min_size(
            bounds.min,
            egui::vec2(field.width as f32, field.height as f32) / field.scale,
        ),
        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    vec![Shape::mesh(mesh)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_backing::{BackingStyle, RingStyle};

    fn fonts() -> Fonts {
        let ctx = Context::default();
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        ctx.fonts(|fonts| fonts.clone())
    }

    fn palette() -> BackingPalette {
        let mut palette = BackingPalette::new(
            &[Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 0, 255)],
            BackingStyle::Rings,
            3.0,
        )
        .unwrap()
        .with_rings(RingStyle {
            blend: 0.0,
            letters: true,
            ..RingStyle::default()
        });
        palette.style = BackingStyle::Rings;
        palette
    }

    #[test]
    fn rings_follow_the_ink_of_the_letters() {
        let fonts = fonts();
        let bounds =
            Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, 60.0));
        let mut stamps = Vec::new();
        // "L": ink on the left and along the bottom, none at the top right.
        stamps_for(
            &fonts,
            bounds.center_top() + egui::vec2(0.0, 20.0),
            "L".into(),
            FontId::monospace(20.0),
            Color32::BLACK,
            &mut stamps,
        );
        assert_eq!(stamps.len(), 1);
        let glyph = Rect::from_min_size(stamps[0].min, stamps[0].size);
        let field = Field::build(1, &fonts, &stamps, bounds, 1.0, 7.0);
        let at = |pos: Pos2| {
            let local = pos - bounds.min;
            field.distance[local.y as usize * field.width + local.x as usize]
        };
        // Ink at the foot of the L; the empty top right corner of its cell
        // is further from ink than a point just left of the stem.
        let foot = glyph.left_bottom() + egui::vec2(2.0, -1.0);
        assert!(at(foot) <= UNIT, "{}", at(foot));
        let corner = glyph.right_top() + egui::vec2(-1.0, 1.0);
        let beside = glyph.left_center() - egui::vec2(2.0, 0.0);
        assert!(at(corner) > at(beside), "{} {}", at(corner), at(beside));
        assert!(at(beside) > 0 && at(beside) < 4 * UNIT);
        assert_eq!(at(bounds.min + egui::vec2(1.0, 1.0)), FAR);

        // Colors: red next to the ink, blue further out, nothing far away.
        let image = field.image(&palette(), 1.0, 1.0);
        let color = |distance: u16| {
            let index =
                field.distance.iter().position(|d| *d == distance).unwrap();
            image.pixels[index]
        };
        assert_eq!(color(UNIT), Color32::from_rgb(255, 0, 0));
        assert_eq!(color(4 * UNIT + 2), Color32::from_rgb(0, 0, 255));
        assert_eq!(color(FAR), Color32::TRANSPARENT);
    }

    /// `cargo test -p egui_term --release --lib full_pane -- --ignored
    /// --nocapture` prints how long a full pane takes.
    #[test]
    #[ignore]
    fn full_pane_timing() {
        let fonts = fonts();
        let bounds = Rect::from_min_size(Pos2::ZERO, egui::vec2(1340.0, 770.0));
        let mut stamps = Vec::new();
        for row in 0..48 {
            for column in 0..160 {
                stamps_for(
                    &fonts,
                    egui::pos2(column as f32 * 8.3 + 4.0, row as f32 * 16.0),
                    char::from(b'A' + ((row + column) % 26) as u8).to_string(),
                    FontId::monospace(14.0),
                    Color32::BLACK,
                    &mut stamps,
                );
            }
        }
        let started = std::time::Instant::now();
        let field = Field::build(1, &fonts, &stamps, bounds, 1.0, 20.0);
        let built = started.elapsed();
        let image = field.image(&palette(), 1.0, 1.0);
        eprintln!(
            "{} glyphs, {}x{}: map {:?}, colors {:?}",
            stamps.len(),
            field.width,
            field.height,
            built,
            started.elapsed() - built
        );
        assert_eq!(image.pixels.len(), field.width * field.height);
    }
}
