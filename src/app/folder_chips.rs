//! Experimental: folder buttons inside the terminal. After `ls` or `dir`,
//! each folder name on screen is drawn as a button over its own cells;
//! clicking one types `cd "name"` into that pane. Detection is the left
//! panel's (`dock_nav::listed_folders`), so it reads only the screen.
//!
//! Cost: the screen is scanned once per output change (cached per pane by
//! output sequence, scroll position and size), not every frame.

use super::*;

/// A folder name on screen: visible row, first column, width in cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderSpan {
    pub row: usize,
    pub column: usize,
    pub width: usize,
    pub name: String,
}

/// Where each listed folder appears in the rows after the listing.
pub(super) fn folder_spans(rows: &[(String, Vec<bool>)]) -> Vec<FolderSpan> {
    let names = dock_nav::listed_folders(rows);
    if names.is_empty() {
        return Vec::new();
    }
    let start = dock_nav::listing_start(rows);
    let mut spans = Vec::new();
    for (row, (text, colored)) in rows.iter().enumerate().skip(start) {
        let chars: Vec<char> = text.chars().collect();
        for name in &names {
            let needle: Vec<char> = name.chars().collect();
            let mut from = 0;
            while from + needle.len() <= chars.len() {
                let Some(offset) = chars[from..]
                    .windows(needle.len())
                    .position(|window| window == needle.as_slice())
                else {
                    break;
                };
                let column = from + offset;
                let end = column + needle.len();
                let before_ok = column == 0 || chars[column - 1].is_whitespace();
                let after_ok = end == chars.len()
                    || chars[end].is_whitespace()
                    || matches!(chars[end], '/' | '\\');
                // With colors, only the colored run is the name (a file can
                // share it: `src` and `src.txt`).
                let colored_ok =
                    !colored.contains(&true) || colored.get(column).copied().unwrap_or(false);
                if before_ok && after_ok && colored_ok {
                    spans.push(FolderSpan {
                        row,
                        column,
                        width: needle.len(),
                        name: name.clone(),
                    });
                }
                from = end.max(column + 1);
            }
        }
    }
    spans
}

#[derive(Clone, Default)]
struct Cached {
    key: (u64, usize, usize, usize),
    spans: Vec<FolderSpan>,
}

/// Draws the buttons over `rect` (the terminal widget's rect) and returns
/// the `cd` command of a clicked one.
pub(super) fn paint(
    ui: &mut egui::Ui,
    tab: &TerminalTab,
    rect: egui::Rect,
    font: &FontId,
    accent: Color32,
) -> Option<String> {
    let content = tab.backend.last_content();
    if content
        .terminal_mode
        .contains(egui_term::TerminalMode::ALT_SCREEN)
    {
        return None;
    }
    let key = (
        tab.output.output_sequence(),
        content.display_offset,
        content.columns,
        content.screen_lines,
    );
    let id = egui::Id::new(("folder-chips", tab.id));
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<Cached>(id))
        .filter(|cached| cached.key == key);
    let spans = match cached {
        Some(cached) => cached.spans,
        None => {
            let spans = folder_spans(&tab.backend.screen_rows());
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    id,
                    Cached {
                        key,
                        spans: spans.clone(),
                    },
                )
            });
            spans
        }
    };
    let cell = egui::vec2(
        f32::from(content.terminal_size.cell_width),
        f32::from(content.terminal_size.cell_height),
    );
    let visuals = ui.visuals().clone();
    let mut run = None;
    for span in &spans {
        let min = rect.min + egui::vec2(span.column as f32 * cell.x, span.row as f32 * cell.y);
        let chip = egui::Rect::from_min_size(min, egui::vec2(span.width as f32 * cell.x, cell.y))
            .expand2(egui::vec2(2.0, 0.0));
        if !rect.intersects(chip) {
            continue;
        }
        // Click only: a drag that starts here still selects text.
        let response = ui
            .interact(
                chip,
                ui.id().with(("folder-chip", tab.id, span.row, span.column)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let widget = if response.hovered() {
            &visuals.widgets.hovered
        } else {
            &visuals.widgets.inactive
        };
        let painter = ui.painter_at(rect);
        painter.rect(
            chip,
            3.0,
            widget.weak_bg_fill,
            egui::Stroke::new(1.0_f32, accent),
            egui::StrokeKind::Inside,
        );
        painter.text(
            chip.center(),
            egui::Align2::CENTER_CENTER,
            &span.name,
            font.clone(),
            widget.fg_stroke.color,
        );
        if response.clicked() {
            run = Some(dock_nav::cd_command(&span.name));
        }
        response.on_hover_text(dock_nav::cd_command(&span.name));
    }
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(lines: &[(&str, &str)]) -> Vec<(String, Vec<bool>)> {
        // Second string marks colored cells with '#'.
        lines
            .iter()
            .map(|(text, mask)| {
                let colored = text
                    .chars()
                    .enumerate()
                    .map(|(index, _)| mask.chars().nth(index) == Some('#'))
                    .collect();
                ((*text).to_owned(), colored)
            })
            .collect()
    }

    #[test]
    fn spans_follow_colored_names_and_skip_files_sharing_a_name() {
        let screen = rows(&[
            ("$ ls", ""),
            ("src  src.txt  docs", "###           ####"),
            ("$ ", ""),
        ]);
        let spans = folder_spans(&screen);
        let found: Vec<_> = spans
            .iter()
            .map(|span| (span.row, span.column, span.width, span.name.as_str()))
            .collect();
        assert_eq!(found, [(1, 0, 3, "src"), (1, 14, 4, "docs")]);
    }

    #[test]
    fn long_listing_names_sit_at_the_end_of_their_rows() {
        let screen = rows(&[
            ("PS C:\\x> dir", ""),
            ("d-----   11/12/2026  10:03 AM   build", ""),
            ("-a----   11/12/2026  10:03 AM   build.log", ""),
        ]);
        let spans = folder_spans(&screen);
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!((spans[0].row, spans[0].column), (1, 32));
    }
}
