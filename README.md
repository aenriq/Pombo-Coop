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
- `Ctrl` + `Left/Right` or `Ctrl` + `h/l`: switch focused panel
- `Ctrl` + `Up/Down` or `Ctrl` + `j/k`: switch subpanel (in `Chat`: transcript/composer)
- `Option` + `h/j/k/l` or `Option` + arrow keys: resize focused panel (macOS)
- `j/k` or `Up/Down`: move within the focused panel
- In `Worktrees` panel, movement selects previous/next worktree
- In `Chat` panel, movement scrolls when transcript is focused
- Middle panel contains two subpanels: chat transcript (top) and dashed composer (bottom)

## Notes

- The first run prompts for Codex link-based sign-in before unlocking the dashboard.
- Provider settings persist to `~/.config/agent_manager_tui/config.toml` by default (or `$XDG_CONFIG_HOME`).
- Config schema already includes placeholders for API key, custom base URL, and preferred model for future providers.
- Pane layout persists as normalized split ratios (`ui.panel_ratios`), so widths restore across restarts.
- Colors use an openapi-tui-inspired default palette and can be overridden in config.

### Color Overrides

Add an optional `[ui.colors]` section to `~/.config/agent_manager_tui/config.toml`:

```toml
[ui.colors]
border_focused = "teal"
border_default = "white"
model_title = "yellow"
panel_background = "black"
panel_foreground = "white"
added = "light_green"
removed = "light_red"
status_text = "dark_gray"
```

Supported formats:
- named colors: `black`, `white`, `light_green`, `light_cyan`, etc.
- indexed colors: `indexed:10`
- hex RGB: `#88ccff`

## Source Layout

- `src/main.rs`: terminal bootstrap + app loop
- `src/app/`: state, provider/auth actions, panel focus/resize behavior
- `src/input/`: keybindings and interaction routing
- `src/ui/`: rendering entrypoint
- `src/ui/components/`: isolated render components (`onboarding`, `panels`, `status_bar`)
- `src/config.rs` and `src/provider.rs`: persisted config + provider registry/probing
