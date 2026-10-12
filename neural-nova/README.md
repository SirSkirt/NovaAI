# Neural Nova v0.1.2 — from-scratch Rust neural network

The custom engine uses **1024 → 512 → 512 → 128** dense layers, ReLU, softmax, cross-entropy, backpropagation, Adam, global gradient clipping, He initialization and binary checkpoints. No external machine-learning crate.

Inputs now represent **64 recent ASCII characters × 16 features each**. The encoding is implemented in `encode_ascii_context`, shared by both the GUI and CLI trainer. Padded positions are all zero. A real space has its own nonzero encoded features.

Run:
```sh
cd neural-nova
cargo test
cargo run --release --bin train_ascii -- data/example.txt 2 checkpoint.nnov
```

The GUI training loop additionally shuffles positions per epoch, trains on multiple contextual prefix lengths, and holds out 10% of each corpus for loss/accuracy validation. The 128 outputs are next-character probabilities. This is not yet a Transformer, long-context language model, or conversational assistant.

**Branch isolation:** all changes are to `experimental/neural-nova`. Legacy Nova remains untouched on `main`, and the original project's source archive is preserved under `releases/`.
