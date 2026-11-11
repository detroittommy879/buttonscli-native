//! Docs screenshots, refreshed by one command.
//!
//! The UI records a few named rectangles ("anchors") while a capture runs;
//! the ignored `native_docs_capture` probe drives the app through the shot
//! list below, crops each screenshot to its anchors and writes it to
//! `docs/guide/images/` only when the picture really changed.
//! Contributor guide: `docs/dev/DOCS-SCREENSHOTS.md`.

#[cfg(all(test, not(target_arch = "wasm32")))]
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// Shader time in every docs shot (terminals and previews): one busy, not
/// blank, frame, so reruns give the same pictures.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) const FROZEN_TIME: f64 = 42.0;

/// True while the docs capture probe runs. Anchors are free otherwise.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn capturing() -> bool {
    CAPTURING.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(all(test, windows)), allow(dead_code))]
struct Anchor {
    rect: egui::Rect,
    pass: u64,
}

fn anchor_id(name: &str) -> egui::Id {
    egui::Id::new(("docs-capture-anchor", name))
}

fn scroll_target_id() -> egui::Id {
    egui::Id::new("docs-capture-scroll-target")
}

/// Names `rect` for docs screenshots. When the current shot asks to scroll
/// to this anchor, the enclosing scroll area brings it into view.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn anchor(ui: &egui::Ui, name: &'static str, rect: egui::Rect) {
    if !capturing() {
        return;
    }
    anchor_at(ui.ctx(), name, rect);
    let wanted = ui
        .ctx()
        .data(|data| data.get_temp::<&'static str>(scroll_target_id()));
    if wanted == Some(name) {
        ui.scroll_to_rect(rect, Some(egui::Align::Center));
    }
}

/// [`anchor`] for rectangles known outside a `Ui`, such as whole windows.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn anchor_at(ctx: &egui::Context, name: &'static str, rect: egui::Rect) {
    if !capturing() {
        return;
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(anchor_id(name), Anchor { rect, pass }));
}

/// The anchor's rectangle if it was drawn in this pass. Panels count too:
/// `tab-bar`, `command-dock`, `status-bar` and `menu-bar`.
#[cfg(all(test, windows))]
fn anchor_rect(ctx: &egui::Context, name: &str) -> Option<egui::Rect> {
    let panel = match name {
        "tab-bar" => Some("tabs"),
        "command-dock" => Some("presets"),
        "status-bar" => Some("status"),
        "menu-bar" => Some("menu"),
        _ => None,
    };
    if let Some(panel) = panel {
        return egui::containers::panel::PanelState::load(ctx, egui::Id::new(panel))
            .map(|state| state.rect);
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|data| data.get_temp::<Anchor>(anchor_id(name)))
        .filter(|anchor| anchor.pass == pass)
        .map(|anchor| anchor.rect)
}

// ---------------------------------------------------------------------------
// Shot list
// ---------------------------------------------------------------------------

/// One screenshot in `docs/guide/images/`. Several parts are placed side by
/// side (the layouts comparison); most shots have one.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) struct Shot {
    pub file: &'static str,
    pub parts: &'static [Part],
}

/// One capture: window size, setup, clicks, then a crop.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[derive(Clone, Copy)]
// Only the Windows probe reads the fields; other targets check the list.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) struct Part {
    /// Layout size of the window in points.
    pub size: [f32; 2],
    /// Pixels per point, the same on every machine.
    pub scale: f32,
    pub setup: fn(&mut ButtonsApp, &egui::Context),
    /// Runs once the first frames have drawn (bundles need the panes laid
    /// out), then the window settles again.
    pub then: Option<fn(&mut ButtonsApp, &egui::Context)>,
    /// Anchors clicked in order once the window settles (menus, dropdowns).
    pub clicks: &'static [&'static str],
    /// An anchor to scroll into view first.
    pub scroll_to: Option<&'static str>,
    /// Crop to the union of these anchors; empty keeps the whole window.
    pub crop: &'static [&'static str],
    /// Padding around the crop, in points.
    pub pad: f32,
    /// Labelled outlines drawn over anchors: (anchor, label).
    pub callouts: &'static [(&'static str, &'static str)],
    /// Shrink the crop to this many pixels wide (overviews that need a big
    /// window, such as four panes in a row).
    pub fit_width: Option<u32>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
