//! Persisted, conflict-checked application shortcuts.

use std::collections::{BTreeMap, BTreeSet};

use egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};

use crate::i18n::MessageKey;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ShortcutAction {
    NewTab,
    CloseTab,
    ReopenTab,
    CopySelection,
    FindTerminal,
    Paste,
    OpenSettings,
    Quit,
}

impl ShortcutAction {
    pub(crate) const ALL: [Self; 8] = [
        Self::NewTab,
        Self::CloseTab,
        Self::ReopenTab,
        Self::CopySelection,
        Self::FindTerminal,
        Self::Paste,
        Self::OpenSettings,
        Self::Quit,
    ];

    pub(crate) fn message_key(self) -> MessageKey {
        match self {
            Self::NewTab => MessageKey::ShortcutNewTab,
            Self::CloseTab => MessageKey::ShortcutCloseTab,
            Self::ReopenTab => MessageKey::ShortcutReopenTab,
            Self::CopySelection => MessageKey::ShortcutCopy,
            Self::FindTerminal => MessageKey::TerminalFind,
            Self::Paste => MessageKey::ShortcutPaste,
            Self::OpenSettings => MessageKey::ShortcutOpenSettings,
            Self::Quit => MessageKey::ShortcutQuit,
        }
    }

    fn storage_key(self) -> &'static str {
        match self {
            Self::NewTab => "new-tab",
            Self::CloseTab => "close-tab",
            Self::ReopenTab => "reopen-tab",
            Self::CopySelection => "copy-selection",
            Self::FindTerminal => "find-terminal",
            Self::Paste => "paste",
            Self::OpenSettings => "open-settings",
            Self::Quit => "quit",
        }
    }

    fn from_storage_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.storage_key() == key)
    }
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ShortcutChord {
    /// egui key name, stored as text so newer or unsupported values stay readable.
    pub(crate) key: String,
    pub(crate) primary: bool,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
}

impl ShortcutChord {
    pub(crate) fn from_input(key: Key, modifiers: Modifiers) -> Option<Self> {
        let key_name = format!("{key:?}");
        parse_key(&key_name)?;
        Some(Self {
            key: key_name,
            primary: modifiers.command,
            ctrl: modifiers.ctrl && !modifiers.command,
            alt: modifiers.alt,
            shift: modifiers.shift,
        })
    }

    pub(crate) fn key(&self) -> Option<Key> {
        parse_key(&self.key)
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.key().is_some() && (self.primary || self.ctrl || self.alt)
    }

    pub(crate) fn is_reserved_terminal_interrupt(&self) -> bool {
        self.key == "C" && !self.shift && (self.primary || self.ctrl)
    }

