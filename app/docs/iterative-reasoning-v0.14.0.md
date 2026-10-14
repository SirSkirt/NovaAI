# Neural Nova v0.14.0 — Iterative evidence reasoning foundation

## What changed
The previous network had three learned latent refinement steps and two reciprocal Dense exchanges, but reused the original pooled attention for all three steps. The new Transformer rereads its original input tokens during each of the THREE recurrent steps, with a learned query generated from the CURRENT hidden state after dense exchange. It can attend to different evidence as its state evolves. The reread output is fused into the SAME hidden representation and the SAME existing text decoder. Backpropagation is exact through the softmax token attention, key/value matrices, evidence query, earlier steps, Dense communications and shared state.

Two 32×32 parameter groups, evidence_query and evidence_response, are appended after ALL v0.13.1 tensors. The response projection is initialized to zero; importing older checkpoints preserves existing predictions and Adam moments before the new pathway learns. There are no hardcoded facts, inference-time string matching, scripted answers or auxiliary solver.

## A genuinely different training objective
Reasoning lessons optionally provide 1–3 ordered EVIDENCE PHRASES copied exactly from the SOURCE FACTS. Each supervised recurrent step learns to focus probability mass on its corresponding source phrase, not to predict the answer text. That objective trains learned evidence-attention separately from ordinary next-character loss and the supported-versus-contradicted answer ranking objective. The teacher annotation and matching code NEVER execute in chat inference; inference uses only weights learned from training.

### Unified lessons
Within an existing [Reasoning Lesson] block, append the OPTIONAL Evidence field to an ordinary comparison:

Premise: Asha has a blue key. Asha puts the key in a red box. Question: Where is the key? Supported: Red box. Contradicted: Blue bag. Evidence: Asha has a blue key. || Asha puts the key in a red box.

Each phrase must be ASCII, at least 3 characters, and occur exactly once in the premise; separate two or three pieces with space-double-pipe-space. Evidence order is the order taught for successive latent reads. Without Evidence:, the original reasoning lessons behave as before.

### Standalone Reasoning TXT
Old format: PREMISE TAB QUESTION TAB SUPPORTED TAB CONTRADICTED.
Optional fifth field: TAB EVIDENCE1 || EVIDENCE2 [|| EVIDENCE3].
Use actual tab characters between the original four fields, and the literal || separator between evidence phrases. Five independent question/premise groups are still required. Holdout is by complete prompt groups; withheld evidence cannot enter train updates.

## Where learning happens
Unified Training adds an optional lower-rate evidence alignment step approximately every four supervised batches. Targeted Reasoning Training supports the same objective with the selected update mask (Dense, Transformer, Interconnect, or Joint). Existing clipping, checkpoint selection, replay, joint output loss and response inference remain in place.

The saved observatory activity reports per-round top attended source tokens through recurrentEvidenceTopTokens. Keep the original brain drawing unchanged. This does not decode hidden thoughts.

## Verification / limitations
Core Rust regression tests cover exact older-weights migration, finite-difference gradients through recurrent evidence and upstream token matrices, optional evidence parser, train/validation isolation, and optimizer masks. Verify core, native Rust, Windows packaging and Android APK on final commit. Back up existing .nhyb and teacher archive before upgrading. Old v0.13.1 executables may reject v0.14.0 checkpoint tensor counts.

This completes a concrete missing architectural mechanism and intermediate-step training loss, NOT the entire scientific problem of generalized reasoning. Nova remains a small 32-dimensional, single-head model; genuine multi-hop reasoning, compositional generalization and autonomous answer accuracy have to be measured on independent examples. Don't mistake good teacher-forced evidence attention, green connections or lower loss for proven reasoning.
