//! Pure viewport planning for live terminal sessions. IDs are stable PTY IDs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutMode {
    Single,
    Columns,
    Rows,
    Grid,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutPlan {
    pub ids: Vec<u64>,
    pub rows: usize,
    pub columns: usize,
    pub hidden: usize,
    pub window_start: usize,
}

const DIVIDER_GAP: f32 = 10.0;

pub(crate) fn minimum_for_font(size: f32) -> Bounds {
    // A review target of 40 columns by 8 rows, including modest pane padding.
    Bounds {
        width: 320.0_f32.max(size * 0.62 * 40.0 + 16.0),
        height: 160.0_f32.max(size * 1.25 * 8.0 + 16.0),
    }
}

#[cfg(test)]
pub(crate) fn plan(
    mode: LayoutMode,
    ordered_ids: &[u64],
    focused_id: u64,
    previous_start: usize,
    bounds: Bounds,
    minimum: Bounds,
) -> LayoutPlan {
    plan_with_grid_columns(
        mode,
        ordered_ids,
        focused_id,
        previous_start,
        bounds,
        minimum,
        None,
    )
}

pub(crate) fn plan_with_grid_columns(
    mode: LayoutMode,
    ordered_ids: &[u64],
    focused_id: u64,
    previous_start: usize,
    bounds: Bounds,
    minimum: Bounds,
    requested_grid_columns: Option<usize>,
) -> LayoutPlan {
    if ordered_ids.is_empty() {
        return LayoutPlan {
            ids: Vec::new(),
            rows: 0,
            columns: 0,
            hidden: 0,
            window_start: 0,
        };
    }
    let max_columns = ((bounds.width.max(0.0) + DIVIDER_GAP) / (minimum.width + DIVIDER_GAP))
        .floor()
        .max(1.0) as usize;
    let max_rows = ((bounds.height.max(0.0) + DIVIDER_GAP) / (minimum.height + DIVIDER_GAP))
        .floor()
        .max(1.0) as usize;
    let capacity = match mode {
        LayoutMode::Single => 1,
        LayoutMode::Grid if requested_grid_columns.is_some() => requested_grid_columns
            .unwrap_or(1)
            .clamp(1, max_columns)
            .saturating_mul(max_rows)
            .max(1),
        _ => max_columns.saturating_mul(max_rows).max(1),
    };
    let count = ordered_ids.len().min(capacity);
    let maximum_start = ordered_ids.len() - count;
    let mut start = previous_start.min(maximum_start);
    if let Some(focus_position) = ordered_ids.iter().position(|id| *id == focused_id) {
        if focus_position < start {
            start = focus_position;
        }
        if focus_position >= start + count {
            start = focus_position + 1 - count;
        }
    }
    let ids = ordered_ids[start..start + count].to_vec();
    let (rows, columns) = match mode {
        LayoutMode::Single => (1, 1),
        LayoutMode::Columns => {
            let columns = count.min(max_columns);
            (count.div_ceil(columns), columns)
        }
        LayoutMode::Rows => {
            let rows = count.min(max_rows);
            (rows, count.div_ceil(rows))
        }
        LayoutMode::Grid => requested_grid_columns
            .map(|columns| {
                let columns = count.min(columns.clamp(1, max_columns));
                (count.div_ceil(columns), columns)
            })
            .unwrap_or_else(|| best_grid(count, max_rows, max_columns, bounds, minimum)),
    };
    LayoutPlan {
        ids,
        rows,
        columns,
        hidden: ordered_ids.len() - count,
        window_start: start,
    }
}

