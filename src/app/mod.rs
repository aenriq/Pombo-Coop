use std::{
    cell::Cell,
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use crate::config::{AppConfig, PanelFocusExpandMode};
use crate::provider::{AuthProbe, ProviderDescriptor, ProviderRegistry};
use crate::theme::UiColors;

pub const PANEL_COUNT: usize = 3;
pub const PANEL_RESIZE_STEP: i16 = 4;
pub const PANEL_MIN_WIDTH_PERCENT: i16 = 16;
pub const DEFAULT_PANEL_WIDTHS: [u16; PANEL_COUNT] = [34, 33, 33];
pub const PANEL_EXPANDED_FOCUS_WIDTHS: [u16; PANEL_COUNT] = [68, 16, 16];
const CHAT_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECTION_TEST_TIMEOUT: Duration = Duration::from_secs(45);
const MODEL_LIST_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MODEL_LIST_REQUEST_LIMIT: u32 = 100;
const THINKING_LIVE_INTERVAL_MS: u64 = 120;
const AGENT_STREAM_MIN_CHARS_PER_TICK: usize = 8;
const THINKING_WAVE_WIDTH: usize = 8;
const THINKING_WAVE_START_HOLD_TICKS: usize = 2;
const THINKING_WAVE_LEVEL_GLYPHS: [char; 4] = ['⢀', '⠠', '⠐', '⠈'];
const THINKING_WAVE_KERNEL: [u8; 8] = [0, 1, 2, 3, 3, 2, 1, 0];
const CODEX_MODEL_CATALOG: [(&str, &str); 5] = [
    ("gpt-5.3-codex", "Latest frontier agentic coding model."),
    ("gpt-5.2-codex", "Frontier agentic coding model."),
    (
        "gpt-5.1-codex-max",
        "Codex-optimized flagship for deep and fast reasoning.",
    ),
    ("gpt-5.2", "Latest frontier model with broad improvements."),
    (
        "gpt-5.1-codex-mini",
        "Optimized for codex: faster and cheaper.",
    ),
];

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

#[derive(Clone)]
pub struct ModelChoice {
    pub id: String,
    pub description: String,
    pub is_current: bool,
}

type ModelListFetchResult = Result<Vec<ModelChoice>, String>;

struct StreamingAgentMessage {
    message_index: usize,
    full_message: String,
    revealed_chars: usize,
    total_chars: usize,
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
    chat_scroll_max_cache: Cell<u16>,
    should_quit: bool,
    status_message: String,
    ui_colors: UiColors,
    providers: ProviderRegistry,
    config: AppConfig,
    chat_response_rx: Option<Receiver<ChatResponse>>,
    chat_request_in_flight: bool,
    streaming_agent_message: Option<StreamingAgentMessage>,
    thinking_wave_step: usize,
    model_picker_active: bool,
    model_picker_options: Vec<ModelChoice>,
    model_picker_selected: usize,
    model_list_rx: Option<Receiver<ModelListFetchResult>>,
    model_list_in_flight: bool,
    connection_test_rx: Option<Receiver<ConnectionTestResult>>,
    connection_test_in_flight: bool,
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
            chat_messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "Agent ready. Ask about the selected worktree.".to_owned(),
            }],
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
            chat_scroll_max_cache: Cell::new(0),
            should_quit: false,
            status_message,
            ui_colors,
            providers,
            config,
            chat_response_rx: None,
            chat_request_in_flight: false,
            streaming_agent_message: None,
            thinking_wave_step: 0,
            model_picker_active: false,
            model_picker_options: Vec::new(),
            model_picker_selected: 0,
            model_list_rx: None,
            model_list_in_flight: false,
            connection_test_rx: None,
            connection_test_in_flight: false,
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

    pub fn model_picker_active(&self) -> bool {
        self.model_picker_active
    }

    pub fn model_picker_options(&self) -> &[ModelChoice] {
        &self.model_picker_options
    }

    pub fn model_picker_selected(&self) -> usize {
        self.model_picker_selected
    }

    pub fn handle_model_picker_key(&mut self, key: KeyEvent) -> bool {
        if !self.model_picker_active {
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                self.model_picker_active = false;
                self.status_message = String::from("Model selection canceled.");
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_model_picker_selection(-1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_model_picker_selection(1);
                true
            }
            KeyCode::Enter => {
                self.apply_selected_model_choice();
                true
            }
            _ => true,
        }
    }

    pub fn thinking_animation_playing(&self) -> bool {
        self.chat_request_in_flight || self.streaming_agent_message.is_some()
    }

    pub fn thinking_tick_interval(&self) -> Duration {
        Duration::from_millis(THINKING_LIVE_INTERVAL_MS)
    }

    pub fn advance_thinking_wave(&mut self) -> bool {
        let mut changed = false;

        if self.chat_request_in_flight {
            let cycle_len = thinking_wave_cycle_len();
            if cycle_len != 0 {
                self.thinking_wave_step = (self.thinking_wave_step + 1) % cycle_len;
                changed = true;
            }
        }

        if self.advance_streaming_agent_message() {
            changed = true;
        }

        changed
    }

    pub fn thinking_wave(&self) -> Option<String> {
        if !self.chat_request_in_flight {
            return None;
        }

        Some(render_thinking_wave(self.thinking_wave_step))
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

    pub fn update_chat_scroll_max(&self, max_scroll: u16) {
        self.chat_scroll_max_cache.set(max_scroll);
    }

    pub fn scroll_chat_transcript(&mut self, direction: i8) {
        let max_scroll = self.chat_scroll_max_cache.get();
        let mut scroll = if self.chat_scroll == u16::MAX {
            max_scroll
        } else {
            self.chat_scroll.min(max_scroll)
        };

        if direction > 0 {
            scroll = (scroll + 1).min(max_scroll);
        } else if direction < 0 {
            scroll = scroll.saturating_sub(1);
        }

        self.chat_scroll = scroll;
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

    pub fn finalize_chat_scroll_anchor(&mut self) -> bool {
        if self.chat_scroll == u16::MAX
            && !self.chat_request_in_flight
            && self.streaming_agent_message.is_none()
        {
            self.chat_scroll = self.chat_scroll_max_cache.get();
            return true;
        }
        false
    }

    pub fn poll_background_updates(&mut self) -> bool {
        let mut changed = false;

        if let Some(rx) = self.chat_response_rx.take() {
            match rx.try_recv() {
                Ok(response) => {
                    self.chat_request_in_flight = false;
                    self.thinking_wave_step = 0;
                    self.chat_scroll = u16::MAX;
                    self.chat_subpanel = ChatSubpanel::Transcript;
                    if response.is_error {
                        self.streaming_agent_message = None;
                        self.chat_messages.push(ChatMessage {
                            role: ChatRole::System,
                            content: response.message.clone(),
                        });
                        self.status_message = String::from("Agent request failed.");
                    } else {
                        let message_index = self.chat_messages.len();
                        self.chat_messages.push(ChatMessage {
                            role: ChatRole::Agent,
                            content: String::new(),
                        });
                        let total_chars = response.message.chars().count();
                        self.streaming_agent_message = Some(StreamingAgentMessage {
                            message_index,
                            full_message: response.message,
                            revealed_chars: 0,
                            total_chars,
                        });
                        self.advance_streaming_agent_message();
                        self.status_message = String::from("Agent replied.");
                    }
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.chat_response_rx = Some(rx);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.chat_request_in_flight = false;
                    self.streaming_agent_message = None;
                    self.thinking_wave_step = 0;
                    self.chat_subpanel = ChatSubpanel::Transcript;
                    self.chat_messages.push(ChatMessage {
                        role: ChatRole::System,
                        content: String::from("Agent process ended without returning a response."),
                    });
                    self.status_message = String::from("Agent request failed.");
                    changed = true;
                }
            }
        }

        if let Some(rx) = self.connection_test_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    self.connection_test_in_flight = false;
                    if result.ok {
                        self.status_message = format!("Connection test passed: {}", result.detail);
                    } else {
                        self.status_message =
                            String::from("Connection test failed (see Chat for details).");
                        self.chat_messages.push(ChatMessage {
                            role: ChatRole::System,
                            content: format!("Connection test failed: {}", result.detail),
                        });
                        self.chat_scroll = u16::MAX;
                    }
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.connection_test_rx = Some(rx);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.connection_test_in_flight = false;
                    self.status_message = String::from("Connection test failed: worker crashed.");
                    changed = true;
                }
            }
        }

        if let Some(rx) = self.model_list_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    self.model_list_in_flight = false;
                    match result {
                        Ok(choices) => {
                            if self.model_picker_active {
                                self.apply_model_picker_choices(choices);
                                self.status_message = format!(
                                    "Loaded {} models from {}.",
                                    self.model_picker_options.len(),
                                    self.active_provider_descriptor().display_name
                                );
                                changed = true;
                            }
                        }
                        Err(detail) => {
                            if self.model_picker_active {
                                self.status_message = format!(
                                    "Model list refresh failed (showing fallback): {detail}"
                                );
                                changed = true;
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.model_list_rx = Some(rx);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.model_list_in_flight = false;
                    if self.model_picker_active {
                        self.status_message = String::from(
                            "Model list refresh failed: background worker disconnected.",
                        );
                        changed = true;
                    }
                }
            }
        }

        changed
    }

    pub fn run_connection_test(&mut self) {
        if self.connection_test_in_flight {
            self.status_message = String::from("Connection test already running.");
            return;
        }

        if self.chat_request_in_flight {
            self.status_message =
                String::from("Wait for the current agent response to finish before testing.");
            return;
        }

        let provider = self.active_provider_descriptor();
        self.status_message = format!("Running {} connection test...", provider.display_name);

        let provider_id = self.active_provider_id().to_owned();
        let model = self.active_model_label().to_owned();
        let (tx, rx) = mpsc::channel();
        self.connection_test_rx = Some(rx);
        self.connection_test_in_flight = true;

        std::thread::spawn(move || {
            let result = run_with_timeout(CONNECTION_TEST_TIMEOUT, move || {
                run_provider_connection_test(&provider_id, &model)
            })
            .unwrap_or_else(|| ConnectionTestResult {
                ok: false,
                detail: format!(
                    "timed out after {}s while waiting for provider reply",
                    CONNECTION_TEST_TIMEOUT.as_secs()
                ),
            });
            let _ = tx.send(result);
        });
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
        let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

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
                if has_shift {
                    self.insert_char_at_cursor('\n');
                } else {
                    self.submit_composer_message();
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
                    self.scroll_chat_transcript(direction);
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

    fn open_model_picker(&mut self) {
        let provider_id = self.active_provider_id().to_owned();
        let current_model = self.active_model_label().to_owned();
        let mut options = build_fallback_model_choices(&provider_id, &current_model);
        if options.is_empty() {
            options.push(ModelChoice {
                id: current_model.clone(),
                description: String::from("Current configured model."),
                is_current: true,
            });
        }

        let selected = options
            .iter()
            .position(|option| option.id == current_model)
            .unwrap_or(0);

        self.model_picker_options = options;
        self.model_picker_selected = selected;
        self.model_picker_active = true;
        self.refresh_provider_model_choices(provider_id, current_model);
        self.status_message =
            String::from("Select a model (j/k or arrows, Enter to apply, Esc to cancel).");
    }

    fn refresh_provider_model_choices(&mut self, provider_id: String, current_model: String) {
        if self.model_list_in_flight {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.model_list_rx = Some(rx);
        self.model_list_in_flight = true;
        std::thread::spawn(move || {
            let result = run_provider_model_choices(&provider_id, &current_model);
            let _ = tx.send(result);
        });
    }

    fn apply_model_picker_choices(&mut self, choices: Vec<ModelChoice>) {
        if choices.is_empty() {
            return;
        }

        let selected_id = self
            .model_picker_options
            .get(self.model_picker_selected)
            .map(|choice| choice.id.clone());
        let current_model = self.active_model_label().to_owned();
        let mut normalized = normalize_model_choices(choices, &current_model);
        if normalized.is_empty() {
            normalized.push(ModelChoice {
                id: current_model,
                description: String::from("Current configured model."),
                is_current: true,
            });
        }

        let selected = selected_id
            .as_deref()
            .and_then(|id| normalized.iter().position(|choice| choice.id == id))
            .or_else(|| normalized.iter().position(|choice| choice.is_current))
            .unwrap_or(0);

        self.model_picker_options = normalized;
        self.model_picker_selected = selected;
    }

    fn move_model_picker_selection(&mut self, direction: i8) {
        if self.model_picker_options.is_empty() {
            self.model_picker_selected = 0;
            return;
        }

        let len = self.model_picker_options.len();
        self.model_picker_selected %= len;
        if direction > 0 {
            self.model_picker_selected = (self.model_picker_selected + 1) % len;
        } else if direction < 0 {
            self.model_picker_selected = if self.model_picker_selected == 0 {
                len - 1
            } else {
                self.model_picker_selected - 1
            };
        }
    }

    fn apply_selected_model_choice(&mut self) {
        if self.model_picker_options.is_empty() {
            self.model_picker_active = false;
            self.status_message = String::from("No model options available.");
            return;
        }

        let selected = self
            .model_picker_options
            .get(self.model_picker_selected)
            .map(|choice| choice.id.clone())
            .unwrap_or_else(|| self.active_model_label().to_owned());
        let current = self.active_model_label().to_owned();
        self.model_picker_active = false;

        if selected == current {
            self.status_message = format!("Model unchanged: {selected}");
            return;
        }

        self.set_active_model(&selected);
    }

    fn set_active_model(&mut self, model: &str) {
        let provider = self.active_provider_descriptor();
        let provider_settings = self.config.ensure_provider(&provider);
        provider_settings.preferred_model = Some(model.to_owned());
        match self.config.save() {
            Ok(()) => {
                self.status_message = format!("Model set to {model}.");
            }
            Err(error) => {
                self.status_message = format!("Model updated in memory only ({model}): {error}");
            }
        }
    }

    fn handle_composer_command(&mut self, message: &str) -> bool {
        let trimmed = message.trim();
        if !trimmed.starts_with('/') {
            return false;
        }

        let mut tokens = trimmed.split_whitespace();
        let command = tokens.next().unwrap_or_default();
        match command {
            "/model" => {
                self.open_model_picker();
                true
            }
            _ => {
                self.status_message =
                    format!("Unknown command '{command}'. Supported commands: /model");
                true
            }
        }
    }

    fn submit_composer_message(&mut self) {
        let message = self.chat_draft.trim().to_owned();
        if message.is_empty() {
            self.status_message = String::from("Type a message before sending.");
            return;
        }

        if self.handle_composer_command(&message) {
            self.chat_draft.clear();
            self.chat_cursor = 0;
            self.chat_preferred_column = None;
            return;
        }

        if self.chat_request_in_flight {
            self.status_message = String::from("Wait for the current agent response to finish.");
            return;
        }
        if self.streaming_agent_message.is_some() {
            self.status_message = String::from("Wait for the current agent response to finish.");
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
        self.status_message = String::from("Asking agent...");
        self.start_agent_response();
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

    fn start_agent_response(&mut self) {
        let provider_id = self.active_provider_id().to_owned();
        let model = self.active_model_label().to_owned();
        let worktree = self.selected_worktree().clone();
        let chat_history = self.chat_messages.clone();
        let (tx, rx) = mpsc::channel();
        self.chat_response_rx = Some(rx);
        self.chat_request_in_flight = true;
        self.streaming_agent_message = None;
        self.thinking_wave_step = 0;

        std::thread::spawn(move || {
            let response = run_with_timeout(CHAT_REQUEST_TIMEOUT, move || {
                run_provider_chat(&provider_id, &model, &worktree, &chat_history)
            })
            .unwrap_or_else(|| ChatResponse {
                is_error: true,
                message: format!(
                    "Agent request timed out after {}s. Check connectivity with Ctrl/Cmd+Y or F8.",
                    CHAT_REQUEST_TIMEOUT.as_secs()
                ),
            });
            let _ = tx.send(response);
        });
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

    fn advance_streaming_agent_message(&mut self) -> bool {
        let Some(streaming) = &mut self.streaming_agent_message else {
            return false;
        };

        if streaming.total_chars == 0 {
            self.streaming_agent_message = None;
            return false;
        }

        if streaming.revealed_chars >= streaming.total_chars {
            self.streaming_agent_message = None;
            return false;
        }

        let step = agent_stream_chars_per_tick(streaming.total_chars);
        let previous = streaming.revealed_chars;
        let next = previous.saturating_add(step).min(streaming.total_chars);

        let start_byte = byte_index_for_char(&streaming.full_message, previous);
        let end_byte = byte_index_for_char(&streaming.full_message, next);
        let chunk = streaming.full_message[start_byte..end_byte].to_string();

        if let Some(message) = self.chat_messages.get_mut(streaming.message_index) {
            message.content.push_str(&chunk);
        } else {
            self.streaming_agent_message = None;
            return false;
        }

        streaming.revealed_chars = next;
        self.chat_scroll = u16::MAX;
        if streaming.revealed_chars >= streaming.total_chars {
            self.streaming_agent_message = None;
        }
        true
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

fn agent_stream_chars_per_tick(total_chars: usize) -> usize {
    let scaled = total_chars / 80;
    AGENT_STREAM_MIN_CHARS_PER_TICK.max(scaled.min(24))
}

fn thinking_wave_cycle_len() -> usize {
    if THINKING_WAVE_WIDTH == 0 || THINKING_WAVE_KERNEL.is_empty() {
        return 0;
    }

    let max_phase = THINKING_WAVE_WIDTH + THINKING_WAVE_KERNEL.len() - 1;
    if max_phase <= 1 {
        return max_phase + 1;
    }

    let forward_len = max_phase + 1;
    let reverse_len = max_phase - 1;
    forward_len
        .saturating_add(reverse_len)
        .saturating_add(THINKING_WAVE_START_HOLD_TICKS)
}

fn thinking_wave_phase(step: usize) -> usize {
    let max_phase = THINKING_WAVE_WIDTH + THINKING_WAVE_KERNEL.len() - 1;
    if max_phase == 0 {
        return 0;
    }
    if max_phase == 1 {
        return step % 2;
    }

    let cycle_len = thinking_wave_cycle_len().max(1);
    let tick = step % cycle_len;

    let forward_len = max_phase + 1;
    if tick < forward_len {
        return tick;
    }

    let reverse_len = max_phase - 1;
    let reverse_end = forward_len + reverse_len;
    if tick < reverse_end {
        let reverse_idx = tick - forward_len;
        return max_phase.saturating_sub(reverse_idx + 1);
    }

    0
}

fn render_thinking_wave(step: usize) -> String {
    if THINKING_WAVE_WIDTH == 0 || THINKING_WAVE_KERNEL.is_empty() {
        return String::new();
    }

    let phase = thinking_wave_phase(step);
    (0..THINKING_WAVE_WIDTH)
        .map(|idx| {
            let kernel_idx = phase as isize - idx as isize;
            let level = if (0..THINKING_WAVE_KERNEL.len() as isize).contains(&kernel_idx) {
                THINKING_WAVE_KERNEL[kernel_idx as usize] as usize
            } else {
                0
            };
            THINKING_WAVE_LEVEL_GLYPHS[level.min(THINKING_WAVE_LEVEL_GLYPHS.len() - 1)]
        })
        .collect()
}

fn build_fallback_model_choices(provider_id: &str, current_model: &str) -> Vec<ModelChoice> {
    let catalog: &[(&str, &str)] = match provider_id {
        "codex" => &CODEX_MODEL_CATALOG,
        _ => &[],
    };

    let choices = catalog
        .iter()
        .map(|(id, description)| ModelChoice {
            id: (*id).to_string(),
            description: (*description).to_string(),
            is_current: false,
        })
        .collect::<Vec<_>>();

    normalize_model_choices(choices, current_model)
}

#[derive(Debug)]
struct ChatResponse {
    message: String,
    is_error: bool,
}

fn run_provider_chat(
    provider_id: &str,
    model: &str,
    worktree: &Worktree,
    chat_history: &[ChatMessage],
) -> ChatResponse {
    match provider_id {
        "codex" => run_codex_chat(model, worktree, chat_history),
        other => ChatResponse {
            is_error: true,
            message: format!("Provider '{other}' does not support chat yet."),
        },
    }
}

fn run_provider_model_choices(provider_id: &str, current_model: &str) -> ModelListFetchResult {
    match provider_id {
        "codex" => run_codex_model_choices(current_model),
        _ => Ok(build_fallback_model_choices(provider_id, current_model)),
    }
}

fn run_codex_model_choices(current_model: &str) -> ModelListFetchResult {
    let payload = run_codex_model_list_request()?;
    parse_codex_model_list_choices(&payload, current_model)
}

fn run_codex_model_list_request() -> Result<Value, String> {
    let mut child = Command::new("codex")
        .arg("app-server")
        .arg("--listen")
        .arg("stdio://")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                String::from("Codex CLI was not found in PATH.")
            } else {
                format!("failed to start Codex app-server: {error}")
            }
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| String::from("failed to open codex app-server stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("failed to open codex app-server stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| String::from("failed to open codex app-server stderr"))?;

    let (stdout_tx, stdout_rx) = mpsc::channel::<String>();
    let (stderr_tx, stderr_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = stdout_tx.send(line);
        }
    });
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = stderr_tx.send(line);
        }
    });

    let initialize = serde_json::json!({
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "agent-manager-tui",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    });
    let initialized = serde_json::json!({ "method": "initialized" });
    let model_list = serde_json::json!({
        "id": 2,
        "method": "model/list",
        "params": {
            "includeHidden": false,
            "limit": MODEL_LIST_REQUEST_LIMIT
        }
    });

    writeln!(stdin, "{initialize}")
        .map_err(|error| format!("failed to write initialize request: {error}"))?;
    writeln!(stdin, "{initialized}")
        .map_err(|error| format!("failed to write initialized notification: {error}"))?;
    writeln!(stdin, "{model_list}")
        .map_err(|error| format!("failed to write model/list request: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("failed to flush model/list request: {error}"))?;

    let deadline = Instant::now() + MODEL_LIST_REQUEST_TIMEOUT;
    let mut response = None;
    let mut rpc_error = None;
    let mut stderr_lines = Vec::new();
    loop {
        while let Ok(line) = stderr_rx.try_recv() {
            if is_non_fatal_codex_warning(&line) {
                continue;
            }
            let simplified = simplify_error_line(&line);
            if is_meaningful_error_line(&simplified) {
                stderr_lines.push(simplified);
            }
        }

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let timeout = deadline.saturating_duration_since(now);
        match stdout_rx.recv_timeout(timeout) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(payload) = serde_json::from_str::<Value>(trimmed) else {
                    continue;
                };
                if payload.get("id").and_then(Value::as_i64) != Some(2) {
                    continue;
                }

                if let Some(result) = payload.get("result") {
                    response = Some(result.clone());
                    break;
                }
                if let Some(error) = payload.get("error") {
                    rpc_error = Some(summarize_json_rpc_error(error));
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    while let Ok(line) = stderr_rx.try_recv() {
        if is_non_fatal_codex_warning(&line) {
            continue;
        }
        let simplified = simplify_error_line(&line);
        if is_meaningful_error_line(&simplified) {
            stderr_lines.push(simplified);
        }
    }

    if let Some(detail) = rpc_error {
        return Err(detail);
    }
    if let Some(result) = response {
        return Ok(result);
    }
    if let Some(detail) = stderr_lines.into_iter().last() {
        return Err(detail);
    }

    Err(format!(
        "timed out after {}s waiting for model list response",
        MODEL_LIST_REQUEST_TIMEOUT.as_secs()
    ))
}

fn parse_codex_model_list_choices(payload: &Value, current_model: &str) -> ModelListFetchResult {
    let data = payload
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| String::from("model/list response was missing 'data'"))?;

    let mut seen_ids = BTreeSet::new();
    let mut choices = Vec::new();
    for model in data {
        if model
            .get("hidden")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }

        let id = model
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| model.get("model").and_then(Value::as_str))
            .unwrap_or_default()
            .trim();
        if id.is_empty() {
            continue;
        }
        if !seen_ids.insert(id.to_owned()) {
            continue;
        }

        let description = model
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Available from provider.")
            .trim();
        let mut description = if description.is_empty() {
            String::from("Available from provider.")
        } else {
            description.to_owned()
        };
        if model
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && !description.contains("(default)")
        {
            description.push_str(" (default)");
        }

        choices.push(ModelChoice {
            id: id.to_owned(),
            description,
            is_current: false,
        });
    }

    if choices.is_empty() {
        return Err(String::from("provider returned no visible models."));
    }

    Ok(normalize_model_choices(choices, current_model))
}

fn normalize_model_choices(mut choices: Vec<ModelChoice>, current_model: &str) -> Vec<ModelChoice> {
    if choices.is_empty() {
        if current_model.is_empty() {
            return choices;
        }
        return vec![ModelChoice {
            id: current_model.to_string(),
            description: String::from("Current configured model."),
            is_current: true,
        }];
    }

    for choice in &mut choices {
        choice.is_current = choice.id == current_model;
    }

    if !current_model.is_empty() && !choices.iter().any(|choice| choice.id == current_model) {
        choices.insert(
            0,
            ModelChoice {
                id: current_model.to_string(),
                description: String::from("Current configured model."),
                is_current: true,
            },
        );
    }

    choices
}

fn summarize_json_rpc_error(error: &Value) -> String {
    if let Some(message) = error.get("message").and_then(Value::as_str) {
        let message = message.trim();
        if !message.is_empty() {
            return message.to_string();
        }
    }

    if let Some(code) = error.get("code").and_then(Value::as_i64) {
        let detail = error
            .get("data")
            .map(Value::to_string)
            .unwrap_or_else(|| String::from("no additional data"));
        return format!("code {code}: {detail}");
    }

    let raw = error.to_string();
    if raw.is_empty() {
        String::from("unknown json-rpc error")
    } else {
        raw
    }
}

fn is_non_fatal_codex_warning(line: &str) -> bool {
    line.contains("could not update PATH")
}

fn run_codex_chat(model: &str, worktree: &Worktree, chat_history: &[ChatMessage]) -> ChatResponse {
    let prompt = build_codex_chat_prompt(worktree, chat_history);
    match run_codex_exec_last_message(model, &prompt) {
        Ok(message) => ChatResponse {
            message,
            is_error: false,
        },
        Err(message) => ChatResponse {
            is_error: true,
            message,
        },
    }
}

fn build_codex_chat_prompt(worktree: &Worktree, chat_history: &[ChatMessage]) -> String {
    let mut prompt = String::from(
        "You are an assistant inside AgentManager TUI. Reply conversationally and concisely in plain text.\n\n",
    );
    prompt.push_str("Current context:\n");
    prompt.push_str(&format!(
        "- repo: {}\n- worktree: {}\n- branch: {}\n- status: {}\n- summary: {}\n\n",
        worktree.repo, worktree.name, worktree.branch, worktree.status, worktree.summary
    ));
    prompt.push_str("Conversation so far:\n");

    let start = chat_history.len().saturating_sub(20);
    for message in chat_history.iter().skip(start) {
        let role = match message.role {
            ChatRole::Agent => "agent",
            ChatRole::User => "user",
            ChatRole::System => "system",
        };
        prompt.push_str(&format!("{role}: {}\n", message.content));
    }

    prompt.push_str("\nRespond to the most recent user message.");
    prompt
}

fn extract_codex_failure_detail(
    stderr: &str,
    stdout: &str,
    status: std::process::ExitStatus,
) -> String {
    let mut lines = Vec::new();
    lines.extend(
        stderr
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned),
    );
    lines.extend(
        stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned),
    );

    if let Some(line) = lines.iter().find(|line| {
        line.contains("stream disconnected before completion")
            || line.contains("error sending request for url")
    }) {
        return format!(
            "{}. Check network/VPN/proxy and run Ctrl/Cmd+T.",
            simplify_error_line(line)
        );
    }

    if let Some(line) = lines.iter().find(|line| {
        line.contains("Logged out")
            || line.contains("not logged in")
            || line.contains("authentication")
    }) {
        return format!(
            "{}. Run `codex login` and retry.",
            simplify_error_line(line)
        );
    }

    if let Some(line) = lines
        .iter()
        .find(|line| line.contains("mcp startup: failed"))
    {
        return format!(
            "{}. In-app chat disables shadcn MCP, but your global Codex config may still load other slow MCP servers.",
            simplify_error_line(line)
        );
    }

    if let Some(line) = lines
        .iter()
        .find(|line| line.contains("no last agent message"))
    {
        return format!(
            "{}. Codex ran but did not produce a final assistant message.",
            simplify_error_line(line)
        );
    }

    if let Some(api_error) = summarize_structured_api_error(&lines) {
        return api_error;
    }

    lines
        .iter()
        .rev()
        .find_map(|line| {
            let simplified = simplify_error_line(line);
            if !is_meaningful_error_line(&simplified) {
                None
            } else {
                Some(simplified)
            }
        })
        .unwrap_or_else(|| {
            format!(
                "request failed ({}). Codex CLI did not emit a parseable error detail.",
                status
            )
        })
}

