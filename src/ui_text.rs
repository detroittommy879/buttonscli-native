//! Apply zone tracking before layout, so widget bounds and wrapping follow the text.
use egui::{Color32, Id, Response, RichText, Ui, WidgetText};

const SPACING_KEY: &str = "buttonscli-zone-letter-spacing";

pub(crate) fn set_default_spacing(ctx: &egui::Context, spacing: f32) {
    ctx.data_mut(|data| data.insert_temp(Id::new(SPACING_KEY), spacing));
}

pub(crate) fn set_zone_spacing(ui: &Ui, spacing: f32) {
    let stamp = (
        ui.ctx().viewport_id(),
        ui.ctx().cumulative_pass_nr(),
        spacing,
    );
    ui.data_mut(|data| data.insert_temp(ui.stack().id.with(SPACING_KEY), stamp));
}

pub(crate) fn spaced_text(ui: &Ui, text: impl Into<WidgetText>) -> WidgetText {
    let viewport = ui.ctx().viewport_id();
    let pass = ui.ctx().cumulative_pass_nr();
    let spacing = ui.data(|data| {
        ui.stack()
            .iter()
            .find_map(|stack| {
                data.get_temp::<(egui::ViewportId, u64, f32)>(stack.id.with(SPACING_KEY))
                    .filter(|stamp| stamp.0 == viewport && stamp.1 == pass)
                    .map(|stamp| stamp.2)
            })
            .or_else(|| data.get_temp::<f32>(Id::new(SPACING_KEY)))
            .unwrap_or(0.0)
    });
    let text = text.into();
    if spacing == 0.0 {
        return text;
    }
    match text {
        WidgetText::RichText(text) => text.extra_letter_spacing(spacing).into(),
        WidgetText::LayoutJob(mut job) => {
            for section in &mut job.sections {
                section.format.extra_letter_spacing += spacing;
            }
            job.into()
        }
        galley @ WidgetText::Galley(_) => galley,
    }
}

pub(crate) fn button(ui: &Ui, text: impl Into<WidgetText>) -> egui::Button<'static> {
    egui::Button::new(spaced_text(ui, text))
}

