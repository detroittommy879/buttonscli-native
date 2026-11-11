use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/themes");
    println!("cargo:rerun-if-changed=assets/generated/legacy-code-themes.json");

    let mut files: Vec<_> = fs::read_dir("assets/themes")
        .expect("theme asset directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();

    let mut generated = String::from("pub const BUNDLED_THEME_JSON: &[(&str, &str)] = &[\n");
    for path in files {
        let file_name = path.file_name().unwrap().to_string_lossy();
        let id = path.file_stem().unwrap().to_string_lossy();
        generated.push_str(&format!(
            "    ({id:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/themes/{file_name}\"))),\n"
        ));
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bundled_themes.rs");
    fs::write(output, generated).expect("write generated theme catalog");

    let version = build_version();
    windows_resources(&version);
}

/// Exe icon (Explorer, pinned taskbar buttons, shortcuts) and file version.
/// The running window's icon is set separately (`buttonscli::app_icon`).
#[cfg(windows)]
fn windows_resources(version: &str) {
    println!("cargo:rerun-if-changed=packaging/icon.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("packaging/icon.ico")
        .set("ProductName", "ButtonsCLI")
        .set("FileDescription", "ButtonsCLI")
        .set("ProductVersion", version)
        .set("FileVersion", version);
    // The numeric version too, or Windows shows the Cargo.toml floor.
    if let Some((major, minor, patch)) = parse_version(version) {
        let number = (major << 48) | (minor << 32) | (patch << 16);
        resource
            .set_version_info(winresource::VersionInfo::FILEVERSION, number)
            .set_version_info(winresource::VersionInfo::PRODUCTVERSION, number);
    }
    if let Err(error) = resource.compile() {
        println!("cargo:warning=exe icon not embedded: {error}");
    }
}

#[cfg(not(windows))]
fn windows_resources(_version: &str) {}

/// `BUTTONSCLI_BUILD_VERSION`. Release builds: the Cargo version, which the
/// release workflow sets (`scripts/release-version.sh`) together with
/// `BUTTONSCLI_RELEASE_BUILD=1`. Other builds: the newer of the Cargo version
/// and the newest release tag behind this commit, plus `+<short commit>`
/// (SemVer build metadata), so bug reports name the exact build.
fn build_version() -> String {
    println!("cargo:rerun-if-env-changed=BUTTONSCLI_RELEASE_BUILD");
    let mut version = env::var("CARGO_PKG_VERSION").unwrap();
    let release = env::var("BUTTONSCLI_RELEASE_BUILD").is_ok_and(|value| value == "1");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|text| !text.is_empty())
    };
    let commit = if release {
        None
    } else {
        git(&["rev-parse", "--short=8", "HEAD"])
    };
    if !release {
        // Cargo.toml stays at its floor while releases count up from tags.
        // Shallow CI checkouts have no tags: keep the Cargo version.
        let tagged = git(&[
            "describe",
            "--tags",
            "--abbrev=0",
            "--match",
            "v[0-9]*.[0-9]*.[0-9]*",
            "--exclude",
            "*-*",
        ])
        .and_then(|tag| parse_version(tag.trim_start_matches('v')));
        if let (Some(tagged), Some(current)) = (tagged, parse_version(&version)) {
            if tagged > current {
                version = format!("{}.{}.{}", tagged.0, tagged.1, tagged.2);
            }
        }
    }
    // Rebuild the version string when the checked-out commit changes. Watch
    // only files that exist: Cargo treats a missing watched path as changed
    // and would rebuild the whole crate on every run (no packed-refs yet,
    // loose refs removed by `git gc`).
    let watch = |path: PathBuf| {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    };
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        watch(PathBuf::from(dir).join("HEAD"));
    }
    if let Some(common) = git(&["rev-parse", "--git-common-dir"]).map(PathBuf::from) {
        watch(common.join("packed-refs"));
        watch(common.join("refs").join("tags"));
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
            watch(common.join(reference));
        }
    }
    let full = match commit {
        Some(commit) => format!("{version}+{commit}"),
        None => version,
    };
    println!("cargo:rustc-env=BUTTONSCLI_BUILD_VERSION={full}");
    full
}

/// `X.Y.Z` (ignoring any `-pre`/`+build` suffix) as numbers, for comparing.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|part| part.parse().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}
