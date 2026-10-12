# Neural Nova 0.2.0 — experimental Transformer Language Engine

See [app/README.md](app/README.md) and [the custom Rust Transformer](neural-nova/src/transformer.rs).

Neural Nova 0.2.0 adds a **from-scratch trainable causal Transformer** alongside the preserved legacy dense model, with a new language pretraining/dialogue panel and real attention inspection. The Transformer uses learned parameters, not hard-coded responses.

Original Nova remains **untouched** on `main`. All experiments and installers are isolated to `experimental/neural-nova`. This is a language-learning prototype, not yet a capable conversational or reasoning AI.
