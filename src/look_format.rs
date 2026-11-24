//! File format versions, upgrades and checks for theme documents and theme
//! bundles: the look files that can come from outside the app (imports,
//! plugins, other ButtonsCLI versions).
//!
//! - A theme document's version is its top-level `version`; a bundle's is
//!   `format_version`. A missing version means 1, the format these files had
//!   before versions were checked.
//! - A file newer than this app is refused ("made by a newer ButtonsCLI") and
//!   never rewritten: theme files are not overwritten and newer bundles stay in
//!   native.json exactly as they were.
//! - To change a format: raise `THEME_FORMAT_VERSION` or
//!   `BUNDLE_FORMAT_VERSION` and add a [`Step`] to `THEME_STEPS` or
//!   `BUNDLE_STEPS` whose `from` is the previous version. Steps run in order
//!   on load, so every older file reaches the current format.
//!
//! Shared with the web build: no file access here.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};

use crate::settings::{ThemeBundle, MAX_PANES, MAX_PANE_LOOKS};
use crate::theme::{parse_color, ShaderBackground, MAX_CUSTOM_SHADER_BYTES, MAX_SHADER_COLORS};

/// The theme document format this app writes and reads (top-level `version`).
pub(crate) const THEME_FORMAT_VERSION: u64 = 1;
/// The bundle format this app writes and reads (`format_version`).
pub(crate) const BUNDLE_FORMAT_VERSION: u64 = 1;

/// Upgrades theme documents; entry `n` has `from: n + 1`.
const THEME_STEPS: &[Step] = &[];
/// Upgrades bundles; entry `n` has `from: n + 1`.
const BUNDLE_STEPS: &[Step] = &[];

pub(crate) const THEME_CHAIN: Chain = Chain {
    what: "theme",
    field: "version",
    current: THEME_FORMAT_VERSION,
    steps: THEME_STEPS,
};

pub(crate) const BUNDLE_CHAIN: Chain = Chain {
    what: "bundle",
    field: "format_version",
    current: BUNDLE_FORMAT_VERSION,
    steps: BUNDLE_STEPS,
};

/// Every field a bundle of the current format has. Others are kept, unused.
pub(crate) const BUNDLE_FIELDS: &[&str] = &[
    "format_version",
    "name",
    "theme_id",
    "app_theme_id",
    "terminal_theme_id",
    "gradient_theme_id",
    "effects_theme_id",
    "panes",
    "layout",
    "visible_panes",
    "row_banding_brightness",
    "chrome",
    "typography",
];

const BUNDLE_THEME_FIELDS: [&str; 5] = [
    "theme_id",
    "app_theme_id",
    "terminal_theme_id",
    "gradient_theme_id",
    "effects_theme_id",
];

const LAYOUTS: [&str; 5] = ["", "single", "columns", "rows", "grid"];

/// A bundle's `format_version`. In memory it is always the current format:
/// bundles are upgraded with [`upgrade_bundle`] before they are deserialized.
/// Version 1 is not written, so files stay as older versions wrote them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(crate) struct BundleFormat(pub(crate) u64);

impl Default for BundleFormat {
    fn default() -> Self {
        Self(BUNDLE_FORMAT_VERSION)
    }
}

impl BundleFormat {
    pub(crate) fn is_first(&self) -> bool {
        self.0 == 1
    }
}

/// One upgrade step: changes a document of version `from` into `from + 1`
/// and returns what it changed, in words. The chain sets the version field.
pub(crate) struct Step {
    pub(crate) from: u64,
    pub(crate) run: fn(&mut Map<String, Value>) -> Vec<String>,
}

/// The versions of one kind of file and the steps between them.
pub(crate) struct Chain {
    pub(crate) what: &'static str,
    pub(crate) field: &'static str,
    pub(crate) current: u64,
    pub(crate) steps: &'static [Step],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatError {
    NotObject {
        what: &'static str,
    },
    BadVersion {
        what: &'static str,
        found: String,
    },
    Newer {
        what: &'static str,
        found: u64,
        supported: u64,
    },
    /// A gap in the step table: a bug in this app, not in the file.
    MissingStep {
        what: &'static str,
        from: u64,
    },
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotObject { what } => {
                write!(f, "a {what} must be a JSON object ({{ ... }})")
            }
            Self::BadVersion { what, found } => write!(
                f,
                "{found} is not a {what} format version (a whole number, 1 or more)"
            ),
            Self::Newer {
                what,
                found,
                supported,
            } => write!(
                f,
                "this {what} was made by a newer ButtonsCLI ({what} format {found}; this \
                 version reads up to {supported}). Update ButtonsCLI to use it"
            ),
            Self::MissingStep { what, from } => {
                write!(f, "this app cannot upgrade {what} format {from}")
            }
        }
    }
}

impl std::error::Error for FormatError {}

impl Chain {
    /// The file's version; a missing (or null) version is 1.
    pub(crate) fn version_of(&self, document: &Value) -> Result<u64, FormatError> {
        match document.get(self.field) {
            None | Some(Value::Null) => Ok(1),
            Some(value) => value
                .as_u64()
                .filter(|version| *version >= 1)
                .ok_or_else(|| FormatError::BadVersion {
                    what: self.what,
                    found: value.to_string(),
                }),
        }
    }

    /// The version of a document this app can read (it may need upgrading).
    pub(crate) fn check(&self, document: &Value) -> Result<u64, FormatError> {
        if !document.is_object() {
            return Err(FormatError::NotObject { what: self.what });
        }
        let version = self.version_of(document)?;
        if version > self.current {
            return Err(FormatError::Newer {
                what: self.what,
                found: version,
                supported: self.current,
            });
        }
        Ok(version)
    }

    /// Runs the steps from the document's version to the current one.
    /// Unknown fields are left alone. Returns what changed (empty when the
    /// document was already current).
    pub(crate) fn upgrade(&self, mut document: Value) -> Result<(Value, Vec<String>), FormatError> {
        let mut version = self.check(&document)?;
        let mut changes = Vec::new();
        while version < self.current {
            let step = self.steps.iter().find(|step| step.from == version).ok_or(
                FormatError::MissingStep {
                    what: self.what,
                    from: version,
                },
            )?;
            let Some(object) = document.as_object_mut() else {
                return Err(FormatError::NotObject { what: self.what });
            };
            changes.extend((step.run)(object));
            version += 1;
            object.insert(self.field.into(), Value::from(version));
            changes.push(format!(
                "upgraded {} format {} to {version}",
                self.what,
                version - 1
            ));
        }
        Ok((document, changes))
    }
}

/// Brings a theme document to the current format (see the module docs).
pub(crate) fn upgrade_theme_document(document: Value) -> Result<(Value, Vec<String>), FormatError> {
    THEME_CHAIN.upgrade(document)
}

