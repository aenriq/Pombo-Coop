# AgentManager

Desktop orchestrator for parallel coding agents with in-app review and publish workflows.

## Development commands
Run from the repository root:

```bash
cargo run -p agent_manager_ui
./ui/dev.sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace --all-targets
```

## Project layout
- `ui/gpui_app/`: current desktop UI bootstrap app.
- `core/orchestrator/`: orchestrator runtime + config system.
- `shared/schema/`: typed command/event contracts shared by UI and core.
- `docs/`: architecture, coding standards, and configuration docs.
- `.github/workflows/`: CI workflows.

## Documentation
- `docs/architecture.md`: A-001 shell decision and rationale.
- `docs/coding-standards.md`: linting, formatting, testing, and build conventions.
- `docs/configuration.md`: local provider/path/concurrency config contract.
