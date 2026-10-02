pub mod settings;

use crate::types::Size;
use alacritty_terminal::event::{
    Event, EventListener, Notify, OnResize, WindowSize,
};
use alacritty_terminal::event_loop::{
    EventLoop, Msg, Notifier, SensitiveInput,
};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{
    Selection, SelectionRange, SelectionType as AlacrittySelectionType,
};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::search::{Match, RegexIter, RegexSearch};
use alacritty_terminal::term::{
    self, cell::Cell, test::TermSize, viewport_to_point, Term, TermMode,
};
use alacritty_terminal::vte::ansi::Color;
use alacritty_terminal::{tty, Grid};
use egui::Modifiers;
use settings::BackendSettings;
use std::borrow::Cow;
use std::cmp::min;
use std::collections::HashSet;
use std::io::Result;
use std::ops::{Index, RangeInclusive};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};

pub type TerminalMode = TermMode;
pub type PtyEvent = Event;
pub type SelectionType = AlacrittySelectionType;
pub type ByteObserver = Arc<dyn Fn(&[u8]) + Send + Sync + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollbackState {
    pub history_lines: usize,
    pub viewport_lines: usize,
    /// Zero is the live bottom; `history_lines` is the oldest retained line.
    pub display_offset: usize,
    pub mouse_reporting: bool,
    pub alternate_screen: bool,
}

impl ScrollbackState {
    fn from_content(content: &RenderableContent) -> Self {
        Self {
            history_lines: content.grid.history_size(),
            viewport_lines: content.grid.screen_lines(),
            display_offset: content.grid.display_offset(),
            mouse_reporting: content
                .terminal_mode
                .intersects(TermMode::MOUSE_MODE),
            alternate_screen: content
                .terminal_mode
                .contains(TermMode::ALT_SCREEN),
        }
    }

    pub fn available(self) -> bool {
        self.history_lines > 0
            && !self.alternate_screen
            && !self.mouse_reporting
    }
}

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Write(Vec<u8>),
    WriteSensitive(SensitiveInput),
    Scroll(i32),
    Resize(Size, Size),
    SelectStart(SelectionType, f32, f32),
    SelectUpdate(f32, f32),
    ProcessLink(LinkAction, Point),
    MouseReport(MouseButton, Modifiers, Point, bool),
}

#[derive(Debug, Clone)]
pub enum MouseMode {
    Sgr,
    Normal(bool),
}

impl From<TermMode> for MouseMode {
    fn from(term_mode: TermMode) -> Self {
        if term_mode.contains(TermMode::SGR_MOUSE) {
            MouseMode::Sgr
        } else if term_mode.contains(TermMode::UTF8_MOUSE) {
            MouseMode::Normal(true)
        } else {
            MouseMode::Normal(false)
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    LeftButton = 0,
    MiddleButton = 1,
    RightButton = 2,
    LeftMove = 32,
    MiddleMove = 33,
    RightMove = 34,
    NoneMove = 35,
    ScrollUp = 64,
    ScrollDown = 65,
    Other = 99,
}

#[derive(Debug, Clone)]
pub enum LinkAction {
    Clear,
    Hover,
    Open,
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cell_width: u16,
    pub cell_height: u16,
    num_cols: u16,
    num_lines: u16,
    layout_size: Size,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cell_width: 1,
            cell_height: 1,
            num_cols: 80,
            num_lines: 50,
            layout_size: Size::default(),
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines as usize
    }

    fn columns(&self) -> usize {
        self.num_cols as usize
    }

    fn last_column(&self) -> Column {
        Column(self.num_cols as usize - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.num_lines as i32 - 1)
    }
}

impl From<TerminalSize> for WindowSize {
    fn from(size: TerminalSize) -> Self {
        Self {
            num_lines: size.num_lines,
            num_cols: size.num_cols,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
        }
    }
}

pub struct TerminalBackend {
    pub id: u64,
    pub url_regex: RegexSearch,
    term: Arc<FairMutex<Term<EventProxy>>>,
    size: TerminalSize,
    notifier: Notifier,
    #[cfg(windows)]
    child_process_id: Option<std::num::NonZeroU32>,
    last_content: RenderableContent,
    input_observer: Option<ByteObserver>,
    search_dirty: Arc<AtomicBool>,
    search_query: Option<String>,
    search_matches: Vec<Match>,
    current_search_match: Option<usize>,
}

impl TerminalBackend {
    pub fn new(
        id: u64,
        app_context: egui::Context,
        pty_event_proxy_sender: Sender<(u64, PtyEvent)>,
        settings: BackendSettings,
    ) -> Result<Self> {
        Self::new_with_observers(
            id,
            app_context,
            pty_event_proxy_sender,
            settings,
            None,
            None,
        )
    }

