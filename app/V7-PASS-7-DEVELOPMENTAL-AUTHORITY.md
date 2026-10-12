# V7 Pass 7 — Developmental Authority

## Goal

Replace JavaScript ownership of Nova's learned experience reflection, developmental category graph, and consolidation/abstraction with typed state and mutation inside the existing Rust `NovaMind`.

This is an architectural replacement pass. It does not add a developmental wrapper around the old JavaScript systems.

## New Rust state

`src-tauri/src/nova/development.rs` defines `RustDevelopmentState`, containing:

- categories and category relations;
- compact developmental episodes;
- provisional abstractions;
- consolidation cadence/reporting.

Experience routes remain in `RustLearningState`, because they directly schedule learned cognitive modules. Pass 7 makes their **learning and updating** Rust-owned as well as their execution-side use.

## Turn completion

`NovaMind::finish_turn` now performs developmental reflection before the completed turn is committed:

- supported turns can create or strengthen an experience route from the actual semantic signature and changed learned modules;
- a failed attempted experience route can decay rather than being permanently trusted;
- the turn becomes a compact developmental episode;
- active/learned categories are reinforced in the Rust category graph;
- periodic consolidation merges compatible experience evidence, prunes weak repeatedly failing routes, and creates/updates provisional experience abstractions.

Routes remain attention/history, not permission. Runtime broadening remains available when an experienced route stalls.

## Learned category operations

`register_category` and `relate_categories` are now supported Rust cognitive operations. They mutate the same `RustDevelopmentState` that subsequent turns/consolidation inspect. Rust therefore no longer reports these operations unsupported merely because storage lived in JavaScript.

## Migration

Pass 7 introduces one-time `nova_mind_import_development`. On upgrade, persisted Rust developmental state is preferred. Otherwise legacy JS categories, developmental episodes and abstractions may be imported once.

After initialization:

- frontend developmental imports are ignored;
- frontend learning synchronization cannot replace Rust experience routes;
- native JavaScript reflection/category/consolidation functions cannot mutate the authoritative mind.

JavaScript still mirrors the exact Rust state into the existing full payload until ISSUE-020 moves payload/file ownership into Rust.

## Deliberately not migrated in Pass 7

- free thought/internal developmental goals;
- conversation teaching-intent detection;
- general response realization;
- complete persistent mind payload/file orchestration.

Free thought is intentionally left for Pass 8 rather than creating a temporary second autonomous executive during this pass.
