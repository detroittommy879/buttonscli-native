//! Settings → Release picks: choose personal themes, saved shaders and theme
//! bundles to ship with the app. "Copy into built-ins" writes them into a
//! source checkout's `assets/` (themes into `assets/themes/`, the rest into
//! `assets/library/`); the next build embeds them. Shown in debug builds, or
//! with `"release_picks": true` in `feature-flags.json`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::*;
use crate::settings::ThemeBundle;
use crate::theme::SavedShader;

#[derive(Default)]
pub(super) struct ReleasePicks {
    themes: BTreeSet<String>,
    shaders: BTreeSet<String>,
    bundles: BTreeSet<String>,
    repo: String,
    message: Option<String>,
}

pub(super) fn available() -> bool {
    cfg!(debug_assertions) || crate::features::local::release_picks_enabled()
}

/// The nearest folder above the working directory or the executable that
/// holds this app's source (`Cargo.toml` and `assets/themes`).
fn find_checkout() -> Option<PathBuf> {
    let starts = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf)),
    ];
    starts.into_iter().flatten().find_map(|start| {
        start
            .ancestors()
            .find(|dir| is_checkout(dir))
            .map(Path::to_path_buf)
    })
}

fn is_checkout(dir: &Path) -> bool {
    dir.join("Cargo.toml").is_file() && dir.join("assets").join("themes").is_dir()
}

/// What "Copy into built-ins" wrote, for the status line and tests.
#[derive(Debug, Default, PartialEq)]
pub(super) struct ExportSummary {
    pub(super) themes: Vec<String>,
    pub(super) shaders: usize,
    pub(super) bundles: usize,
    /// Bundle theme references that point at personal themes left out.
    pub(super) unresolved: usize,
}

impl ButtonsApp {
    pub(super) fn release_picks_settings(&mut self, ui: &mut egui::Ui) {
        ui.spaced_heading("Release picks");
        ui.spaced_label("Pick personal themes, saved shaders and theme bundles to ship with the app. \"Copy into built-ins\" writes them into the source checkout's assets; build and commit to include them. Bundles keep pointing at the picked themes.");
        if self.release_picks.repo.is_empty() {
            if let Some(found) = find_checkout() {
                self.release_picks.repo = found.display().to_string();
            }
        }
        wrapping_row(ui, |ui| {
            ui.spaced_label("Source checkout");
            ui.add(
                egui::TextEdit::singleline(&mut self.release_picks.repo)
                    .hint_text(r"e.g. G:\z\buttonscli-native")
                    .desired_width(360.0),
            );
        });
        let repo = PathBuf::from(self.release_picks.repo.trim());
        if !is_checkout(&repo) {
            ui.spaced_colored_label(
                self.colors().warning,
                "Not a ButtonsCLI source folder (needs Cargo.toml and assets/themes).",
            );
        }

        let personal: Vec<(String, String)> = self
            .themes
            .all()
            .iter()
            .filter(|theme| theme.id.starts_with("personal:"))
            .map(|theme| (theme.id.clone(), theme.name.clone()))
            .collect();
        let picks = &mut self.release_picks;
        pick_list(
            ui,
            "Themes",
            personal.iter().map(|(id, name)| (id.clone(), name.clone())),
            &mut picks.themes,
        );
        pick_list(
            ui,
            "Shaders",
            self.preferences
                .saved_shaders
                .iter()
                .map(|item| (item.shader.name.clone(), item.shader.name.clone())),
            &mut picks.shaders,
        );
        pick_list(
            ui,
            "Theme bundles",
            self.preferences
                .theme_bundles
                .iter()
                .map(|bundle| (bundle.name.clone(), bundle.name.clone())),
            &mut picks.bundles,
        );
        if ui
            .spaced_button("Also pick the themes these bundles use")
            .clicked()
        {
            for bundle in &self.preferences.theme_bundles {
                if self.release_picks.bundles.contains(&bundle.name) {
                    for id in bundle_theme_ids(bundle) {
                        if id.starts_with("personal:") {
                            self.release_picks.themes.insert(id.clone());
                        }
                    }
                }
            }
        }

        wrapping_row(ui, |ui| {
            if ui
                .add_enabled(
                    is_checkout(&repo),
                    egui::Button::new("Copy into built-ins"),
                )
                .on_hover_text("Write the picks into assets/themes and assets/library. Same names are replaced.")
                .clicked()
            {
                self.release_picks.message = Some(match self.export_release_picks(&repo) {
                    Ok(summary) => {
                        let mut text = format!(
                            "Copied {} theme(s), {} shader(s) and {} bundle(s). Rebuild to see them as built-ins, then commit assets/.",
                            summary.themes.len(),
                            summary.shaders,
                            summary.bundles
                        );
                        if summary.unresolved > 0 {
                            text.push_str(&format!(
                                " {} bundle theme reference(s) point at personal themes you did not pick; those parts will be skipped for other people.",
                                summary.unresolved
                            ));
                        }
                        text
                    }
                    Err(error) => format!("Copy failed: {error}"),
                });
            }
            if ui
                .spaced_button("Save list as JSON")
                .on_hover_text("Write release-picks.json (names only) next to your profile, to send or review.")
                .clicked()
            {
                self.release_picks.message = Some(match self.save_release_picks_list() {
                    Ok(path) => format!("Saved {}", path.display()),
                    Err(error) => format!("Save failed: {error}"),
                });
            }
        });
        if let Some(message) = &self.release_picks.message {
            ui.spaced_label(message);
        }
    }

