# Nova V7 Pass 4 — Cognitive Runtime

Pass 4 targets the next troublesome boundary exposed after the Rust semantic-goal executive: learned cognitive programs still depended on the JavaScript interpreter. Native Tauri learned-module execution is now Rust-owned.

Key changes:

- modular Rust cognitive runtime under `src-tauri/src/nova/cognition/`;
- condition evaluator with recursive `all` / `any` / `not`;
- generic program operations split by responsibility;
- per-reference learned module execution;
- open typed extension fields for learned structures;
- explicit known-capability gating;
- execution-side experience route scheduling and stale-route broadening;
- Rust canonical-state validation/rollback if a learned program creates an invalid typed state;
- honest unsupported-operation diagnostics;
- no module runtime run after an already-supported Rust semantic result.

The current JavaScript synthesis system still creates/stores learned modules and capability/Stored Only metadata. Pass 4 bridges that data into Rust; it does not call the JS interpreter from Rust.

`register_category` and `relate_categories` remain intentionally unsupported in the Rust runtime because the persistent category-development graph is still JS-owned. They must not be falsely acknowledged.
