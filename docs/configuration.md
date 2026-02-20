# Local Configuration (EPIC A-005)

This document defines the config format used by `agent_manager_core::config`.

## Resolution order
`resolve_config_path()` checks locations in this order:
1. `AGENT_MANAGER_CONFIG` if set
2. `.agent-manager/config.toml` in the current workspace if it exists
3. `~/.config/agent-manager/config.toml` if it exists
4. fallback write target: `.agent-manager/config.toml` in the current workspace

## Example config
```toml
default_provider = "openai"

[providers.openai]
enabled = true
model = "gpt-4.1"
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[providers.anthropic]
enabled = false
model = "claude-3-7-sonnet"
api_key_env = "ANTHROPIC_API_KEY"

[providers.local]
enabled = true
model = "qwen2.5-coder:14b"
api_base = "http://localhost:11434/v1"

[paths]
workspace_root = "."
runs_dir = ".agent-manager/runs"
logs_dir = ".agent-manager/logs"

[concurrency]
max_parallel_runs = 3
max_parallel_steps_per_run = 4
event_buffer = 512
```

## Field notes
- `default_provider`: one of `openai`, `anthropic`, `local`; must point to an enabled provider.
- `providers.*.enabled`: toggles provider availability.
- `providers.*.model`: default model for that provider.
- `providers.*.api_base`: optional custom endpoint.
- `providers.*.api_key_env`: optional environment variable name storing credentials.
- `paths.workspace_root`: base workspace path.
- `paths.runs_dir`: runtime state directory.
- `paths.logs_dir`: runtime logs directory.
- `concurrency.max_parallel_runs`: number of concurrent agent runs.
- `concurrency.max_parallel_steps_per_run`: per-run parallel step cap.
- `concurrency.event_buffer`: in-memory event buffer capacity.

## Defaults
Defaults are created automatically by `OrchestratorConfig::load()` / `load_or_default()` if no config exists.
