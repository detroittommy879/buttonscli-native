//! Bounded terminal input payload construction shared by reviewed actions.

pub(crate) const MAX_INPUT_BYTES: usize = 64 * 1024;
const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";
const DEFAULT_SLOW_TYPED_DELAY_MS: u64 = 14;
const MAX_SLOW_TYPED_DELAY_MS: u64 = 250;
const MAX_SLOW_TYPED_DURATION_MS: u64 = 30_000;
const MAX_SLOW_TYPED_CHARS: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeliveryMode {
    Raw,
    Bracketed,
    SlowTyped,
}

impl DeliveryMode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Bracketed => "bracketed",
            Self::SlowTyped => "slow-typed",
        }
    }
}

pub(crate) struct PreparedDelivery {
    pub(crate) mode: DeliveryMode,
    pub(crate) chunks: Vec<Vec<u8>>,
    pub(crate) logical_input: Vec<u8>,
    pub(crate) delay_ms: u64,
}

pub(crate) fn prepare_delivery(
    payload: &[u8],
    enter: bool,
    mode: DeliveryMode,
    requested_delay_ms: Option<u64>,
) -> Result<PreparedDelivery, &'static str> {
    if payload.len() > MAX_INPUT_BYTES || payload.contains(&0) {
        return Err("terminal input exceeds the 64 KiB limit or contains NUL");
    }
    let delay_ms = requested_delay_ms.unwrap_or(DEFAULT_SLOW_TYPED_DELAY_MS);
    if requested_delay_ms == Some(0) {
        return Err("delayMs must be greater than 0");
    }
    if mode == DeliveryMode::SlowTyped && delay_ms > MAX_SLOW_TYPED_DELAY_MS {
        return Err("delayMs must be between 1 and 250");
    }

    let mut logical_input = payload.to_vec();
    if enter {
        logical_input.push(b'\r');
    }
    let bytes = match mode {
        DeliveryMode::Raw | DeliveryMode::SlowTyped => logical_input.clone(),
        DeliveryMode::Bracketed => {
            let mut bytes = Vec::with_capacity(
                payload.len()
                    + BRACKETED_PASTE_START.len()
                    + BRACKETED_PASTE_END.len()
                    + if enter { 1 } else { 0 },
            );
            bytes.extend_from_slice(BRACKETED_PASTE_START);
            bytes.extend_from_slice(payload);
            bytes.extend_from_slice(BRACKETED_PASTE_END);
            if enter {
                bytes.push(b'\r');
            }
            bytes
        }
    };
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("terminal input exceeds the 64 KiB limit after delivery framing");
    }

    let chunks = match mode {
        DeliveryMode::Raw | DeliveryMode::Bracketed => vec![bytes],
        DeliveryMode::SlowTyped => {
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| "slow-typed delivery requires valid UTF-8 input")?;
            let char_count = text.chars().count();
            if char_count > MAX_SLOW_TYPED_CHARS {
                return Err("slow-typed delivery is limited to 512 Unicode characters");
            }
            let duration = char_count.saturating_sub(1) as u64 * delay_ms;
            if duration > MAX_SLOW_TYPED_DURATION_MS {
                return Err("slow-typed input would exceed the 30 second delivery limit");
            }
            text.chars()
                .map(|character| {
                    let mut buffer = [0_u8; 4];
                    character.encode_utf8(&mut buffer).as_bytes().to_vec()
                })
                .collect()
        }
    };
    Ok(PreparedDelivery {
        mode,
        chunks,
        logical_input,
        delay_ms,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalKey {
    CtrlC,
    CtrlD,
    CtrlZ,
    Enter,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
}

pub(crate) fn command_bytes(command: &str, press_enter: bool) -> Result<Vec<u8>, &'static str> {
    if command.is_empty()
        || command.len() > 4096
        || command.chars().any(|ch| matches!(ch, '\0' | '\n' | '\r'))
    {
        return Err("reviewed command is empty or outside the input limit");
    }
    let mut bytes = literal_bytes(command)?;
    if press_enter {
        bytes.push(b'\r');
    }
    Ok(bytes)
}

pub(crate) fn literal_bytes(text: &str) -> Result<Vec<u8>, &'static str> {
    if text.len() > MAX_INPUT_BYTES || text.contains('\0') {
        return Err("terminal input exceeds the 64 KiB limit or contains NUL");
    }
    Ok(text.as_bytes().to_vec())
}

pub(crate) fn key_bytes(key: TerminalKey) -> &'static [u8] {
    match key {
        TerminalKey::CtrlC => b"\x03",
        TerminalKey::CtrlD => b"\x04",
        TerminalKey::CtrlZ => b"\x1a",
        TerminalKey::Enter => b"\r",
        TerminalKey::Tab => b"\t",
        TerminalKey::Escape => b"\x1b",
        TerminalKey::Up => b"\x1b[A",
        TerminalKey::Down => b"\x1b[B",
        TerminalKey::Right => b"\x1b[C",
        TerminalKey::Left => b"\x1b[D",
    }
}
