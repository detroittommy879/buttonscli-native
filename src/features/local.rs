//! Explicit local access for testing the native app with its ordinary profile.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

pub const FILE_NAME: &str = "feature-flags.json";
const MAX_BYTES: u64 = 16 * 1024;
static FLAGS: OnceLock<LocalFeatureFlags> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LocalFeatureFlags {
    pub enable_all: bool,
}

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
    FLAGS.get_or_init(|| match LocalFeatureFlags::load(root) {
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
    });
}

pub(crate) fn enabled() -> bool {
    FLAGS.get().is_some_and(|flags| flags.enable_all)
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
        std::fs::write(root.join(FILE_NAME), vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(LocalFeatureFlags::load(&root).is_err());
        assert_eq!(
            std::fs::read(&profile).unwrap(),
            b"existing buttons and settings"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
