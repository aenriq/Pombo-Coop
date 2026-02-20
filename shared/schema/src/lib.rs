use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Pending,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStatus {
    pub run_id: String,
    pub provider: ProviderKind,
    pub state: RunState,
    pub steps_completed: u32,
    pub steps_total: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub kind: ProviderKind,
    pub enabled: bool,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcurrencyLimits {
    pub max_parallel_runs: u16,
    pub max_parallel_steps_per_run: u16,
    pub event_buffer: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartRunCommand {
    pub objective: String,
    pub provider: ProviderKind,
    pub max_steps: u32,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum UiCommand {
    StartRun(StartRunCommand),
    CancelRun { run_id: String },
    RequestRuns,
    RequestConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMessage {
    pub correlation_id: Option<String>,
    pub command: UiCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStateChangedEvent {
    pub run_id: String,
    pub state: RunState,
    pub previous_state: Option<RunState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOutputEvent {
    pub run_id: String,
    pub stream: OutputStream,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshotEvent {
    pub default_provider: ProviderKind,
    pub providers: Vec<ProviderDescriptor>,
    pub concurrency: ConcurrencyLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum OrchestratorEvent {
    RunQueued {
        run_id: String,
        provider: ProviderKind,
    },
    RunStateChanged(RunStateChangedEvent),
    RunOutput(RunOutputEvent),
    RunList {
        runs: Vec<RunStatus>,
    },
    ConfigSnapshot(ConfigSnapshotEvent),
    Error(ErrorEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestratorMessage {
    pub correlation_id: Option<String>,
    pub event: OrchestratorEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_provider_kind_as_expected() {
        let serialized = serde_json::to_string(&ProviderKind::OpenAi).unwrap();
        assert_eq!(serialized, "\"openai\"");
    }

    #[test]
    fn round_trips_ui_message() {
        let message = UiMessage {
            correlation_id: Some("req-1".to_owned()),
            command: UiCommand::StartRun(StartRunCommand {
                objective: "ship epic a".to_owned(),
                provider: ProviderKind::Anthropic,
                max_steps: 25,
                working_directory: Some("/tmp/project".to_owned()),
            }),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        let parsed: UiMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(parsed, message);
    }
}
