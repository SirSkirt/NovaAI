# Nova V7 Pass 4.1 — Cognitive Trace UI

Pass 4.1 does not add another cognitive layer. It separates two presentations of the same authoritative Rust turn.

## Thinking

`CanonicalCognitiveState.cognitiveTrace` is produced inside Rust from structures that the current turn actually created. Each event carries a stage, human-readable meaning-level text, source IDs, and structured data. The JavaScript Thinking panel renders those events and does not inspect technical counters to invent a story afterward.

Typical stages are Observe, Reference, Context, Interpret, Relationship, Goal, Cognition, Unresolved, and Conclusion.

## Developer Trace

The JavaScript UI records exact technical telemetry after each completed turn: Rust semantic report, Rust cognitive-runtime report, semantic technical trace, executive state, canonical record IDs, cognitive search/index activity, and timing. These records are shown in a separate whole-chat collapsible drawer. They never feed cognition.

## Brain Inspector

Brain Inspector remains the deepest current/persistent state view and continues to expose canonical state, learned cognition, Stored Only diagnostics, synthesis, experience routes, goals, persistence, and technical failure state.

## Conversation context

Recent visible conversation turns are now included in the Rust turn input as read-only context evidence. This is diagnostic/representational plumbing, not a hardcoded reference rule. If Nova sees a prior assistant response but does not link a follow-up reference to it, Thinking can now say exactly that.
