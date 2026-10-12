# Neural Nova v0.3.0 — three-tab hybrid brain lab

All new development is on `experimental/neural-nova`. The original `main` app has not been changed.

## One brain, two trainable pathways

The hybrid runs **a single neural decoder**. The dense 1024→512→512→128 pathway processes the current 64-character ASCII feature window and sends 128 continuous pre-softmax activations through a trainable 32×128 matrix into the Transformer's 32-dimensional hidden representation. The Transformer independently attends to the last 128 ASCII characters, but its final distribution is computed from the **fused** hidden representation.

The resulting one cross-entropy loss is differentiated through both pathways. It updates Transformer parameters, learned bridge parameters, and dense-network weights. There is no model answer voting, fallback selection, scripted response, or two-model chat view.

Core: `neural-nova/src/hybrid.rs` and `neural-nova/src/transformer.rs`. Native commands: `app/src-tauri/src/hybrid_lab.rs`. Frontend: `app/dist/index.html`, `app/dist/styles.css`, `app/dist/js/nova-ui.js`.

## UI

- **Teach**: text / dialogue datasets, editable lessons, file loading, teacher corrections, shared epochs, stop/reset, held-out evaluation.
- **Chat**: one Nova with combined neural generation; correct a response by entering a desired reply in Teach. This does not silently rewrite weights.
- **Visualize**: a schematic left/right brain. Real last-token attention weights, sampled dense activations, and bridge projections drive the display.
- **Debugger**: collapsible, always on the page and auto-expanded on errors.

The earlier legacy dense and standalone Transformer APIs/sources are deliberately retained in the repository but are not loaded by the new main UI.

## Checkpoints

`neural-nova-hybrid-0.3.nhyb` is **separate** from `neural-nova-transformer-0.2.ntrf` and `neural-nova-v0.1.2.nnov`. Hybrid checkpoints include both neural pathways, learned bridge, both Adam optimizer states, and step counts. Reset in the v0.3.0 UI affects only the hybrid.

Standalone v0.2 Transformer checkpoints can still be read into the augmented standalone Transformer with newly initialized unused bridge weights. The joint hybrid starts from its own new checkpoint; existing standalone weight files are not silently imported or modified.

## Requirements and limitations

128 ASCII tokens, 128-character Transformer context, 64-character dense feature window, CPU-only reference implementation, single-turn interaction. This remains a small experimental learning system; neither reliable conversation nor reasoning has been demonstrated.

```sh
cd neural-nova && cargo test --all-targets
cd ../app/src-tauri && cargo test --lib
node --check ../../app/dist/js/nova-ui.js
```

GitHub Actions produces Windows NSIS and Android debug artifacts after pushes to the experimental branch. A successful CI run does **not** establish verified device runtime behavior.