    pub(crate) fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        let Some(expected_key) = self.key() else {
            return false;
        };
        if key != expected_key {
            return false;
        }
        modifiers.matches_exact(Modifiers {
            command: self.primary,
            ctrl: self.ctrl,
            alt: self.alt,
            shift: self.shift,
            ..Modifiers::default()
        })
    }

    pub(crate) fn label(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.primary {
            parts.push("Primary".to_owned());
        }
        if self.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        parts.push(key_label(&self.key));
        parts.join("+")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutAssignError {
    Invalid,
    ReservedTerminalInterrupt,
    Conflict(ShortcutAction),
    UnknownConflict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ShortcutSettings {
    /// Every action is stored explicitly; `None` means the user cleared it.
    /// String keys keep bindings from newer app versions round-trippable.
    bindings: BTreeMap<String, Option<ShortcutChord>>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        let defaults = [
            (ShortcutAction::NewTab, "T"),
            (ShortcutAction::CloseTab, "W"),
            (ShortcutAction::ReopenTab, "U"),
            (ShortcutAction::CopySelection, "C"),
            (ShortcutAction::FindTerminal, "F"),
            (ShortcutAction::Paste, "V"),
            (ShortcutAction::OpenSettings, "Comma"),
            (ShortcutAction::Quit, "Q"),
        ];
        Self {
            bindings: defaults
                .into_iter()
                .map(|(action, key)| {
                    (
                        action.storage_key().to_owned(),
                        Some(ShortcutChord {
                            key: key.into(),
                            primary: true,
                            ctrl: false,
                            alt: false,
                            shift: true,
                        }),
                    )
                })
                .collect(),
        }
    }
}

impl ShortcutSettings {
    pub(crate) fn binding(&self, action: ShortcutAction) -> Option<&ShortcutChord> {
        self.bindings
            .get(action.storage_key())
            .and_then(Option::as_ref)
    }

    pub(crate) fn clear(&mut self, action: ShortcutAction) {
        self.bindings.insert(action.storage_key().to_owned(), None);
    }

    pub(crate) fn assign(
        &mut self,
        action: ShortcutAction,
        chord: ShortcutChord,
    ) -> Result<(), ShortcutAssignError> {
        if !chord.is_valid() {
            return Err(ShortcutAssignError::Invalid);
        }
        if chord.is_reserved_terminal_interrupt() {
            return Err(ShortcutAssignError::ReservedTerminalInterrupt);
        }
        if let Some((other, _)) = self.bindings.iter().find(|(other, assigned)| {
            other.as_str() != action.storage_key() && assigned.as_ref() == Some(&chord)
        }) {
            return Err(ShortcutAction::from_storage_key(other)
                .map(ShortcutAssignError::Conflict)
                .unwrap_or(ShortcutAssignError::UnknownConflict));
        }
        self.bindings
            .insert(action.storage_key().to_owned(), Some(chord));
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn normalize(&mut self) {
        let defaults = Self::default();
        for action in ShortcutAction::ALL {
            self.bindings
                .entry(action.storage_key().to_owned())
                .or_insert_with(|| defaults.bindings[action.storage_key()].clone());
        }
        let known_keys: BTreeSet<&str> = ShortcutAction::ALL
            .iter()
            .map(|action| action.storage_key())
            .collect();
        let mut used: BTreeSet<ShortcutChord> = self
            .bindings
            .iter()
            .filter(|(key, _)| !known_keys.contains(key.as_str()))
            .filter_map(|(_, binding)| binding.as_ref())
            .filter(|binding| binding.is_valid() && !binding.is_reserved_terminal_interrupt())
            .cloned()
            .collect();
        for action in ShortcutAction::ALL {
            let key = action.storage_key().to_owned();
            let Some(mut binding) = self.bindings.get(&key).cloned().flatten() else {
                continue;
            };
            if !binding.is_valid() || binding.is_reserved_terminal_interrupt() {
                binding = defaults.bindings[action.storage_key()]
                    .clone()
                    .expect("default shortcut must be set");
            }
            if !used.insert(binding.clone()) {
                self.bindings.insert(key, None);
            } else {
                self.bindings.insert(key, Some(binding));
            }
        }
    }
}

fn parse_key(value: &str) -> Option<Key> {
    Some(match value {
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        "Num0" => Key::Num0,
        "Num1" => Key::Num1,
        "Num2" => Key::Num2,
        "Num3" => Key::Num3,
        "Num4" => Key::Num4,
        "Num5" => Key::Num5,
        "Num6" => Key::Num6,
        "Num7" => Key::Num7,
        "Num8" => Key::Num8,
        "Num9" => Key::Num9,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "Comma" => Key::Comma,
        "Plus" => Key::Plus,
        "Minus" => Key::Minus,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Space" => Key::Space,
        _ => return None,
    })
}

fn key_label(name: &str) -> String {
    match name {
        "Num0" => "0".into(),
        "Num1" => "1".into(),
        "Num2" => "2".into(),
        "Num3" => "3".into(),
        "Num4" => "4".into(),
        "Num5" => "5".into(),
        "Num6" => "6".into(),
        "Num7" => "7".into(),
        "Num8" => "8".into(),
        "Num9" => "9".into(),
        "Comma" => ",".into(),
        "Plus" => "+".into(),
        "Minus" => "-".into(),
        _ => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_modifier_matches_windows_ctrl_and_macos_command() {
        let chord = ShortcutChord {
            key: "T".into(),
            primary: true,
            ctrl: false,
            alt: false,
            shift: true,
        };
        assert!(chord.matches(
            Key::T,
            Modifiers {
                ctrl: true,
                command: true,
                shift: true,
                ..Modifiers::default()
            }
        ));
        assert!(chord.matches(
            Key::T,
            Modifiers {
                mac_cmd: true,
                command: true,
                shift: true,
                ..Modifiers::default()
            }
        ));
        assert!(!chord.matches(
            Key::T,
            Modifiers {
                ctrl: true,
                command: true,
                shift: true,
                alt: true,
                ..Modifiers::default()
            }
        ));
    }

    #[test]
    fn conflicts_and_terminal_ctrl_c_are_rejected() {
        let mut settings = ShortcutSettings::default();
        let close_tab = settings.binding(ShortcutAction::CloseTab).unwrap().clone();
        assert_eq!(
            settings.assign(ShortcutAction::NewTab, close_tab),
            Err(ShortcutAssignError::Conflict(ShortcutAction::CloseTab))
        );
        assert_eq!(
            settings.assign(
                ShortcutAction::CopySelection,
                ShortcutChord {
                    key: "C".into(),
                    primary: true,
                    ctrl: false,
                    alt: false,
                    shift: false,
                }
            ),
            Err(ShortcutAssignError::ReservedTerminalInterrupt)
        );
        assert_eq!(
            settings.assign(
                ShortcutAction::CopySelection,
                ShortcutChord {
                    key: "C".into(),
                    primary: true,
                    ctrl: false,
                    alt: true,
                    shift: false,
                }
            ),
            Err(ShortcutAssignError::ReservedTerminalInterrupt)
        );
    }

    #[test]
    fn missing_keys_restore_safe_defaults_and_invalid_legacy_chords_are_disabled() {
        let mut settings: ShortcutSettings = serde_json::from_str(
            r#"{"bindings":{"new-tab":{"key":"not-a-key","primary":false,"ctrl":false,"alt":false,"shift":false}}}"#,
        )
        .unwrap();
        settings.normalize();
        assert_eq!(
            settings.binding(ShortcutAction::NewTab).unwrap().label(),
            "Primary+Shift+T"
        );
        assert!(settings.binding(ShortcutAction::CloseTab).is_some());
    }

    #[test]
    fn unknown_future_action_bindings_survive_and_prevent_conflicting_assignment() {
        let mut settings: ShortcutSettings = serde_json::from_str(
            r#"{"bindings":{"future-zoom":{"key":"Z","primary":true,"ctrl":false,"alt":false,"shift":true}}}"#,
        )
        .unwrap();
        settings.normalize();
        let chord = ShortcutChord {
            key: "Z".into(),
            primary: true,
            ctrl: false,
            alt: false,
            shift: true,
        };
        assert_eq!(
            settings.assign(ShortcutAction::Quit, chord),
            Err(ShortcutAssignError::UnknownConflict)
        );
        assert!(serde_json::to_string(&settings)
            .unwrap()
            .contains("future-zoom"));
    }
}
