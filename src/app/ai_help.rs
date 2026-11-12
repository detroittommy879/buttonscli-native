// AI Help view and request routing. No model-generated UI or executable markup.
use super::*;

#[cfg(test)]
#[path = "ai_help_tests.rs"]
mod tests;

#[derive(Clone)]
struct ChatTerminal {
    id: u64,
    title: String,
    shell: String,
    simulated: bool,
    exited: bool,
}

struct ChatView {
    terminals: Vec<ChatTerminal>,
    provider: Option<ProviderProfile>,
    available: bool,
    agent_available: bool,
    locale: String,
}

impl AiHelpWindowState {
    fn invalidate_context(&mut self) {
        self.context_generation = self.context_generation.wrapping_add(1);
        self.context_preview = None;
        self.context_busy = false;
    }

    fn select_terminal(&mut self, terminal: Option<(u64, String)>) {
        if self.selected_target.as_ref().map(|t| t.0) != terminal.as_ref().map(|t| t.0) {
            self.invalidate_context();
            self.status = None;
        }
        self.selected_target = terminal;
    }

    fn preview_request(&mut self) -> Option<AiHelpCommand> {
        let target_id = self.selected_target.as_ref()?.0;
        self.invalidate_context();
        self.context_busy = true;
        Some(AiHelpCommand::PreviewContext {
            target_id,
            generation: self.context_generation,
        })
    }
}

impl ChatView {
    fn terminal_label(terminal: &ChatTerminal) -> String {
        format!(
            "{} · {} · {} · #{}",
            terminal.title,
            if terminal.simulated {
                "SIMULATED"
            } else {
                "Real shell"
            },
            terminal.shell,
            terminal.id
        )
    }
    fn selected(&self, state: &AiHelpWindowState) -> Option<&ChatTerminal> {
        let id = state.selected_target.as_ref()?.0;
        self.terminals.iter().find(|t| t.id == id && !t.exited)
    }