fn simplify_error_line(line: &str) -> String {
    let trimmed = line.trim();
    if let Some(idx) = trimmed.find("ERROR:") {
        return trimmed[idx + "ERROR:".len()..].trim().to_string();
    }
    if let Some(idx) = trimmed.find("Caused by:") {
        return trimmed[idx + "Caused by:".len()..].trim().to_string();
    }

    trimmed.to_string()
}

fn is_meaningful_error_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    if matches!(trimmed, "{" | "}" | "[" | "]" | "," | ":" | "--------") {
        return false;
    }

    if trimmed.eq_ignore_ascii_case("assistant")
        || trimmed.eq_ignore_ascii_case("user")
        || trimmed.eq_ignore_ascii_case("system")
    {
        return false;
    }

    if is_codex_metadata_line(trimmed) {
        return false;
    }

    trimmed.chars().any(char::is_alphanumeric)
}

fn summarize_structured_api_error(lines: &[String]) -> Option<String> {
    let mut message = None;
    let mut code = None;
    let mut param = None;

    for line in lines {
        if message.is_none() {
            message = extract_json_like_field(line, "message");
        }
        if code.is_none() {
            code = extract_json_like_field(line, "code");
        }
        if param.is_none() {
            param = extract_json_like_field(line, "param");
        }
    }

    if message.is_none() && code.is_none() && param.is_none() {
        return None;
    }

    let mut detail = String::new();
    if let Some(value) = message {
        detail.push_str(&value);
    }

    let mut extras = Vec::new();
    if let Some(value) = code {
        extras.push(format!("code={value}"));
    }
    if let Some(value) = param {
        extras.push(format!("param={value}"));
    }

    if !extras.is_empty() {
        if !detail.is_empty() {
            detail.push(' ');
        }
        detail.push('(');
        detail.push_str(&extras.join(", "));
        detail.push(')');
    }

    let normalized = detail.trim().to_ascii_lowercase();
    if normalized.contains("unsupported_value")
        || normalized.contains("unsupported value")
        || normalized.contains("model_reasoning_effort")
    {
        detail.push_str(". Try a supported model or reasoning effort.");
    }

    Some(detail)
}

