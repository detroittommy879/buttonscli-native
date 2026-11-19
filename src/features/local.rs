//! Explicit local access for testing the native app with its ordinary profile.
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "feature-flags.json";
const MAX_BYTES: u64 = 16 * 1024;
/// Read once at startup; Settings → Feature flags can change it later.
static FLAGS: OnceLock<RwLock<LocalFeatureFlags>> = OnceLock::new();
/// Where `FLAGS` came from, so changes are saved back to the same file.
static ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LocalFeatureFlags {
    pub enable_all: bool,
    /// Shows Settings → Release picks in optimized builds (debug builds always
    /// do, and so does `enable_all`).
    pub release_picks: bool,
    /// Shows Settings → Debug in optimized builds (debug builds always do).
    pub debug: bool,
    /// Shows Settings → Feature flags, where these switches can be changed.
    pub admin: bool,
    /// File → New shader window: a window with only a shader in it.
    pub shader_window: bool,
    /// View → AI chat: chat without a terminal, styled text from the model.
    pub rich_chat: bool,
}

/// One switch in Settings → Feature flags.
pub struct FlagInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub get: fn(&LocalFeatureFlags) -> bool,
    pub set: fn(&mut LocalFeatureFlags, bool),
}

/// Every local flag, in the order Settings shows them.
pub const FLAGS_INFO: [FlagInfo; 6] = [
    FlagInfo {
        name: "enable_all",
        description: "Turns every flag below on at once.",
        get: |flags| flags.enable_all,
        set: |flags, on| flags.enable_all = on,
    },
    FlagInfo {
        name: "admin",
        description: "Shows this Feature flags tab. Turning it off hides the tab (edit feature-flags.json to get it back).",
        get: |flags| flags.admin,
        set: |flags, on| flags.admin = on,
    },
    FlagInfo {
        name: "debug",
        description: "Settings → Debug and Gallery in release builds.",
        get: |flags| flags.debug,
        set: |flags, on| flags.debug = on,
    },
    FlagInfo {
        name: "release_picks",
        description: "Settings → Release picks in release builds.",
        get: |flags| flags.release_picks,
        set: |flags, on| flags.release_picks = on,
    },
    FlagInfo {
        name: "shader_window",
        description: "File → New shader window: a window with only a shader, for a second screen or a background.",
        get: |flags| flags.shader_window,
        set: |flags, on| flags.shader_window = on,
    },
    FlagInfo {
        name: "rich_chat",
        description: "View → AI chat: chat without a terminal, where the model can style words with fonts, sizes, colors, gradients and gentle motion.",
        get: |flags| flags.rich_chat,
        set: |flags, on| flags.rich_chat = on,
    },
];

impl LocalFeatureFlags {
    /// Missing files retain ordinary access. Invalid files never grant access.
    pub fn load(root: &Path) -> io::Result<Self> {
        let file = match File::open(root.join(FILE_NAME)) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "feature flags file is too large",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid feature flags JSON"))
    }
}

pub(crate) fn initialize(root: &Path) {
    FLAGS.get_or_init(|| {
        let _ = ROOT.set(root.to_path_buf());
        RwLock::new(match LocalFeatureFlags::load(root) {
            Ok(flags) => {
                if flags.enable_all {
                    log::info!(
                        "local feature override enabled: {}",
                        root.join(FILE_NAME).display()
                    );
                }
                flags
            }
            Err(error) => {
                log::warn!("local feature override ignored: {error}");
                LocalFeatureFlags::default()
            }
        })
    });
}

/// The flags in effect now (all off before `initialize`).
pub(crate) fn current() -> LocalFeatureFlags {
    FLAGS
        .get()
        .and_then(|flags| flags.read().ok().map(|flags| *flags))
        .unwrap_or_default()
}

/// The file the flags are read from and saved to, once known.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn file_path() -> Option<PathBuf> {
    ROOT.get().map(|root| root.join(FILE_NAME))
}

/// Saves `flags` (atomically) and applies them at once.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn save(flags: LocalFeatureFlags) -> io::Result<()> {
    let path = file_path().ok_or_else(|| io::Error::other("feature flags are not initialized"))?;
    let bytes = serde_json::to_vec_pretty(&flags).map_err(io::Error::other)?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temp, &bytes)?;
    if let Err(error) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    if let Some(lock) = FLAGS.get() {
        if let Ok(mut current) = lock.write() {
            *current = flags;
        }
    }
    Ok(())
}

pub(crate) fn enabled() -> bool {
    current().enable_all
}

pub(crate) fn debug_enabled() -> bool {
    let flags = current();
    flags.debug || flags.enable_all
}

pub(crate) fn release_picks_enabled() -> bool {
    let flags = current();
    flags.release_picks || flags.enable_all
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn admin_enabled() -> bool {
    let flags = current();
    flags.admin || flags.enable_all
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn shader_window_enabled() -> bool {
    let flags = current();
    flags.shader_window || flags.enable_all
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn rich_chat_enabled() -> bool {
    let flags = current();
    flags.rich_chat || flags.enable_all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_json_flag_is_required_and_existing_profile_is_untouched() {
        let root =
            std::env::temp_dir().join(format!("buttonscli-flags-{:032x}", fastrand::u128(..)));
        std::fs::create_dir_all(root.join("profiles/default")).unwrap();
        let profile = root.join("profiles/default/native.json");
        std::fs::write(&profile, b"existing buttons and settings").unwrap();
        assert!(!LocalFeatureFlags::load(&root).unwrap().enable_all);
        for (json, expected) in [
            ("{}", false),
            (r#"{"enable_all": false}"#, false),
            (r#"{"enable_all": true}"#, true),
        ] {
            std::fs::write(root.join(FILE_NAME), json).unwrap();
            assert_eq!(LocalFeatureFlags::load(&root).unwrap().enable_all, expected);
        }
        for json in [
            "",
            "{",
            r#"{"enable_all": "true"}"#,
            r#"{"enableAll": true}"#,
            r#"{"enable_all": true, "extra": 1}"#,
        ] {
            std::fs::write(root.join(FILE_NAME), json).unwrap();
            assert!(LocalFeatureFlags::load(&root).is_err());
        }
        let all_on = r#"{"enable_all":true,"release_picks":true,"debug":true,"admin":true,"shader_window":true,"rich_chat":true}"#;
        std::fs::write(root.join(FILE_NAME), all_on).unwrap();
        let mut flags = LocalFeatureFlags::load(&root).unwrap();
        for info in &FLAGS_INFO {
            assert!((info.get)(&flags), "{}", info.name);
            (info.set)(&mut flags, false);
        }
        assert_eq!(flags, LocalFeatureFlags::default());
        std::fs::write(root.join(FILE_NAME), vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(LocalFeatureFlags::load(&root).is_err());
        assert_eq!(
            std::fs::read(&profile).unwrap(),
            b"existing buttons and settings"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