/// Brings a bundle to the current format (see the module docs).
pub(crate) fn upgrade_bundle(bundle: Value) -> Result<(Value, Vec<String>), FormatError> {
    BUNDLE_CHAIN.upgrade(bundle)
}

/// One problem in a file, at a JSON pointer (`""` is the whole file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) path: String,
    pub(crate) reason: String,
}

impl Finding {
    /// The path as people write it: `theme.terminal.background`, `panes[3]`.
    pub(crate) fn place(&self) -> String {
        if self.path.is_empty() {
            return "(whole file)".into();
        }
        let mut place = String::new();
        for part in self.path.trim_start_matches('/').split('/') {
            let part = part.replace("~1", "/").replace("~0", "~");
            if !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()) {
                place.push_str(&format!("[{part}]"));
            } else {
                if !place.is_empty() {
                    place.push('.');
                }
                place.push_str(&part);
            }
        }
        place
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.place(), self.reason)
    }
}

/// What a check found. Errors: the file cannot be used. Warnings: it can be
/// used, but something in it is ignored or changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatReport {
    /// The file's format version, when it could be read.
    pub(crate) version: Option<u64>,
    pub(crate) errors: Vec<Finding>,
    pub(crate) warnings: Vec<Finding>,
}

impl FormatReport {
    /// Up to `limit` findings (errors first) on one line.
    pub(crate) fn summary(&self, limit: usize) -> String {
        let all: Vec<String> = self
            .errors
            .iter()
            .chain(&self.warnings)
            .map(ToString::to_string)
            .collect();
        let mut text = all
            .iter()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        if all.len() > limit {
            text.push_str(&format!("; and {} more", all.len() - limit));
        }
        text
    }

    fn error(&mut self, path: impl Into<String>, reason: impl Into<String>) {
        self.errors.push(Finding {
            path: path.into(),
            reason: reason.into(),
        });
    }

    fn warn(&mut self, path: impl Into<String>, reason: impl Into<String>) {
        self.warnings.push(Finding {
            path: path.into(),
            reason: reason.into(),
        });
    }

    /// Records a version problem; returns false when nothing else can be checked.
    fn check_version(&mut self, chain: &Chain, document: &Value) -> bool {
        match chain.check(document) {
            Ok(version) => {
                self.version = Some(version);
                true
            }
            Err(error @ FormatError::NotObject { .. }) => {
                self.error("", error.to_string());
                false
            }
            Err(error) => {
                self.error(format!("/{}", chain.field), error.to_string());
                false
            }
        }
    }
}

// ---------------------------------------------------------------- themes

/// App and terminal colors: (object, keys). A bad one falls back to a default.
const THEME_COLORS: &[(&str, &[&str])] = &[
    (
        "/theme/app/shell",
        &[
            "background",
            "backgroundSecondary",
            "panel",
            "buttonBackground",
            "border",
            "borderColor",
            "textMain",
            "foreground",
            "textDim",
            "accent",
            "accentHover",
        ],
    ),
    ("/theme/app/shell/paneDivider", &["color"]),
    (
        "/theme/app/tabs",
        &[
            "background",
            "backgroundColor",
            "idleBackground",
            "inactiveBackground",
            "activeBackground",
            "activeBackgroundColor",
            "activeBorder",
            "activeBorderColor",
        ],
    ),
    (
        "/theme/app/presetDock",
        &[
            "background",
            "buttonBackground",
            "buttonHover",
            "buttonText",
            "accent",
            "widthAccent",
        ],
    ),
    ("/theme/app/settings", &["background"]),
    (
        "/theme/app/statusBar",
        &[
            "background",
            "backgroundColor",
            "text",
            "foreground",
            "border",
            "borderColor",
            "warning",
            "highlight",
        ],
    ),
    (
        "/theme/terminal",
        &[
            "background",
            "foreground",
            "gradientStart",
            "gradientEnd",
            "gradientColorC",
            "gradientColorD",
        ],
    ),
    ("/effects", &["rowBandingColor"]),
];

const ANSI_NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightMagenta",
    "brightCyan",
    "brightWhite",
];

#[derive(Clone, Copy)]
enum Scale {
    /// Used as written.
    Plain,
    /// A fraction; values above 1 are read as percentages.
    Unit,
    /// A percentage (0–100).
    Percent,
}

/// Effect numbers the app clamps: (key, scale, lowest, highest) in the
/// units the file uses (fractions for `Unit`).
const EFFECT_NUMBERS: &[(&str, Scale, f64, f64)] = &[
    ("staticOpacity", Scale::Unit, 0.0, 0.35),
    ("staticDensity", Scale::Unit, 0.02, 1.0),
    ("staticIntensity", Scale::Percent, 0.0, 100.0),
    ("staticAmplitude", Scale::Percent, 0.0, 100.0),
    ("staticBrightness", Scale::Percent, 0.0, 100.0),
    ("scanlinesStrength", Scale::Unit, 0.0, 0.35),
    ("scanlinesPeriod", Scale::Plain, 2.0, 16.0),
    ("rowBandingOpacity", Scale::Unit, 0.0, 0.35),
    ("simpleNoiseAmount", Scale::Percent, 0.0, 100.0),
    ("simpleNoiseResolution", Scale::Percent, 8.0, 100.0),
    ("simpleNoiseFps", Scale::Plain, 0.0, 60.0),
    ("simpleNoiseMinBrightness", Scale::Percent, 0.0, 100.0),
    ("simpleNoiseMaxBrightness", Scale::Percent, 0.0, 100.0),
    ("simpleNoiseIdleAmount", Scale::Percent, 0.0, 100.0),
    ("simpleNoiseIdleDelaySeconds", Scale::Plain, 0.0, 3600.0),
    ("simpleNoiseIdleRampSeconds", Scale::Plain, 1.0, 300.0),
];

/// Only the problems that make a theme document unusable.
pub(crate) fn theme_document_errors(document: &Value) -> Vec<Finding> {
    let mut report = FormatReport::default();
    check_theme_shape(&mut report, document);
    report.errors
}

/// Checks a theme document (a personal theme file or an import) against the
/// current format, the way the app reads it.
pub(crate) fn validate_theme_document(document: &Value) -> FormatReport {
    let mut report = FormatReport::default();
    if !check_theme_shape(&mut report, document) {
        return report;
    }
    for (object, keys) in THEME_COLORS {
        let parent = pointer(document, object);
        for key in *keys {
            if let Some(value) = parent.and_then(|parent| parent.get(*key)) {
                check_color(&mut report, &format!("{object}/{key}"), value);
            }
        }
    }
    check_ansi(&mut report, document);
    check_gradient_colors(&mut report, document);
    if let Some(thickness) = pointer(document, "/theme/app/shell/paneDivider/thickness") {
        let path = "/theme/app/shell/paneDivider/thickness";
        match thickness.as_f64() {
            Some(value) => check_range(&mut report, path, value, 1.0, 6.0, ""),
            None if !thickness.is_null() => report.warn(
                path,
                format!("{thickness} is not a number; the default 2 is used"),
            ),
            None => {}
        }
    }
    if let Some(effects) = document.get("effects") {
        for (key, scale, low, high) in EFFECT_NUMBERS {
            if let Some(value) = effects.get(*key) {
                check_effect_number(&mut report, key, value, *scale, *low, *high);
            }
        }
        if let Some(shader) = effects.get("shaderBackground") {
            check_shader(&mut report, shader);
        }
    }
    report
}

