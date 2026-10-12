# Nova V7.0.0-alpha.9.3.1 — Compile Fix

Alpha 9.3.1 is a compile-only repair to the alpha 9.3 Basic English + Teacher Refinement release. The architecture and behavior remain alpha 9.3; the Rust lifetime on `sentence_kind()` is now explicit. It does **not** add another language model, Teacher reasoner, reflection engine, compatibility layer, or fallback brain.

The update adds a deliberately small **Basic English bootstrap** to the existing Rust semantic pipeline: enough structural English to understand simple Year 1 teaching, but not a built-in English education.

> **Enough English to attend school, not enough English to skip school.**

The bootstrap preserves whole utterances and basic structural cues such as phrase groupings, left-link-right relations, simple copular forms, negation, quotation boundaries, statement/question/request cues, deictic/reference cues, coordination, and a small amount of conversational continuity. It does not preteach school concepts such as noun, pronoun, proper name, semantic-role terminology, proposition, reasoning, evidence, reflection, school, lesson, teacher, student, test, or grammar rules.

Teacher tooling is also refined:

- points such as `0.5 / 1`, optional percentage and grade;
- Teacher Notes;
- exact strike/highlight/underline spans on immutable question/answer text;
- optional notes attached to each marked span;
- attempt number and lesson/source context;
- preserved Rust Thinking trace and Inspector context;
- feedback stored as provenance-bearing **non-executable Knowledge**;
- Start Reflection still sends an ordinary Teacher utterance through Nova's normal Rust turn path.

See `ARCHITECTURE.md`, `TEACHING.md`, `V7-ALPHA-9.3-BASIC-ENGLISH-TEACHER.md`, `V7-ALPHA-9.3-VALIDATION.md`, and `handoff.txt`.

Build versions:

- package: `7.0.0-alpha.9.3.1`
- Tauri: `7.0.0-alpha.9.3.1`
- Cargo: `7.0.0-alpha.9.3.1`


Compile repair details: see `V7-ALPHA-9.3.1-COMPILE-FIX.md` and `V7-ALPHA-9.3.1-VALIDATION.md`.
