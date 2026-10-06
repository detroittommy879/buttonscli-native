use alacritty_terminal::index::Point as TerminalGridPoint;
use alacritty_terminal::term::cell;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use egui::epaint::RectShape;
use egui::Modifiers;
use egui::MouseWheelUnit;
use egui::Shape;
use egui::Widget;
use egui::{
    Align2, Color32, Mesh, Painter, Pos2, Rect, Response, Stroke, Vec2,
};
use egui::{CornerRadius, Key};
use egui::{Id, PointerButton};

use crate::backend::BackendCommand;
use crate::backend::TerminalBackend;
use crate::backend::{LinkAction, MouseButton, SelectionType};
use crate::bindings::Binding;
use crate::bindings::{BindingAction, BindingsLayout, InputKind};
use crate::font::TerminalFont;
use crate::theme::TerminalTheme;
use crate::types::Size;

const EGUI_TERM_WIDGET_ID_PREFIX: &str = "egui_term::instance::";

#[derive(Debug, Clone)]
enum InputAction {
    BackendCall(BackendCommand),
    WriteToClipboard(String),
    Ignore,
}

#[derive(Clone, Default)]
pub struct TerminalViewState {
    is_dragged: bool,
    is_selecting: bool,
    selection_scroll_at: Option<f64>,
    ime_composing: bool,
    scroll_pixels: f32,
    current_mouse_position_on_grid: TerminalGridPoint,
}

pub struct TerminalView<'a> {
    widget_id: Id,
    has_focus: bool,
    interactive: bool,
    size: Vec2,
    backend: &'a mut TerminalBackend,
    font: TerminalFont,
    theme: TerminalTheme,
    background_gradient: Option<BackgroundGradient>,
    row_banding_color: Option<Color32>,
    row_brightness: u8,
    ctrl_c_copies_selection: bool,
    copy_on_selection: bool,
    bracketed_paste: bool,
    option_as_meta: bool,
    draw_bold_bright: bool,
    bindings_layout: BindingsLayout,
}

#[derive(Clone, Copy, Debug)]
pub enum BackgroundGradient {
    Linear {
        colors: [Color32; 4],
        angle_degrees: f32,
    },
    RepeatingLinear {
        colors: [Color32; 4],
        angle_degrees: f32,
    },
    Radial {
        colors: [Color32; 4],
        center: [f32; 2],
    },
    RepeatingRadial {
        colors: [Color32; 4],
        center: [f32; 2],
    },
    Conic {
        colors: [Color32; 4],
        center: [f32; 2],
        angle_degrees: f32,
    },
    RepeatingConic {
        colors: [Color32; 4],
        center: [f32; 2],
        angle_degrees: f32,
    },
}

impl Widget for TerminalView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (_, rect) = ui.allocate_space(self.size);
        let layout =
            ui.interact(rect, self.widget_id, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let widget_id = self.widget_id;
        let mut state = ui.memory(|m| {
            m.data
                .get_temp::<TerminalViewState>(widget_id)
                .unwrap_or_default()
        });

        self.focus(&layout)
            .resize(&layout)
            .process_input(&layout, &mut state)
            .show(&mut state, &layout, &painter);

        ui.memory_mut(|m| m.data.insert_temp(widget_id, state));
        layout
    }
}

impl<'a> TerminalView<'a> {
    pub fn new(ui: &mut egui::Ui, backend: &'a mut TerminalBackend) -> Self {
        let widget_id = ui.make_persistent_id(format!(
            "{}{}",
            EGUI_TERM_WIDGET_ID_PREFIX, backend.id
        ));

        Self {
            widget_id,
            has_focus: false,
            interactive: true,
            size: ui.available_size(),
            backend,
            font: TerminalFont::default(),
            theme: TerminalTheme::default(),
            background_gradient: None,
            row_banding_color: None,
            row_brightness: 0,
            ctrl_c_copies_selection: true,
            copy_on_selection: false,
            bracketed_paste: true,
            option_as_meta: false,
            draw_bold_bright: false,
            bindings_layout: BindingsLayout::new(),
        }
    }