    fn blocked_reason(&self, state: &AiHelpWindowState) -> Option<&'static str> {
        if state.busy {
            return Some("Waiting for the current response…");
        }
        if !self.available {
            return Some("AI Help is disabled by the current feature settings.");
        }
        let Some(provider) = &self.provider else {
            return Some("Choose an AI Help provider in AI providers.");
        };
        if provider.model.trim().is_empty() || validate_endpoint(&provider.endpoint).is_err() {
            return Some("Choose a model and a valid endpoint in AI providers.");
        }
        if state.selected_target.is_some() && self.selected(state).is_none() {
            return Some("This terminal has closed. Select another terminal or Chat only.");
        }
        if state.agent_mode {
            if !self.agent_available {
                return Some("Agent Mode is unavailable. Turn it off to use ordinary chat.");
            }
            let Some(terminal) = self.selected(state) else {
                return Some("Select a terminal for Agent Mode.");
            };
            if terminal.simulated {
                return Some("Turn off Agent Mode to chat about this simulation. Simulation execution is not available yet.");
            }
        }
        if state.include_context || state.agent_mode {
            if self.selected(state).is_none() {
                return Some("Select a terminal to include its output, or turn off the snapshot.");
            }
            if state.context_busy {
                return Some("Preparing the terminal snapshot…");
            }
            if state.context_preview.as_ref().map(|p| p.session_id)
                != state.selected_target.as_ref().map(|t| t.0)
            {
                return Some("Preview the selected terminal's snapshot before sending.");
            }
        }
        if state.input.trim().is_empty() {
            return Some("Type a question to get started.");
        }
        if state.input.chars().count() > 16_384 {
            return Some("Shorten your question to 16,384 characters or fewer.");
        }
        None
    }

    fn header(
        &self,
        ui: &mut egui::Ui,
        state: &mut AiHelpWindowState,
        queue: &dyn Fn(AiHelpCommand),
    ) {
        wrapping_row(ui, |ui| {
            ui.spaced_heading("AI Help");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.spaced_small_button("AI providers").clicked() {
                    queue(AiHelpCommand::OpenSettings);
                }
            });
        });
        if let Some(provider) = &self.provider {
            ui.spaced_weak(format!("{} · {}", provider.name, provider.model));
        } else {
            ui.spaced_weak("Connect a provider to start chatting.");
            if ui
                .spaced_button("Sign in with ChatGPT")
                .on_hover_text("Opens AI providers. A ChatGPT plan needs no API key.")
                .clicked()
            {
                queue(AiHelpCommand::OpenSettings);
            }
        }
        ui.add_space(6.0);
        ui.add_enabled_ui(!state.busy, |ui| {
            // A nonwrapping row keeps ComboBox's inherited content layout
            // from measuring against the next full row when the window shrinks.
            ui.horizontal(|ui| {
                ui.spaced_strong("Terminal");
                let previous = state.selected_target.clone();
                let selected = previous.as_ref().map_or_else(
                    || "Chat only".to_owned(),
                    |(id, title)| {
                        self.terminals
                            .iter()
                            .find(|t| t.id == *id)
                            .map_or_else(|| format!("{title} · closed"), Self::terminal_label)
                    },
                );
                let mut next = previous.clone();
                egui::ComboBox::from_id_salt("ai-help-terminal")
                    .width((ui.available_width() - 2.0).clamp(60.0, 540.0))
                    .truncate()
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut next, None, "Chat only");
                        for terminal in self.terminals.iter().filter(|t| !t.exited) {
                            ui.selectable_value(
                                &mut next,
                                Some((terminal.id, terminal.title.clone())),
                                Self::terminal_label(terminal),
                            );
                        }
                    });
                if next != previous {
                    state.select_terminal(next);
                    if state.include_context {
                        if let Some(command) = state.preview_request() {
                            queue(command);
                        }
                    }
                }
            });
        });
        if state.selected_target.is_some() {
            ui.spaced_small("Shell type goes with your question. Terminal output is optional.");
        }
        ui.add_space(6.0);
    }

    fn composer(
        &self,
        ui: &mut egui::Ui,
        state: &mut AiHelpWindowState,
        queue: &dyn Fn(AiHelpCommand),
    ) {
        use crate::i18n::{text, MessageKey};
        let locale = &self.locale;
        if let Some(error) = &state.error {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.spaced_colored_label(ui.visuals().error_fg_color, error);
                    if !state.busy && !state.last_request_agent {
                        if let Some((question, context, target)) = state.last_request.clone() {
                            if ui
                                .spaced_small_button(text(locale, MessageKey::AiHelpRetry, &[]))
                                .clicked()
                            {
                                queue(AiHelpCommand::Submit(question, context, target));
                            }
                        }
                    }
                });
        }
        if state.busy {
            wrapping_row(ui, |ui| {
                ui.spinner();
                ui.spaced_label("Receiving response…");
                if ui
                    .spaced_button(text(locale, MessageKey::Cancel, &[]))
                    .clicked()
                {
                    if let Some(cancel) = &state.cancel {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            });
        } else if let Some(status) = &state.status {
            ui.spaced_weak(status);
        }
        ui.add_enabled_ui(!state.busy, |ui| {
            wrapping_row(ui, |ui| {
                if ui
                    .add_enabled(
                        !state.agent_mode,
                        egui::Checkbox::new(&mut state.include_context, "Include terminal output"),
                    )
                    .changed()
                {
                    state.invalidate_context();
                    if state.include_context {
                        if let Some(command) = state.preview_request() {
                            queue(command);
                        }
                    }
                }
                if (self.agent_available || state.agent_mode)
                    && ui
                        .spaced_checkbox(&mut state.agent_mode, "Agent Mode")
                        .changed()
                {
                    state.include_context = state.agent_mode;
                    state.reviewed_actions.clear();
                    state.invalidate_context();
                    if state.include_context {
                        if let Some(command) = state.preview_request() {
                            queue(command);
                        }
                    }
                }
            });
            if state.agent_mode {
                ui.spaced_small(text(locale, MessageKey::AiAgentConsent, &[]));
            }
            if state.include_context || state.agent_mode {
                if state.context_busy {
                    ui.spaced_small("Preparing snapshot…");
                } else if let Some(preview) = &state.context_preview {
                    egui::CollapsingHeader::new(format!(
                        "Snapshot ready · {} · #{}",
                        preview.title, preview.session_id
                    ))
                    .id_salt("ai-help-snapshot")
                    .show(ui, |ui| {
                        ui.spaced_small(&preview.shell);
                        egui::ScrollArea::vertical()
                            .id_salt("ai-help-context-output")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&preview.output).monospace())
                                        .wrap()
                                        .selectable(true),
                                );
                            });
                    });
                }
                if !state.context_busy && ui.spaced_small_button("Refresh snapshot").clicked() {
                    if let Some(command) = state.preview_request() {
                        queue(command);
                    }
                }
                ui.spaced_small(
                    "Included output goes to your AI provider. Secret masking is best effort.",
                );
            }
            ui.add_space(5.0);
            let editor = ui.add(
                egui::TextEdit::multiline(&mut state.input)
                    .id_salt("ai-help-input")
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("Ask a question or describe what you want to do…"),
            );
            let blocked = self.blocked_reason(state);
            let shortcut = editor.has_focus()
                && ui.input_mut(|input| {
                    input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                });
            let mut send = false;
            wrapping_row(ui, |ui| {
                let mut button = egui::Button::new("Send").min_size(Vec2::new(76.0, 30.0));
                if blocked.is_none() {
                    button = egui::Button::new(
                        RichText::new("Send")
                            .color(button_text_color(ui.visuals().selection.bg_fill)),
                    )
                    .fill(ui.visuals().selection.bg_fill)
                    .min_size(Vec2::new(76.0, 30.0));
                }
                send = ui.add_enabled(blocked.is_none(), button).clicked();
                ui.spaced_small(blocked.unwrap_or("Ready · Ctrl/Cmd+Enter to send"));
            });
            if (send || shortcut) && blocked.is_none() {
                let question = std::mem::take(&mut state.input).trim().to_owned();
                let context = (state.include_context || state.agent_mode)
                    .then(|| state.context_preview.clone())
                    .flatten();
                queue(AiHelpCommand::Submit(
                    question,
                    context,
                    state.selected_target.as_ref().map(|t| t.0),
                ));
            }
        });
    }

    fn transcript(
        &self,
        ui: &mut egui::Ui,
        state: &AiHelpWindowState,
        queue: &dyn Fn(AiHelpCommand),
    ) {
        egui::ScrollArea::vertical().id_salt("ai-help-conversation").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if state.messages.is_empty() {
                ui.add_space(25.0);
                ui.spaced_heading("What are you working on?");
                ui.spaced_weak("Ask about a command, understand an error, or plan your next step.");
                ui.add_space(8.0);
                ui.spaced_small("Terminal output is shared only when you include a snapshot. Suggested actions wait for your review.");
            }
            for (index, (assistant, message)) in state.messages.iter().enumerate() {
                ui.push_id(index, |ui| {
                    let fill = if *assistant { ui.visuals().widgets.inactive.bg_fill.gamma_multiply(0.55) } else { ui.visuals().selection.bg_fill.gamma_multiply(0.24) };
                    egui::Frame::new().fill(fill).corner_radius(8).inner_margin(12).show(ui, |ui| {
                        ui.set_width((ui.available_width() - 1.0).max(0.0));
                        wrapping_row(ui, |ui| {
                            ui.spaced_strong(if *assistant { "AI Help" } else { "You" });
                            if let Some((id, title)) = state.message_targets.get(&index) {
                                ui.spaced_weak(format!("{title} · #{id}"));
                            }
                            if !message.is_empty() && ui.spaced_small_button("Copy").clicked() {
                                ui.ctx().copy_text(message.clone());
                            }
                        });
                        ui.add_space(4.0);
                        if message.is_empty() && state.busy {
                            ui.spaced_weak("Thinking…");
                        } else if *assistant {
                            render_message(ui, message);
                        } else {
                            ui.add(egui::Label::new(message).wrap().selectable(true));
                        }
                    });
                    ui.add_space(10.0);
                });
            }
            for (index, action) in state.reviewed_actions.iter().enumerate() {
                ui.push_id(("reviewed-action", index), |ui| {
                    egui::Frame::new().stroke(Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)).corner_radius(8).inner_margin(12).show(ui, |ui| {
                        ui.set_width((ui.available_width() - 1.0).max(0.0));
                        ui.spaced_small("SUGGESTED ACTION · REVIEW BEFORE SENDING");
                        let target = state.target.as_ref();
                        let target_id = target.map(|t| t.0);
                        let ready = self.available && self.terminals.iter().any(|t| Some(t.id) == target_id && !t.exited);
                        ui.spaced_weak(target.map_or_else(|| "No terminal selected".into(), |(id, title)| format!("Target: {title} · #{id}{}", if ready { "" } else { " · unavailable" })));
                        match action {
                            crate::assistant::reply::SuggestedAction::Command { label, description, command, .. } => {
                                ui.spaced_strong(label);
                                ui.spaced_label(description);
                                code_block(ui, command);
                                wrapping_row(ui, |ui| {
                                    if ui.add_enabled(ready, egui::Button::new("Insert")).clicked() {
                                        queue(AiHelpCommand::Deliver { target_id, action: action.clone(), press_enter: false });
                                    }
                                    if ui.add_enabled(ready, egui::Button::new("Insert + Enter")).clicked() {
                                        queue(AiHelpCommand::Deliver { target_id, action: action.clone(), press_enter: true });
                                    }
                                });
                            }
                            crate::assistant::reply::SuggestedAction::Control { label, description, key } => {
                                ui.spaced_strong(label);
                                ui.spaced_label(description);
                                ui.monospace(format!("{key:?}"));
                                if ui.add_enabled(ready, egui::Button::new("Send reviewed key")).clicked() {
                                    queue(AiHelpCommand::Deliver { target_id, action: action.clone(), press_enter: false });
                                }
                            }
                        }
                    });
                });
            }
        });
    }

    fn body(
        &self,
        ui: &mut egui::Ui,
        state: &mut AiHelpWindowState,
        queue: &dyn Fn(AiHelpCommand),
    ) {
        self.header(ui, state, queue);
        egui::TopBottomPanel::bottom("ai-help-composer")
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                top: 10,
                ..Default::default()
            }))
            .resizable(false)
            .show_inside(ui, |ui| self.composer(ui, state, queue));
        self.transcript(ui, state, queue);
    }
}

