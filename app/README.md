## v0.7.1 — Text validation split and lower neural learning rates (2026-10-12)

- **Text/Language validation changed**: no more 'first 90% of bytes train / last 10% validate'. The teacher prepares independent whole **paragraphs** (blank-line-separated) when there are at least five, otherwise complete **sentences or lines**. Every section is assigned as a whole to training or validation using a deterministic hash; equal text up to case/whitespace normalization stays in the same group. Approximately 20% of unique section groups are held out, not necessarily exactly 20% of bytes. Neither training nor validation contexts can span section boundaries, and tokenizer vocabulary growth only sees the training sections. Reversing the file order doesn't change the assignment. This is stronger than the old tail-based holdout but does **not** prevent paraphrase/topic contamination and doesn't guarantee representative topic coverage.
- Text files with **fewer than five distinct meaningful sections now return an explanatory error** instead of a misleading held-out accuracy. Recommend longer varied text, preferably separate paragraphs with blank lines. ASCII limit remains 120–50,000 bytes; user is preparing a larger varied corpus.
- Rehearsal archives re-prepare previously saved lessons using this updated split: existing Text rehearsal sources also need at least five distinct sections. Dialogue and multi-turn preparation are unchanged.
- **Two new train learning rates**: `0.0003` (Very gentle) and `0.0001` (Extra gentle), in addition to old 0.001/0.003/0.005. The native trainer accepts values down to 0.0001. Both neural pathways already honor the actual selected rate. Separately, adaptive learning now derives a monotonic floor from the selected rate; choosing 0.0001 can never silently jump up to the old 0.0003 floor. The user can leave adaptive OFF to hold the selected rate fixed. Test & Corrections corrective LR remains separate and unchanged.
- New Rust core unit tests verify non-overlapping paragraph holdouts, deterministic reordering, duplicate protection, refusal of unsplittable corpora, and nonincreasing adaptive schedules below 0.0003.
- Train JSONL `lesson_before_vocabulary` now records `trainingSections`, `heldoutSections`, and `validationStrategy`. This is teaching/measurement code only; the transformer, dense network, inference, weights, and checkpoint format did not change.

**Recommended comparison:** Back up the working 399-step checkpoint, keep previous dialogue rehearsal and the exact greeting test enabled, then train the new diversified Text curriculum at fixed **0.0003**, early stopping ON, replay and prior-skill guard ON, and JSONL recording ON. For a controlled comparison with **0.0001**, start from the **same** saved 399-step checkpoint and identical archive/curriculum. Larger, more varied corpora and lower rates may reduce overfitting, but are not guaranteed to solve it. Inspect per-epoch training/held-out loss and greeting accuracy. Build status must be confirmed in the latest Actions run before installation.


## v0.7.0 — Trainer regression fix: rehearsal + protected checkpoint selection (2026-10-12)

**Why:** The user's 399-step to 735-step diagnostic JSONL showed later Dialogue training replaced the greeting `hello → Hello!` with a dominant `Yes, a...` pattern. Tokenizer growth alone did not change greeting predictions; gradient batches did, with no episodic memory involved. Training loss fell, but older skills declined. The old trainer picked the best epoch by current-lesson held-out next-character loss only.

