# Nova V7 Pass 3 Validation

Validation performed in the packaging environment includes:

- JavaScript syntax checking for every runtime file;
- JSON configuration parsing;
- package/Tauri/Cargo version consistency;
- source scan for accidental production hard-coded curriculum questions;
- source scan for removed Pass 2 version/ownership identifiers;
- modularity/line-count audit of the Rust Nova tree;
- ZIP integrity test;
- Rust unit tests added for general goal construction, evidence-driven concept opening, procedure-step evidence, effect relationships, and unrelated transfer.

`cargo`/`rustc` are not installed in this packaging environment, so native Rust compilation cannot be claimed here. The user's Windows/GitHub Rust build is the authoritative compiler/test check.