const WINDOW: Part = Part {
    size: [1280.0, 760.0],
    scale: 1.0,
    setup: setup::tour,
    then: None,
    clicks: &[],
    scroll_to: None,
    crop: &[],
    pad: 0.0,
    callouts: &[],
    fit_width: None,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
const SETTINGS: Part = Part {
    size: [1280.0, 900.0],
    crop: &["settings-window"],
    ..WINDOW
};

#[cfg(all(test, not(target_arch = "wasm32")))]
const LAYOUT: Part = Part {
    // Four columns or rows need room: each pane keeps 40×8 cells.
    size: [1400.0, 920.0],
    fit_width: Some(470),
    crop: &["workspace"],
    ..WINDOW
};

/// Every automated shot. Each `file` must have an `<!-- IMAGE: file … -->`
/// marker in `docs/guide` (checked by `shot_list_matches_image_markers`).
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) const SHOTS: &[Shot] = &[
    Shot {
        file: "hero.jpg",
        parts: &[Part {
            setup: setup::grid,
            then: Some(setup::bundle_hero),
            ..WINDOW
        }],
    },
    Shot {
        file: "window-tour.png",
        parts: &[Part {
            callouts: &[
                ("tab-bar", "1  Tabs"),
                ("command-dock", "2  Command dock"),
                ("workspace", "3  Panes"),
                ("status-bar", "4  Status bar"),
            ],
            ..WINDOW
        }],
    },
    Shot {
        file: "layouts.png",
        parts: &[
            Part {
                setup: setup::columns,
                ..LAYOUT
            },
            Part {
                setup: setup::rows,
                ..LAYOUT
            },
            Part {
                setup: setup::grid,
                ..LAYOUT
            },
        ],
    },
    Shot {
        file: "button-editor.png",
        parts: &[Part {
            size: [1100.0, 700.0],
            scale: 1.5,
            setup: setup::button_editor,
            crop: &["preset-editor"],
            pad: 12.0,
            ..WINDOW
        }],
    },
    Shot {
        file: "theme-library.png",
        parts: &[Part {
            setup: setup::library,
            ..SETTINGS
        }],
    },
    Shot {
        file: "edit-for-row.png",
        parts: &[Part {
            scale: 2.0,
            setup: setup::theme_editor,
            crop: &["edit-for-row"],
            pad: 8.0,
            ..SETTINGS
        }],
    },
    Shot {
        file: "theme-editor.png",
        parts: &[Part {
            setup: setup::theme_editor,
            ..SETTINGS
        }],
    },
    Shot {
        file: "bundle-applied.jpg",
        parts: &[Part {
            setup: setup::grid,
            then: Some(setup::bundle_other),
            ..WINDOW
        }],
    },
    Shot {
        file: "shaders-page.png",
        parts: &[Part {
            setup: setup::two,
            then: Some(setup::shaders),
            ..SETTINGS
        }],
    },
    Shot {
        file: "pattern-picker.png",
        parts: &[Part {
            scale: 1.5,
            setup: setup::two,
            then: Some(setup::shaders),
            clicks: &["shader-pattern"],
            crop: &["shader-pattern-row", "shader-pattern-popup"],
            pad: 10.0,
            ..SETTINGS
        }],
    },
    Shot {
        file: "colors-row.png",
        parts: &[Part {
            scale: 2.0,
            setup: setup::two,
            then: Some(setup::shaders),
            scroll_to: Some("shader-colors-row"),
            crop: &["shader-colors-row"],
            pad: 8.0,
            ..SETTINGS
        }],
    },
    Shot {
        file: "status-shader-menu.png",
        parts: &[Part {
            size: [1280.0, 800.0],
            scale: 1.5,
            setup: setup::two,
            then: Some(setup::bundle_hero),
            clicks: &["status-shader-button"],
            crop: &["status-shader-button", "status-shader-menu"],
            pad: 12.0,
            ..WINDOW
        }],
    },
];

/// Markers that still need a person (or state the probe cannot fake yet).
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) const MANUAL: &[(&str, &str)] = &[
    (
        "create-with-ai.png",
        "needs four finished AI versions and their flowcharts",
    ),
    ("random-ideas.png", "needs an AI-written ideas list"),
    ("generate-tab.png", "needs a theme generation in progress"),
    ("chatgpt-signin.png", "needs a signed-in ChatGPT provider"),
];

