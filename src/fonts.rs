use egui::{FontData, FontDefinitions, FontFamily, FontId};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::hash::{Hash, Hasher};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontKind {
    Ui,
    Mono,
    Novelty,
    Symbols,
    Emoji,
}

#[derive(Clone, Copy, Debug)]
pub struct FontFace {
    pub id: &'static str,
    pub family: &'static str,
    pub file: &'static str,
    pub weight: u16,
    pub kind: FontKind,
    pub bytes: &'static [u8],
}

macro_rules! face {
    ($id:literal, $family:literal, $file:literal, $weight:literal, $kind:ident) => {
        FontFace {
            id: $id,
            family: $family,
            file: $file,
            weight: $weight,
            kind: FontKind::$kind,
            bytes: include_bytes!(concat!("../assets/fonts/", $file)),
        }
    };
}

pub static FONT_FACES: &[FontFace] = &[
    face!(
        "commit-mono-200",
        "Commit Mono Bundled",
        "CommitMono-200-Regular.otf",
        200,
        Mono
    ),
    face!(
        "daddy-time-mono",
        "Daddy Time Mono Bundled",
        "DaddyTimeMono.otf",
        400,
        Mono
    ),
    face!(
        "fira-code",
        "Fira Code Bundled",
        "FiraCode-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "go-noto-current",
        "Go Noto Current Bundled",
        "GoNotoCurrent-Regular.ttf",
        400,
        Ui
    ),
    face!(
        "noto-sans-kr",
        "Noto Sans KR Bundled",
        "NotoSansKR-Subset.ttf",
        400,
        Ui
    ),
    face!(
        "jetbrains-mono-200",
        "JetBrains Mono Bundled",
        "JetBrainsMono-ExtraLight.ttf",
        200,
        Mono
    ),
    face!(
        "jetbrains-mono-300",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Light.ttf",
        300,
        Mono
    ),
    face!(
        "jetbrains-mono-400",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "jetbrains-mono-100",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Thin.ttf",
        100,
        Mono
    ),
    face!(
        "monoisome",
        "Monoisome Bundled",
        "Monoisome-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "noto-color-emoji",
        "Noto Color Emoji Bundled",
        "NotoColorEmoji.ttf",
        400,
        Emoji
    ),
    face!(
        "recursive-mono-casual",
        "Recursive Mono Csl St Bundled",
        "RecursiveMonoCslSt-Light.ttf",
        300,
        Mono
    ),
    face!(
        "recursive-mono-linear",
        "Recursive Mono Lnr St Bundled",
        "RecursiveMonoLnrSt-Light.ttf",
        300,
        Mono
    ),
    face!(
        "recursive-sans-casual",
        "Recursive Sans Csl St Bundled",
        "RecursiveSansCslSt-Light.ttf",
        300,
        Ui
    ),
    face!(
        "recursive-sans-linear",
        "Recursive Sans Lnr St Bundled",
        "RecursiveSansLnrSt-Light.ttf",
        300,
        Ui
    ),
    face!("roboto-300", "Roboto Bundled", "Roboto-Light.ttf", 300, Ui),
    face!(
        "roboto-300-italic",
        "Roboto Bundled",
        "Roboto-LightItalic.ttf",
        300,
        Ui
    ),
    face!("roboto-500", "Roboto Bundled", "Roboto-Medium.ttf", 500, Ui),
    face!(
        "roboto-400",
        "Roboto Bundled",
        "Roboto-Regular.ttf",
        400,
        Ui
    ),
    face!("roboto-100", "Roboto Bundled", "Roboto-Thin.ttf", 100, Ui),
    face!(
        "serious-shanns",
        "Serious Shanns Bundled",
        "SeriousShanns-Light.otf",
        300,
        Novelty
    ),
    face!(
        "symbols-nerd",
        "Symbols Nerd Font Mono Bundled",
        "SymbolsNerdFontMono-Regular.ttf",
        400,
        Symbols
    ),
    face!(
        "code-new-roman",
        "Code New Roman Bundled",
        "cnr.otf",
        400,
        Ui
    ),
    face!(
        "ia-writer-mono",
        "iA Writer Mono S Bundled",
        "iAWriterMonoS-Regular.ttf",
        400,
        Mono
    ),
    face!("monofur-400", "monof55 Bundled", "monof55.ttf", 400, Mono),
    face!(
        "monofur-italic",
        "monof56 Bundled",
        "monof56.ttf",
        400,
        Mono
    ),
    face!("saxmono", "saxmono Bundled", "saxmono.ttf", 400, Mono),
];

