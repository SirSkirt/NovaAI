# Nova Messenger v0.19.0 — chat-first transition (2026-10-15)

## Scope

The experimental/neural-nova app now presents **Chat** first, with the existing
Teach, Foundation V2, ZIP lessons, tests/corrections, checkpoint import/export,
and Targeted Training preserved inside one **Train** workspace. Training history
has its own Train subtab. These features remain temporary until Yendory Studio
is a working independent trainer. No neural weights or mathematical architecture
are changed by this UI transition.

The original hemisphereBrain SVG paths, gradients, hemisphere shapes and
connections remain untouched. Its parent card is moved to a **right-side brain
inspector** on desktop, or a toggleable right drawer on narrow screens. The
inspector now calls the already-existing native nova2_inspect command, which
was previously not registered. No old HybridBrain is invoked.

## What the inspector actually measures

nova2_inspect returns:
- output probability changes after removing individual prompt tokens;
- sampled learned Dense **weights** (not real-time neuron activations);
- top next-token probabilities;
- a binary check for whether ablating the right pathway changes the output.

The visualized dots are schematic positions, not anatomical locations or
a map of specific brain neurons. The right hemisphere is a sample of stored
weights, not a neural activation trace. The original visualizer appearance is
preserved while adding an accurately labeled overlay.

The **Thinking tree** is a disclosure below chat showing only known evidence:
submitted message, checkpoint optimizer step, generated-token count,
KV-cache attention-key reads, and the explicit fact that a genuine internal
reasoning-step trace is unavailable. No invented chain of thought is shown.

The app retains Windows / Android Tauri, Rust neural inference and existing
.nnova2 import/export. Version bumps across all app manifests support
same-signed in-place Android upgrades.

## Still outstanding

- The 19,000-step user-supplied checkpoint has **not yet been embedded into the
  GitHub APK/installer** by this source-only UI change. Import remains available.
- The precise long-context cache parity fix against Claude's Python reference
  remains a separate compatibility task; UI changes alone do not certify it.
- Validate on a real Windows install and Android device, including native
  inspection performance, keyboard, file picker, sidebar and checkpoint round-trip.
- Preserve historical training source until Yendory Studio can use it.
