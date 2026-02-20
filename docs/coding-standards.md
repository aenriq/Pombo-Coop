# Coding Standards (EPIC A-003)

## Toolchain
- Rust channel: `stable` (local and CI).
- Run all commands from the repository root.

## Formatting
- Apply: `cargo fmt --all`
- Verify: `cargo fmt --all -- --check`
- Rule: formatting is required; no manual style exceptions.

## Linting
- Run: `cargo clippy --workspace --all-targets -- -D warnings`
- Rule: clippy warnings fail CI.

## Testing
- Run: `cargo test --workspace --all-targets`
- Add or update tests when behavior changes.
- Keep tests deterministic and offline-safe.

## Build verification
- Run: `cargo build --workspace --all-targets`
- Every change must compile cleanly before merge.

## Recommended pre-PR check sequence
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace --all-targets
```
