//! Bounded terminal input payload construction shared by reviewed actions.

pub(crate) const MAX_INPUT_BYTES: usize = 64 * 1024;

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
