# Nova V7 Pass 4.1 Validation

## Scope

Pass 4.1 separates the presentation of one authoritative cognitive turn into:

- **Thinking** — meaning-level Rust cognitive events;
- **Developer Trace** — exact technical Rust/executive telemetry grouped by conversation turn;
- **Brain Inspector** — deepest current/persistent cognitive state.

It does **not** add a second reasoning/explanation system. `cognitiveTrace` is derived in Rust from canonical structures already produced by the authoritative turn and the actual Rust cognitive-runtime report.

## Static validation completed

- 24/24 runtime JavaScript files pass `node --check`.
- `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` all report `7.0.0-alpha.4.1`.
- Rust canonical state contains separate `semanticTrace` (technical) and `cognitiveTrace` (meaning-level) fields.
- `v628ThinkingTrace` renders native V7 Thinking from `cognitiveTrace`; it does not consume `rustSemanticReport.trace`.
- `trace/meaning.rs` contains no developer-counter narration such as Rust executive cycle counts or module-reference counts.
- Developer Trace retains semantic/runtime/executive telemetry separately, including canonical IDs, semantic cycles, request counts, module counts, search mode, experience-index activity, and synthesis/debug data.
- Developer Trace is whole-chat state, persisted separately from Thinking history, capped at 100 turns, and rendered as collapsible per-turn entries.
- Recent conversation messages are passed into Rust as context evidence. The cognitive trace can report whether an explicit semantic link to prior context was actually formed without hardcoding `that -> previous response`.
- The exact follow-up probe `Can you say that in a more natural way?` appears only in Rust tests, not production Rust cognition.
- Stable native state path remains `%LOCALAPPDATA%\QTech\Nova\UserData`.
- Production Nova Rust remains modular: 38 production Rust files; largest current production file is `semantics/types.rs` at 449 lines.
- Root-level `handoff.txt` is present and preserves the permanent issue ledger, including ISSUE-028 through ISSUE-030 for this trace/UI work.

## Regression targets encoded

- A Rust meaning-level `goal` event appears in Thinking while technical `semanticTrace` text does not.
- Developer Trace retains the technical semantic trace and cognitive runtime counts independently.
- A turn with a prior assistant response available but no explicit semantic link produces a truthful context event saying the prior response was available but was not linked.
- Existing Rust-authoritative semantic conclusions remain authoritative.

## Compiler limitation

This execution environment does not contain Cargo/rustc, so native Rust compiler/test validation must occur in Quinn's Windows/GitHub build. Pass 4.1 was statically inspected after fixing the owned Rust trace formatting/source changes, but the actual Rust compiler remains authoritative.
