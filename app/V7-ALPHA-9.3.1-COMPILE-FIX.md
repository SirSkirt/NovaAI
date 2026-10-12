# Nova V7.0.0-alpha.9.3.1 — Compile Fix

This release is a source-level compile repair for alpha 9.3. It does not change Nova's architecture, English bootstrap semantics, Teacher tools, reasoning, reflection, or persistence design.

## rustc finding

Rust reported E0106 in `src/nova/semantics/bootstrap.rs` because `sentence_kind()` returned a borrowed `&str` while receiving two borrowed parameters. The return value actually comes from `CanonicalCognitiveState.english_bootstrap`, so the lifetime must be tied to `state`.

Before:

```rust
pub fn sentence_kind(state: &CanonicalCognitiveState, sentence_id: &str) -> Option<&str>
```

After:

```rust
pub fn sentence_kind<'a>(
    state: &'a CanonicalCognitiveState,
    sentence_id: &str,
) -> Option<&'a str>
```

This expresses the existing ownership model directly. No clone, owned-string conversion, fallback parser, or second language mechanism was introduced.

The source tree was also reviewed for other functions returning borrowed values; the comparable helpers already use explicit lifetimes or `'static` returns.
