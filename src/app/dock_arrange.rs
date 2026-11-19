//! Left panel → Arrange: drag the left-command buttons into a new order.

use super::*;

fn arrange_id() -> egui::Id {
    egui::Id::new("dock-arrange-mode")
}

pub(super) fn arranging(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(arrange_id()))
        .unwrap_or(false)
}

pub(super) fn set_arranging(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|data| data.insert_temp(arrange_id(), on));
}

/// Moves `items[from]` so it lands before the item that was at `to`
/// (`to == len` puts it last). Out-of-range moves do nothing.
pub(super) fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= items.len() || to > items.len() || to == from || to == from + 1 {
        return false;
    }
    let item = items.remove(from);
    let to = if to > from { to - 1 } else { to };
    items.insert(to, item);
    true
}

impl ButtonsApp {
    /// The left commands as drag handles. Returns true when the order changed.
    pub(super) fn dock_arrange_list(&mut self, ui: &mut egui::Ui) -> bool {
        let colors = self.colors();
        ui.spaced_label(
            RichText::new(crate::i18n::literal(
                &self.locale,
                "Drag buttons to reorder them, then press Done.",
            ))
            .small()
            .color(colors.muted),
        );
        ui.add_space(4.0);
        let labels: Vec<String> = self
            .preferences
            .ssh_presets
            .iter()
            .map(|preset| preset.label.clone())
            .collect();
        let count = labels.len();
        let mut moved = None;
        let width = ui.available_width();
        for (index, label) in labels.iter().enumerate() {
            let response = ui
                .dnd_drag_source(egui::Id::new(("dock-arrange", index)), index, |ui| {
                    egui::Frame::NONE
                        .fill(colors.dock_button)
                        .stroke(Stroke::new(1.0_f32, colors.accent_alt))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.set_width((width - 16.0).max(40.0));
                            ui.label(
                                RichText::new(format!("☰  {label}")).color(colors.dock_button_text),
                            );
                        });
                })
                .response;
            if let (Some(pointer), Some(_)) = (
                ui.input(|input| input.pointer.interact_pos()),
                response.dnd_hover_payload::<usize>(),
            ) {
                let rect = response.rect;
                let above = pointer.y < rect.center().y;
                let y = if above {
                    rect.top() - 2.0
                } else {
                    rect.bottom() + 2.0
                };
                ui.painter()
                    .hline(rect.x_range(), y, Stroke::new(3.0_f32, colors.accent));
                if let Some(from) = response.dnd_release_payload::<usize>() {
                    moved = Some((*from, if above { index } else { index + 1 }));
                }
            }
            ui.add_space(2.0);
        }
        let mut changed = false;
        if let Some((from, to)) = moved {
            if from < count {
                changed = move_item(&mut self.preferences.ssh_presets, from, to);
            }
        }
        ui.add_space(6.0);
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                crate::ui_text::button(ui, crate::i18n::literal(&self.locale, "Done arranging")),
            )
            .clicked()
        {
            set_arranging(ui.ctx(), false);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_land_before_the_target_and_noops_are_noops() {
        let mut items = vec!['a', 'b', 'c', 'd'];
        assert!(move_item(&mut items, 0, 3));
        assert_eq!(items, ['b', 'c', 'a', 'd']);
        assert!(move_item(&mut items, 3, 0));
        assert_eq!(items, ['d', 'b', 'c', 'a']);
        assert!(move_item(&mut items, 1, 4));
        assert_eq!(items, ['d', 'c', 'a', 'b']);
        for (from, to) in [(1, 1), (1, 2), (9, 0), (0, 9)] {
            assert!(!move_item(&mut items, from, to));
        }
        assert_eq!(items, ['d', 'c', 'a', 'b']);
    }
}
