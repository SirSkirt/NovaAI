# Neural Nova v0.4.0 — three-tab hybrid brain lab

## v0.4.0 — Context and generation diagnostics

**Why:** Nova sometimes says "Yello!" and sometimes produces gibberish for the same question with unchanged weights. Random sampling and losing the prompt after 128 characters are distinct, testable hypotheses. Neither is proof of language understanding.

- Chat now exposes **reply limit** (32/64/128/192/256 characters), **temperature** (0.2–2.0), **context length** (128/256 characters), and **sampling** (random, fixed seed 42, or most-likely/greedy).
- The **Compare 128 vs 256** button sends the same prompt through the same *joint* hybrid network twice at matching sampling settings/seed and displays side-by-side replies. No training or response selection occurs.
- Every reply reports the amount of the original `U: ...\nN: ` prompt still retained, whether the original prompt began falling out of context, and whether the model ended its reply or hit the length limit.
- The Transformer learned positional embedding array expanded from 128 to 256 positions. Legacy weights, the trainable bridge and both optimizers are migrated; old 128-position embeddings map into the **last** 128 positions so outputs match under the original 128-character setting. The 128 extra positions are initially untrained and need examples reaching them.
- Checkpoint v0.4 is `neural-nova-hybrid-0.4.nhyb`. On the first boot it migrates existing `neural-nova-hybrid-0.3.nhyb` **without overwriting it**. Explicit model resets disable automatic old-checkpoint import, so reset stays reset. Old `main` is unchanged.
- Context option affects **Transformer attention**, not the dense pathway's existing **64-character encoder**, and **does not** add multi-turn conversation memory.
- For a clean experiment, use *Compare* with matched seed or use greedy sampling and inspect the prompt-retention readout. If a reply falls apart before the original prompt is truncated, losing that prompt is not a sufficient explanation. Even if extended context helps, the new untrained positions are a separate variable.

Rust tests include short-input equality between context settings, positional/optimizer migration, end-of-prompt reporting and greedy sampling. CI compilation/device behavior must still be confirmed.

## v0.3.2 — Independent learning controls

Teach now provides two independent toggles for adaptive learning-rate changes and early stopping. Default: Gentle (0.001), fixed rate, early stopping enabled. The teacher passes `adaptiveLearningRate` and `earlyStopping` separately to the joint Rust trainer. Fixed rate means no learning-rate changes for any epoch, even while early stopping can respond to held-out validation loss. Settings persist across app restarts. The previous adaptive schedule only **lowered**, never increased, the rate. No new model answering logic was introduced.

## v0.3.1 — Better teacher, same autonomous neural Nova

This update changes **how the joint network is trained**, not its inference model or its answer generation.

- A new teacher-only Rust curriculum module gives a deterministic, whole-question grouped train/validation split (including differently cased/punctuated variants) and removes text-context leakage at the evaluation boundary.
- Up to 1,024 distinct dialogue pairs, 384-character replies, 50,000 characters/tokens per CPU session, and 60 epochs.
- The first dialogue epoch warms up on shorter contexts; following epochs deterministically shuffle all examples. Training data is never used as a scripted response table.
- Adaptive learning rate, early stopping, and **best held-out joint checkpoint** selection. Both pathways and the learned bridge are still trained from one shared objective.
- A batch-wide gradient accumulator reduces large per-character Rust allocations without changing the gradient math.
- The Teach tab shows learning-rate options, progress phase, held-out lesson counts, throughput, best epoch and history.

**Caution:** The old 1,386-step screenshot and fragmented replies are baseline outputs from the **previous** lessons. No new curriculum results are available yet, so do not claim the update improved answer quality. Validation metrics are sensitive to dataset/split changes, and this tiny ASCII model is not yet a competent general conversational AI.

All new development is on `experimental/neural-nova`. The original `main` app has not been changed.

## One brain, two trainable pathways

The hybrid runs **a single neural decoder**. The dense 1024→512→512→128 pathway processes the current 64-character ASCII feature window and sends 128 continuous pre-softmax activations through a trainable 32×128 matrix into the Transformer's 32-dimensional hidden representation. The Transformer independently attends to the last 128 ASCII characters, but its final distribution is computed from the **fused** hidden representation.

The resulting one cross-entropy loss is differentiated through both pathways. It updates Transformer parameters, learned bridge parameters, and dense-network weights. There is no model answer voting, fallback selection, scripted response, or two-model chat view.

Core: `neural-nova/src/hybrid.rs` and `neural-nova/src/transformer.rs`. Native commands: `app/src-tauri/src/hybrid_lab.rs`. Frontend: `app/dist/index.html`, `app/dist/styles.css`, `app/dist/js/nova-ui.js`.

## UI

- **Teach**: text / dialogue datasets, editable lessons, file loading, teacher corrections, shared epochs, stop/reset, held-out evaluation.
- **Chat**: one Nova with combined neural generation; correct a response by entering a desired reply in Teach. This does not silently rewrite weights.
- **Visualize**: a schematic left/right brain. Real last-token attention weights, sampled dense activations, and bridge projections drive the display.
- **Debugger**: collapsible, always on the page and auto-expanded on errors.

The earlier legacy dense and standalone Transformer APIs/sources are deliberately retained in the repository but are not loaded by the new main UI.

## Checkpoints

`neural-nova-hybrid-0.3.nhyb` is **separate** from `neural-nova-transformer-0.2.ntrf` and `neural-nova-v0.1.2.nnov`. Hybrid checkpoints include both neural pathways, learned bridge, both Adam optimizer states, and step counts. Reset in the v0.3.0 UI affects only the hybrid.

Standalone v0.2 Transformer checkpoints can still be read into the augmented standalone Transformer with newly initialized unused bridge weights. The joint hybrid starts from its own new checkpoint; existing standalone weight files are not silently imported or modified.

## Requirements and limitations

128 ASCII tokens, 128-character Transformer context, 64-character dense feature window, CPU-only reference implementation, single-turn interaction. This remains a small experimental learning system; neither reliable conversation nor reasoning has been demonstrated.

```sh
cd neural-nova && cargo test --all-targets
cd ../app/src-tauri && cargo test --lib
node --check ../../app/dist/js/nova-ui.js
```

GitHub Actions produces Windows NSIS and Android debug artifacts after pushes to the experimental branch. A successful CI run does **not** establish verified device runtime behavior.
