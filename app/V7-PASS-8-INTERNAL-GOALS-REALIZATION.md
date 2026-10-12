# Nova V7 Pass 8 — Internal Goals & Realization Authority

Pass 8 replaces the remaining live JavaScript cognitive authority for free thought/internal goals, native conversation teaching-event classification, and response realization.

## Rust-owned additions

- `RustInternalCognitionState`: persistent self-generated internal goals and history.
- Generic internal-goal creation after learning or unresolved cognition.
- Bounded internal-goal attempts through the same `interpret_turn` + `run_cognitive_runtime` path used by ordinary cognition.
- `RealizationReport`: selected semantic surface, terminal state, reason code, meaning availability, and visible-response requirement.
- Rust current-turn `eventKind` for native teaching-vs-cognition coordination.
- Rust turn completion uses its own stored realization; the frontend cannot overwrite terminal meaning state.

## Retired native JavaScript authority

- `v621AfterLearning`, `v621AfterBlockedTurn`, `v621RunFreeThoughtTick`, and JS internal-goal execution are not native cognition paths.
- `v450DetectTeachingAct` is not consulted for native turn routing.
- JS no longer decides the speakable surface or SUPPORTED/BLOCKED/EXHAUSTED status after Rust cognition.

JavaScript temporarily persists an opaque `rustInternalCognitionState` mirror with the other Rust mirrors. This is transport only; full payload ownership is Pass 9.
