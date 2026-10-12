# Neural Nova v0.1 — experimental-only replacement

This is the **new** Neural Nova application for branch `experimental/neural-nova`. Its original rule-based cognition is not compiled or loaded; the only reasoning backend is the independent, from-scratch Rust neural network in `/neural-nova`.

## What works
- 1,024 → 512 → 512 → 128 fully connected neural network; 853,120 learnable weights/biases
- 8 ASCII characters × 128 one-hot features as input; 128-way next-character distribution
- Text-corpus training (Adam, backpropagation, gradients, cross-entropy)
- Real activations for **all 2,176 neurons**; inspect top weighted contributions
- Brain visualizer in the input/hidden/hidden/output format, responsive to mobile/desktop
- On-device asynchronous training, cancellation, progress and epoch losses
- Local weights-only checkpoint in separate Neural Nova app data, automatic on epoch/stop
- Next-character generation with sampling and temperature
- Safe reset confined to experimental model data

## Building
```sh
cd neural-nova && cargo test
cd ../app && npm install && npm run tauri build
```

Android debug APK can be built with Tauri Android SDK/NDK installed:
`cd app && npm run android:init && npm run android:debug-apk`

**Not a chat LLM:** this is an eight-character lookback feed-forward character predictor. The UI labels it accordingly. Model outputs are only meaningful after training. Large text corpora take a long time, especially on phones. Character vocabulary is currently ASCII only.

Neural Nova's package identifier is `com.qtech.neuralnova.experimental`, distinct from original Nova. Original Nova remains on `main`. The original ZIP is preserved only as an archival source snapshot, not used at runtime.
