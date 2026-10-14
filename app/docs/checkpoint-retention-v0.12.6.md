# Neural Nova v0.12.6 — protected joint checkpoint selection

## Problem
A previous 'NEW BEST' label meant **eligible by a weighted joint score**, not lowest held-out loss. The older-skills guard previously tolerated 0.10 degradation in an average teacher-forced loss, and complete generated answers from earlier archived lessons were not protected. Small permitted regression over multiple selected epochs could also accumulate relative to the true lesson-start baseline.

## Selection criteria
A unified lesson candidate is accepted only if:
1. The current composite held-out loss remains no more than 0.08 above the **fixed, original lesson baseline**, and at most 0.08 above the last selected checkpoint; these are training-only selection tolerances, not inference rules.
2. The same lesson's self-generated heldout score improves its weighted joint criterion, preserves all previously correct complete answers on that lesson and its first-character accuracy, and does not meaningfully increase its self-conditioned loss.
3. Previous archived heldout loss remains within the user's explicit tolerance. New users default to **0.02**, not 0.10. Previously saved user choices remain respected.
4. **Representative older and newer archived response panels** are re-evaluated by actual greedy, self-conditioned neural generation at candidate time. They must retain the correct complete answers and first characters that the baseline could generate. The panel is immutable for the whole current session, and its teacher answers never enter inference or rehearsal.
5. Existing optional saved teacher benchmark and the same-weights checkpoint persistence/rollback rules remain.

If nothing qualifies, the trainer retains the baseline and the ZIP queue reports the lesson *skipped*, not learned. Real save failures still terminate the run. This increases honesty rather than guaranteeing training progress.

## Interpretation
Epoch history now says **SELECTED (joint score)** or **NOT SELECTED**, not 'NEW BEST'. It reports (a) the current held-out loss, (b) the fixed current-lesson baseline, (c) earlier held-out loss vs its own baseline, (d) the current autonomous probes, (e) previously selected complete-answer performance, (f) older independent-response panel retention, and (g) a stable heldout-set identifier. Compare baseline and epoch screenshots only when the evaluation set identifier is identical.

A bounded 4–8 prior-reply probe panel is a **safeguard**, not a complete guarantee against forgetting or independent proof of reasoning. Real learning quality still requires subsequent unseen dialogue, property, quantity, transfer, unknown-information and multistep tests. Do not confuse validation used to choose a checkpoint with a separately withheld final evaluation.

## Migration
The .nhyb format, Rust neural model, original brain visualizer, Android package/signature and single shared generative neural system are unchanged. Finish training and export a checkpoint backup before updating. The older-lesson rehearsal bank has its own persistence; exporting .nhyb alone may not export that archive. Do not delete the prior checkpoint until the new version's output has been independently checked.
