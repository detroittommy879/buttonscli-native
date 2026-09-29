//! Bounded raw PTY output capture and activity metadata for CLI/AI consumers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const MAX_OUTPUT_CHARS: usize = 200_000;
const MAX_LAST_INPUT_CHARS: usize = 200_000;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct OutputSnapshot {
    pub stored_output_chars: usize,
    pub output_truncated: bool,
    pub last_input: String,
    pub last_input_at_ms: Option<u64>,
    pub last_output_at_ms: Option<u64>,
    pub last_updated_at_ms: u64,
    pub output_sequence: u64,
}

struct State {
    tail: String,
    tail_chars: usize,
    output_truncated: bool,
    last_input: String,
    last_input_at_ms: Option<u64>,
    last_output_at_ms: Option<u64>,
    last_updated_at_ms: u64,
    output_sequence: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tail: String::new(),
            tail_chars: 0,
            output_truncated: false,
            last_input: String::new(),
            last_input_at_ms: None,
            last_output_at_ms: None,
            last_updated_at_ms: now_ms(),
            output_sequence: 0,
        }
    }
}

pub(crate) struct OutputCapture {
    state: Mutex<State>,
    activity_at_ms: AtomicU64,
}

impl Default for OutputCapture {
    fn default() -> Self {
        let state = State::default();
        Self {
            activity_at_ms: AtomicU64::new(state.last_updated_at_ms),
            state: Mutex::new(state),
        }
    }
}

impl std::fmt::Debug for OutputCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OutputCapture")
            .field(&self.snapshot())
            .finish()
    }
}

impl PartialEq for OutputCapture {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot() == other.snapshot()
    }
}

impl Eq for OutputCapture {}

impl OutputCapture {
    /// Called from Alacritty's existing PTY reader; retain its chunk-wise UTF-8 behavior.
    pub(crate) fn record_output_bytes(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(bytes);
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let added_chars = text.chars().count();
        state.tail.push_str(&text);
        state.tail_chars = state.tail_chars.saturating_add(added_chars);
        if state.tail_chars > MAX_OUTPUT_CHARS {
            let remove_chars = state.tail_chars - MAX_OUTPUT_CHARS;
            let remove_bytes = state
                .tail
                .char_indices()
                .nth(remove_chars)
                .map_or(state.tail.len(), |(index, _)| index);
            state.tail.drain(..remove_bytes);
            state.tail_chars = MAX_OUTPUT_CHARS;
            state.output_truncated = true;
        }
        let now = now_ms();
        state.last_output_at_ms = Some(now);
        state.last_updated_at_ms = now;
        self.activity_at_ms.store(now, Ordering::Relaxed);
        state.output_sequence = state.output_sequence.saturating_add(1);
    }

    /// Called when the app sends bytes through the terminal's normal input path.
    pub(crate) fn record_input_bytes(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(bytes);
        let mut chars = text
            .chars()
            .rev()
            .take(MAX_LAST_INPUT_CHARS)
            .collect::<Vec<_>>();
        chars.reverse();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.last_input = chars.into_iter().collect();
        let now = now_ms();
        state.last_input_at_ms = Some(now);
        state.last_updated_at_ms = now;
        self.activity_at_ms.store(now, Ordering::Relaxed);
    }

    /// Update activity after a vault paste without retaining its literal bytes.
    pub(crate) fn record_sensitive_input(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.last_input.clear();
        let now = now_ms();
        state.last_input_at_ms = Some(now);
        state.last_updated_at_ms = now;
        self.activity_at_ms.store(now, Ordering::Relaxed);
    }

    /// Lock-free timestamp used by the renderer's idle-effect scheduler.
    pub(crate) fn last_updated_at_ms(&self) -> u64 {
        self.activity_at_ms.load(Ordering::Relaxed)
    }

    pub(crate) fn snapshot(&self) -> OutputSnapshot {
        let Ok(state) = self.state.lock() else {
            return OutputSnapshot::default();
        };
        OutputSnapshot {
            stored_output_chars: state.tail_chars,
            output_truncated: state.output_truncated,
            last_input: state.last_input.clone(),
            last_input_at_ms: state.last_input_at_ms,
            last_output_at_ms: state.last_output_at_ms,
            last_updated_at_ms: state.last_updated_at_ms,
            output_sequence: state.output_sequence,
        }
    }

    pub(crate) fn read_chars(&self, limit: usize, from_top: bool) -> String {
        let Ok(state) = self.state.lock() else {
            return String::new();
        };
        if from_top {
            state.tail.chars().take(limit).collect()
        } else {
            let start = state.tail_chars.saturating_sub(limit);
            state.tail.chars().skip(start).collect()
        }
    }

    pub(crate) fn read_lines(&self, limit: usize, from_top: bool) -> String {
        if limit == 0 {
            return String::new();
        }
        let Ok(state) = self.state.lock() else {
            return String::new();
        };
        let lines = state.tail.lines().collect::<Vec<_>>();
        if from_top {
            lines.into_iter().take(limit).collect::<Vec<_>>().join("\n")
        } else {
            let start = lines.len().saturating_sub(limit);
            lines.into_iter().skip(start).collect::<Vec<_>>().join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_timestamp_tracks_input_and_output_without_a_snapshot_lock() {
        let capture = OutputCapture::default();
        let created_at = capture.last_updated_at_ms();
        capture.record_input_bytes(b"command");
        let after_input = capture.last_updated_at_ms();
        assert!(after_input >= created_at);
        assert_eq!(after_input, capture.snapshot().last_updated_at_ms);

        capture.record_output_bytes(b"result");
        let after_output = capture.last_updated_at_ms();
        assert!(after_output >= after_input);
        assert_eq!(after_output, capture.snapshot().last_updated_at_ms);
    }

    #[test]
    fn sensitive_input_updates_activity_without_keeping_prior_input_text() {
        let capture = OutputCapture::default();
        capture.record_input_bytes(b"previous ordinary command");
        capture.record_sensitive_input();
        let snapshot = capture.snapshot();
        assert!(snapshot.last_input.is_empty());
        assert!(snapshot.last_input_at_ms.is_some());
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
