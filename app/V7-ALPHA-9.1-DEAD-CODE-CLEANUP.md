# Nova V7.0.0-alpha.9.1 — Post-Migration Dead-Code Cleanup

Alpha 9.1 cleans production Rust after the Pass 9 authority migration. It does not add a new architecture pass.

Removed from production: obsolete raw location helper, unused cognition/knowledge helpers, redundant developmental relation method, and direct per-subsystem import/sync/reset APIs superseded by whole-mind bootstrap/migration. Historical regression construction helpers are `cfg(test)` only.

No dead-code allowances, JavaScript fallback brain, compatibility wrapper, or hardcoded cognitive behavior was added.
