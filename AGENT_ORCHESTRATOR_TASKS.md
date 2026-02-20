# Agent Orchestrator V1 Tasks

Last updated: 2026-02-20

## 1) Scope
Build a lightweight desktop app with:
- Left pane: parallel agent lanes
- Middle pane: git diff reviewer
- Right pane: changed files list
- Per-agent git worktrees
- Commit + PR flow via GitHub SDK/API

## 2) Delivery approach
- Track work in vertical slices.
- Keep process count bounded and measurable from day one.
- Ship MVP before advanced automation.

## 3) MVP success criteria
- Users can run 3 agents in parallel without UI lockups.
- Users can review staged/unstaged diffs file-by-file and hunk-by-hunk.
- Users can commit selected changes and open a PR from the app.
- App runs with no always-on Python process.

## 4) Task backlog

### EPIC A: Project foundation
- [x] `A-001` Choose shell architecture: `Tauri + Rust + React` (default) or `SwiftUI + Rust core`.
- [x] `A-002` Initialize repo structure (`ui/`, `core/`, `shared/`, `docs/`).
- [x] `A-003` Add coding standards, linting, formatting, and CI checks.
- [x] `A-004` Define typed event schema for orchestrator <-> UI communication.
- [x] `A-005` Add local config system for providers, paths, and concurrency limits.

Definition of done:
- One command starts app in dev mode.
- CI validates build + lint + unit tests.

### EPIC B: Three-pane UI shell
- [ ] `B-001` Implement app shell layout with resizable panes.
- [ ] `B-002` Left pane agent lane list with status badges.
- [ ] `B-003` Middle pane diff viewer scaffold (split/unified toggle).
- [ ] `B-004` Right pane changed files list scaffold.
- [ ] `B-005` Keyboard shortcuts: file navigation, stage/unstage, revert.
- [ ] `B-006` Persist pane sizes and last review mode.

Definition of done:
- Static shell works end-to-end with mock data.
- Navigation between panes is smooth and keyboard accessible.

### EPIC C: Agent orchestration runtime
- [ ] `C-001` Build Rust orchestrator service with bounded worker pool.
- [ ] `C-002` Define agent run state machine (`queued/running/blocked/failed/completed`).
- [ ] `C-003` Implement run lifecycle APIs (`start`, `stop`, `resume`, `retry`).
- [ ] `C-004` Add provider adapter interface and shared run contract.
- [ ] `C-005` Implement `CodexAdapter`.
- [ ] `C-006` Implement `ClaudeCodeAdapter`.
- [ ] `C-007` Implement generic API-key adapter.
- [ ] `C-008` Stream agent events to UI via local IPC/WebSocket.

Definition of done:
- Three concurrent runs work reliably.
- Worker pool enforces max concurrency setting.

### EPIC D: Worktree management
- [ ] `D-001` Implement worktree create/list/remove operations.
- [ ] `D-002` Enforce branch uniqueness across active worktrees.
- [ ] `D-003` Add deterministic branch naming policy.
- [ ] `D-004` Add cleanup policy with retention and `pin worktree` support.
- [ ] `D-005` Add safety checks before commit/push.
- [ ] `D-006` Add recovery flow for orphaned/stale worktrees.

Definition of done:
- Each agent run is isolated in its own worktree.
- No branch collision in parallel runs.

### EPIC E: Git diff review engine
- [ ] `E-001` Build git diff service (`working`, `branch-vs-base`, `last-turn`).
- [ ] `E-002` Parse and expose file-level change metadata (`A/M/D/R`, +/-, conflicts).
- [ ] `E-003` Implement middle-pane diff rendering with lazy hunk loading.
- [ ] `E-004` Implement right-pane file list with filtering and quick-jump.
- [ ] `E-005` Implement hunk/file stage and unstage actions.
- [ ] `E-006` Implement hunk/file revert with confirmations.
- [ ] `E-007` Add inline review comments persisted against file+line+diff hash.
- [ ] `E-008` Add file review statuses (`approved`, `needs changes`, `unresolved`).

Definition of done:
- Reviewer can curate diff entirely in-app without external git UI.