    /// Attach lightweight callbacks to input writes and the existing PTY read loop.
    pub fn new_with_observers(
        id: u64,
        app_context: egui::Context,
        pty_event_proxy_sender: Sender<(u64, PtyEvent)>,
        settings: BackendSettings,
        input_observer: Option<ByteObserver>,
        output_observer: Option<ByteObserver>,
    ) -> Result<Self> {
        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(settings.shell, settings.args)),
            working_directory: settings.working_directory,
            ..tty::Options::default()
        };
        let config = term::Config::default();
        let terminal_size = TerminalSize::default();
        let pty = tty::new(&pty_config, terminal_size.into(), id)?;
        #[cfg(windows)]
        let child_process_id = pty.child_watcher().pid();
        let search_dirty = Arc::new(AtomicBool::new(true));
        let output_observer = {
            let search_dirty = Arc::clone(&search_dirty);
            let output_observer = output_observer.clone();
            Some(Arc::new(move |bytes: &[u8]| {
                search_dirty.store(true, Ordering::Relaxed);
                if let Some(observer) = &output_observer {
                    observer(bytes);
                }
            }) as ByteObserver)
        };
        let (event_sender, event_receiver) = mpsc::channel();
        let event_proxy = EventProxy(event_sender);
        let mut term = Term::new(config, &terminal_size, event_proxy.clone());
        let initial_content = RenderableContent {
            grid: term.grid().clone(),
            selectable_range: None,
            terminal_mode: *term.mode(),
            terminal_size,
            cursor: term.grid_mut().cursor_cell().clone(),
            hovered_hyperlink: None,
            search_highlights: HashSet::new(),
            current_search_highlights: HashSet::new(),
        };
        let term = Arc::new(FairMutex::new(term));
        let pty_event_loop = EventLoop::new_with_output_observer(
            term.clone(),
            event_proxy,
            pty,
            false,
            false,
            output_observer,
        )?;
        let notifier = Notifier(pty_event_loop.channel());
        let url_regex = RegexSearch::new(r#"(ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\u{0000}-\u{001F}\u{007F}-\u{009F}<>"\s{-}\^⟨⟩`]+"#).unwrap();
        let _pty_event_loop_thread = pty_event_loop.spawn();
        let _pty_event_subscription = std::thread::Builder::new()
            .name(format!("pty_event_subscription_{}", id))
            .spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    if pty_event_proxy_sender.send((id, event.clone())).is_err()
                    {
                        break;
                    }
                    app_context.request_repaint();
                    if matches!(event, Event::Exit) {
                        break;
                    }
                }
            })?;

        Ok(Self {
            id,
            url_regex,
            term: term.clone(),
            size: terminal_size,
            notifier,
            #[cfg(windows)]
            child_process_id,
            last_content: initial_content,
            input_observer,
            search_dirty,
            search_query: None,
            search_matches: Vec::new(),
            current_search_match: None,
        })
    }

    pub fn process_command(&mut self, cmd: BackendCommand) {
        let term = self.term.clone();
        let mut term = term.lock();
        match cmd {
            BackendCommand::Write(input) => {
                if let Some(observer) = &self.input_observer {
                    observer(&input);
                }
                self.write(input);
                term.scroll_display(Scroll::Bottom);
            },
            BackendCommand::WriteSensitive(input) => {
                self.notifier.notify_sensitive(input);
                term.scroll_display(Scroll::Bottom);
            },
            BackendCommand::Scroll(delta) => {
                self.scroll(&mut term, delta);
            },
            BackendCommand::Resize(layout_size, font_size) => {
                self.resize(&mut term, layout_size, font_size);
            },
            BackendCommand::SelectStart(selection_type, x, y) => {
                self.start_selection(&mut term, selection_type, x, y);
            },
            BackendCommand::SelectUpdate(x, y) => {
                self.update_selection(&mut term, x, y);
            },
            BackendCommand::ProcessLink(link_action, point) => {
                self.process_link_action(&term, link_action, point);
            },
            BackendCommand::MouseReport(button, modifiers, point, pressed) => {
                self.process_mouse_report(button, modifiers, point, pressed);
            },
        };
    }

    pub fn selection_point(
        x: f32,
        y: f32,
        terminal_size: &TerminalSize,
        display_offset: usize,
    ) -> Point {
        let col = (x as usize) / (terminal_size.cell_width as usize);
        let col = min(Column(col), Column(terminal_size.num_cols as usize - 1));

        let line = (y as usize) / (terminal_size.cell_height as usize);
        let line = min(line, terminal_size.num_lines as usize - 1);

        viewport_to_point(display_offset, Point::new(line, col))
    }

    pub fn selectable_content(&self) -> String {
        // Alacritty preserves line breaks, wrapped lines and selections in history.
        self.term.lock().selection_to_string().unwrap_or_default()
    }

    /// Search this terminal's retained grid, including wrapped lines and scrollback.
    /// Results are cached until new PTY output arrives or another search is requested.
    pub fn search_next(
        &mut self,
        query: &str,
    ) -> std::result::Result<Option<(usize, usize)>, String> {
        self.search(query, true)
    }

    /// Move to the previous match in this terminal's retained grid.
    pub fn search_previous(
        &mut self,
        query: &str,
    ) -> std::result::Result<Option<(usize, usize)>, String> {
        self.search(query, false)
    }

    fn search(
        &mut self,
        query: &str,
        forward: bool,
    ) -> std::result::Result<Option<(usize, usize)>, String> {
        let query = query.trim();
        if query.is_empty() {
            self.clear_search();
            return Ok(None);
        }

        let query_changed = self.search_query.as_deref() != Some(query);
        let output_changed = self.search_dirty.swap(false, Ordering::Relaxed);
        if query_changed || output_changed {
            let term = self.term.clone();
            let terminal = term.lock();
            let matches = match collect_search_matches(&terminal, query) {
                Ok(matches) => matches,
                Err(error) => {
                    self.search_matches.clear();
                    self.current_search_match = None;
                    self.search_query = Some(query.to_owned());
                    self.last_content.search_highlights.clear();
                    self.last_content.current_search_highlights.clear();
                    return Err(error);
                },
            };
            self.search_matches = matches;
            self.current_search_match = None;
            self.search_query = Some(query.to_owned());
        }

        if self.search_matches.is_empty() {
            self.current_search_match = None;
            return Ok(None);
        }

        let count = self.search_matches.len();
        let index =
            next_search_index(self.current_search_match, count, forward);
        let term = self.term.clone();
        let mut terminal = term.lock();
        scroll_to_search_match(&mut terminal, &self.search_matches[index]);
        self.current_search_match = Some(index);
        Ok(Some((index + 1, count)))
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.current_search_match = None;
        self.search_dirty.store(false, Ordering::Relaxed);
        self.last_content.search_highlights.clear();
        self.last_content.current_search_highlights.clear();
    }

    pub fn search_status(&self) -> (usize, Option<usize>) {
        (
            self.search_matches.len(),
            self.current_search_match.map(|index| index + 1),
        )
    }

    /// Select every retained line in this terminal, including scrollback.
    pub fn select_all(&mut self) {
        let term = self.term.clone();
        let mut terminal = term.lock();
        select_all_in_term(&mut terminal);
    }

    /// Clear the visible screen without sending input to or restarting the shell.
    /// The cleared viewport is retained in scrollback, matching normal `clear` behavior.
    pub fn clear_screen(&mut self) {
        let term = self.term.clone();
        let mut terminal = term.lock();
        clear_visible_screen(&mut terminal);
        drop(terminal);
        self.clear_search();
    }

    pub fn sync(&mut self) -> &RenderableContent {
        let term = self.term.clone();
        let mut terminal = term.lock();
        if self.search_dirty.swap(false, Ordering::Relaxed) {
            if let Some(query) = self.search_query.as_deref() {
                let selected_match = self
                    .current_search_match
                    .and_then(|index| self.search_matches.get(index))
                    .cloned();
                self.search_matches = collect_search_matches(&terminal, query)
                    .unwrap_or_default();
                self.current_search_match = selected_match.and_then(|old| {
                    self.search_matches
                        .iter()
                        .position(|current| current == &old)
                });
            }
        }
        let selectable_range = match &terminal.selection {
            Some(s) => s.to_range(&terminal),
            None => None,
        };

        let cursor = terminal.grid_mut().cursor_cell().clone();
        self.last_content.grid = terminal.grid().clone();
        self.last_content.selectable_range = selectable_range;
        self.last_content.cursor = cursor.clone();
        self.last_content.terminal_mode = *terminal.mode();
        self.last_content.terminal_size = self.size;
        let (search_highlights, current_search_highlights) =
            visible_search_highlights(
                &terminal,
                &self.search_matches,
                self.current_search_match,
            );
        self.last_content.search_highlights = search_highlights;
        self.last_content.current_search_highlights = current_search_highlights;
        self.last_content()
    }

    pub fn last_content(&self) -> &RenderableContent {
        &self.last_content
    }

    /// A no-lock snapshot updated by `TerminalView` after input and resize.
    pub fn scrollback_state(&self) -> ScrollbackState {
        ScrollbackState::from_content(self.last_content())
    }

    /// Copy a bounded plain-text tail from the grid snapshot already owned by the UI.
    /// This does not read the PTY or acquire the terminal grid lock.
    pub fn plain_text_tail(&self, max_chars: usize) -> String {
        let limit = max_chars.min(200_000);
        if limit == 0 {
            return String::new();
        }
        let grid = &self.last_content.grid;
        let columns = grid.columns();
        if columns == 0 {
            return String::new();
        }
        let history = grid.history_size();
        let screen = grid.screen_lines();
        let rows = limit
            .div_ceil(columns)
            .saturating_add(1)
            .min(history.saturating_add(screen));
        // Grid lines run from -history through screen - 1. Count backwards
        // from the end, rather than subtracting history a second time.
        let first = screen as i32 - rows as i32;
        let mut output =
            String::with_capacity(limit.min(rows.saturating_mul(columns)));
        for row in first..screen as i32 {
            let mut line = String::with_capacity(columns);
            for column in 0..columns {
                let ch = grid.index(Point::new(Line(row), Column(column))).c;
                if ch != '\0' {
                    line.push(ch);
                }
            }
            output.push_str(line.trim_end());
            output.push('\n');
        }
        let mut chars: Vec<char> = output.chars().rev().take(limit).collect();
        chars.reverse();
        chars.into_iter().collect()
    }

    fn process_link_action(
        &mut self,
        terminal: &Term<EventProxy>,
        link_action: LinkAction,
        point: Point,
    ) {
        match link_action {
            LinkAction::Hover => {
                self.last_content.hovered_hyperlink = self.regex_match_at(
                    terminal,
                    point,
                    &mut self.url_regex.clone(),
                );
            },
            LinkAction::Clear => {
                self.last_content.hovered_hyperlink = None;
            },
            LinkAction::Open => {
                self.open_link();
            },
        };
    }

    fn open_link(&self) {
        if let Some(range) = &self.last_content.hovered_hyperlink {
            let start = range.start();
            let end = range.end();

            let mut url = String::from(self.last_content.grid.index(*start).c);
            for indexed in self.last_content.grid.iter_from(*start) {
                url.push(indexed.c);
                if indexed.point == *end {
                    break;
                }
            }

            open::that(url).unwrap_or_else(|_| {
                panic!("link opening is failed");
            })
        }
    }

    fn process_mouse_report(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        point: Point,
        pressed: bool,
    ) {
        let mut mods = 0;
        if modifiers.contains(Modifiers::SHIFT) {
            mods += 4;
        }
        if modifiers.contains(Modifiers::ALT) {
            mods += 8;
        }
        if modifiers.contains(Modifiers::COMMAND) {
            mods += 16;
        }

        match MouseMode::from(self.last_content().terminal_mode) {
            MouseMode::Sgr => {
                self.sgr_mouse_report(point, button as u8 + mods, pressed)
            },
            MouseMode::Normal(is_utf8) => {
                if pressed {
                    self.normal_mouse_report(
                        point,
                        button as u8 + mods,
                        is_utf8,
                    )
                } else {
                    self.normal_mouse_report(point, 3 + mods, is_utf8)
                }
            },
        }
    }

    fn sgr_mouse_report(&self, point: Point, button: u8, pressed: bool) {
        let c = if pressed { 'M' } else { 'm' };

        let msg = format!(
            "\x1b[<{};{};{}{}",
            button,
            point.column + 1,
            point.line + 1,
            c
        );

        self.notifier.notify(msg.as_bytes().to_vec());
    }

    fn normal_mouse_report(&self, point: Point, button: u8, is_utf8: bool) {
        let Point { line, column } = point;
        let max_point = if is_utf8 { 2015 } else { 223 };

        if line >= max_point || column >= max_point {
            return;
        }

        let mut msg = vec![b'\x1b', b'[', b'M', 32 + button];

        let mouse_pos_encode = |pos: usize| -> Vec<u8> {
            let pos = 32 + 1 + pos;
            let first = 0xC0 + pos / 64;
            let second = 0x80 + (pos & 63);
            vec![first as u8, second as u8]
        };

        if is_utf8 && column >= Column(95) {
            msg.append(&mut mouse_pos_encode(column.0));
        } else {
            msg.push(32 + 1 + column.0 as u8);
        }

        if is_utf8 && line >= 95 {
            msg.append(&mut mouse_pos_encode(line.0 as usize));
        } else {
            msg.push(32 + 1 + line.0 as u8);
        }

        self.notifier.notify(msg);
    }

    fn start_selection(
        &mut self,
        terminal: &mut Term<EventProxy>,
        selection_type: SelectionType,
        x: f32,
        y: f32,
    ) {
        let location = Self::selection_point(
            x,
            y,
            &self.size,
            terminal.grid().display_offset(),
        );
        terminal.selection = Some(Selection::new(
            selection_type,
            location,
            self.selection_side(x),
        ));
    }

    fn update_selection(
        &mut self,
        terminal: &mut Term<EventProxy>,
        x: f32,
        y: f32,
    ) {
        let display_offset = terminal.grid().display_offset();
        if let Some(ref mut selection) = terminal.selection {
            let location =
                Self::selection_point(x, y, &self.size, display_offset);
            selection.update(location, self.selection_side(x));
        }
    }

    fn selection_side(&self, x: f32) -> Side {
        let cell_x = x as usize % self.size.cell_width as usize;
        let half_cell_width = (self.size.cell_width as f32 / 2.0) as usize;

        if cell_x > half_cell_width {
            Side::Right
        } else {
            Side::Left
        }
    }

    fn resize(
        &mut self,
        terminal: &mut Term<EventProxy>,
        layout_size: Size,
        font_size: Size,
    ) {
        if layout_size == self.size.layout_size
            && font_size.width as u16 == self.size.cell_width
            && font_size.height as u16 == self.size.cell_height
        {
            return;
        }

        let lines = (layout_size.height / font_size.height.floor()) as u16;
        let cols = (layout_size.width / font_size.width.floor()) as u16;
        if lines > 0 && cols > 0 {
            self.size = TerminalSize {
                layout_size,
                cell_height: font_size.height as u16,
                cell_width: font_size.width as u16,
                num_lines: lines,
                num_cols: cols,
            };

            self.notifier.on_resize(self.size.into());
            terminal.resize(TermSize::new(
                self.size.num_cols as usize,
                self.size.num_lines as usize,
            ));
        }
    }

    fn write<I: Into<Cow<'static, [u8]>>>(&self, input: I) {
        self.notifier.notify(input);
    }

    fn scroll(&mut self, terminal: &mut Term<EventProxy>, delta_value: i32) {
        if delta_value != 0 {
            let scroll = Scroll::Delta(delta_value);
            if terminal
                .mode()
                .contains(TermMode::ALTERNATE_SCROLL | TermMode::ALT_SCREEN)
            {
                let line_cmd = if delta_value > 0 { b'A' } else { b'B' };
                let mut content = vec![];

                for _ in 0..delta_value.abs() {
                    content.push(0x1b);
                    content.push(b'O');
                    content.push(line_cmd);
                }

                self.notifier.notify(content);
            } else {
                terminal.grid_mut().scroll_display(scroll);
            }
        }
    }

    /// Based on alacritty/src/display/hint.rs > regex_match_at
    /// Retrieve the match, if the specified point is inside the content matching the regex.
    fn regex_match_at(
        &self,
        terminal: &Term<EventProxy>,
        point: Point,
        regex: &mut RegexSearch,
    ) -> Option<Match> {
        let x = visible_regex_match_iter(terminal, regex)
            .find(|rm| rm.contains(&point));
        x
    }
}