    fn save_release_picks_list(&self) -> Result<PathBuf, String> {
        let store = self
            .native_store
            .as_ref()
            .ok_or("Native profile storage is unavailable.")?;
        let names = |ids: &BTreeSet<String>| -> Vec<String> {
            ids.iter()
                .map(|id| self.themes.get(id).name.clone())
                .collect()
        };
        let list = json!({
            "themes": names(&self.release_picks.themes),
            "themeIds": self.release_picks.themes,
            "shaders": self.release_picks.shaders,
            "bundles": self.release_picks.bundles,
        });
        let path = store.profile_dir().join("release-picks.json");
        std::fs::write(&path, serde_json::to_string_pretty(&list).unwrap())
            .map_err(|error| error.to_string())?;
        Ok(path)
    }

    pub(super) fn export_release_picks(&self, repo: &Path) -> Result<ExportSummary, String> {
        if !is_checkout(repo) {
            return Err("that folder is not a ButtonsCLI source checkout".into());
        }
        let themes_dir = repo.join("assets").join("themes");
        let library = repo.join("assets").join("library");
        std::fs::create_dir_all(&library).map_err(|error| error.to_string())?;
        let mut summary = ExportSummary::default();

        // Themes: personal "personal:<profile>:<stem>" becomes built-in "<stem>".
        let built_in: BTreeSet<&str> = self
            .themes
            .all()
            .iter()
            .filter(|theme| !theme.id.starts_with("personal:") && !theme.id.starts_with("preview:"))
            .map(|theme| theme.id.as_str())
            .collect();
        let mut renamed: BTreeMap<String, String> = BTreeMap::new();
        for id in &self.release_picks.themes {
            let Some(mut document) = self.themes.personal_document(id).cloned() else {
                continue;
            };
            let base = crate::theme_files::suggested_file_name(&self.themes.get(id).name)
                .trim_end_matches(".json")
                .to_owned();
            // Re-exporting the same theme replaces its file; a different
            // built-in with that id gets a numbered name instead.
            let mut stem = base.clone();
            let mut number = 2;
            while built_in.contains(stem.as_str())
                && !same_theme(&themes_dir.join(format!("{stem}.json")), &document)
            {
                stem = format!("{base}-{number}");
                number += 1;
            }
            document["metadata"]["id"] = json!(stem);
            let text =
                serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
            std::fs::write(themes_dir.join(format!("{stem}.json")), text + "\n")
                .map_err(|error| error.to_string())?;
            renamed.insert(id.clone(), stem.clone());
            summary.themes.push(stem);
        }

        // Shaders and bundles merge into the library by name.
        let picked_shaders: Vec<SavedShader> = self
            .preferences
            .saved_shaders
            .iter()
            .filter(|item| self.release_picks.shaders.contains(&item.shader.name))
            .cloned()
            .collect();
        summary.shaders = picked_shaders.len();
        merge_library(&library.join("shaders.json"), picked_shaders, |item| {
            item.shader.name.clone()
        })?;
        let mut picked_bundles = Vec::new();
        for bundle in &self.preferences.theme_bundles {
            if !self.release_picks.bundles.contains(&bundle.name) {
                continue;
            }
            let mut bundle = bundle.clone();
            for id in bundle_theme_ids_mut(&mut bundle) {
                if let Some(stem) = renamed.get(id.as_str()) {
                    id.clone_from(stem);
                } else if id.starts_with("personal:") {
                    summary.unresolved += 1;
                }
            }
            picked_bundles.push(bundle);
        }
        summary.bundles = picked_bundles.len();
        merge_library(&library.join("bundles.json"), picked_bundles, |bundle| {
            bundle.name.clone()
        })?;
        Ok(summary)
    }
}

fn same_theme(path: &Path, document: &Value) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|existing| existing["metadata"]["name"] == document["metadata"]["name"])
}

fn bundle_theme_ids(bundle: &ThemeBundle) -> Vec<&String> {
    [
        &bundle.theme_id,
        &bundle.app_theme_id,
        &bundle.terminal_theme_id,
        &bundle.gradient_theme_id,
        &bundle.effects_theme_id,
    ]
    .into_iter()
    .chain(
        bundle
            .panes
            .iter()
            .filter_map(|look| look.theme_id.as_ref()),
    )
    .collect()
}

fn bundle_theme_ids_mut(bundle: &mut ThemeBundle) -> Vec<&mut String> {
    [
        &mut bundle.theme_id,
        &mut bundle.app_theme_id,
        &mut bundle.terminal_theme_id,
        &mut bundle.gradient_theme_id,
        &mut bundle.effects_theme_id,
    ]
    .into_iter()
    .chain(
        bundle
            .panes
            .iter_mut()
            .filter_map(|look| look.theme_id.as_mut()),
    )
    .collect()
}