fn extract_json_like_field(line: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let key_idx = line.find(&marker)?;
    let tail = line.get(key_idx + marker.len()..)?.trim_start();
    if !tail.starts_with(':') {
        return None;
    }

    let raw = tail.get(1..)?.trim_start();
    if raw.is_empty() {
        return None;
    }

    if let Some(quoted) = raw.strip_prefix('"') {
        let end = quoted.find('"')?;
        let value = quoted.get(..end)?.trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }

    let end = raw
        .find(|ch: char| ch == ',' || ch == '}' || ch.is_whitespace())
        .unwrap_or(raw.len());
    let value = raw.get(..end)?.trim().trim_matches('"').trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[derive(Debug)]
struct ConnectionTestResult {
    ok: bool,
    detail: String,
}

fn run_provider_connection_test(provider_id: &str, model: &str) -> ConnectionTestResult {
    match provider_id {
        "codex" => run_codex_connection_test(model),
        other => ConnectionTestResult {
            ok: false,
            detail: format!("provider '{other}' does not support tests yet"),
        },
    }
}

fn run_codex_connection_test(model: &str) -> ConnectionTestResult {
    match run_codex_exec_last_message(model, "Reply with exactly OK.") {
        Ok(reply) => {
            if reply.trim().eq_ignore_ascii_case("ok") {
                return ConnectionTestResult {
                    ok: true,
                    detail: format!("{model} responded"),
                };
            }

            ConnectionTestResult {
                ok: true,
                detail: format!(
                    "{model} responded ({}).",
                    truncate_for_status(reply.trim(), 48)
                ),
            }
        }
        Err(detail) => ConnectionTestResult { ok: false, detail },
    }
}

fn run_codex_exec_last_message(model: &str, prompt: &str) -> Result<String, String> {
    let output_last_message_path = codex_output_last_message_path();

    let output = Command::new("codex")
        .arg("exec")
        .arg("-c")
        .arg("mcp_servers.shadcn.enabled=false")
        .arg("-c")
        .arg("model_reasoning_effort=\"medium\"")
        .arg("--skip-git-repo-check")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--model")
        .arg(model)
        .arg("--output-last-message")
        .arg(&output_last_message_path)
        .arg(prompt)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            let detail = if error.kind() == std::io::ErrorKind::NotFound {
                String::from(
                    "Codex CLI was not found in PATH. Install/open Codex CLI first and try again.",
                )
            } else {
                format!("failed to start Codex CLI: {error}")
            };
            return Err(detail);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let detail = extract_codex_failure_detail(&stderr, &stdout, output.status);
        let _ = fs::remove_file(&output_last_message_path);
        return Err(format!("Codex request failed: {detail}"));
    }

    let merged_output = if stdout.is_empty() {
        stderr.clone()
    } else if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{stdout}\n{stderr}")
    };

    if let Some(message) = read_output_last_message(&output_last_message_path) {
        let _ = fs::remove_file(&output_last_message_path);
        return Ok(message);
    }

    let _ = fs::remove_file(&output_last_message_path);

    if let Some(message) = extract_assistant_message_from_codex_stdout(&stdout)
        .or_else(|| extract_assistant_message_from_codex_stdout(&stderr))
        .or_else(|| extract_assistant_message_from_codex_stdout(&merged_output))
    {
        return Ok(message);
    }

    let detail = extract_codex_failure_detail(&stderr, &stdout, output.status);
    Err(format!(
        "Codex completed without a final assistant message: {detail}"
    ))
}