fn best_grid(
    count: usize,
    max_rows: usize,
    max_columns: usize,
    bounds: Bounds,
    minimum: Bounds,
) -> (usize, usize) {
    let mut best = (1, count);
    let mut best_score = f32::NEG_INFINITY;
    for rows in 1..=max_rows.min(count) {
        let columns = count.div_ceil(rows);
        if columns > max_columns {
            continue;
        }
        let leaf_width = (bounds.width - DIVIDER_GAP * (columns - 1) as f32) / columns as f32;
        let leaf_height = (bounds.height - DIVIDER_GAP * (rows - 1) as f32) / rows as f32;
        let score = (leaf_width / minimum.width).min(leaf_height / minimum.height);
        if score > best_score {
            best = (rows, columns);
            best_score = score;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(
        mode: LayoutMode,
        ids: &[u64],
        focused: u64,
        start: usize,
        bounds: Bounds,
    ) -> LayoutPlan {
        super::plan(
            mode,
            ids,
            focused,
            start,
            bounds,
            Bounds {
                width: 320.0,
                height: 160.0,
            },
        )
    }

    #[test]
    fn two_to_ten_panes_fit_or_hide_without_crushing_cells() {
        for count in 2..=10 {
            let ids: Vec<u64> = (100..100 + count as u64).collect();
            for mode in [LayoutMode::Columns, LayoutMode::Rows, LayoutMode::Grid] {
                let wide = plan(
                    mode,
                    &ids,
                    ids[0],
                    0,
                    Bounds {
                        width: 1280.0,
                        height: 820.0,
                    },
                );
                assert_eq!(wide.ids, ids);
                assert_eq!(wide.hidden, 0);
                let narrow = plan(
                    mode,
                    &ids,
                    ids[0],
                    0,
                    Bounds {
                        width: 760.0,
                        height: 480.0,
                    },
                );
                assert_eq!(narrow.ids.len(), count.min(4));
                assert_eq!(narrow.hidden, count.saturating_sub(4));
                assert!(narrow.columns <= 2 && narrow.rows <= 2);
            }
        }
    }

    #[test]
    fn column_row_and_grid_order_wrap_differently() {
        let ids: Vec<u64> = (1..=6).collect();
        let bounds = Bounds {
            width: 900.0,
            height: 600.0,
        };
        let columns = plan(LayoutMode::Columns, &ids, 1, 0, bounds);
        let rows = plan(LayoutMode::Rows, &ids, 1, 0, bounds);
        let grid = plan(LayoutMode::Grid, &ids, 1, 0, bounds);
        assert_eq!((columns.rows, columns.columns), (3, 2));
        assert_eq!((rows.rows, rows.columns), (3, 2));
        assert_eq!((grid.rows, grid.columns), (3, 2));
        assert_eq!(columns.ids, ids);
    }

    #[test]
    fn focus_window_and_resize_back_preserve_session_order() {
        let ids: Vec<u64> = (1..=10).collect();
        let small = Bounds {
            width: 760.0,
            height: 480.0,
        };
        let first = plan(LayoutMode::Grid, &ids, 1, 0, small);
        assert_eq!(first.ids, vec![1, 2, 3, 4]);
        let shifted = plan(LayoutMode::Grid, &ids, 8, first.window_start, small);
        assert_eq!(shifted.ids, vec![5, 6, 7, 8]);
        let stable = plan(LayoutMode::Grid, &ids, 6, shifted.window_start, small);
        assert_eq!(stable.ids, shifted.ids);
        let wide = plan(
            LayoutMode::Grid,
            &ids,
            6,
            stable.window_start,
            Bounds {
                width: 1280.0,
                height: 820.0,
            },
        );
        assert_eq!(wide.ids, ids);
        let small_again = plan(LayoutMode::Grid, &ids, 6, wide.window_start, small);
        assert!(small_again.ids.contains(&6));
        assert_eq!(small_again.ids.len(), 4);
    }

    #[test]
    fn one_tiny_viewport_keeps_focused_session() {
        let ids = [10, 20, 30];
        let tiny = Bounds {
            width: 200.0,
            height: 100.0,
        };
        let plan = plan(LayoutMode::Grid, &ids, 30, 0, tiny);
        assert_eq!(plan.ids, vec![30]);
        assert_eq!(plan.hidden, 2);
    }

    #[test]
    fn larger_terminal_font_reduces_visible_capacity() {
        let ids: Vec<u64> = (1..=10).collect();
        let bounds = Bounds {
            width: 1280.0,
            height: 820.0,
        };
        let normal = super::plan(LayoutMode::Grid, &ids, 1, 0, bounds, minimum_for_font(14.0));
        let large = super::plan(LayoutMode::Grid, &ids, 1, 0, bounds, minimum_for_font(32.0));
        assert_eq!(normal.ids.len(), 10);
        assert!(large.ids.len() < normal.ids.len());
        assert_eq!(large.ids[0], 1);
    }
}
