# Neural Nova v0.1.2 — experimental brain lab

Only the `experimental/neural-nova` branch was changed. Original Nova's `main` remains untouched.

The **custom Rust feed-forward neural network** uses the same 1024 → 512 → 512 → 128 architecture (853,120 parameters). Neural Nova v0.1.2 expands input context from eight to **64 ASCII characters**, represented as 16 deterministic character features each (7 bipolar ASCII bits plus letter/case/digit/punctuation/sentence-boundary features). No ML frameworks.

## More capable training studio

- Enter multiple varied, full sentences or load an ASCII text file, up to 250,000 bytes per run.
- Shuffle training positions per epoch, and vary contextual history (8, 16, 32, 64 chars) to support both short and long prompts.
- Hold out the last 10% of each corpus for **validation loss and next-character accuracy**. This is a useful basic check, but repeated texts can leak statistical patterns into validation; it is not a benchmark of general language understanding.
- Display live progress, epoch losses, and validation statistics. Stop training without losing the last epoch's weights.
- Inspect all 2,176 real neurons and sampled weighted links; generate continuations with temperature.
- Experimental weights are saved as `neural-nova-v0.1.2.nnov` in the independent app-data directory. Existing v0.1 weights remain untouched but are **not compatible** with the new input encoding, so training starts fresh.

## Run
```sh
cd neural-nova && cargo test
cd ../app && npm install && npm run tauri build
```

The output is still a distribution over 128 ASCII characters. **This is a fixed-window feed-forward character model, not a Transformer or conversational LLM.** Longer prompts only provide the final 64 characters of context. Better training does not imply factual reasoning or comprehension. CPU training on large corpora can be slow.
