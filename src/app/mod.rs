use std::process::Command;

use crate::config::AppConfig;
use crate::provider::{AuthProbe, ProviderDescriptor, ProviderRegistry};
use crate::theme::UiColors;

pub const PANEL_COUNT: usize = 3;
pub const PANEL_RESIZE_STEP: i16 = 4;
pub const PANEL_MIN_WIDTH_PERCENT: i16 = 16;
pub const DEFAULT_PANEL_WIDTHS: [u16; PANEL_COUNT] = [34, 33, 33];

#[cfg(target_os = "macos")]
pub const RESIZE_MODIFIER_LABEL: &str = "Option";
#[cfg(not(target_os = "macos"))]
pub const RESIZE_MODIFIER_LABEL: &str = "Alt";

#[derive(Clone)]
pub struct FileChange {
    pub path: &'static str,
    pub additions: u16,
    pub deletions: u16,
}

#[derive(Clone)]
pub struct Worktree {
    pub repo: &'static str,
    pub name: &'static str,
    pub branch: &'static str,
    pub status: &'static str,
    pub pr_number: u16,
    pub summary: &'static str,
    pub changed_files: Vec<FileChange>,
}

#[derive(Clone, Copy)]
pub enum ChatRole {
    Agent,
    User,
    System,
}

#[derive(Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChatSubpanel {
    Transcript,
    Composer,
}

pub struct App {
    worktrees: Vec<Worktree>,
    chat_messages: Vec<ChatMessage>,
    chat_draft: String,
    selected_idx: usize,
    right_selected_idx: usize,
    chat_scroll: u16,
    chat_subpanel: ChatSubpanel,
    focused_panel: usize,
    panel_widths: [u16; PANEL_COUNT],
    should_quit: bool,
    status_message: String,
    ui_colors: UiColors,
    providers: ProviderRegistry,
    config: AppConfig,
}

