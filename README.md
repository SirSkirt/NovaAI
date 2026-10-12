# Neural Nova v0.5.0 — one jointly trainable hybrid brain

**Experimental branch only:** `experimental/neural-nova`. Original Nova on `main` is untouched.

## v0.5.0 — Learned word units and multi-turn neural memory

**Final design notes (v0.5.0):** The memory reader is initialized to **zero**, so a migrated older checkpoint is not disturbed by new, untrained memory signals. Once multi-turn lessons train the reader, neural memory influences the one generated output. The memory mechanism uses differentiable content-based attention over up to 12 compressed prior-turn slots (32 dimensions each); writing uses a trainable tanh projection of pooled word/character embeddings, and shared-gradient training flows into both the reader and writer. Whole conversation sessions are held out for validation.

The trainer now samples up to **512 raw ASCII characters** before applying learned token compression and 256-token attention truncation. This permits the word-input tokenizer to save context positions. The generative output head is **still character based** (128 ASCII classes), and the dense pathway still sees 64 characters. Do not claim full BPE/subword output generation, general semantic memory, or long-term retrieval. Word units are added from *training-only* text, assigned stable IDs and saved in the new checkpoint. New word/memory weights need training. Frontend syntax and native-command mapping passed static checks; the Rust project and native builds remain unverified.



This experimental upgrade keeps Neural Nova as **ONE trainable brain** (Transformer + dense network + shared-loss bridge). There is no second chatbot, canned-answer search, arbitrary response selection, or hard-coded semantic lookup.

**Input tokenizer:** `neural-nova/src/tokenizer.rs` provides stable, lossless word-unit tokens with ASCII character fallback for unseen words. Word units are learned from the training split only; words repeated at least twice (3–24 ASCII characters) may enter a 256-slot vocabulary. Token embeddings are part of the Transformer weights and are trained by the existing neural loss. Word-unit IDs never reorder. New words must be trained: they do not arrive with preloaded meaning. **Replies are still decoded/generated character by character**, so this is an input-side tokenizer improvement, not yet a full generative word/subword head. Only ASCII is supported.

**Neural multi-turn memory:** Separate from the 128/256-token attention window, Nova stores the last 64 conversation turns for display and persistence; the last **12 completed turns** are fed to a bounded *neural episodic memory* pathway, one compressed 32-dimensional vector per turn. Learned word/character embeddings are pooled and passed through a trainable tanh write matrix; a learned content-based attention score compares the current hidden representation against each slot; the weighted memory signal is projected into the Transformer's hidden state through a trainable read matrix. Gradients train both the attention interaction and its neural writer/reader. Shared-loss conversation training backpropagates through both memory matrices and past-turn input embeddings, as well as the existing dense pathway and Transformer. Saved turns are *data*, not scripted answers. The teacher does not decide which answer Nova generates. This is an experimental compressed latent memory **not** a proven semantic memory or learned selective-retrieval system; older turns beyond 12 no longer influence neural generation.

**Multi-turn lessons:** Teach → "Multi-turn conversations". Use complete alternated `U: ...` and `N: ...` lines, with blank lines/`---` separating at least five independent sessions of at least two turns each. Validation withholds **whole sessions**. A supervised next-character loss on each Nova reply trains the neural writer/reader only when prior turns exist. Existing text and single-turn dialogue modes remain available. The trainer still only teaches; Nova generates from its own combined weights.

**Persistent chat:** `neural-nova-conversation-v0.5.json` (separate from weights) stores up to 64 completed turns; after app restart, the saved chat is restored. "New conversation" clears conversation records and memory but **does not reset neural weights**. Saving uses a temporary file, sync and backup restoration on rename failures. The 12 most recent turns are passed through trained neural memory embeddings, **not appended to a growing character buffer**.

**Checkpoint migration:** New separate checkpoint `neural-nova-hybrid-0.5.nhyb` stores the expanded Transformer weights (word embeddings and trainable memory matrices), dense weights, both Adam states and stable vocabulary mapping in a `WORDUNIT` trailer. On first launch existing `neural-nova-hybrid-0.4.nhyb` or `neural-nova-hybrid-0.3.nhyb` is migrated **without overwriting the older file**. New word and memory parameters are randomly initialized and need training. Original `main` remains untouched. Reset opt-out marker prevents reimporting prior weights.

**Diagnostics:** The dual-hemisphere visualizer displays actual attention token labels (including learned words), activations and shared bridge. Chat reports original prompt token retention and number of past turns used. The 128 vs 256 comparison uses `commit:false` so neither run modifies conversation history, ensuring comparable memory input. Context still offers 128 or 256 *tokens*, but only tokens learned in the vocabulary are compressed; unseen words fall back to ASCII characters. The dense network retains its 64-character encoder.

**Verification needed:** Code was committed on `experimental/neural-nova`; frontend syntax/API wiring can be checked statically. Native Rust build and on-device Windows/Android training **not yet verified**; do not claim otherwise. Training memory efficacy should be evaluated on unseen multi-turn sessions, not just whether the chat history survives restart.

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

## v0.3.2 — Independent learning schedule switches

The **Teach** tab now has independent **Adaptive learning rate** and **Early stopping** checkboxes. Default: **Gentle 0.001 (fixed)** with early stopping enabled (six stalled validation epochs). Turning off adaptation prevents ANY learning-rate changes, without disabling validation-based stopping. Both settings are passed independently into the Rust teacher, saved locally in the UI, and locked during active training. Older clients' combined `adaptive` flag remains a compatibility fallback. Four new Rust tests verify independence.

**Note:** The previous adaptive scheduler only reduced the learning rate; it never increased it. Changes in response quality may have other causes and should be evaluated with controlled experiments.

## v0.3.1 — Better teacher, same autonomous neural Nova

This update changes **how the joint network is trained**, not its inference model or its answer generation.

- A new teacher-only Rust curriculum module gives a deterministic, whole-question grouped train/validation split (including differently cased/punctuated variants) and removes text-context leakage at the evaluation boundary.
- Up to 1,024 distinct dialogue pairs, 384-character replies, 50,000 characters/tokens per CPU session, and 60 epochs.
- The first dialogue epoch warms up on shorter contexts; following epochs deterministically shuffle all examples. Training data is never used as a scripted response table.
- Adaptive learning rate, early stopping, and **best held-out joint checkpoint** selection. Both pathways and the learned bridge are still trained from one shared objective.
- A batch-wide gradient accumulator reduces large per-character Rust allocations without changing the gradient math.
- The Teach tab shows learning-rate options, progress phase, held-out lesson counts, throughput, best epoch and history.

**Caution:** The old 1,386-step screenshot and fragmented replies are baseline outputs from the **previous** lessons. No new curriculum results are available yet, so do not claim the update improved answer quality. Validation metrics are sensitive to dataset/split changes, and this tiny ASCII model is not yet a competent general conversational AI.

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
