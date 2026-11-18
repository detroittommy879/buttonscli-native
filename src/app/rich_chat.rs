//! View → AI chat (local flag `rich_chat`): chat without a terminal, where
//! the model may style its words with a tiny markup language.
//!
//! Safety model: the reply is only ever *data*. The parser below accepts a
//! fixed list of attributes with clamped values (colors, sizes, known font
//! families, a few gentle motions); anything else is shown as plain text.
//! There is no HTML, no links, no images, no file or network access and no
//! code execution, and every glyph is pushed to a minimum contrast against
//! the chat background, so a reply can be ugly but never unreadable or
//! harmful.

use super::*;
use egui::text::{LayoutJob, TextFormat};
use std::sync::Mutex;

/// Contrast every styled glyph must keep against the chat background.
const MIN_CONTRAST: f32 = 4.5;
const MAX_REPLY_CHARS: usize = 40_000;
const MAX_SPANS: usize = 4_000;
const MAX_DEPTH: usize = 6;
const SIZE_RANGE: std::ops::RangeInclusive<f32> = 9.0..=40.0;

pub(super) fn available() -> bool {
    crate::features::local::rich_chat_enabled()
}

/// The markup the model is told about.
const SYSTEM_PROMPT: &str = "You are a friendly, concise assistant in a chat window that can show styled text. \
Write normal text, and when it helps, style words with tags: [[attributes]]text[[/]]. Tags can nest. Attributes, space separated: \
color=#rrggbb, bg=#rrggbb (highlight), size=9..40 (points, normal is 15), font=sans|mono|serif or another font name, \
italic, underline, strike, gradient=#rrggbb,#rrggbb[,#rrggbb] (colors across the letters), \
motion=pulse|shimmer|breathe (gentle movement; use rarely). \
Example: [[size=22 gradient=#ff7a45,#ffd23f]]Big idea[[/]] then [[color=#7fd1ff italic]]a calm note[[/]]. \
The background is near black; pick colors that read well on it. Style for meaning and emphasis, not on every word. \
Use plain line breaks for structure; no Markdown, HTML or links.";

