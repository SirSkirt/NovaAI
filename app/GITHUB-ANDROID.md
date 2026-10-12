# Nova V7 Pass 4 — GitHub / Android Build Notes

Pass 4 keeps the Tauri shell and expands the modular Rust mind under `src-tauri/src/nova/` with the native learned cognitive-module runtime.

The existing GitHub Actions workflow at `.github/workflows/build.yml` remains the intended compile/package path for Windows and Android development builds.

Because Rust/Cargo are not available in the ChatGPT execution environment used to prepare this source release, the GitHub/user build is the authoritative Rust compiler validation. Do not claim a local Rust compile unless it was actually performed.
