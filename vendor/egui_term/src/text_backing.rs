use egui::{epaint::Mesh, Color32, Rect, Shape};

#[derive(Clone, Copy, Debug)]
pub struct TextBacking {
    pub opacity: f32,
    pub feather: f32,
    pub whole_line: bool,
}

/// Join neighboring glyphs before feathering, so letters never acquire dark seams.
fn runs(
    cells: impl Iterator<Item = Rect>,
    bounds: Rect,
    gap: f32,
    whole_line: bool,
) -> Vec<Rect> {
    let mut runs: Vec<Rect> = Vec::new();
    for mut rect in cells {
        if !rect.intersects(bounds) {
            continue;
        }
        if whole_line {
            rect.min.x = bounds.left();
            rect.max.x = bounds.right();
        }
        if let Some(last) = runs.last_mut() {
            if last.top() == rect.top() && rect.left() <= last.right() + gap {
                *last = last.union(rect);
                continue;
            }
        }
        runs.push(rect);
    }
    runs
}

pub(crate) fn shapes(
    cells: impl Iterator<Item = Rect>,
    bounds: Rect,
    cell_width: f32,
    color: Color32,
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
    let color = Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (options.opacity.min(1.0) * 255.0).round() as u8,
    );
    let mut mesh = Mesh::default();
    for rect in runs(cells, bounds, cell_width * 1.01, options.whole_line) {
        let outer = rect.expand(feather);
        let xs = [outer.left(), rect.left(), rect.right(), outer.right()];
        let ys = [outer.top(), rect.top(), rect.bottom(), outer.bottom()];
        let base = mesh.vertices.len() as u32;
        for (y, &py) in ys.iter().enumerate() {
            for (x, &px) in xs.iter().enumerate() {
                let edge = x == 0 || x == 3 || y == 0 || y == 3;
                mesh.colored_vertex(
                    egui::pos2(px, py),
                    if edge { Color32::TRANSPARENT } else { color },
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
    if mesh.is_empty() {
        Vec::new()
    } else {
        vec![Shape::mesh(mesh)]
    }
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
        assert_eq!(runs(cells.into_iter(), bounds, 8.1, false).len(), 3);
        let lines = runs(cells.into_iter(), bounds, 8.1, true);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].width(), 200.0);
    }
    #[test]
    fn text_backing_feathers_to_transparent_and_zero_opacity_skips_work() {
        let bounds = rect(0.0, 0.0).expand(30.0);
        let mut options = TextBacking {
            opacity: 0.75,
            feather: 6.0,
            whole_line: false,
        };
        let result = shapes(
            [rect(0.0, 0.0)].into_iter(),
            bounds,
            8.0,
            Color32::BLACK,
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
            [rect(0.0, 0.0)].into_iter(),
            bounds,
            8.0,
            Color32::BLACK,
            options
        )
        .is_empty());
    }
}
