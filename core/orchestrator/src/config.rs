use agent_manager_shared::ProviderKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OrchestratorConfig {
    pub default_provider: ProviderKind,
    pub providers: ProviderConfigs,
    pub paths: PathConfig,
    pub concurrency: ConcurrencyConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderKind::OpenAi,
            providers: ProviderConfigs::default(),
            paths: PathConfig::default(),
            concurrency: ConcurrencyConfig::default(),
        }
    }
}

impl OrchestratorConfig {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_or_default(resolve_config_path())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let config: Self = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            let config = Self::default();
            config.save_to_path(path)?;
            return Ok(config);
        }

        Self::load_from_path(path)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        self.validate()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let raw = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        fs::write(path, raw).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.paths.validate()?;
        self.concurrency.validate()?;
        self.providers.validate()?;

        let selected_provider = self.providers.get(self.default_provider);
        if !selected_provider.enabled {
            return Err(ConfigError::Validation(
                "default provider must be enabled".to_owned(),
            ));
        }

        Ok(())
    }
}

pub fn resolve_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("AGENT_MANAGER_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let workspace_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".agent-manager/config.toml");

    if workspace_path.exists() {
        return workspace_path;
    }

    let user_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("agent-manager/config.toml");

    if user_path.exists() {
        return user_path;
    }

    workspace_path
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfigs {
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub local: ProviderConfig,
}

impl Default for ProviderConfigs {
    fn default() -> Self {
        Self {
            openai: ProviderConfig::enabled("gpt-4.1"),
            anthropic: ProviderConfig::disabled("claude-3-7-sonnet"),
            local: ProviderConfig::disabled("qwen2.5-coder:14b"),
        }
    }
}

impl ProviderConfigs {
    pub fn get(&self, kind: ProviderKind) -> &ProviderConfig {
        match kind {
            ProviderKind::OpenAi => &self.openai,
            ProviderKind::Anthropic => &self.anthropic,
            ProviderKind::Local => &self.local,
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.openai
            .validate("openai")
            .and_then(|_| self.anthropic.validate("anthropic"))
            .and_then(|_| self.local.validate("local"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub model: String,
    pub api_base: Option<String>,
    pub api_key_env: Option<String>,
}

impl ProviderConfig {
    fn enabled(model: &str) -> Self {
        Self {
            enabled: true,
            model: model.to_owned(),
            api_base: None,
            api_key_env: None,
        }
    }

    fn disabled(model: &str) -> Self {
        Self {
            enabled: false,
            model: model.to_owned(),
            api_base: None,
            api_key_env: None,
        }
    }

    fn validate(&self, provider_name: &str) -> Result<(), ConfigError> {
        if self.enabled && self.model.trim().is_empty() {
            return Err(ConfigError::Validation(format!(
                "provider '{provider_name}' is enabled but model is empty"
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathConfig {
    pub workspace_root: PathBuf,
    pub runs_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            runs_dir: PathBuf::from(".agent-manager/runs"),
            logs_dir: PathBuf::from(".agent-manager/logs"),
        }
    }
}

impl PathConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.workspace_root.as_os_str().is_empty() {
            return Err(ConfigError::Validation(
                "paths.workspace_root cannot be empty".to_owned(),
            ));
        }

        if self.runs_dir.as_os_str().is_empty() {
            return Err(ConfigError::Validation(
                "paths.runs_dir cannot be empty".to_owned(),
            ));
        }

        if self.logs_dir.as_os_str().is_empty() {
            return Err(ConfigError::Validation(
                "paths.logs_dir cannot be empty".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConcurrencyConfig {
    pub max_parallel_runs: u16,
    pub max_parallel_steps_per_run: u16,
    pub event_buffer: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_parallel_runs: 2,
            max_parallel_steps_per_run: 4,
            event_buffer: 512,
        }
    }
}

impl ConcurrencyConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.max_parallel_runs == 0 {
            return Err(ConfigError::Validation(
                "concurrency.max_parallel_runs must be greater than zero".to_owned(),
            ));
        }

        if self.max_parallel_steps_per_run == 0 {
            return Err(ConfigError::Validation(
                "concurrency.max_parallel_steps_per_run must be greater than zero".to_owned(),
            ));
        }

        if self.event_buffer == 0 {
            return Err(ConfigError::Validation(
                "concurrency.event_buffer must be greater than zero".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize config: {0}")]
    Serialize(#[source] toml::ser::Error),
    #[error("failed to create parent directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config at {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_to_disk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("orchestrator.toml");

        let mut config = OrchestratorConfig::default();
        config.concurrency.max_parallel_runs = 3;
        config.providers.local.enabled = true;

        config.save_to_path(&path).unwrap();
        let parsed = OrchestratorConfig::load_from_path(&path).unwrap();

        assert_eq!(parsed, config);
    }

    #[test]
    fn load_or_default_creates_missing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("nested/orchestrator.toml");

        let config = OrchestratorConfig::load_or_default(&path).unwrap();

        assert_eq!(config, OrchestratorConfig::default());
        assert!(path.exists());
    }

    #[test]
    fn validate_rejects_invalid_concurrency() {
        let mut config = OrchestratorConfig::default();
        config.concurrency.max_parallel_runs = 0;

        let err = config.validate().unwrap_err();

        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn validate_rejects_disabled_default_provider() {
        let config = OrchestratorConfig {
            default_provider: ProviderKind::Local,
            ..OrchestratorConfig::default()
        };

        let err = config.validate().unwrap_err();

        assert!(matches!(err, ConfigError::Validation(_)));
    }
}
