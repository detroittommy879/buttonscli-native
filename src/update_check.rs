//! Looks once at startup for a newer ButtonsCLI release on GitHub. It never
//! downloads or installs anything: a newer version only shows a green link
//! to the releases page in the status bar. Settings → Workspace switches it
//! off, and so does `BUTTONSCLI_NO_UPDATE_CHECK=1`.
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Where the status bar link goes when the release has no usable page.
pub(crate) const RELEASES_URL: &str =
    "https://github.com/detroittommy879/buttonscli-native/releases/latest";
const API_URL: &str =
    "https://api.github.com/repos/detroittommy879/buttonscli-native/releases/latest";
const RELEASE_PAGES: &str = "https://github.com/detroittommy879/buttonscli-native/releases/";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Update {
    /// The new version, without a leading `v`.
    pub(crate) version: String,
    /// Its GitHub release page.
    pub(crate) url: String,
}

/// The release version when it is newer than `current`. Build metadata
/// (`1.4.2+abc123` on development builds) is ignored, and so are
/// pre-releases and tags that are not versions.
pub(crate) fn newer_release(current: &str, tag: &str) -> Option<semver::Version> {
    let mut current = semver::Version::parse(current).ok()?;
    current.build = semver::BuildMetadata::EMPTY;
    let release = semver::Version::parse(tag.trim().trim_start_matches(['v', 'V'])).ok()?;
    (release.pre.is_empty() && release > current).then_some(release)
}

/// Reads GitHub's "latest release" answer (which leaves out drafts and
/// pre-releases) into an update, when it is newer than `current`.
pub(crate) fn update_from_release(current: &str, release: &serde_json::Value) -> Option<Update> {
    if release["draft"].as_bool() == Some(true) || release["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let version = newer_release(current, release["tag_name"].as_str()?)?;
    let url = release["html_url"]
        .as_str()
        .filter(|url| url.starts_with(RELEASE_PAGES))
        .unwrap_or(RELEASES_URL)
        .to_owned();
    Some(Update {
        version: version.to_string(),
        url,
    })
}

/// Starts the check on a background thread. The receiver gets one update
/// when a newer release exists; failures (offline, rate limits) say nothing.
pub(crate) fn spawn(ctx: egui::Context) -> Option<Receiver<Update>> {
    if std::env::var("BUTTONSCLI_NO_UPDATE_CHECK").is_ok_and(|value| value == "1") {
        return None;
    }
    let (sender, receiver) = channel();
    let spawned = std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            let release = (|| {
                reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .user_agent("ButtonsCLI-update-check")
                    .build()
                    .ok()?
                    .get(API_URL)
                    .header("Accept", "application/vnd.github+json")
                    .send()
                    .ok()?
                    .error_for_status()
                    .ok()?
                    .json::<serde_json::Value>()
                    .ok()
            })();
            match release.and_then(|release| update_from_release(crate::BUILD_VERSION, &release)) {
                Some(update) => {
                    log::info!("a newer release is available: {}", update.version);
                    let _ = sender.send(update);
                    ctx.request_repaint();
                }
                None => log::debug!("no newer release found"),
            }
        });
    spawned.ok().map(|_| receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_newer_full_releases_count() {
        let newer = |current, tag| newer_release(current, tag).map(|v| v.to_string());
        assert_eq!(newer("1.0.0", "v1.1.0").as_deref(), Some("1.1.0"));
        assert_eq!(newer("1.0.0+2529bffc", "v1.0.1").as_deref(), Some("1.0.1"));
        assert_eq!(newer("1.0.0+2529bffc", "v1.0.0"), None, "a dev build of it");
        assert_eq!(newer("1.2.0", "v1.1.9"), None);
        assert_eq!(newer("1.0.0", "v2.0.0-beta.1"), None);
        assert_eq!(newer("1.0.0", "nightly"), None);
    }

    #[test]
    fn release_answers_become_a_link_to_github() {
        let release = json!({
            "tag_name": "v1.2.0",
            "html_url": "https://github.com/detroittommy879/buttonscli-native/releases/tag/v1.2.0",
            "draft": false,
            "prerelease": false
        });
        let update = update_from_release("1.1.0", &release).unwrap();
        assert_eq!(update.version, "1.2.0");
        assert!(update.url.ends_with("/releases/tag/v1.2.0"));
        let mut elsewhere = release.clone();
        elsewhere["html_url"] = json!("https://example.com/download");
        assert_eq!(
            update_from_release("1.1.0", &elsewhere).unwrap().url,
            RELEASES_URL
        );
        let mut draft = release;
        draft["draft"] = json!(true);
        assert_eq!(update_from_release("1.1.0", &draft), None);
    }
}
