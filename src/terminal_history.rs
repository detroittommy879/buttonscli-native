//! Opt-in, bounded plain-text snapshots. Only our own files are expired.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::storage::store::{atomic_replace, ensure_native_path};

pub(crate) const SNAPSHOT_CHARS: usize = 200_000;

pub(crate) struct Snapshot {
    pub id: u64,
    pub title: String,
    pub text: String,
}

struct Batch {
    snapshots: Vec<Snapshot>,
    retention_days: u32,
    now: SystemTime,
}

pub(crate) struct HistoryWriter {
    pub root: PathBuf,
    sender: Option<SyncSender<Batch>>,
    worker: Option<JoinHandle<()>>,
    errors: Receiver<String>,
}

impl HistoryWriter {
    pub fn new(profile: &Path) -> Result<Self, String> {
        let root = profile.join("terminal-history");
        ensure_native_path(profile, &root).map_err(|error| error.to_string())?;
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        ensure_native_path(profile, &root).map_err(|error| error.to_string())?;
        let (sender, receiver) = mpsc::sync_channel::<Batch>(2);
        let (errors_tx, errors) = mpsc::sync_channel(1);
        let folder = root.clone();
        let instance = format!("{:032x}", fastrand::u128(..));
        let worker = std::thread::Builder::new()
            .name("terminal-history".into())
            .spawn(move || {
                for batch in receiver {
                    if let Err(error) = save_batch(&folder, &instance, batch) {
                        let _ = errors_tx.try_send(error);
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            root,
            sender: Some(sender),
            worker: Some(worker),
            errors,
        })
    }

    /// Bounded queue; the caller retries on the next tick when storage is slow.
    pub fn save(&self, snapshots: Vec<Snapshot>, retention_days: u32) -> bool {
        let batch = Batch {
            snapshots,
            retention_days,
            now: SystemTime::now(),
        };
        self.sender
            .as_ref()
            .is_some_and(|sender| match sender.try_send(batch) {
                Ok(()) => true,
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
            })
    }

    pub fn error(&self) -> Option<String> {
        self.errors.try_recv().ok()
    }
}

impl Drop for HistoryWriter {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn save_batch(root: &Path, instance: &str, batch: Batch) -> Result<(), String> {
    prune(root, batch.now, batch.retention_days).map_err(|error| error.to_string())?;
    if batch.snapshots.is_empty() {
        return Ok(());
    }
    let day = batch
        .now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    let folder = root.join(date_folder(day));
    ensure_native_path(root, &folder).map_err(|error| error.to_string())?;
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    for snapshot in batch.snapshots {
        let path = folder.join(format!("native-{instance}-{}.txt", snapshot.id));
        ensure_native_path(root, &path).map_err(|error| error.to_string())?;
        let title: String = snapshot
            .title
            .chars()
            .filter(|ch| !ch.is_control())
            .take(60)
            .collect();
        let text: String = snapshot.text.chars().take(SNAPSHOT_CHARS).collect();
        let document = format!("Terminal: {title}\nSaved date (UTC): {}\nSnapshot of retained terminal text; older discarded lines are not included.\n\n{text}", date_folder(day));
        atomic_replace(&path, document.as_bytes()).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn owned_file(name: &str) -> bool {
    let Some(name) = name
        .strip_prefix("native-")
        .and_then(|name| name.strip_suffix(".txt"))
    else {
        return false;
    };
    let Some((instance, id)) = name.split_once('-') else {
        return false;
    };
    instance.len() == 32
        && instance.bytes().all(|ch| ch.is_ascii_hexdigit())
        && id.parse::<u64>().is_ok()
}

fn dated_folder(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, ch)| index == 4 || index == 7 || ch.is_ascii_digit())
}

/// Never recurse or follow links; unknown files/folders are left alone.
fn prune(root: &Path, now: SystemTime, days: u32) -> std::io::Result<()> {
    let age = Duration::from_secs(days.clamp(1, 365) as u64 * 86_400);
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || !dated_folder(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let folder = entry.path();
        if ensure_native_path(root, &folder).is_err() {
            continue;
        }
        for file in fs::read_dir(&folder)? {
            let file = file?;
            if !file.file_type()?.is_file() || !owned_file(&file.file_name().to_string_lossy()) {
                continue;
            }
            if ensure_native_path(root, &file.path()).is_err() {
                continue;
            }
            if now
                .duration_since(file.metadata()?.modified()?)
                .is_ok_and(|elapsed| elapsed >= age)
            {
                fs::remove_file(file.path())?;
            }
        }
        // Fails harmlessly if the folder contains anything we do not own.
        let _ = fs::remove_dir(&folder);
    }
    Ok(())
}

// Gregorian civil date from Unix days (400-year eras), independent of locale.
fn date_folder(days: u64) -> String {
    let z = days.min(2_932_896) as i64 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_handles_epoch_leap_day_and_year_rollover() {
        assert_eq!(date_folder(0), "1970-01-01");
        assert_eq!(date_folder(11_016), "2000-02-29");
        assert_eq!(date_folder(19_723), "2024-01-01");
        assert_eq!(date_folder(19_782), "2024-02-29");
    }

    #[test]
    fn snapshots_replace_and_expiry_preserves_foreign_files_and_nested_folders() {
        let profile =
            std::env::temp_dir().join(format!("buttonscli-history-{:032x}", fastrand::u128(..)));
        fs::create_dir_all(&profile).unwrap();
        let writer = HistoryWriter::new(&profile).unwrap();
        let root = writer.root.clone();
        let now = SystemTime::now();
        let snapshot = |text: &str| Snapshot {
            id: 3,
            title: "term3".into(),
            text: text.into(),
        };
        save_batch(
            &root,
            "00000000000000000000000000000000",
            Batch {
                snapshots: vec![snapshot("first")],
                retention_days: 7,
                now,
            },
        )
        .unwrap();
        save_batch(
            &root,
            "00000000000000000000000000000000",
            Batch {
                snapshots: vec![snapshot("new 🦇")],
                retention_days: 7,
                now,
            },
        )
        .unwrap();
        let folder = fs::read_dir(&root).unwrap().next().unwrap().unwrap().path();
        let path = folder.join("native-00000000000000000000000000000000-3.txt");
        assert!(fs::read_to_string(&path).unwrap().ends_with("new 🦇"));
        fs::write(folder.join("personal.txt"), "keep").unwrap();
        fs::create_dir(folder.join("nested")).unwrap();
        fs::write(folder.join("nested/keep.txt"), "keep").unwrap();
        prune(&root, now + Duration::from_secs(8 * 86_400), 7).unwrap();
        assert!(!path.exists());
        assert!(folder.join("personal.txt").exists());
        assert!(folder.join("nested/keep.txt").exists());
        assert!(writer.save(vec![snapshot("queued before shutdown")], 7));
        drop(writer);
        assert!(fs::read_dir(&folder)
            .unwrap()
            .any(|entry| fs::read_to_string(entry.unwrap().path())
                .is_ok_and(|text| text.ends_with("queued before shutdown"))));
        fs::remove_dir_all(profile).unwrap();
    }
}