/// Copied from alacritty/src/display/hint.rs:
/// Iterate over all visible regex matches.
fn visible_regex_match_iter<'a>(
    term: &'a Term<EventProxy>,
    regex: &'a mut RegexSearch,
) -> impl Iterator<Item = Match> + 'a {
    let viewport_start = Line(-(term.grid().display_offset() as i32));
    let viewport_end = viewport_start + term.bottommost_line();
    let mut start =
        term.line_search_left(Point::new(viewport_start, Column(0)));
    let mut end = term.line_search_right(Point::new(viewport_end, Column(0)));
    start.line = start.line.max(viewport_start - 100);
    end.line = end.line.min(viewport_end + 100);

    RegexIter::new(start, end, Direction::Right, term, regex)
        .skip_while(move |rm| rm.end().line < viewport_start)
        .take_while(move |rm| rm.start().line <= viewport_end)
}

fn collect_search_matches<T>(
    term: &Term<T>,
    query: &str,
) -> std::result::Result<Vec<Match>, String> {
    if query.is_empty() || term.grid().columns() == 0 {
        return Ok(Vec::new());
    }

    let mut regex =
        RegexSearch::new(query).map_err(|error| error.to_string())?;
    let grid = term.grid();
    let start = Point::new(grid.topmost_line(), Column(0));
    let end = Point::new(grid.bottommost_line(), grid.last_column());
    Ok(
        RegexIter::new(start, end, Direction::Right, term, &mut regex)
            .collect(),
    )
}

