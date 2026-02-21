mod buttons;
mod chrome;
mod diff_panel;
mod left_panel;
mod right_panel;
mod textarea;
mod tooltip;

use crate::color_system::{ThemePalette, ThemeSelection};
use crate::mock_data::{epic_b_mock_data, DiffViewMode, EpicBMockData};
use crate::ui_state::{PaneSizes, ReviewMode, UiState};
use gpui::*;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    time::Instant,
};

const DEFAULT_LEFT_PANE_WIDTH: f32 = 260.0;
const DEFAULT_RIGHT_PANE_WIDTH: f32 = 320.0;
const MIN_SIDE_PANE_WIDTH: f32 = 180.0;
const MAX_SIDE_PANE_WIDTH: f32 = 640.0;
const MIN_MIDDLE_PANE_WIDTH: f32 = 420.0;
const SIDE_PANE_RESIZE_STEP: f32 = 24.0;
const SPLITTER_TRACK_WIDTH: f32 = 1.0;
const SPLITTER_HIT_WIDTH: f32 = 8.0;
const ACCORDION_ANIMATION_MS: u64 = 180;
const LANE_ROW_ESTIMATED_HEIGHT: f32 = 58.0;
const LANE_ROW_GAP: f32 = 2.0;
const LANE_SECTION_HEIGHT_SLACK: f32 = 6.0;
const LANE_ROW_HEIGHT_ERROR_MARGIN: f32 = 2.0;
const TOP_BAR_HEIGHT: f32 = 40.0;
#[cfg(target_os = "macos")]
const TOP_BAR_TRAFFIC_LIGHT_SPACER: f32 = 72.0;
#[cfg(not(target_os = "macos"))]
const TOP_BAR_TRAFFIC_LIGHT_SPACER: f32 = 0.0;
const KEY_CONTEXT: &str = "agent-manager-shell";
const ICON_ARROW_UP: &str = "icons/lucide-arrow-up.svg";
const ICON_CHEVRON_DOWN: &str = "icons/lucide-chevron-down.svg";
const ICON_CHEVRON_RIGHT: &str = "icons/lucide-chevron-right.svg";
const ICON_ARCHIVE: &str = "icons/lucide-archive.svg";
const ICON_MIC: &str = "icons/lucide-mic.svg";
const ICON_FOLDER: &str = "icons/lucide-folder.svg";
const ICON_FOLDER_OPEN: &str = "icons/lucide-folder-open.svg";
const ICON_FOLDER_PLUS: &str = "icons/lucide-folder-plus.svg";
const ICON_GIT_BRANCH: &str = "icons/lucide-git-branch.svg";
const ICON_SETTINGS: &str = "icons/lucide-settings.svg";
const ICON_SQUARE_MINUS: &str = "icons/lucide-square-minus.svg";
const ICON_SQUARE_PLUS: &str = "icons/lucide-square-plus.svg";
const ICON_SQUARE_DOT: &str = "icons/lucide-square-dot.svg";

struct DraggedLeftPaneHandle;
struct DraggedRightPaneHandle;

#[derive(Debug, Clone)]
pub(super) struct ChatMessage {
    pub author: SharedString,
    pub text: SharedString,
    pub outgoing: bool,
}