/// One piece of text with its style.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Span {
    pub(super) text: String,
    pub(super) style: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Motion {
    #[default]
    None,
    /// Size swells slightly and back.
    Pulse,
    /// The gradient slides along the word.
    Shimmer,
    /// Opacity breathes between 70% and 100%.
    Breathe,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Style {
    pub(super) color: Option<Color32>,
    pub(super) background: Option<Color32>,
    pub(super) size: Option<f32>,
    pub(super) font: Option<String>,
    pub(super) italic: bool,
    pub(super) underline: bool,
    pub(super) strike: bool,
    pub(super) gradient: Vec<Color32>,
    pub(super) motion: Motion,
}

impl Style {
    /// Applies one tag's attributes on top of the enclosing style. Unknown
    /// or invalid attributes are ignored.
    fn with(&self, attributes: &str) -> Self {
        let mut style = self.clone();
        for attribute in attributes.split_whitespace().take(16) {
            let (key, value) = attribute.split_once('=').unwrap_or((attribute, ""));
            let color = || crate::theme::parse_color(value);
            match key.to_ascii_lowercase().as_str() {
                "color" | "c" => style.color = color().or(style.color),
                "bg" | "background" => style.background = color().or(style.background),
                "size" | "s" => {
                    if let Ok(size) = value.parse::<f32>() {
                        if size.is_finite() {
                            style.size = Some(size.clamp(*SIZE_RANGE.start(), *SIZE_RANGE.end()));
                        }
                    }
                }
                "font" | "f" if !value.is_empty() && value.len() <= 64 => {
                    style.font = Some(value.to_owned());
                }
                "italic" | "i" => style.italic = true,
                "underline" | "u" => style.underline = true,
                "strike" => style.strike = true,
                "gradient" | "g" => {
                    let colors: Vec<_> = value
                        .split(',')
                        .take(6)
                        .filter_map(crate::theme::parse_color)
                        .collect();
                    if colors.len() >= 2 {
                        style.gradient = colors;
                    }
                }
                "motion" | "m" => {
                    style.motion = match value {
                        "pulse" => Motion::Pulse,
                        "shimmer" => Motion::Shimmer,
                        "breathe" => Motion::Breathe,
                        _ => style.motion,
                    }
                }
                _ => {}
            }
        }
        style
    }
}

/// Splits a reply into styled spans. Never fails: anything that is not a
/// well-formed tag stays visible as text.
pub(super) fn parse(markup: &str) -> Vec<Span> {
    let markup = match markup.char_indices().nth(MAX_REPLY_CHARS) {
        Some((cut, _)) => &markup[..cut],
        None => markup,
    };
    let mut spans: Vec<Span> = Vec::new();
    let mut stack = vec![Style::default()];
    let mut rest = markup;
    let push = |spans: &mut Vec<Span>, text: &str, style: &Style| {
        if text.is_empty() || spans.len() >= MAX_SPANS {
            return;
        }
        match spans.last_mut() {
            Some(last) if last.style == *style => last.text.push_str(text),
            _ => spans.push(Span {
                text: text.to_owned(),
                style: style.clone(),
            }),
        }
    };
    while let Some(open) = rest.find("[[") {
        let style = stack.last().cloned().unwrap_or_default();
        push(&mut spans, &rest[..open], &style);
        let after = &rest[open + 2..];
        let Some(close) = after.find("]]").filter(|close| *close <= 400) else {
            push(&mut spans, "[[", &style);
            rest = after;
            continue;
        };
        let tag = after[..close].trim();
        if tag == "/" {
            if stack.len() > 1 {
                stack.pop();
            }
        } else if stack.len() <= MAX_DEPTH && !tag.contains('[') {
            stack.push(style.with(tag));
        } else {
            push(&mut spans, &rest[open..open + 2 + close + 2], &style);
        }
        rest = &after[close + 2..];
    }
    let style = stack.last().cloned().unwrap_or_default();
    push(&mut spans, rest, &style);
    spans
}

fn gradient_at(colors: &[Color32], t: f32) -> Color32 {
    let t = t.rem_euclid(1.0) * (colors.len() - 1) as f32;
    let index = (t.floor() as usize).min(colors.len() - 2);
    let local = t - index as f32;
    let [a, b] = [colors[index], colors[index + 1]];
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * local).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// Lays the spans out for egui. `time` drives motion (None when Calm).
pub(super) fn layout(
    spans: &[Span],
    base: &TextFormat,
    background: Color32,
    families: &[egui::FontFamily],
    time: Option<f64>,
    width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    for span in spans {
        let style = &span.style;
        let mut format = base.clone();
        let size = style.size.unwrap_or(base.font_id.size);
        let pulse = match (style.motion, time) {
            (Motion::Pulse, Some(t)) => 1.0 + 0.08 * ((t * 2.0).sin() as f32),
            _ => 1.0,
        };
        format.font_id.size = size * pulse;
        if let Some(font) = &style.font {
            format.font_id.family = match font.to_ascii_lowercase().as_str() {
                "mono" | "monospace" | "code" => egui::FontFamily::Monospace,
                "sans" | "serif" | "proportional" => egui::FontFamily::Proportional,
                _ => families
                    .iter()
                    .find(|family| matches!(family, egui::FontFamily::Name(name) if name.eq_ignore_ascii_case(font)))
                    .cloned()
                    .unwrap_or(egui::FontFamily::Proportional),
            };
        }
        format.italics = style.italic;
        if style.underline {
            format.underline = Stroke::new(1.0_f32, format.color);
        }
        if style.strike {
            format.strikethrough = Stroke::new(1.0_f32, format.color);
        }
        let behind = style.background.unwrap_or(background);
        if let Some(highlight) = style.background {
            format.background = highlight;
        }
        let alpha = match (style.motion, time) {
            (Motion::Breathe, Some(t)) => 0.85 + 0.15 * ((t * 1.5).sin() as f32),
            _ => 1.0,
        };
        let readable = |color: Color32| {
            let color = egui_term::ensure_contrast(color, behind, MIN_CONTRAST);
            if alpha < 1.0 {
                color.gamma_multiply(alpha)
            } else {
                color
            }
        };
        if style.gradient.len() >= 2 {
            let count = span.text.chars().count().max(2) as f32;
            let shift = match (style.motion, time) {
                (Motion::Shimmer, Some(t)) => (t * 0.25).fract() as f32,
                _ => 0.0,
            };
            for (index, ch) in span.text.chars().enumerate() {
                let t = if shift > 0.0 {
                    // Back and forth, so the colors loop smoothly.
                    let x = index as f32 / count + shift;
                    1.0 - (2.0 * x.rem_euclid(1.0) - 1.0).abs()
                } else {
                    index as f32 / (count - 1.0)
                };
                let mut glyph = format.clone();
                glyph.color = readable(gradient_at(&style.gradient, t.min(0.9999)));
                let mut buffer = [0u8; 4];
                job.append(ch.encode_utf8(&mut buffer), 0.0, glyph);
            }
        } else {
            format.color = readable(style.color.unwrap_or(base.color));
            job.append(&span.text, 0.0, format);
        }
    }
    job
}

/// Chat transcript and worker status, shared with the request thread.
#[derive(Default)]
struct ChatState {
    open: bool,
    /// (from the assistant, text).
    messages: Vec<(bool, String)>,
    draft: String,
    busy: bool,
    status: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    show_markup: bool,
}

type SharedChat = Arc<Mutex<ChatState>>;

fn chat_id() -> egui::Id {
    egui::Id::new("rich-chat-state")
}

fn shared(ctx: &egui::Context) -> SharedChat {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with(chat_id(), || Arc::new(Mutex::new(ChatState::default())))
            .clone()
    })
}

