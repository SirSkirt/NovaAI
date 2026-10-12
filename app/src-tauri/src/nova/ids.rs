use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }
    };
}

string_id!(TurnId);
string_id!(FrameId);
string_id!(EntityId);
string_id!(EventId);
string_id!(RelationId);
string_id!(PropositionId);
string_id!(RequestId);
string_id!(ConceptId);
string_id!(ModuleId);
string_id!(GoalId);
string_id!(EvidenceId);
string_id!(CapabilityKey);


/// Deterministic 64-bit FNV-1a identifier for persisted Rust-owned mind records.
/// Do not use `DefaultHasher` for persisted IDs: its algorithm is not a file-format contract.
pub(crate) fn stable_hash_id(prefix: &str, signature: &str) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001B3;
    let mut hash = OFFSET;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{prefix}_{hash:016x}")
}

fn stable_source_component(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace('_', " ").replace('-', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a source name itself is a durable identity. Generic `document` and
/// `conversation` labels are containers/events rather than named source files.
pub(crate) fn source_has_named_identity(name: &str) -> bool {
    let name_key = stable_source_component(name);
    !name_key.is_empty() && !matches!(name_key.as_str(), "document" | "conversation")
}

/// Stable identity for source-backed learned state. Named documents retain the
/// same identity across edits so re-teaching replaces their prior contribution.
/// Anonymous/default and conversation sources use content identity unless the
/// caller supplies an explicit source ID.
pub(crate) fn stable_source_id(name: &str, text: &str) -> String {
    let name_key = stable_source_component(name);
    let signature = if source_has_named_identity(name) {
        format!("named|{name_key}")
    } else {
        format!("anonymous|{}", stable_source_component(text))
    };
    stable_hash_id("source", &signature)
}