#[derive(Clone, Debug)]
struct LoadedFace {
    id: String,
    family: String,
    file: String,
    weight: u16,
    kind: FontKind,
    data: Arc<FontData>,
}

#[derive(Clone, Debug)]
pub(crate) struct FontCatalog {
    faces: Vec<LoadedFace>,
}

impl FontCatalog {
    pub(crate) fn bundled() -> Self {
        Self {
            faces: FONT_FACES
                .iter()
                .map(|face| LoadedFace {
                    id: face.id.to_owned(),
                    family: face.family.to_owned(),
                    file: face.file.to_owned(),
                    weight: face.weight,
                    kind: face.kind,
                    data: Arc::new(FontData::from_static(face.bytes)),
                })
                .collect(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load(profile_fonts: Option<&Path>) -> (Self, Vec<String>) {
        Self::load_with_directories(profile_fonts, &system_font_directories())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_with_directories(
        profile_fonts: Option<&Path>,
        system_directories: &[PathBuf],
    ) -> (Self, Vec<String>) {
        let mut catalog = Self::bundled();
        let mut warnings = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();
        let mut total_bytes = catalog
            .faces
            .iter()
            .map(|face| face.data.font.len())
            .sum::<usize>();
        let mut remaining = MAX_DISCOVERED_FONT_FILES;
        let mut remaining_entries = MAX_SCANNED_FONT_ENTRIES;

        if let Some(directory) = profile_fonts {
            catalog.append_directory(
                directory,
                FontSource::Custom,
                true,
                &mut seen_paths,
                (&mut remaining, &mut remaining_entries, &mut total_bytes),
                &mut warnings,
            );
        }
        for directory in system_directories {
            catalog.append_directory(
                directory,
                FontSource::System,
                false,
                &mut seen_paths,
                (&mut remaining, &mut remaining_entries, &mut total_bytes),
                &mut warnings,
            );
        }
        catalog.faces.sort_by(|left, right| {
            left.family
                .cmp(&right.family)
                .then(left.weight.cmp(&right.weight))
                .then(left.id.cmp(&right.id))
        });
        (catalog, warnings)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn append_directory(
        &mut self,
        directory: &Path,
        source: FontSource,
        report_invalid: bool,
        seen_paths: &mut std::collections::HashSet<PathBuf>,
        budget: (&mut usize, &mut usize, &mut usize),
        warnings: &mut Vec<String>,
    ) {
        let (remaining, remaining_entries, total_bytes) = budget;
        if !directory.is_dir() || *remaining == 0 {
            return;
        }
        let mut paths = Vec::new();
        collect_font_paths(directory, 0, &mut paths, remaining_entries);
        paths.sort();
        for path in paths {
            if *remaining == 0 || *total_bytes >= MAX_TOTAL_FONT_BYTES {
                break;
            }
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !seen_paths.insert(canonical) {
                continue;
            }
            *remaining -= 1;
            match load_external_face(&path, source) {
                Ok(face) => {
                    let size = face.data.font.len();
                    if total_bytes.saturating_add(size) > MAX_TOTAL_FONT_BYTES {
                        if report_invalid {
                            warnings.push(format!(
                                "{}: font catalog reached its memory limit",
                                path.display()
                            ));
                        }
                        break;
                    }
                    *total_bytes += size;
                    if !self.faces.iter().any(|existing| existing.id == face.id) {
                        self.faces.push(face);
                    }
                }
                Err(error) if report_invalid => {
                    warnings.push(format!("{}: {error}", path.display()));
                }
                Err(_) => {}
            }
        }
    }

    pub(crate) fn family_names(&self, monospace_only: bool) -> Vec<String> {
        let mut names = self
            .faces
            .iter()
            .filter(|face| {
                !monospace_only || matches!(face.kind, FontKind::Mono | FontKind::Symbols)
            })
            .map(|face| face.family.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        names
    }

    pub(crate) fn weights_for(&self, family: &str) -> Vec<u16> {
        let mut weights = self
            .faces
            .iter()
            .filter(|face| face.family == family)
            .map(|face| face.weight)
            .collect::<Vec<_>>();
        weights.sort_unstable();
        weights.dedup();
        weights
    }

    pub(crate) fn face_files(&self, family: &str) -> Vec<String> {
        self.faces
            .iter()
            .filter(|face| face.family == family)
            .map(|face| face.file.clone())
            .collect()
    }

    pub(crate) fn is_available(&self, family: &str) -> bool {
        self.faces.iter().any(|face| face.family == family)
    }

    pub(crate) fn resolved_weight(&self, family: &str, requested: u16) -> u16 {
        self.nearest_face(family, requested)
            .map(|face| face.weight)
            .unwrap_or(requested)
    }

    pub(crate) fn font_family(&self, zone: &FontZone, monospace_only: bool) -> FontFamily {
        let selected = self.nearest_face(&zone.family, zone.weight).or_else(|| {
            let fallback = if monospace_only {
                DEFAULT_MONO_FONT
            } else {
                DEFAULT_UI_FONT
            };
            self.nearest_face(fallback, 400)
        });
        selected
            .map(|face| FontFamily::Name(format!("face:{}", face.id).into()))
            .unwrap_or(if monospace_only {
                FontFamily::Monospace
            } else {
                FontFamily::Proportional
            })
    }

    pub(crate) fn font_id(&self, zone: &FontZone, monospace_only: bool) -> FontId {
        FontId::new(zone.size, self.font_family(zone, monospace_only))
    }

    pub(crate) fn install(&self, ctx: &egui::Context) {
        let mut definitions = FontDefinitions::default();
        let mut families: BTreeMap<String, Vec<&LoadedFace>> = BTreeMap::new();
        for face in &self.faces {
            definitions
                .font_data
                .insert(face.id.clone(), face.data.clone());
            families.entry(face.family.clone()).or_default().push(face);
            let mut fallback = vec![face.id.clone()];
            for id in ["symbols-nerd", "go-noto-current", "noto-sans-kr"] {
                if id != face.id {
                    fallback.push(id.into());
                }
            }
            definitions.families.insert(
                FontFamily::Name(format!("face:{}", face.id).into()),
                fallback,
            );
        }
        for (family, mut faces) in families {
            faces.sort_by_key(|face| face.weight.abs_diff(400));
            let mut fallback = faces.iter().map(|face| face.id.clone()).collect::<Vec<_>>();
            for id in ["symbols-nerd", "go-noto-current", "noto-sans-kr"] {
                if !fallback.iter().any(|value| value == id) {
                    fallback.push(id.into());
                }
            }
            definitions
                .families
                .insert(FontFamily::Name(family.into()), fallback);
        }
        definitions
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .splice(
                0..0,
                [
                    "go-noto-current".into(),
                    "symbols-nerd".into(),
                    "noto-sans-kr".into(),
                ],
            );
        definitions
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .splice(
                0..0,
                [
                    "recursive-mono-casual".into(),
                    "symbols-nerd".into(),
                    "go-noto-current".into(),
                    "noto-sans-kr".into(),
                ],
            );
        ctx.set_fonts(definitions);
    }

    fn nearest_face(&self, family: &str, weight: u16) -> Option<&LoadedFace> {
        self.faces
            .iter()
            .filter(|face| face.family == family)
            .min_by_key(|face| face.weight.abs_diff(weight))
    }
}

#[cfg(not(target_arch = "wasm32"))]
const MAX_FONT_FILE_BYTES: u64 = 32 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const MAX_TOTAL_FONT_BYTES: usize = 256 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const MAX_DISCOVERED_FONT_FILES: usize = 512;
#[cfg(not(target_arch = "wasm32"))]
const MAX_SCANNED_FONT_ENTRIES: usize = 8192;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum FontSource {
    Custom,
    System,
}

#[cfg(not(target_arch = "wasm32"))]
fn load_external_face(path: &Path, source: FontSource) -> Result<LoadedFace, String> {
    let bytes = read_font_bytes(path)?;
    let (family, weight, kind) = inspect_font_bytes(&bytes)?;

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    path.file_name().hash(&mut hasher);
    let source_key = match source {
        FontSource::Custom => "custom",
        FontSource::System => "system",
    };
    source_key.hash(&mut hasher);
    let id = format!("installed-font-{:016x}", hasher.finish());
    let display_source = match source {
        FontSource::Custom => "Custom",
        FontSource::System => "System",
    };
    Ok(LoadedFace {
        id,
        family: format!("{family} ({display_source})"),
        file: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("font file")
            .to_owned(),
        weight,
        kind,
        data: Arc::new(FontData::from_owned(bytes)),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn read_font_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_FONT_FILE_BYTES {
        return Err("font file is larger than 32 MiB or is not a regular file".into());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "ttf" | "otf") {
        return Err("only TrueType .ttf and OpenType .otf files are supported".into());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_FONT_FILE_BYTES {
        return Err("font file is larger than 32 MiB".into());
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn inspect_font_bytes(bytes: &[u8]) -> Result<(String, u16, FontKind), String> {
    let face = ttf_parser::Face::parse(bytes, 0)
        .map_err(|error| format!("font tables could not be read: {error:?}"))?;
    let family = preferred_name(&face).ok_or_else(|| "font family name is missing".to_owned())?;
    ab_glyph::FontArc::try_from_vec(bytes.to_vec())
        .map_err(|error| format!("font outlines are unsupported: {error}"))?;
    let kind = if face.is_monospaced() {
        FontKind::Mono
    } else if family.to_ascii_lowercase().contains("nerd") {
        FontKind::Symbols
    } else {
        FontKind::Ui
    };
    Ok((family, face.weight().to_number().clamp(100, 900), kind))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn import_custom_font_file(
    store: &crate::storage::store::NativeStore,
    source: &Path,
) -> Result<String, String> {
    let bytes = read_font_bytes(source)?;
    inspect_font_bytes(&bytes)?;
    let original_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "font file name is not valid Unicode".to_owned())?;
    let directory = store
        .profile_fonts_dir()
        .map_err(|error| error.to_string())?;
    let mut file_name = safe_font_file_name(original_name);
    for _ in 0..32 {
        match store.write_font_file(&file_name, &bytes) {
            Ok(()) => return Ok(file_name),
            Err(crate::storage::store::StoreError::FontCollision) => {
                file_name = unique_font_file_name(&directory, original_name);
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("font filename kept colliding; choose another file name and retry".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn safe_font_file_name(original: &str) -> String {
    let extension = Path::new(original)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let stem = Path::new(original)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("custom-font");
    let mut stem = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while stem.contains("--") {
        stem = stem.replace("--", "-");
    }
    stem = stem.trim_matches(['-', '_']).chars().take(80).collect();
    if stem.is_empty() {
        stem = "custom-font".into();
    }
    format!("{stem}.{}", if extension == "otf" { "otf" } else { "ttf" })
}

#[cfg(not(target_arch = "wasm32"))]
fn unique_font_file_name(directory: &Path, original: &str) -> String {
    let base = safe_font_file_name(original);
    let path = Path::new(&base);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("font")
        .to_owned();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("ttf")
        .to_owned();
    let mut candidate = base;
    let mut suffix = 2_u32;
    while directory.join(&candidate).exists() {
        candidate = format!("{stem}-{suffix}.{extension}");
        suffix = suffix.saturating_add(1);
    }
    candidate
}

#[cfg(not(target_arch = "wasm32"))]
fn preferred_name(face: &ttf_parser::Face<'_>) -> Option<String> {
    use ttf_parser::name_id;

    let names = face.names();
    for target in [name_id::TYPOGRAPHIC_FAMILY, name_id::FAMILY] {
        let records = names
            .into_iter()
            .filter(|record| record.name_id == target)
            .collect::<Vec<_>>();
        if let Some(record) = records
            .iter()
            .find(|record| record.language() == ttf_parser::Language::English_UnitedStates)
            .or_else(|| records.first())
        {
            if let Some(name) = record.to_string().filter(|name| !name.trim().is_empty()) {
                return Some(name.trim().to_owned());
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_font_paths(
    directory: &Path,
    depth: usize,
    paths: &mut Vec<PathBuf>,
    remaining_entries: &mut usize,
) {
    if depth > 3 || paths.len() >= MAX_DISCOVERED_FONT_FILES || *remaining_entries == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if paths.len() >= MAX_DISCOVERED_FONT_FILES || *remaining_entries == 0 {
            break;
        }
        *remaining_entries -= 1;
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect_font_paths(&path, depth + 1, paths, remaining_entries);
        } else if kind.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("ttf") || extension.eq_ignore_ascii_case("otf")
                })
        {
            paths.push(path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn system_font_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(windows) = std::env::var_os("WINDIR") {
            directories.push(PathBuf::from(windows).join("Fonts"));
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            directories.push(PathBuf::from(local_app_data).join("Microsoft/Windows/Fonts"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        directories.extend([
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ]);
        if let Some(home) = home::home_dir() {
            directories.push(home.join(".fonts"));
            directories.push(home.join(".local/share/fonts"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        directories.extend([
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/Library/Fonts"),
        ]);
        if let Some(home) = home::home_dir() {
            directories.push(home.join("Library/Fonts"));
        }
    }
    directories
}

pub const DEFAULT_UI_FONT: &str = "Go Noto Current Bundled";
pub const DEFAULT_DISPLAY_FONT: &str = "Recursive Sans Csl St Bundled";
pub const DEFAULT_MONO_FONT: &str = "Recursive Mono Csl St Bundled";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FontZone {
    pub family: String,
    pub size: f32,
    pub weight: u16,
    pub letter_spacing: f32,
}

impl Default for FontZone {
    fn default() -> Self {
        Self {
            family: DEFAULT_UI_FONT.into(),
            size: 14.0,
            weight: 400,
            letter_spacing: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Typography {
    pub shell: FontZone,
    pub tabs: FontZone,
    pub preset_dock: FontZone,
    pub settings: FontZone,
    pub assistant: FontZone,
    pub status_bar: FontZone,
    pub terminal: FontZone,
    pub terminal_bold_weight: u16,
    pub draw_bold_bright: bool,
}

/// Session-local terminal typography, independent of application chrome.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PaneFont {
    pub zone: FontZone,
    pub bold_weight: u16,
    pub draw_bold_bright: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&Typography> for PaneFont {
    fn from(value: &Typography) -> Self {
        Self {
            zone: value.terminal.clone(),
            bold_weight: value.terminal_bold_weight,
            draw_bold_bright: value.draw_bold_bright,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            shell: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            tabs: FontZone {
                family: DEFAULT_DISPLAY_FONT.into(),
                size: 13.0,
                weight: 500,
                ..Default::default()
            },
            preset_dock: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 13.0,
                ..Default::default()
            },
            settings: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            assistant: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            status_bar: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 12.0,
                ..Default::default()
            },
            terminal: FontZone {
                family: DEFAULT_MONO_FONT.into(),
                size: 13.0,
                weight: 300,
                ..Default::default()
            },
            terminal_bold_weight: 700,
            draw_bold_bright: true,
        }
    }
}

pub(crate) fn install(ctx: &egui::Context, catalog: &FontCatalog) {
    catalog.install(ctx);
}

pub fn family_names(monospace_only: bool) -> Vec<&'static str> {
    let mut names = Vec::new();
    for face in FONT_FACES {
        let allowed = !monospace_only || matches!(face.kind, FontKind::Mono | FontKind::Symbols);
        if allowed && !names.contains(&face.family) {
            names.push(face.family);
        }
    }
    names
}

pub fn weights_for(family: &str) -> Vec<u16> {
    let mut weights: Vec<_> = FONT_FACES
        .iter()
        .filter(|face| face.family == family)
        .map(|face| face.weight)
        .collect();
    weights.sort_unstable();
    weights.dedup();
    weights
}

pub fn resolve_bundled_family(requested: &str, monospace_only: bool) -> String {
    let requested = match requested {
        "Jet Brains Mono Bundled" => "JetBrains Mono Bundled",
        "i A Writer Mono S Bundled" => "iA Writer Mono S Bundled",
        other => other,
    };
    let families = family_names(monospace_only);
    if let Some(family) = families.iter().find(|family| **family == requested) {
        return (*family).to_owned();
    }
    if let Some(family) = families.iter().find(|family| {
        family
            .strip_suffix(" Bundled")
            .is_some_and(|name| name.eq_ignore_ascii_case(requested))
    }) {
        return (*family).to_owned();
    }
    if monospace_only {
        DEFAULT_MONO_FONT.into()
    } else {
        DEFAULT_UI_FONT.into()
    }
}

pub fn font_family(zone: &FontZone) -> FontFamily {
    let selected = nearest_face(&zone.family, zone.weight);
    selected
        .map(|face| FontFamily::Name(format!("face:{}", face.id).into()))
        .unwrap_or_else(|| FontFamily::Name(zone.family.clone().into()))
}

pub fn font_id(zone: &FontZone) -> FontId {
    FontId::new(zone.size, font_family(zone))
}

pub fn resolved_weight(family: &str, requested: u16) -> u16 {
    nearest_face(family, requested)
        .map(|face| face.weight)
        .unwrap_or(requested)
}

fn nearest_face(family: &str, weight: u16) -> Option<&'static FontFace> {
    FONT_FACES
        .iter()
        .filter(|face| face.family == family)
        .min_by_key(|face| face.weight.abs_diff(weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "buttonscli-font-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn bundled_face_catalog_has_expected_families() {
        assert_eq!(FONT_FACES.len(), 27);
        assert_eq!(family_names(false).len(), 20);
        assert_eq!(family_names(true).len(), 12);
        assert!(family_names(false).contains(&"Roboto Bundled"));
        assert!(family_names(false).contains(&"Noto Sans KR Bundled"));
        assert!(!family_names(true).contains(&"Noto Sans KR Bundled"));
    }

    #[test]
    fn nearest_weight_selects_real_face() {
        assert_eq!(
            nearest_face("Roboto Bundled", 450).unwrap().id,
            "roboto-500"
        );
    }

    #[test]
    fn legacy_and_online_names_sanitize_to_packaged_families() {
        assert_eq!(
            resolve_bundled_family("Fira Code", true),
            "Fira Code Bundled"
        );
        assert_eq!(
            resolve_bundled_family("Inter", false),
            "Go Noto Current Bundled"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn invalid_custom_font_is_skipped_and_missing_selection_uses_bundled_fallback() {
        let root = test_path("invalid");
        let custom = root.join("custom");
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(custom.join("broken.ttf"), b"not a font").unwrap();
        let (catalog, warnings) = FontCatalog::load_with_directories(Some(&custom), &[]);
        assert_eq!(catalog.faces.len(), FONT_FACES.len());
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("broken.ttf")));

        let missing_mono = FontZone {
            family: "Uninstalled Custom Font".into(),
            ..FontZone::default()
        };
        let fallback = FontZone {
            family: DEFAULT_MONO_FONT.into(),
            ..FontZone::default()
        };
        assert_eq!(
            catalog.font_family(&missing_mono, true),
            catalog.font_family(&fallback, true)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn directory_walk_stops_at_its_entry_budget() {
        let directory = test_path("entry-budget");
        for index in 0..12 {
            std::fs::write(directory.join(format!("plain-{index}.txt")), "not a font").unwrap();
        }
        let mut paths = Vec::new();
        let mut remaining_entries = 5;

        collect_font_paths(&directory, 0, &mut paths, &mut remaining_entries);

        assert!(paths.is_empty());
        assert_eq!(remaining_entries, 0);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn custom_font_import_is_validated_stored_and_loaded_without_network_access() {
        let root = test_path("import");
        let native = root.join("native");
        let store = crate::storage::store::NativeStore::open(
            crate::storage::paths::NativeDataRoot(native),
            root.join("original"),
        )
        .unwrap();
        let source = root.join("Fira Code.ttf");
        let bundled = FONT_FACES
            .iter()
            .find(|face| face.id == "fira-code")
            .unwrap();
        std::fs::write(&source, bundled.bytes).unwrap();

        let first = import_custom_font_file(&store, &source).unwrap();
        let second = import_custom_font_file(&store, &source).unwrap();
        assert_eq!(first, "fira-code.ttf");
        assert_eq!(second, "fira-code-2.ttf");
        let custom_dir = store.profile_fonts_dir().unwrap();
        let (catalog, warnings) = FontCatalog::load_with_directories(Some(&custom_dir), &[]);
        assert!(warnings.is_empty());
        assert!(catalog.is_available("Fira Code (Custom)"));
        assert!(catalog.weights_for("Fira Code (Custom)").contains(&400));
        assert!(catalog.face_files("Fira Code (Custom)").contains(&first));

        let system_dir = root.join("system");
        std::fs::create_dir_all(&system_dir).unwrap();
        std::fs::copy(&source, system_dir.join("Fira Code.ttf")).unwrap();
        let (with_system_fonts, warnings) =
            FontCatalog::load_with_directories(Some(&custom_dir), &[system_dir]);
        assert!(warnings.is_empty());
        assert!(with_system_fonts.is_available("Fira Code (Custom)"));
        assert!(with_system_fonts.is_available("Fira Code (System)"));

        let broken = root.join("broken.ttf");
        std::fs::write(&broken, b"invalid").unwrap();
        assert!(import_custom_font_file(&store, &broken).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn terminal_font_selection_keeps_regular_and_bold_faces_and_unicode_fallback() {
        let context = egui::Context::default();
        let catalog = FontCatalog::bundled();
        let regular = FontZone {
            family: "JetBrains Mono Bundled".into(),
            weight: 200,
            size: 13.0,
            letter_spacing: 0.0,
        };
        let bold = FontZone {
            weight: 400,
            ..regular.clone()
        };
        let regular_id = catalog.font_id(&regular, true);
        let bold_id = catalog.font_id(&bold, true);
        assert_ne!(regular_id.family, bold_id.family);
        catalog.install(&context);
        let mut widths = None;
        let _ = context.run(egui::RawInput::default(), |ctx| {
            widths = Some(ctx.fonts(|fonts| {
                (
                    fonts.glyph_width(&regular_id, 'm'),
                    fonts.glyph_width(&bold_id, 'm'),
                    fonts.glyph_width(&regular_id, '한'),
                )
            }));
        });
        let (regular_width, bold_width, korean_width) = widths.unwrap();
        assert!(regular_width > 0.0);
        assert!(bold_width > 0.0);
        assert!(korean_width > 0.0);
    }
}
