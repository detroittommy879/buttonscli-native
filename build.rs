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
}
