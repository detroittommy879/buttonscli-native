//! Provider metadata only. API keys never belong in this serializable model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub credential_ref: Option<String>,
}

impl Default for ProviderProfile {
    fn default() -> Self {
        Self {
            id: "default-openrouter".into(),
            name: "OpenRouter".into(),
            endpoint: "https://openrouter.ai/api/v1/chat/completions".into(),
            model: "openai/gpt-4.1-mini".into(),
            credential_ref: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProviderSettings {
    pub providers: Vec<ProviderProfile>,
    pub active_provider_id: String,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            providers: vec![ProviderProfile::default()],
            active_provider_id: "default-openrouter".into(),
        }
    }
}

impl ProviderSettings {
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
            });
        }
        let mut settings = Self {
            providers,
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
    }

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