fn next_search_index(
    current: Option<usize>,
    count: usize,
    forward: bool,
) -> usize {
    debug_assert!(count > 0);
    match (current, forward) {
        (None, true) => 0,
        (None, false) => count - 1,
        (Some(index), true) => (index + 1) % count,
        (Some(0), false) => count - 1,
        (Some(index), false) => (index - 1) % count,
    }
}

fn scroll_to_search_match<T: EventListener>(
    term: &mut Term<T>,
    matched: &Match,
) {
    let grid = term.grid();
    let half_viewport = (grid.screen_lines() / 2) as i32;
    let top_line = matched.start().line.0 - half_viewport;
    let target_offset = (-top_line).clamp(0, grid.history_size() as i32);
    let delta = target_offset - grid.display_offset() as i32;
    if delta != 0 {
        term.scroll_display(Scroll::Delta(delta));
    }
}

fn select_all_in_term<T>(term: &mut Term<T>) {
    let grid = term.grid();
    let start = Point::new(grid.topmost_line(), Column(0));
    let end = Point::new(grid.bottommost_line(), grid.last_column());
    let mut selection =
        Selection::new(AlacrittySelectionType::Lines, start, Side::Left);
    selection.update(end, Side::Right);
    term.selection = Some(selection);
}

fn clear_visible_screen<T: EventListener>(term: &mut Term<T>) {
    term.scroll_display(Scroll::Bottom);
    term.grid_mut().clear_viewport::<Color>();
    term.selection = None;
}

