//! Debug copies of profile files, taken before anything replaces or deletes
//! them. They are plain text, kept under `<native root>/debug/backups/<profile>/`
//! and pruned oldest first. They exist so a lost setting or theme can be
//! found and compared later; nothing reads them automatically.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Copies kept per profile before the oldest are removed.
pub(crate) const MAX_BACKUP_FILES: usize = 2000;
/// Total size kept per profile before the oldest are removed.
pub(crate) const MAX_BACKUP_BYTES: u64 = 256 * 1024 * 1024;
/// Routine saves of the same file are copied at most this often.
pub(crate) const ROUTINE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Why a copy is taken. Everything except `Routine` is always copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackupReason {
    /// An ordinary save over the file (rate limited).
    Routine,
    /// The first read of the file in this run.
    Startup,
    /// The file is about to be replaced by a different version.
    Replace,
    /// The file is about to be deleted.
    Delete,
    /// Loading changed or dropped something, or could not read the file.
    Questionable,
}

impl BackupReason {
    fn tag(self) -> &'static str {
        match self {
            Self::Routine => "save",
            Self::Startup => "start",
            Self::Replace => "replace",
            Self::Delete => "delete",
            Self::Questionable => "check",
        }
    }
}

/// The folder copies for `profile` go to.
pub(crate) fn backup_dir(native_root: &Path, profile: &str) -> PathBuf {
    native_root.join("debug").join("backups").join(profile)
}

/// Copies `source` (if it exists) into the profile's backup folder. Failures
/// are logged and ignored: a backup must never stop a save.
pub(crate) fn backup_file(
    native_root: &Path,
    profile: &str,
    source: &Path,
    reason: BackupReason,
) -> Option<PathBuf> {
    match try_backup(native_root, profile, source, reason, SystemTime::now()) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(?error, source = %source.display(), "debug backup failed");
            None
        }
    }
}

/// Stores `bytes` (for example text that could not be parsed) as a copy.
pub(crate) fn backup_bytes(
    native_root: &Path,
    profile: &str,
    label: &str,
    bytes: &[u8],
    reason: BackupReason,
) -> Option<PathBuf> {
    let dir = backup_dir(native_root, profile);
    let result = (|| -> std::io::Result<PathBuf> {
        fs::create_dir_all(&dir)?;
        let path = dir.join(file_name(SystemTime::now(), reason, label));
        fs::write(&path, bytes)?;
        prune(&dir)?;
        Ok(path)
    })();
    match result {
        Ok(path) => Some(path),
        Err(error) => {
            tracing::warn!(?error, label, "debug backup failed");
            None
        }
    }
}

fn try_backup(
    native_root: &Path,
    profile: &str,
    source: &Path,
    reason: BackupReason,
    now: SystemTime,
) -> std::io::Result<Option<PathBuf>> {
    let bytes = match fs::read(source) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let label = label_for(source);
    let dir = backup_dir(native_root, profile);
    fs::create_dir_all(&dir)?;
    if let Some((newest, taken)) = newest_copy(&dir, &label)? {
        let same = Sha256::digest(&bytes) == Sha256::digest(fs::read(&newest)?);
        let recent = now
            .duration_since(taken)
            .map(|age| age < ROUTINE_INTERVAL)
            .unwrap_or(true);
        if same || (reason == BackupReason::Routine && recent) {
            return Ok(None);
        }
    }
    let path = dir.join(file_name(now, reason, &label));
    fs::write(&path, &bytes)?;
    prune(&dir)?;
    Ok(Some(path))
}

/// `native.json` → `native.json`; `themes/aurora.json` → `theme~aurora.json`.
fn label_for(source: &Path) -> String {
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let parent = source
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    match parent.as_deref() {
        Some("themes") => format!("theme~{name}"),
        _ => name,
    }
}

/// `20261124T153012.123Z-replace-theme~aurora.json`: sorts by time.
fn file_name(time: SystemTime, reason: BackupReason, label: &str) -> String {
    format!("{}-{}-{}", stamp(time), reason.tag(), label)
}

fn stamp(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = since.as_secs();
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let rest = seconds % 86_400;
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}.{:03}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60,
        since.subsec_millis()
    )
}

fn parse_stamp(name: &str) -> Option<SystemTime> {
    let stamp = name.get(..20)?;
    let number = |range: std::ops::Range<usize>| stamp.get(range)?.parse::<u64>().ok();
    let (year, month, day) = (number(0..4)?, number(4..6)?, number(6..8)?);
    let (hour, minute, second, millis) = (
        number(9..11)?,
        number(11..13)?,
        number(13..15)?,
        number(16..19)?,
    );
    let days = days_from_civil(year as i64, month as u32, day as u32);
    let seconds = days as u64 * 86_400 + hour * 3600 + minute * 60 + second;
    Some(UNIX_EPOCH + Duration::from_secs(seconds) + Duration::from_millis(millis))
}