    #[inline]
    pub fn set_theme(mut self, theme: TerminalTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Replace the global terminal background fill with a gradient mesh. Cells
    /// that explicitly set a background color still paint over it.
    #[inline]
    pub fn set_background_gradient(
        mut self,
        gradient: Option<BackgroundGradient>,
    ) -> Self {
        self.background_gradient = gradient;
        self
    }

    /// Add a translucent tint to every second terminal row. The band pitch
    /// comes from the measured terminal cell height, not a font-size guess.
    #[inline]
    pub fn set_row_banding(mut self, color: Option<Color32>) -> Self {
        self.row_banding_color = color;
        self
    }

    pub fn set_row_brightness(mut self, amount: u8) -> Self {
        self.row_brightness = amount;
        self
    }

    pub fn set_keyboard_options(
        mut self,
        ctrl_copy: bool,
        auto_copy: bool,
        bracketed: bool,
        option_meta: bool,
    ) -> Self {
        self.ctrl_c_copies_selection = ctrl_copy;
        self.copy_on_selection = auto_copy;
        self.bracketed_paste = bracketed;
        self.option_as_meta = option_meta;
        self
    }

    #[inline]
    pub fn set_draw_bold_bright(mut self, enabled: bool) -> Self {
        self.draw_bold_bright = enabled;
        self
    }

    #[inline]
    pub fn set_font(mut self, font: TerminalFont) -> Self {
        self.font = font;
        self
    }

    #[inline]
    pub fn set_focus(mut self, has_focus: bool) -> Self {
        self.has_focus = has_focus;
        self
    }

    /// Disable all input while a modal owns the workspace.
    pub fn set_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    #[inline]
    pub fn set_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    #[inline]
    pub fn add_bindings(
        mut self,
        bindings: Vec<(Binding<InputKind>, BindingAction)>,
    ) -> Self {
        self.bindings_layout.add_bindings(bindings);
        self
    }

    fn focus(self, layout: &Response) -> Self {
        if self.has_focus
            && !layout.context_menu_opened()
            && !layout.ctx.memory(|memory| memory.any_popup_open())
            && layout.ctx.memory(|memory| {
                memory.focused().is_none() || memory.has_focus(layout.id)
            })
        {
            // request_focus resets egui's event filter, even when this widget
            // already owns focus. Preserve the filter across terminal frames.
            if !layout.ctx.memory(|memory| memory.has_focus(layout.id)) {
                layout.request_focus();
            }
        } else if !self.has_focus && !layout.gained_focus() {
            layout.surrender_focus();
        }

        if self.interactive
            && self.has_focus
            && layout.ctx.memory(|memory| memory.has_focus(layout.id))
        {
            lock_terminal_focus(layout);
        }

        self
    }

    fn resize(self, layout: &Response) -> Self {
        self.backend.process_command(BackendCommand::Resize(
            Size::from(layout.rect.size()),
            self.font.font_measure(&layout.ctx),
        ));

        self
    }

    fn process_input(
        self,
        layout: &Response,
        state: &mut TerminalViewState,
    ) -> Self {
        if !self.interactive {
            state.is_dragged = false;
            state.is_selecting = false;
            state.selection_scroll_at = None;
            state.ime_composing = false;
            return self;
        }
        if !self.has_focus || !layout.has_focus() {
            state.ime_composing = false;
        }

        let modifiers = layout.ctx.input(|i| i.modifiers);
        let mut text_modifiers = modifiers;
        let events = layout.ctx.input(|i| i.events.clone());
        let mut wheel_scrolled = false;
        for event in events {
            if let egui::Event::Key {
                pressed: true,
                modifiers,
                ..
            } = &event
            {
                text_modifiers = *modifiers;
            }
            let selection_finished = self.copy_on_selection
                && state.is_selecting
                && matches!(
                    &event,
                    egui::Event::PointerButton {
                        button: PointerButton::Primary,
                        pressed: false,
                        ..
                    }
                );
            let mut input_actions = vec![];

            match event {
                egui::Event::Ime(event)
                    if self.has_focus
                        && layout.has_focus()
                        && !layout
                            .ctx
                            .memory(|memory| memory.any_popup_open()) =>
                {
                    match event {
                        egui::ImeEvent::Enabled => {},
                        egui::ImeEvent::Preedit(text) => {
                            state.ime_composing = !text.is_empty()
                        },
                        egui::ImeEvent::Commit(text) => {
                            state.ime_composing = false;
                            if !text.is_empty() {
                                input_actions.push(InputAction::BackendCall(
                                    BackendCommand::Write(text.into_bytes()),
                                ));
                            }
                        },
                        egui::ImeEvent::Disabled => state.ime_composing = false,
                    }
                },
                egui::Event::Text(_) | egui::Event::Key { .. }
                    if state.ime_composing => {},
                egui::Event::Text(_)
                | egui::Event::Key { .. }
                | egui::Event::Copy
                | egui::Event::Paste(_)
                    if self.has_focus
                        && layout.has_focus()
                        && !layout.context_menu_opened()
                        && !layout
                            .ctx
                            .memory(|memory| memory.any_popup_open()) =>
                {
                    input_actions.push(process_keyboard_event(
                        event,
                        self.backend,
                        &self.bindings_layout,
                        text_modifiers,
                        self.ctrl_c_copies_selection,
                        self.bracketed_paste,
                        self.option_as_meta,
                    ))
                },
                egui::Event::MouseWheel { unit, delta, .. }
                    if layout.contains_pointer() || state.is_selecting =>
                {
                    wheel_scrolled = true;
                    input_actions.push(process_mouse_wheel(
                        state,
                        self.font.font_type().size,
                        unit,
                        delta,
                    ));
                    // Scrolling with a stationary pointer must extend the
                    // selection to the newly visible line as well.
                    if state.is_selecting {
                        if let Some(pos) = layout.ctx.pointer_latest_pos() {
                            input_actions.push(InputAction::BackendCall(
                                BackendCommand::SelectUpdate(
                                    pos.x - layout.rect.left(),
                                    pos.y - layout.rect.top(),
                                ),
                            ));
                        }
                    }
                },
                egui::Event::PointerButton {
                    button,
                    pressed,
                    modifiers,
                    pos,
                    ..
                } if ((layout.contains_pointer()
                    || layout.is_pointer_button_down_on())
                    && layout.rect.contains(pos))
                    || (button == PointerButton::Primary
                        && !pressed
                        && state.is_dragged) =>
                {
                    if pressed && button == PointerButton::Primary {
                        if !layout.has_focus() {
                            layout.request_focus();
                        }
                        lock_terminal_focus(layout);
                    }
                    input_actions.push(process_button_click(
                        state,
                        layout,
                        self.backend,
                        &self.bindings_layout,
                        button,
                        pos,
                        &modifiers,
                        pressed,
                    ));
                },
                egui::Event::PointerMoved(pos)
                    if layout.contains_pointer() || state.is_dragged =>
                {
                    input_actions = process_mouse_move(
                        state,
                        layout,
                        self.backend,
                        pos,
                        &modifiers,
                    )
                },
                _ => {},
            };

            for action in input_actions {
                match action {
                    InputAction::BackendCall(cmd) => {
                        self.backend.process_command(cmd);
                    },
                    InputAction::WriteToClipboard(data) => {
                        layout.ctx.copy_text(data);
                    },
                    InputAction::Ignore => {},
                }
            }
            if selection_finished {
                let selected = self.backend.selectable_content();
                if !selected.is_empty() {
                    layout.ctx.copy_text(selected);
                }
            }
        }

        if state.is_selecting && layout.ctx.input(|i| i.pointer.primary_down())
        {
            if let Some(pos) = layout.ctx.pointer_latest_pos() {
                let delta = selection_edge_scroll(
                    layout.rect,
                    pos,
                    self.font.font_type().size,
                );
                if delta != 0 {
                    let now = layout.ctx.input(|i| i.time);
                    if !wheel_scrolled
                        && state
                            .selection_scroll_at
                            .is_none_or(|last| now - last >= 0.05)
                    {
                        self.backend
                            .process_command(BackendCommand::Scroll(delta));
                        self.backend.process_command(
                            BackendCommand::SelectUpdate(
                                pos.x - layout.rect.left(),
                                pos.y - layout.rect.top(),
                            ),
                        );
                        state.selection_scroll_at = Some(now);
                    }
                    layout.ctx.request_repaint_after(
                        std::time::Duration::from_millis(50),
                    );
                } else {
                    state.selection_scroll_at = None;
                }
            }
        } else {
            state.selection_scroll_at = None;
            state.is_selecting = false;
            state.is_dragged = false;
        }

        self
    }

    fn show(
        self,
        state: &mut TerminalViewState,
        layout: &Response,
        painter: &Painter,
    ) {
        let content = self.backend.sync();
        let layout_min = layout.rect.min;
        let layout_max = layout.rect.max;
        let cell_height = content.terminal_size.cell_height as f32;
        let cell_width = content.terminal_size.cell_width as f32;
        if self.interactive
            && self.has_focus
            && layout.has_focus()
            && !layout.ctx.memory(|memory| memory.any_popup_open())
        {
            let cursor = content.grid.cursor.point;
            let origin = layout_min
                + egui::vec2(
                    cursor.column.0 as f32 * cell_width,
                    cursor.line.0.max(0) as f32 * cell_height,
                );
            layout.ctx.output_mut(|output| {
                output.ime = Some(egui::output::IMEOutput {
                    rect: layout.rect,
                    cursor_rect: Rect::from_min_size(
                        origin,
                        egui::vec2(cell_width, cell_height),
                    )
                    .intersect(layout.rect),
                })
            });
        }
        let global_bg =
            self.theme.get_color(Color::Named(NamedColor::Background));

        let background = if let Some(gradient) = self.background_gradient {
            Shape::mesh(background_gradient_mesh(
                Rect::from_min_max(layout_min, layout_max),
                gradient,
            ))
        } else {
            Shape::Rect(RectShape::filled(
                Rect::from_min_max(layout_min, layout_max),
                CornerRadius::ZERO,
                global_bg,
            ))
        };
        let mut shapes = vec![background];
        if self.row_brightness > 0 {
            shapes.extend(brightness_banding_shapes(
                layout.rect,
                cell_height,
                global_bg,
                self.row_brightness,
                self.background_gradient,
            ));
        }

        for indexed in content.grid.display_iter() {
            let flags = indexed.cell.flags;
            let is_wide_char_spacer =
                flags.contains(cell::Flags::WIDE_CHAR_SPACER);
            if is_wide_char_spacer {
                continue;
            }

            let is_app_cursor_mode =
                content.terminal_mode.contains(TermMode::APP_CURSOR);
            let is_wide_char = flags.contains(cell::Flags::WIDE_CHAR);
            let is_inverse = flags.contains(cell::Flags::INVERSE);
            let is_bold =
                flags.intersects(cell::Flags::BOLD | cell::Flags::DIM_BOLD);
            let is_dim =
                flags.intersects(cell::Flags::DIM | cell::Flags::DIM_BOLD);
            let is_selected = content
                .selectable_range
                .is_some_and(|r| r.contains(indexed.point));
            let search_cell = (indexed.point.line.0, indexed.point.column.0);
            let is_current_search_match =
                content.current_search_highlights.contains(&search_cell);
            let is_search_match =
                content.search_highlights.contains(&search_cell);
            let is_hovered_hyperling =
                content.hovered_hyperlink.as_ref().is_some_and(|r| {
                    r.contains(&indexed.point)
                        && r.contains(&state.current_mouse_position_on_grid)
                });

            let x = layout_min.x + (cell_width * indexed.point.column.0 as f32);
            let line_num =
                indexed.point.line.0 + content.grid.display_offset() as i32;
            let y = layout_min.y + (cell_height * line_num as f32);

            let mut fg = self.theme.get_color(indexed.fg);
            let mut bg = self.theme.get_color(indexed.bg);
            let cell_width = if is_wide_char {
                cell_width * 2.0
            } else {
                cell_width
            };

            if is_bold && self.draw_bold_bright {
                fg = self.theme.get_bold_color(indexed.fg);
            }
            if is_dim {
                fg = fg.linear_multiply(0.7);
            }

            if is_inverse || is_selected {
                std::mem::swap(&mut fg, &mut bg);
            }
            if !is_selected && is_current_search_match {
                bg = Color32::from_rgb(191, 119, 18);
                fg = Color32::WHITE;
            } else if !is_selected && is_search_match {
                bg = Color32::from_rgb(105, 80, 31);
                fg = Color32::WHITE;
            }

            if global_bg != bg {
                shapes.push(Shape::Rect(RectShape::filled(
                    Rect::from_min_size(
                        Pos2::new(x, y),
                        // + 1.0 is to fill grid border
                        Vec2::new(cell_width + 1., cell_height + 1.),
                    ),
                    CornerRadius::ZERO,
                    bg,
                )));
            }

            // Handle hovered hyperlink underline
            if is_hovered_hyperling {
                let underline_height = y + cell_height;
                shapes.push(Shape::LineSegment {
                    points: [
                        Pos2::new(x, underline_height),
                        Pos2::new(x + cell_width, underline_height),
                    ],
                    stroke: Stroke::new(cell_height * 0.15, fg).into(),
                });
            }

            // Handle cursor rendering
            if content.grid.cursor.point == indexed.point {
                let cursor_color = self.theme.get_color(content.cursor.fg);
                shapes.push(Shape::Rect(RectShape::filled(
                    Rect::from_min_size(
                        Pos2::new(x, y),
                        Vec2::new(cell_width, cell_height),
                    ),
                    CornerRadius::default(),
                    cursor_color,
                )));
            }

            // Draw text content
            if indexed.c != ' ' && indexed.c != '\t' {
                if content.grid.cursor.point == indexed.point
                    && is_app_cursor_mode
                {
                    std::mem::swap(&mut fg, &mut bg);
                }

                shapes.push(Shape::text(
                    &painter.fonts(|c| c.clone()),
                    Pos2 {
                        x: x + (cell_width / 2.0),
                        y,
                    },
                    Align2::CENTER_TOP,
                    indexed.c,
                    if is_bold {
                        self.font.bold_font_type()
                    } else {
                        self.font.font_type()
                    },
                    fg,
                ));
            }
        }

        if let Some(color) = self.row_banding_color {
            shapes.extend(row_banding_shapes(layout.rect, cell_height, color));
        }

        painter.extend(shapes);
    }
}

fn brightness_banding_shapes(
    rect: Rect,
    cell_height: f32,
    base: Color32,
    amount: u8,
    gradient: Option<BackgroundGradient>,
) -> Vec<Shape> {
    if !cell_height.is_finite() || cell_height <= 0.0 || amount == 0 {
        return Vec::new();
    }
    if let Some(gradient) = gradient {
        let original = background_gradient_mesh(rect, gradient);
        let mut mesh = Mesh::default();
        for row in 0..(rect.height() / cell_height).ceil() as usize {
            let top = rect.top() + row as f32 * cell_height;
            let bottom = (top + cell_height).min(rect.bottom());
            for indices in original.indices.chunks_exact(3) {
                let triangle: Vec<_> = indices
                    .iter()
                    .map(|&i| original.vertices[i as usize])
                    .collect();
                let clipped = clip_background_polygon(
                    &clip_background_polygon(&triangle, top, true),
                    bottom,
                    false,
                );
                if clipped.len() < 3 {
                    continue;
                }
                let offset = mesh.vertices.len() as u32;
                for mut vertex in clipped {
                    let channel = |v: u8| {
                        if row % 2 == 0 {
                            v.saturating_add(amount)
                        } else {
                            v.saturating_sub(amount)
                        }
                    };
                    vertex.color = Color32::from_rgb(
                        channel(vertex.color.r()),
                        channel(vertex.color.g()),
                        channel(vertex.color.b()),
                    );
                    mesh.vertices.push(vertex);
                }
                for i in 1..(mesh.vertices.len() as u32 - offset - 1) {
                    mesh.indices.extend([offset, offset + i, offset + i + 1]);
                }
            }
        }
        return vec![Shape::mesh(mesh)];
    }
    (0..(rect.height() / cell_height).ceil() as usize)
        .map(|row| {
            let channel = |v: u8| {
                if row % 2 == 0 {
                    v.saturating_add(amount)
                } else {
                    v.saturating_sub(amount)
                }
            };
            Shape::Rect(RectShape::filled(
                Rect::from_min_max(
                    Pos2::new(
                        rect.left(),
                        rect.top() + row as f32 * cell_height,
                    ),
                    Pos2::new(
                        rect.right(),
                        (rect.top() + (row + 1) as f32 * cell_height)
                            .min(rect.bottom()),
                    ),
                ),
                CornerRadius::ZERO,
                Color32::from_rgb(
                    channel(base.r()),
                    channel(base.g()),
                    channel(base.b()),
                ),
            ))
        })
        .collect()
}

fn clip_background_polygon(
    vertices: &[egui::epaint::Vertex],
    y: f32,
    above: bool,
) -> Vec<egui::epaint::Vertex> {
    let mut result = Vec::new();
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        let inside = |v: egui::epaint::Vertex| {
            if above {
                v.pos.y >= y
            } else {
                v.pos.y <= y
            }
        };
        if inside(a) {
            result.push(a);
        }
        if inside(a) != inside(b) {
            let t = (y - a.pos.y) / (b.pos.y - a.pos.y);
            let channel = |x: u8, z: u8| {
                (x as f32 + (z as f32 - x as f32) * t).round() as u8
            };
            result.push(egui::epaint::Vertex {
                pos: egui::pos2(a.pos.x + (b.pos.x - a.pos.x) * t, y),
                uv: a.uv + (b.uv - a.uv) * t,
                color: Color32::from_rgba_premultiplied(
                    channel(a.color.r(), b.color.r()),
                    channel(a.color.g(), b.color.g()),
                    channel(a.color.b(), b.color.b()),
                    channel(a.color.a(), b.color.a()),
                ),
            });
        }
    }
    result
}