fn visible_search_highlights<T>(
    term: &Term<T>,
    matches: &[Match],
    current: Option<usize>,
) -> (HashSet<(i32, usize)>, HashSet<(i32, usize)>) {
    let grid = term.grid();
    let viewport_start = -(grid.display_offset() as i32);
    let viewport_end = viewport_start + grid.screen_lines() as i32 - 1;
    let mut highlights = HashSet::new();
    let mut current_highlights = HashSet::new();

    let first_visible = matches
        .partition_point(|matched| matched.end().line.0 < viewport_start);
    for (index, matched) in matches.iter().enumerate().skip(first_visible) {
        if matched.start().line.0 > viewport_end {
            break;
        }
        let target = if current == Some(index) {
            &mut current_highlights
        } else {
            &mut highlights
        };
        let first = matched.start();
        let last = matched.end();
        let first_line = first.line.0.max(viewport_start);
        let last_line = last.line.0.min(viewport_end);
        if first_line > last_line {
            continue;
        }

        for line in first_line..=last_line {
            let start_column = if line == first.line.0 {
                first.column.0
            } else {
                0
            };
            let end_column = if line == last.line.0 {
                last.column.0
            } else {
                grid.last_column().0
            };
            for column in start_column..=end_column {
                target.insert((line, column));
            }
        }
    }

    (highlights, current_highlights)
}

