//! Shaders and theme bundles that ship with the app. They live in
//! `assets/library/` and are chosen with Settings → Release picks; built-in
//! themes stay in `assets/themes/`.
use std::sync::OnceLock;

use crate::settings::ThemeBundle;
use crate::theme::SavedShader;

const SHADERS_JSON: &str = include_str!("../assets/library/shaders.json");
const BUNDLES_JSON: &str = include_str!("../assets/library/bundles.json");

/// Built-in shaders, listed after the user's saved ones in the Pattern picker.
pub(crate) fn shaders() -> &'static [SavedShader] {
    static SHADERS: OnceLock<Vec<SavedShader>> = OnceLock::new();
    SHADERS.get_or_init(|| parse(SHADERS_JSON, "shaders"))
}

/// Built-in theme bundles, listed after the user's own.
pub(crate) fn bundles() -> &'static [ThemeBundle] {
    static BUNDLES: OnceLock<Vec<ThemeBundle>> = OnceLock::new();
    BUNDLES.get_or_init(|| parse(BUNDLES_JSON, "bundles"))
}

fn parse<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> Vec<T> {
    serde_json::from_str(json).unwrap_or_else(|error| {
        log::warn!("built-in {what} are invalid: {error}");
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn built_in_library_parses_and_every_shader_compiles() {
        assert!(serde_json::from_str::<serde_json::Value>(super::SHADERS_JSON).is_ok());
        assert!(serde_json::from_str::<serde_json::Value>(super::BUNDLES_JSON).is_ok());
        for item in super::shaders() {
            crate::plugins::effects::shader_expression::function(&item.shader)
                .unwrap_or_else(|error| panic!("{}: {error}", item.shader.name));
        }
        super::bundles();
    }
}
