# V7 Alpha 9.2 Validation

## Static checks available in this environment

- JavaScript syntax checked with `node --check` across `dist/js`.
- package/Tauri JSON parsed successfully.
- `Cargo.toml` parsed successfully.
- changed Rust source reviewed for balanced delimiters and known invalid leading-dot float literal mistakes.
- teacher annotation offset convention is UTF-16 on both UI and Rust sides.
- separate Developer Trace DOM/control references removed from native UI.
- native Teacher update commands expose grading/notes/annotations, not original-answer mutation.

## Rust regression tests added

- teacher feedback preserves Nova’s original answer;
- partial credit and annotations persist while executable-rule count remains unchanged;
- answer-key/grading evidence is absent from semantic retrieval until Rust finalizes the full test run;
- teacher feedback is exposed as `teacher_feedback` evidence with `nonExecutable=true`;
- annotation ranges outside preserved text are rejected;
- teacher/test history remains Rust-owned while the reasoning owner remains the existing authoritative Rust mind.

## Compiler limitation

This ChatGPT execution environment has no `cargo`, `rustc`, or `rustfmt`. The source therefore requires the normal GitHub/Tauri build for authoritative compiler verification. No warning/error suppression or fallback implementation was introduced in place of that verification.