/// Version, metadata and the two required objects. Returns false when the
/// rest cannot be checked.
fn check_theme_shape(report: &mut FormatReport, document: &Value) -> bool {
    if !report.check_version(&THEME_CHAIN, document) {
        return false;
    }
    if document.get("version").is_none_or(Value::is_null) {
        report.warn("/version", "is missing; the file is read as theme format 1");
    }
    for (key, what) in [("id", "an id"), ("name", "a name")] {
        let path = format!("/metadata/{key}");
        match pointer(document, &path) {
            Some(Value::String(text)) if !text.trim().is_empty() => {}
            Some(Value::String(_)) => report.error(path, format!("is empty; a theme needs {what}")),
            None | Some(Value::Null) => {
                report.error(path, format!("is missing; a theme needs {what}"))
            }
            Some(other) => report.error(path, format!("{other} should be a string")),
        }
    }
    if !document.get("theme").is_some_and(Value::is_object) {
        report.error(
            "/theme",
            "is missing or not an object; it holds the app and terminal colors",
        );
    }
    if !document.get("effects").is_some_and(Value::is_object) {
        report.error(
            "/effects",
            "is missing or not an object; write {} for a theme without effects",
        );
    }
    true
}

fn pointer<'a>(document: &'a Value, path: &str) -> Option<&'a Value> {
    document.pointer(path)
}

fn check_color(report: &mut FormatReport, path: &str, value: &Value) {
    match value {
        Value::Null => {}
        Value::String(text) if text.is_empty() => {}
        Value::String(text) => {
            if parse_color(text).is_none() {
                report.warn(
                    path,
                    format!("{text:?} is not a #rrggbb color; the default color is used"),
                );
            }
        }
        other => report.warn(
            path,
            format!("{other} is not a color; write colors as \"#rrggbb\""),
        ),
    }
}

fn check_ansi(report: &mut FormatReport, document: &Value) {
    let path = "/theme/terminal/ansiColors";
    match pointer(document, path) {
        None | Some(Value::Null) => {}
        Some(Value::Object(colors)) => {
            for (index, name) in ANSI_NAMES.iter().enumerate() {
                for key in [name.to_string(), index.to_string()] {
                    if let Some(value) = colors.get(&key) {
                        check_color(report, &format!("{path}/{key}"), value);
                    }
                }
            }
        }
        Some(Value::Array(_)) => report.warn(
            path,
            "is a list; this app reads an object with names (\"black\", \"red\", ... or \
             \"0\"-\"15\"), so the default ANSI colors are used",
        ),
        Some(other) => report.warn(
            path,
            format!("{other} is not an object of colors; the default ANSI colors are used"),
        ),
    }
}

fn check_gradient_colors(report: &mut FormatReport, document: &Value) {
    let path = "/theme/terminal/gradientColors";
    match pointer(document, path) {
        None | Some(Value::Null) => {}
        Some(Value::Array(colors)) => {
            for (index, value) in colors.iter().enumerate() {
                check_color(report, &format!("{path}/{index}"), value);
            }
            let usable = colors
                .iter()
                .filter_map(Value::as_str)
                .filter(|text| parse_color(text).is_some())
                .count();
            if usable > 4 {
                report.warn(
                    path,
                    format!("has {usable} colors; only the first 4 are used"),
                );
            }
        }
        Some(other) => report.warn(
            path,
            format!("{other} is not a list of colors; the gradient uses other colors"),
        ),
    }
}

/// Like the theme loader: a number, or a numeric string such as "12px".
fn loose_number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        value.as_str().and_then(|text| {
            text.trim()
                .trim_end_matches("px")
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
        })
    })
}

fn check_effect_number(
    report: &mut FormatReport,
    key: &str,
    value: &Value,
    scale: Scale,
    low: f64,
    high: f64,
) {
    let path = format!("/effects/{key}");
    if value.is_null() {
        return;
    }
    let Some(raw) = loose_number(value) else {
        report.warn(
            path,
            format!("{value} is not a number; the default is used"),
        );
        return;
    };
    match scale {
        Scale::Plain => check_range(report, &path, raw, low, high, ""),
        Scale::Percent => check_range(report, &path, raw, low, high, "%"),
        // Above 1 the file means a percentage: compare in its own units.
        Scale::Unit if raw > 1.0 => check_range(report, &path, raw, low * 100.0, high * 100.0, "%"),
        Scale::Unit => check_range(report, &path, raw, low, high, ""),
    }
}

/// Warns when `value` is outside `low..=high` (the app clamps it).
fn check_range(report: &mut FormatReport, path: &str, value: f64, low: f64, high: f64, unit: &str) {
    let (shown, low_shown, high_shown) = (number(value), number(low), number(high));
    if value < low {
        report.warn(
            path,
            format!("{shown}{unit} is below the lowest value, {low_shown}{unit}; {low_shown}{unit} is used"),
        );
    } else if value > high {
        report.warn(
            path,
            format!("{shown}{unit} is above the highest value, {high_shown}{unit}; {high_shown}{unit} is used"),
        );
    }
}

/// A number as people write it: 0.35, not 0.3499999940395355.
fn number(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    format!("{rounded}")
}