// Howard Hinnant's civil date algorithms (proleptic Gregorian, UTC).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn newest_copy(dir: &Path, label: &str) -> std::io::Result<Option<(PathBuf, SystemTime)>> {
    let suffix = format!("-{label}");
    let mut newest: Option<(String, PathBuf)> = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // `<stamp>-<reason>-<label>`: the label follows the second dash.
        let matches = name
            .get(21..)
            .and_then(|rest| rest.split_once('-'))
            .is_some_and(|(_, rest)| format!("-{rest}") == suffix);
        if matches && newest.as_ref().is_none_or(|(best, _)| &name > best) {
            newest = Some((name, entry.path()));
        }
    }
    Ok(newest.and_then(|(name, path)| Some((path, parse_stamp(&name)?))))
}

fn prune(dir: &Path) -> std::io::Result<()> {
    let mut files: Vec<(String, PathBuf, u64)> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| {
            let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
                size,
            )
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut total: u64 = files.iter().map(|file| file.2).sum();
    let mut count = files.len();
    for (_, path, size) in files {
        if count <= MAX_BACKUP_FILES && total <= MAX_BACKUP_BYTES {
            break;
        }
        fs::remove_file(path)?;
        count -= 1;
        total = total.saturating_sub(size);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "buttonscli-backups-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn stamps_round_trip_and_sort_by_time() {
        let time = UNIX_EPOCH + Duration::from_millis(1_795_512_345_678);
        let text = stamp(time);
        assert_eq!(text.len(), 20);
        assert_eq!(parse_stamp(&text), Some(time));
        assert!(stamp(time + Duration::from_secs(1)) > text);
        assert_eq!(stamp(UNIX_EPOCH), "19700101T000000.000Z");
    }

    #[test]
    fn copies_change_skip_duplicates_and_rate_limit_routine_saves() {
        let root = scratch("copies");
        let themes = root.join("profiles/default/themes");
        fs::create_dir_all(&themes).unwrap();
        let theme = themes.join("aurora.json");
        let native = root.join("profiles/default/native.json");
        let start = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let backups = backup_dir(&root, "default");

        // Nothing to copy yet.
        assert!(
            try_backup(&root, "default", &theme, BackupReason::Replace, start)
                .unwrap()
                .is_none()
        );
        fs::write(&theme, "one").unwrap();
        let first = try_backup(&root, "default", &theme, BackupReason::Replace, start)
            .unwrap()
            .unwrap();
        assert!(first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with("-replace-theme~aurora.json"));
        // Same bytes again: skipped, whatever the reason.
        let later = start + Duration::from_secs(1);
        assert!(
            try_backup(&root, "default", &theme, BackupReason::Delete, later)
                .unwrap()
                .is_none()
        );
        fs::write(&theme, "two").unwrap();
        assert!(
            try_backup(&root, "default", &theme, BackupReason::Replace, later)
                .unwrap()
                .is_some()
        );

        fs::write(&native, "a").unwrap();
        assert!(
            try_backup(&root, "default", &native, BackupReason::Routine, start)
                .unwrap()
                .is_some()
        );
        fs::write(&native, "b").unwrap();
        let soon = start + Duration::from_secs(60);
        assert!(
            try_backup(&root, "default", &native, BackupReason::Routine, soon)
                .unwrap()
                .is_none(),
            "routine saves are rate limited"
        );
        assert!(
            try_backup(&root, "default", &native, BackupReason::Questionable, soon)
                .unwrap()
                .is_some(),
            "a questionable change is always copied"
        );
        let much_later = soon + ROUTINE_INTERVAL;
        fs::write(&native, "c").unwrap();
        assert!(
            try_backup(&root, "default", &native, BackupReason::Routine, much_later)
                .unwrap()
                .is_some()
        );
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 5);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn oldest_copies_are_pruned_first() {
        let dir = scratch("prune");
        for index in 0..(MAX_BACKUP_FILES + 3) {
            fs::write(dir.join(format!("{index:020}-save-native.json")), "x").unwrap();
        }
        prune(&dir).unwrap();
        let mut names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names.len(), MAX_BACKUP_FILES);
        assert_eq!(names[0], format!("{:020}-save-native.json", 3));
        fs::remove_dir_all(&dir).ok();
    }
}
