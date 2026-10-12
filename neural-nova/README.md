# Neural Nova — isolated experimental Rust brain

**Branch only:** `experimental/neural-nova`. This crate is additive; it does not remove Nova's old brain or modify `main`.

Neural Nova v0.1 is a **from-scratch, feed-forward neural network**, built using only the Rust standard library. It includes row-major dense layers, He initialization, ReLU, softmax, cross-entropy loss, analytic backpropagation, mini-batch gradient averaging, global gradient clipping, Adam, and binary parameter checkpoints.

## Architecture

`1024 → 512 → 512 → 128` (two hidden layers), with **853,120** trainable weights and biases.

## Build and verify

```sh
cd neural-nova
cargo test
cargo run --release --bin train_ascii -- data/example.txt 2 checkpoint.nnov
```

The proof-of-concept language trainer uses **8 ASCII characters × 128 one-hot values = 1024 inputs** to predict the next ASCII character (128 outputs). It is deliberately small-context and *not yet an LLM*. Its job is to test whether gradients, training, and persistence work.

The tests check configuration/parameter count, softmax probability normalization, analytic gradients against finite differences, loss reduction on a simple learning task, and exact checkpoint round-tripping.

## Safety and isolation

- All files are under `neural-nova/`; original source archive remains under `releases/`.
- Do not merge to `main` or publish over the original Nova releases.
- Nova's existing brain remains intact while this experimental core is validated.
- The current app source is in a ZIP in this Git repository. Use the experimental-only unpack workflow to expose the source before replacing cognition components.
- Future work: implement trainable embeddings, attention, residuals, layer norm, Transformer blocks, validation-split benchmarks, optimizer checkpointing, and teacher UI integration.
