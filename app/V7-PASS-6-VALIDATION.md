# V7 Pass 6 Validation — Semantic & Teaching Authority

## Scope

Pass 6 replaces native JavaScript authority for semantic memory and lesson/source teaching with Rust. Validation therefore focuses on the authority boundary as well as source integrity: native turns must reason from Rust-owned memory, native teaching must enter Rust as raw source text, and retired JavaScript brain roots must not become fallback paths.

## Validation completed in this packaging environment

### JavaScript syntax

Every `dist/js/*.js` file was checked with `node --check` after the final authority-boundary edits.

Result: **0 JavaScript syntax failures**.

### Source-integrity checks

Static checks confirmed:

- no unresolved merge-conflict markers in `src-tauri/src` or `dist/js`;
- no live use of Rust `DefaultHasher` / `std::hash` for persisted IDs;
- current version/architecture metadata is Pass 6 / `7.0.0-alpha.6`;
- ordinary native turn startup sends `knowledge: null` and `cognition: null`;
- native turn startup fails closed if authoritative Rust NovaMind cannot start;
- Rust exposes both `nova_mind_parse_source` and `nova_mind_ingest_source`;
- native document intake asks the Rust parser whether the source is instructional/test material before brain learning;
- native lesson/conversation learning sends raw teaching to Rust and does not execute the retired JS learner/compiler first;
- retired native JS learning/synthesis roots contain Tauri fail-closed/retired guards;
- Rust semantic and learning state are persisted as opaque mirrors in the existing payload rather than regenerated as a second JS brain;
- native learned documents do not pass a JavaScript-generated brain source ID into Rust; Rust derives the stable source ID and mirrors it back;
- re-teaching a stable source ID removes that source's previous semantic evidence and Rust-generated cognition before relearning the current source;
- one-time legacy semantic/cognitive imports normalize named source identities to the same deterministic Rust source ID;
- persisted Rust IDs use the deterministic ID helper in `nova/ids.rs`;
- a lightweight Rust source delimiter scan found no unbalanced `()`, `[]`, or `{}` after ignoring comments/string literals.

## Rust regression coverage added

Pass 6 adds/extends Rust tests for these behaviors:

- Pass 6 remains explicitly `PARTIAL_RUST_MIND` while Rust owns semantic/teaching authority;
- Rust parser classification and lesson/test boundaries;
- raw Rust teaching ingestion creates semantic memory and retains synthesis candidates;
- initialized Rust semantic memory rejects later per-turn JS replacement;
- initialized Rust learning state rejects frontend rule/candidate injection;
- re-teaching the same source replaces obsolete source-backed semantic/cognitive state;
- named fallback source IDs remain stable across content edits;
- opaque persisted Rust semantic-state restore;
- opaque persisted Rust learning-state restore with current-substrate revalidation;
- one-time legacy semantic import rewrites named JS source IDs and evidence to deterministic Rust source identity;
- one-time legacy learning import rewrites named rule/module/candidate source IDs before synthesis;
- legacy conversation teaching events retain their explicit event source IDs across both semantic and cognitive import;
- whole test documents are rejected from learning before attempt.

## Environment limitation

This packaging environment does **not** contain `cargo`, `rustc`, or `rustfmt`. Therefore the Rust crate and Rust regression tests could not be compiled/executed here. The static checks above are not a substitute for a compiler.

Before treating alpha.6 as build-verified, run in a Rust-enabled development/build environment:

```text
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Any compiler failure should be fixed in the authoritative Rust subsystem itself. Do not restore a JavaScript brain fallback to make the build or behavior appear successful.

## Architecture acceptance criteria

Pass 6 is architecturally acceptable only if all of the following remain true after compilation/runtime testing:

1. A native turn uses `NovaMind.semantic_memory` rather than a frontend-built current knowledge snapshot.
2. A native lesson/source is classified and learned by Rust; JS may render/extract Teacher UI data but cannot decide semantic/cognitive truth.
3. Rust-generated rules, candidates, modules, capability state and Stored Only decisions survive restart through the mirrored Rust states without JS retranslation.
4. Updating the same source replaces that source's previous contribution rather than stacking old and new versions.
5. Rust failure does not cause native Nova to resume the retired JavaScript learner/reasoner.
6. Remaining JavaScript brain responsibilities are explicitly pending migration and are replaced directly in later passes.
