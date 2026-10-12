# Nova V7 Pass 8 — Validation

Static validation in the packaging environment:

- all JavaScript files parse with `node --check`;
- package/tauri JSON parses;
- no Rust leading-dot float literals remain;
- Pass 8 Rust modules are wired into `nova::mod`, Tauri commands, `NovaMind`, reset, snapshot mirroring, and persistence transport;
- Pass 8 regressions cover subsystem ownership, Rust realization, Rust teaching event-kind classification, and Rust internal-goal creation/attempt;
- alpha6.1 persistence source is intentionally not redesigned by Pass 8.

The packaging environment does not include Cargo/rustc. The first normal Tauri/GitHub build remains compile/test verification; any compiler diagnostics must be fixed in the authoritative Rust implementation rather than by restoring JavaScript fallbacks.