/// Replaces entries with the same key and appends new ones, keeping the rest.
fn merge_library<T: serde::Serialize + serde::de::DeserializeOwned>(
    path: &Path,
    picked: Vec<T>,
    key: impl Fn(&T) -> String,
) -> Result<(), String> {
    if picked.is_empty() {
        return Ok(());
    }
    let mut existing: Vec<T> = match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("{} is not valid: {error}", path.display()))?,
        Err(_) => Vec::new(),
    };
    for item in picked {
        let name = key(&item);
        match existing.iter_mut().find(|old| key(old) == name) {
            Some(old) => *old = item,
            None => existing.push(item),
        }
    }
    let text = serde_json::to_string_pretty(&existing).map_err(|error| error.to_string())?;
    std::fs::write(path, text + "\n").map_err(|error| error.to_string())
}

/// A titled, collapsible list of checkboxes keyed by `id`.
fn pick_list(
    ui: &mut egui::Ui,
    title: &str,
    items: impl Iterator<Item = (String, String)>,
    picked: &mut BTreeSet<String>,
) {
    let items: Vec<(String, String)> = items.collect();
    egui::CollapsingHeader::new(format!("{title} ({} of {})", picked.len(), items.len()))
        .id_salt(("release-picks", title))
        .default_open(true)
        .show(ui, |ui| {
            if items.is_empty() {
                ui.spaced_label(RichText::new("None saved yet.").weak());
            }
            for (id, name) in items {
                let mut on = picked.contains(&id);
                if ui.checkbox(&mut on, name).changed() {
                    if on {
                        picked.insert(id);
                    } else {
                        picked.remove(&id);
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::PaneLook;

    #[test]
    fn picks_copy_into_a_checkout_and_bundles_point_at_the_copied_themes() {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-release-picks-{:032x}",
            fastrand::u128(..)
        ));
        let repo = base.join("repo");
        std::fs::create_dir_all(repo.join("assets").join("themes")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\n").unwrap();
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("legacy"),
        )
        .unwrap();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        // One personal theme, saved the normal way.
        app.sync_theme_editor_to_current();
        let mut draft = app.theme_editor_document.clone().unwrap();
        draft["metadata"]["name"] = json!("Night Picks");
        app.theme_editor_document = Some(draft);
        app.name_draft_as_next_version();
        app.save_personal_theme_draft();
        let personal = app
            .themes
            .all()
            .iter()
            .find(|theme| theme.id.starts_with("personal:"))
            .unwrap()
            .id
            .clone();
        app.preferences.saved_shaders.push(SavedShader {
            shader: crate::theme::CustomShader {
                name: "Glow".into(),
                ..Default::default()
            },
            colors: Some(vec!["#ff0000".into(), "#00ff00".into()]),
        });
        app.preferences.theme_bundles.push(ThemeBundle {
            name: "Trio".into(),
            theme_id: "basic2".into(),
            app_theme_id: "basic2".into(),
            terminal_theme_id: personal.clone(),
            gradient_theme_id: "basic2".into(),
            effects_theme_id: "personal:default:left-out".into(),
            panes: vec![PaneLook {
                theme_id: Some(personal.clone()),
                ..Default::default()
            }],
            ..Default::default()
        });
        app.release_picks.themes.insert(personal.clone());
        app.release_picks.shaders.insert("Glow".into());
        app.release_picks.bundles.insert("Trio".into());

        let summary = app.export_release_picks(&repo).unwrap();
        assert_eq!(summary.themes.len(), 1);
        assert_eq!(
            (summary.shaders, summary.bundles, summary.unresolved),
            (1, 1, 1)
        );
        let stem = &summary.themes[0];
        let theme = std::fs::read_to_string(
            repo.join("assets")
                .join("themes")
                .join(format!("{stem}.json")),
        )
        .unwrap();
        let parsed = crate::theme::parse_bundled_theme(stem, &theme).unwrap();
        assert_eq!(parsed.name, app.themes.get(&personal).name);
        let library = repo.join("assets").join("library");
        let shaders: Vec<SavedShader> =
            serde_json::from_slice(&std::fs::read(library.join("shaders.json")).unwrap()).unwrap();
        assert_eq!(shaders[0].colors.as_ref().unwrap().len(), 2);
        let bundles: Vec<ThemeBundle> =
            serde_json::from_slice(&std::fs::read(library.join("bundles.json")).unwrap()).unwrap();
        assert_eq!(&bundles[0].terminal_theme_id, stem);
        assert_eq!(bundles[0].panes[0].theme_id.as_ref(), Some(stem));

        // Copying again replaces by name instead of adding duplicates.
        app.export_release_picks(&repo).unwrap();
        let shaders: Vec<SavedShader> =
            serde_json::from_slice(&std::fs::read(library.join("shaders.json")).unwrap()).unwrap();
        assert_eq!(shaders.len(), 1);
        assert_eq!(
            std::fs::read_dir(repo.join("assets").join("themes"))
                .unwrap()
                .count(),
            1
        );
        std::fs::remove_dir_all(base).unwrap();
    }
}
