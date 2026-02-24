use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::provider::{AuthStrategy, ProviderDescriptor};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub active_provider: Option<String>,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub auth: ProviderAuth,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env_var: Option<String>,
    #[serde(default)]
    pub preferred_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderAuth {
    #[default]
    NotAuthenticated,
    LinkCompleted { completed_unix_seconds: u64 },
    CliDetected {
        cli: String,
        detected_unix_seconds: u64,
    },
    ApiKeyConfigured { env_var: String },
}

impl ProviderAuth {
    pub fn is_authenticated(&self) -> bool {
        !matches!(self, Self::NotAuthenticated)
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_path();
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)
            .map_err(|error| io::Error::other(format!("serialize config: {error}")))?;
        fs::write(path, raw)
    }

    pub fn provider_settings(&self, provider_id: &str) -> Option<&ProviderConfig> {
        self.providers.get(provider_id)
    }

    pub fn ensure_provider(&mut self, descriptor: &ProviderDescriptor) -> &mut ProviderConfig {
        self.providers
            .entry(descriptor.id.to_owned())
            .or_insert_with(|| ProviderConfig {
                auth: ProviderAuth::NotAuthenticated,
                base_url: None,
                api_key_env_var: if matches!(descriptor.auth_strategy, AuthStrategy::ApiKey) {
                    Some("OPENAI_API_KEY".to_owned())
                } else {
                    None
                },
                preferred_model: Some(descriptor.default_model.to_owned()),
            })
    }

    pub fn is_authenticated(&self, provider_id: &str) -> bool {
        self.provider_settings(provider_id)
            .is_some_and(|provider| provider.auth.is_authenticated())
    }

    pub fn mark_link_completed(&mut self, provider_id: &str) {
        let completed_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let provider = self
            .providers
            .entry(provider_id.to_owned())
            .or_default();
        provider.auth = ProviderAuth::LinkCompleted {
            completed_unix_seconds,
        };
    }

    pub fn mark_cli_detected(&mut self, provider_id: &str, cli: &str) {
        let detected_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let provider = self
            .providers
            .entry(provider_id.to_owned())
            .or_default();
        provider.auth = ProviderAuth::CliDetected {
            cli: cli.to_owned(),
            detected_unix_seconds,
        };
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg)
            .join("agent_manager_tui")
            .join("config.toml");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("agent_manager_tui")
            .join("config.toml");
    }

    PathBuf::from(".agent-manager").join("config.toml")
}
