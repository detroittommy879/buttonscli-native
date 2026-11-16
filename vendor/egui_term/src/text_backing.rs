use egui::{epaint::Mesh, Color32, Rect, Shape};

#[derive(Clone, Copy, Debug)]
pub struct TextBacking {
    pub opacity: f32,
    pub feather: f32,
    pub whole_line: bool,
    pub invert: bool,
    /// Custom colors instead of the pane background (or text, inverted).
    pub palette: Option<BackingPalette>,
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
    /// Outline rings around the text, first color innermost.
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
        };
        palette.colors[..palette.count]
            .copy_from_slice(&colors[..palette.count]);
        Some(palette)
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

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
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
        options.feather.clamp(0.0, 20.0)
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
            // Outermost ring first for every run, then the next one in, so
            // inner colors always cover the outer rings of neighboring rows.
            let colors = palette.colors();
            for ring in (0..colors.len()).rev() {
                let color = with_alpha(colors[ring], opacity);
                let outermost = ring + 1 == colors.len();
                let edge = if outermost { feather.max(0.75) } else { 0.75 };
                for (rect, _) in &runs {
                    let grown = rect.expand(palette.ring_width * ring as f32);
                    feathered_rect(&mut mesh, grown, edge, |_| color);
                }
            }
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
        whole_line: false,
        invert: false,
        palette: None,
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
    fn text_backing_feathers_to_transparent_and_zero_opacity_skips_work() {
        let bounds = rect(0.0, 0.0).expand(30.0);
        let mut options = TextBacking {
            opacity: 0.75,
            feather: 6.0,
            whole_line: false,
            invert: false,
            palette: None,
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
        let mut rings = palette;
        rings.style = BackingStyle::Rings;
        let options = TextBacking {
            opacity: 1.0,
            feather: 4.0,
            whole_line: false,
            invert: false,
            palette: Some(rings),
        };
        let result = shapes(
            [(rect(0.0, 0.0), Color32::BLACK)].into_iter(),
            bounds,
            8.0,
            options,
        );
        let Shape::Mesh(mesh) = &result[0] else {
            panic!("expected a mesh");
        };
        // One feathered quad (16 vertices) per ring; outer ring drawn first.
        assert_eq!(mesh.vertices.len(), 32);
        assert_eq!(mesh.vertices[5].color, blue);
        assert_eq!(mesh.vertices[16 + 5].color, red);
        // The outer ring reaches ring_width beyond the text.
        assert_eq!(mesh.vertices[5].pos, egui::pos2(-3.0, -3.0));
    }
}
