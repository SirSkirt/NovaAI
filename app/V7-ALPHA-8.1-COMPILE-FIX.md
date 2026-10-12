# Nova V7.0.0-alpha.8.1 — Pass 8 Compile Fix

This maintenance release fixes two Rust compiler errors reported by the alpha 8 build without changing Pass 8 architecture.

## Fixes

1. `src-tauri/src/nova/internal.rs` constructed an internal-goal stable ID with `format!("{}|{}|{}", signature, revision)`. The identity contains two values, so the format is now `format!("{}|{}", signature, revision)`.
2. `internal.rs` imported `super::unix_ms`, which does not exist at the `nova` module root. It now imports the existing authoritative utility from `super::mind::unix_ms`. No duplicate clock helper or wrapper was introduced.

## Architecture

No authority boundaries changed. Pass 8 remains Rust-authoritative for internal goals/free thought, conversation teaching-event classification, response realization, and terminal-state ownership.
