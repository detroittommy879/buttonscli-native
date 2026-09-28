//! Scrollbar geometry for the terminal's real retained grid history.
use egui_term::ScrollbackState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Thumb {
    pub top: f32,
    pub height: f32,
}

pub(crate) fn thumb(state: ScrollbackState, track_height: f32) -> Option<Thumb> {
    if !state.available() || track_height <= 0.0 {
        return None;
    }
    let total = state
        .history_lines
        .saturating_add(state.viewport_lines)
        .max(1);
    let height = (track_height * state.viewport_lines as f32 / total as f32)
        .clamp(24.0_f32.min(track_height), track_height);
    let travel = track_height - height;
    let fraction_from_top =
        1.0 - state.display_offset.min(state.history_lines) as f32 / state.history_lines as f32;
    Some(Thumb {
        top: travel * fraction_from_top,
        height,
    })
}

pub(crate) fn offset_for_pointer(
    state: ScrollbackState,
    track_height: f32,
    pointer_y: f32,
    grab_y: f32,
) -> usize {
    let Some(thumb) = thumb(state, track_height) else {
        return 0;
    };
    let travel = track_height - thumb.height;
    if travel <= 0.0 {
        return 0;
    }
    let top = (pointer_y - grab_y).clamp(0.0, travel);
    ((1.0 - top / travel) * state.history_lines as f32).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(history: usize, viewport: usize, offset: usize) -> ScrollbackState {
        ScrollbackState {
            history_lines: history,
            viewport_lines: viewport,
            display_offset: offset,
            mouse_reporting: false,
            alternate_screen: false,
        }
    }

    #[test]
    fn thumb_tracks_output_resize_and_truncation() {
        assert!(thumb(state(0, 24, 0), 200.0).is_none());
        let initial = thumb(state(100, 24, 0), 200.0).unwrap();
        assert!(initial.top > 0.0);
        assert!(thumb(state(200, 24, 0), 200.0).unwrap().height < initial.height);
        assert!(thumb(state(100, 40, 0), 200.0).unwrap().height > initial.height);
        assert_eq!(thumb(state(100, 24, 100), 200.0).unwrap().top, 0.0);
        assert_eq!(thumb(state(50, 24, 90), 200.0).unwrap().top, 0.0);
    }

    #[test]
    fn dragging_to_bottom_and_top_maps_to_grid_offsets() {
        let state = state(100, 24, 50);
        let thumb = thumb(state, 200.0).unwrap();
        assert_eq!(
            offset_for_pointer(state, 200.0, 200.0, thumb.height / 2.0),
            0
        );
        assert_eq!(
            offset_for_pointer(state, 200.0, 0.0, thumb.height / 2.0),
            100
        );
    }

    #[test]
    fn alternate_screen_and_mouse_reporting_disable_track() {
        let mut state = state(100, 24, 0);
        state.alternate_screen = true;
        assert!(thumb(state, 200.0).is_none());
        state.alternate_screen = false;
        state.mouse_reporting = true;
        assert!(thumb(state, 200.0).is_none());
    }
}
