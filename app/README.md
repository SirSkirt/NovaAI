# Nova Messenger v0.20.0

Nova is a **local messaging app** for the Bicameral Nova-2 neural network.
**Yendory Studio is the trainer.** Nova contains no teaching interface, lesson
queues, targeted training, teacher corrections, grading or foreground-training
service. Its native Tauri command list contains only inference, checkpoint
transfer and read-only brain inspection.

## Getting started

Install the Android APK or Windows NSIS build. The application bundles the
19,000-step checkpoint `resources/nova2-step19000.nnova2` on first launch.
It checks for an existing installed checkpoint before using the bundled
default. **Installing newer APKs never replaces imported or trained weights.**

Use **Checkpoints** to import a later Yendory-trained `.nnova2` checkpoint.
The importer validates the model, preserves a backup of the previous checkpoint
and starts a new conversation after a successful replacement. The active
checkpoint can also be exported. Android uses the system Files picker,
including a SHA-256 read-back-verified export; Windows uses a normal download.

The original dual-hemisphere SVG is preserved in the read-only inspector.
Its telemetry displays output sensitivity and sampled Dense weights, **not**
human-style internal thoughts. The chat thinking-tree section is intentionally
a truthful placeholder for generation statistics until structured reasoning
traces become available.

## Code ownership and preservation

The app's pre-v0.20 training-specific UI, Rust teacher modules, tests, and
Android training bridge are preserved under
`training-archive/nova-app-v0.19.1/` in this experimental branch. The
`neural-nova/` Rust crate remains available as the original neural/training
library, with the Bicameral weights/checkpoint format unchanged.

The Android package ID `com.qtech.neuralnova.experimental` and pinned debug
signing certificate remain unchanged to preserve in-place APK updates.
Downloads remain `Nova-[Version].apk`, and GitHub Actions publishes successful
experimental installers as prereleases.

The existing conversation file `neural-nova-bicameral-conversation.json` and
checkpoint file `neural-nova-bicameral-v2.nnova2` retain their original
app-data locations to preserve installations.

## Known limits

The bundled 19k-step model is early training and may answer poorly. Nova
currently accepts ASCII messages (up to 300 bytes) with 256-token maximum
context, a 12-turn conversation cache and greedily decoded replies by default.
Long-context cache re-windowing prioritizes Python reference compatibility
over peak speed. A newer Yendory-trained checkpoint can be imported without
rebuilding Nova.
