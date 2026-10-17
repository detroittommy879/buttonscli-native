//! Provider metadata only. API keys never belong in this serializable model.

use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProviderProfile {
    pub id: String,
    pub name: String,
    #[serde(alias = "base_url")]
    pub endpoint: String,
    #[serde(alias = "default_model")]
    pub model: String,
    pub credential_ref: Option<String>,
    #[serde(alias = "format")]
    pub api: ProviderApi,
    pub models_endpoint: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderApi {
    #[default]
    Auto,
    ChatCompletions,
    Responses,
    Anthropic,
}

impl ProviderProfile {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn uses_responses(&self) -> bool {
        self.resolved_api() == ProviderApi::Responses
    }

    pub(crate) fn resolved_api(&self) -> ProviderApi {
        if self.api != ProviderApi::Auto {
            return self.api;
        }
        let Ok(url) = url::Url::parse(&self.endpoint) else {
            return ProviderApi::ChatCompletions;
        };
        let path = url.path().trim_end_matches('/');
        if path.ends_with("/chat/completions") {
            ProviderApi::ChatCompletions
        } else if path.ends_with("/messages") || url.host_str() == Some("api.anthropic.com") {
            ProviderApi::Anthropic
        } else if path.ends_with("/responses") || url.host_str() == Some("api.openai.com") {
            ProviderApi::Responses
        } else {
            ProviderApi::ChatCompletions
        }
    }

    pub(crate) fn base_url(&self) -> Option<String> {
        let mut url = url::Url::parse(&self.endpoint).ok()?;
        let path = url.path().trim_end_matches('/');
        let base = [
            "/chat/completions",
            "/responses",
            "/messages",
            "/models",
            "/completions",
        ]
        .iter()
        .find_map(|suffix| path.strip_suffix(suffix))
        .unwrap_or(path)
        .to_owned();
        url.set_path(if base.is_empty() { "/v1" } else { &base });
        Some(url.to_string().trim_end_matches('/').into())
    }
}

impl Default for ProviderProfile {
    fn default() -> Self {
        Self {
            id: "default-openrouter".into(),
            name: "OpenRouter".into(),
            endpoint: "https://openrouter.ai/api/v1".into(),
            model: "openai/gpt-4.1-mini".into(),
            credential_ref: None,
            api: ProviderApi::Auto,
            models_endpoint: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProviderSettings {
    pub providers: Vec<ProviderProfile>,
    pub active_provider_id: String,
    pub tasks: BTreeMap<AiTask, TaskBinding>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AiTask {
    Help,
    Theme,
    Shader,
    /// Reviews and repairs generated shaders; works best as a different model.
    ShaderJudge,
    Simulation,
}

impl AiTask {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const ALL: [Self; 5] = [
        Self::Help,
        Self::Theme,
        Self::Shader,
        Self::ShaderJudge,
        Self::Simulation,
    ];

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Help => "AI Help model",
            Self::Theme => "Theme generation model",
            Self::Shader => "Shader generation model",
            Self::ShaderJudge => "Shader judge model (checks and fixes; pick a different model)",
            Self::Simulation => "Simulated terminal model",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ModelTarget {
    pub provider_id: String,
    /// Empty uses this provider's configured model.
    pub model: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TaskBinding {
    /// None inherits the shared default dynamically, without copying credentials.
    pub primary: Option<ModelTarget>,
    pub fallbacks: Vec<ModelTarget>,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            providers: vec![ProviderProfile::default()],
            active_provider_id: "default-openrouter".into(),
            tasks: BTreeMap::new(),
        }
    }
}

impl ProviderSettings {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn configuration_json(&self) -> String {
        let mut value = serde_json::to_value(self).unwrap();
        for provider in value["providers"].as_array_mut().unwrap() {
            let object = provider.as_object_mut().unwrap();
            object.remove("credential_ref");
            for (old, new) in [
                ("endpoint", "base_url"),
                ("api", "format"),
                ("model", "default_model"),
            ] {
                if let Some(value) = object.remove(old) {
                    object.insert(new.into(), value);
                }
            }
        }
        serde_json::to_string_pretty(&value).unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn apply_configuration_json(&self, raw: &str) -> Result<Self, String> {
        if raw.len() > 128 * 1024 {
            return Err("Provider JSON exceeds 128 KiB.".into());
        }
        let value: Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
        if value.as_object().is_none_or(|object| {
            object
                .keys()
                .any(|key| !["providers", "active_provider_id", "tasks"].contains(&key.as_str()))
        }) {
            return Err(
                "Use providers, active_provider_id and tasks only; API keys stay outside JSON."
                    .into(),
            );
        }
        let entries = value["providers"]
            .as_array()
            .ok_or("Add a providers array.")?;
        if entries.len() > 32 {
            return Err("At most 32 provider connections are supported.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for entry in entries {
            let object = entry
                .as_object()
                .ok_or("Each provider must be an object.")?;
            if object.keys().any(|key| {
                ![
                    "id",
                    "name",
                    "base_url",
                    "endpoint",
                    "format",
                    "api",
                    "default_model",
                    "model",
                    "models_endpoint",
                ]
                .contains(&key.as_str())
            }) {
                return Err("Unknown provider field. Keep API keys in the saved-key controls, outside JSON.".into());
            }
            let id = entry["id"]
                .as_str()
                .ok_or("Every provider needs a stable id.")?;
            if id.is_empty() || id.len() > 80 || id.chars().any(char::is_control) || !ids.insert(id)
            {
                return Err(
                    "Provider IDs must be unique, nonempty and at most 80 characters.".into(),
                );
            }
        }
        let mut settings: Self =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        if !settings.active_provider_id.is_empty()
            && !settings
                .providers
                .iter()
                .any(|provider| provider.id == settings.active_provider_id)
        {
            return Err("The main provider ID must refer to a configured provider.".into());
        }
        for binding in settings.tasks.values() {
            if binding.fallbacks.len() > 3
                || binding
                    .primary
                    .iter()
                    .chain(&binding.fallbacks)
                    .any(|target| {
                        !settings
                            .providers
                            .iter()
                            .any(|provider| provider.id == target.provider_id)
                    })
            {
                return Err(
                    "Task models must use configured providers and at most three fallbacks.".into(),
                );
            }
        }
        for provider in &mut settings.providers {
            if !provider.endpoint.is_empty() {
                validate_endpoint(&provider.endpoint).map_err(str::to_owned)?;
            }
            if let Some(endpoint) = &provider.models_endpoint {
                validate_endpoint(endpoint).map_err(str::to_owned)?;
            }
            provider.credential_ref = self
                .providers
                .iter()
                .find(|old| old.id == provider.id)
                .and_then(|old| old.credential_ref.clone());
        }
        let mut warnings = vec![];
        settings.normalize(&mut warnings);
        Ok(settings)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_legacy(assistant: &Value) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let source = assistant
            .get("namedProviders")
            .and_then(Value::as_array)
            .map(|entries| (entries, false))
            .or_else(|| {
                assistant
                    .get("savedProfiles")
                    .and_then(Value::as_array)
                    .map(|entries| (entries, true))
            });
        let mut providers = Vec::new();
        if let Some((entries, deprecated)) = source {
            for (index, entry) in entries.iter().enumerate() {
                let id = entry["id"]
                    .as_str()
                    .filter(|id| !id.trim().is_empty() && id.len() <= 80)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("imported-{}", index + 1));
                let name = entry[if deprecated { "label" } else { "name" }]
                    .as_str()
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("Provider {}", index + 1));
                let Some(endpoint) = entry["endpoint"].as_str().and_then(sanitize_endpoint) else {
                    warnings.push(format!(
                        "provider {} has no usable HTTP endpoint",
                        index + 1
                    ));
                    continue;
                };
                let model = entry["model"].as_str().unwrap_or("").to_owned();
                providers.push(ProviderProfile {
                    id,
                    name,
                    endpoint,
                    model,
                    credential_ref: None,
                    api: ProviderApi::Auto,
                    models_endpoint: None,
                });
            }
        } else if let Some(endpoint) = assistant["endpoint"].as_str().and_then(sanitize_endpoint) {
            providers.push(ProviderProfile {
                id: "imported-default".into(),
                name: assistant["provider"]
                    .as_str()
                    .unwrap_or("Imported provider")
                    .into(),
                endpoint,
                model: assistant["model"].as_str().unwrap_or("").into(),
                credential_ref: None,
                api: ProviderApi::Auto,
                models_endpoint: None,
            });
        }
        let mut settings = Self {
            providers,
            tasks: BTreeMap::new(),
            active_provider_id: assistant["activeProviderId"]
                .as_str()
                .or_else(|| assistant["activeProfileId"].as_str())
                .unwrap_or("")
                .to_owned(),
        };
        settings.normalize(&mut warnings);
        (settings, warnings)
    }

    pub(crate) fn normalize(&mut self, warnings: &mut Vec<String>) {
        let mut seen = std::collections::HashSet::new();
        for (index, provider) in self.providers.iter_mut().enumerate() {
            if provider.id.trim().is_empty()
                || provider.id.len() > 80
                || !seen.insert(provider.id.clone())
            {
                let mut candidate = format!("provider-{}", index + 1);
                while !seen.insert(candidate.clone()) {
                    candidate.push('x');
                }
                provider.id = candidate;
                warnings.push(format!(
                    "provider {} received a unique native ID",
                    index + 1
                ));
            }
            // Native configuration never trusts a credential ref from imported JSON.
            if provider.credential_ref.as_ref().is_some_and(|reference| {
                reference.strip_prefix("native:v1:").is_none_or(|hex| {
                    hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
            }) {
                provider.credential_ref = None;
            }
            if let Some(safe) = sanitize_endpoint(&provider.endpoint) {
                if safe != provider.endpoint {
                    provider.endpoint = safe;
                    warnings.push(format!(
                        "provider {} endpoint credentials or parameters were removed",
                        index + 1
                    ));
                }
                let protocol = provider.resolved_api();
                if let Some(base) = provider.base_url() {
                    if base != provider.endpoint {
                        // Preserve the API formerly selected by a legacy full URL.
                        provider.api = protocol;
                        provider.endpoint = base;
                    }
                }
            } else {
                provider.endpoint.clear();
                warnings.push(format!(
                    "provider {} has no usable HTTP endpoint",
                    index + 1
                ));
            }
            provider.models_endpoint = provider
                .models_endpoint
                .as_deref()
                .and_then(sanitize_endpoint);
        }
        if !self
            .providers
            .iter()
            .any(|provider| provider.id == self.active_provider_id)
        {
            self.active_provider_id = self
                .providers
                .first()
                .map(|provider| provider.id.clone())
                .unwrap_or_default();
        }
        for binding in self.tasks.values_mut() {
            if binding.primary.as_ref().is_some_and(|target| {
                !self
                    .providers
                    .iter()
                    .any(|provider| provider.id == target.provider_id)
            }) {
                binding.primary = None;
                warnings.push("Missing task provider reset to shared default".into());
            }
            binding.fallbacks.retain(|target| {
                self.providers
                    .iter()
                    .any(|provider| provider.id == target.provider_id)
            });
            binding.fallbacks.truncate(3);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn for_task(&self, task: AiTask) -> Option<ProviderProfile> {
        match self
            .tasks
            .get(&task)
            .and_then(|binding| binding.primary.as_ref())
        {
            Some(target) => self.resolve(target),
            None => self
                .providers
                .iter()
                .find(|p| p.id == self.active_provider_id)
                .cloned(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn chain(&self, task: AiTask) -> Vec<ProviderProfile> {
        let mut chain: Vec<_> = self.for_task(task).into_iter().collect();
        if let Some(binding) = self.tasks.get(&task) {
            for target in binding.fallbacks.iter().take(3) {
                if let Some(provider) = self.resolve(target) {
                    if !chain
                        .iter()
                        .any(|p| p.id == provider.id && p.model == provider.model)
                    {
                        chain.push(provider);
                    }
                }
            }
        }
        chain
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resolve(&self, target: &ModelTarget) -> Option<ProviderProfile> {
        let mut provider = self
            .providers
            .iter()
            .find(|p| p.id == target.provider_id)?
            .clone();
        if !target.model.trim().is_empty() {
            provider.model = target.model.trim().into();
        }
        Some(provider)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn active(&self) -> Option<&ProviderProfile> {
        self.providers
            .iter()
            .find(|provider| provider.id == self.active_provider_id)
    }
}

pub(crate) fn sanitize_endpoint(raw: &str) -> Option<String> {
    let mut endpoint = url::Url::parse(raw).ok()?;
    if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
        return None;
    }
    endpoint.set_username("").ok()?;
    endpoint.set_password(None).ok()?;
    endpoint.set_query(None);
    endpoint.set_fragment(None);
    Some(endpoint.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn validate_endpoint(raw: &str) -> Result<(), &'static str> {
    let endpoint =
        url::Url::parse(raw).map_err(|_| "Enter a complete HTTP or HTTPS endpoint URL")?;
    if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
        return Err("Use an HTTP or HTTPS endpoint URL");
    }
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err("Keep credentials, query strings, and fragments out of endpoint URLs");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_inherit_live_default_and_task_overrides_keep_credentials() {
        let mut settings: ProviderSettings = serde_json::from_value(serde_json::json!({
            "providers": [{"id":"one","model":"first"}], "active_provider_id":"one"
        }))
        .unwrap();
        assert_eq!(settings.for_task(AiTask::Theme).unwrap().model, "first");
        settings.providers[0].model = "new-default".into();
        assert!(AiTask::ALL
            .iter()
            .all(|task| settings.for_task(*task).unwrap().model == "new-default"));
        settings.tasks.insert(
            AiTask::Help,
            TaskBinding {
                primary: Some(ModelTarget {
                    provider_id: "one".into(),
                    model: "help-only".into(),
                }),
                fallbacks: vec![ModelTarget {
                    provider_id: "one".into(),
                    model: "new-default".into(),
                }],
            },
        );
        assert_eq!(
            settings
                .chain(AiTask::Help)
                .iter()
                .map(|p| p.model.as_str())
                .collect::<Vec<_>>(),
            ["help-only", "new-default"]
        );
        assert_eq!(
            settings.for_task(AiTask::Simulation).unwrap().model,
            "new-default"
        );
        let round_trip: ProviderSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(round_trip, settings);
        settings.providers.clear();
        settings.normalize(&mut Vec::new());
        assert!(settings.tasks[&AiTask::Help].primary.is_none());
        assert!(settings.chain(AiTask::Help).is_empty());
    }

    #[test]
    fn legacy_named_and_deprecated_providers_exclude_keys() {
        let named = serde_json::json!({"activeProviderId":"p2","namedProviders":[
            {"id":"p1","name":"Local","endpoint":"http://localhost:11434/v1/chat/completions","model":"m1","apiKey":"FAKE-KEY"},
            {"id":"p2","name":"Remote","endpoint":"https://user:FAKE-KEY@example.test/v1?token=FAKE-KEY","model":"m2","apiKey":"FAKE-KEY"}
        ]});
        let (settings, warnings) = ProviderSettings::from_legacy(&named);
        assert!(warnings.is_empty());
        assert_eq!(settings.active().unwrap().id, "p2");
        assert_eq!(settings.providers[1].endpoint, "https://example.test/v1");
        assert!(!serde_json::to_string(&settings)
            .unwrap()
            .contains("FAKE-KEY"));
        let old = serde_json::json!({"savedProfiles":[{"id":"old","label":"Old","endpoint":"https://example.test/v1","model":"legacy","apiKey":"FAKE-KEY"}]});
        let (settings, _) = ProviderSettings::from_legacy(&old);
        assert_eq!(settings.active().unwrap().name, "Old");
    }

    #[test]
    fn explicit_empty_list_and_invalid_endpoint_do_not_seed_defaults() {
        let (empty, _) = ProviderSettings::from_legacy(&serde_json::json!({"namedProviders":[]}));
        assert!(empty.providers.is_empty());
        let (bad, warnings) = ProviderSettings::from_legacy(
            &serde_json::json!({"namedProviders":[{"endpoint":"file:///etc/passwd"}]}),
        );
        assert!(bad.providers.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(validate_endpoint("https://user:pass@example.test/v1").is_err());
        assert!(validate_endpoint("http://127.0.0.1:11434/v1/chat/completions").is_ok());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod configuration_tests {
    use super::*;
    #[test]
    fn full_url_migration_and_json_edit_keep_keys_routes_and_custom_prefixes() {
        let mut settings = ProviderSettings::default();
        settings.providers[0].endpoint = "https://api.openai.com/v1/chat/completions".into();
        settings.providers[0].credential_ref = Some(format!("native:v1:{}", "a".repeat(64)));
        let id = settings.active_provider_id.clone();
        settings.tasks.insert(
            AiTask::Simulation,
            TaskBinding {
                primary: Some(ModelTarget {
                    provider_id: id,
                    model: "custom-model".into(),
                }),
                ..Default::default()
            },
        );
        settings.normalize(&mut vec![]);
        assert_eq!(settings.providers[0].endpoint, "https://api.openai.com/v1");
        assert_eq!(settings.providers[0].api, ProviderApi::ChatCompletions);
        let raw = settings.configuration_json();
        assert!(!raw.contains("credential_ref"));
        let updated = settings.apply_configuration_json(&raw).unwrap();
        assert_eq!(updated, settings);
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value["providers"][0]["api_key"] = serde_json::json!("never-store-this");
        assert!(settings
            .apply_configuration_json(&value.to_string())
            .is_err());
        value["providers"][0]
            .as_object_mut()
            .unwrap()
            .remove("api_key");
        value["active_provider_id"] = serde_json::json!("missing");
        assert!(settings
            .apply_configuration_json(&value.to_string())
            .is_err());
        let provider = ProviderProfile {
            endpoint: "https://example.test/proxy/v7/".into(),
            api: ProviderApi::Anthropic,
            ..Default::default()
        };
        assert_eq!(
            provider.base_url().unwrap(),
            "https://example.test/proxy/v7"
        );
        assert_eq!(
            super::super::client::provider_url(&provider).unwrap(),
            "https://example.test/proxy/v7/messages"
        );
    }
}
