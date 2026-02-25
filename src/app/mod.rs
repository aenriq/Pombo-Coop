use std::{collections::BTreeSet, process::Command};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{AppConfig, PanelFocusExpandMode};
use crate::provider::{AuthProbe, ProviderDescriptor, ProviderRegistry};
use crate::theme::UiColors;

pub const PANEL_COUNT: usize = 3;
pub const PANEL_RESIZE_STEP: i16 = 4;
pub const PANEL_MIN_WIDTH_PERCENT: i16 = 16;
pub const DEFAULT_PANEL_WIDTHS: [u16; PANEL_COUNT] = [34, 33, 33];
pub const PANEL_EXPANDED_FOCUS_WIDTHS: [u16; PANEL_COUNT] = [68, 16, 16];

#[derive(Clone)]
pub struct FileChange {
    pub path: &'static str,
    pub additions: u16,
    pub deletions: u16,
    pub kind: FileChangeKind,
    pub staged: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
    Ignored,
}

impl FileChangeKind {
    pub fn code(self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::TypeChanged => 'T',
            Self::Unmerged => '!',
            Self::Untracked => 'U',
            Self::Ignored => 'I',
        }
    }
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
    pub content: String,
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
    chat_cursor: usize,
    chat_preferred_column: Option<usize>,
    right_search_active: bool,
    right_search_query: String,
    selected_idx: usize,
    right_selected_idx: usize,
    right_multi_selected: BTreeSet<usize>,
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
        let panel_widths =
            panel_widths_from_saved_ratios(config.panel_ratios()).unwrap_or(DEFAULT_PANEL_WIDTHS);

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
                            kind: FileChangeKind::Modified,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/events.rs",
                            additions: 6,
                            deletions: 2,
                            kind: FileChangeKind::Untracked,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/new_pane.rs",
                            additions: 31,
                            deletions: 0,
                            kind: FileChangeKind::Added,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/legacy_layout.rs",
                            additions: 0,
                            deletions: 64,
                            kind: FileChangeKind::Deleted,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/tree_row.rs -> src/shell/worktree_row.rs",
                            additions: 14,
                            deletions: 12,
                            kind: FileChangeKind::Renamed,
                            staged: true,
                        },
                        FileChange {
                            path: "src/shell/worktree_row_copy.rs",
                            additions: 18,
                            deletions: 0,
                            kind: FileChangeKind::Copied,
                            staged: false,
                        },
                        FileChange {
                            path: "assets/icon.svg",
                            additions: 5,
                            deletions: 3,
                            kind: FileChangeKind::TypeChanged,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/merge_state.rs",
                            additions: 42,
                            deletions: 16,
                            kind: FileChangeKind::Unmerged,
                            staged: false,
                        },
                        FileChange {
                            path: ".cache/build-index.json",
                            additions: 0,
                            deletions: 0,
                            kind: FileChangeKind::Ignored,
                            staged: false,
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
                            kind: FileChangeKind::Unmerged,
                            staged: false,
                        },
                        FileChange {
                            path: "src/diff/ui.rs",
                            additions: 53,
                            deletions: 2,
                            kind: FileChangeKind::Renamed,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/right_panel.rs",
                            additions: 32,
                            deletions: 0,
                            kind: FileChangeKind::Added,
                            staged: true,
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
                            kind: FileChangeKind::Deleted,
                            staged: false,
                        },
                        FileChange {
                            path: "src/shell/diff_panel.rs",
                            additions: 8,
                            deletions: 4,
                            kind: FileChangeKind::Modified,
                            staged: false,
                        },
                    ],
                },
            ],
            chat_messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "Loaded diff context for animated-orb.ts".to_owned(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content:
                        "Can you fix the duplicate shockwaves assignment in this block?"
                            .to_owned(),
                },
                ChatMessage {
                    role: ChatRole::Agent,
                    content: "I found the duplicate line in the build() section.\nI am going to patch the repeated `shockwaves` assignment and keep the fallback behavior.".to_owned(),
                },
                ChatMessage {
                    role: ChatRole::Agent,
                    content:
                        "Edit /Users/mrnugget/work/amp/cli/src/tui/widgets/animated-orb.ts"
                            .to_owned(),
                },
            ],
            chat_draft: String::new(),
            chat_cursor: 0,
            chat_preferred_column: None,
            right_search_active: false,
            right_search_query: String::new(),
            selected_idx: 0,
            right_selected_idx: 0,
            right_multi_selected: BTreeSet::new(),
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
            self.status_message = format!(
                "Switched provider to {}. Sign-in required.",
                next.display_name
            );
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

    pub fn right_search_active(&self) -> bool {
        self.right_search_active
    }

    pub fn right_search_has_query(&self) -> bool {
        !self.right_search_query.trim().is_empty()
    }

    pub fn right_search_query(&self) -> &str {
        &self.right_search_query
    }

    pub fn open_right_search(&mut self) {
        self.focus_panel_by_index(2);
        self.right_search_active = true;
        self.status_message =
            String::from("Search changed files by path (supports globs like *.rs, **/src/**).");
        self.ensure_right_selection_valid();
    }

    pub fn handle_right_search_key(&mut self, key: KeyEvent) -> bool {
        if !self.right_search_active || self.focused_panel != 2 {
            return false;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
            || key.modifiers.contains(KeyModifiers::SUPER)
        {
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                self.right_search_active = false;
                if self.right_search_has_query() {
                    self.status_message =
                        String::from("Search closed. Filter is still applied (press C to clear).");
                } else {
                    self.status_message = String::from("Search closed.");
                }
                self.ensure_right_selection_valid();
                true
            }
            KeyCode::Enter => {
                self.right_search_active = false;
                self.status_message = String::from("Changed files search applied.");
                self.ensure_right_selection_valid();
                true
            }
            KeyCode::Backspace => {
                self.right_search_query.pop();
                self.ensure_right_selection_valid();
                true
            }
            KeyCode::Char(ch) => {
                self.right_search_query.push(ch);
                self.ensure_right_selection_valid();
                true
            }
            _ => false,
        }
    }

    pub fn clear_right_search(&mut self) {
        self.right_search_query.clear();
        self.right_search_active = false;
        self.status_message = String::from("Changed files search cleared.");
        self.ensure_right_selection_valid();
    }

    pub fn select_right_file(&mut self, file_idx: usize) -> bool {
        if file_idx >= self.selected_worktree().changed_files.len() {
            return false;
        }

        if self.right_selected_idx == file_idx {
            return false;
        }

        self.right_selected_idx = file_idx;
        true
    }

    pub fn right_multi_selected(&self) -> &BTreeSet<usize> {
        &self.right_multi_selected
    }

    pub fn toggle_right_multi_selected(&mut self) {
        if self.focused_panel != 2 {
            self.status_message = String::from("Focus Changed Files to select files.");
            return;
        }

        let visible = self.right_panel_display_order();
        if visible.is_empty() {
            self.status_message = String::from("No files to select.");
            return;
        }
        if !visible.contains(&self.right_selected_idx) {
            self.right_selected_idx = visible[0];
        }

        if !self.right_multi_selected.insert(self.right_selected_idx) {
            self.right_multi_selected.remove(&self.right_selected_idx);
        }

        let selected_count = self.right_multi_selected.len();
        if selected_count == 0 {
            self.status_message = String::from("Selection cleared.");
        } else if selected_count == 1 {
            self.status_message = String::from("1 file selected.");
        } else {
            self.status_message = format!("{selected_count} files selected.");
        }
    }

    pub fn clear_right_multi_selected(&mut self) {
        self.right_multi_selected.clear();
        self.status_message = String::from("Selection cleared.");
    }

    pub fn changed_file_index_for_list_row(&self, row: usize) -> Option<usize> {
        let (unstaged, staged) = self.changed_file_sections();
        let mut current_row = 0usize;

        if self.right_search_has_query() {
            current_row += 1; // Query summary row
        }

        current_row += 1; // Unstaged header
        for idx in &unstaged {
            if row == current_row {
                return Some(*idx);
            }
            current_row += 1;
        }
        if unstaged.is_empty() {
            current_row += 1; // "(none)" row
        }

        current_row += 1; // Staged header
        for idx in &staged {
            if row == current_row {
                return Some(*idx);
            }
            current_row += 1;
        }

        None
    }

    pub fn toggle_selected_changed_file_staging(&mut self) {
        if self.focused_panel != 2 {
            self.status_message = String::from("Focus Changed Files to update staged files.");
            return;
        }

        let pre_order = self.right_panel_display_order();
        if pre_order.is_empty() {
            self.status_message = String::from("No files match current filter.");
            return;
        }
        if !pre_order.contains(&self.right_selected_idx) {
            self.right_selected_idx = pre_order[0];
        }
        let anchor_position = pre_order
            .iter()
            .position(|idx| *idx == self.right_selected_idx)
            .unwrap_or(0);
        let target_indices = self.right_action_target_indices(&pre_order);

        let selected_idx = self.selected_idx;
        let mut moved = BTreeSet::new();
        let mut staged_count = 0usize;
        let mut unstaged_count = 0usize;
        let mut only_path = String::new();
        for file_idx in target_indices {
            let Some(file) = self
                .worktrees
                .get_mut(selected_idx)
                .and_then(|worktree| worktree.changed_files.get_mut(file_idx))
            else {
                continue;
            };

            file.staged = !file.staged;
            if file.staged {
                staged_count += 1;
            } else {
                unstaged_count += 1;
            }

            moved.insert(file_idx);
            if moved.len() == 1 {
                only_path = file.path.to_string();
            }
        }

        if moved.is_empty() {
            self.status_message = String::from("No files selected.");
            return;
        }

        if moved.len() == 1 {
            if staged_count == 1 {
                self.status_message = format!("Staged '{}'.", only_path);
            } else {
                self.status_message = format!("Unstaged '{}'.", only_path);
            }
        } else {
            self.status_message = format!(
                "Updated {} files ({} staged, {} unstaged).",
                moved.len(),
                staged_count,
                unstaged_count
            );
        }

        self.advance_right_selection_after_action(&pre_order, anchor_position, &moved);
        self.right_multi_selected.clear();
        self.ensure_right_selection_valid();
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

    pub fn composer_is_focused(&self) -> bool {
        self.focused_panel == 1 && self.chat_subpanel == ChatSubpanel::Composer
    }

    pub fn chat_cursor_line_column(&self) -> (usize, usize) {
        self.cursor_line_column()
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

    pub fn effective_panel_widths(&self, terminal_width: u16) -> [u16; PANEL_COUNT] {
        if !self.should_expand_focused_panel(terminal_width) {
            return self.panel_widths;
        }

        let mut widths = [PANEL_EXPANDED_FOCUS_WIDTHS[1]; PANEL_COUNT];
        widths[self.focused_panel] = PANEL_EXPANDED_FOCUS_WIDTHS[0];
        widths
    }

    pub fn panel_focus_expand_mode_summary(&self, terminal_width: u16) -> String {
        let config = &self.config.ui.panel_focus_expand;
        let enabled = self.should_expand_focused_panel(terminal_width);
        match config.mode {
            PanelFocusExpandMode::Off => String::from("off"),
            PanelFocusExpandMode::On => String::from("on"),
            PanelFocusExpandMode::Auto => {
                let state = if enabled { "on" } else { "off" };
                format!("auto<= {} ({state})", config.breakpoint_cols)
            }
        }
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

    pub fn focus_panel_by_index(&mut self, panel: usize) {
        if panel >= PANEL_COUNT || self.focused_panel == panel {
            return;
        }

        self.focused_panel = panel;
        self.status_message = format!("Focused panel: {}.", self.focused_panel_name());
    }

    pub fn focus_subpanel(&mut self, direction: i8) {
        if direction == 0 {
            return;
        }

        if self.focused_panel != 1 {
            self.status_message =
                format!("Panel '{}' has no subpanels.", self.focused_panel_name());
            return;
        }

        self.chat_subpanel = if direction < 0 {
            ChatSubpanel::Transcript
        } else {
            ChatSubpanel::Composer
        };
        self.status_message = format!("Chat subpanel: {}.", self.chat_subpanel_name());
    }

    pub fn handle_composer_key(&mut self, key: KeyEvent) -> bool {
        if !self.composer_is_focused() {
            return false;
        }

        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_super = key.modifiers.contains(KeyModifiers::SUPER);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Char(c) if has_ctrl || has_super => match c.to_ascii_lowercase() {
                'a' => {
                    self.chat_cursor = 0;
                    self.chat_preferred_column = None;
                    true
                }
                'e' => {
                    self.chat_cursor = self.chat_draft.chars().count();
                    self.chat_preferred_column = None;
                    true
                }
                _ => false,
            },
            KeyCode::Char(c) => {
                if has_alt {
                    return false;
                }
                self.insert_char_at_cursor(c);
                true
            }
            KeyCode::Enter => {
                if has_ctrl || has_super {
                    self.submit_composer_message();
                } else {
                    self.insert_char_at_cursor('\n');
                }
                true
            }
            KeyCode::Backspace => {
                self.delete_char_backward();
                true
            }
            KeyCode::Delete => {
                self.delete_char_forward();
                true
            }
            KeyCode::Left => {
                if has_ctrl || has_super {
                    self.chat_cursor = 0;
                } else {
                    self.chat_cursor = self.chat_cursor.saturating_sub(1);
                }
                self.chat_preferred_column = None;
                true
            }
            KeyCode::Right => {
                let len = self.chat_draft.chars().count();
                if has_ctrl || has_super {
                    self.chat_cursor = len;
                } else {
                    self.chat_cursor = (self.chat_cursor + 1).min(len);
                }
                self.chat_preferred_column = None;
                true
            }
            KeyCode::Home => {
                let (start, _) = self.current_line_bounds();
                self.chat_cursor = start;
                self.chat_preferred_column = None;
                true
            }
            KeyCode::End => {
                let (_, end) = self.current_line_bounds();
                self.chat_cursor = end;
                self.chat_preferred_column = None;
                true
            }
            KeyCode::Up => {
                self.move_cursor_vertical(-1);
                true
            }
            KeyCode::Down => {
                self.move_cursor_vertical(1);
                true
            }
            KeyCode::Tab => {
                self.insert_char_at_cursor(' ');
                self.insert_char_at_cursor(' ');
                true
            }
            _ => false,
        }
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
                    self.move_cursor_vertical(direction);
                }
            }
            2 => {
                let display_order = self.right_panel_display_order();
                if display_order.is_empty() {
                    self.right_selected_idx = 0;
                    return;
                }
                let current = display_order
                    .iter()
                    .position(|idx| *idx == self.right_selected_idx)
                    .unwrap_or(0);
                let next = if direction > 0 {
                    (current + 1) % display_order.len()
                } else if current == 0 {
                    display_order.len() - 1
                } else {
                    current - 1
                };
                self.right_selected_idx = display_order[next];
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
            self.status_message = format!(
                "Panel '{}' reached minimum width.",
                self.focused_panel_name()
            );
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

    fn should_expand_focused_panel(&self, terminal_width: u16) -> bool {
        let config = &self.config.ui.panel_focus_expand;
        match config.mode {
            PanelFocusExpandMode::Off => false,
            PanelFocusExpandMode::On => true,
            PanelFocusExpandMode::Auto => terminal_width <= config.breakpoint_cols,
        }
    }

    fn submit_composer_message(&mut self) {
        let message = self.chat_draft.trim().to_owned();
        if message.is_empty() {
            self.status_message = String::from("Type a message before sending.");
            return;
        }

        self.chat_messages.push(ChatMessage {
            role: ChatRole::User,
            content: message,
        });
        self.chat_draft.clear();
        self.chat_cursor = 0;
        self.chat_preferred_column = None;
        self.chat_scroll = u16::MAX;
        self.status_message = String::from("Message added to transcript.");
    }

    fn insert_char_at_cursor(&mut self, ch: char) {
        let len = self.chat_draft.chars().count();
        self.chat_cursor = self.chat_cursor.min(len);
        let byte_index = byte_index_for_char(&self.chat_draft, self.chat_cursor);
        self.chat_draft.insert(byte_index, ch);
        self.chat_cursor += 1;
        self.chat_preferred_column = None;
    }

    fn delete_char_backward(&mut self) {
        if self.chat_cursor == 0 {
            return;
        }

        let start = byte_index_for_char(&self.chat_draft, self.chat_cursor - 1);
        let end = byte_index_for_char(&self.chat_draft, self.chat_cursor);
        self.chat_draft.replace_range(start..end, "");
        self.chat_cursor -= 1;
        self.chat_preferred_column = None;
    }

    fn delete_char_forward(&mut self) {
        let len = self.chat_draft.chars().count();
        if self.chat_cursor >= len {
            return;
        }

        let start = byte_index_for_char(&self.chat_draft, self.chat_cursor);
        let end = byte_index_for_char(&self.chat_draft, self.chat_cursor + 1);
        self.chat_draft.replace_range(start..end, "");
        self.chat_preferred_column = None;
    }

    fn move_cursor_vertical(&mut self, direction: i8) {
        if direction == 0 {
            return;
        }

        let chars = self.chat_draft.chars().collect::<Vec<_>>();
        let len = chars.len();
        self.chat_cursor = self.chat_cursor.min(len);
        let (line_start, line_end) = self.current_line_bounds();
        let current_column = self.chat_cursor.saturating_sub(line_start);
        let preferred = self.chat_preferred_column.unwrap_or(current_column);

        if direction < 0 {
            if line_start == 0 {
                self.chat_preferred_column = Some(preferred);
                return;
            }

            let prev_line_end = line_start - 1;
            let prev_line_start = chars[..prev_line_end]
                .iter()
                .rposition(|ch| *ch == '\n')
                .map_or(0, |idx| idx + 1);
            let prev_line_len = prev_line_end.saturating_sub(prev_line_start);
            self.chat_cursor = prev_line_start + preferred.min(prev_line_len);
        } else {
            if line_end >= len {
                self.chat_preferred_column = Some(preferred);
                return;
            }

            let next_line_start = line_end + 1;
            let next_line_end = chars[next_line_start..]
                .iter()
                .position(|ch| *ch == '\n')
                .map_or(len, |idx| next_line_start + idx);
            let next_line_len = next_line_end.saturating_sub(next_line_start);
            self.chat_cursor = next_line_start + preferred.min(next_line_len);
        }

        self.chat_preferred_column = Some(preferred);
    }

    fn current_line_bounds(&self) -> (usize, usize) {
        let chars = self.chat_draft.chars().collect::<Vec<_>>();
        let len = chars.len();
        let cursor = self.chat_cursor.min(len);

        let line_start = chars[..cursor]
            .iter()
            .rposition(|ch| *ch == '\n')
            .map_or(0, |idx| idx + 1);
        let line_end = chars[cursor..]
            .iter()
            .position(|ch| *ch == '\n')
            .map_or(len, |idx| cursor + idx);

        (line_start, line_end)
    }

    fn cursor_line_column(&self) -> (usize, usize) {
        let cursor = self.chat_cursor.min(self.chat_draft.chars().count());
        let mut line = 0;
        let mut column = 0;

        for (idx, ch) in self.chat_draft.chars().enumerate() {
            if idx >= cursor {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        (line, column)
    }

    fn persist_panel_ratios(&mut self) {
        self.config
            .set_panel_ratios(panel_ratios_from_widths(self.panel_widths));
        if let Err(error) = self.config.save() {
            self.status_message = format!("Layout resized but failed to save: {error}");
        }
    }

    fn ensure_right_selection_valid(&mut self) {
        let display_order = self.right_panel_display_order();
        if display_order.is_empty() {
            self.right_selected_idx = 0;
            self.right_multi_selected.clear();
            return;
        }

        self.right_multi_selected
            .retain(|idx| display_order.contains(idx));

        if !display_order.contains(&self.right_selected_idx) {
            self.right_selected_idx = display_order[0];
        }
    }

    fn right_panel_display_order(&self) -> Vec<usize> {
        let (mut unstaged, staged) = self.changed_file_sections();
        let mut order = Vec::with_capacity(unstaged.len() + staged.len());
        order.append(&mut unstaged);
        order.extend(staged);
        order
    }

    pub fn changed_file_sections(&self) -> (Vec<usize>, Vec<usize>) {
        let query = self.right_search_query.trim().to_ascii_lowercase();
        let mut unstaged = Vec::new();
        let mut staged = Vec::new();
        for (idx, change) in self.selected_worktree().changed_files.iter().enumerate() {
            if !changed_file_matches_query(change.path, &query) {
                continue;
            }
            if change.staged {
                staged.push(idx);
            } else {
                unstaged.push(idx);
            }
        }
        (unstaged, staged)
    }

    fn sync_panel_state_for_selected_worktree(&mut self) {
        self.chat_scroll = 0;
        self.chat_subpanel = ChatSubpanel::Transcript;
        self.right_multi_selected.clear();
        self.ensure_right_selection_valid();
    }

    fn right_action_target_indices(&self, pre_order: &[usize]) -> Vec<usize> {
        if self.right_multi_selected.is_empty() {
            return vec![self.right_selected_idx];
        }

        pre_order
            .iter()
            .copied()
            .filter(|idx| self.right_multi_selected.contains(idx))
            .collect()
    }

    fn advance_right_selection_after_action(
        &mut self,
        pre_order: &[usize],
        anchor_position: usize,
        moved: &BTreeSet<usize>,
    ) {
        for idx in pre_order.iter().skip(anchor_position + 1) {
            if !moved.contains(idx) {
                self.right_selected_idx = *idx;
                return;
            }
        }

        for idx in pre_order[..anchor_position].iter().rev() {
            if !moved.contains(idx) {
                self.right_selected_idx = *idx;
                return;
            }
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

fn panel_widths_from_saved_ratios(
    ratios: Option<[f32; PANEL_COUNT]>,
) -> Option<[u16; PANEL_COUNT]> {
    let ratios = ratios?;
    if ratios
        .iter()
        .any(|ratio| !ratio.is_finite() || *ratio <= 0.0)
    {
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

    Some([widths[0] as u16, widths[1] as u16, widths[2] as u16])
}

fn byte_index_for_char(text: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(byte_idx, _)| byte_idx)
}

fn changed_file_matches_query(path: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let normalized_path = path.replace('\\', "/").to_ascii_lowercase();
    if !query_contains_glob_wildcards(query) {
        return normalized_path.contains(query);
    }

    if glob_pattern_match(query, &normalized_path) {
        return true;
    }

    if !query.contains('/') {
        if let Some(file_name) = normalized_path.rsplit('/').next() {
            if glob_pattern_match(query, file_name) {
                return true;
            }
        }

        let prefixed = format!("**/{query}");
        return glob_pattern_match(&prefixed, &normalized_path);
    }

    false
}

fn query_contains_glob_wildcards(query: &str) -> bool {
    query.contains('*') || query.contains('?')
}

fn glob_pattern_match(pattern: &str, text: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let text_chars = text.chars().collect::<Vec<_>>();
    let mut memo = vec![None; (pattern_chars.len() + 1) * (text_chars.len() + 1)];
    glob_pattern_match_recursive(&pattern_chars, &text_chars, 0, 0, &mut memo)
}

fn glob_pattern_match_recursive(
    pattern: &[char],
    text: &[char],
    pattern_idx: usize,
    text_idx: usize,
    memo: &mut [Option<bool>],
) -> bool {
    let memo_width = text.len() + 1;
    let memo_idx = pattern_idx * memo_width + text_idx;
    if let Some(cached) = memo[memo_idx] {
        return cached;
    }

    let matched = if pattern_idx == pattern.len() {
        text_idx == text.len()
    } else {
        match pattern[pattern_idx] {
            '*' => {
                let double_star =
                    pattern_idx + 1 < pattern.len() && pattern[pattern_idx + 1] == '*';
                if double_star {
                    let mut next_pattern = pattern_idx + 2;
                    while next_pattern < pattern.len() && pattern[next_pattern] == '*' {
                        next_pattern += 1;
                    }

                    let mut cursor = text_idx;
                    let mut found = false;
                    while cursor <= text.len() {
                        if glob_pattern_match_recursive(pattern, text, next_pattern, cursor, memo) {
                            found = true;
                            break;
                        }
                        cursor += 1;
                    }
                    found
                } else {
                    let mut cursor = text_idx;
                    let mut found = false;
                    loop {
                        if glob_pattern_match_recursive(
                            pattern,
                            text,
                            pattern_idx + 1,
                            cursor,
                            memo,
                        ) {
                            found = true;
                            break;
                        }
                        if cursor >= text.len() || text[cursor] == '/' {
                            break;
                        }
                        cursor += 1;
                    }
                    found
                }
            }
            '?' => {
                text_idx < text.len()
                    && text[text_idx] != '/'
                    && glob_pattern_match_recursive(
                        pattern,
                        text,
                        pattern_idx + 1,
                        text_idx + 1,
                        memo,
                    )
            }
            literal => {
                text_idx < text.len()
                    && literal == text[text_idx]
                    && glob_pattern_match_recursive(
                        pattern,
                        text,
                        pattern_idx + 1,
                        text_idx + 1,
                        memo,
                    )
            }
        }
    };

    memo[memo_idx] = Some(matched);
    matched
}