fn paste_payload(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        // Escape cannot be allowed to terminate the application's paste envelope.
        format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', "")).into_bytes()
    } else {
        text.replace("\r\n", "\n").replace('\n', "\r").into_bytes()
    }
}

fn row_banding_shapes(
    rect: Rect,
    cell_height: f32,
    color: Color32,
) -> Vec<Shape> {
    if !cell_height.is_finite() || cell_height <= 0.0 || color.a() == 0 {
        return Vec::new();
    }

    let rows = (rect.height() / cell_height).ceil().max(0.0) as usize;
    (1..rows)
        .step_by(2)
        .filter_map(|row| {
            let top = rect.top() + row as f32 * cell_height;
            let bottom = (top + cell_height).min(rect.bottom());
            (bottom > top).then(|| {
                Shape::Rect(RectShape::filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left(), top),
                        Pos2::new(rect.right(), bottom),
                    ),
                    CornerRadius::ZERO,
                    color,
                ))
            })
        })
        .collect()
}

fn background_gradient_mesh(rect: Rect, gradient: BackgroundGradient) -> Mesh {
    match gradient {
        BackgroundGradient::Linear {
            colors,
            angle_degrees,
        } => linear_gradient_mesh(rect, colors, angle_degrees),
        BackgroundGradient::RepeatingLinear {
            colors,
            angle_degrees,
        } => repeating_linear_gradient_mesh(rect, colors, angle_degrees),
        BackgroundGradient::Radial { colors, center } => {
            radial_gradient_mesh(rect, colors, center, false)
        },
        BackgroundGradient::RepeatingRadial { colors, center } => {
            radial_gradient_mesh(rect, colors, center, true)
        },
        BackgroundGradient::Conic {
            colors,
            center,
            angle_degrees,
        } => conic_gradient_mesh(rect, colors, center, angle_degrees, false),
        BackgroundGradient::RepeatingConic {
            colors,
            center,
            angle_degrees,
        } => conic_gradient_mesh(rect, colors, center, angle_degrees, true),
    }
}