### EPIC F: Commit and GitHub PR publish (SDK/API)
- [ ] `F-001` Add commit message generation/edit flow.
- [ ] `F-002` Implement commit creation from staged selection.
- [ ] `F-003` Implement push branch flow with clear errors.
- [ ] `F-004` Integrate GitHub auth (GitHub App preferred, PAT fallback).
- [ ] `F-005` Implement create PR endpoint (draft and ready modes).
- [ ] `F-006` Implement update PR metadata (title/body/labels/reviewers).
- [ ] `F-007` Add publish history tracking with links.
- [ ] `F-008` Add retry-safe idempotency keys for publish actions.

Definition of done:
- User can commit and open PR from app for a selected worktree branch.

### EPIC G: Auth and secrets
- [ ] `G-001` Provider connection UI: Codex, Claude, API key.
- [ ] `G-002` Credential verification on save.
- [ ] `G-003` Store secrets in OS keychain/secure store.
- [ ] `G-004` Redact secrets from logs and telemetry.
- [ ] `G-005` Add revoke/re-auth flows.

Definition of done:
- No plaintext secrets in local DB, UI storage, or logs.

### EPIC H: Performance and resource guardrails
- [ ] `H-001` Add process budget telemetry (count, CPU, memory).
- [ ] `H-002` Enforce default concurrency limit (target: 3 agents).
- [ ] `H-003` Add idle-time shutdown for worker subprocesses.
- [ ] `H-004` Virtualize changed-files list for large diffs.
- [ ] `H-005` Add diff cache with invalidation by git hash.
- [ ] `H-006` Load-test with large repos and collect baseline metrics.

Definition of done:
- App stays within defined resource envelope during default usage.

### EPIC I: Reviewer workflow enhancements
- [ ] `I-001` Add reviewer checklist widget in middle pane.
- [ ] `I-002` Add “send feedback to agent” action from selected findings.
- [ ] `I-003` Add reviewer-agent optional mode (read-only analysis).
- [ ] `I-004` Add severity classification (`blocker/high/medium/low`).

Definition of done:
- Human reviewer can convert findings into actionable agent feedback loops.

### EPIC J: Reliability, testing, and release
- [ ] `J-001` Unit tests for orchestrator state machine and worktree policies.
- [ ] `J-002` Integration tests for git operations and diff actions.
- [ ] `J-003` End-to-end test: multi-agent run -> review -> commit -> PR.
- [ ] `J-004` Failure injection tests (auth failure, network timeout, git conflict).
- [ ] `J-005` Crash recovery test for interrupted runs.
- [ ] `J-006` Packaging + signing pipeline for target platforms.

Definition of done:
- Reproducible release build and stable E2E path for core workflow.

## 5) Recommended implementation order
1. `A` Foundation
2. `B` Three-pane shell
3. `C` Runtime
4. `D` Worktrees
5. `E` Diff + changed files
6. `F` Commit + PR publish
7. `G` Auth + secrets
8. `H` Performance hardening
9. `J` Testing + release
10. `I` Reviewer automation improvements

## 6) Suggested initial milestones
- `M1` Shell + mock review UX (`A`, `B`)
- `M2` Real agent runs with isolated worktrees (`C`, `D`)
- `M3` End-to-end review curation in app (`E`)
- `M4` Commit + PR publish from app (`F`, `G`)
- `M5` Resource/perf hardening and release readiness (`H`, `J`)

## 7) Open decisions
- Confirm final shell: Tauri cross-platform vs SwiftUI macOS-first.
- Confirm GitHub auth strategy for MVP: GitHub App only or App + PAT fallback.
- Confirm whether reviewer-agent mode ships in MVP or phase 2.

## 8) Immediate next tasks (this week)
- [x] `NOW-001` Finalize shell decision and bootstrap project.
- [ ] `NOW-002` Implement three-pane layout with mock data.
- [x] `NOW-003` Build Rust orchestrator skeleton and event schema.
- [ ] `NOW-004` Implement worktree create/list/remove commands.
- [ ] `NOW-005` Implement right-pane changed-files list from real git diff.
