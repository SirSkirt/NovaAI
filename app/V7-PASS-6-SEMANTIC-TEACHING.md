# V7 Pass 6 — Semantic Memory & Teaching Authority

## Goal

Replace the remaining native JavaScript semantic-memory and teaching-ingestion authority with Rust. This pass is a replacement of current authority, not a Rust wrapper around the old learners.

## Main changes

1. Added `RustSemanticMemoryState` under `nova/semantics/knowledge/`.
2. `NovaMind::begin_turn` now reads semantic memory from Rust state; ordinary native callers send `knowledge: null`.
3. Added Rust teaching modules:
   - `teaching/parser.rs` — source identity, structured blocks, lesson/test boundaries and scopes;
   - `teaching/semantic.rs` — concepts, claims, guidance, examples, procedures and provenance;
   - `teaching/cognition.rs` — generic declarative learned policy/cognitive candidates currently representable by Rust.
4. Added `NovaMind::ingest_source`, Tauri `nova_mind_ingest_source`, and non-mutating Rust `nova_mind_parse_source` so native document learning classification no longer depends on the JS parser.
5. Rust-generated synthesis candidates are retained in `RustLearningState`, so Stored Only reconsideration and restart recovery do not depend on JavaScript retranslating the lesson.
6. Native document and conversation learning call Rust raw-source ingestion directly.
7. Native legacy JS learners are retired/fail-closed rather than used as fallback.
8. Native turn startup fails closed if authoritative Rust NovaMind cannot start.
9. Existing stable persistence payload temporarily stores exact Rust learning/semantic mirrors; startup transports them opaquely back to Rust, where semantic state is rehydrated and learned cognition is revalidated. No parallel database was created.
10. Rust owns learned document source identity on native Tauri. Named sources receive a deterministic ID that survives content edits; the returned Rust ID is then mirrored to JavaScript for UI/persistence.
11. Re-teaching the same stable source ID removes that source's previous semantic evidence and Rust-generated cognition before ingesting its current contents. One-time Pass-5-or-older imports normalize named legacy source IDs/evidence onto the same Rust identity so future corrections can replace them cleanly; generic conversation sources preserve their explicit event IDs instead of being collapsed into a named-source identity.
12. Persisted fallback IDs use the existing Rust ID subsystem's deterministic FNV-1a helper rather than Rust `DefaultHasher`.
13. Updated rule/module merging so distinct declarative programs from the same source are not collapsed merely because their descriptive metadata matches.

## One-time upgrade compatibility

Pass 5 and older persisted minds may still contain JS semantic/rule/module stores. Rust can import these once **only while the corresponding Rust state is uninitialized**. This is migration ingestion, not ongoing co-authority. After initialization, later frontend semantic/cognitive snapshots cannot replace Rust state.

## Deliberately still pending

- whole-mind durable payload/file ownership;
- experience-route reflection and persistent index learning;
- developmental category graph mutation;
- consolidation/abstraction;
- free thought/internal developmental goals;
- general response realization;
- conversation teaching-intent detection/coordinator decision.

The target after these migrations remains Rust brain + JavaScript UI.