fn code_block(ui: &mut egui::Ui, code: &str) {
    egui::Frame::new()
        .fill(ui.visuals().code_bg_color)
        .corner_radius(5)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 1.0).max(0.0));
            if ui.spaced_small_button("Copy code").clicked() {
                ui.ctx().copy_text(code.to_owned());
            }
            ui.add(
                egui::Label::new(RichText::new(code).monospace())
                    .wrap()
                    .selectable(true),
            );
        });
}

fn button_text_color(background: Color32) -> Color32 {
    let linear = |channel: u8| {
        let value = channel as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(background.r())
        + 0.7152 * linear(background.g())
        + 0.0722 * linear(background.b());
    if (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05) {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

// A deliberately small Markdown subset. HTML, URLs, images and styling tags
// remain inert text. Code fences are recognized before inline formatting.
#[derive(Debug, PartialEq)]
enum ChatBlock<'a> {
    Text(&'a str),
    Code(&'a str),
}

fn chat_blocks(source: &str) -> Vec<ChatBlock<'_>> {
    let mut blocks = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut start = 0;
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        let first = trimmed.as_bytes().first().copied().unwrap_or_default();
        let count = trimmed.bytes().take_while(|b| *b == first).count();
        if first == b'`' || first == b'~' {
            if let Some((marker, width)) = fence {
                if first == marker && count >= width && trimmed[count..].is_empty() {
                    blocks.push(ChatBlock::Code(&source[start..offset]));
                    start = offset + line.len();
                    fence = None;
                }
            } else if count >= 3 {
                if start < offset {
                    blocks.push(ChatBlock::Text(&source[start..offset]));
                }
                fence = Some((first, count));
                start = offset + line.len();
            }
        }
        offset += line.len();
    }
    if start < source.len() {
        blocks.push(if fence.is_some() {
            ChatBlock::Code(&source[start..])
        } else {
            ChatBlock::Text(&source[start..])
        });
    }
    blocks
}

fn inline_text(ui: &egui::Ui, source: &str, strong: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = TextStyle::Body.resolve(ui.style());
    let mut remaining = source;
    while !remaining.is_empty() {
        let next = ["`", "**"]
            .iter()
            .filter_map(|marker| remaining.find(marker).map(|index| (index, *marker)))
            .min_by_key(|(index, _)| *index);
        let mut append = |text: &str, bold: bool, code: bool| {
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: if code {
                        TextStyle::Monospace.resolve(ui.style())
                    } else {
                        font.clone()
                    },
                    color: if bold || strong {
                        ui.visuals().strong_text_color()
                    } else {
                        ui.visuals().text_color()
                    },
                    background: if code {
                        ui.visuals().code_bg_color
                    } else {
                        Color32::TRANSPARENT
                    },
                    ..Default::default()
                },
            );
        };
        let Some((index, marker)) = next else {
            append(remaining, strong, false);
            break;
        };
        append(&remaining[..index], strong, false);
        let after = &remaining[index + marker.len()..];
        if let Some(end) = after.find(marker) {
            append(&after[..end], marker == "**", marker == "`");
            remaining = &after[end + marker.len()..];
        } else {
            append(&remaining[index..], strong, false);
            break;
        }
    }
    job
}