/// Setup steps. Each starts from a fresh app in an owned profile with
/// frozen shader time; see `fresh_app`.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod setup {
    use super::*;

    /// Synthetic terminal content: no real hosts, paths or names.
    const PANES: [(&str, &str); 4] = [
        (
            "api",
            "\x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/api\x1b[0m$ cargo test\r\n\
             \x1b[32m   Compiling\x1b[0m api v0.4.2\r\n\
             \x1b[32m    Finished\x1b[0m `test` profile in 3.41s\r\n\
             \x1b[32m     Running\x1b[0m unittests src/lib.rs\r\n\r\n\
             running 6 tests\r\n\
             test routes::health ... \x1b[32mok\x1b[0m\r\n\
             test routes::login_rejects_bad_token ... \x1b[32mok\x1b[0m\r\n\
             test store::migrations_apply_in_order ... \x1b[32mok\x1b[0m\r\n\
             test store::rollback_is_atomic ... \x1b[32mok\x1b[0m\r\n\
             test jobs::retry_backs_off ... \x1b[32mok\x1b[0m\r\n\
             test jobs::queue_drains ... \x1b[32mok\x1b[0m\r\n\r\n\
             test result: \x1b[32mok\x1b[0m. 6 passed; 0 failed\r\n\r\n\
             \x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/api\x1b[0m$ ",
        ),
        (
            "web",
            "\x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/web\x1b[0m$ npm run dev\r\n\r\n\
             > web@1.8.0 dev\r\n> vite\r\n\r\n\
             \x1b[1;32m  VITE v6.0.3\x1b[0m  ready in \x1b[1m412\x1b[0m ms\r\n\r\n\
             \x1b[32m  ➜\x1b[0m  \x1b[1mLocal\x1b[0m:   \x1b[36mhttp://localhost:5173/\x1b[0m\r\n\
             \x1b[2m  ➜  Network: use --host to expose\x1b[0m\r\n\r\n\
             \x1b[2m10:24:07\x1b[0m \x1b[36m[vite]\x1b[0m page reload \x1b[2msrc/App.tsx\x1b[0m\r\n\
             \x1b[2m10:24:31\x1b[0m \x1b[36m[vite]\x1b[0m hmr update \x1b[2m/src/theme.css\x1b[0m\r\n",
        ),
        (
            "git",
            "\x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/api\x1b[0m$ git log --oneline --graph\r\n\
             * \x1b[33m3f9c2a1\x1b[0m \x1b[36m(HEAD -> main)\x1b[0m feat: rate limits per token\r\n\
             * \x1b[33mb71e0d4\x1b[0m fix: retry jobs with backoff\r\n\
             *   \x1b[33m9a40c55\x1b[0m Merge branch 'store-migrations'\r\n\
             |\\  \r\n\
             | * \x1b[33m2d18e7f\x1b[0m test: migrations apply in order\r\n\
             | * \x1b[33m6c5b913\x1b[0m feat: versioned migrations\r\n\
             |/  \r\n\
             * \x1b[33me0f7a62\x1b[0m docs: API quick start\r\n\
             * \x1b[33m41ab3c8\x1b[0m chore: first commit\r\n\r\n\
             \x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/api\x1b[0m$ ",
        ),
        (
            "logs",
            "\x1b[32mdemo@workstation\x1b[0m:\x1b[34m~/api\x1b[0m$ tail -f logs/app.log\r\n\
             \x1b[2m10:24:01\x1b[0m \x1b[32mINFO \x1b[0m server listening on :8080\r\n\
             \x1b[2m10:24:03\x1b[0m \x1b[32mINFO \x1b[0m GET /health 200 2ms\r\n\
             \x1b[2m10:24:09\x1b[0m \x1b[32mINFO \x1b[0m POST /login 200 41ms\r\n\
             \x1b[2m10:24:12\x1b[0m \x1b[33mWARN \x1b[0m slow query: 312ms (orders)\r\n\
             \x1b[2m10:24:15\x1b[0m \x1b[32mINFO \x1b[0m GET /orders 200 18ms\r\n\
             \x1b[2m10:24:20\x1b[0m \x1b[31mERROR\x1b[0m job 42 failed, retrying in 4s\r\n\
             \x1b[2m10:24:24\x1b[0m \x1b[32mINFO \x1b[0m job 42 done\r\n",
        ),
    ];

    /// Opens `count` simulated panes (never real shells) in `layout`.
    pub(super) fn panes(
        app: &mut ButtonsApp,
        ctx: &egui::Context,
        count: usize,
        layout: PaneLayout,
    ) {
        app.pane_layout = layout;
        for index in 0..count {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
            let (title, text) = PANES[index % PANES.len()];
            let tab = app.tabs.last_mut().unwrap();
            // Keep the virtual screen but drop the AI simulator, which would
            // ask a provider for its first screen.
            tab.simulated = None;
            tab.title = title.into();
            tab.backend
                .feed_virtual_output(format!("\x1b[2J\x1b[H{text}").as_bytes());
        }
        app.visible_panes = (0..count).collect();
        app.auto_tile.requested_count = count.clamp(1, 10);
        app.focused = 0;
    }

    fn bundle(app: &mut ButtonsApp, ctx: &egui::Context, name: &str) {
        assert!(
            app.apply_theme_bundle(ctx, name).is_some(),
            "built-in bundle {name:?} is missing; pick another in docs_capture.rs"
        );
    }

    fn settings(app: &mut ButtonsApp, tab: ThemeSettingsTab) {
        app.show_settings = true;
        app.settings_tab = SettingsTab::Themes;
        app.theme_settings_tab = tab;
    }

    /// The bundle in the hero picture and most shader shots.
    pub(super) fn bundle_hero(app: &mut ButtonsApp, ctx: &egui::Context) {
        bundle(app, ctx, "geometryflow");
    }

    pub(super) fn bundle_other(app: &mut ButtonsApp, ctx: &egui::Context) {
        bundle(app, ctx, "night_tripper");
    }

    pub(super) fn two(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 2, PaneLayout::Columns);
    }

    pub(super) fn tour(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 3, PaneLayout::Columns);
    }

    pub(super) fn columns(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 4, PaneLayout::Columns);
    }

    pub(super) fn rows(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 4, PaneLayout::Rows);
    }

    pub(super) fn grid(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 4, PaneLayout::Grid);
    }

    pub(super) fn button_editor(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 2, PaneLayout::Columns);
        app.open_add_preset_editor(PresetCollection::Commands);
        app.preset_label_draft = "Run tests".into();
        app.preset_command_draft = "cargo test --workspace".into();
    }

    pub(super) fn library(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 2, PaneLayout::Columns);
        settings(app, ThemeSettingsTab::Library);
    }

    pub(super) fn theme_editor(app: &mut ButtonsApp, ctx: &egui::Context) {
        panes(app, ctx, 3, PaneLayout::Columns);
        settings(app, ThemeSettingsTab::Edit);
    }

    /// Pane 1's own look in the Shaders tab, as the pane menu opens it.
    pub(super) fn shaders(app: &mut ButtonsApp, ctx: &egui::Context) {
        bundle_hero(app, ctx);
        app.theme_editor_pane_mode = true;
        settings(app, ThemeSettingsTab::Shaders);
    }
}