pub(crate) trait SpacedUi {
    fn spaced_label(&mut self, text: impl Into<WidgetText>) -> Response;
    fn spaced_heading(&mut self, text: impl Into<RichText>) -> Response;
    fn spaced_small(&mut self, text: impl Into<RichText>) -> Response;
    #[cfg(not(target_arch = "wasm32"))]
    fn spaced_strong(&mut self, text: impl Into<RichText>) -> Response;
    fn spaced_weak(&mut self, text: impl Into<RichText>) -> Response;
    fn spaced_colored_label(&mut self, color: Color32, text: impl Into<RichText>) -> Response;
    fn spaced_button(&mut self, text: impl Into<WidgetText>) -> Response;
    fn spaced_small_button(&mut self, text: impl Into<WidgetText>) -> Response;
    fn spaced_selectable_label(&mut self, selected: bool, text: impl Into<WidgetText>) -> Response;
    fn spaced_selectable_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response;
    fn spaced_checkbox(&mut self, checked: &mut bool, text: impl Into<WidgetText>) -> Response;
    fn spaced_radio_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response;
    fn spaced_menu_button<R>(
        &mut self,
        text: impl Into<WidgetText>,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>>;
    /// A top-level menu button with its own open state, for toolbars that are
    /// not a menu bar (status bar, tab strip). Never call it inside a menu.
    fn solo_menu_button<R>(
        &mut self,
        id_salt: impl std::hash::Hash,
        text: impl Into<WidgetText>,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>>;
}

/// egui 0.31 keys `menu_button` state by the parent `Ui`, so every sibling
/// menu button acts as one menu bar: once one opens, hovering any other opens
/// it. The open menu is also keyed by the button's auto ID, which shifts when
/// earlier widgets appear, leaving an invisible "open" menu that turns every
/// hover into a click. Here each button owns its state under a stable ID and
/// re-keys its own open menu to the button's current ID.
pub(crate) fn solo_menu_button<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    text: impl Into<WidgetText>,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let bar_id = ui.id().with(("buttonscli-solo-menu", id_salt));
    let mut state = egui::menu::BarState::load(ui.ctx(), bar_id);
    let mut button = egui::Button::new(spaced_text(ui, text));
    if state.is_some() {
        button = button
            .fill(ui.visuals().widgets.open.weak_bg_fill)
            .stroke(ui.visuals().widgets.open.bg_stroke);
    }
    let response = ui.add(button);
    // egui flips a menu upward only as far as the bottom row of the screen,
    // so a menu from a wrapped status bar covered its own button and the
    // buttons around it. Open menus in the lower half above their button.
    let screen = ui.ctx().screen_rect();
    let above = response.rect.center().y > screen.center().y;
    if let Some(root) = state.as_mut() {
        root.id = response.id;
        if above {
            let mut menu = root.menu_state.write();
            let height = menu.rect.height();
            if height > 0.0 {
                let top = (response.rect.top() - height).max(screen.top());
                menu.rect = menu.rect.translate(egui::vec2(0.0, top - menu.rect.top()));
            }
        }
    }
    let mut anchor = response.clone();
    if above {
        // egui places a menu that would pass the screen bottom at
        // `screen.bottom - menu height - anchor height`: this anchor makes
        // that the button's top.
        anchor.rect = egui::Rect::from_min_max(
            response.rect.min,
            egui::pos2(
                response.rect.right(),
                screen.bottom().max(response.rect.bottom()),
            ),
        );
    }
    let inner = state.bar_menu(&anchor, contents);
    state.store(ui.ctx(), bar_id);
    egui::InnerResponse::new(inner.map(|inner| inner.inner), response)
}

impl SpacedUi for Ui {
    fn spaced_label(&mut self, text: impl Into<WidgetText>) -> Response {
        self.label(spaced_text(self, text))
    }
    fn spaced_heading(&mut self, text: impl Into<RichText>) -> Response {
        self.spaced_label(text.into().heading())
    }
    fn spaced_small(&mut self, text: impl Into<RichText>) -> Response {
        self.spaced_label(text.into().small())
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn spaced_strong(&mut self, text: impl Into<RichText>) -> Response {
        self.spaced_label(text.into().strong())
    }
    fn spaced_weak(&mut self, text: impl Into<RichText>) -> Response {
        self.spaced_label(text.into().weak())
    }
    fn spaced_colored_label(&mut self, color: Color32, text: impl Into<RichText>) -> Response {
        self.spaced_label(text.into().color(color))
    }
    fn spaced_button(&mut self, text: impl Into<WidgetText>) -> Response {
        self.button(spaced_text(self, text))
    }
    fn spaced_small_button(&mut self, text: impl Into<WidgetText>) -> Response {
        self.small_button(spaced_text(self, text))
    }
    fn spaced_selectable_label(&mut self, selected: bool, text: impl Into<WidgetText>) -> Response {
        self.selectable_label(selected, spaced_text(self, text))
    }
    fn spaced_selectable_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response {
        self.selectable_value(current, value, spaced_text(self, text))
    }
    fn spaced_checkbox(&mut self, checked: &mut bool, text: impl Into<WidgetText>) -> Response {
        self.checkbox(checked, spaced_text(self, text))
    }
    fn spaced_radio_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response {
        self.radio_value(current, value, spaced_text(self, text))
    }
    fn spaced_menu_button<R>(
        &mut self,
        text: impl Into<WidgetText>,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        self.menu_button(spaced_text(self, text), contents)
    }
    fn solo_menu_button<R>(
        &mut self,
        id_salt: impl std::hash::Hash,
        text: impl Into<WidgetText>,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        solo_menu_button(self, id_salt, text, contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_changes_layout_and_hit_bounds_and_inherits_through_children() {
        let ctx = egui::Context::default();
        let mut widths = Vec::new();
        for spacing in [-1.0, 0.0, 4.0] {
            set_default_spacing(&ctx, spacing);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    widths.push(ui.spaced_button("Tracking test").rect.width());
                    set_zone_spacing(ui, 2.0);
                    ui.horizontal(|ui| {
                        let WidgetText::RichText(text) = spaced_text(ui, "Inherited") else {
                            panic!("expected rich text");
                        };
                        let galley = WidgetText::from(text).into_galley(
                            ui,
                            None,
                            f32::INFINITY,
                            egui::TextStyle::Body,
                        );
                        assert_eq!(galley.job.sections[0].format.extra_letter_spacing, 2.0);
                    });
                });
            });
        }
        assert!(
            widths[0] < widths[1],
            "negative tracking tightens the button"
        );
        assert!(
            widths[1] < widths[2],
            "positive tracking expands the hit bounds"
        );
    }

    #[test]
    fn solo_menus_open_on_click_only_and_above_buttons_low_on_screen() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut rects = Vec::new();
        let mut opened = Vec::new();
        let mut frame = |events: Vec<egui::Event>| {
            rects.clear();
            opened.clear();
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::TopBottomPanel::bottom("bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for name in ["One", "Two"] {
                            let mut shown = None;
                            let menu = ui.solo_menu_button(name, name, |ui| {
                                ui.label(format!("{name} contents"));
                                shown = Some(ui.min_rect());
                            });
                            rects.push(menu.response.rect);
                            opened.push(shown);
                        }
                    });
                });
            });
            (rects.clone(), opened.clone())
        };
        let (buttons, _) = frame(Vec::new());
        let press = |pressed| egui::Event::PointerButton {
            pos: buttons[0].center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(vec![egui::Event::PointerMoved(buttons[0].center())]);
        frame(vec![press(true)]);
        frame(vec![press(false)]);
        for _ in 0..3 {
            frame(Vec::new());
        }
        let (_, open) = frame(Vec::new());
        let menu = open[0].expect("a click opens the first menu");
        assert!(
            menu.bottom() <= buttons[0].top(),
            "menu {menu:?} sits above its button {:?}",
            buttons[0]
        );
        // Hovering a sibling with a menu open must not switch menus.
        frame(vec![egui::Event::PointerMoved(buttons[1].center())]);
        let (_, open) = frame(vec![egui::Event::PointerMoved(buttons[1].center())]);
        assert!(open[1].is_none(), "hover alone never opens a menu");
        assert!(open[0].is_some(), "the clicked menu stays open");
    }
}