actions!(
    agent_manager_ui,
    [
        FocusLeftPane,
        FocusMiddlePane,
        FocusRightPane,
        FocusNextPane,
        FocusPreviousPane,
        SelectNextFile,
        SelectPreviousFile,
        ToggleDiffMode,
        ToggleThemeMode,
        StageOrUnstageSelectedFile,
        RevertSelectedFile,
        IncreaseSidePaneWidth,
        DecreaseSidePaneWidth
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivePane {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    Neutral,
    Primary,
    Success,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonSize {
    Regular,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonVariant {
    Text,
    Icon,
    IconText,
}

impl ButtonSize {
    fn height(self) -> f32 {
        match self {
            Self::Regular => 24.0,
            Self::Compact => 14.0,
        }
    }

    fn horizontal_padding(self) -> f32 {
        match self {
            Self::Regular => 8.0,
            Self::Compact => 2.0,
        }
    }
}

impl ActivePane {
    fn next(self) -> Self {
        match self {
            Self::Left => Self::Middle,
            Self::Middle => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Middle => Self::Left,
            Self::Right => Self::Middle,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Left => "Agents",
            Self::Middle => "Diff",
            Self::Right => "Files",
        }
    }
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-1", FocusLeftPane, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-2", FocusMiddlePane, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-3", FocusRightPane, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-tab", FocusNextPane, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-tab", FocusPreviousPane, Some(KEY_CONTEXT)),
        KeyBinding::new("down", SelectNextFile, Some(KEY_CONTEXT)),
        KeyBinding::new("j", SelectNextFile, Some(KEY_CONTEXT)),
        KeyBinding::new("up", SelectPreviousFile, Some(KEY_CONTEXT)),
        KeyBinding::new("k", SelectPreviousFile, Some(KEY_CONTEXT)),
        KeyBinding::new("d", ToggleDiffMode, Some(KEY_CONTEXT)),
        KeyBinding::new("cmd-shift-l", ToggleThemeMode, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-l", ToggleThemeMode, Some(KEY_CONTEXT)),
        KeyBinding::new("s", StageOrUnstageSelectedFile, Some(KEY_CONTEXT)),
        KeyBinding::new("r", RevertSelectedFile, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-right", IncreaseSidePaneWidth, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-left", DecreaseSidePaneWidth, Some(KEY_CONTEXT)),
    ]);
}

pub struct AppShell {
    focus_handle: FocusHandle,
    active_pane: ActivePane,
    left_pane_width: f32,
    right_pane_width: f32,
    theme: ThemeSelection,
    data: EpicBMockData,
    selected_lane_id: Option<u32>,
    hovered_tooltip_id: Option<SharedString>,
    composer_focus_handle: FocusHandle,
    composer_content: SharedString,
    composer_placeholder: SharedString,
    composer_selected_range: Range<usize>,
    composer_marked_range: Option<Range<usize>>,
    composer_last_bounds: Option<Bounds<Pixels>>,
    composer_total_visual_lines: usize,
    composer_input_scroll: ScrollHandle,
    composer_mouse_selecting: bool,
    composer_mouse_selection_anchor: Option<usize>,
    composer_mouse_down_point: Option<Point<Pixels>>,
    composer_caret_blink_started_at: Instant,
    chat_messages: Vec<ChatMessage>,
    chat_thread_scroll: ScrollHandle,
    collapsed_repo_groups: HashSet<String>,
    repo_group_animation_versions: HashMap<String, u64>,
    status_text: SharedString,
}

impl AppShell {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut data = epic_b_mock_data();
        let mut left_pane_width = DEFAULT_LEFT_PANE_WIDTH;
        let mut right_pane_width = DEFAULT_RIGHT_PANE_WIDTH;
        let mut theme = ThemeSelection::default();
        let selected_lane_id = data.agent_lanes().first().map(|lane| lane.id);
        data.select_file_for_lane(selected_lane_id);
        let status_text: SharedString;

        match UiState::load() {
            Ok(ui_state) => {
                left_pane_width = Self::clamp_side_pane_width(ui_state.panes.left_px);
                right_pane_width = Self::clamp_side_pane_width(ui_state.panes.right_px);
                data.set_diff_mode(Self::review_mode_to_diff_mode(ui_state.last_review_mode));
                theme = ui_state.theme;
                status_text = "Loaded UI state from .agent-manager/ui-state.toml".into();
            }
            Err(error) => {
                eprintln!("Failed to load UI state: {error}");
                status_text = format!("UI state load failed: {error}").into();
            }
        }

        Self {
            focus_handle: cx.focus_handle(),
            active_pane: ActivePane::Middle,
            left_pane_width,
            right_pane_width,
            theme,
            data,
            selected_lane_id,
            hovered_tooltip_id: None,
            composer_focus_handle: cx.focus_handle(),
            composer_content: "".into(),
            composer_placeholder: "Message agent about this diff...".into(),
            composer_selected_range: 0..0,
            composer_marked_range: None,
            composer_last_bounds: None,
            composer_total_visual_lines: 1,
            composer_input_scroll: ScrollHandle::new(),
            composer_mouse_selecting: false,
            composer_mouse_selection_anchor: None,
            composer_mouse_down_point: None,
            composer_caret_blink_started_at: Instant::now(),
            chat_messages: vec![
                ChatMessage {
                    author: "Assistant".into(),
                    text: "Perfect! Added missing imports and verified the dialog compiles cleanly."
                        .into(),
                    outgoing: false,
                },
                ChatMessage {
                    author: "Reviewer".into(),
                    text: "Could we reuse command-style keyboard navigation in this dialog?".into(),
                    outgoing: true,
                },
                ChatMessage {
                    author: "Assistant".into(),
                    text: "Yes. I can wire list navigation with up/down + enter in the same pass."
                        .into(),
                    outgoing: false,
                },
            ],
            chat_thread_scroll: ScrollHandle::new(),
            collapsed_repo_groups: HashSet::new(),
            repo_group_animation_versions: HashMap::new(),
            status_text,
        }
    }

    pub fn root_focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    fn colors(&self) -> ThemePalette {
        self.theme.palette()
    }

    fn toggle_theme_mode(&mut self, cx: &mut Context<Self>) {
        self.theme = self.theme.toggled_mode();
        self.persist_ui_state();
        self.status_text = format!("Theme: neutral {}", self.theme.mode.label()).into();
        cx.notify();
    }

    fn select_lane(&mut self, lane_id: u32, cx: &mut Context<Self>) {
        self.selected_lane_id = Some(lane_id);
        self.data.select_file_for_lane(self.selected_lane_id);
        self.active_pane = ActivePane::Left;
        self.status_text = "Selected workspace lane".into();
        cx.notify();
    }

    fn review_mode_to_diff_mode(review_mode: ReviewMode) -> DiffViewMode {
        match review_mode {
            ReviewMode::Unified => DiffViewMode::Unified,
            ReviewMode::Split => DiffViewMode::Split,
        }
    }

    fn diff_mode_to_review_mode(diff_mode: DiffViewMode) -> ReviewMode {
        match diff_mode {
            DiffViewMode::Unified => ReviewMode::Unified,
            DiffViewMode::Split => ReviewMode::Split,
        }
    }

    fn persist_ui_state(&mut self) {
        let ui_state = UiState {
            panes: PaneSizes {
                left_px: self.left_pane_width,
                right_px: self.right_pane_width,
            },
            last_review_mode: Self::diff_mode_to_review_mode(self.data.diff_mode()),
            theme: self.theme,
        };

        if let Err(error) = ui_state.save() {
            self.status_text = format!("UI state save failed: {error}").into();
        }
    }

    fn clamp_side_pane_width(width: f32) -> f32 {
        width.clamp(MIN_SIDE_PANE_WIDTH, MAX_SIDE_PANE_WIDTH)
    }

    fn set_active_pane(&mut self, pane: ActivePane, cx: &mut Context<Self>) {
        if self.active_pane != pane {
            self.active_pane = pane;
            self.status_text = format!("Focused {} pane", pane.label()).into();
            cx.notify();
        }
    }

    fn select_next_file(&mut self, cx: &mut Context<Self>) {
        if self.data.select_next_file_for_lane(self.selected_lane_id) {
            self.status_text = "Selected next file".into();
            cx.notify();
        }
    }

    fn select_previous_file(&mut self, cx: &mut Context<Self>) {
        if self.data.select_previous_file_for_lane(self.selected_lane_id) {
            self.status_text = "Selected previous file".into();
            cx.notify();
        }
    }

    fn toggle_diff_mode(&mut self, cx: &mut Context<Self>) {
        let mode = self.data.toggle_diff_mode();
        self.persist_ui_state();
        self.status_text = match mode {
            DiffViewMode::Unified => "Diff mode: unified".into(),
            DiffViewMode::Split => "Diff mode: split".into(),
        };
        cx.notify();
    }

    fn set_diff_mode(&mut self, target_mode: DiffViewMode, cx: &mut Context<Self>) {
        if self.data.diff_mode() != target_mode {
            self.data.set_diff_mode(target_mode);
            self.persist_ui_state();
            self.status_text = match target_mode {
                DiffViewMode::Unified => "Diff mode: unified".into(),
                DiffViewMode::Split => "Diff mode: split".into(),
            };
            cx.notify();
        }
    }

    fn stage_or_unstage_selected_file(&mut self, cx: &mut Context<Self>) {
        if self.data.stage_or_unstage_selected_file() {
            self.status_text = "Toggled selected file stage state".into();
            cx.notify();
        }
    }

    fn revert_selected_file(&mut self, cx: &mut Context<Self>) {
        if self.data.revert_selected_file() {
            self.data.select_file_for_lane(self.selected_lane_id);
            self.status_text = "Reverted selected file from mock list".into();
            cx.notify();
        }
    }

    fn resize_side_pane(&mut self, pane: ActivePane, delta: f32, cx: &mut Context<Self>) {
        match pane {
            ActivePane::Left => {
                self.left_pane_width = Self::clamp_side_pane_width(self.left_pane_width + delta);
            }
            ActivePane::Right => {
                self.right_pane_width = Self::clamp_side_pane_width(self.right_pane_width + delta);
            }
            ActivePane::Middle => {
                return;
            }
        }

        self.persist_ui_state();
        self.status_text = format!(
            "Resized panes: left {:.0}px / right {:.0}px",
            self.left_pane_width, self.right_pane_width
        )
        .into();
        cx.notify();
    }

    fn resize_active_side_pane(&mut self, delta: f32, cx: &mut Context<Self>) {
        match self.active_pane {
            ActivePane::Left | ActivePane::Right => {
                self.resize_side_pane(self.active_pane, delta, cx)
            }
            ActivePane::Middle => {
                self.status_text = "Activate left or right pane to resize side widths".into();
                cx.notify();
            }
        }
    }

    fn max_left_width_for_row(&self, row_width: f32) -> f32 {
        let reserved = self.right_pane_width + MIN_MIDDLE_PANE_WIDTH + (SPLITTER_TRACK_WIDTH * 2.0);
        (row_width - reserved).clamp(MIN_SIDE_PANE_WIDTH, MAX_SIDE_PANE_WIDTH)
    }

    fn max_right_width_for_row(&self, row_width: f32) -> f32 {
        let reserved = self.left_pane_width + MIN_MIDDLE_PANE_WIDTH + (SPLITTER_TRACK_WIDTH * 2.0);
        (row_width - reserved).clamp(MIN_SIDE_PANE_WIDTH, MAX_SIDE_PANE_WIDTH)
    }

    fn resize_from_left_drag(
        &mut self,
        drag_event: &DragMoveEvent<DraggedLeftPaneHandle>,
        cx: &mut Context<Self>,
    ) {
        let row_width = f32::from(drag_event.bounds.right() - drag_event.bounds.left());
        let pointer_from_left = f32::from(drag_event.event.position.x - drag_event.bounds.left());
        let max_left_width = self.max_left_width_for_row(row_width);
        let new_left = pointer_from_left.clamp(MIN_SIDE_PANE_WIDTH, max_left_width);

        if (new_left - self.left_pane_width).abs() > 0.5 {
            self.left_pane_width = Self::clamp_side_pane_width(new_left);
            self.status_text = format!(
                "Resized panes: left {:.0}px / right {:.0}px",
                self.left_pane_width, self.right_pane_width
            )
            .into();
            cx.notify();
        }
    }

    fn resize_from_right_drag(
        &mut self,
        drag_event: &DragMoveEvent<DraggedRightPaneHandle>,
        cx: &mut Context<Self>,
    ) {
        let row_width = f32::from(drag_event.bounds.right() - drag_event.bounds.left());
        let pointer_from_left = f32::from(drag_event.event.position.x - drag_event.bounds.left());
        let pointer_from_right = row_width - pointer_from_left;
        let max_right_width = self.max_right_width_for_row(row_width);
        let new_right = pointer_from_right.clamp(MIN_SIDE_PANE_WIDTH, max_right_width);

        if (new_right - self.right_pane_width).abs() > 0.5 {
            self.right_pane_width = Self::clamp_side_pane_width(new_right);
            self.status_text = format!(
                "Resized panes: left {:.0}px / right {:.0}px",
                self.left_pane_width, self.right_pane_width
            )
            .into();
            cx.notify();
        }
    }

    fn on_split_drag_end(&mut self, cx: &mut Context<Self>) {
        self.persist_ui_state();
        cx.notify();
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root_key_context = if self.composer_focus_handle.is_focused(window) {
            "middle-chat-focused"
        } else {
            KEY_CONTEXT
        };

        div()
            .id("app-shell")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(self.colors().app_bg))
            .text_color(rgb(self.colors().text_primary))
            .track_focus(&self.focus_handle)
            .key_context(root_key_context)
            .on_mouse_move(|_, window, _| window.refresh())
            .on_action(cx.listener(|this, _: &FocusLeftPane, _, cx| {
                this.set_active_pane(ActivePane::Left, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusMiddlePane, _, cx| {
                this.set_active_pane(ActivePane::Middle, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusRightPane, _, cx| {
                this.set_active_pane(ActivePane::Right, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusNextPane, _, cx| {
                this.set_active_pane(this.active_pane.next(), cx);
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousPane, _, cx| {
                this.set_active_pane(this.active_pane.previous(), cx);
            }))
            .on_action(cx.listener(|this, _: &SelectNextFile, window, cx| {
                if this.composer_focus_handle.is_focused(window) {
                    return;
                }
                this.select_next_file(cx);
            }))
            .on_action(cx.listener(|this, _: &SelectPreviousFile, window, cx| {
                if this.composer_focus_handle.is_focused(window) {
                    return;
                }
                this.select_previous_file(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleDiffMode, _, cx| {
                this.toggle_diff_mode(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleThemeMode, _, cx| {
                this.toggle_theme_mode(cx);
            }))
            .on_action(cx.listener(|this, _: &StageOrUnstageSelectedFile, _, cx| {
                this.stage_or_unstage_selected_file(cx);
            }))
            .on_action(cx.listener(|this, _: &RevertSelectedFile, _, cx| {
                this.revert_selected_file(cx);
            }))
            .on_action(cx.listener(|this, _: &IncreaseSidePaneWidth, _, cx| {
                this.resize_active_side_pane(SIDE_PANE_RESIZE_STEP, cx);
            }))
            .on_action(cx.listener(|this, _: &DecreaseSidePaneWidth, _, cx| {
                this.resize_active_side_pane(-SIDE_PANE_RESIZE_STEP, cx);
            }))
            .child(self.render_top_bar(cx))
            .child(
                div()
                    .id("pane-row")
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_row()
                    .on_drag_move::<DraggedLeftPaneHandle>(cx.listener(|this, event, _, cx| {
                        this.resize_from_left_drag(event, cx);
                    }))
                    .on_drag_move::<DraggedRightPaneHandle>(cx.listener(|this, event, _, cx| {
                        this.resize_from_right_drag(event, cx);
                    }))
                    .on_drop::<DraggedLeftPaneHandle>(cx.listener(|this, _, _, cx| {
                        this.on_split_drag_end(cx);
                    }))
                    .on_drop::<DraggedRightPaneHandle>(cx.listener(|this, _, _, cx| {
                        this.on_split_drag_end(cx);
                    }))
                    .child(self.render_agents_pane(cx))
                    .child(self.render_left_splitter())
                    .child(self.render_diff_viewer_pane(cx))
                    .child(self.render_right_splitter())
                    .child(self.render_changed_files_pane(cx)),
            )
            .child(
                div()
                    .h(px(24.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .bg(rgb(self.colors().header_bg))
                    .border_t_1()
                    .border_color(rgb(self.colors().border))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(self.colors().text_muted))
                            .child(self.status_text.clone()),
                    ),
            )
            .child(self.render_shortcut_legend())
    }
}