/// The `<!-- IMAGE: file … -->` markers in `docs/guide` (all languages).
#[cfg(all(test, not(target_arch = "wasm32")))]
fn image_markers() -> std::collections::BTreeSet<String> {
    fn walk(dir: &std::path::Path, found: &mut std::collections::BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "md")
                && path.file_name().is_some_and(|name| name != "IMAGES.md")
            {
                let text = std::fs::read_to_string(&path).unwrap();
                for (_, rest) in text
                    .match_indices("<!-- IMAGE:")
                    .map(|(at, _)| text.split_at(at + 11))
                {
                    if let Some(file) = rest.split_whitespace().next() {
                        found.insert(file.to_owned());
                    }
                }
            }
        }
    }
    let mut found = std::collections::BTreeSet::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/guide"),
        &mut found,
    );
    found
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn shot_list_matches_image_markers() {
    let markers = image_markers();
    assert!(!markers.is_empty(), "no IMAGE markers found in docs/guide");
    let mut listed = std::collections::BTreeSet::new();
    for file in SHOTS
        .iter()
        .map(|shot| shot.file)
        .chain(MANUAL.iter().map(|(file, _)| *file))
    {
        assert!(listed.insert(file.to_owned()), "{file} is listed twice");
        assert!(
            markers.contains(file),
            "{file} is in the docs shot list but no guide page has an IMAGE marker for it"
        );
    }
    let missing: Vec<_> = markers.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "IMAGE markers with no automated shot or MANUAL entry in src/app/docs_capture.rs: {missing:?}"
    );
    for shot in SHOTS {
        assert!(!shot.parts.is_empty(), "{} has no parts", shot.file);
        assert!(
            shot.file.ends_with(".png") || shot.file.ends_with(".jpg"),
            "{}: docs images are .png (UI) or .jpg (shaders)",
            shot.file
        );
    }
}

// ---------------------------------------------------------------------------
// Writing images
// ---------------------------------------------------------------------------

/// How much of the picture differs from the file already in the docs.
#[cfg(all(test, windows))]
fn changed_fraction(old: &image::RgbaImage, new: &image::RgbaImage, tolerance: u8) -> f64 {
    if old.dimensions() != new.dimensions() {
        return 1.0;
    }
    let changed = old
        .pixels()
        .zip(new.pixels())
        .filter(|(a, b)| {
            a.0.iter()
                .zip(b.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > tolerance)
        })
        .count();
    changed as f64 / f64::from(new.width() * new.height()).max(1.0)
}

#[cfg(all(test, windows))]
#[derive(serde::Serialize)]
struct ShotResult {
    file: String,
    /// new, changed, unchanged, rewritten (forced) or failed.
    status: &'static str,
    width: u32,
    height: u32,
    /// Percent of pixels that differ from the previous file.
    changed_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// The file's bytes: PNG for UI, JPG (quality 85) for shader-heavy shots.
#[cfg(all(test, windows))]
fn encode(picture: &image::RgbaImage, jpg: bool) -> image::ImageResult<Vec<u8>> {
    let mut bytes = Vec::new();
    if jpg {
        let rgb = image::DynamicImage::ImageRgba8(picture.clone()).to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85).encode_image(&rgb)?;
    } else {
        // Smallest PNG the encoder makes: these are committed and served.
        picture.write_with_encoder(image::codecs::png::PngEncoder::new_with_quality(
            &mut bytes,
            image::codecs::png::CompressionType::Best,
            image::codecs::png::FilterType::Adaptive,
        ))?;
    }
    Ok(bytes)
}

/// Writes `picture` unless the existing file looks the same: identical
/// bytes, or at most 0.05 % of pixels off by more than a small tolerance
/// (both sides decoded, so JPG noise compares like with like).
#[cfg(all(test, windows))]
fn write_if_changed(path: &std::path::Path, picture: &image::RgbaImage, force: bool) -> ShotResult {
    let jpg = path.extension().is_some_and(|ext| ext == "jpg");
    let mut result = ShotResult {
        file: path.file_name().unwrap().to_string_lossy().into_owned(),
        status: "new",
        width: picture.width(),
        height: picture.height(),
        changed_percent: 100.0,
        error: None,
    };
    let bytes = match encode(picture, jpg) {
        Ok(bytes) => bytes,
        Err(error) => {
            result.status = "failed";
            result.error = Some(error.to_string());
            return result;
        }
    };
    if let Ok(old_bytes) = std::fs::read(path) {
        let fraction = if old_bytes == bytes {
            0.0
        } else {
            match (
                image::load_from_memory(&old_bytes),
                image::load_from_memory(&bytes),
            ) {
                (Ok(old), Ok(new)) => changed_fraction(&old.to_rgba8(), &new.to_rgba8(), 16),
                _ => 1.0,
            }
        };
        result.changed_percent = (fraction * 10_000.0).round() / 100.0;
        if fraction > 0.0005 {
            result.status = "changed";
        } else if force {
            result.status = "rewritten";
        } else {
            result.status = "unchanged";
            return result;
        }
    }
    if let Err(error) = std::fs::write(path, &bytes) {
        result.status = "failed";
        result.error = Some(error.to_string());
    }
    result
}

