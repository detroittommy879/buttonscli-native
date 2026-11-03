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
        // A syntactically valid JSON document can still fail typed deserialization.
        // Do not let the runtime's empty-list fallback hide broken shipped assets.
        let shaders: Vec<crate::theme::SavedShader> =
            serde_json::from_str(super::SHADERS_JSON).unwrap();
        let bundles: Vec<crate::settings::ThemeBundle> =
            serde_json::from_str(super::BUNDLES_JSON).unwrap();
        assert!(!shaders.is_empty() && !bundles.is_empty());
        for item in super::shaders() {
            crate::plugins::effects::shader_expression::function(&item.shader)
                .unwrap_or_else(|error| panic!("{}: {error}", item.shader.name));
        }
        let catalog = crate::theme::ThemeCatalog::load();
        for bundle in &bundles {
            assert!(bundle.panes.len() <= crate::settings::MAX_PANE_LOOKS);
            for id in [
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
                    .filter_map(|pane| pane.theme_id.as_ref()),
            )
            .filter(|id| !id.is_empty())
            {
                assert!(
                    catalog.all().iter().any(|theme| &theme.id == id),
                    "{} references missing theme {id}",
                    bundle.name
                );
            }
        }
        // Some imported bodies predate the current dialect. Validate every
        // new Pane Focus body, including the copy embedded in each theme.
        for (id, json) in crate::theme::BUNDLED_THEME_JSON
            .iter()
            .filter(|(id, _)| id.starts_with("pane-focus-"))
        {
            let theme = crate::theme::parse_bundled_theme(id, json).unwrap();
            if let Some(custom) = theme.effects.shader_background.and_then(|s| s.custom) {
                crate::plugins::effects::shader_expression::program(&custom)
                    .unwrap_or_else(|error| panic!("{id}: {error}"));
            }
        }
    }
}
