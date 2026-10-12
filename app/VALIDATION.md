# Nova V7 Validation

Current release: **Nova V7.0.0-alpha.9.3.1 — Compile Fix**.

Alpha 9.3.1 preserves the alpha 9.3 architecture and fixes the explicit lifetime required by the Rust compiler for `sentence_kind()`.

Primary checks:

- Basic English bootstrap lives inside the current Rust semantic pipeline;
- whole utterance is preserved while simple Year 1 structures can be grouped;
- `A person can answer a question.` retains left=`A person`, operator=`can`, link=`answer`, right=`a question`;
- simple question/request, negation, coordination and previous-utterance cues are represented;
- capitalization does not preteach the concept of a proper name;
- bootstrap state contains no built-in classroom labels such as noun/pronoun/agent/patient/grammar rule;
- teacher points, percentage, grade, notes, exact spans and per-span notes are Rust-owned;
- original question/answer stay immutable;
- conflicting point and percent grades are rejected;
- teacher/test feedback remains non-executable evidence;
- answer-key evidence remains hidden until the whole test run finalizes;
- old alpha 9.2 state restores through defaults/normalization into mind schema 12, teacher schema 2 and semantic schema 5;
- Thinking/Brain Inspector are projections of the same Rust state;
- Start Reflection remains an ordinary Teacher turn;
- alpha 6.1 low-level atomic persistence behavior remains unchanged.

See `V7-ALPHA-9.3-VALIDATION.md` for release-specific validation.

- alpha 9.3.1 compile repair: `sentence_kind` explicitly returns a borrow tied to `CanonicalCognitiveState`, not `sentence_id`; no clone/fallback was introduced.
