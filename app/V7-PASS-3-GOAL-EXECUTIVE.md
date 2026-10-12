# Nova V7 Pass 3 — Semantic Goal Executive

Pass 3 targets the failure exposed immediately after Pass 2: Nova could open the learned `lesson` concept and answer `What is a lesson?`, but richer questions often activated some meaning and still constructed zero requests.

The current Rust request path was therefore improved rather than wrapped.

## New current behavior

1. Build the same canonical cognitive state.
2. Try the migrated structural request constructors.
3. If they do not model the question, rank learned semantic evidence against the complete question and active concepts.
4. Allow strongly relevant evidence to activate another connected concept.
5. Construct one generic semantic-information goal.
6. Compose a supported proposition from the best learned evidence.
7. Reconsider the same state for a bounded number of cycles when cognition changes.
8. If Rust reaches a supported result, do not run the pending JavaScript cognitive scheduler over it.

## Regression probes

The Rust unit suite now includes the four Lesson 1 shapes that Pass 2 exposed, plus an unrelated invented-content transfer test. Those tests exercise the same generic goal machinery; production code contains no exact question handlers.

## Semantic-store correction

Pass 3 also fixes an upstream reason these questions could fail: natural `relationship:` and structured guidance/warning material could be absent from the semantic bridge even though Nova had received the lesson. The current document learner now retains those roles as semantic evidence and performs a one-time semantic hygiene/reindex upgrade for stored instructional sources.
