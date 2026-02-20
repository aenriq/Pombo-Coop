use gpui::SharedString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StatusBadge {
    pub label: SharedString,
    pub tone: BadgeTone,
}

impl StatusBadge {
    pub fn new(label: impl Into<SharedString>, tone: BadgeTone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentLaneStatus {
    Queued,
    Running,
    Blocked,
    Failed,
    Completed,
}

impl AgentLaneStatus {
    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Completed => "completed",
        }
    }

    #[allow(dead_code)]
    pub const fn tone(self) -> BadgeTone {
        match self {
            Self::Queued => BadgeTone::Neutral,
            Self::Running => BadgeTone::Info,
            Self::Blocked => BadgeTone::Warning,
            Self::Failed => BadgeTone::Danger,
            Self::Completed => BadgeTone::Success,
        }
    }

    #[allow(dead_code)]
    pub fn badge(self) -> StatusBadge {
        StatusBadge::new(self.label(), self.tone())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AgentLane {
    pub id: u32,
    pub repo_name: SharedString,
    pub name: SharedString,
    pub role: SharedString,
    pub provider: SharedString,
    pub branch: SharedString,
    pub worktree_path: SharedString,
    pub status: AgentLaneStatus,
    pub last_action: SharedString,
    pub elapsed_seconds: u64,
    pub token_estimate: Option<u32>,
    pub cost_estimate_cents: Option<u32>,
    pub retry_count: u32,
}

impl AgentLane {
    #[allow(dead_code)]
    pub fn status_badge(&self) -> StatusBadge {
        self.status.badge()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Conflict,
}

impl FileChangeKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Conflict => "!",
        }
    }

    pub const fn tone(self) -> BadgeTone {
        match self {
            Self::Added => BadgeTone::Success,
            Self::Modified => BadgeTone::Info,
            Self::Deleted => BadgeTone::Danger,
            Self::Renamed => BadgeTone::Neutral,
            Self::Conflict => BadgeTone::Danger,
        }
    }

    #[allow(dead_code)]
    pub fn badge(self) -> StatusBadge {
        StatusBadge::new(self.code(), self.tone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStageGroup {
    Staged,
    Unstaged,
    Untracked,
    Conflict,
}

impl FileStageGroup {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Unstaged => "unstaged",
            Self::Untracked => "untracked",
            Self::Conflict => "conflict",
        }
    }

    #[allow(dead_code)]
    pub const fn tone(self) -> BadgeTone {
        match self {
            Self::Staged => BadgeTone::Success,
            Self::Unstaged => BadgeTone::Warning,
            Self::Untracked => BadgeTone::Neutral,
            Self::Conflict => BadgeTone::Danger,
        }
    }

    #[allow(dead_code)]
    pub fn badge(self) -> StatusBadge {
        StatusBadge::new(self.label(), self.tone())
    }
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub id: u32,
    pub path: SharedString,
    #[allow(dead_code)]
    pub previous_path: Option<SharedString>,
    pub kind: FileChangeKind,
    pub additions: u32,
    pub deletions: u32,
    pub stage_group: FileStageGroup,
    pub owner_lane_id: Option<u32>,
}

impl ChangedFile {
    #[allow(dead_code)]
    pub fn kind_badge(&self) -> StatusBadge {
        self.kind.badge()
    }

