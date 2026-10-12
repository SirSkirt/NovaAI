mod capabilities;
mod synthesis;
mod types;

pub use synthesis::{replace_rust_teaching_source, synchronize_learning};
pub use types::{
    LearnedRuleRecord, LearningSynthesisReport, LearningSyncInput, RuleSynthesisCandidate,
    RustLearningState,
};

#[cfg(test)]
pub use types::LearningSyncReceipt;
