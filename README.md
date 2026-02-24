# AgentManager (Ratatui TUI)

This branch resets the project into a clean terminal UI baseline using [Ratatui](https://ratatui.rs/).

## Run

```bash
cargo run
```

## Controls

- First launch (provider onboarding):
  - automatic check: if Codex CLI is already logged in, onboarding is skipped
  - `o`: open provider login link
  - `Enter`: confirm login completion
  - `r`: refresh local CLI login detection
  - `p` / `Tab`: cycle provider (future provider support)
  - `q`: quit
- `Ctrl` + `h/j/k/l` or `Ctrl` + arrow keys: switch focused panel
- `Option` + `h/j/k/l` or `Option` + arrow keys: resize focused panel (macOS)
- `j/k` or `Up/Down`: move within the focused panel
- In `Worktrees` panel, movement selects previous/next worktree
- `q`: quit

## Notes

- The first run prompts for Codex link-based sign-in before unlocking the dashboard.
- Provider settings persist to `~/.config/agent_manager_tui/config.toml` by default (or `$XDG_CONFIG_HOME`).
- Config schema already includes placeholders for API key, custom base URL, and preferred model for future providers.

## Source Layout

- `src/main.rs`: terminal bootstrap + app loop
- `src/app/`: state, provider/auth actions, panel focus/resize behavior
- `src/input/`: keybindings and interaction routing
- `src/ui/`: rendering entrypoint
- `src/ui/components/`: isolated render components (`onboarding`, `panels`, `status_bar`)
- `src/config.rs` and `src/provider.rs`: persisted config + provider registry/probing
