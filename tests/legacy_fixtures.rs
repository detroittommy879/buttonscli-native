use serde_json::Value;

const CONFIG: &str = include_str!("fixtures/legacy/profiles/Work_Space/config.json");
const ROOT_CONFIG: &str = include_str!("fixtures/legacy/config.json");

#[test]
fn synthetic_profile_preserves_original_edge_cases() {
    let config: Value = serde_json::from_str(CONFIG).unwrap();
    let presets = config["presets"].as_array().unwrap();
    assert_eq!(presets.len(), 2);
    assert_eq!(presets[0]["command"], "  echo café  ");
    assert_eq!(presets[0]["sendEnter"], false);
    assert!(presets[1].get("sendEnter").is_none());
    assert!(presets[1]["command"].as_str().unwrap().ends_with('\n'));
    assert!(config["sshPresets"].as_array().unwrap().is_empty());
    assert_eq!(
        config["assistant"]["namedProviders"][0]["id"],
        "fixture-local"
    );
    assert!(config["theme"]["typography"]["unknownFutureZone"].is_object());
    assert!(config["effects"]["futureVisualField"].is_object());
    assert!(
        serde_json::from_str::<Value>(ROOT_CONFIG).unwrap()["presets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn theme_collision_and_malformed_metadata_are_explicit() {
    let first: Value = serde_json::from_str(include_str!(
        "fixtures/legacy/profiles/Work_Space/themes/duplicate-a.json"
    ))
    .unwrap();
    let second: Value = serde_json::from_str(include_str!(
        "fixtures/legacy/profiles/Work_Space/themes/duplicate-b.json"
    ))
    .unwrap();
    assert_eq!(first["metadata"]["id"], second["metadata"]["id"]);
    assert_ne!(first["theme"]["terminal"], second["theme"]["terminal"]);
    assert!(serde_json::from_str::<Value>(include_str!(
        "fixtures/legacy/malformed/active-profile.json"
    ))
    .is_err());
}