fn check_shader(report: &mut FormatReport, shader: &Value) {
    let path = "/effects/shaderBackground";
    let ignored = "the shader is ignored";
    let object = match shader {
        Value::Null => return,
        Value::Object(object) => object,
        other => {
            report.warn(path, format!("{other} is not a shader object; {ignored}"));
            return;
        }
    };
    if let Some(version) = object.get("version") {
        match version.as_u64() {
            Some(1) => {}
            Some(newer) if newer > 1 => report.warn(
                format!("{path}/version"),
                format!(
                    "shader format {newer} was made by a newer ButtonsCLI (this version \
                     reads format 1); {ignored}"
                ),
            ),
            _ => report.warn(
                format!("{path}/version"),
                format!("{version} is not shader format 1; {ignored}"),
            ),
        }
    }
    if let Some(preset) = object.get("preset").and_then(Value::as_str) {
        if !ShaderBackground::PRESETS
            .iter()
            .any(|(id, _)| *id == preset)
        {
            report.warn(
                format!("{path}/preset"),
                format!(
                    "{preset:?} is not a known preset (for example \"aurora\", \"waves\", \
                     \"nebula\"); {ignored}"
                ),
            );
        }
    }
    let parsed: ShaderBackground = match serde_json::from_value(shader.clone()) {
        Ok(parsed) => parsed,
        Err(error) => {
            report.warn(path, format!("cannot be read ({error}); {ignored}"));
            return;
        }
    };
    for (key, value, range) in [
        ("strength", parsed.strength, ShaderBackground::STRENGTH),
        ("speed", parsed.speed, ShaderBackground::SPEED),
        ("scale", parsed.scale, ShaderBackground::SCALE),
    ] {
        check_range(
            report,
            &format!("{path}/{key}"),
            f64::from(value),
            f64::from(*range.start()),
            f64::from(*range.end()),
            "",
        );
    }
    if let Some(colors) = &parsed.colors {
        let colors_path = format!("{path}/colors");
        if colors.is_empty() {
            report.warn(
                &colors_path,
                format!("is empty; leave it out to use the theme's colors; {ignored}"),
            );
        } else if colors.len() > MAX_SHADER_COLORS {
            report.warn(
                &colors_path,
                format!(
                    "has {} colors; at most {MAX_SHADER_COLORS} (c1-c5) are allowed; {ignored}",
                    colors.len()
                ),
            );
        }
        for (index, color) in colors.iter().enumerate() {
            if parse_color(color).is_none() {
                report.warn(
                    format!("{colors_path}/{index}"),
                    format!("{color:?} is not a #rrggbb color; {ignored}"),
                );
            }
        }
    }
    let Some(custom) = &parsed.custom else {
        return;
    };
    let code_path = format!("{path}/custom/expression");
    if custom.expression.trim().is_empty() {
        report.warn(&code_path, "is empty; the custom shader has no code to run");
    } else if custom.expression.len() > MAX_CUSTOM_SHADER_BYTES {
        report.warn(
            &code_path,
            format!(
                "is {} bytes; shader code may be at most {MAX_CUSTOM_SHADER_BYTES} bytes; \
                 {ignored}",
                custom.expression.len()
            ),
        );
    }
    if custom.variables.len() > 4 {
        report.warn(
            format!("{path}/custom/variables"),
            format!(
                "has {} sliders; at most 4 are allowed; {ignored}",
                custom.variables.len()
            ),
        );
    }
    for (index, variable) in custom.variables.iter().enumerate() {
        let slider = format!("{path}/custom/variables/{index}");
        let (min, max, value) = (
            f64::from(variable.min),
            f64::from(variable.max),
            f64::from(variable.value),
        );
        if min >= max {
            report.warn(
                &slider,
                format!("min {min} is not below max {max}; {ignored}"),
            );
        } else if min < -100.0 || max > 100.0 {
            report.warn(
                &slider,
                format!("the range {min} to {max} must stay within -100 to 100; {ignored}"),
            );
        } else {
            check_range(report, &format!("{slider}/value"), value, min, max, "");
        }
    }
}

// --------------------------------------------------------------- bundles

/// Checks a bundle (one entry of native.json's `theme_bundles`, or a built-in
/// one). `known_theme` says whether a theme id is installed.
pub(crate) fn validate_bundle(bundle: &Value, known_theme: impl Fn(&str) -> bool) -> FormatReport {
    let mut report = FormatReport::default();
    if !report.check_version(&BUNDLE_CHAIN, bundle) {
        return report;
    }
    let Some(object) = bundle.as_object() else {
        return report;
    };
    match object.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {}
        Some(Value::String(_)) => {
            report.error("/name", "is empty; bundles are listed and applied by name")
        }
        None | Some(Value::Null) => report.error(
            "/name",
            "is missing; bundles are listed and applied by name",
        ),
        Some(other) => report.error("/name", format!("{other} should be a string")),
    }
    for key in object.keys() {
        if !BUNDLE_FIELDS.contains(&key.as_str()) {
            report.warn(
                format!("/{}", key.replace('~', "~0").replace('/', "~1")),
                "is not a bundle field this version knows; it is kept but not used",
            );
        }
    }
    for key in BUNDLE_THEME_FIELDS {
        let path = format!("/{key}");
        match object.get(key) {
            None => {}
            Some(Value::String(id)) if id.is_empty() => {}
            Some(Value::String(id)) => {
                if !known_theme(id) {
                    report.warn(
                        path,
                        format!(
                            "theme {id:?} is not installed; this part of the bundle is skipped"
                        ),
                    );
                }
            }
            Some(other) => report.error(
                path,
                format!("{other} should be a theme id string (\"\" keeps the current theme)"),
            ),
        }
    }
    check_bundle_panes(&mut report, object.get("panes"), &known_theme);
    match object.get("layout") {
        None => {}
        Some(Value::String(layout)) if LAYOUTS.contains(&layout.as_str()) => {}
        Some(Value::String(layout)) => report.warn(
            "/layout",
            format!(
                "{layout:?} is not a layout (single, columns, rows or grid; \"\" keeps the \
                 current one); it is read as single"
            ),
        ),
        Some(other) => report.error("/layout", format!("{other} should be a layout name")),
    }
    match object.get("visible_panes") {
        None => {}
        Some(value) => match value.as_u64() {
            Some(count) if count > MAX_PANES as u64 => report.warn(
                "/visible_panes",
                format!("{count} is above the highest value, {MAX_PANES}; {MAX_PANES} is used"),
            ),
            Some(_) => {}
            None => report.error(
                "/visible_panes",
                format!("{value} should be a whole number of panes"),
            ),
        },
    }
    match object.get("row_banding_brightness") {
        None | Some(Value::Null) => {}
        Some(value) => match value.as_u64() {
            Some(level) if level <= 64 => {}
            Some(level) if level <= 255 => report.warn(
                "/row_banding_brightness",
                format!("{level} is above the highest value, 64; 64 is used"),
            ),
            _ => report.error(
                "/row_banding_brightness",
                format!("{value} should be a whole number from 0 to 64"),
            ),
        },
    }
    match object.get("chrome") {
        None | Some(Value::Null) => {}
        Some(Value::Object(chrome)) => {
            for key in ["dock_background", "status_background"] {
                match chrome.get(key) {
                    None | Some(Value::Null) => {}
                    Some(Value::String(color)) if parse_color(color).is_some() => {}
                    Some(value) => report.warn(
                        format!("/chrome/{key}"),
                        format!("{value} is not a #rrggbb color; the theme's color is used"),
                    ),
                }
            }
        }
        Some(other) => report.error(
            "/chrome",
            format!("{other} should be an object with the bar colors"),
        ),
    }
    if report.errors.is_empty() {
        if let Err(error) = serde_json::from_value::<ThemeBundle>(bundle.clone()) {
            report.error("", format!("cannot be read: {error}"));
        }
    }
    report
}

