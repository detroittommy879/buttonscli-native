#[allow(dead_code)]
#[path = "../distribution.rs"]
mod distribution;

use distribution::{build_signed_release_package, ReleaseBuildRequest, ReleaseTarget};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)?;
    if command == "--help" || command == "-h" {
        println!("{}", usage());
        return Ok(());
    }
    if command != "package" {
        return Err(usage());
    }

    let options = parse_options(arguments.collect())?;
    let source_directory = required_path(&options, "--source")?;
    let output_directory = required_path(&options, "--output")?;
    let signing_key_file = required_path(&options, "--key-file")?;
    let version = required_text(&options, "--version")?;
    let target_os = required_text(&options, "--target-os")?;
    let target_arch = required_text(&options, "--target-arch")?;
    let minimum_updater_version = required_text(&options, "--minimum-updater-version")?;
    let channel = required_text(&options, "--channel")?;
    let key_id = required_text(&options, "--key-id")?;

    let built = build_signed_release_package(ReleaseBuildRequest {
        source_directory: &source_directory,
        output_directory: &output_directory,
        signing_key_file: &signing_key_file,
        version: &version,
        target: ReleaseTarget {
            os: &target_os,
            arch: &target_arch,
        },
        minimum_updater_version: &minimum_updater_version,
        channel: &channel,
        key_id: &key_id,
    })
    .map_err(|error| error.to_string())?;

    println!("artifact: {}", built.artifact_path.display());
    println!("manifest: {}", built.manifest_path.display());
    println!("signature: {}", built.signature_path.display());
    println!(
        "public key for the reviewed native trust store ({}): {}",
        key_id,
        built
            .public_key
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    println!("The public key requires manual review before release verification can trust it.");
    Ok(())
}

fn parse_options(arguments: Vec<OsString>) -> Result<BTreeMap<String, OsString>, String> {
    let mut options = BTreeMap::new();
    let mut arguments = arguments.into_iter();
    while let Some(name) = arguments.next() {
        let name = name
            .into_string()
            .map_err(|_| "option names must be UTF-8".to_owned())?;
        if !matches!(
            name.as_str(),
            "--source"
                | "--output"
                | "--key-file"
                | "--version"
                | "--target-os"
                | "--target-arch"
                | "--minimum-updater-version"
                | "--channel"
                | "--key-id"
        ) {
            return Err(format!("unknown option: {name}\n{}", usage()));
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {name}\n{}", usage()))?;
        if options.insert(name.clone(), value).is_some() {
            return Err(format!("duplicate option: {name}\n{}", usage()));
        }
    }
    Ok(options)
}

fn required_path(options: &BTreeMap<String, OsString>, name: &str) -> Result<PathBuf, String> {
    options
        .get(name)
        .cloned()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required option: {name}\n{}", usage()))
}

fn required_text(options: &BTreeMap<String, OsString>, name: &str) -> Result<String, String> {
    options
        .get(name)
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing or non-UTF-8 option: {name}\n{}", usage()))
}

fn usage() -> String {
    "Usage: native-release package --source <prepared-dir> --output <new-dir> --key-file <32-byte-seed> --version <semver> --target-os <os> --target-arch <arch> --minimum-updater-version <semver> --channel <stable|beta> --key-id <id>\n\
         The signing seed must be supplied and stored outside the source directory."
        .to_owned()
}