pub struct RenderableContent {
    pub grid: Grid<Cell>,
    pub hovered_hyperlink: Option<RangeInclusive<Point>>,
    pub selectable_range: Option<SelectionRange>,
    pub cursor: Cell,
    pub terminal_mode: TermMode,
    pub terminal_size: TerminalSize,
    pub search_highlights: HashSet<(i32, usize)>,
    pub current_search_highlights: HashSet<(i32, usize)>,
}

impl Default for RenderableContent {
    fn default() -> Self {
        Self {
            grid: Grid::new(0, 0, 0),
            hovered_hyperlink: None,
            selectable_range: None,
            cursor: Cell::default(),
            terminal_mode: TermMode::empty(),
            terminal_size: TerminalSize::default(),
            search_highlights: HashSet::new(),
            current_search_highlights: HashSet::new(),
        }
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);
    }
}

#[cfg(windows)]
impl TerminalBackend {
    /// Identity of the shell launched by this backend, for bounded lifecycle checks.
    pub fn child_process_id(&self) -> Option<std::num::NonZeroU32> {
        self.child_process_id
    }
}

#[derive(Clone)]
pub struct EventProxy(mpsc::Sender<Event>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event.clone());
    }
}

#[cfg(test)]
mod scrollback_tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::term::test::mock_term;

    #[test]
    fn snapshot_tracks_retained_grid_offset_and_mode() {
        let mut content = RenderableContent {
            grid: Grid::new(24, 80, 100),
            ..Default::default()
        };
        assert!(!ScrollbackState::from_content(&content).available());
        content.grid.scroll_up::<Color>(&(Line(0)..Line(24)), 60);
        let state = ScrollbackState::from_content(&content);
        assert_eq!(state.viewport_lines, 24);
        assert_eq!(state.history_lines, 60);
        assert_eq!(state.display_offset, 0);
        content.grid.scroll_display(Scroll::Top);
        assert_eq!(ScrollbackState::from_content(&content).display_offset, 60);
        content.grid.update_history(10);
        let truncated = ScrollbackState::from_content(&content);
        assert_eq!(truncated.history_lines, 10);
        assert_eq!(truncated.display_offset, 10);
        content.terminal_mode =
            TermMode::ALT_SCREEN | TermMode::MOUSE_REPORT_CLICK;
        assert!(!ScrollbackState::from_content(&content).available());
        content.grid.clear_history();
        assert_eq!(ScrollbackState::from_content(&content).history_lines, 0);
    }

    #[test]
    fn search_matches_wrapped_wide_unicode_and_retained_scrollback() {
        let wrapped = mock_term("ab🦇\nX\r\nend");
        let matches = collect_search_matches(&wrapped, "🦇X").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start(), &Point::new(Line(0), Column(2)));
        assert_eq!(matches[0].end(), &Point::new(Line(1), Column(0)));

        let size = TermSize::new(8, 2);
        let mut history = Term::new(
            term::Config {
                scrolling_history: 4,
                ..term::Config::default()
            },
            &size,
            VoidListener,
        );
        history
            .grid_mut()
            .scroll_up::<Color>(&(Line(0)..Line(2)), 1);
        for (column, character) in "older".chars().enumerate() {
            history.grid_mut()[Line(-1)][Column(column)].c = character;
        }
        let matches = collect_search_matches(&history, "older").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start().line, Line(-1));
    }

    #[test]
    fn search_navigation_wraps_at_both_ends() {
        assert_eq!(next_search_index(None, 3, true), 0);
        assert_eq!(next_search_index(None, 3, false), 2);
        assert_eq!(next_search_index(Some(2), 3, true), 0);
        assert_eq!(next_search_index(Some(0), 3, false), 2);
        assert_eq!(next_search_index(Some(1), 3, false), 0);
    }

    #[test]
    fn select_all_covers_retained_buffer_and_clear_keeps_terminal_state() {
        let size = TermSize::new(8, 2);
        let mut terminal = Term::new(
            term::Config {
                scrolling_history: 4,
                ..term::Config::default()
            },
            &size,
            VoidListener,
        );
        terminal
            .grid_mut()
            .scroll_up::<Color>(&(Line(0)..Line(2)), 1);
        for (column, character) in "older".chars().enumerate() {
            terminal.grid_mut()[Line(-1)][Column(column)].c = character;
        }
        for (line, text) in [(0, "first"), (1, "second")] {
            for (column, character) in text.chars().enumerate() {
                terminal.grid_mut()[Line(line)][Column(column)].c = character;
            }
        }

        select_all_in_term(&mut terminal);
        let selected = terminal.selection_to_string().unwrap();
        assert!(selected.contains("older"));
        assert!(selected.contains("first"));
        assert!(selected.contains("second"));

        clear_visible_screen(&mut terminal);
        assert!(terminal.selection.is_none());
        assert!(terminal.grid().history_size() > 0);
        assert!(terminal.grid().display_iter().all(|cell| cell.c == ' '));
        assert_eq!(
            collect_search_matches(&terminal, "first").unwrap().len(),
            1
        );
    }

    #[test]
    fn search_rejects_invalid_regex_and_empty_query() {
        let terminal = mock_term("content");
        assert!(collect_search_matches(&terminal, "[").is_err());
        assert!(collect_search_matches(&terminal, "").unwrap().is_empty());
    }
}