// ---------------------------------------------------------------------------
// The probe
// ---------------------------------------------------------------------------

#[cfg(all(test, windows))]
mod probe {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    /// Frames an anchor must keep the same rectangle before the shot.
    const STABLE_FRAMES: u32 = 8;
    /// Time for shaders to compile and bundle looks to land.
    const MIN_SETTLE: Duration = Duration::from_millis(1500);
    const PART_TIMEOUT: Duration = Duration::from_secs(25);

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Phase {
        Setup,
        Resize,
        Settle,
        /// Press, release, leave; then settle again.
        Click(u8),
        Shoot,
    }

    #[derive(Clone, Copy)]
    struct Tag(usize, usize);

    struct Probe {
        shots: Vec<&'static Shot>,
        shot: usize,
        part: usize,
        app: Option<ButtonsApp>,
        phase: Phase,
        phase_at: Instant,
        clicks_done: usize,
        then_done: bool,
        stable: u32,
        last_rects: Vec<Option<egui::Rect>>,
        pending: Vec<egui::Event>,
        parts: Vec<image::RgbaImage>,
        failure: Option<String>,
        results: Vec<ShotResult>,
        root: PathBuf,
        out: PathBuf,
        work: PathBuf,
        locale: String,
        force: bool,
        crop_scale: f32,
        /// Shader backgrounds find the GPU format in egui's memory, which
        /// each shot clears; it is put back from here.
        target_format: Option<eframe::wgpu::TextureFormat>,
    }

    impl Probe {
        fn current(&self) -> Part {
            self.shots[self.shot].parts[self.part]
        }

        fn enter(&mut self, phase: Phase) {
            self.phase = phase;
            self.phase_at = Instant::now();
            self.stable = 0;
            self.last_rects.clear();
        }

        fn close_app(&mut self) {
            if let Some(app) = &mut self.app {
                while !app.tabs.is_empty() {
                    app.close_tab(0);
                }
            }
            self.app = None;
        }

        /// A new app for each part: owned profile, release-like UI, no
        /// onboarding, no network, shader time frozen.
        fn fresh_app(&mut self, ctx: &egui::Context) -> ButtonsApp {
            let dir = self.root.join(format!("{}-{}", self.shot, self.part));
            let store = NativeStore::open(
                crate::storage::paths::NativeDataRoot(dir.join("native")),
                dir.join("legacy"),
            )
            .unwrap();
            let mut preferences = Preferences {
                shader_backgrounds_enabled: true,
                show_sidebar: false,
                show_presets: true,
                ..Default::default()
            };
            preferences.simulation.show_notices = false;
            preferences.simulation.show_badges = false;
            preferences.localization.manual_locale = self.locale.clone();
            preferences.settings_layout.size = [1040.0, 820.0];
            preferences.settings_layout.position = None;
            // Nothing in a capture may reach a provider or the OS keychain:
            // an unknown provider id on a discard port.
            for provider in &mut preferences.provider_settings.providers {
                provider.id = "docs-capture-offline".into();
                provider.endpoint = "http://127.0.0.1:9/v1".into();
                provider.models_endpoint = None;
                provider.credential_ref = None;
            }
            preferences.provider_settings.active_provider_id = "docs-capture-offline".into();
            preferences.normalize_theme_sources();
            let mut app = ButtonsApp::empty(preferences);
            app.native_store = Some(store);
            app.show_localization_onboarding = false;
            app.motion_time = FROZEN_TIME;
            fonts::install(ctx, &app.font_catalog);
            app.apply_style(ctx);
            // The shader checker's rules (loops on): built-in shaders need them.
            app.apply_shader_lab();
            app
        }

