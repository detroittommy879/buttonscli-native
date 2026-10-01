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
    pub input_sequence: u64,
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
    input_sequence: u64,
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
            input_sequence: 0,
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
        state.input_sequence = state.input_sequence.saturating_add(1);
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
        state.input_sequence = state.input_sequence.saturating_add(1);
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
            input_sequence: state.input_sequence,
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
    use serde_json::Value;
    use std::time::Instant;

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    b'A'..=b'F' => byte - b'A' + 10,
                    _ => panic!("invalid hex digit"),
                };
                (digit(pair[0]) << 4) | digit(pair[1])
            })
            .collect()
    }

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

    #[test]
    fn output_transcript_fixture_preserves_raw_controls_and_chunkwise_utf8() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/output-transcript.json"))
                .unwrap();
        assert_eq!(fixture["sourceRevision"], "032c9f2");

        for case in fixture["cases"].as_array().unwrap() {
            let capture = OutputCapture::default();
            let chunks = case["chunksHex"].as_array().unwrap();
            for chunk in chunks {
                capture.record_output_bytes(&decode_hex(chunk.as_str().unwrap()));
            }
            let expected = case["expected"].as_str().unwrap();
            assert_eq!(
                capture.read_chars(MAX_OUTPUT_CHARS, true),
                expected,
                "{}",
                case["name"]
            );
            assert_eq!(capture.snapshot().output_sequence, chunks.len() as u64);
            assert!(!capture.snapshot().output_truncated);
        }
    }

    #[test]
    fn identical_redraw_chunks_are_activity_even_when_text_repeats() {
        let capture = OutputCapture::default();
        capture.record_output_bytes(b"building 20%\r");
        capture.record_output_bytes(b"building 20%\r");

        assert_eq!(
            capture.read_chars(MAX_OUTPUT_CHARS, true),
            "building 20%\rbuilding 20%\r"
        );
        assert_eq!(capture.snapshot().output_sequence, 2);
        assert!(capture.snapshot().last_output_at_ms.is_some());
    }

    #[test]
    fn output_tail_clamps_by_unicode_scalar_without_partial_utf8() {
        let capture = OutputCapture::default();
        let output = "🧪".repeat(MAX_OUTPUT_CHARS + 1);
        capture.record_output_bytes(output.as_bytes());

        assert_eq!(capture.snapshot().stored_output_chars, MAX_OUTPUT_CHARS);
        assert!(capture.snapshot().output_truncated);
        assert_eq!(capture.read_chars(2, false), "🧪🧪");
        assert_eq!(capture.read_chars(2, true), "🧪🧪");
    }

    #[test]
    #[ignore = "manual output observer throughput and lock/allocation probe"]
    fn manual_output_capture_throughput_probe() {
        const TRIALS: usize = 5;
        const TOTAL_BYTES: usize = 32 * 1024 * 1024;
        const CHUNK_BYTES: usize = 4 * 1024;
        let chunk = vec![b'x'; CHUNK_BYTES];
        for trial in 1..=TRIALS {
            let capture = OutputCapture::default();
            let mut durations = Vec::with_capacity(TOTAL_BYTES / CHUNK_BYTES);
            let started = Instant::now();
            for _ in 0..(TOTAL_BYTES / CHUNK_BYTES) {
                let chunk_started = Instant::now();
                capture.record_output_bytes(&chunk);
                durations.push(chunk_started.elapsed());
            }
            let elapsed = started.elapsed();
            durations.sort_unstable();
            let p95 = durations[durations.len() * 95 / 100];
            let max = durations.last().copied().unwrap_or_default();
            let tail_read_started = Instant::now();
            let tail = capture.read_chars(MAX_OUTPUT_CHARS, false);
            let tail_read_elapsed = tail_read_started.elapsed();
            let snapshot = capture.snapshot();
            let mib_per_second = TOTAL_BYTES as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64();

            assert_eq!(snapshot.stored_output_chars, MAX_OUTPUT_CHARS);
            assert_eq!(tail.chars().count(), MAX_OUTPUT_CHARS);
            println!(
                "trial={trial} bytes={TOTAL_BYTES} chunk_bytes={CHUNK_BYTES} elapsed_ms={} mib_per_second={mib_per_second:.2} record_chunk_p95_us={} record_chunk_max_us={} saturated_tail_read_us={}",
                elapsed.as_millis(),
                p95.as_micros(),
                max.as_micros(),
                tail_read_elapsed.as_micros(),
            );
        }
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
