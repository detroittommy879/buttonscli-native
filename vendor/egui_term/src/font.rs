use egui::{Context, FontId};

use crate::types::Size;

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub font_type: FontId,
    pub bold_font_type: Option<FontId>,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            font_type: FontId::monospace(14.0),
            bold_font_type: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFont {
    font_type: FontId,
    bold_font_type: Option<FontId>,
}

impl Default for TerminalFont {
    fn default() -> Self {
        Self {
            font_type: FontSettings::default().font_type,
            bold_font_type: None,
        }
    }
}

impl TerminalFont {
    pub fn new(settings: FontSettings) -> Self {
        Self {
            font_type: settings.font_type,
            bold_font_type: settings.bold_font_type,
        }
    }

    pub fn font_type(&self) -> FontId {
        self.font_type.clone()
    }

    pub fn bold_font_type(&self) -> FontId {
        self.bold_font_type
            .clone()
            .unwrap_or_else(|| self.font_type())
    }

    pub fn font_measure(&self, ctx: &Context) -> Size {
        let (width, height) = ctx.fonts(|f| {
            (
                f.glyph_width(&self.font_type, 'm'),
                f.row_height(&self.font_type),
            )
        });

        Size::new(width, height)
    }
}
