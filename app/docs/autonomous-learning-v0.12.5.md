# Neural Nova v0.12.5 — autonomous education evaluation

This training update does NOT declare Nova a fluent or independently reasoning AI. Nova's existing dense/context pathway, Transformer, learned recurrent two-way exchange, tokenizer, 12-slot neural memory, shared output generator and checkpoint format stay unchanged. The existing brain visualizer is untouched.

## Why the previous trainer could accept incoherent language
- Teacher-forced validation knew every previous correct answer character.
- Actual chat must use Nova's own generated characters as context.
- The old selected checkpoint was the lowest weighted character-level/contrastive loss (subject to retention), while independently generated complete answers were *optional diagnostics only*.
- The exported v0.12.4 checkpoint with 15 accepted / 12 skipped lessons therefore is evidence that the optimizer found improvements, NOT that Nova mastered 15 lessons.

## New training behavior
1. Explanatory prose is trained under a separate teacher-context prefix `T: `, not as Nova's user-chat response. Explanations still update her SHARED neural weights, with no scripted parser or separate model at inference.
2. Unified teaching schedules an auxiliary reply-boundary mini-batch after up to eight ordinary batches. Teacher-supplied first characters and learned newline terminations from Dialogue, Understanding, Multi-Turn or Reasoning are included. Each category cycles; heldout rows and instructional prose never enter these batches. Accounting includes additional samples.
3. Held-out **autonomous validation** uses one independent question from each represented response objective. Nova generates the next character greedily, then conditions on her OWN output rather than the teacher's preceding answer. The teacher scores the generated prefix (at most 12 characters), first-character correctness, and complete answers where the answer plus newline fits in the limit.
4. A checkpoint must improve existing composite teacher-forced heldout loss, pass earlier-skill retention guards, AND meet an independent-generation safeguard. The self-conditioned loss cannot deteriorate by >0.05; first-character accuracy or already correct complete replies may not regress. A weighted teacher-forced + self-conditioned validation score must also improve. When Nova is still a novice with 0 correctly generated complete replies, smooth autonomous heldout loss can improve without imposing impossible fluency requirements. These conditions are **teacher checkpoint selection**, not coded answers, parsing, lookup tables or changes to Nova's inference.
5. The Teach panel exposes BEST CHECKPOINT autonomous first-character accuracy, self-generated prefix accuracy, and complete held-out replies. The old accuracy label is now explicitly next-character accuracy. No changes to the visualizer.

## What this does NOT prove
- Twelve characters and one held-out question per category are a bounded diagnostic, not a comprehensive test of comprehension or multi-step generalization.
- A checkpoint passing these gates is NOT proof Nova can converse. Passing A/B architecture or backprop tests also is not proof of reasoning.
- Novel combinations, more conversation turns, unknown-fact handling and transfer/quantity tests must be evaluated independently of all training/rehearsal and model-selection probes.
- The current ~964k-parameter / 32-dimensional language pathway may require additional capacity, a better input representation, and a substantially larger balanced curriculum if genuine generalization remains weak. Do NOT introduce scripted mathematical solvers or hidden external models.

## Safe retraining on v0.12.5
- Keep the already-exported `.nhyb` backup and finish any running training before updating the app. The Android development signing identity, package ID and checkpoint format remain unchanged.
- After installation, if starting over is intended, use Nova's explicit reset control (only AFTER verifying the export). This may remove the previous teacher rehearsal archive; exporting just the neural checkpoint is not the same as saving the archived lesson material.
- Review the chosen learning rate: existing local preferences may retain the user's prior rate. It is reasonable to TEST 0.0003 against 0.001, but no rate has yet been proven superior for Nova.
- Validate a ZIP with the four structured block types, then run sequential teaching. Watch accepted/skipped counts, early stopping and now the autonomous metrics. If the start/prefix metrics remain zero across lessons, do NOT call the curriculum learned.
- Compare an unchanged initial checkpoint with/without multi-objective training and evaluate genuinely unseen entity/property/quantity combinations. Preserve old learned checkpoints, use rollback, and keep the brain visualizer untouched.
