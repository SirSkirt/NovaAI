# Nova V7 Pass 9 Validation

Static validation performed in the packaging environment:

- all JavaScript files pass `node --check`;
- package/Tauri JSON and Cargo TOML parse;
- native IPC no longer exposes raw state-file commands or JS-driven brain-state import/sync commands;
- native turn input sends no JavaScript-selected conversation context;
- native startup does not run legacy JavaScript cognitive maintenance;
- Brain Inspector derives native cognitive views directly from the Rust snapshot;
- native checkpoints provide only opaque UI state; Rust snapshots and serializes `NovaMind` itself;
- queued, awaited, visibility/page-hide saves share one serialized write chain;
- Pass 9 migration reads legacy Rust mirrors in Rust and writes the new envelope once;
- no invalid leading-dot Rust float literals are present;
- changed Rust source passed delimiter/static integrity checks;
- no merge markers or temporary packaging artifacts are present.

The current environment does not contain `cargo`/`rustc`, so the real Tauri/GitHub build remains the authoritative compiler validation. Pass 9 was reviewed specifically for the source-language errors previously caught by Grandpa Rust: format-arity mistakes, incorrect module imports, moved ownership, invalid numeric syntax and test trait requirements.

- Native begin-turn command DTO exposes only `text` and `sourceLabel`.
- Native finish-turn DTO exposes only `turnId` and `responseProduced`; terminal/reason are Rust-owned.
- No raw `nova_state_*` or per-subsystem mind mutation command is present in the native invoke surface or JavaScript native invoke strings.
- Both native turn DTOs use serde unknown-field rejection so retired brain-shaped frontend fields cannot be silently accepted.