        /// The crop's anchors plus the next click target and callouts.
        fn watched(&self) -> Vec<&'static str> {
            let part = self.current();
            // Crop anchors such as popups only exist after the clicks.
            let mut names: Vec<&'static str> = match part.clicks.get(self.clicks_done) {
                _ if part.then.is_some() && !self.then_done => Vec::new(),
                Some(click) => vec![click],
                None => part.crop.to_vec(),
            };
            names.extend(part.callouts.iter().map(|(name, _)| *name));
            names
        }

        fn fail(&mut self, error: String) {
            eprintln!("docs capture: {}: {error}", self.shots[self.shot].file);
            self.failure = Some(error);
            self.finish_shot();
        }

        fn finish_part(&mut self, picture: image::RgbaImage) {
            self.parts.push(picture);
            self.part += 1;
            self.close_app();
            if self.part == self.shots[self.shot].parts.len() {
                self.finish_shot();
            } else {
                self.enter(Phase::Setup);
            }
        }

        fn finish_shot(&mut self) {
            let file = self.shots[self.shot].file;
            let result = match self.failure.take() {
                Some(error) => ShotResult {
                    file: file.into(),
                    status: "failed",
                    width: 0,
                    height: 0,
                    changed_percent: 0.0,
                    error: Some(error),
                },
                None => {
                    let picture = side_by_side(std::mem::take(&mut self.parts));
                    write_if_changed(&self.out.join(file), &picture, self.force)
                }
            };
            println!(
                "docs capture: {:<24} {:<9} {}x{} ({}% changed)",
                result.file, result.status, result.width, result.height, result.changed_percent
            );
            self.results.push(result);
            self.parts.clear();
            self.close_app();
            self.shot += 1;
            self.part = 0;
            self.enter(Phase::Setup);
        }

        fn step(
            &mut self,
            ctx: &egui::Context,
            screenshot: Option<(Tag, std::sync::Arc<egui::ColorImage>)>,
        ) {
            if self.shot == self.shots.len() {
                return;
            }
            let part = self.current();
            if self.phase != Phase::Setup && self.phase_at.elapsed() > PART_TIMEOUT {
                let missing: Vec<_> = self
                    .watched()
                    .into_iter()
                    .filter(|name| anchor_rect(ctx, name).is_none())
                    .collect();
                let settings = self.app.as_ref().is_some_and(|app| app.show_settings);
                self.fail(format!(
                    "timed out in {:?}; anchors not drawn: {missing:?} (Settings open: {settings})",
                    self.phase
                ));
                return;
            }
            match self.phase {
                Phase::Setup => {
                    ctx.memory_mut(|memory| {
                        memory.data.clear();
                        memory.reset_areas();
                        memory.close_popup();
                    });
                    if let Some(format) = self.target_format {
                        crate::plugins::effects::shader_background::register(ctx, format);
                    }
                    ctx.set_pixels_per_point(part.scale);
                    if let Some(target) = part.scroll_to {
                        ctx.data_mut(|data| data.insert_temp(scroll_target_id(), target));
                    }
                    self.clicks_done = 0;
                    self.then_done = false;
                    self.enter(Phase::Resize);
                }
                Phase::Resize => {
                    // Sizes are in points at the current zoom, so wait for
                    // the new pixels-per-point before asking for the size.
                    if (ctx.pixels_per_point() - part.scale).abs() > 0.001 {
                        ctx.set_pixels_per_point(part.scale);
                    } else if (ctx.screen_rect().size() - egui::Vec2::from(part.size)).length()
                        < 1.5
                    {
                        // Build the app only now: windows that open on the
                        // first frame size themselves to the screen.
                        let mut app = self.fresh_app(ctx);
                        (part.setup)(&mut app, ctx);
                        self.app = Some(app);
                        self.enter(Phase::Settle);
                        return;
                    }
                    let elapsed = self.phase_at.elapsed();
                    if elapsed > Duration::from_secs(8) {
                        self.fail(format!(
                            "window stayed {:?} at {}x, wanted {:?} at {}x",
                            ctx.screen_rect().size(),
                            ctx.pixels_per_point(),
                            part.size,
                            part.scale
                        ));
                    } else if (ctx.pixels_per_point() - part.scale).abs() <= 0.001 {
                        // Ask again now and then: Windows may ignore one.
                        if self.stable % 60 == 0 {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                part.size.into(),
                            ));
                        }
                        self.stable += 1;
                    }
                }
                Phase::Settle => {
                    let rects: Vec<_> = self
                        .watched()
                        .iter()
                        .map(|name| anchor_rect(ctx, name))
                        .collect();
                    if rects.iter().all(Option::is_some) && rects == self.last_rects {
                        self.stable += 1;
                    } else {
                        self.stable = 0;
                    }
                    self.last_rects = rects;
                    if self.stable >= STABLE_FRAMES && self.phase_at.elapsed() >= MIN_SETTLE {
                        if let (Some(then), false) = (part.then, self.then_done) {
                            self.then_done = true;
                            then(self.app.as_mut().unwrap(), ctx);
                            self.enter(Phase::Settle);
                        } else if self.clicks_done < part.clicks.len() {
                            self.enter(Phase::Click(0));
                        } else {
                            self.crop_scale = ctx.pixels_per_point();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                                egui::UserData::new(Tag(self.shot, self.part)),
                            ));
                            self.enter(Phase::Shoot);
                        }
                    }
                }
                Phase::Click(stage) => {
                    let name = part.clicks[self.clicks_done];
                    let Some(rect) = anchor_rect(ctx, name) else {
                        return;
                    };
                    let pos = rect.center();
                    let button = |pressed| egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    };
                    match stage {
                        0 => self
                            .pending
                            .extend([egui::Event::PointerMoved(pos), button(true)]),
                        1 => self.pending.push(button(false)),
                        _ => self.pending.push(egui::Event::PointerGone),
                    }
                    if stage < 2 {
                        self.phase = Phase::Click(stage + 1);
                    } else {
                        self.clicks_done += 1;
                        self.enter(Phase::Settle);
                    }
                }
                Phase::Shoot => {
                    let Some((Tag(shot, part_index), image)) = screenshot else {
                        return;
                    };
                    if (shot, part_index) != (self.shot, self.part) {
                        return;
                    }
                    let full = image::RgbaImage::from_raw(
                        image.width() as u32,
                        image.height() as u32,
                        image
                            .pixels
                            .iter()
                            .flat_map(|pixel| pixel.to_array())
                            .collect(),
                    )
                    .unwrap();
                    let stem = std::path::Path::new(self.shots[shot].file)
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    let _ = full.save(self.work.join(format!("{stem}-{part_index}.png")));
                    let crop = if part.crop.is_empty() {
                        None
                    } else {
                        part.crop
                            .iter()
                            .map(|name| anchor_rect(ctx, name))
                            .collect::<Option<Vec<_>>>()
                            .map(|rects| {
                                rects
                                    .into_iter()
                                    .reduce(|a, b| a.union(b))
                                    .unwrap()
                                    .expand(part.pad)
                                    .intersect(ctx.screen_rect())
                            })
                    };
                    let picture = match crop {
                        None if !part.crop.is_empty() => {
                            self.fail("a crop anchor vanished before the screenshot".into());
                            return;
                        }
                        None => full,
                        Some(rect) => {
                            let scale = self.crop_scale;
                            let x = (rect.left() * scale).round().max(0.0) as u32;
                            let y = (rect.top() * scale).round().max(0.0) as u32;
                            let right = ((rect.right() * scale).round() as u32).min(full.width());
                            let bottom =
                                ((rect.bottom() * scale).round() as u32).min(full.height());
                            image::imageops::crop_imm(
                                &full,
                                x,
                                y,
                                right.saturating_sub(x),
                                bottom.saturating_sub(y),
                            )
                            .to_image()
                        }
                    };
                    let picture = match part.fit_width {
                        Some(width) if width < picture.width() => {
                            let height = (u64::from(picture.height()) * u64::from(width)
                                / u64::from(picture.width()))
                                as u32;
                            image::imageops::resize(
                                &picture,
                                width,
                                height.max(1),
                                image::imageops::FilterType::Lanczos3,
                            )
                        }
                        _ => picture,
                    };
                    self.finish_part(picture);
                }
            }
        }

        /// Numbered outlines for annotated shots, painted by the app's own
        /// fonts so they look like part of the UI.
        fn callouts(&self, ctx: &egui::Context) {
            let part = self.current();
            if part.callouts.is_empty() || matches!(self.phase, Phase::Setup | Phase::Resize) {
                return;
            }
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("docs-callouts"),
            ));
            let ring = Color32::from_rgb(0xff, 0xc8, 0x57);
            let font = egui::FontId::proportional(15.0);
            for (name, label) in part.callouts {
                let Some(rect) = anchor_rect(ctx, name) else {
                    continue;
                };
                let rect = rect.shrink(1.5);
                painter.rect_stroke(
                    rect,
                    4.0,
                    Stroke::new(2.0_f32, ring),
                    egui::StrokeKind::Inside,
                );
                let galley =
                    painter.layout_no_wrap((*label).to_owned(), font.clone(), Color32::BLACK);
                let size = galley.size() + egui::vec2(16.0, 6.0);
                // Bars: centered at the right end. Larger areas: the bottom
                // right corner, where terminal text rarely reaches.
                let top = if rect.height() < 60.0 {
                    rect.center().y - size.y / 2.0
                } else {
                    rect.bottom() - size.y - 10.0
                };
                let pill =
                    egui::Rect::from_min_size(egui::pos2(rect.right() - size.x - 10.0, top), size);
                painter.rect_filled(pill, size.y / 2.0, ring);
                painter.galley(pill.min + egui::vec2(8.0, 3.0), galley, Color32::BLACK);
            }
        }
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            self.close_app();
        }
    }

    /// Parts next to each other on the docs page background.
    fn side_by_side(parts: Vec<image::RgbaImage>) -> image::RgbaImage {
        if parts.len() == 1 {
            return parts.into_iter().next().unwrap();
        }
        const GAP: u32 = 16;
        let width =
            parts.iter().map(image::RgbaImage::width).sum::<u32>() + GAP * (parts.len() as u32 - 1);
        let height = parts
            .iter()
            .map(image::RgbaImage::height)
            .max()
            .unwrap_or(0);
        let mut picture =
            image::RgbaImage::from_pixel(width, height, image::Rgba([17, 19, 24, 255]));
        let mut x = 0;
        for part in &parts {
            image::imageops::replace(&mut picture, part, i64::from(x), 0);
            x += part.width() + GAP;
        }
        picture
    }

    impl eframe::App for Probe {
        fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
            // Only our own input: the real mouse must not hover or click.
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            input.events.append(&mut self.pending);
            input.focused = true;
            input.modifiers = egui::Modifiers::NONE;
            input.hovered_files.clear();
            input.dropped_files.clear();
            for viewport in input.viewports.values_mut() {
                viewport.focused = Some(true);
            }
        }

        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot {
                        image, user_data, ..
                    } => user_data
                        .data
                        .as_ref()
                        .and_then(|data| data.downcast_ref::<Tag>())
                        .map(|tag| (*tag, image.clone())),
                    _ => None,
                })
            });
            if self.shot == self.shots.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            if let Some(app) = &mut self.app {
                app.motion_time = FROZEN_TIME;
                app.update(ctx, frame);
            }
            self.callouts(ctx);
            self.step(ctx, screenshot);
            ctx.request_repaint();
        }
    }

    /// An empty variable counts as unset.
    fn env_path(name: &str, default: PathBuf) -> PathBuf {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or(default)
    }

    /// Refreshes `docs/guide/images/` from the shot list. Run it with
    /// `scripts/docs-capture.ps1`; see `docs/dev/DOCS-SCREENSHOTS.md`.
    #[test]
    #[ignore = "opens an owned native window with simulated panes and rewrites docs/guide/images"]
    fn native_docs_capture() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let locale = std::env::var("BUTTONSCLI_DOCS_CAPTURE_LOCALE")
            .ok()
            .filter(|locale| !locale.is_empty())
            .unwrap_or_else(|| "en".into());
        let images = manifest.join("docs/guide/images");
        let default_out = if locale == "en" {
            images
        } else {
            images.join(&locale)
        };
        let out = env_path("BUTTONSCLI_DOCS_CAPTURE_OUT", default_out);
        let work = env_path(
            "BUTTONSCLI_DOCS_CAPTURE_WORK",
            manifest.join("target/docs-capture"),
        );
        let only: Vec<String> = std::env::var("BUTTONSCLI_DOCS_CAPTURE_ONLY")
            .unwrap_or_default()
            .split(',')
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect();
        let force = std::env::var_os("BUTTONSCLI_DOCS_CAPTURE_FORCE")
            .is_some_and(|value| !value.is_empty());
        let shots: Vec<&'static Shot> = SHOTS
            .iter()
            .filter(|shot| {
                let stem = shot.file.split('.').next().unwrap();
                only.is_empty() || only.iter().any(|name| name == shot.file || name == stem)
            })
            .collect();
        assert!(
            !shots.is_empty(),
            "BUTTONSCLI_DOCS_CAPTURE_ONLY matched no shot: {only:?}"
        );
        std::fs::create_dir_all(&out).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        let root = std::env::temp_dir().join(format!(
            "buttonscli-docs-capture-{:032x}",
            fastrand::u128(..)
        ));
        std::fs::create_dir_all(&root).unwrap();
        // No feature-flags file: the UI a normal install shows.
        crate::features::local::initialize(&root);
        CAPTURING.store(true, Ordering::Relaxed);
        let results = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let shared = results.clone();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("ButtonsCLI docs capture")
                .with_inner_size([1280.0, 760.0])
                .with_resizable(true),
            persist_window: false,
            event_loop_builder: Some(Box::new(|builder| {
                winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(
                    builder, true,
                );
            })),
            ..Default::default()
        };
        let probe_root = root.clone();
        let probe_work = work.clone();
        eframe::run_native(
            "ButtonsCLI docs capture",
            options,
            Box::new(move |cc| {
                if let Some(render) = &cc.wgpu_render_state {
                    crate::plugins::effects::shader_background::register(
                        &cc.egui_ctx,
                        render.target_format,
                    );
                }
                // Settings draws inside the main window, so one screenshot has it.
                cc.egui_ctx.set_embed_viewports(true);
                struct Report(Probe, std::sync::Arc<std::sync::Mutex<Vec<ShotResult>>>);
                impl Drop for Report {
                    fn drop(&mut self) {
                        *self.1.lock().unwrap() = std::mem::take(&mut self.0.results);
                    }
                }
                impl eframe::App for Report {
                    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
                        self.0.raw_input_hook(ctx, input);
                    }
                    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
                        self.0.update(ctx, frame);
                    }
                }
                Ok(Box::new(Report(
                    Probe {
                        shots,
                        shot: 0,
                        part: 0,
                        app: None,
                        phase: Phase::Setup,
                        phase_at: Instant::now(),
                        clicks_done: 0,
                        then_done: false,
                        stable: 0,
                        last_rects: Vec::new(),
                        pending: Vec::new(),
                        parts: Vec::new(),
                        failure: None,
                        results: Vec::new(),
                        root: probe_root,
                        out,
                        work: probe_work,
                        locale,
                        force,
                        crop_scale: 1.0,
                        target_format: cc
                            .wgpu_render_state
                            .as_ref()
                            .map(|render| render.target_format),
                    },
                    shared,
                )))
            }),
        )
        .unwrap();
        CAPTURING.store(false, Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&root);
        let results = std::mem::take(&mut *results.lock().unwrap());
        std::fs::write(
            work.join("report.json"),
            serde_json::to_string_pretty(&results).unwrap(),
        )
        .unwrap();
        let failed: Vec<_> = results
            .iter()
            .filter(|result| result.status == "failed")
            .map(|result| format!("{}: {}", result.file, result.error.as_deref().unwrap_or("")))
            .collect();
        assert!(
            failed.is_empty(),
            "docs shots failed:\n{}",
            failed.join("\n")
        );
    }
}
