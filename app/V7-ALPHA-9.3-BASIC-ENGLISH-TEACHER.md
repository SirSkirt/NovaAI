# Nova V7.0.0-alpha.9.3 — Basic English + Teacher Refinement

## Purpose

Alpha 9.3 gives Nova the minimum structural English substrate needed to enter Year 1 while refining the existing Rust Teacher evidence model.

This is not a new architecture pass. Pass 9 remains final: Rust is Nova's brain and JavaScript is native UI/thin IPC.

## Basic English

The new `semantics/bootstrap.rs` operates inside the authoritative Rust semantic pipeline.

Its boundary is intentionally narrow:

- preserve language structure before learned concepts exist;
- never replace learned English;
- never hardcode curriculum answers;
- never use capitalization as hidden proper-name knowledge;
- never silently assign classroom semantic-role names;
- preserve uncertainty and let later learning refine structure.

The bootstrap carries a small amount of English function/glue vocabulary because otherwise simple teaching instructions cannot be structurally preserved. Content knowledge remains absent.

Motto: **Enough English to attend school, not enough English to skip school.**

## Teacher refinement

Alpha 9.3 evolves the existing Rust `teacher_state` rather than adding a Teacher cognition system.

Added/strengthened:

- score points (`awarded / possible`);
- optional percent and grade label;
- consistent-grade validation;
- deterministic attempt numbering, including normalization of older attempts;
- lesson/source context;
- optional note on every exact marked span;
- preserved Rust Thinking and Inspector context;
- richer non-executable Knowledge projection with Teacher provenance.

Reflection remains ordinary cognition initiated by an ordinary Teacher message.

## Non-goals

Alpha 9.3 does not preteach nouns, pronouns as a school category, proper names, semantic roles, propositions, reasoning, evidence, reflection, school, lessons, teachers, students, tests, or grammar rules. It does not add scripted reflection questions, Q/A shortcuts, a language fallback, or a Teacher reasoner.