fn render_message(ui: &mut egui::Ui, message: &str) {
    for block in chat_blocks(message) {
        match block {
            ChatBlock::Code(code) => {
                code_block(ui, code);
            }
            ChatBlock::Text(text) => {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        ui.add_space(5.0);
                        continue;
                    }
                    let heading = trimmed
                        .strip_prefix("### ")
                        .or_else(|| trimmed.strip_prefix("## "))
                        .or_else(|| trimmed.strip_prefix("# "));
                    let text = heading.unwrap_or(trimmed);
                    let text = text
                        .strip_prefix("- ")
                        .or_else(|| text.strip_prefix("* "))
                        .map_or_else(|| text.to_owned(), |text| format!("• {text}"));
                    let job = inline_text(ui, &text, heading.is_some());
                    ui.add(egui::Label::new(job).wrap().selectable(true));
                }
            }
        }
    }
}

impl ButtonsApp {
    pub(super) fn open_ai_help(&mut self) {
        let mut preview = None;
        if let Ok(mut state) = self.ai_help_state.lock() {
            state.open = true;
            state.select_terminal(
                self.tabs
                    .get(self.focused)
                    .filter(|t| !t.exited)
                    .map(|t| (t.id, t.title.clone())),
            );
            if !state.busy && state.include_context && state.context_preview.is_none() {
                preview = state.preview_request();
            }
        }
        if let Some(command) = preview {
            let _ = self.ai_help_tx.send(command);
        }
    }

