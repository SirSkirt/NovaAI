# Nova V7.0.0-alpha.9.3.1 — Validation

## Compile finding addressed

- rustc E0106 for `sentence_kind()` fixed by tying the returned `&str` lifetime to `CanonicalCognitiveState`.
- No semantic behavior change.
- No allocation/clone workaround.
- No fallback or alternate language engine.

## Static validation

- all JavaScript files are syntax-checked;
- JSON/TOML release configuration is parsed;
- Rust source is scanned for other ambiguous borrowed-return signatures introduced by alpha 9.3;
- invalid leading-dot numeric literals are scanned;
- version metadata is updated to `7.0.0-alpha.9.3.1`;
- ZIP integrity is verified after packaging.

The container used to prepare this release does not provide `cargo`/`rustc`, so the user's normal Tauri build remains the authoritative compiler/test run.
