# Neural Nova v0.15.0 — Foundation Training v1

## Training policy
Opt-in training for the first FIVE completed Foundation lesson sessions, then automatic return to ordinary joint training. Foundation progress is persisted in the checkpoint-matched rehearsal archive and survives app restarts and sequential native ZIP teaching; restored unrelated checkpoints cannot inherit old counters. Cancelled runs do not advance the count, but completed rejected lessons do. Disabling the switch pauses foundation training without resetting progress.

## Four mutually exclusive complete-brain candidates
For each epoch, clone the SAME complete accepted HybridBrain including Dense, Transformer, bridge, learned tokenizer and all Adam moments and update counters. The same ordered batches are presented to four candidates: Dense-only, Transformer-language-only, interconnect-only (bridge/reciprocal/evidence), and normal Joint. Only the gradient update masks differ: all four still compute through the same forward neural networks. Each trained candidate is scored as a COMPLETE brain. Select the best eligible WHOLE brain with its optimizer state, never merging unrelated pathway arrays. Rejected branches are discarded. The next epoch again starts all four from the previous accepted complete checkpoint, and a lesson without improvements retains its baseline.

## Candidate scoring and guardrails
Every candidate gets the same current-lesson heldout composite/next-character evaluation. When full-answer probes exist, autonomous greedy rollout scores include real complete replies, self-conditioned likelihood and first-character accuracy; an improvement in teacher-forced loss cannot override independent generated-answer regression guards. Checkpoint selection uses the existing 0.35 autonomous likelihood weighting, early-stop policy, fixed baseline tolerance, and the strict archived earlier-lesson heldout loss, prior autonomous answer, and optional user-selected Test & Corrections benchmark guards. New teacher targets are SCORING ONLY and cannot train the checkpoint. Every candidate's eligibility and score appears in private epoch-observer JSONL and on-screen history.

Legacy Dialogue, Reasoning and Conversation lessons now provide full heldout reply probes. Conversation probes include actual prior turns. Legacy Text still has no task-specific complete reply and therefore uses heldout character loss plus historical retention.

## Running
Enable the Teach tab checkbox under Foundation Training v1. Keep Rehearsal and previous-skill protection on: native training rejects foundation mode if disabled. Start one lesson or a native sequential ZIP normally. Progress includes four candidate passes each epoch; this can take about 4x ordinary training time on Android (plus extra evaluation overhead). The UI displays 0–5 completed foundation lessons, candidate score comparisons, and automatic transition to ordinary joint learning after five sessions. The Targeted Training tab is unchanged.

## Compatibility
v0.14.0 neural architecture, .nhyb complete-checkpoint format, trained network weights, Adam state, tokenizer, original SVG brain visualizer, Android signing key and package identifier are untouched. Older rehearsal archives load with default foundation counter zero; unrelated restored checkpoints reset only the teacher-side counter, never mutate neural weights. Physical Android runtime/performance validation is still needed. Back up the existing .nhyb before starting any long training.

Foundation v1 is an experiment to test learning and retention; it is not proof that Nova can reason, and it does not assume the Transformer is defective. Compare autonomous answers on novel questions and the original control under comparable training budgets.