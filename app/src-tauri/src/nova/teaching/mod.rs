mod cognition;
mod parser;
mod semantic;
mod types;

pub(crate) use cognition::translate_cognition;
pub(crate) use parser::{parse_source, stable_source_id};
pub(crate) use semantic::apply_semantic_learning;
pub use types::{ParsedTeachingSource, TeachingIngestionReceipt, TeachingSourceInput};