**Design — ONE neural brain, no hardcoded cognition:**
- Separate Rust teacher archive `app/src-tauri/src/rehearsal.rs`, stored in app-data `neural-nova-rehearsal-v0.7.json`. Max **3** earlier curricula. Each is saved in original ASCII source form and re-prepared deterministically; **only its training split** can ever be replayed. Its held-out split remains evaluation-only and is never added to gradient batches.
- Archive is linked to the **current joint weight fingerprint**, so resetting Nova or externally loading an unrelated checkpoint will not silently replay another model's lessons. Accepted lessons are archived after their best checkpoint is saved. If no acceptable epoch is found, old checkpoint is restored and no lesson is promoted.
- Training now interleaves **one real previous-lesson gradient batch after every three new batches**. Mixed prior Dialogue, Text, and Conversation sources each use the same joint HybridBrain training entry point appropriate to their memory mode; no path produces a separate answer, canned response, or teacher inference.
- The trainer measures average **prior held-out token loss**, equal weight per archived lesson, max 256 deterministic validation examples per old lesson. Default guard accepts a new checkpoint only when (a) the new lesson's held-out loss improves, (b) prior held-out loss remains at most **baseline +0.10** nats, and (c) if selected, teacher Test & Corrections greedy answer accuracy is no lower than before. It selects the lowest new-lesson loss **among eligible epochs**. Guard can be disabled for controlled experiments; the loss tolerance has strict 0.05, moderate 0.10 and permissive 0.20 choices.
- The previously-existing early-stopping and adaptive-rate toggles remain independent. Gentle 0.001 remains fixed with adaptive off. Retention guard rejects bad checkpoints but does not change neural sampling or cognition.
- Pre-lesson snapshots and JSONL diagnostics continue. New events include `rehearsal_baseline`, `rehearsal_batch`, `train_epoch.priorHeldoutLoss`, `train_epoch.retentionGuardAccepted`, and a `training_selected_checkpoint` policy description.
- Teach tab: compact expandable **Keep earlier lessons** section includes replay and retention toggles, threshold, archive status, and an explicit import for old lesson TXT (critical for the user's 399-step checkpoint trained before this version). Importing archives the teacher source only; it **does not train** Nova until the next run. Teacher grades and expected replies are never used at inference.
- Reset wipes the teacher replay archive in addition to neural weights. The chatbot and visualizer remain unchanged. Archive is **not** shared with the independent original Nova on main.

**Important caveats and test steps:** This is a teacher/trainer policy, not proof of general language comprehension. Older holdouts are sampled; rare behaviors outside them may still be forgotten. For a precise greeting guard, save greeting questions as a separate Test & Corrections teacher benchmark and select it for the next lesson. Protect a copy of the original 399-step checkpoint. Import its original prior dialogue lesson TXT into the archive, enable Replay and Protect, choose the saved benchmark, and repeat the same Lesson 03 with Gentle 0.001 and early stopping. Compare old/new heldout loss, teacher accuracy, accepted epochs, and JSONL. Avoid test corrections during evaluation.

**Validation:** Check latest GitHub Actions Rust core tests AND both Windows/Android native builds for the final commit before calling the installers verified. Earlier cancelled runs do not count.

# Neural Nova v0.5.0 — three-tab hybrid brain lab

## v0.6.1 — Opt-in forensic logs for trainer vs episodic memory

**Purpose:** User wants to repeat the 399-step → 735-step dialogue/text learning experiment and determine whether later regression stems from tokenizer growth, optimizer/lesson updates, checkpoint selection, or saved conversational memory taking precedence. No architectural rebalancing or response rules have been introduced. Everything in this update is read-only instrumentation except writing logs to disk.

### How to record
1. Back up the working and regressed joint checkpoints (and keep them separate).
2. Open **Debugger → Deep neural logging**, click **Start recording** before the first new lesson. This writes one timestamped **JSONL** file under Tauri's platform app-data folder, `neural-nova-diagnostics/`. The absolute file path is displayed; every recording starts a new file. It also runs a read-only initial fixed-prompt probe.
3. Use **Inspect same weights** with `hello`, `Hi`, `good morning`, and `hey there`. With the checkbox OFF it uses zero prior turns; with the checkbox ON it applies saved conversation turns to the exact same learned weights and reports a zero-memory baseline in the same probe. The test does not add to or mutate chat history. The comparison uses 128- or 256-token context from the Chat setting.
4. Rerun the original Dialogue/Text lessons with unchanged Gentle rate and original lesson order. The recorder automatically appends `lesson_before_vocabulary` and `lesson_after_vocabulary` (same weights before/after token-ID growth), `lesson_validation_baseline`, every `train_batch` (loss, real step, LR, Transformer/dense/bridge gradient norms, clipping, whether episodic memory is used), each `train_epoch` (loss, model fingerprint, fixed isolated greeting probes and checkpoint decision), and `training_selected_checkpoint` (best restored weights, not just the last epoch).
5. If using **Tests & Corrections**, the log also captures `test_started`, all `test_baseline` answers, `test_correction` weight changes and `test_retry` responses. Turn corrections **OFF** if you want an observation-only assessment rather than additional training.
6. For chat, `chat_inference` records actual generated output, context settings, first-token probabilities with and without remembered turns, token IDs, attention weights and neural bridge signal. The Visualize inspector also records read-only snapshots. No hard-coded answer selection.
7. Click **Stop recording**, then **Save JSONL** or **Copy JSONL**. Android's WebView may not support direct browser downloads; the original remains at the displayed app-data path and Copy can be used as a fallback. Share logs only after reviewing them—questions and test answer keys may appear. Raw lesson corpora are fingerprinted (FNV-1a correlation ID, not a security hash), not copied into each entry.

### Interpretation / invariants
- Dialogue/Text **training** calls `train_batch`, whose Transformer gradient uses an **empty episodic memory list**. Multi-turn Conversation teaching uses `train_memory_batch`, and ordinary Chat / Visualize may read the persisted conversation history. Saved chat memory therefore cannot be *assumed* to have trained the wrong greetings; measure its contribution directly using the exact same checkpoint and prompt.
- Recorded `memoryTotalVariation` compares next-character predictions **with and without saved episodic turns**, with no neural updates, which directly tests whether learned memory is changing current inference. A displayed "5/12 slots" is not proof those slots dominate: influence can be zero if the reader has not been trained.
- The train loop still selects the best weights by held-out **next-character loss on the current lesson** (separate from the optional earlier-skill benchmark), so a later lesson can improve its own loss while altering earlier dialogue. Logs preserve both metrics and model fingerprints.
- The bridge ablation is a diagnostic on the learned **joint** model, not a second chatbot. Attention weights are not a guaranteed explanation of decisions.
- Logging is opt-in and observation-only. A log file error disables the recorder rather than altering model outputs/training, and every record is flushed to disk. It adds some CPU/disk overhead; disable it for performance measurements.
- New code: `app/src-tauri/src/diagnostic_log.rs`, `hybrid_diagnostic_probe`, `app/dist/js/diagnostic-ui.js`; the UI is in the existing collapsible debugger, not a new giant tab. Core brain weights and checkpoint format remain compatible and unchanged.
- Do not interpret a 399 vs 735 comparison as proof of any one pathway's fault until looking at matched no-memory logits before/after vocabulary growth and after each training epoch.


## v0.6.0 — Tests & Corrections: external teacher, one neural brain

### Test data and independent answer keys
Within **Teach → Tests & Corrections** (collapsed by default), create/save/select/delete a named test. Paste or import separate printable-ASCII TXT files (one question per line and corresponding one answer per line), or import combined `question<TAB>answer` lines into the questions box with the answer box empty. The native parser checks 1–100 pairs, rejects missing/extra answers and unsafe/mismatched input, and keeps answer keys in teacher-only metadata, **never in the model's context, episodic memory or inference output**. Optional held-out unseen question/key TXT pairs must not overlap correction questions.

### Execution and measurement
By default, test with greedy generation, 3 independent attempts/question, 128 token window and Gentle 0.001 corrective rate; seeded sampling is also available. **All baseline first attempts across every question (and optionally all unseen variations) happen before any corrective update.** After baseline, an incorrect response may receive at most one normal teacher-forced supervised pass (up to 256 ASCII reply characters) through `HybridBrain::train_batch` before another *new independent neural inference*. No correction after the final attempt, on correct answers, or on ambiguous/needs-review answers. The neural brain is not altered for assessment, retention or generalization. All attempts are recorded even after early success.

The Test Overview retains the **exact original response**, automatic and teacher-adjusted grading, answer key, sampled/greedy status, loss, gradient-update count and model step. Reports separate **untouched initial accuracy** from **post-correction accuracy**; they record training steps and how many initially incorrect answers became correct. Optional retention replay occurs after other corrections; optional generalization compares unseen questions before/after training without teaching to that set. Because prior test questions can improve later questions, **only the initial all-question baseline is untouched**.

### Grade review, safety and persistence
Conservative exact match ignores only surrounding whitespace. Capitalization/punctuation-only differences receive **Needs Teacher Review**; arbitrary semantic paraphrases are never auto-assumed correct. A human can override assessment and optionally record a different intended answer *without altering the original answer key, the original generated answer or Nova's weights*. Explicit "Send correction to lessons" opens the ordinary manual lesson editor; actual learning still requires the normal teacher action. Tests and reports are stored in `neural-nova-tests-v0.6.json`, separate from `neural-nova-hybrid-0.5.nhyb`, with safe temporary file/backup saves and interrupted-run labeling. Corrections use an atomic joint checkpoint, and an interrupted or failed partial correction rolls back in-memory weights. Pause/resume/stop operate at safe task boundaries.

### Architecture fix
The **existing dense Adam optimizer** formerly always used 0.001 regardless of the selected Transformer rate. `neural-nova/src/lib.rs` now exposes `Adam::set_learning_rate`; both `train_batch` and `train_memory_batch` use the same user-selected rate for **both neural pathways** (and the trainable Transformer bridge). This fixes the user-observed "Gentle" inconsistency; no scripted answers or second teacher brain were added. Existing weights/checkpoints are preserved. The native teacher-only code lives in `app/src-tauri/src/test_lab.rs` and `app/src-tauri/src/hybrid_lab.rs`, not in the model's inference implementation.

### Validation
Static frontend parsing and command wiring checks completed during development. Added Rust tests for separate/combined TXT parsing, conservative grading, invalid budgets, independent greedy inference, and a controlled case showing actual shared-gradient improvement. GitHub Actions builds/tests should be checked before treating a native installer as verified. Check Android and Windows job results separately; successful Rust core tests alone are not proof of native packaging.

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
