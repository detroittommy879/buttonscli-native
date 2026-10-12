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
    pub endpoint: String,
    pub model: String,
    pub credential_ref: Option<String>,
    pub api: ProviderApi,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderApi {
    #[default]
    Auto,
    ChatCompletions,
    Responses,
}

impl ProviderProfile {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn uses_responses(&self) -> bool {
        match self.api {
            ProviderApi::Responses => true,
            ProviderApi::ChatCompletions => false,
            ProviderApi::Auto => url::Url::parse(&self.endpoint)
                .is_ok_and(|url| url.path().trim_end_matches('/').ends_with("/responses")),
        }
    }
}

impl Default for ProviderProfile {
    fn default() -> Self {
        Self {
            id: "default-openrouter".into(),
            name: "OpenRouter".into(),
            endpoint: "https://openrouter.ai/api/v1/chat/completions".into(),
            model: "openai/gpt-4.1-mini".into(),
            credential_ref: None,
            api: ProviderApi::Auto,
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
    Simulation,
}

impl AiTask {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const ALL: [Self; 4] = [Self::Help, Self::Theme, Self::Shader, Self::Simulation];

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Help => "AI Help model",
            Self::Theme => "Theme generation model",
            Self::Shader => "Shader generation model",
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
            } else {
                provider.endpoint.clear();
                warnings.push(format!(
                    "provider {} has no usable HTTP endpoint",
                    index + 1
                ));
            }
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