fn codex_output_last_message_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.push(format!(
        "agent-manager-codex-last-message-{}-{}.txt",
        std::process::id(),
        nonce
    ));
    path
}

fn read_output_last_message(path: &PathBuf) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn truncate_for_status(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    if max_chars <= 1 {
        return "…".to_string();
    }

    let keep = max_chars - 1;
    let prefix = value.chars().take(keep).collect::<String>();
    format!("{prefix}…")
}

fn run_with_timeout<T, F>(timeout: Duration, task: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let output = task();
        let _ = tx.send(output);
    });

    rx.recv_timeout(timeout).ok()
}

fn extract_assistant_message_from_codex_stdout(stdout: &str) -> Option<String> {
    let trimmed_stdout = stdout.trim();
    if trimmed_stdout.is_empty() {
        return None;
    }

    let lines = stdout.lines().collect::<Vec<_>>();
    if let Some(assistant_start) = lines
        .iter()
        .enumerate()
        .rev()
        .find_map(|(idx, line)| (line.trim().eq_ignore_ascii_case("assistant")).then_some(idx))
    {
        let mut body = Vec::new();
        for line in lines.iter().skip(assistant_start + 1) {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                if !body.is_empty() {
                    body.push(String::new());
                }
                continue;
            }

            if looks_like_codex_log_line(trimmed)
                || trimmed.starts_with("mcp: ")
                || trimmed.starts_with("mcp startup:")
                || trimmed.eq_ignore_ascii_case("user")
                || trimmed.eq_ignore_ascii_case("assistant")
            {
                break;
            }

            body.push(trimmed.to_string());
        }

        let transcript_message = body.join("\n").trim().to_string();
        if !transcript_message.is_empty() {
            return Some(transcript_message);
        }
    }

    let mut trailing_block = Vec::new();
    let mut stopped_on_user_or_system = false;
    for line in lines.iter().rev() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !trailing_block.is_empty() {
                trailing_block.push(String::new());
            }
            continue;
        }

        if trimmed.eq_ignore_ascii_case("assistant")
            || trimmed.eq_ignore_ascii_case("user")
            || trimmed.eq_ignore_ascii_case("system")
        {
            stopped_on_user_or_system =
                trimmed.eq_ignore_ascii_case("user") || trimmed.eq_ignore_ascii_case("system");
            break;
        }

        if is_codex_metadata_line(trimmed) {
            if !trailing_block.is_empty() {
                break;
            }
            continue;
        }

        trailing_block.push(trimmed.to_string());
    }

    while matches!(trailing_block.last(), Some(line) if line.is_empty()) {
        trailing_block.pop();
    }

    trailing_block.reverse();
    let trailing_message = trailing_block.join("\n").trim().to_string();
    if !trailing_message.is_empty() && !stopped_on_user_or_system {
        return Some(trailing_message);
    }

    if !lines.iter().any(|line| is_codex_metadata_line(line.trim())) {
        return Some(trimmed_stdout.to_string());
    }

    None
}

