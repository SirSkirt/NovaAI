# Neural Nova 0.2.0 — Experimental Language Engine

**All changes are isolated to `experimental/neural-nova`; original Nova `main` remains unchanged.**

## From-scratch small causal Transformer

The **new default language-training experiment** is implemented in `neural-nova/src/transformer.rs` with:
- A 128-character ASCII vocabulary and 128-character causal attention context
- Trainable token and positional embeddings, model width 32
- One causal self-attention head and an output projection
- Residual connections, a 64-wide ReLU feed-forward block, and softmax next-character prediction
- Manual, verified backpropagation through attention, projections, residual connections, embeddings and output
- Adam optimizer, gradient clipping, and persistent weights **and optimizer moments**
- A live attention heatmap showing which previous characters the query attends to

No machine-learning framework and no scripted or fallback responses are used.

## Training

The Transformer Language Engine at the top of the application supports two learning modes:
- **Language pretraining:** trains character prediction on varied ASCII text (120–14,000 bytes/session), holds out the last 10% for validation
- **Dialogue teaching:** trains **only target assistant reply characters** conditioned on the provided question (at least five distinct TAB-separated pairs), holds out entire pairs for validation

The existing dense network/teacher tools are retained **only for comparison**. They are independent models, not response fallbacks. A Transformer chat command generates exclusively from the new neural network.

## Caveats and honest expectations

This initial decoder is a *single-block, single-head character model*, not an LLM that can reason or converse reliably. It has a longer context than the previous dense version, but still only 128 **characters**, not 128 subword tokens. Thirty short dialogues cannot produce robust conversational understanding. More diverse pretraining data, better tokenization, multi-layer attention, improved evaluation and compute optimization are needed before reasoning can be tested.

Checkpoint `neural-nova-transformer-0.2.ntrf` is stored in the separate experimental application data folder; dense-network checkpoint is not modified.

`cd neural-nova && cargo test` then `cd ../app && npm install && npm run build`.
