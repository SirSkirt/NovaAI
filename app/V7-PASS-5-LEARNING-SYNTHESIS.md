# Nova V7 Pass 5 — Learning & Synthesis Authority

Pass 5 moves the authority for **what learned cognition is executable** into Rust.

## New Rust learning package

```text
src-tauri/src/nova/learning/
  mod.rs
  types.rs
  capabilities.rs
  synthesis.rs
```

`RustLearningState` stores the current native learned rules, validated modules, capability graph, experience-route mirror, module states and last synthesis report.

## Candidate boundary

Natural teaching is still parsed by the current JS teaching system in this pass. JS may propose:

- a rule-level `CognitiveModule` candidate;
- whole-source/whole-lesson candidate modules.

Those candidates are inert data until Rust validates them. Native JS does not store the proposal into the active cognitive catalog as proof of executability.

## Rust synthesis process

1. Import older module data for migration.
2. Validate every imported module's conditions and program operations.
3. Reject invalid imported modules and demote dependent executable claims.
4. Validate/adopt whole-source candidates.
5. Build capability producer/waiter truth from explicit module outputs and Stored Only diagnostics.
6. Consider rule candidates.
7. Reject unsupported operations/conditions or unavailable explicit capabilities.
8. Adopt supported modules and mark their rules executable.
9. Rebuild the graph and reconsider older Stored Only cognition.
10. Stop at fixed point or after the bounded maximum of 8 rounds.

## Important invariants

- broad categories/domains never certify a capability;
- diagnostic `unmapped_cognitive_state_transformation` never becomes a capability;
- imported `executable` status is not trusted without current Rust validation;
- whole-source construction remains available during migration;
- initialized Rust learning state cannot be replaced by a later frontend turn snapshot;
- unsupported cognition remains Stored Only rather than silently falling back to JS.

## What this pass does not fix

Pass 5 intentionally does not repair current reference/context or semantic reasoning mistakes. The goal is to finish moving brain authority first. `handoff.txt` records the remaining migration families and existing reasoning issues.

## Reset consistency

Because learned cognition is now authoritative in-process Rust state, the existing user reset path also invokes `nova_mind_reset_learning`. This prevents a cleared frontend/disk state from leaving old learned modules active until restart. Full mind reset/persistence ownership will be consolidated when the remaining persistence subsystem migrates.