fn looks_like_codex_log_line(line: &str) -> bool {
    line.len() > 24
        && line.chars().nth(4) == Some('-')
        && line.contains('T')
        && (line.contains(" WARN ")
            || line.contains(" ERROR ")
            || line.contains(" INFO ")
            || line.contains(" DEBUG "))
}

fn is_codex_metadata_line(line: &str) -> bool {
    looks_like_codex_log_line(line)
        || line.starts_with("mcp: ")
        || line.starts_with("mcp startup:")
        || line.starts_with("OpenAI Codex")
        || line.starts_with("--------")
        || line.starts_with("workdir:")
        || line.starts_with("model:")
        || line.starts_with("provider:")
        || line.starts_with("approval:")
        || line.starts_with("sandbox:")
        || line.starts_with("reasoning effort:")
        || line.starts_with("reasoning summaries:")
        || line.starts_with("session id:")
        || line.starts_with("Reconnecting...")
        || line.starts_with("WARNING:")
        || line.starts_with("ERROR:")
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

#[cfg(test)]
mod tests {
    use super::{
        extract_assistant_message_from_codex_stdout, extract_codex_failure_detail,
        normalize_model_choices, parse_codex_model_list_choices, render_thinking_wave,
        summarize_structured_api_error, thinking_wave_cycle_len,
    };
    use serde_json::json;

    fn failing_status() -> std::process::ExitStatus {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("exit 7")
            .status()
            .expect("failed to create exit status for test")
    }

    #[test]
    fn extracts_assistant_block_from_transcript() {
        let output = "\
OpenAI Codex v0.104.0
--------
user
Say hi
assistant
Hi there
";

        assert_eq!(
            extract_assistant_message_from_codex_stdout(output),
            Some(String::from("Hi there"))
        );
    }

    #[test]
    fn extracts_plain_output_without_transcript_markers() {
        assert_eq!(
            extract_assistant_message_from_codex_stdout("OK"),
            Some(String::from("OK"))
        );
    }

    #[test]
    fn does_not_treat_user_prompt_as_assistant_reply() {
        let output = "\
OpenAI Codex v0.104.0
--------
workdir: /tmp
user
Reply with exactly OK.
";

        assert_eq!(extract_assistant_message_from_codex_stdout(output), None);
    }

    #[test]
    fn captures_full_trailing_message_without_assistant_marker() {
        let output = "\
OpenAI Codex v0.104.0
--------
mcp startup: no servers

Here are the steps:
1. Create a ShaderMaterial and new shader.
2. Paste your shader code and assign it.

That's all you need to attach and iterate on shaders in Godot.
2026-02-25T22:23:21.809926Z  WARN codex_core::state_db: sample warning
";

        let parsed = extract_assistant_message_from_codex_stdout(output)
            .expect("expected trailing assistant-like message");
        assert!(parsed.contains("Here are the steps:"));
        assert!(parsed.contains("That's all you need"));
    }

    #[test]
    fn extracts_network_failure_detail() {
        let stderr = "ERROR: stream disconnected before completion: error sending request for url (https://chatgpt.com/backend-api/codex/responses)";
        let detail = extract_codex_failure_detail(stderr, "", failing_status());

        assert!(detail.contains("stream disconnected before completion"));
        assert!(detail.contains("Check network/VPN/proxy"));
    }

    #[test]
    fn ignores_brace_only_fallback_noise() {
        let stderr = "\
OpenAI Codex v0.104.0
--------
}
";
        let detail = extract_codex_failure_detail(stderr, "", failing_status());

        assert!(detail.contains("request failed"));
        assert_ne!(detail.trim(), "}");
    }

    #[test]
    fn summarizes_structured_api_error_fields() {
        let lines = vec![
            String::from("{"),
            String::from("\"message\": \"Invalid value for model_reasoning_effort\""),
            String::from("\"code\": \"unsupported_value\""),
            String::from("\"param\": \"model_reasoning_effort\""),
            String::from("}"),
        ];

        let detail = summarize_structured_api_error(&lines).expect("expected structured summary");
        assert!(detail.contains("Invalid value for model_reasoning_effort"));
        assert!(detail.contains("code=unsupported_value"));
        assert!(detail.contains("param=model_reasoning_effort"));
    }

    #[test]
    fn thinking_wave_boomerangs() {
        assert_eq!(thinking_wave_cycle_len(), 32);
        assert_eq!(render_thinking_wave(0), "⢀⢀⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(1), "⠠⢀⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(2), "⠐⠠⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(3), "⠈⠐⠠⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(4), "⠈⠈⠐⠠⢀⢀⢀⢀"); // 33210000
        assert_eq!(render_thinking_wave(5), "⠐⠈⠈⠐⠠⢀⢀⢀"); // 23321000
        assert_eq!(render_thinking_wave(6), "⠠⠐⠈⠈⠐⠠⢀⢀"); // 12332100
        assert_eq!(render_thinking_wave(7), "⢀⠠⠐⠈⠈⠐⠠⢀"); // 01233210
        assert_eq!(render_thinking_wave(15), "⢀⢀⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(16), "⢀⢀⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(17), "⢀⢀⢀⢀⢀⢀⢀⠠");
        assert_eq!(render_thinking_wave(21), "⢀⢀⢀⠠⠐⠈⠈⠐");
        assert_eq!(render_thinking_wave(23), "⢀⠠⠐⠈⠈⠐⠠⢀");
        assert_eq!(render_thinking_wave(29), "⠠⢀⢀⢀⢀⢀⢀⢀");
        assert_eq!(render_thinking_wave(30), render_thinking_wave(0));
        assert_eq!(render_thinking_wave(31), render_thinking_wave(0));
        assert_eq!(render_thinking_wave(32), render_thinking_wave(0));
        assert_eq!(render_thinking_wave(33), render_thinking_wave(1));
    }

    #[test]
    fn parses_model_list_response_and_marks_current() {
        let payload = json!({
            "data": [
                {
                    "id": "gpt-5.3-codex",
                    "description": "Latest frontier agentic coding model.",
                    "hidden": false,
                    "isDefault": true
                },
                {
                    "id": "gpt-5.2-codex",
                    "description": "Frontier agentic coding model.",
                    "hidden": false,
                    "isDefault": false
                }
            ]
        });

        let choices = parse_codex_model_list_choices(&payload, "gpt-5.2-codex")
            .expect("expected parsed model list");
        assert_eq!(choices.len(), 2);
        assert!(
            choices
                .iter()
                .any(|choice| choice.id == "gpt-5.2-codex" && choice.is_current)
        );
        assert!(
            choices.iter().any(
                |choice| choice.id == "gpt-5.3-codex" && choice.description.contains("default")
            )
        );
    }

    #[test]
    fn normalize_model_choices_inserts_current_when_missing() {
        let choices = normalize_model_choices(
            vec![super::ModelChoice {
                id: String::from("gpt-5.3-codex"),
                description: String::from("Latest"),
                is_current: false,
            }],
            "custom-model",
        );

        assert_eq!(choices[0].id, "custom-model");
        assert!(choices[0].is_current);
    }
}
