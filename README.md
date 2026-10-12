# Neural Nova v0.3.0 — one jointly trainable hybrid brain

**Experimental branch only:** `experimental/neural-nova`. Original Nova on `main` is untouched.

Neural Nova combines its from-scratch Rust Transformer and existing dense neural network into **one model with one output distribution**. This is not two separately generated answers, an answer selector, or a hard-coded ensemble.

## Neural architecture

```text
                 input ASCII context
                        │
               ┌────────┴────────┐
               │                 │
       Left / language      Right / association
         Transformer          Dense network
         self-attention      1024→512→512→128
               │                 │
               │       dense pre-softmax features
               │                 │
               └── trainable 32×128 bridge ─┘
                        │
             single fused hidden state
                        │
              single softmax / reply
                        │
                 shared loss
                        │
            backpropagates into BOTH
```

- **Transformer:** embeddings, 128-character causal attention, 32-wide hidden state, and output projection.
- **Dense pathway:** original fully connected topology and ASCII feature encoder; its 128 pre-softmax features flow through a **learned projection** into the Transformer hidden state.
- **Shared training:** cross-entropy on the fused result. Gradients update Transformer weights, all bridge weights, and the dense network weights and biases. Both receive gradients during a single training step.
- **Joint checkpoint:** `neural-nova-hybrid-0.3.nhyb` in the experimental app's local data folder; contains both sets of weights, the trainable bridge, and Adam optimizer state for both hemispheres.
- The previous independent Transformer and dense core implementations, commands, and checkpoints remain present as development/compatibility resources, but the **main UI** now uses only the unified hybrid commands.

## UI

Exactly three main tabs:
1. **Teach** — lesson import, question/answer corrections, text or dialogue training, progress, metrics, reset.
2. **Chat** — a single Nova chat, no model picker or response arbitrator. Correction button transfers the question to Teach; corrections are not added as canned responses.
3. **Visualize** — stylized left/right brain connected by the learned bridge; plots measured attention weights, sampled real dense activations, and learned bridge signal magnitudes.

The compact debugger is available beneath all three tabs and opens automatically on errors.

## Limitations

This is a very small CPU-only experimental character model, **not** a reasoning LLM or established language-understanding system. Context is 128 ASCII characters; chat is single-turn. Some generated replies may be nonsense, even after training. Validation on held-out examples is necessary but not sufficient evidence of generalization. The brain visualization is *schematic*, not an anatomical claim. Its displayed activity is derived from real model state.

## Build and test

- Core: `cd neural-nova && cargo test --all-targets`
- Tauri Rust: `cd app/src-tauri && cargo test --lib`
- Frontend: `node --check app/dist/js/nova-ui.js`
- Windows: `cd app && npm install && npm run build -- --bundles nsis`
- Android: see `.github/workflows/neural-nova-app.yml`.

CI builds are not real-device testing; verify Windows/Android chat, training, checkpoint resumption, and visualization on installed builds.
