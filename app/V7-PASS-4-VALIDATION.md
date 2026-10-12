# Nova V7 Pass 4 Validation

## Validated in the preparation environment

- all runtime JavaScript files pass `node --check`;
- package and Tauri JSON parse;
- package/Tauri/Cargo metadata are aligned at `7.0.0-alpha.4`;
- Rust source remains modular and is delimiter/source-structure checked;
- JS registered cognitive operation names are represented by the Rust runtime or deliberately remain unsupported where persistent ownership has not migrated;
- native coordinator does not call the JS learned-module interpreter as a fallback;
- migrated old active JS semantic operations remain absent;
- production Rust contains no curriculum answer table or action-phrase semantic mapping;
- stable persistence path remains `%LOCALAPPDATA%\QTech\Nova\UserData`;
- ZIP integrity is checked after packaging.

## Rust tests included for external/GitHub compile

- Rust learned module executes and changes current cognition;
- explicit unavailable capability gates a module;
- broad module categories do not falsely count as produced capabilities;
- nested `invoke_module` calls cannot bypass the same capability gate;
- experience route is tried first and broadens if it stalls;
- module runtime cannot overwrite an already-supported Rust semantic answer;
- open learned request fields survive typed serialization and numeric array paths;
- reference modules run per mention and preserve extension fields;
- learned surface-family condition captures a quoted payload from learned condition data;
- recursive compound `all`/`not` conditions behave as conditions rather than defaulting to always.
- compound `all`/`any`/`not` evaluation rolls back temporary capture/context side effects from unsuccessful or negated branches;
- nested `invoke_module` execution uses the same explicit capability gate as top-level scheduling.

## Environment limitation

The preparation container does not include `cargo`, `rustc`, or `rustfmt`. Therefore the source release is not claimed to have been compiled locally. The user's/GitHub Windows build is the authoritative compiler check, as in earlier V7 passes.
