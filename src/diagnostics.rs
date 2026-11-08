//! Local logs and crash reports, for bug reports. Nothing is uploaded.
//!
//! `~/.buttonscli-native/logs/`:
//! - `buttonscli.log`: this launch's log (`RUST_LOG`, default `warn,buttonscli=info`);
//!   the previous launch's is kept as `buttonscli.previous.log`.
//! - `crash-<unix seconds>.txt`: version, OS, panic message, location and
//!   backtrace. The newest [`KEEP_CRASH_REPORTS`] are kept.
//!
//! Logs record app events only. Never log keys, tokens or terminal contents.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Crash reports kept in the logs folder; older ones are deleted at startup.
const KEEP_CRASH_REPORTS: usize = 10;

/// `~/.buttonscli-native/logs`, or `None` without a home directory.
pub fn logs_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join(".buttonscli-native").join("logs"))
}

/// Starts logging to stderr and the log file, and installs the crash-report
/// panic hook. Failures to create the folder or file only disable the file.
pub fn init() {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "warn,buttonscli=info".into())
    };
    let file = logs_dir().and_then(|dir| open_log(&dir).ok());
    match file {
        Some(file) => tracing_subscriber::fmt()
            .with_env_filter(filter())
            .with_ansi(false)
            .compact()
            .with_writer(std::io::stderr.and(Mutex::new(file)))
            .init(),
        None => tracing_subscriber::fmt()
            .with_env_filter(filter())
            .compact()
            .init(),
    }
    if let Some(dir) = logs_dir() {
        prune_crash_reports(&dir, KEEP_CRASH_REPORTS);
        install_panic_hook(dir);
    }
    tracing::info!(
        version = crate::BUILD_VERSION,
        os = std::env::consts::OS,
        "ButtonsCLI started"
    );
}

fn open_log(dir: &Path) -> std::io::Result<std::fs::File> {
    std::fs::create_dir_all(dir)?;
    let current = dir.join("buttonscli.log");
    if current.exists() {
        let _ = std::fs::rename(&current, dir.join("buttonscli.previous.log"));
    }
    std::fs::File::create(current)
}

fn install_panic_hook(dir: PathBuf) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = crash_report(info);
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let path = dir.join(format!("crash-{seconds}.txt"));
        let written = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::File::create(&path))
            .and_then(|mut file| file.write_all(report.as_bytes()));
        tracing::error!(
            report = %path.display(),
            saved = written.is_ok(),
            "panic: {}",
            panic_message(info)
        );
        default(info);
    }));
}

fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".into())
}

fn crash_report(info: &std::panic::PanicHookInfo<'_>) -> String {
    let location = info
        .location()
        .map_or_else(|| "unknown".into(), |location| location.to_string());
    let thread = std::thread::current();
    format!(
        "ButtonsCLI crash report\n\
         version: {}\n\
         os: {} {}\n\
         thread: {}\n\
         location: {location}\n\
         message: {}\n\n\
         backtrace:\n{}\n",
        crate::BUILD_VERSION,
        std::env::consts::OS,
        std::env::consts::ARCH,
        thread.name().unwrap_or("unnamed"),
        panic_message(info),
        std::backtrace::Backtrace::force_capture(),
    )
}

/// Keeps the newest `keep` crash reports (by file name, which sorts by time).
fn prune_crash_reports(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut reports: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("crash-") && name.ends_with(".txt"))
        })
        .collect();
    // Same-length numeric names sort by time; sort by length first for safety.
    reports.sort_by_key(|path| {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        (name.len(), name)
    });
    let excess = reports.len().saturating_sub(keep);
    for path in reports.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_rotation_and_crash_pruning_stay_inside_their_folder() {
        let dir = std::env::temp_dir().join(format!("buttonscli-logs-{:016x}", fastrand::u64(..)));
        drop(open_log(&dir).unwrap());
        std::fs::write(dir.join("buttonscli.log"), "first launch").unwrap();
        drop(open_log(&dir).unwrap());
        assert_eq!(
            std::fs::read_to_string(dir.join("buttonscli.previous.log")).unwrap(),
            "first launch"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("buttonscli.log")).unwrap(),
            ""
        );
        for seconds in [900, 1000, 1100, 99] {
            std::fs::write(dir.join(format!("crash-{seconds}.txt")), "x").unwrap();
        }
        prune_crash_reports(&dir, 2);
        let mut left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("crash-"))
            .collect();
        left.sort();
        assert_eq!(left, ["crash-1000.txt", "crash-1100.txt"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
