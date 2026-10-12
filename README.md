# Neural Nova Experimental

**Isolation:** This is the experimental neural-network fork of Nova on branch `experimental/neural-nova`. The original Nova application remains on `main`, unmodified.

The full legacy Nova project was first unpacked into the branch. Its old Rust-authoritative mind, rule-based cognition, memory, learning, and JavaScript brain were then removed from the experimental app. The experimental application now uses its own **from-scratch Rust neural network** (1024 → 512 → 512 → 128), with live per-neuron visualization, training, next-character generation, checkpointing, cancellation and resetting.

- [Neural Nova experimental application](app/)
- [Neural math library and tests](neural-nova/)
- [Windows/Android experimental build workflow](.github/workflows/neural-nova-app.yml)
- [Original legacy source archive](releases/) (historical only; not built or executed)

**Warning:** v0.1 is an eight-character ASCII next-character predictor, not a general conversational LLM. This is the first learning-system experiment before tackling embeddings and Transformers.

The original Nova package ID was `com.qtech.nova`. Neural Nova uses `com.qtech.neuralnova.experimental` and saves its checkpoints in its own app-data location. Builds here never publish or replace original Nova releases.
