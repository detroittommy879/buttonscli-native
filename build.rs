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

    build_version();
}

/// `BUTTONSCLI_BUILD_VERSION`: the Cargo version, plus `+<short commit>` for
/// builds that are not release builds, so bug reports name the exact build.
/// The release workflow sets `BUTTONSCLI_RELEASE_BUILD=1` after writing the
/// auto-incremented version into Cargo.toml (`scripts/release-version.sh`).
fn build_version() {
    println!("cargo:rerun-if-env-changed=BUTTONSCLI_RELEASE_BUILD");
    let version = env::var("CARGO_PKG_VERSION").unwrap();
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
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
            watch(common.join(reference));
        }
    }
    let full = match commit {
        Some(commit) => format!("{version}+{commit}"),
        None => version,
    };
    println!("cargo:rustc-env=BUTTONSCLI_BUILD_VERSION={full}");
}