fn linear_gradient_mesh(
    rect: Rect,
    colors: [Color32; 4],
    angle_degrees: f32,
) -> Mesh {
    let angle = angle_degrees.to_radians();
    let direction = Vec2::new(angle.cos(), angle.sin());
    let extent = direction.x.abs() + direction.y.abs();
    let color_at = |x: f32, y: f32| {
        let projection = (x - 0.5) * direction.x + (y - 0.5) * direction.y;
        gradient_color(colors, 0.5 + projection / extent.max(0.001))
    };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), color_at(0.0, 0.0));
    mesh.colored_vertex(rect.right_top(), color_at(1.0, 0.0));
    mesh.colored_vertex(rect.left_bottom(), color_at(0.0, 1.0));
    mesh.colored_vertex(rect.right_bottom(), color_at(1.0, 1.0));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    mesh
}

fn repeating_linear_gradient_mesh(
    rect: Rect,
    colors: [Color32; 4],
    angle_degrees: f32,
) -> Mesh {
    const BAND_COUNT: usize = 10;
    let angle = angle_degrees.to_radians();
    let direction = Vec2::new(angle.cos(), angle.sin());
    let extent = (direction.x.abs() + direction.y.abs()).max(0.001);
    let corners = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    let progress = |point: Pos2| {
        0.5 + ((point.x - 0.5) * direction.x + (point.y - 0.5) * direction.y)
            / extent
    };
    let mut mesh = Mesh::default();

    for band in 0..BAND_COUNT {
        let low = band as f32 / BAND_COUNT as f32;
        let high = (band + 1) as f32 / BAND_COUNT as f32;
        let polygon =
            clip_gradient_polygon(corners.to_vec(), low, true, &progress);
        let polygon = clip_gradient_polygon(polygon, high, false, &progress);
        if polygon.len() < 3 {
            continue;
        }

        let first_vertex = mesh.vertices.len() as u32;
        let first_color = colors[band % 3];
        let second_color = colors[(band % 3) + 1];
        for point in &polygon {
            let amount =
                ((progress(*point) - low) / (high - low)).clamp(0.0, 1.0);
            let position = Pos2::new(
                rect.left() + rect.width() * point.x,
                rect.top() + rect.height() * point.y,
            );
            mesh.colored_vertex(
                position,
                mix_color(first_color, second_color, amount),
            );
        }
        for vertex in 1..polygon.len() - 1 {
            mesh.add_triangle(
                first_vertex,
                first_vertex + vertex as u32,
                first_vertex + vertex as u32 + 1,
            );
        }
    }
    mesh
}

