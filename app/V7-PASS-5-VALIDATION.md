# Nova V7 Pass 5 Validation

## Static validation completed in the packaging environment

- 24 runtime JavaScript files pass `node --check`.
- Project JSON files parse successfully.
- Cargo TOML parses successfully.
- package/Tauri/Cargo versions agree on `7.0.0-alpha.5`.
- Rust architecture id is `7.0.0-learning-synthesis-pass-5` and Rust mind schema is 6.
- Rust source delimiter/structure scan passes.
- No `unsafe` occurs in Nova Rust source.
- Production Nova Rust remains modular; largest production file is below 500 lines at packaging time.
- Production cognitive source contains no curriculum-answer handler for `What is a lesson?`, `Why should examples...`, or `Repeat after me`; synthetic regression strings may exist in `#[cfg(test)]` code only.
- Native turn bridge sends `cognition: null` after startup learning synchronization rather than replacing Rust learned state each turn.
- Native JS learning paths call `nova_mind_sync_learning`; browser paths may retain JS synthesis for browser development.
- Native post-hydration/source-rebuild/root cognitive compilation is hard-gated away from `v520GeneralSynthesisPass`; it defers native executability to Rust.
- Existing reset flow invokes `nova_mind_reset_learning` so clearing UI/disk state does not leave authoritative Rust learned cognition alive in-process.

## Rust regression tests added for Pass 5

The Rust test suite now includes cases for:

- adoption of a supported candidate only after Rust validation;
- unsupported program operation + unsupported condition staying Stored Only;
- fixed-point reconsideration where one learned module produces capability P and a previously blocked module waiting for P becomes executable in a later round;
- `unmapped_cognitive_state_transformation` never entering the capability graph;
- initialized Rust learned state rejecting a later JS cognition snapshot as authority;
- whole-source candidate validation/adoption without JS storage authority;
- reset clearing authoritative Rust learning/module/capability state;
- prior Pass 2/3/4 semantic/runtime regressions.

## Compiler limitation

This packaging environment does **not** contain `cargo` or `rustc`. The Rust code therefore cannot be compiler-validated here. Quinn's Windows/GitHub Tauri build remains the authoritative native compile/test check. Compiler errors must be fixed in the current Rust architecture; do not route around them with JS fallback.

## Behavioral baseline

The latest user-observed curriculum baseline before Pass 5 was V7 alpha3 Lesson 1 at **3/5**. Pass 5 is a migration pass, not a claim that those reasoning results improved. Re-run curriculum behavior only after the build compiles; do not treat migration tests as proof of better answers.
