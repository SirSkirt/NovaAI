# Neural Nova v0.12.8 — Targeted Training

Targeted Training is a temporary **teacher workspace**, not a second inference engine. Nova still has one dense/Transformer cooperative brain and a single generative output. The original brain visualizer is **unchanged**.

## What the controls do

- **Starting checkpoint**: Current = clone active Nova's whole neural state; Imported = transfer an external compatible `.nhyb` file into an isolated candidate workspace. Import is bounded (24 MiB) and validated using the existing Rust `HybridBrain::load`; **import does not replace the active checkpoint**.
- **Optimizer target**: Dense = update only dense network weights and Adam state; Transformer = update only language/attention/memory/recurrent-reasoning Transformer weights, with bridge/reciprocal matrices frozen; Interconnect = update only Transformer-owned learned dense-to-Transformer bridge and reciprocal query/response matrices; Joint = update both hemispheres plus connection. ALL pathways still run during loss and backpropagation; a frozen pathway is neither disconnected nor replaced by hardcoded reasoning.
- **Learning objective**: Dialogue, contextual Understanding, multi-turn Conversation, or Reasoning. Supply structured `[... Lesson]` blocks with at least five independent examples of the selected type. The strict existing teacher parser separates train/heldout groups; Targeted Training filters SUPERVISED examples by that category. Understanding/Reasoning also train the existing learned whole-answer contrastive preference objective.
- **Experiment**: Native Rust trains a cloned candidate asynchronously, retaining the active model untouched. The best independently validated candidate is staged in RAM. Neither imported sources nor focused candidates are saved to the active `.nhyb` without an explicit approval.
- **Review**: Before/after values use identical held-out groups for the current objective and sampled archived earlier abilities: next-character held-out loss/accuracy, autonomous greedy prefix/first character/complete short replies (up to 32 characters), and contrastive accuracy when available. They are TEACHER validation, **not an unseen final exam or evidence of generalized reasoning**.
- **Accept / Discard**: Acceptance creates a timestamped backup of the previous active checkpoint, verifies live weights have not changed since the experiment began, and saves the ENTIRE accepted cooperative brain atomically. Chat history clears. The teacher must explicitly opt in and reconfirm if any sampled retention regression is detected. Discard does not touch live weights.
- **Back to Teach**: Returning to normal Teach uses the new whole-brain baseline and jointly trains every pathway. Training on one region cannot by itself establish that region learned an isolated human-interpretable attribute. The small 64-character dense input context remains a fundamental limitation.

## Safeguards and limitations
- An imported checkpoint can contain a different tokenizer and optimizer state; acceptance installs the full compatible checkpoint, never copies individual parameters from incompatible lineages.
- Existing `.nhyb` binary format, app package, Android experimental signing certificate and neural architecture remain unchanged.
- The separate rehearsal archive is not magically included with `.nhyb`; after an imported candidate is accepted, only matching archived rehearsal material is eligible. A mismatched archive is backed up by the normal archive mechanism.
- Targeted candidates exist in app memory, so **export a checkpoint and accept/discard before closing the app**. Training/validation runs in local native Rust; no cloud assistant computes Nova's inference.
- CPU training cost depends on number of supervised characters, requested epochs and model capacity. Stopping is cooperative at the next mini-batch boundary.
- This is **training-time specialization of gradient updates**, not guaranteed disentanglement of cognition by hemispheres. Interconnect is its own parameter group, but inference still uses both hemispheres continuously.

## Verification
The Rust optimizer tests assert frozen Transformer and dense weights AND optimizer moments remain bit-for-bit unchanged, while targeted groups can update. Tests also check filter selection, incomplete inputs and regression warnings. The experimental CI verifies Rust+JavaScript checks plus Windows NSIS and Android APK packaging.