impl App {
    pub fn new() -> Self {
        let providers = ProviderRegistry::with_defaults();
        let mut config = AppConfig::load();
        let mut status_message = String::from("Press q to quit.");
        let panel_widths = panel_widths_from_saved_ratios(config.panel_ratios())
            .unwrap_or(DEFAULT_PANEL_WIDTHS);

        let active_provider = config
            .active_provider
            .clone()
            .filter(|provider_id| providers.contains(provider_id))
            .unwrap_or_else(|| providers.default_provider_id().to_owned());
        config.active_provider = Some(active_provider.clone());

        if let Some(descriptor) = providers.descriptor(&active_provider) {
            config.ensure_provider(&descriptor);
        }

        let ui_colors = UiColors::from_config(&config.ui.colors);

        // Keep persisted split state normalized as ratios; this is common for resizable panes.
        config.set_panel_ratios(panel_ratios_from_widths(panel_widths));

        if let Err(error) = config.save() {
            status_message = format!("Could not save config: {error}");
        }

        let mut app = Self {
            worktrees: vec![
                Worktree {
                    repo: "conductor",
                    name: "Planner",
                    branch: "epic-b-shell",
                    status: "In progress",
                    pr_number: 1,
                    summary: "Planning panel state + routing behavior.",
                    changed_files: vec![
                        FileChange {
                            path: "src/shell/planner.rs",
                            additions: 24,
                            deletions: 9,
                        },
                        FileChange {
                            path: "src/shell/events.rs",
                            additions: 6,
                            deletions: 2,
                        },
                    ],
                },
                Worktree {
                    repo: "conductor",
                    name: "Reviewer",
                    branch: "epic-b-diff",
                    status: "Merge conflicts",
                    pr_number: 2,
                    summary: "Diff parsing and conflict summarization changes.",
                    changed_files: vec![
                        FileChange {
                            path: "src/diff/parser.rs",
                            additions: 98,
                            deletions: 12,
                        },
                        FileChange {
                            path: "src/diff/ui.rs",
                            additions: 53,
                            deletions: 2,
                        },
                        FileChange {
                            path: "src/shell/right_panel.rs",
                            additions: 32,
                            deletions: 0,
                        },
                    ],
                },
                Worktree {
                    repo: "melty_home",
                    name: "Regression Sweep",
                    branch: "epic-b-regression",
                    status: "Needs changes",
                    pr_number: 4,
                    summary: "Catches keyboard edge cases in the composer.",
                    changed_files: vec![
                        FileChange {
                            path: "src/shell/textarea.rs",
                            additions: 0,
                            deletions: 73,
                        },
                        FileChange {
                            path: "src/shell/diff_panel.rs",
                            additions: 8,
                            deletions: 4,
                        },
                    ],
                },
            ],
            chat_messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "Loaded diff context for animated-orb.ts",
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: "Can you fix the duplicate shockwaves assignment in this block?",
                },
                ChatMessage {
                    role: ChatRole::Agent,
                    content: "I found the duplicate line in the build() section.\nI am going to patch the repeated `shockwaves` assignment and keep the fallback behavior.",
                },
                ChatMessage {
                    role: ChatRole::Agent,
                    content: "Edit /Users/mrnugget/work/amp/cli/src/tui/widgets/animated-orb.ts",
                },
            ],
            chat_draft: String::new(),
            selected_idx: 0,
            right_selected_idx: 0,
            chat_scroll: 0,
            chat_subpanel: ChatSubpanel::Transcript,
            focused_panel: 0,
            panel_widths,
            should_quit: false,
            status_message,
            ui_colors,
            providers,
            config,
        };

        if app.auth_required() {
            app.refresh_auth_from_local_cli(true);
        }

        app
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn request_quit(&mut self) {
        self.should_quit = true;
    }

    pub fn active_provider_descriptor(&self) -> ProviderDescriptor {
        self.providers
            .descriptor(self.active_provider_id())
            .expect("active provider should always exist")
    }

    pub fn active_model_label(&self) -> &str {
        self.config
            .provider_settings(self.active_provider_id())
            .and_then(|provider| provider.preferred_model.as_deref())
            .unwrap_or(self.active_provider_descriptor().default_model)
    }

    pub fn auth_required(&self) -> bool {
        !self.config.is_authenticated(self.active_provider_id())
    }

    pub fn complete_link_login(&mut self) {
        let provider_id = self.active_provider_id().to_owned();
        self.config.mark_link_completed(&provider_id);
        match self.config.save() {
            Ok(()) => {
                let provider = self.active_provider_descriptor();
                self.status_message =
                    format!("{} login saved. Dashboard unlocked.", provider.display_name);
            }
            Err(error) => {
                self.status_message = format!("Failed to persist login state: {error}");
            }
        }
    }

    pub fn refresh_auth_from_local_cli(&mut self, announce_failures: bool) -> bool {
        let provider_id = self.active_provider_id().to_owned();
        let provider = self.active_provider_descriptor();

        match self.providers.probe_local_auth(&provider_id) {
            AuthProbe::Authenticated { source } => {
                self.config.mark_cli_detected(&provider_id, &source);
                match self.config.save() {
                    Ok(()) => {
                        self.status_message = format!(
                            "Detected existing {} CLI login. Dashboard unlocked.",
                            provider.display_name
                        );
                    }
                    Err(error) => {
                        self.status_message = format!(
                            "{} login detected but config save failed: {error}",
                            provider.display_name
                        );
                    }
                }
                true
            }
            AuthProbe::NotAuthenticated => {
                if announce_failures {
                    self.status_message = format!(
                        "No {} CLI login detected yet. Press O to sign in.",
                        provider.display_name
                    );
                }
                false
            }
            AuthProbe::Unsupported { reason } => {
                if announce_failures {
                    self.status_message = format!(
                        "{} auto-detection unavailable: {}",
                        provider.display_name, reason
                    );
                }
                false
            }
            AuthProbe::Error { reason } => {
                if announce_failures {
                    self.status_message =
                        format!("{} login probe failed: {}", provider.display_name, reason);
                }
                false
            }
        }
    }

    pub fn cycle_provider(&mut self) {
        let descriptors = self.providers.descriptors();
        if descriptors.is_empty() {
            self.status_message = String::from("No providers are registered.");
            return;
        }

        let current_id = self.active_provider_id().to_owned();
        let current_idx = descriptors
            .iter()
            .position(|descriptor| descriptor.id == current_id)
            .unwrap_or(0);
        let next = &descriptors[(current_idx + 1) % descriptors.len()];

        self.config.active_provider = Some(next.id.to_owned());
        self.config.ensure_provider(next);
        match self.config.save() {
            Ok(()) => {
                self.status_message = format!("Switched provider to {}.", next.display_name);
            }
            Err(error) => {
                self.status_message = format!("Provider switch saved in memory only: {error}");
            }
        }

        if self.auth_required() && !self.refresh_auth_from_local_cli(false) {
            self.status_message = format!("Switched provider to {}. Sign-in required.", next.display_name);
        }
    }

    pub fn open_provider_login(&mut self) {
        let provider = self.active_provider_descriptor();
        match open_external_url(provider.login_url) {
            Ok(()) => {
                self.status_message =
                    format!("Opened {} login page in browser.", provider.display_name);
            }
            Err(error) => {
                self.status_message = format!("Could not open browser: {error}");
            }
        }
    }

    pub fn worktrees(&self) -> &[Worktree] {
        &self.worktrees
    }

    pub fn selected_worktree_idx(&self) -> usize {
        self.selected_idx
    }

    pub fn selected_worktree(&self) -> &Worktree {
        &self.worktrees[self.selected_idx]
    }

    pub fn chat_messages(&self) -> &[ChatMessage] {
        &self.chat_messages
    }

    pub fn chat_draft(&self) -> &str {
        &self.chat_draft
    }

    pub fn right_selected_idx(&self) -> usize {
        self.right_selected_idx
    }

    pub fn chat_scroll(&self) -> u16 {
        self.chat_scroll
    }

    pub fn chat_subpanel(&self) -> ChatSubpanel {
        self.chat_subpanel
    }

    pub fn chat_subpanel_name(&self) -> &'static str {
        match self.chat_subpanel {
            ChatSubpanel::Transcript => "Transcript",
            ChatSubpanel::Composer => "Composer",
        }
    }

    pub fn focused_panel(&self) -> usize {
        self.focused_panel
    }

    pub fn panel_widths(&self) -> [u16; PANEL_COUNT] {
        self.panel_widths
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn ui_colors(&self) -> UiColors {
        self.ui_colors
    }

    pub fn focused_panel_name(&self) -> &'static str {
        match self.focused_panel {
            0 => "Worktrees",
            1 => "Chat",
            2 => "Changed Files",
            _ => "Unknown",
        }
    }

    pub fn next(&mut self) {
        if self.worktrees.is_empty() {
            self.selected_idx = 0;
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.worktrees.len();
        self.sync_panel_state_for_selected_worktree();
    }

    pub fn previous(&mut self) {
        if self.worktrees.is_empty() {
            self.selected_idx = 0;
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            self.worktrees.len() - 1
        } else {
            self.selected_idx - 1
        };
        self.sync_panel_state_for_selected_worktree();
    }

    pub fn focus_next_panel(&mut self) {
        self.focused_panel = (self.focused_panel + 1) % PANEL_COUNT;
        self.status_message = format!("Focused panel: {}.", self.focused_panel_name());
    }

    pub fn focus_previous_panel(&mut self) {
        self.focused_panel = if self.focused_panel == 0 {
            PANEL_COUNT - 1
        } else {
            self.focused_panel - 1
        };
        self.status_message = format!("Focused panel: {}.", self.focused_panel_name());
    }

    pub fn focus_subpanel(&mut self, direction: i8) {
        if direction == 0 {
            return;
        }

        if self.focused_panel != 1 {
            self.status_message = format!(
                "Panel '{}' has no subpanels.",
                self.focused_panel_name()
            );
            return;
        }

        self.chat_subpanel = if direction < 0 {
            ChatSubpanel::Transcript
        } else {
            ChatSubpanel::Composer
        };
        self.status_message = format!("Chat subpanel: {}.", self.chat_subpanel_name());
    }

    pub fn move_within_focused_panel(&mut self, direction: i8) {
        if direction == 0 {
            return;
        }

        match self.focused_panel {
            0 => {
                if direction > 0 {
                    self.next();
                } else {
                    self.previous();
                }
            }
            1 => {
                if self.chat_subpanel == ChatSubpanel::Transcript {
                    if direction > 0 {
                        self.chat_scroll = self.chat_scroll.saturating_add(1);
                    } else {
                        self.chat_scroll = self.chat_scroll.saturating_sub(1);
                    }
                } else {
                    self.status_message = String::from(
                        "Composer focused. Text input is not wired yet.",
                    );
                }
            }
            2 => {
                let changed_files_len = self.selected_worktree().changed_files.len();
                if changed_files_len == 0 {
                    self.right_selected_idx = 0;
                    return;
                }
                if direction > 0 {
                    self.right_selected_idx = (self.right_selected_idx + 1) % changed_files_len;
                } else if self.right_selected_idx == 0 {
                    self.right_selected_idx = changed_files_len - 1;
                } else {
                    self.right_selected_idx -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn resize_focused_panel(&mut self, direction: i8) {
        if direction == 0 {
            return;
        }

        let step = PANEL_RESIZE_STEP;
        let active = self.focused_panel;

        let (from_idx, to_idx) = if direction < 0 {
            if active > 0 {
                // Move boundary left: grow focused panel by taking width from left neighbor.
                (active - 1, active)
            } else if active + 1 < PANEL_COUNT {
                // Left edge: moving left shrinks focused panel into the middle panel.
                (active, active + 1)
            } else {
                return;
            }
        } else if active + 1 < PANEL_COUNT {
            // Move boundary right: grow focused panel by taking width from right neighbor.
            (active + 1, active)
        } else if active > 0 {
            // Right edge: moving right shrinks focused panel into the middle panel.
            (active, active - 1)
        } else {
            return;
        };

        let from_width = self.panel_widths[from_idx] as i16;
        let transferable = (from_width - PANEL_MIN_WIDTH_PERCENT).max(0);
        let adjusted = step.min(transferable);

        if adjusted == 0 {
            self.status_message = format!("Panel '{}' reached minimum width.", self.focused_panel_name());
            return;
        }

        self.panel_widths[from_idx] = (self.panel_widths[from_idx] as i16 - adjusted) as u16;
        self.panel_widths[to_idx] = (self.panel_widths[to_idx] as i16 + adjusted) as u16;

        self.status_message = format!(
            "{} resized | L:{}% M:{}% R:{}%",
            self.focused_panel_name(),
            self.panel_widths[0],
            self.panel_widths[1],
            self.panel_widths[2]
        );

        self.persist_panel_ratios();
    }

    fn active_provider_id(&self) -> &str {
        self.config
            .active_provider
            .as_deref()
            .filter(|provider_id| self.providers.contains(provider_id))
            .unwrap_or(self.providers.default_provider_id())
    }

    fn persist_panel_ratios(&mut self) {
        self.config
            .set_panel_ratios(panel_ratios_from_widths(self.panel_widths));
        if let Err(error) = self.config.save() {
            self.status_message = format!("Layout resized but failed to save: {error}");
        }
    }

    fn sync_panel_state_for_selected_worktree(&mut self) {
        self.chat_scroll = 0;
        self.chat_subpanel = ChatSubpanel::Transcript;
        let changed_files_len = self.selected_worktree().changed_files.len();
        if changed_files_len == 0 {
            self.right_selected_idx = 0;
            return;
        }
        if self.right_selected_idx >= changed_files_len {
            self.right_selected_idx = changed_files_len - 1;
        }
    }
}

fn open_external_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return Command::new("open").arg(url).spawn().map(|_| ());
    }

    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return Command::new("xdg-open").arg(url).spawn().map(|_| ());
    }

    #[allow(unreachable_code)]
    Err(std::io::Error::other(
        "opening URLs is not supported on this platform",
    ))
}

fn panel_ratios_from_widths(widths: [u16; PANEL_COUNT]) -> [f32; PANEL_COUNT] {
    let total = widths.iter().copied().map(f32::from).sum::<f32>();
    if total <= 0.0 {
        return [0.34, 0.33, 0.33];
    }
    [
        widths[0] as f32 / total,
        widths[1] as f32 / total,
        widths[2] as f32 / total,
    ]
}

fn panel_widths_from_saved_ratios(ratios: Option<[f32; PANEL_COUNT]>) -> Option<[u16; PANEL_COUNT]> {
    let ratios = ratios?;
    if ratios.iter().any(|ratio| !ratio.is_finite() || *ratio <= 0.0) {
        return None;
    }

    let sum = ratios.iter().sum::<f32>();
    if sum <= 0.0 {
        return None;
    }

    let normalized = [ratios[0] / sum, ratios[1] / sum, ratios[2] / sum];
    let left = (normalized[0] * 100.0).round() as i16;
    let middle = (normalized[1] * 100.0).round() as i16;
    let right = 100 - left - middle;
    let widths = [left, middle, right];

    if widths.iter().any(|width| *width < PANEL_MIN_WIDTH_PERCENT) {
        return None;
    }

    Some([
        widths[0] as u16,
        widths[1] as u16,
        widths[2] as u16,
    ])
}
