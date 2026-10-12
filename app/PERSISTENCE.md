# Nova Persistence — Pass 9 Authority, Alpha 9.3 State

## One durable authority

Nova continues to use the stable Tauri `app_data_dir()` associated with `com.qtech.nova`:

- `nova-state.json`
- `nova-state.backup.json`
- `nova-state.tmp.json`

The application version is never part of the path. The retired `%LOCALAPPDATA%\QTech\Nova\UserData` location remains migration input only under the alpha 6.1 migration rules.

## Rust owns the mind payload

Pass 9 replaces JavaScript full-payload orchestration with Rust `PersistentMindEnvelope` serialization. The envelope separates:

- `mind` — direct Rust `NovaMind` state;
- `ui` — opaque chat/Thinking/technical-telemetry presentation state;
- `persistence` — schema/version/timestamp/reason metadata.

Alpha 9.3 keeps Rust-owned `teacher_state` inside `NovaMind` and adds the Rust Basic English observation to canonical cognitive state. Alpha 9.2 state restores through serde defaults and Rust normalization into mind schema 12, teacher schema 2 and semantic schema 5. No parallel teacher or language-state file is created.

JavaScript cannot submit a replacement mind. `nova_mind_checkpoint` takes a locked Rust snapshot itself, accepts only UI state from the frontend, serializes the envelope in Rust and commits it through the existing atomic writer.

## Startup

`nova_mind_bootstrap` loads the newest valid state through the existing alpha 6.1 recovery/migration rules. A Pass 9 envelope restores `NovaMind` directly. An older alpha 8.1 payload is migrated inside Rust by extracting the existing Rust-owned mirrors and prior visible conversation history. After migration, Rust commits the Pass 9 envelope immediately.

Native JavaScript does not arbitrate localStorage against the mind file, hydrate cognition, rebuild mirrors, run post-load brain maintenance or synchronize learned state.

## Atomic write contract

The proven flow remains:

1. serialize/validate the complete Rust-owned envelope;
2. write and fsync temp;
3. preserve a valid current snapshot as backup;
4. commit temp to current;
5. fsync current;
6. reread and verify exact committed bytes;
7. restore the known-good backup if commit verification fails.

Frontend checkpoint requests are serialized through one promise chain so multiple UI events cannot race concurrent native commits.
