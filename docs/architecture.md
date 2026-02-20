# Architecture Decision A-001: App Shell

- Status: Accepted
- Date: 2026-02-20
- Scope: EPIC A project foundation

## Decision
Use `Tauri 2 + Rust core + React/TypeScript UI` as the default shell architecture.

## Context
- V1 needs a desktop UI for multi-agent orchestration, diff review, and publish flows.
- Runtime-critical orchestration and git operations are Rust-first.
- MVP should ship across macOS, Windows, and Linux from one codebase.

## Options considered
1. `Tauri + Rust + React` (cross-platform default)
2. `SwiftUI + Rust core` (macOS-first native shell)

## Rationale
- Cross-platform delivery without maintaining separate native shells.
- Keeps orchestration logic centralized in Rust.
- Lower process and memory overhead than Electron-class shells.
- Faster iteration for complex review UI using mature web tooling.

## Consequences
- UI layer uses web technologies instead of fully native widgets.
- Desktop integration uses Tauri commands/events between UI and Rust.
- If the product scope becomes macOS-only, revisit SwiftUI in a follow-up ADR.

## Implementation note
- Current repository bootstrap code may temporarily differ while EPIC A/B scaffolding is in progress.
- New shell-facing work should target the Tauri + Rust + React direction from this decision.