    pub(super) fn ai_help_window(&self, ctx: &egui::Context) {
        if !self.ai_help_state.lock().is_ok_and(|s| s.open) {
            return;
        }
        let view = ChatView {
            terminals: self
                .tabs
                .iter()
                .map(|t| ChatTerminal {
                    id: t.id,
                    title: t.title.clone(),
                    shell: t.shell_name.clone(),
                    simulated: t.simulated.is_some(),
                    exited: t.exited,
                })
                .collect(),
            provider: self.preferences.provider_settings.for_task(AiTask::Help),
            available: ai_help_available(),
            agent_available: ai_agent_available(),
            locale: self.locale.clone(),
        };
        let style = ctx.style_of(egui::Theme::Dark);
        let catalog = self.font_catalog.clone();
        let assistant_font = self.preferences.typography.assistant.clone();
        let state = Arc::clone(&self.ai_help_state);
        let actions = self.ai_help_tx.clone();
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-ai-help"),
            egui::ViewportBuilder::default()
                .with_title("AI Help")
                .with_icon(crate::app_icon())
                .with_inner_size([780.0, 760.0])
                .with_min_inner_size([480.0, 480.0]),
            move |child_ctx, class| {
                child_ctx.set_theme(egui::Theme::Dark);
                child_ctx.set_style((*style).clone());
                let Ok(mut state) = state.lock() else {
                    return;
                };
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    if state.last_request_agent {
                        if let Some(cancel) = &state.cancel {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                    state.open = false;
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                let queue = |command| {
                    if actions.send(command).is_ok() {
                        child_ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                };
                let mut body = |ui: &mut egui::Ui| {
                    apply_zone_style(ui, &catalog, &assistant_font);
                    view.body(ui, &mut state, &queue);
                };
                if class == egui::ViewportClass::Embedded {
                    egui::Window::new("AI Help")
                        .default_size([760.0, 720.0])
                        .show(child_ctx, &mut body);
                } else {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::central_panel(&child_ctx.style()).inner_margin(16))
                        .show(child_ctx, body);
                }
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn process_ai_help_commands(&mut self, ctx: &egui::Context) {
        let locale = self.locale.clone();
        let help_viewport = egui::ViewportId::from_hash_of("buttonscli-ai-help");
        let mut processed_any = false;
        while let Ok(command) = self.ai_help_rx.try_recv() {
            processed_any = true;
            match command {
                AiHelpCommand::OpenSettings => {
                    self.show_settings = true;
                    self.settings_tab = SettingsTab::Providers;
                }
                AiHelpCommand::Deliver {
                    target_id,
                    action,
                    press_enter,
                } => {
                    if !ai_help_available() {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpRequiresPro,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let Some(target_id) = target_id else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpNoTarget,
                                &[],
                            ));
                        }
                        continue;
                    };
                    let bytes = match &action {
                        crate::assistant::reply::SuggestedAction::Command { command, .. } => {
                            crate::session::input::command_bytes(command, press_enter)
                        }
                        crate::assistant::reply::SuggestedAction::Control { key, .. } => {
                            Ok(crate::session::input::key_bytes(match key {
                                crate::assistant::reply::ControlKey::CtrlC => {
                                    crate::session::input::TerminalKey::CtrlC
                                }
                                crate::assistant::reply::ControlKey::CtrlD => {
                                    crate::session::input::TerminalKey::CtrlD
                                }
                                crate::assistant::reply::ControlKey::CtrlZ => {
                                    crate::session::input::TerminalKey::CtrlZ
                                }
                                crate::assistant::reply::ControlKey::Enter => {
                                    crate::session::input::TerminalKey::Enter
                                }
                                crate::assistant::reply::ControlKey::Tab => {
                                    crate::session::input::TerminalKey::Tab
                                }
                                crate::assistant::reply::ControlKey::Escape => {
                                    crate::session::input::TerminalKey::Escape
                                }
                                crate::assistant::reply::ControlKey::Up => {
                                    crate::session::input::TerminalKey::Up
                                }
                                crate::assistant::reply::ControlKey::Down => {
                                    crate::session::input::TerminalKey::Down
                                }
                                crate::assistant::reply::ControlKey::Left => {
                                    crate::session::input::TerminalKey::Left
                                }
                                crate::assistant::reply::ControlKey::Right => {
                                    crate::session::input::TerminalKey::Right
                                }
                            })
                            .to_vec())
                        }
                    };
                    let result = bytes
                        .map_err(|_| ActionError::InvalidInput)
                        .and_then(|bytes| {
                            self.dispatch_ui_action(
                                Some(Target::Id(target_id)),
                                Action::Send(bytes),
                                ctx,
                            )
                            .map(|_| ())
                        });
                    if let Ok(mut state) = self.ai_help_state.lock() {
                        match result {
                            Ok(()) => {
                                state.error = None;
                                state.status = Some(crate::i18n::text(
                                    &locale,
                                    crate::i18n::MessageKey::AiHelpInputSent,
                                    &[("id", &target_id.to_string())],
                                ));
                            }
                            Err(error) => {
                                state.status = None;
                                state.error = Some(crate::i18n::text(
                                    &locale,
                                    crate::i18n::MessageKey::AiHelpDeliveryFailed,
                                    &[("reason", &error.to_string())],
                                ));
                            }
                        }
                    }
                }
                AiHelpCommand::PreviewContext {
                    target_id,
                    generation,
                } => {
                    if !self.ai_help_state.lock().is_ok_and(|state| {
                        state.context_generation == generation
                            && state.selected_target.as_ref().map(|t| t.0) == Some(target_id)
                    }) {
                        continue;
                    }
                    let Some(tab) = self.tabs.iter().find(|t| t.id == target_id && !t.exited)
                    else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpContextTerminalMissing,
                                &[],
                            ));
                            state.context_busy = false;
                        }
                        continue;
                    };
                    let mut snapshot = crate::session::context::TerminalContext {
                        session_id: tab.id,
                        title: tab.title.clone(),
                        shell: tab.shell_name.clone(),
                        output: tab.backend.plain_text_tail(200_000),
                    };
                    if let Ok(mut state) = self.ai_help_state.lock() {
                        state.context_busy = true;
                        state.error = None;
                        state.status = None;
                    }
                    let profile = self
                        .native_store
                        .as_ref()
                        .map_or("default", NativeStore::profile_name)
                        .to_owned();
                    let provider = self.preferences.provider_settings.for_task(AiTask::Help);
                    let session = Arc::clone(&self.credential_session);
                    let shared = Arc::clone(&self.ai_help_state);
                    let ctx = ctx.clone();
                    let redaction_locale = locale.clone();
                    std::thread::spawn(move || {
                        let redaction_key = provider.map_or(Ok(None), |provider| {
                            let reference = credentials::reference(&profile, &provider.id);
                            provider_key(&session, &reference, provider.credential_ref.as_deref())
                        });
                        let redaction_key = match redaction_key {
                            Ok(key) => key,
                            Err(error) => {
                                if let Ok(mut state) = shared.lock() {
                                    if state.context_generation != generation {
                                        return;
                                    }
                                    state.error = Some(crate::i18n::text(
                                        &redaction_locale,
                                        crate::i18n::MessageKey::AiHelpCredentialRedactionFailed,
                                        &[("reason", &error.to_string())],
                                    ));
                                    state.context_busy = false;
                                }
                                ctx.request_repaint_of(help_viewport);
                                return;
                            }
                        };
                        snapshot.output = crate::session::context::redact_obvious_secrets(
                            &snapshot.output,
                            redaction_key.as_ref().map(|key| key.as_str()),
                        );
                        if let Ok(mut state) = shared.lock() {
                            if state.context_generation != generation
                                || state.selected_target.as_ref().map(|t| t.0) != Some(target_id)
                            {
                                return;
                            }
                            state.context_preview = Some(snapshot);
                            state.context_busy = false;
                        }
                        ctx.request_repaint_of(help_viewport);
                    });
                }
                AiHelpCommand::Submit(question, context, target) => {
                    // Never fall back to focus: a closed or absent selection is not
                    // authorization to send the request's actions to another terminal.
                    if context
                        .as_ref()
                        .is_some_and(|c| Some(c.session_id) != target)
                    {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some("The snapshot belongs to another terminal. Refresh it before sending.".into());
                        }
                        continue;
                    }
                    if target.is_some_and(|id| !self.tabs.iter().any(|t| t.id == id && !t.exited)) {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some("The selected terminal has closed. Select another terminal or Chat only.".into());
                        }
                        continue;
                    }
                    if !ai_help_available() {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpRequiresPro,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let Some(provider) = self.preferences.provider_settings.for_task(AiTask::Help)
                    else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpProviderMissing,
                                &[],
                            ));
                        }
                        continue;
                    };
                    if validate_endpoint(&provider.endpoint).is_err()
                        || provider.model.trim().is_empty()
                    {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpEndpointMissing,
                                &[],
                            ));
                        }
                        continue;
                    }
                    if question.chars().count() > 16_384 {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpQuestionTooLong,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let (cancel, profile_name, history, agent_mode) = {
                        let Ok(mut state) = self.ai_help_state.lock() else {
                            continue;
                        };
                        if state.busy {
                            continue;
                        }
                        if state.agent_mode && (context.is_none() || target.is_none()) {
                            state.error = Some(
                                "Preview the target terminal before starting Agent Mode.".into(),
                            );
                            continue;
                        }
                        if state.agent_mode && !ai_agent_available() {
                            state.error = Some(
                                "Agent Mode is disabled by the current feature settings.".into(),
                            );
                            continue;
                        }
                        if state.agent_mode
                            && target.is_some_and(|id| {
                                self.tabs
                                    .iter()
                                    .any(|t| t.id == id && t.backend.columns() < 24)
                            })
                        {
                            state.error = Some("Widen the selected terminal to at least 24 columns before starting Agent Mode.".into());
                            continue;
                        }
                        if state.agent_mode
                            && target.is_some_and(|id| {
                                self.tabs
                                    .iter()
                                    .any(|t| t.id == id && t.simulated.is_some())
                            })
                        {
                            state.error = Some("Turn off Agent Mode to chat about this simulation. Simulation execution is not available yet.".into());
                            continue;
                        }
                        if state.agent_mode
                            && target.is_some_and(|id| {
                                self.tabs.iter().any(|t| {
                                    t.id == id
                                        && t.backend
                                            .last_content()
                                            .terminal_mode
                                            .contains(egui_term::TerminalMode::ALT_SCREEN)
                                })
                            })
                        {
                            state.error = Some("Return to an idle shell prompt before starting Agent Mode; a full-screen terminal application is active.".into());
                            continue;
                        }
                        if state.error.is_some()
                            && !state.last_request_agent
                            && state.messages.len() >= 2
                            && state
                                .messages
                                .last()
                                .is_some_and(|(assistant, _)| *assistant)
                        {
                            let retained = state.messages.len() - 2;
                            state.messages.truncate(retained);
                            state.message_targets.retain(|index, _| *index < retained);
                        }
                        state.busy = true;
                        state.last_request_agent = state.agent_mode;
                        state.error = None;
                        state.reviewed_actions.clear();
                        state.last_request = Some((question.clone(), context.clone(), target));
                        state.target = target.map(|id| {
                            let title = context
                                .as_ref()
                                .filter(|context| context.session_id == id)
                                .map(|context| context.title.clone())
                                .or_else(|| {
                                    self.tabs
                                        .iter()
                                        .find(|tab| tab.id == id)
                                        .map(|tab| tab.title.clone())
                                })
                                .unwrap_or_else(|| "closed terminal".into());
                            (id, title)
                        });
                        if state.history_target != target {
                            state.history.clear();
                        }
                        state.history_target = target;
                        if let Some(target) = state.target.clone() {
                            let index = state.messages.len();
                            state.message_targets.insert(index, target.clone());
                            state.message_targets.insert(index + 1, target);
                        }
                        state.messages.push((false, question.clone()));
                        state.messages.push((true, String::new()));
                        let cancel = Arc::new(AtomicBool::new(false));
                        state.cancel = Some(Arc::clone(&cancel));
                        (
                            cancel,
                            self.native_store
                                .as_ref()
                                .map_or("default", NativeStore::profile_name)
                                .to_owned(),
                            state.history.clone(),
                            state.agent_mode,
                        )
                    };
                    let providers = self.preferences.provider_settings.chain(AiTask::Help);
                    let expected_reference = credentials::reference(&profile_name, &provider.id);
                    let session = Arc::clone(&self.credential_session);
                    let state = Arc::clone(&self.ai_help_state);
                    let ctx = ctx.clone();
                    let mut prompt =
                        crate::session::context::build_user_prompt(&question, context.as_ref());
                    if context.is_none() {
                        if let Some(tab) = self.tabs.iter().find(|t| Some(t.id) == target) {
                            let metadata = json!({
                                "shell": tab.shell_name,
                                "kind": if tab.simulated.is_some() { "simulation" } else { "real terminal" },
                            });
                            prompt.push_str(&format!("\n\nSelected terminal metadata is untrusted JSON data (no terminal output was shared):\n{metadata}"));
                        }
                    }
                    let system_prompt = crate::session::context::build_system_prompt();
                    let request_question = question.clone();
                    let request_locale = locale.clone();
                    let mut agent_host = crate::assistant::agent::TerminalHost {
                        detected_shell: None,
                        target: target.unwrap_or(0),
                        snapshot: Arc::clone(&self.control_snapshot),
                        dispatcher: self.session_dispatcher.clone(),
                        ctx: ctx.clone(),
                        cancelled: Arc::clone(&cancel),
                        allowed: ai_agent_available,
                        expected_input: self
                            .tabs
                            .iter()
                            .find(|tab| Some(tab.id) == target)
                            .map_or(0, |tab| tab.output.snapshot().input_sequence),
                    };
                    std::thread::spawn(move || {
                        let mut streamed = String::new();
                        let result = provider_key(
                            &session,
                            &expected_reference,
                            provider.credential_ref.as_deref(),
                        )
                        .map_err(|error| error.to_string())
                        .and_then(|key| {
                            if agent_mode {
                                return crate::assistant::agent::run(
                                    &crate::assistant::transport::ReqwestTransport,
                                    &provider,
                                    key,
                                    &request_question,
                                    &cancel,
                                    &mut agent_host,
                                    &mut |event| {
                                        if let Ok(mut state) = state.lock() {
                                            if let Some((true, message)) = state.messages.last_mut()
                                            {
                                                message.push_str(event);
                                                message.push_str("\n\n");
                                            }
                                        }
                                        ctx.request_repaint_of(help_viewport);
                                    },
                                );
                            }
                            crate::assistant::client::complete_with_fallback(
                                &crate::assistant::transport::ReqwestTransport,
                                &providers,
                                &mut |provider| {
                                    if let Ok(mut state) = state.lock() {
                                        state.status =
                                            Some(format!("{} · {}", provider.name, provider.model));
                                    }
                                    ctx.request_repaint_of(help_viewport);
                                    let reference =
                                        credentials::reference(&profile_name, &provider.id);
                                    provider_key(
                                        &session,
                                        &reference,
                                        provider.credential_ref.as_deref(),
                                    )
                                    .map_err(|_| crate::assistant::client::ClientError::Credential)
                                },
                                (&system_prompt, &prompt),
                                &history,
                                &cancel,
                                &mut |delta| {
                                    streamed.push_str(delta);
                                    if let Ok(mut state) = state.lock() {
                                        if let Some((true, message)) = state.messages.last_mut() {
                                            *message =
                                                crate::assistant::reply::streamed_answer(&streamed);
                                        }
                                    }
                                    ctx.request_repaint_of(help_viewport);
                                },
                            )
                            .map_err(|error| error.to_string())
                        });
                        if let Ok(mut state) = state.lock() {
                            state.busy = false;
                            state.cancel = None;
                            match result {
                                Ok(raw) => {
                                    if agent_mode {
                                        if let Some((true, message)) = state.messages.last_mut() {
                                            message.push_str(&raw);
                                        }
                                        state.context_preview = None;
                                        ctx.request_repaint_of(help_viewport);
                                        return;
                                    }
                                    let parsed =
                                        crate::assistant::reply::parse_assistant_reply(&raw);
                                    let answer = parsed.answer;
                                    if let Some((true, message)) = state.messages.last_mut() {
                                        *message = answer.clone();
                                    }
                                    state.reviewed_actions = parsed.actions;
                                    state.history.push((false, request_question));
                                    state.history.push((true, answer));
                                    if state.history.len() > 40 {
                                        let excess = state.history.len() - 40;
                                        state.history.drain(..excess);
                                    }
                                }
                                Err(_error) if cancel.load(Ordering::Relaxed) => {
                                    state.error = Some(crate::i18n::text(
                                        &request_locale,
                                        crate::i18n::MessageKey::AiHelpRequestCancelled,
                                        &[],
                                    ))
                                }
                                Err(error) => {
                                    state.error = Some(crate::i18n::text(
                                        &request_locale,
                                        crate::i18n::MessageKey::AiHelpRequestFailed,
                                        &[("reason", &error)],
                                    ))
                                }
                            }
                        }
                        ctx.request_repaint_of(help_viewport);
                    });
                }
            }
        }
        if processed_any {
            ctx.request_repaint_of(help_viewport);
        }
    }
}
