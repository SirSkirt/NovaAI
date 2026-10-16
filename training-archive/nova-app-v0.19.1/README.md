# Nova Messenger training-code archive

This is a byte-for-byte copy of the app's pre-v0.20.0 teaching UI, teacher/
ZIP/targeted Rust modules, grading tests and Android training service.
The archived files are **not compiled or packaged** in Nova Messenger.

Training is now Yendory Studio's responsibility. The Bicameral model's
standalone original source and training APIs remain in the root
`neural-nova/` Rust library. Restore any archived source from this directory
only for migration/reference or Yendory work; don't re-enable it in the
messenger as a temporary compatibility layer.

The Android application ID and development signing certificate were NOT
changed. Checkpoint import/export and local inference are still provided by
`app/src-tauri/src/nova2_lab.rs` (messenger-only edition).
