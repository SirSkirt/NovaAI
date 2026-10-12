# Nova V7.0.0-alpha.6.1 — Durable Persistence Repair

Alpha 6.1 does not introduce a new brain pass. It keeps the Pass 6 Rust authority unchanged and repairs the native persistence contract around that mind.

## Why this patch exists

Live update testing showed that uninstalling Nova while choosing to keep application data, then installing the next build, could start Nova without the work performed in the prior build. Nova V5 had behaved reliably through this workflow.

A source comparison against Nova V5.5.2 found that V5 used Tauri's stable `app_data_dir()` as the one native state authority. Later V6/V7 builds moved authority to `%LOCALAPPDATA%\QTech\Nova\UserData`. The low-level temp/backup/rename idea had not disappeared; the persistence contract/location and save-completion behavior had evolved around it. The audit also found a more direct loss mechanism: `memory.js` can exist before the native persistence bridge and hydration. An early `saveState()` could give an unhydrated/default WebView snapshot a fresh `savedAt`, after which bootstrap's "newest wins" arbitration could promote that snapshot over the real durable mind.

## Replacement, not wrapper

Alpha 6.1 changes the current persistence authority directly:

- Tauri app-data is authoritative again.
- The custom V7 folder is a migration source only.
- There is no dual-write and no read-through fallback after migration.
- Reset keeps the migration marker so retired locations cannot resurrect old state.
- No cognition or learning path falls back to JavaScript.

## Recovery and update guarantees

- Primary, backup and temporary snapshots live together in the stable app-data authority.
- First-run migration chooses the newest valid historical snapshot rather than assuming a location is newest.
- Invalid current state cannot replace a known-good backup.
- Native writes are flushed and reread before success is reported.
- Failed commits attempt to restore the previous valid backup.
- Persistence starts hydration-closed, so script-load saves cannot outrank the state that has not been loaded yet.
- Cache and disk writes remain gated until authoritative Rust hydration succeeds; failed Rust startup leaves the prior state untouched.
- High-value actions use awaited durable checkpoints.

## Validation status

The JavaScript syntax/static release validation can run in this environment. A Rust toolchain is not installed here, so the Rust persistence tests included with this patch still require `cargo test`/a normal Tauri build on a Rust-enabled machine before compile verification. Live installer validation is specifically required for ISSUE-007: teach/use Nova, close it, uninstall while keeping application data, install alpha 6.1 or a subsequent build with the same identifier, and verify the current mind is restored.
