# Nova V7 Pass 9 — Rust-Authoritative Complete Mind

Pass 9 completes the V7 JavaScript-to-Rust brain migration. Native Tauri JavaScript is now UI/IPC only; it does not assemble, hydrate, mirror, synchronize, or persist Nova's cognitive state.

## Final authority boundary

Rust `NovaMind` owns:

- turn lifecycle and conversation continuity;
- canonical semantics and discourse memory;
- semantic memory and source provenance;
- learned cognitive rules/modules/candidates and capability graph;
- Stored Only state and fixed-point reconsideration;
- experience routes, reflection, developmental episodes, categories and abstractions;
- internal goals/free thought through the same executive;
- teaching-event interpretation and response realization;
- the complete durable mind snapshot, hydration, migration, serialization, recovery and reset.

JavaScript owns native presentation only:

- input controls and chat rendering;
- Thinking and Developer Trace presentation;
- Brain Inspector rendering from a read-only Rust snapshot;
- Teacher/test presentation and settings;
- thin Tauri IPC calls.

## Persistence

The alpha 6.1 V5-style persistence contract is retained: one stable Tauri `app_data_dir()` under `com.qtech.nova`, with `nova-state.json`, backup and temp files. Pass 9 changes the payload owner, not the durable location.

Rust writes a `PersistentMindEnvelope` containing:

- `mind`: a direct serialization of `NovaMind`;
- `ui`: opaque presentation state supplied by JavaScript;
- persistence metadata.

JavaScript never receives raw authority over the `mind` object for saving. It can request a checkpoint and supply only UI state. Rust takes its own locked snapshot and serializes it.

## Legacy migration

On first Pass 9 startup, Rust can read the alpha 8.1-era payload and extract the existing Rust semantic, learning, developmental and internal-cognition mirrors itself. It also imports prior visible conversation messages into Rust-owned conversation history. Rust immediately commits the migrated Pass 9 envelope. There is no continuing read-through or dual-write architecture.

## IPC hardening

The frontend no longer exposes raw `nova_state_load/save/clear` commands or JavaScript-driven semantic/learning/development/internal import commands. Native IPC exposes current-mind actions and whole-mind lifecycle only.

No Rust failure wakes JavaScript cognition. A failed Rust bootstrap/checkpoint is an error, not permission to construct a new JS mind.


## Final IPC boundary
Native begin-turn IPC accepts only the new utterance and source label. Native finish-turn IPC accepts only the turn ID and whether the Rust-selected response rendered. The frontend cannot inject knowledge/cognition/context snapshots or terminal/reason judgments.
