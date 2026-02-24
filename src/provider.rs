use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AuthStrategy {
    Link,
    ApiKey,
}

#[derive(Debug, Clone)]
pub struct ProviderDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub login_url: &'static str,
    pub default_model: &'static str,
    pub auth_strategy: AuthStrategy,
}

#[derive(Debug, Clone)]
pub enum AuthProbe {
    Authenticated { source: String },
    NotAuthenticated,
    Unsupported { reason: String },
    Error { reason: String },
}

pub trait Provider: Send + Sync {
    fn descriptor(&self) -> ProviderDescriptor;

    fn probe_local_auth(&self) -> AuthProbe {
        AuthProbe::Unsupported {
            reason: String::from("provider does not expose local auth state"),
        }
    }
}

#[derive(Debug, Default)]
pub struct CodexProvider;

impl Provider for CodexProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: "codex",
            display_name: "Codex",
            login_url: "https://chatgpt.com/codex",
            default_model: "gpt-5-codex",
            auth_strategy: AuthStrategy::Link,
        }
    }

    fn probe_local_auth(&self) -> AuthProbe {
        let output = match Command::new("codex").args(["login", "status"]).output() {
            Ok(output) => output,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::NotFound {
                    return AuthProbe::Unsupported {
                        reason: String::from("codex CLI not found in PATH"),
                    };
                }
                return AuthProbe::Error {
                    reason: format!("failed to run codex login status: {error}"),
                };
            }
        };

        if output.status.success() {
            return AuthProbe::Authenticated {
                source: String::from("codex login status"),
            };
        }

        AuthProbe::NotAuthenticated
    }
}

pub struct ProviderRegistry {
    providers: BTreeMap<String, Box<dyn Provider>>,
    default_provider_id: String,
}

impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self {
            providers: BTreeMap::new(),
            default_provider_id: String::new(),
        };
        registry.register(CodexProvider);
        registry
    }

    pub fn register<P>(&mut self, provider: P)
    where
        P: Provider + 'static,
    {
        let descriptor = provider.descriptor();
        if self.default_provider_id.is_empty() {
            self.default_provider_id = descriptor.id.to_owned();
        }
        self.providers
            .insert(descriptor.id.to_owned(), Box::new(provider));
    }

    pub fn contains(&self, id: &str) -> bool {
        self.providers.contains_key(id)
    }

    pub fn default_provider_id(&self) -> &str {
        &self.default_provider_id
    }

    pub fn descriptor(&self, id: &str) -> Option<ProviderDescriptor> {
        self.providers.get(id).map(|provider| provider.descriptor())
    }

    pub fn descriptors(&self) -> Vec<ProviderDescriptor> {
        self.providers
            .values()
            .map(|provider| provider.descriptor())
            .collect()
    }

    pub fn probe_local_auth(&self, id: &str) -> AuthProbe {
        match self.providers.get(id) {
            Some(provider) => provider.probe_local_auth(),
            None => AuthProbe::Unsupported {
                reason: format!("provider '{id}' is not registered"),
            },
        }
    }
}
