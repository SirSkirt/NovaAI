# V7 Alpha 9.2 — Teacher & Observability Tools

## Goal

Make the final Rust-authoritative Nova substantially easier to debug and teach **without** creating an explanation engine, teacher brain, or reflection facade beside the real mind.

## Human-readable Thinking

Rust already owns the structured `cognitiveTrace`. Alpha 9.2 improves its meaning-level wording and keeps the UI as a renderer only. Thinking emphasizes notice, interpretation, connection, goal, consideration, unresolved state, and conclusion. Exact modules/search/compiler/runtime detail moves to Brain Inspector.

## Rust teacher/test state

A new focused `teacher.rs` module is part of `NovaMind`. It stores bounded completed-attempt history and teacher feedback. Original question/answer fields are never present in the feedback-update DTO, so grading cannot rewrite the attempted response.

Teacher annotations use validated UTF-16 spans so browser text selection and Rust persistence share one offset convention.

## Non-executable feedback Knowledge

Each teacher record projects into semantic memory as a `KnowledgeExample` whose evidence is tagged:

- `sourceType: teacher_feedback`
- `sourceRole: non_executable_knowledge`
- `nonExecutable: true`
- attempt/run/suite/case provenance

The evidence surface also identifies the actual marked excerpts. The projection never calls the teaching cognition translator or synthesizer.

### Test-run release boundary

Rust records each attempted question immediately, but `evidence_released` remains false during the active run. Expected answers and grading context are therefore not part of normal semantic retrieval between test questions. After the last attempt, `finalize_teacher_run` marks that run released and projects its records. The release is idempotent.

## Reflection

Start Reflection sends a normal Teacher utterance through `handleUserMessage` -> native Rust turn. It does not call a dedicated reflection command. No reflective questions are supplied by code. The preserved feedback evidence merely becomes available for Nova to retrieve if her own learned/current cognition can use it.

## Brain Inspector

The Inspector now answers the debugging question first—`Why did Nova do that?`—then separates semantic interpretation, goals, relationships, evidence, Stored Only cognition, test/teacher history, synthesis/development, detailed telemetry, and ownership.

The old standalone Developer Trace drawer is retired; its presentation-only history remains available under `Grandpa Rust telemetry`.


### Preserved attempt reasoning
Each native test attempt stores Nova's authoritative Rust meaning-level cognitive trace and a compact cognitive-context snapshot alongside the immutable question/answer. This is debugging/reflection evidence only; JavaScript cannot submit or rewrite the preserved reasoning.
