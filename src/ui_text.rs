//! Apply zone tracking before layout, so widget bounds and wrapping follow the text.
use egui::{Color32, Id, Response, RichText, Ui, WidgetText};

const SPACING_KEY: &str = "buttonscli-zone-letter-spacing";

pub(crate) fn set_default_spacing(ctx: &egui::Context, spacing: f32) {
    ctx.data_mut(|data| data.insert_temp(Id::new(SPACING_KEY), spacing));
}

pub(crate) fn set_zone_spacing(ui: &Ui, spacing: f32) {
    ui.data_mut(|data| data.insert_temp(ui.stack().id.with(SPACING_KEY), spacing));
}

pub(crate) fn spaced_text(ui: &Ui, text: impl Into<WidgetText>) -> WidgetText {
    let spacing = ui.data(|data| {
        ui.stack()
            .iter()
            .find_map(|stack| data.get_temp::<f32>(stack.id.with(SPACING_KEY)))
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
                    set_zone_spacing(ui, spacing);
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
}
