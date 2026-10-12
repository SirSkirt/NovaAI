# Nova V7.0.0-alpha.6.1 — Persistence Repair Validation

## Scope

Alpha 6.1 keeps the Pass 6 brain architecture unchanged and validates the persistence replacement around it.

## Static validation completed in this packaging environment

- all 24 JavaScript files pass `node --check` (0 syntax failures);
- a Node VM persistence harness verifies that the startup write gate blocks both local-cache replacement and native writes while suspended, then allows one awaited native checkpoint after reopening;
- `package.json` and `src-tauri/tauri.conf.json` parse as JSON;
- no merge-conflict markers are present;
- Rust persistence source passes a delimiter/static-integrity scan;
- native persistence has one current write authority (`app_data_dir()`); the retired `%LOCALAPPDATA%\QTech\Nova\UserData` path appears only as migration input/documented history;
- native turns still fail closed when Rust NovaMind is unavailable; no JS brain fallback was introduced;
- Pass 6 Rust architecture ID and mind/semantic schemas are unchanged.

## Rust tests added/retained

`src-tauri/src/persistence.rs` contains tests for:

- primary round-trip and backup recovery;
- invalid UTF-8 primary falling back to a valid backup;
- protecting a valid backup when current state is corrupt;
- migration choosing a newer `persistence.savedAt` regardless of location;
- mtime/primary tie-breaking;
- invalid/oversized payload rejection.

## Environment limitation

This environment has no `cargo`, `rustc`, or `rustfmt`, so Rust compilation and unit-test execution could not be performed here. A normal Rust-enabled Tauri build must run those checks.

## Required live update test for ISSUE-007

1. Install alpha 6.1 with identifier `com.qtech.nova`.
2. Teach Nova or create an unmistakable conversation/memory change.
3. Complete a turn and allow the UI response to finish (the new checkpoint is awaited internally).
4. Close Nova.
5. Uninstall while selecting the option to keep application data.
6. Install another build using the same identifier/app-data contract.
7. Verify the learned state/conversation is restored.
8. Inspect Brain/Persistence diagnostics to confirm the authoritative directory is Tauri app-data and primary/backup exist.

Also test an upgrade where both an old V5 app-data snapshot and a newer V7 custom-folder snapshot exist; alpha 6.1 should migrate the newer `savedAt` snapshot into app-data once.