fn check_bundle_panes(
    report: &mut FormatReport,
    panes: Option<&Value>,
    known_theme: &impl Fn(&str) -> bool,
) {
    let panes = match panes {
        None => return,
        Some(Value::Array(panes)) => panes,
        Some(other) => {
            report.error("/panes", format!("{other} should be a list of pane looks"));
            return;
        }
    };
    for (index, pane) in panes.iter().enumerate() {
        let path = format!("/panes/{index}");
        let Some(pane) = pane.as_object() else {
            report.error(
                path,
                format!("{pane} should be an object with theme_id and font"),
            );
            continue;
        };
        match pane.get("theme_id") {
            None | Some(Value::Null) => {}
            Some(Value::String(id)) => {
                if !known_theme(id) {
                    report.warn(
                        format!("{path}/theme_id"),
                        format!("theme {id:?} is not installed; this pane look is skipped"),
                    );
                }
            }
            Some(other) => report.error(
                format!("{path}/theme_id"),
                format!("{other} should be a theme id string or null"),
            ),
        }
    }
    if panes.len() > MAX_PANE_LOOKS {
        report.warn(
            format!("/panes/{MAX_PANE_LOOKS}"),
            format!(
                "the bundle has {} pane looks; only the first {MAX_PANE_LOOKS} are applied, \
                 the rest are kept",
                panes.len()
            ),
        );
    }
}

/// A bundle from native.json this app cannot use: made by a newer version or
/// unreadable. It is kept, unchanged, at its place in the list.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HeldBundle {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) document: Value,
    pub(crate) reason: String,
}

/// native.json's bundles, sorted out for loading.
#[derive(Debug, Default)]
pub(crate) struct BundleSplit {
    /// Bundles this app reads, upgraded to the current format.
    pub(crate) kept: Vec<Value>,
    pub(crate) held: Vec<HeldBundle>,
    /// Fields of kept bundles this version does not know, by bundle name.
    pub(crate) extras: BTreeMap<String, Map<String, Value>>,
    /// What upgrading changed, one line per bundle.
    pub(crate) upgraded: Vec<String>,
}

/// Upgrades the bundles this app can read and holds back the rest.
pub(crate) fn split_bundles(bundles: Vec<Value>) -> BundleSplit {
    let mut split = BundleSplit::default();
    for (index, bundle) in bundles.into_iter().enumerate() {
        let name = bundle
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let hold = |reason: String| HeldBundle {
            index,
            name: name.clone(),
            document: bundle.clone(),
            reason,
        };
        let upgraded = match upgrade_bundle(bundle.clone()) {
            Ok(upgraded) => upgraded,
            Err(error) => {
                split.held.push(hold(error.to_string()));
                continue;
            }
        };
        let (document, changes) = upgraded;
        if let Err(error) = serde_json::from_value::<ThemeBundle>(document.clone()) {
            split.held.push(hold(format!("it cannot be read: {error}")));
            continue;
        }
        if !changes.is_empty() {
            split
                .upgraded
                .push(format!("bundle {name:?}: {}", changes.join("; ")));
        }
        let extra: Map<String, Value> = document
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(key, _)| !BUNDLE_FIELDS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if !extra.is_empty() {
            split.extras.insert(name.clone(), extra);
        }
        split.kept.push(document);
    }
    split
}