pub(super) fn open(ctx: &egui::Context) {
    if let Ok(mut state) = shared(ctx).lock() {
        state.open = true;
    }
}

const CHAT_BACKGROUND: Color32 = Color32::from_rgb(14, 14, 18);

impl ButtonsApp {
    fn send_rich_chat(&mut self, ctx: &egui::Context, chat: &SharedChat, prompt: String) {
        let providers = self.preferences.provider_settings.chain(AiTask::Help);
        let Ok(mut state) = chat.lock() else { return };
        if providers.is_empty() {
            state.status = Some(crate::i18n::literal(
                &self.locale,
                "Choose an AI provider in Settings → Providers first.",
            ));
            return;
        }
        let history: Vec<(bool, String)> = state
            .messages
            .iter()
            .rev()
            .take(20)
            .rev()
            .cloned()
            .collect();
        state.messages.push((false, prompt.clone()));
        state.messages.push((true, String::new()));
        state.busy = true;
        let cancel = Arc::new(AtomicBool::new(false));
        state.cancel = Some(Arc::clone(&cancel));
        drop(state);
        let profile = self
            .native_store
            .as_ref()
            .map_or("default", NativeStore::profile_name)
            .to_owned();
        let session = Arc::clone(&self.credential_session);
        let chat = Arc::clone(chat);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut streamed = String::new();
            let result = crate::assistant::client::complete_with_fallback(
                &crate::assistant::transport::ReqwestTransport,
                &providers,
                &mut |provider| {
                    if let Ok(mut state) = chat.lock() {
                        state.status = Some(format!("{} · {}", provider.name, provider.model));
                    }
                    let reference = credentials::reference(&profile, &provider.id);
                    provider_key(&session, &reference, provider.credential_ref.as_deref())
                        .map_err(|_| crate::assistant::client::ClientError::Credential)
                },
                (SYSTEM_PROMPT, &prompt),
                &history,
                &cancel,
                &mut |delta| {
                    streamed.push_str(delta);
                    if let Ok(mut state) = chat.lock() {
                        if let Some((true, message)) = state.messages.last_mut() {
                            message.clone_from(&streamed);
                        }
                    }
                    ctx.request_repaint();
                },
            );
            if let Ok(mut state) = chat.lock() {
                state.busy = false;
                state.cancel = None;
                match result {
                    Ok(answer) => {
                        if let Some((true, message)) = state.messages.last_mut() {
                            *message = answer;
                        }
                    }
                    Err(error) => state.status = Some(error.to_string()),
                }
            }
            ctx.request_repaint();
        });
    }

    pub(super) fn rich_chat_window(&mut self, ctx: &egui::Context) {
        if !available() {
            return;
        }
        let chat = shared(ctx);
        if !chat.lock().is_ok_and(|state| state.open) {
            return;
        }
        let locale = self.locale.clone();
        let calm = self.preferences.calm_mode;
        let mut send = None;
        let builder = egui::ViewportBuilder::default()
            .with_title(crate::i18n::literal(&locale, "ButtonsCLI Chat"))
            .with_inner_size([620.0, 720.0])
            .with_min_inner_size([360.0, 320.0])
            .with_icon(crate::app_icon());
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-rich-chat"),
            builder,
            |child, _class| {
                let Ok(mut state) = chat.lock() else { return };
                if child.input(|input| input.viewport().close_requested()) {
                    state.open = false;
                    return;
                }
                let now = child.input(|input| input.time);
                let families: Vec<egui::FontFamily> = child.fonts(|fonts| fonts.families());
                // The chat keeps its own fixed dark background (replies
                // choose colors for it). Never `set_visuals` here: the style
                // is shared with the main window.
                egui::TopBottomPanel::bottom("rich-chat-input")
                    .frame(egui::Frame::NONE.fill(CHAT_BACKGROUND).inner_margin(10.0))
                    .show(child, |ui| {
                        if let Some(status) = &state.status {
                            ui.small(status.as_str());
                        }
                        let input = ui.add(
                            egui::TextEdit::multiline(&mut state.draft)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .hint_text(crate::i18n::literal(
                                    &locale,
                                    "Message (Enter sends, Shift+Enter new line)",
                                )),
                        );
                        let enter = input.has_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                        ui.horizontal(|ui| {
                            if state.busy {
                                if crate::ui_text::safe_button(
                                    ui,
                                    crate::i18n::literal(&locale, "Stop"),
                                )
                                .clicked()
                                {
                                    if let Some(cancel) = &state.cancel {
                                        cancel.store(true, Ordering::Relaxed);
                                    }
                                }
                            } else if (ui.button(crate::i18n::literal(&locale, "Send")).clicked()
                                || enter)
                                && !state.draft.trim().is_empty()
                            {
                                send = Some(std::mem::take(&mut state.draft).trim().to_owned());
                            }
                            ui.checkbox(
                                &mut state.show_markup,
                                crate::i18n::literal(&locale, "Show markup"),
                            );
                            if !state.busy
                                && crate::ui_text::safe_button(
                                    ui,
                                    crate::i18n::literal(&locale, "Clear"),
                                )
                                .clicked()
                            {
                                state.messages.clear();
                                state.status = None;
                            }
                        });
                    });
                let mut animated = false;
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(CHAT_BACKGROUND).inner_margin(12.0))
                    .show(child, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let width = ui.available_width();
                                let base = TextFormat {
                                    font_id: egui::FontId::proportional(15.0),
                                    color: Color32::from_rgb(232, 232, 236),
                                    ..TextFormat::default()
                                };
                                for (assistant, text) in &state.messages {
                                    if !*assistant {
                                        egui::Frame::NONE
                                            .fill(Color32::from_rgb(34, 36, 46))
                                            .corner_radius(8.0)
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(text)
                                                        .color(base.color)
                                                        .size(15.0),
                                                );
                                            });
                                    } else if state.show_markup {
                                        ui.label(RichText::new(text).monospace().color(base.color));
                                    } else {
                                        let spans = parse(text);
                                        let moving = !calm
                                            && spans
                                                .iter()
                                                .any(|span| span.style.motion != Motion::None);
                                        animated |= moving;
                                        let job = layout(
                                            &spans,
                                            &base,
                                            CHAT_BACKGROUND,
                                            &families,
                                            moving.then_some(now),
                                            width,
                                        );
                                        ui.label(job);
                                    }
                                    ui.add_space(10.0);
                                }
                            });
                    });
                if animated {
                    child.request_repaint_after(std::time::Duration::from_millis(33));
                }
            },
        );
        if let Some(prompt) = send {
            self.send_rich_chat(ctx, &chat, prompt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_styles_nest_and_bad_tags_stay_visible() {
        let spans = parse("Hi [[color=#ff0000 size=22]]big [[italic]]red[[/]][[/]] end");
        let texts: Vec<_> = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(texts, ["Hi ", "big ", "red", " end"]);
        assert_eq!(spans[1].style.color, Some(Color32::RED));
        assert_eq!(spans[1].style.size, Some(22.0));
        assert!(spans[2].style.italic && spans[2].style.color == Some(Color32::RED));
        assert_eq!(spans[3].style, Style::default());
        // Unclosed brackets, unknown attributes and absurd values are harmless.
        let spans = parse("a [[ b");
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "a [[ b"
        );
        let style =
            Style::default().with("size=9999 color=javascript:alert(1) onclick=x gradient=#fff");
        assert_eq!(style.size, Some(*SIZE_RANGE.end()));
        assert_eq!(style.color, None);
        assert!(style.gradient.is_empty());
        // Deep nesting stops adding styles but never drops text.
        let deep = "[[i]]".repeat(20) + "x" + &"[[/]]".repeat(20);
        assert_eq!(
            parse(&deep)
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "[[i]]".repeat(20 - MAX_DEPTH) + "x"
        );
    }

    #[test]
    fn layout_keeps_every_glyph_readable_on_the_chat_background() {
        let spans = parse("[[color=#101010]]dark[[/]] [[gradient=#000000,#202020]]grad[[/]]");
        let base = TextFormat {
            font_id: egui::FontId::proportional(15.0),
            color: Color32::WHITE,
            ..TextFormat::default()
        };
        let job = layout(&spans, &base, CHAT_BACKGROUND, &[], None, 300.0);
        for section in &job.sections {
            let color = section.format.color;
            assert!(
                egui_term::contrast_ratio(color, CHAT_BACKGROUND) >= MIN_CONTRAST - 0.05,
                "{color:?}"
            );
        }
    }
}