    #[allow(dead_code)]
    pub fn stage_badge(&self) -> StatusBadge {
        self.stage_group.badge()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffViewMode {
    Unified,
    Split,
}

impl DiffViewMode {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Unified => Self::Split,
            Self::Split => Self::Unified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

impl DiffLineKind {
    pub const fn prefix(self) -> char {
        match self {
            Self::Context => ' ',
            Self::Added => '+',
            Self::Removed => '-',
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnifiedDiffLine {
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub kind: DiffLineKind,
    pub text: SharedString,
}

#[derive(Debug, Clone)]
pub struct SplitDiffCell {
    pub line_number: Option<u32>,
    pub kind: DiffLineKind,
    pub text: SharedString,
}

#[derive(Debug, Clone)]
pub struct SplitDiffRow {
    pub left: Option<SplitDiffCell>,
    pub right: Option<SplitDiffCell>,
}

#[derive(Debug, Clone)]
pub struct DiffTextScaffold {
    pub unified: SharedString,
    pub split: SharedString,
}

impl DiffTextScaffold {
    pub fn for_mode(&self, mode: DiffViewMode) -> &SharedString {
        match mode {
            DiffViewMode::Unified => &self.unified,
            DiffViewMode::Split => &self.split,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: SharedString,
    pub unified_lines: Vec<UnifiedDiffLine>,
    pub split_rows: Vec<SplitDiffRow>,
    pub text: DiffTextScaffold,
}

pub enum DiffRows<'a> {
    Unified(&'a [UnifiedDiffLine]),
    Split(&'a [SplitDiffRow]),
}

impl DiffHunk {
    pub fn rows(&self, mode: DiffViewMode) -> DiffRows<'_> {
        match mode {
            DiffViewMode::Unified => DiffRows::Unified(&self.unified_lines),
            DiffViewMode::Split => DiffRows::Split(&self.split_rows),
        }
    }

    pub fn text_for_mode(&self, mode: DiffViewMode) -> &SharedString {
        self.text.for_mode(mode)
    }
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_id: u32,
    pub file_path: SharedString,
    pub kind: FileChangeKind,
    pub hunks: Vec<DiffHunk>,
    pub text: DiffTextScaffold,
}

impl FileDiff {
    pub fn text_for_mode(&self, mode: DiffViewMode) -> &SharedString {
        self.text.for_mode(mode)
    }
}

#[derive(Debug, Clone)]
pub struct EpicBMockData {
    agent_lanes: Vec<AgentLane>,
    changed_files: Vec<ChangedFile>,
    file_diffs: Vec<FileDiff>,
    selected_file_id: Option<u32>,
    diff_mode: DiffViewMode,
}

impl Default for EpicBMockData {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicBMockData {
    pub fn new() -> Self {
        let agent_lanes = mock_agent_lanes();
        let changed_files = mock_changed_files();
        let file_diffs = mock_file_diffs();
        let selected_file_id = changed_files.first().map(|file| file.id);

        Self {
            agent_lanes,
            changed_files,
            file_diffs,
            selected_file_id,
            diff_mode: DiffViewMode::Unified,
        }
    }

    pub fn agent_lanes(&self) -> &[AgentLane] {
        &self.agent_lanes
    }

    pub fn changed_files(&self) -> &[ChangedFile] {
        &self.changed_files
    }

    pub fn changed_files_for_lane(&self, lane_id: Option<u32>) -> Vec<&ChangedFile> {
        match lane_id {
            Some(lane_id) => self
                .changed_files
                .iter()
                .filter(|file| file.owner_lane_id == Some(lane_id))
                .collect(),
            None => self.changed_files.iter().collect(),
        }
    }

    pub fn diff_mode(&self) -> DiffViewMode {
        self.diff_mode
    }

    pub fn set_diff_mode(&mut self, mode: DiffViewMode) {
        self.diff_mode = mode;
    }

    pub fn toggle_diff_mode(&mut self) -> DiffViewMode {
        self.diff_mode = self.diff_mode.toggled();
        self.diff_mode
    }

    pub fn selected_file_id(&self) -> Option<u32> {
        self.selected_file_id
    }

    pub fn selected_file_index(&self) -> Option<usize> {
        let selected_file_id = self.selected_file_id?;
        self.changed_files
            .iter()
            .position(|file| file.id == selected_file_id)
    }

    fn lane_file_ids(&self, lane_id: Option<u32>) -> Vec<u32> {
        self.changed_files_for_lane(lane_id)
            .into_iter()
            .map(|file| file.id)
            .collect()
    }

    pub fn select_file_for_lane(&mut self, lane_id: Option<u32>) -> bool {
        let lane_file_ids = self.lane_file_ids(lane_id);
        let next_selected = if lane_file_ids.is_empty() {
            None
        } else {
            self.selected_file_id
                .filter(|selected_id| lane_file_ids.contains(selected_id))
                .or_else(|| lane_file_ids.first().copied())
        };

        let changed = self.selected_file_id != next_selected;
        self.selected_file_id = next_selected;
        changed
    }

    pub fn select_file_by_id(&mut self, file_id: u32) -> bool {
        if self.changed_files.iter().any(|file| file.id == file_id) {
            self.selected_file_id = Some(file_id);
            true
        } else {
            false
        }
    }

    pub fn select_next_file_for_lane(&mut self, lane_id: Option<u32>) -> bool {
        let lane_file_ids = self.lane_file_ids(lane_id);
        if lane_file_ids.is_empty() {
            self.selected_file_id = None;
            return false;
        }

        let next_index = match self.selected_file_id {
            Some(selected_file_id) => lane_file_ids
                .iter()
                .position(|file_id| *file_id == selected_file_id)
                .map(|index| (index + 1) % lane_file_ids.len())
                .unwrap_or(0),
            None => 0,
        };

        self.selected_file_id = Some(lane_file_ids[next_index]);
        true
    }

    pub fn select_previous_file_for_lane(&mut self, lane_id: Option<u32>) -> bool {
        let lane_file_ids = self.lane_file_ids(lane_id);
        if lane_file_ids.is_empty() {
            self.selected_file_id = None;
            return false;
        }

        let previous_index = match self.selected_file_id {
            Some(selected_file_id) => match lane_file_ids
                .iter()
                .position(|file_id| *file_id == selected_file_id)
            {
                Some(0) | None => lane_file_ids.len() - 1,
                Some(index) => index - 1,
            },
            None => lane_file_ids.len() - 1,
        };

        self.selected_file_id = Some(lane_file_ids[previous_index]);
        true
    }

    pub fn stage_or_unstage_selected_file(&mut self) -> bool {
        let Some(selected_index) = self.selected_file_index() else {
            return false;
        };

        let file = &mut self.changed_files[selected_index];
        file.stage_group = match file.stage_group {
            FileStageGroup::Staged => FileStageGroup::Unstaged,
            FileStageGroup::Unstaged => FileStageGroup::Staged,
            FileStageGroup::Untracked => FileStageGroup::Staged,
            FileStageGroup::Conflict => FileStageGroup::Staged,
        };

        true
    }

    pub fn revert_selected_file(&mut self) -> bool {
        let Some(selected_index) = self.selected_file_index() else {
            return false;
        };

        let reverted_file = self.changed_files.remove(selected_index);
        self.file_diffs
            .retain(|diff| diff.file_id != reverted_file.id);

        if self.changed_files.is_empty() {
            self.selected_file_id = None;
        } else {
            let next_index = selected_index.min(self.changed_files.len() - 1);
            self.selected_file_id = Some(self.changed_files[next_index].id);
        }

        true
    }

    pub fn selected_file(&self) -> Option<&ChangedFile> {
        self.selected_file_id
            .and_then(|id| self.changed_files.iter().find(|file| file.id == id))
    }

    pub fn selected_diff(&self) -> Option<&FileDiff> {
        self.selected_file_id.and_then(|id| self.diff_for_file(id))
    }

    pub fn diff_for_file(&self, file_id: u32) -> Option<&FileDiff> {
        self.file_diffs.iter().find(|diff| diff.file_id == file_id)
    }
}

pub fn epic_b_mock_data() -> EpicBMockData {
    EpicBMockData::new()
}

pub fn mock_agent_lanes() -> Vec<AgentLane> {
    vec![
        AgentLane {
            id: 1,
            repo_name: "conductor".into(),
            name: "Planner".into(),
            role: "Implements three-pane shell structure".into(),
            provider: "Codex (GPT-5)".into(),
            branch: "codex/epic-b-shell".into(),
            worktree_path: "/tmp/agent-manager/worktrees/planner".into(),
            status: AgentLaneStatus::Running,
            last_action: "Wired pane resize interactions".into(),
            elapsed_seconds: 932,
            token_estimate: Some(12_300),
            cost_estimate_cents: Some(24),
            retry_count: 0,
        },
        AgentLane {
            id: 2,
            repo_name: "conductor".into(),
            name: "Reviewer".into(),
            role: "Builds diff viewer scaffold and key bindings".into(),
            provider: "Claude Code".into(),
            branch: "codex/epic-b-diff".into(),
            worktree_path: "/tmp/agent-manager/worktrees/reviewer".into(),
            status: AgentLaneStatus::Blocked,
            last_action: "Waiting on split layout API decision".into(),
            elapsed_seconds: 478,
            token_estimate: Some(8_140),
            cost_estimate_cents: Some(17),
            retry_count: 1,
        },
        AgentLane {
            id: 3,
            repo_name: "conductor".into(),
            name: "Schema Sync".into(),
            role: "Aligns UI models with orchestrator state schema".into(),
            provider: "Codex (GPT-5)".into(),
            branch: "codex/epic-b-schema-sync".into(),
            worktree_path: "/tmp/agent-manager/worktrees/schema-sync".into(),
            status: AgentLaneStatus::Queued,
            last_action: "Queued behind reviewer lane".into(),
            elapsed_seconds: 88,
            token_estimate: None,
            cost_estimate_cents: None,
            retry_count: 0,
        },
        AgentLane {
            id: 4,
            repo_name: "melty_home".into(),
            name: "Regression Sweep".into(),
            role: "Runs shell smoke tests after UI changes".into(),
            provider: "Local Adapter".into(),
            branch: "codex/epic-b-regression".into(),
            worktree_path: "/tmp/agent-manager/worktrees/regression".into(),
            status: AgentLaneStatus::Failed,
            last_action: "GPUI render panic during diff toggle".into(),
            elapsed_seconds: 311,
            token_estimate: Some(1_020),
            cost_estimate_cents: Some(2),
            retry_count: 2,
        },
        AgentLane {
            id: 5,
            repo_name: "conductor_docs".into(),
            name: "Docs Polish".into(),
            role: "Updates architecture docs for three-pane shell".into(),
            provider: "Codex (GPT-5)".into(),
            branch: "codex/epic-b-docs".into(),
            worktree_path: "/tmp/agent-manager/worktrees/docs".into(),
            status: AgentLaneStatus::Completed,
            last_action: "Ready for review".into(),
            elapsed_seconds: 184,
            token_estimate: Some(2_770),
            cost_estimate_cents: Some(5),
            retry_count: 0,
        },
    ]
}

pub fn mock_changed_files() -> Vec<ChangedFile> {
    vec![
        ChangedFile {
            id: 1001,
            path: "core/orchestrator/src/runtime.rs".into(),
            previous_path: None,
            kind: FileChangeKind::Modified,
            additions: 24,
            deletions: 9,
            stage_group: FileStageGroup::Unstaged,
            owner_lane_id: Some(1),
        },
        ChangedFile {
            id: 1002,
            path: "ui/gpui_app/src/app_shell.rs".into(),
            previous_path: None,
            kind: FileChangeKind::Added,
            additions: 162,
            deletions: 0,
            stage_group: FileStageGroup::Untracked,
            owner_lane_id: Some(2),
        },
        ChangedFile {
            id: 1003,
            path: "shared/schema/src/lib.rs".into(),
            previous_path: None,
            kind: FileChangeKind::Modified,
            additions: 18,
            deletions: 4,
            stage_group: FileStageGroup::Staged,
            owner_lane_id: Some(3),
        },
        ChangedFile {
            id: 1004,
            path: "docs/architecture.md".into(),
            previous_path: Some("docs/adr-shell.md".into()),
            kind: FileChangeKind::Renamed,
            additions: 12,
            deletions: 11,
            stage_group: FileStageGroup::Staged,
            owner_lane_id: Some(5),
        },
        ChangedFile {
            id: 1005,
            path: "core/orchestrator/src/worker_pool.rs".into(),
            previous_path: None,
            kind: FileChangeKind::Conflict,
            additions: 21,
            deletions: 14,
            stage_group: FileStageGroup::Conflict,
            owner_lane_id: Some(2),
        },
        ChangedFile {
            id: 1006,
            path: "ui/gpui_app/src/legacy_panel.rs".into(),
            previous_path: None,
            kind: FileChangeKind::Deleted,
            additions: 0,
            deletions: 73,
            stage_group: FileStageGroup::Unstaged,
            owner_lane_id: Some(4),
        },
    ]
}

pub fn mock_file_diffs() -> Vec<FileDiff> {
    vec![
        FileDiff {
            file_id: 1001,
            file_path: "core/orchestrator/src/runtime.rs".into(),
            kind: FileChangeKind::Modified,
            hunks: vec![DiffHunk {
                header: "@@ -42,8 +42,12 @@ impl Scheduler {".into(),
                unified_lines: vec![
                    unified_line(
                        Some(42),
                        Some(42),
                        DiffLineKind::Context,
                        "    fn enqueue(&mut self, run: RunRequest) {",
                    ),
                    unified_line(
                        Some(43),
                        None,
                        DiffLineKind::Removed,
                        "        self.queue.push_back(run);",
                    ),
                    unified_line(
                        None,
                        Some(43),
                        DiffLineKind::Added,
                        "        let run_id = run.id.clone();",
                    ),
                    unified_line(
                        None,
                        Some(44),
                        DiffLineKind::Added,
                        "        self.queue.push_back(run);",
                    ),
                    unified_line(
                        None,
                        Some(45),
                        DiffLineKind::Added,
                        "        self.metrics.mark_enqueued(&run_id);",
                    ),
                    unified_line(Some(44), Some(46), DiffLineKind::Context, "    }"),
                ],
                split_rows: vec![
                    split_row(
                        Some(split_cell(
                            Some(42),
                            DiffLineKind::Context,
                            "    fn enqueue(&mut self, run: RunRequest) {",
                        )),
                        Some(split_cell(
                            Some(42),
                            DiffLineKind::Context,
                            "    fn enqueue(&mut self, run: RunRequest) {",
                        )),
                    ),
                    split_row(
                        Some(split_cell(
                            Some(43),
                            DiffLineKind::Removed,
                            "        self.queue.push_back(run);",
                        )),
                        Some(split_cell(
                            Some(43),
                            DiffLineKind::Added,
                            "        let run_id = run.id.clone();",
                        )),
                    ),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(44),
                            DiffLineKind::Added,
                            "        self.queue.push_back(run);",
                        )),
                    ),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(45),
                            DiffLineKind::Added,
                            "        self.metrics.mark_enqueued(&run_id);",
                        )),
                    ),
                    split_row(
                        Some(split_cell(Some(44), DiffLineKind::Context, "    }")),
                        Some(split_cell(Some(46), DiffLineKind::Context, "    }")),
                    ),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -42,8 +42,12 @@ impl Scheduler {\n     fn enqueue(&mut self, run: RunRequest) {\n-        self.queue.push_back(run);\n+        let run_id = run.id.clone();\n+        self.queue.push_back(run);\n+        self.metrics.mark_enqueued(&run_id);\n     }".into(),
                    split: "@@ -42,8 +42,12 @@ impl Scheduler {\n42 |     fn enqueue(&mut self, run: RunRequest) { || 42 |     fn enqueue(&mut self, run: RunRequest) {\n43 |         self.queue.push_back(run);         || 43 |         let run_id = run.id.clone();\n   |                                           || 44 |         self.queue.push_back(run);\n   |                                           || 45 |         self.metrics.mark_enqueued(&run_id);\n44 |     }                                     || 46 |     }".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -42,8 +42,12 @@ impl Scheduler {\n     fn enqueue(&mut self, run: RunRequest) {\n-        self.queue.push_back(run);\n+        let run_id = run.id.clone();\n+        self.queue.push_back(run);\n+        self.metrics.mark_enqueued(&run_id);\n     }".into(),
                split: "@@ -42,8 +42,12 @@ impl Scheduler {\n42 |     fn enqueue(&mut self, run: RunRequest) { || 42 |     fn enqueue(&mut self, run: RunRequest) {\n43 |         self.queue.push_back(run);         || 43 |         let run_id = run.id.clone();\n   |                                           || 44 |         self.queue.push_back(run);\n   |                                           || 45 |         self.metrics.mark_enqueued(&run_id);\n44 |     }                                     || 46 |     }".into(),
            },
        },
        FileDiff {
            file_id: 1002,
            file_path: "ui/gpui_app/src/app_shell.rs".into(),
            kind: FileChangeKind::Added,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,6 @@".into(),
                unified_lines: vec![
                    unified_line(None, Some(1), DiffLineKind::Added, "use gpui::*;"),
                    unified_line(None, Some(2), DiffLineKind::Added, ""),
                    unified_line(
                        None,
                        Some(3),
                        DiffLineKind::Added,
                        "pub struct AppShellState {",
                    ),
                    unified_line(
                        None,
                        Some(4),
                        DiffLineKind::Added,
                        "    pub selected_file: Option<u32>,",
                    ),
                    unified_line(None, Some(5), DiffLineKind::Added, "}"),
                    unified_line(None, Some(6), DiffLineKind::Added, ""),
                ],
                split_rows: vec![
                    split_row(None, Some(split_cell(Some(1), DiffLineKind::Added, "use gpui::*;"))),
                    split_row(None, Some(split_cell(Some(2), DiffLineKind::Added, ""))),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(3),
                            DiffLineKind::Added,
                            "pub struct AppShellState {",
                        )),
                    ),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(4),
                            DiffLineKind::Added,
                            "    pub selected_file: Option<u32>,",
                        )),
                    ),
                    split_row(None, Some(split_cell(Some(5), DiffLineKind::Added, "}"))),
                    split_row(None, Some(split_cell(Some(6), DiffLineKind::Added, ""))),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -0,0 +1,6 @@\n+use gpui::*;\n+\n+pub struct AppShellState {\n+    pub selected_file: Option<u32>,\n+}\n+".into(),
                    split: "@@ -0,0 +1,6 @@\n   |                              || 1 | use gpui::*;\n   |                              || 2 |\n   |                              || 3 | pub struct AppShellState {\n   |                              || 4 |     pub selected_file: Option<u32>,\n   |                              || 5 | }\n   |                              || 6 |".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -0,0 +1,6 @@\n+use gpui::*;\n+\n+pub struct AppShellState {\n+    pub selected_file: Option<u32>,\n+}\n+".into(),
                split: "@@ -0,0 +1,6 @@\n   |                              || 1 | use gpui::*;\n   |                              || 2 |\n   |                              || 3 | pub struct AppShellState {\n   |                              || 4 |     pub selected_file: Option<u32>,\n   |                              || 5 | }\n   |                              || 6 |".into(),
            },
        },
        FileDiff {
            file_id: 1003,
            file_path: "shared/schema/src/lib.rs".into(),
            kind: FileChangeKind::Modified,
            hunks: vec![DiffHunk {
                header: "@@ -20,6 +20,9 @@ pub enum RunState {".into(),
                unified_lines: vec![
                    unified_line(Some(20), Some(20), DiffLineKind::Context, " pub enum RunState {"),
                    unified_line(Some(21), Some(21), DiffLineKind::Context, "     Pending,"),
                    unified_line(None, Some(22), DiffLineKind::Added, "     Blocked,"),
                    unified_line(Some(22), Some(23), DiffLineKind::Context, "     Queued,"),
                    unified_line(Some(23), Some(24), DiffLineKind::Context, "     Running,"),
                    unified_line(None, Some(25), DiffLineKind::Added, "     Completed,"),
                ],
                split_rows: vec![
                    split_row(
                        Some(split_cell(Some(20), DiffLineKind::Context, " pub enum RunState {")),
                        Some(split_cell(Some(20), DiffLineKind::Context, " pub enum RunState {")),
                    ),
                    split_row(
                        Some(split_cell(Some(21), DiffLineKind::Context, "     Pending,")),
                        Some(split_cell(Some(21), DiffLineKind::Context, "     Pending,")),
                    ),
                    split_row(
                        None,
                        Some(split_cell(Some(22), DiffLineKind::Added, "     Blocked,")),
                    ),
                    split_row(
                        Some(split_cell(Some(22), DiffLineKind::Context, "     Queued,")),
                        Some(split_cell(Some(23), DiffLineKind::Context, "     Queued,")),
                    ),
                    split_row(
                        Some(split_cell(Some(23), DiffLineKind::Context, "     Running,")),
                        Some(split_cell(Some(24), DiffLineKind::Context, "     Running,")),
                    ),
                    split_row(
                        None,
                        Some(split_cell(Some(25), DiffLineKind::Added, "     Completed,")),
                    ),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -20,6 +20,9 @@ pub enum RunState {\n  pub enum RunState {\n      Pending,\n+     Blocked,\n      Queued,\n      Running,\n+     Completed,".into(),
                    split: "@@ -20,6 +20,9 @@ pub enum RunState {\n20 |  pub enum RunState { || 20 |  pub enum RunState {\n21 |      Pending,        || 21 |      Pending,\n   |                       || 22 |      Blocked,\n22 |      Queued,         || 23 |      Queued,\n23 |      Running,        || 24 |      Running,\n   |                       || 25 |      Completed,".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -20,6 +20,9 @@ pub enum RunState {\n  pub enum RunState {\n      Pending,\n+     Blocked,\n      Queued,\n      Running,\n+     Completed,".into(),
                split: "@@ -20,6 +20,9 @@ pub enum RunState {\n20 |  pub enum RunState { || 20 |  pub enum RunState {\n21 |      Pending,        || 21 |      Pending,\n   |                       || 22 |      Blocked,\n22 |      Queued,         || 23 |      Queued,\n23 |      Running,        || 24 |      Running,\n   |                       || 25 |      Completed,".into(),
            },
        },
        FileDiff {
            file_id: 1004,
            file_path: "docs/architecture.md".into(),
            kind: FileChangeKind::Renamed,
            hunks: vec![DiffHunk {
                header: "@@ -1,4 +1,4 @@".into(),
                unified_lines: vec![
                    unified_line(Some(1), Some(1), DiffLineKind::Context, "# Architecture Decision A-001: App Shell"),
                    unified_line(Some(2), Some(2), DiffLineKind::Removed, "- Status: Proposed"),
                    unified_line(Some(3), Some(2), DiffLineKind::Added, "- Status: Accepted"),
                    unified_line(Some(4), Some(3), DiffLineKind::Added, "- Date: 2026-02-20"),
                ],
                split_rows: vec![
                    split_row(
                        Some(split_cell(
                            Some(1),
                            DiffLineKind::Context,
                            "# Architecture Decision A-001: App Shell",
                        )),
                        Some(split_cell(
                            Some(1),
                            DiffLineKind::Context,
                            "# Architecture Decision A-001: App Shell",
                        )),
                    ),
                    split_row(
                        Some(split_cell(
                            Some(2),
                            DiffLineKind::Removed,
                            "- Status: Proposed",
                        )),
                        Some(split_cell(
                            Some(2),
                            DiffLineKind::Added,
                            "- Status: Accepted",
                        )),
                    ),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(3),
                            DiffLineKind::Added,
                            "- Date: 2026-02-20",
                        )),
                    ),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -1,4 +1,4 @@\n # Architecture Decision A-001: App Shell\n-- Status: Proposed\n+- Status: Accepted\n+- Date: 2026-02-20".into(),
                    split: "@@ -1,4 +1,4 @@\n1 | # Architecture Decision A-001: App Shell || 1 | # Architecture Decision A-001: App Shell\n2 | - Status: Proposed                        || 2 | - Status: Accepted\n  |                                           || 3 | - Date: 2026-02-20".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -1,4 +1,4 @@\n # Architecture Decision A-001: App Shell\n-- Status: Proposed\n+- Status: Accepted\n+- Date: 2026-02-20".into(),
                split: "@@ -1,4 +1,4 @@\n1 | # Architecture Decision A-001: App Shell || 1 | # Architecture Decision A-001: App Shell\n2 | - Status: Proposed                        || 2 | - Status: Accepted\n  |                                           || 3 | - Date: 2026-02-20".into(),
            },
        },
        FileDiff {
            file_id: 1005,
            file_path: "core/orchestrator/src/worker_pool.rs".into(),
            kind: FileChangeKind::Conflict,
            hunks: vec![DiffHunk {
                header: "@@ -67,7 +67,11 @@ fn spawn_workers(...)".into(),
                unified_lines: vec![
                    unified_line(Some(67), Some(67), DiffLineKind::Context, "<<<<<<< ours"),
                    unified_line(
                        Some(68),
                        None,
                        DiffLineKind::Removed,
                        "let max_parallel = config.max_parallel_runs;",
                    ),
                    unified_line(None, Some(68), DiffLineKind::Added, "let max_parallel = 4;"),
                    unified_line(Some(69), Some(69), DiffLineKind::Context, "======="),
                    unified_line(
                        None,
                        Some(70),
                        DiffLineKind::Added,
                        "let max_parallel = runtime_limit();",
                    ),
                    unified_line(Some(70), Some(71), DiffLineKind::Context, ">>>>>>> theirs"),
                ],
                split_rows: vec![
                    split_row(
                        Some(split_cell(Some(67), DiffLineKind::Context, "<<<<<<< ours")),
                        Some(split_cell(Some(67), DiffLineKind::Context, "<<<<<<< ours")),
                    ),
                    split_row(
                        Some(split_cell(
                            Some(68),
                            DiffLineKind::Removed,
                            "let max_parallel = config.max_parallel_runs;",
                        )),
                        Some(split_cell(
                            Some(68),
                            DiffLineKind::Added,
                            "let max_parallel = 4;",
                        )),
                    ),
                    split_row(
                        Some(split_cell(Some(69), DiffLineKind::Context, "=======")),
                        Some(split_cell(Some(69), DiffLineKind::Context, "=======")),
                    ),
                    split_row(
                        None,
                        Some(split_cell(
                            Some(70),
                            DiffLineKind::Added,
                            "let max_parallel = runtime_limit();",
                        )),
                    ),
                    split_row(
                        Some(split_cell(Some(70), DiffLineKind::Context, ">>>>>>> theirs")),
                        Some(split_cell(Some(71), DiffLineKind::Context, ">>>>>>> theirs")),
                    ),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -67,7 +67,11 @@ fn spawn_workers(...)\n <<<<<<< ours\n-let max_parallel = config.max_parallel_runs;\n+let max_parallel = 4;\n =======\n+let max_parallel = runtime_limit();\n >>>>>>> theirs".into(),
                    split: "@@ -67,7 +67,11 @@ fn spawn_workers(...)\n67 | <<<<<<< ours                           || 67 | <<<<<<< ours\n68 | let max_parallel = config.max_parallel_runs; || 68 | let max_parallel = 4;\n69 | =======                                 || 69 | =======\n   |                                          || 70 | let max_parallel = runtime_limit();\n70 | >>>>>>> theirs                          || 71 | >>>>>>> theirs".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -67,7 +67,11 @@ fn spawn_workers(...)\n <<<<<<< ours\n-let max_parallel = config.max_parallel_runs;\n+let max_parallel = 4;\n =======\n+let max_parallel = runtime_limit();\n >>>>>>> theirs".into(),
                split: "@@ -67,7 +67,11 @@ fn spawn_workers(...)\n67 | <<<<<<< ours                           || 67 | <<<<<<< ours\n68 | let max_parallel = config.max_parallel_runs; || 68 | let max_parallel = 4;\n69 | =======                                 || 69 | =======\n   |                                          || 70 | let max_parallel = runtime_limit();\n70 | >>>>>>> theirs                          || 71 | >>>>>>> theirs".into(),
            },
        },
        FileDiff {
            file_id: 1006,
            file_path: "ui/gpui_app/src/legacy_panel.rs".into(),
            kind: FileChangeKind::Deleted,
            hunks: vec![DiffHunk {
                header: "@@ -1,5 +0,0 @@".into(),
                unified_lines: vec![
                    unified_line(Some(1), None, DiffLineKind::Removed, "use gpui::*;"),
                    unified_line(
                        Some(2),
                        None,
                        DiffLineKind::Removed,
                        "pub fn render_legacy_panel() -> impl IntoElement {",
                    ),
                    unified_line(
                        Some(3),
                        None,
                        DiffLineKind::Removed,
                        "    div().child(\"Legacy panel\")",
                    ),
                    unified_line(Some(4), None, DiffLineKind::Removed, "}"),
                    unified_line(Some(5), None, DiffLineKind::Removed, ""),
                ],
                split_rows: vec![
                    split_row(
                        Some(split_cell(Some(1), DiffLineKind::Removed, "use gpui::*;")),
                        None,
                    ),
                    split_row(
                        Some(split_cell(
                            Some(2),
                            DiffLineKind::Removed,
                            "pub fn render_legacy_panel() -> impl IntoElement {",
                        )),
                        None,
                    ),
                    split_row(
                        Some(split_cell(
                            Some(3),
                            DiffLineKind::Removed,
                            "    div().child(\"Legacy panel\")",
                        )),
                        None,
                    ),
                    split_row(Some(split_cell(Some(4), DiffLineKind::Removed, "}")), None),
                    split_row(Some(split_cell(Some(5), DiffLineKind::Removed, "")), None),
                ],
                text: DiffTextScaffold {
                    unified: "@@ -1,5 +0,0 @@\n-use gpui::*;\n-pub fn render_legacy_panel() -> impl IntoElement {\n-    div().child(\"Legacy panel\")\n-}\n-".into(),
                    split: "@@ -1,5 +0,0 @@\n1 | use gpui::*;                                  ||\n2 | pub fn render_legacy_panel() -> impl IntoElement { ||\n3 |     div().child(\"Legacy panel\")               ||\n4 | }                                               ||\n5 |                                                 ||".into(),
                },
            }],
            text: DiffTextScaffold {
                unified: "@@ -1,5 +0,0 @@\n-use gpui::*;\n-pub fn render_legacy_panel() -> impl IntoElement {\n-    div().child(\"Legacy panel\")\n-}\n-".into(),
                split: "@@ -1,5 +0,0 @@\n1 | use gpui::*;                                  ||\n2 | pub fn render_legacy_panel() -> impl IntoElement { ||\n3 |     div().child(\"Legacy panel\")               ||\n4 | }                                               ||\n5 |                                                 ||".into(),
            },
        },
    ]
}

fn unified_line(
    old_line_number: Option<u32>,
    new_line_number: Option<u32>,
    kind: DiffLineKind,
    text: impl Into<SharedString>,
) -> UnifiedDiffLine {
    UnifiedDiffLine {
        old_line_number,
        new_line_number,
        kind,
        text: text.into(),
    }
}

fn split_cell(
    line_number: Option<u32>,
    kind: DiffLineKind,
    text: impl Into<SharedString>,
) -> SplitDiffCell {
    SplitDiffCell {
        line_number,
        kind,
        text: text.into(),
    }
}

fn split_row(left: Option<SplitDiffCell>, right: Option<SplitDiffCell>) -> SplitDiffRow {
    SplitDiffRow { left, right }
}