/// The reverse of [`split_bundles`] when saving: gives bundles back the
/// fields this version did not know (matched by name, never replacing a
/// field) and puts held bundles back where they were.
pub(crate) fn restore_bundles(
    bundles: &mut Vec<Value>,
    held: &[HeldBundle],
    extras: &BTreeMap<String, Map<String, Value>>,
) {
    for bundle in bundles.iter_mut() {
        let Some(object) = bundle.as_object_mut() else {
            continue;
        };
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if let Some(extra) = extras.get(&name) {
            for (key, value) in extra {
                object.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
    }
    let mut held: Vec<&HeldBundle> = held.iter().collect();
    held.sort_by_key(|bundle| bundle.index);
    for bundle in held {
        let index = bundle.index.min(bundles.len());
        bundles.insert(index, bundle.document.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn theme() -> Value {
        json!({
            "version": 1,
            "metadata": {"id": "night", "name": "Night"},
            "theme": {
                "app": {"shell": {"background": "#101010", "paneDivider": {"thickness": 2}}},
                "terminal": {"background": "#000000", "foreground": "#ffffff"}
            },
            "effects": {}
        })
    }

    fn reasons(findings: &[Finding]) -> Vec<String> {
        findings.iter().map(ToString::to_string).collect()
    }

    fn only_warning(document: &Value) -> String {
        let report = validate_theme_document(document);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        report.warnings[0].to_string()
    }

    fn only_error(document: &Value) -> String {
        let report = validate_theme_document(document);
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        report.errors[0].to_string()
    }

    #[test]
    fn a_clean_theme_has_no_findings_and_is_current() {
        let report = validate_theme_document(&theme());
        assert_eq!(
            report,
            FormatReport {
                version: Some(1),
                ..Default::default()
            }
        );
        let (upgraded, changes) = upgrade_theme_document(theme()).unwrap();
        assert_eq!(upgraded, theme());
        assert!(changes.is_empty());
    }

    #[test]
    fn theme_errors_name_the_field_and_the_reason() {
        assert_eq!(
            only_error(&json!([1, 2])),
            "(whole file): a theme must be a JSON object ({ ... })"
        );
        let mut document = theme();
        document["version"] = json!(2);
        let newer = only_error(&document);
        assert!(newer.starts_with("version: this theme was made by a newer ButtonsCLI (theme format 2; this version reads up to 1)"), "{newer}");
        document["version"] = json!("one");
        assert_eq!(
            only_error(&document),
            "version: \"one\" is not a theme format version (a whole number, 1 or more)"
        );
        document["version"] = json!(0);
        assert!(only_error(&document).contains("0 is not a theme format version"));

        let mut document = theme();
        document["metadata"]["id"] = json!("  ");
        assert_eq!(
            only_error(&document),
            "metadata.id: is empty; a theme needs an id"
        );
        document["metadata"]["id"] = json!(7);
        assert_eq!(only_error(&document), "metadata.id: 7 should be a string");
        let mut document = theme();
        document["metadata"].as_object_mut().unwrap().remove("name");
        assert_eq!(
            only_error(&document),
            "metadata.name: is missing; a theme needs a name"
        );
        let mut document = theme();
        document["theme"] = json!("dark");
        assert!(only_error(&document).starts_with("theme: is missing or not an object"));
        let mut document = theme();
        document.as_object_mut().unwrap().remove("effects");
        assert!(only_error(&document).starts_with("effects: is missing or not an object"));
    }

    #[test]
    fn a_missing_version_reads_as_format_one() {
        let mut document = theme();
        document.as_object_mut().unwrap().remove("version");
        assert_eq!(
            only_warning(&document),
            "version: is missing; the file is read as theme format 1"
        );
        assert_eq!(THEME_CHAIN.check(&document), Ok(1));
        let (upgraded, changes) = upgrade_theme_document(document.clone()).unwrap();
        assert_eq!(upgraded, document);
        assert!(changes.is_empty());
    }

    #[test]
    fn theme_color_warnings_say_what_is_used_instead() {
        let mut document = theme();
        document["theme"]["terminal"]["background"] = json!("#12345");
        assert_eq!(
            only_warning(&document),
            "theme.terminal.background: \"#12345\" is not a #rrggbb color; the default color is used"
        );
        let mut document = theme();
        document["theme"]["app"]["tabs"] = json!({"activeBorder": 12});
        assert_eq!(
            only_warning(&document),
            "theme.app.tabs.activeBorder: 12 is not a color; write colors as \"#rrggbb\""
        );
        // Not ASCII: must not panic in the color parser.
        let mut document = theme();
        document["theme"]["terminal"]["foreground"] = json!("#éa");
        assert!(only_warning(&document).contains("is not a #rrggbb color"));
        let mut document = theme();
        document["theme"]["terminal"]["ansiColors"] = json!({"red": "red", "9": "#ff0000"});
        assert_eq!(
            only_warning(&document),
            "theme.terminal.ansiColors.red: \"red\" is not a #rrggbb color; the default color is used"
        );
        document["theme"]["terminal"]["ansiColors"] = json!(["#000000"]);
        assert!(only_warning(&document).starts_with("theme.terminal.ansiColors: is a list"));
        let mut document = theme();
        document["theme"]["terminal"]["gradientColors"] =
            json!(["#000", "#111111", "#222222", "#333333", "#444444"]);
        assert_eq!(
            only_warning(&document),
            "theme.terminal.gradientColors: has 5 colors; only the first 4 are used"
        );
        document["theme"]["terminal"]["gradientColors"] = json!(["#000", "blue"]);
        assert_eq!(
            only_warning(&document),
            "theme.terminal.gradientColors[1]: \"blue\" is not a #rrggbb color; the default color is used"
        );
        let mut document = theme();
        document["effects"]["rowBandingColor"] = json!("#zzzzzz");
        assert!(only_warning(&document).starts_with("effects.rowBandingColor:"));
    }

    #[test]
    fn theme_number_warnings_follow_the_app_clamps() {
        let mut document = theme();
        document["theme"]["app"]["shell"]["paneDivider"]["thickness"] = json!(9);
        assert_eq!(
            only_warning(&document),
            "theme.app.shell.paneDivider.thickness: 9 is above the highest value, 6; 6 is used"
        );
        document["theme"]["app"]["shell"]["paneDivider"]["thickness"] = json!("thick");
        assert_eq!(
            only_warning(&document),
            "theme.app.shell.paneDivider.thickness: \"thick\" is not a number; the default 2 is used"
        );
        let mut document = theme();
        document["effects"]["scanlinesPeriod"] = json!(1);
        assert_eq!(
            only_warning(&document),
            "effects.scanlinesPeriod: 1 is below the lowest value, 2; 2 is used"
        );
        // Fractions above 1 are percentages, as the loader reads them.
        document["effects"] = json!({"staticOpacity": 50});
        assert_eq!(
            only_warning(&document),
            "effects.staticOpacity: 50% is above the highest value, 35%; 35% is used"
        );
        document["effects"] = json!({"staticOpacity": 0.5});
        assert_eq!(
            only_warning(&document),
            "effects.staticOpacity: 0.5 is above the highest value, 0.35; 0.35 is used"
        );
        document["effects"] = json!({"staticOpacity": 0.2, "simpleNoiseFps": "30px"});
        assert!(validate_theme_document(&document).warnings.is_empty());
        document["effects"] = json!({"simpleNoiseResolution": 5});
        assert_eq!(
            only_warning(&document),
            "effects.simpleNoiseResolution: 5% is below the lowest value, 8%; 8% is used"
        );
        document["effects"] = json!({"simpleNoiseFps": true});
        assert_eq!(
            only_warning(&document),
            "effects.simpleNoiseFps: true is not a number; the default is used"
        );
    }

    fn with_shader(shader: Value) -> Value {
        let mut document = theme();
        document["effects"]["shaderBackground"] = shader;
        document
    }

    #[test]
    fn shader_warnings_cover_code_size_sliders_and_versions() {
        let good = json!({
            "version": 1, "preset": "aurora", "strength": 0.5, "speed": 0.2, "scale": 1.0,
            "colors": ["#000000", "#111111", "#222222"],
            "custom": {"expression": "return bg;", "variables": [
                {"name": "a", "min": 0.0, "max": 1.0, "value": 0.5}
            ]}
        });
        assert!(validate_theme_document(&with_shader(good.clone()))
            .warnings
            .is_empty());
        assert!(validate_theme_document(&with_shader(Value::Null))
            .warnings
            .is_empty());

        let mut shader = good.clone();
        shader["version"] = json!(2);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.version: shader format 2 was made by a newer ButtonsCLI \
             (this version reads format 1); the shader is ignored"
        );
        let mut shader = good.clone();
        shader["preset"] = json!("lava2");
        assert!(only_warning(&with_shader(shader))
            .starts_with("effects.shaderBackground.preset: \"lava2\" is not a known preset"));
        let mut shader = good.clone();
        shader["strength"] = json!(3.0);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.strength: 3 is above the highest value, 1; 1 is used"
        );
        let mut shader = good.clone();
        shader["speed"] = json!("fast");
        assert!(only_warning(&with_shader(shader))
            .starts_with("effects.shaderBackground: cannot be read ("));
        let mut shader = good.clone();
        shader["colors"] = json!([]);
        assert!(only_warning(&with_shader(shader))
            .starts_with("effects.shaderBackground.colors: is empty"));
        let mut shader = good.clone();
        shader["colors"] = json!(["#000", "#000", "#000", "#000", "#000", "#000"]);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.colors: has 6 colors; at most 5 (c1-c5) are allowed; \
             the shader is ignored"
        );
        let mut shader = good.clone();
        shader["colors"][1] = json!("teal");
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.colors[1]: \"teal\" is not a #rrggbb color; the shader is ignored"
        );
        let mut shader = good.clone();
        shader["custom"]["expression"] = json!("   ");
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.custom.expression: is empty; the custom shader has no code to run"
        );
        let mut shader = good.clone();
        shader["custom"]["expression"] = json!("x".repeat(MAX_CUSTOM_SHADER_BYTES + 1));
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.custom.expression: is 4097 bytes; shader code may be at \
             most 4096 bytes; the shader is ignored"
        );
        let mut shader = good.clone();
        shader["custom"]["variables"] = json!([{}, {}, {}, {}, {}]);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.custom.variables: has 5 sliders; at most 4 are allowed; \
             the shader is ignored"
        );
        let mut shader = good.clone();
        shader["custom"]["variables"][0]["min"] = json!(2.0);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.custom.variables[0]: min 2 is not below max 1; the shader is ignored"
        );
        let mut shader = good.clone();
        shader["custom"]["variables"][0]["max"] = json!(500.0);
        assert!(only_warning(&with_shader(shader)).contains("must stay within -100 to 100"));
        let mut shader = good.clone();
        shader["custom"]["variables"][0]["value"] = json!(4.0);
        assert_eq!(
            only_warning(&with_shader(shader)),
            "effects.shaderBackground.custom.variables[0].value: 4 is above the highest value, 1; 1 is used"
        );
        assert!(only_warning(&with_shader(json!("aurora"))).ends_with("the shader is ignored"));
    }

    #[test]
    fn shader_warnings_match_what_the_theme_loader_drops() {
        // Every "ignored" warning must mean the loader really drops the shader.
        for shader in [
            json!({"version": 2}),
            json!({"preset": "nope"}),
            json!({"colors": []}),
            json!({"colors": ["teal"]}),
            json!({"custom": {"expression": "x".repeat(MAX_CUSTOM_SHADER_BYTES + 1)}}),
            json!({"custom": {"variables": [{}, {}, {}, {}, {}]}}),
            json!({"custom": {"variables": [{"min": 2.0, "max": 1.0}]}}),
        ] {
            let warning = only_warning(&with_shader(shader.clone()));
            assert!(warning.ends_with("the shader is ignored"), "{warning}");
            assert_eq!(ShaderBackground::from_value(&shader), None, "{shader}");
        }
        let clamped = json!({"strength": 3.0});
        assert!(ShaderBackground::from_value(&clamped).is_some());
        assert!(only_warning(&with_shader(clamped)).ends_with("1 is used"));
    }

    fn bundle() -> Value {
        json!({
            "name": "Pair",
            "theme_id": "night",
            "app_theme_id": "night",
            "terminal_theme_id": "",
            "gradient_theme_id": "night",
            "effects_theme_id": "night",
            "panes": [{"theme_id": "night", "font": null}, {"theme_id": null}],
            "layout": "columns",
            "visible_panes": 2,
            "row_banding_brightness": 12,
            "chrome": {"dock_background": "#101010"}
        })
    }

    fn known(id: &str) -> bool {
        id == "night"
    }

    fn bundle_warning(bundle: &Value) -> String {
        let report = validate_bundle(bundle, known);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        report.warnings[0].to_string()
    }

    fn bundle_error(bundle: &Value) -> String {
        let report = validate_bundle(bundle, known);
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        report.errors[0].to_string()
    }

    #[test]
    fn bundle_findings_name_the_field_and_the_reason() {
        assert_eq!(
            validate_bundle(&bundle(), known),
            FormatReport {
                version: Some(1),
                ..Default::default()
            }
        );
        let mut newer = bundle();
        newer["format_version"] = json!(3);
        assert!(bundle_error(&newer).starts_with(
            "format_version: this bundle was made by a newer ButtonsCLI (bundle format 3; this version reads up to 1)"
        ));
        assert_eq!(
            bundle_error(&json!("Pair")),
            "(whole file): a bundle must be a JSON object ({ ... })"
        );
        let mut nameless = bundle();
        nameless["name"] = json!("");
        assert_eq!(
            bundle_error(&nameless),
            "name: is empty; bundles are listed and applied by name"
        );
        nameless.as_object_mut().unwrap().remove("name");
        assert_eq!(
            bundle_error(&nameless),
            "name: is missing; bundles are listed and applied by name"
        );
        let mut missing = bundle();
        missing["app_theme_id"] = json!("dawn");
        assert_eq!(
            bundle_warning(&missing),
            "app_theme_id: theme \"dawn\" is not installed; this part of the bundle is skipped"
        );
        missing["app_theme_id"] = json!(null);
        assert!(
            bundle_error(&missing).starts_with("app_theme_id: null should be a theme id string")
        );
        let mut pane = bundle();
        pane["panes"][1]["theme_id"] = json!("dawn");
        assert_eq!(
            bundle_warning(&pane),
            "panes[1].theme_id: theme \"dawn\" is not installed; this pane look is skipped"
        );
        pane["panes"][1] = json!("dawn");
        assert!(bundle_error(&pane).starts_with("panes[1]: \"dawn\" should be an object"));
        pane["panes"] = json!({});
        assert!(bundle_error(&pane).starts_with("panes: {} should be a list"));
        let mut many = bundle();
        many["panes"] = json!(vec![json!({"theme_id": "night"}); MAX_PANE_LOOKS + 2]);
        assert_eq!(
            bundle_warning(&many),
            "panes[8]: the bundle has 10 pane looks; only the first 8 are applied, the rest are kept"
        );
        let mut layout = bundle();
        layout["layout"] = json!("tiles");
        assert!(bundle_warning(&layout).ends_with("it is read as single"));
        layout["layout"] = json!(4);
        assert_eq!(bundle_error(&layout), "layout: 4 should be a layout name");
        let mut count = bundle();
        count["visible_panes"] = json!(500);
        assert_eq!(
            bundle_warning(&count),
            "visible_panes: 500 is above the highest value, 64; 64 is used"
        );
        count["visible_panes"] = json!(-1);
        assert!(bundle_error(&count).starts_with("visible_panes: -1 should be a whole number"));
        let mut banding = bundle();
        banding["row_banding_brightness"] = json!(100);
        assert_eq!(
            bundle_warning(&banding),
            "row_banding_brightness: 100 is above the highest value, 64; 64 is used"
        );
        banding["row_banding_brightness"] = json!(300);
        assert!(bundle_error(&banding).starts_with("row_banding_brightness: 300 should be"));
        let mut chrome = bundle();
        chrome["chrome"]["status_background"] = json!("#12");
        assert_eq!(
            bundle_warning(&chrome),
            "chrome.status_background: \"#12\" is not a #rrggbb color; the theme's color is used"
        );
        chrome["chrome"] = json!(true);
        assert!(bundle_error(&chrome).starts_with("chrome: true should be an object"));
        let mut extra = bundle();
        extra["notes"] = json!("from a newer app");
        assert_eq!(
            bundle_warning(&extra),
            "notes: is not a bundle field this version knows; it is kept but not used"
        );
        let mut font = bundle();
        font["typography"] = json!("Fira");
        assert!(bundle_error(&font).starts_with("(whole file): cannot be read: "));
    }

    #[test]
    fn bundle_fields_match_what_the_app_writes() {
        let full = ThemeBundle {
            row_banding_brightness: Some(1),
            chrome: Some(Default::default()),
            typography: Some(Default::default()),
            format_version: BundleFormat(2),
            ..Default::default()
        };
        let written = serde_json::to_value(full).unwrap();
        let mut keys: Vec<&str> = written
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut expected = BUNDLE_FIELDS.to_vec();
        keys.sort_unstable();
        expected.sort_unstable();
        assert_eq!(keys, expected);
        // Format 1 is not written, so files stay as older versions wrote them.
        let plain = serde_json::to_value(ThemeBundle::default()).unwrap();
        assert!(plain.get("format_version").is_none());
        let read: ThemeBundle = serde_json::from_value(json!({"name": "Old"})).unwrap();
        assert_eq!(read.format_version, BundleFormat(1));
    }

    fn rename(object: &mut Map<String, Value>) -> Vec<String> {
        let old = object.remove("colour").unwrap_or(Value::Null);
        object.insert("color".into(), old);
        vec!["renamed colour to color".into()]
    }

    fn darken(object: &mut Map<String, Value>) -> Vec<String> {
        // Needs the field the step before created.
        let color = object["color"].as_str().unwrap().to_owned();
        object.insert("dark".into(), Value::from(format!("{color}-dark")));
        vec!["added dark".into()]
    }

    // Listed out of order: the chain picks steps by `from`.
    const FAKE: Chain = Chain {
        what: "theme",
        field: "version",
        current: 3,
        steps: &[
            Step {
                from: 2,
                run: darken,
            },
            Step {
                from: 1,
                run: rename,
            },
        ],
    };

    #[test]
    fn upgrade_steps_run_in_order_and_keep_unknown_fields() {
        let (document, changes) = FAKE
            .upgrade(json!({"colour": "red", "plugin": {"keep": true}}))
            .unwrap();
        assert_eq!(
            document,
            json!({"version": 3, "color": "red", "dark": "red-dark", "plugin": {"keep": true}})
        );
        assert_eq!(
            changes,
            [
                "renamed colour to color",
                "upgraded theme format 1 to 2",
                "added dark",
                "upgraded theme format 2 to 3"
            ]
        );
        // A file already at version 2 only runs the last step.
        let (document, changes) = FAKE
            .upgrade(json!({"version": 2, "color": "blue"}))
            .unwrap();
        assert_eq!(document["dark"], "blue-dark");
        assert_eq!(changes.len(), 2);
        // Current files are not touched.
        let current = json!({"version": 3, "color": "x"});
        assert_eq!(
            FAKE.upgrade(current.clone()).unwrap(),
            (current, Vec::new())
        );
        assert_eq!(
            FAKE.upgrade(json!({"version": 4})),
            Err(FormatError::Newer {
                what: "theme",
                found: 4,
                supported: 3
            })
        );
        let gap = Chain { steps: &[], ..FAKE };
        assert_eq!(
            gap.upgrade(json!({})),
            Err(FormatError::MissingStep {
                what: "theme",
                from: 1
            })
        );
    }

    #[test]
    fn real_chains_have_a_step_for_every_older_version() {
        for chain in [THEME_CHAIN, BUNDLE_CHAIN] {
            for from in 1..chain.current {
                assert_eq!(
                    chain.steps.iter().filter(|step| step.from == from).count(),
                    1,
                    "{} format {from}",
                    chain.what
                );
            }
            assert!(chain.steps.iter().all(|step| step.from < chain.current));
        }
    }

    #[test]
    fn newer_and_unreadable_bundles_are_held_in_place_and_put_back() {
        let mut newer = bundle();
        newer["name"] = json!("Future");
        newer["format_version"] = json!(9);
        newer["panes"] = json!({"slots": "a new shape"});
        let mut broken = bundle();
        broken["name"] = json!("Broken");
        broken["visible_panes"] = json!("two");
        let mut extra = bundle();
        extra["name"] = json!("Extra");
        extra["notes"] = json!({"by": "a newer app"});
        let original = vec![bundle(), newer.clone(), extra.clone(), broken.clone()];
        let split = split_bundles(original.clone());
        assert_eq!(split.kept, vec![bundle(), extra.clone()]);
        assert_eq!(
            split
                .held
                .iter()
                .map(|held| (held.index, held.name.as_str()))
                .collect::<Vec<_>>(),
            [(1, "Future"), (3, "Broken")]
        );
        assert!(split.held[0].reason.contains("made by a newer ButtonsCLI"));
        assert!(split.held[1].reason.starts_with("it cannot be read"));
        assert_eq!(split.extras["Extra"]["notes"], json!({"by": "a newer app"}));
        // Saving what was read (unknown fields are not in ThemeBundle) gives
        // the original list back.
        let mut written: Vec<Value> = split
            .kept
            .iter()
            .map(|bundle| {
                let read: ThemeBundle = serde_json::from_value(bundle.clone()).unwrap();
                serde_json::to_value(read).unwrap()
            })
            .collect();
        assert!(written[1].get("notes").is_none());
        restore_bundles(&mut written, &split.held, &split.extras);
        assert_eq!(written.len(), 4);
        assert_eq!(written[1], newer);
        assert_eq!(written[3], broken);
        assert_eq!(written[2]["notes"], extra["notes"]);
        // A field the app now writes is never replaced by an old extra.
        let mut extras = BTreeMap::new();
        extras.insert(
            "Pair".to_owned(),
            json!({"layout": "grid"}).as_object().unwrap().clone(),
        );
        let mut list = vec![bundle()];
        restore_bundles(&mut list, &[], &extras);
        assert_eq!(list[0]["layout"], "columns");
        // Deleted bundles leave the held ones at the end, in order.
        let mut short = Vec::new();
        restore_bundles(&mut short, &split.held, &split.extras);
        assert_eq!(short, vec![newer, broken]);
    }

    #[test]
    fn shipped_themes_and_bundles_are_current_and_load_unchanged() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/themes");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let document: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let report = validate_theme_document(&document);
            assert!(
                report.errors.is_empty(),
                "{}: {:?}",
                path.display(),
                report.errors
            );
            let (upgraded, changes) = upgrade_theme_document(document.clone()).unwrap();
            assert_eq!(upgraded, document, "{}", path.display());
            assert!(changes.is_empty());
            checked += 1;
        }
        assert!(checked > 0);
        let catalog = crate::theme::ThemeCatalog::load();
        let installed = |id: &str| catalog.all().iter().any(|theme| theme.id == id);
        let bundles: Vec<Value> =
            serde_json::from_str(include_str!("../assets/library/bundles.json")).unwrap();
        assert!(!bundles.is_empty());
        for bundle in &bundles {
            let report = validate_bundle(bundle, installed);
            assert!(
                report.errors.is_empty() && report.warnings.is_empty(),
                "{}: {}",
                bundle["name"],
                report.summary(10)
            );
        }
        let split = split_bundles(bundles.clone());
        assert_eq!(split.kept, bundles);
        assert!(split.held.is_empty() && split.extras.is_empty());
    }

    #[test]
    fn summary_lists_errors_first_and_counts_the_rest() {
        let mut report = FormatReport::default();
        report.warn("/a", "w1");
        report.warn("/b/0", "w2");
        report.error("", "e1");
        assert_eq!(report.summary(2), "(whole file): e1; a: w1; and 1 more");
        assert!(!report.errors.is_empty());
        assert_eq!(reasons(&report.warnings), ["a: w1", "b[0]: w2"]);
    }
}