fn clip_gradient_polygon(
    polygon: Vec<Pos2>,
    boundary: f32,
    keep_greater: bool,
    progress: &impl Fn(Pos2) -> f32,
) -> Vec<Pos2> {
    if polygon.is_empty() {
        return polygon;
    }
    let inside = |point: Pos2| {
        if keep_greater {
            progress(point) >= boundary
        } else {
            progress(point) <= boundary
        }
    };
    let mut clipped = Vec::with_capacity(polygon.len() + 2);
    let mut previous = *polygon.last().expect("nonempty polygon checked");
    let mut previous_inside = inside(previous);
    for current in polygon {
        let current_inside = inside(current);
        if current_inside != previous_inside {
            let previous_progress = progress(previous);
            let current_progress = progress(current);
            let denominator = current_progress - previous_progress;
            if denominator.abs() > f32::EPSILON {
                let amount = ((boundary - previous_progress) / denominator)
                    .clamp(0.0, 1.0);
                clipped.push(previous + (current - previous) * amount);
            }
        }
        if current_inside {
            clipped.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
    clipped
}

fn radial_gradient_mesh(
    rect: Rect,
    colors: [Color32; 4],
    center: [f32; 2],
    repeating: bool,
) -> Mesh {
    const SEGMENTS: usize = 48;
    const NORMAL_RINGS: usize = 4;
    const REPEATING_RINGS: usize = 31;
    let ring_count = if repeating {
        REPEATING_RINGS
    } else {
        NORMAL_RINGS
    };
    let center = Pos2::new(
        rect.left() + rect.width() * center[0].clamp(0.0, 1.0),
        rect.top() + rect.height() * center[1].clamp(0.0, 1.0),
    );
    let radius = [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .into_iter()
    .map(|corner| center.distance(corner))
    .fold(0.0_f32, f32::max);
    let mut mesh = Mesh::default();
    for ring in 0..ring_count {
        let progress = ring as f32 / (ring_count - 1) as f32;
        let color = if repeating {
            repeating_linear_color(colors, progress)
        } else {
            gradient_color(colors, progress)
        };
        for segment in 0..SEGMENTS {
            let angle =
                std::f32::consts::TAU * segment as f32 / SEGMENTS as f32;
            mesh.colored_vertex(
                center + Vec2::angled(angle) * radius * progress,
                color,
            );
        }
    }
    for ring in 0..ring_count - 1 {
        for segment in 0..SEGMENTS {
            let next = (segment + 1) % SEGMENTS;
            let inner = (ring * SEGMENTS + segment) as u32;
            let inner_next = (ring * SEGMENTS + next) as u32;
            let outer = ((ring + 1) * SEGMENTS + segment) as u32;
            let outer_next = ((ring + 1) * SEGMENTS + next) as u32;
            mesh.add_triangle(inner, outer, inner_next);
            mesh.add_triangle(inner_next, outer, outer_next);
        }
    }
    mesh
}

fn conic_gradient_mesh(
    rect: Rect,
    colors: [Color32; 4],
    center: [f32; 2],
    angle_degrees: f32,
    repeating: bool,
) -> Mesh {
    const SEGMENTS: usize = 64;
    let center = Pos2::new(
        rect.left() + rect.width() * center[0].clamp(0.0, 1.0),
        rect.top() + rect.height() * center[1].clamp(0.0, 1.0),
    );
    let radius = [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .into_iter()
    .map(|corner| center.distance(corner))
    .fold(0.0_f32, f32::max);
    let offset = angle_degrees / 360.0;
    let mut mesh = Mesh::default();
    for segment in 0..SEGMENTS {
        let start = segment as f32 / SEGMENTS as f32;
        let end = (segment + 1) as f32 / SEGMENTS as f32;
        let start_angle = std::f32::consts::TAU * start;
        let end_angle = std::f32::consts::TAU * end;
        let start_color = if repeating {
            repeating_conic_color(colors, start + offset)
        } else {
            conic_color(colors, start + offset)
        };
        let end_color = if repeating {
            repeating_conic_color(colors, end + offset)
        } else {
            conic_color(colors, end + offset)
        };
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(center, start_color);
        mesh.colored_vertex(
            center + Vec2::angled(start_angle) * radius,
            start_color,
        );
        mesh.colored_vertex(
            center + Vec2::angled(end_angle) * radius,
            end_color,
        );
        mesh.add_triangle(base, base + 1, base + 2);
    }
    mesh
}

fn gradient_color(colors: [Color32; 4], progress: f32) -> Color32 {
    let scaled = progress.clamp(0.0, 1.0) * 3.0;
    let index = (scaled.floor() as usize).min(2);
    mix_color(colors[index], colors[index + 1], scaled - index as f32)
}

fn conic_color(colors: [Color32; 4], progress: f32) -> Color32 {
    let scaled = progress.rem_euclid(1.0) * 4.0;
    let index = (scaled.floor() as usize).min(3);
    mix_color(
        colors[index],
        colors[(index + 1) % 4],
        scaled - index as f32,
    )
}

fn repeating_conic_color(colors: [Color32; 4], progress: f32) -> Color32 {
    let within_turn = progress.rem_euclid(1.0 / 3.0) * 0.9;
    repeating_linear_color(colors, within_turn)
}

fn repeating_linear_color(colors: [Color32; 4], progress: f32) -> Color32 {
    let within_period = progress.rem_euclid(0.3);
    let segment = (within_period / 0.1).floor() as usize;
    let index = segment.min(2);
    let amount = ((within_period - index as f32 * 0.1) / 0.1).clamp(0.0, 1.0);
    let amount = if amount < 0.00001 {
        0.0
    } else if 1.0 - amount < 0.00001 {
        1.0
    } else {
        amount
    };
    mix_color(colors[index], colors[index + 1], amount)
}

fn mix_color(first: Color32, second: Color32, amount: f32) -> Color32 {
    let channel =
        |a: u8, b: u8| (a as f32 * (1.0 - amount) + b as f32 * amount) as u8;
    Color32::from_rgba_premultiplied(
        channel(first.r(), second.r()),
        channel(first.g(), second.g()),
        channel(first.b(), second.b()),
        channel(first.a(), second.a()),
    )
}

fn lock_terminal_focus(layout: &Response) {
    // Shell navigation, completion and Escape belong to the terminal. The app
    // explicitly surrenders focus for F6 control navigation and UI text fields.
    layout.ctx.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            layout.id,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
}

fn process_keyboard_event(
    event: egui::Event,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    modifiers: Modifiers,
    ctrl_c_copies_selection: bool,
    bracketed_paste: bool,
    option_as_meta: bool,
) -> InputAction {
    match event {
        egui::Event::Text(text) => {
            if cfg!(target_os = "macos") && modifiers.alt && !option_as_meta {
                return InputAction::BackendCall(BackendCommand::Write(
                    text.into_bytes(),
                ));
            }
            if cfg!(target_os = "macos") && modifiers.alt && option_as_meta {
                // Key bindings emitted Escape + key; skip their composed Text companion.
                return InputAction::Ignore;
            }
            process_text_event(&text, modifiers, backend, bindings_layout)
        },
        // A Paste event already contains text explicitly requested from the
        // platform clipboard. Forward the payload verbatim. Checking modifier
        // state here is racy because clipboard delivery can arrive on a later
        // frame, after Ctrl/Command and Shift have been released.
        egui::Event::Paste(text) => {
            InputAction::BackendCall(BackendCommand::Write(paste_payload(
                &text,
                bracketed_paste
                    && backend
                        .last_content()
                        .terminal_mode
                        .contains(TermMode::BRACKETED_PASTE),
            )))
        },
        egui::Event::Copy => copy_action(
            backend.selectable_content(),
            modifiers,
            ctrl_c_copies_selection,
            cfg!(target_os = "macos"),
        ),
        egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } => {
            if cfg!(target_os = "macos")
                && modifiers.alt
                && !option_as_meta
                && !modifiers.ctrl
                && !modifiers.command
                && (key.name().chars().count() == 1 || key == Key::Space)
            {
                // Let composed Text (including non-US Option characters) own typing.
                InputAction::Ignore
            } else {
                process_keyboard_key(
                    backend,
                    bindings_layout,
                    key,
                    modifiers,
                    pressed,
                )
            }
        },
        _ => InputAction::Ignore,
    }
}

fn copy_action(
    content: String,
    modifiers: Modifiers,
    ctrl_copy: bool,
    macos: bool,
) -> InputAction {
    if !content.is_empty()
        && (macos || ctrl_copy || modifiers.shift || !modifiers.ctrl)
    {
        InputAction::WriteToClipboard(content)
    } else if !macos && !modifiers.shift {
        InputAction::BackendCall(BackendCommand::Write(vec![0x3]))
    } else {
        InputAction::Ignore
    }
}

fn process_text_event(
    text: &str,
    modifiers: Modifiers,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
) -> InputAction {
    if let Some(key) = Key::from_name(text) {
        if bindings_layout.get_action(
            InputKind::KeyCode(key),
            modifiers,
            backend.last_content().terminal_mode,
        ) == BindingAction::Ignore
        {
            InputAction::BackendCall(BackendCommand::Write(
                text.as_bytes().to_vec(),
            ))
        } else {
            InputAction::Ignore
        }
    } else {
        InputAction::BackendCall(BackendCommand::Write(
            text.as_bytes().to_vec(),
        ))
    }
}

fn process_keyboard_key(
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
) -> InputAction {
    if !pressed {
        return InputAction::Ignore;
    }

    let terminal_mode = backend.last_content().terminal_mode;
    let binding_action = bindings_layout.get_action(
        InputKind::KeyCode(key),
        modifiers,
        terminal_mode,
    );

    match binding_action {
        BindingAction::Char(c) => {
            let mut buf = [0, 0, 0, 0];
            let str = c.encode_utf8(&mut buf);
            InputAction::BackendCall(BackendCommand::Write(
                str.as_bytes().to_vec(),
            ))
        },
        BindingAction::Esc(seq) => InputAction::BackendCall(
            BackendCommand::Write(seq.as_bytes().to_vec()),
        ),
        _ => InputAction::Ignore,
    }
}

fn process_mouse_wheel(
    state: &mut TerminalViewState,
    font_size: f32,
    unit: MouseWheelUnit,
    delta: Vec2,
) -> InputAction {
    match unit {
        MouseWheelUnit::Line => {
            let lines = delta.y.signum() * delta.y.abs().ceil();
            InputAction::BackendCall(BackendCommand::Scroll(lines as i32))
        },
        MouseWheelUnit::Point => {
            state.scroll_pixels -= delta.y;
            let lines = (state.scroll_pixels / font_size).trunc();
            state.scroll_pixels %= font_size;
            if lines != 0.0 {
                InputAction::BackendCall(BackendCommand::Scroll(-lines as i32))
            } else {
                InputAction::Ignore
            }
        },
        MouseWheelUnit::Page => InputAction::Ignore,
    }
}

fn selection_edge_scroll(rect: Rect, pointer: Pos2, cell_height: f32) -> i32 {
    let edge = cell_height.clamp(8.0, 24.0);
    if pointer.y < rect.top() + edge {
        (1.0 + (rect.top() + edge - pointer.y) / cell_height.max(1.0))
            .clamp(1.0, 8.0) as i32
    } else if pointer.y > rect.bottom() - edge {
        -((1.0 + (pointer.y - rect.bottom() + edge) / cell_height.max(1.0))
            .clamp(1.0, 8.0) as i32)
    } else {
        0
    }
}

fn process_button_click(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    button: PointerButton,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    match button {
        PointerButton::Primary => process_left_button(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
            pressed,
        ),
        _ => InputAction::Ignore,
    }
}

fn process_left_button(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    let terminal_mode = backend.last_content().terminal_mode;
    if terminal_mode.intersects(TermMode::MOUSE_MODE) && !modifiers.shift {
        state.is_dragged = pressed;
        state.is_selecting = false;
        InputAction::BackendCall(BackendCommand::MouseReport(
            MouseButton::LeftButton,
            *modifiers,
            state.current_mouse_position_on_grid,
            pressed,
        ))
    } else if pressed {
        process_left_button_pressed(state, layout, position)
    } else {
        process_left_button_released(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
        )
    }
}

fn process_left_button_pressed(
    state: &mut TerminalViewState,
    layout: &Response,
    position: Pos2,
) -> InputAction {
    state.is_dragged = true;
    state.is_selecting = true;
    state.selection_scroll_at = None;
    InputAction::BackendCall(build_start_select_command(layout, position))
}

fn process_left_button_released(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
) -> InputAction {
    state.is_dragged = false;
    state.is_selecting = false;
    state.selection_scroll_at = None;
    if layout.double_clicked() || layout.triple_clicked() {
        InputAction::BackendCall(build_start_select_command(layout, position))
    } else {
        let terminal_content = backend.last_content();
        let binding_action = bindings_layout.get_action(
            InputKind::Mouse(PointerButton::Primary),
            *modifiers,
            terminal_content.terminal_mode,
        );

        if binding_action == BindingAction::LinkOpen {
            InputAction::BackendCall(BackendCommand::ProcessLink(
                LinkAction::Open,
                state.current_mouse_position_on_grid,
            ))
        } else {
            InputAction::Ignore
        }
    }
}

fn build_start_select_command(
    layout: &Response,
    cursor_position: Pos2,
) -> BackendCommand {
    let selection_type = if layout.double_clicked() {
        SelectionType::Semantic
    } else if layout.triple_clicked() {
        SelectionType::Lines
    } else {
        SelectionType::Simple
    };

    BackendCommand::SelectStart(
        selection_type,
        cursor_position.x - layout.rect.min.x,
        cursor_position.y - layout.rect.min.y,
    )
}

fn process_mouse_move(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    position: Pos2,
    modifiers: &Modifiers,
) -> Vec<InputAction> {
    let terminal_content = backend.last_content();
    let cursor_x = position.x - layout.rect.min.x;
    let cursor_y = position.y - layout.rect.min.y;
    state.current_mouse_position_on_grid = TerminalBackend::selection_point(
        cursor_x,
        cursor_y,
        &terminal_content.terminal_size,
        terminal_content.grid.display_offset(),
    );

    let mut actions = vec![];
    // Handle command or selection update based on terminal mode and modifiers
    if state.is_dragged {
        let terminal_mode = terminal_content.terminal_mode;
        let cmd = if terminal_mode.contains(TermMode::MOUSE_MOTION)
            && modifiers.is_none()
        {
            InputAction::BackendCall(BackendCommand::MouseReport(
                MouseButton::LeftMove,
                *modifiers,
                state.current_mouse_position_on_grid,
                true,
            ))
        } else {
            InputAction::BackendCall(BackendCommand::SelectUpdate(
                cursor_x, cursor_y,
            ))
        };

        actions.push(cmd);
    }

    // Handle link hover if applicable
    if modifiers.command_only() {
        actions.push(InputAction::BackendCall(BackendCommand::ProcessLink(
            LinkAction::Hover,
            state.current_mouse_position_on_grid,
        )));
    }

    actions
}

#[cfg(test)]
mod background_gradient_tests {
    use super::*;

    #[test]
    fn copy_survives_released_modifiers_and_keeps_mac_command_separate_from_interrupt(
    ) {
        assert!(
            matches!(copy_action("selected".into(), Modifiers::NONE, true, false), InputAction::WriteToClipboard(text) if text == "selected")
        );
        assert!(
            matches!(copy_action("selected".into(), Modifiers::CTRL, false, false), InputAction::BackendCall(BackendCommand::Write(bytes)) if bytes == [3])
        );
        assert!(
            matches!(copy_action(String::new(), Modifiers::CTRL, true, false), InputAction::BackendCall(BackendCommand::Write(bytes)) if bytes == [3])
        );
        assert!(matches!(
            copy_action(String::new(), Modifiers::NONE, true, true),
            InputAction::Ignore
        ));
        assert!(matches!(
            copy_action(
                "Mac selection".into(),
                Modifiers::MAC_CMD,
                false,
                true
            ),
            InputAction::WriteToClipboard(_)
        ));
        assert!(matches!(
            copy_action(
                String::new(),
                Modifiers::CTRL | Modifiers::SHIFT,
                true,
                false
            ),
            InputAction::Ignore
        ));
    }

    #[test]
    fn paste_wraps_requested_mode_and_normalizes_shell_line_endings() {
        assert_eq!(
            paste_payload("one\ntwo", true),
            b"\x1b[200~one\ntwo\x1b[201~"
        );
        assert_eq!(paste_payload("one\r\ntwo\n", false), b"one\rtwo\r");
        assert_eq!(
            paste_payload("a\x1b[201~b", true),
            b"\x1b[200~a[201~b\x1b[201~"
        );
    }

    #[test]
    fn brightness_bands_center_hex_channels_and_keep_gradient_geometry() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 40.0));
        let shapes = brightness_banding_shapes(
            rect,
            20.0,
            Color32::from_rgb(0x55, 0x55, 0x55),
            16,
            None,
        );
        for (shape, expected) in shapes.iter().zip([0x65, 0x45]) {
            let Shape::Rect(shape) = shape else {
                panic!("solid band");
            };
            assert_eq!(
                shape.fill,
                Color32::from_rgb(expected, expected, expected)
            );
        }
        let gradient = BackgroundGradient::Linear {
            colors: [
                Color32::from_rgb(60, 60, 60),
                Color32::from_rgb(80, 80, 80),
                Color32::from_rgb(100, 100, 100),
                Color32::from_rgb(120, 120, 120),
            ],
            angle_degrees: 0.0,
        };
        let bands = brightness_banding_shapes(
            rect,
            20.0,
            Color32::BLACK,
            1,
            Some(gradient),
        );
        let Shape::Mesh(mesh) = &bands[0] else {
            panic!("gradient mesh");
        };
        assert!(mesh.is_valid());
        assert!(mesh.vertices.iter().all(|v| rect.contains(v.pos)));
        assert!(mesh.vertices.iter().any(|v| v.color.r() == 61));
        assert!(mesh.vertices.iter().any(|v| v.color.r() == 59));
    }

    fn colors() -> [Color32; 4] {
        [Color32::RED, Color32::GREEN, Color32::BLUE, Color32::WHITE]
    }

    #[test]
    fn repeating_gradient_samples_cycle_at_the_legacy_thirty_percent_period() {
        let colors = colors();
        assert_eq!(repeating_linear_color(colors, 0.0), Color32::RED);
        assert_eq!(repeating_linear_color(colors, 0.1), Color32::GREEN);
        assert_eq!(repeating_linear_color(colors, 0.2), Color32::BLUE);
        assert_eq!(repeating_linear_color(colors, 0.3), Color32::RED);
        assert_eq!(
            repeating_linear_color(colors, 0.35),
            mix_color(Color32::RED, Color32::GREEN, 0.5)
        );
        assert_eq!(
            repeating_conic_color(colors, 0.0),
            repeating_conic_color(colors, 1.0 / 3.0)
        );
    }

    #[test]
    fn repeating_linear_mesh_covers_the_terminal_rect_with_multiple_bands() {
        let rect =
            Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(210.0, 120.0));
        let mesh = repeating_linear_gradient_mesh(rect, colors(), 135.0);
        assert!(mesh.vertices.len() > 4);
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|vertex| {
            vertex.pos.x >= rect.left() - 0.01
                && vertex.pos.x <= rect.right() + 0.01
                && vertex.pos.y >= rect.top() - 0.01
                && vertex.pos.y <= rect.bottom() + 0.01
        }));
    }

    #[test]
    fn repeating_radial_and_conic_meshes_generate_their_repeated_regions() {
        let rect =
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(160.0, 80.0));
        let radial = radial_gradient_mesh(rect, colors(), [0.5, 0.5], true);
        assert_eq!(radial.vertices.len(), 31 * 48);
        assert_eq!(radial.indices.len(), 30 * 48 * 6);

        let conic = conic_gradient_mesh(rect, colors(), [0.5, 0.5], 45.0, true);
        assert_eq!(conic.vertices.len(), 64 * 3);
        assert_eq!(conic.indices.len(), 64 * 3);
    }

    #[test]
    fn row_bands_follow_cell_pitch_and_clip_the_last_partial_row() {
        let rect =
            Rect::from_min_max(Pos2::new(2.0, 3.0), Pos2::new(22.0, 38.0));
        let color = Color32::from_rgba_unmultiplied(1, 2, 3, 24);
        let shapes = row_banding_shapes(rect, 10.0, color);
        assert_eq!(shapes.len(), 2);

        let Shape::Rect(first) = &shapes[0] else {
            panic!("expected a filled row band");
        };
        assert_eq!(
            first.rect,
            Rect::from_min_max(Pos2::new(2.0, 13.0), Pos2::new(22.0, 23.0))
        );
        assert_eq!(first.fill, color);

        let Shape::Rect(last) = &shapes[1] else {
            panic!("expected a filled row band");
        };
        assert_eq!(
            last.rect,
            Rect::from_min_max(Pos2::new(2.0, 33.0), Pos2::new(22.0, 38.0))
        );
    }

    #[test]
    fn row_banding_skips_zero_alpha_and_invalid_cell_heights() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(10.0, 30.0));
        assert!(row_banding_shapes(rect, 10.0, Color32::TRANSPARENT).is_empty());
        assert!(row_banding_shapes(rect, 0.0, Color32::WHITE).is_empty());
        assert!(row_banding_shapes(rect, f32::NAN, Color32::WHITE).is_empty());
    }
}
