use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ids::stable_hash_id;
use super::mind::unix_ms;

const MAX_INTERNAL_GOALS: usize = 160;
const MAX_INTERNAL_HISTORY: usize = 240;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalGoal {
    pub id: String,
    pub kind: String,
    pub description: String,
    pub status: String,
    pub source: String,
    pub importance: f32,
    #[serde(default)]
    pub evidence: Vec<Value>,
    pub created_at: u128,
    pub updated_at: u128,
    pub attempts: u32,
    pub last_reason_code: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCognitionEvent {
    pub id: String,
    pub goal_id: Option<String>,
    pub event_type: String,
    pub detail: String,
    pub at: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustInternalCognitionState {
    pub schema_version: u32,
    pub initialized: bool,
    pub revision: u64,
    #[serde(default)]
    pub goals: Vec<InternalGoal>,
    #[serde(default)]
    pub history: Vec<InternalCognitionEvent>,
    pub updated_at: u128,
}

impl RustInternalCognitionState {
    pub fn new_initialized() -> Self {
        Self { schema_version: 1, initialized: true, revision: 1, goals: Vec::new(), history: Vec::new(), updated_at: unix_ms() }
    }

    pub fn queue_goal(&mut self, kind: &str, description: &str, source: &str, importance: f32, evidence: Vec<Value>) -> Option<String> {
        let description = description.split_whitespace().collect::<Vec<_>>().join(" ");
        if description.is_empty() { return None; }
        if !self.initialized { *self = Self::new_initialized(); }
        let signature = format!("{}|{}|{}", kind.trim().to_ascii_lowercase(), description.to_ascii_lowercase(), source.trim().to_ascii_lowercase());
        if self.goals.iter().any(|goal| goal.status == "QUEUED" && format!("{}|{}|{}", goal.kind.to_ascii_lowercase(), goal.description.to_ascii_lowercase(), goal.source.to_ascii_lowercase()) == signature) {
            return None;
        }
        let now = unix_ms();
        let id = stable_hash_id("internal_goal", &format!("{}|{}", signature, self.revision));
        self.goals.push(InternalGoal { id: id.clone(), kind: kind.to_string(), description, status: "QUEUED".to_string(), source: source.to_string(), importance: importance.clamp(0.0, 1.0), evidence, created_at: now, updated_at: now, attempts: 0, last_reason_code: None });
        if self.goals.len() > MAX_INTERNAL_GOALS { self.goals.remove(0); }
        self.record(Some(id.clone()), "GOAL_QUEUED", source);
        Some(id)
    }

    pub fn next_goal(&self) -> Option<InternalGoal> {
        self.goals.iter().filter(|goal| goal.status == "QUEUED")
            .max_by(|left, right| left.importance.partial_cmp(&right.importance).unwrap_or(std::cmp::Ordering::Equal).then_with(|| right.created_at.cmp(&left.created_at)))
            .cloned()
    }

    pub fn reopen_blocked_after_learning(&mut self) -> usize {
        let now = unix_ms();
        let mut reopened = 0;
        for goal in self.goals.iter_mut().rev().take(12) {
            if goal.status == "BLOCKED" {
                goal.status = "QUEUED".to_string();
                goal.importance = (goal.importance * 0.92).max(0.45);
                goal.updated_at = now;
                reopened += 1;
            }
        }
        if reopened > 0 { self.record(None, "BLOCKED_GOALS_REOPENED_AFTER_LEARNING", &reopened.to_string()); }
        reopened
    }

    pub fn mark_result(&mut self, goal_id: &str, terminal_state: &str, reason_code: &str) {
        let now = unix_ms();
        if let Some(goal) = self.goals.iter_mut().find(|goal| goal.id == goal_id) {
            goal.attempts = goal.attempts.saturating_add(1);
            goal.status = match terminal_state { "SUPPORTED" => "SUPPORTED", "BLOCKED" => "BLOCKED", _ => "EXHAUSTED" }.to_string();
            goal.last_reason_code = Some(reason_code.to_string());
            goal.updated_at = now;
        }
        self.record(Some(goal_id.to_string()), "GOAL_EVALUATED", reason_code);
    }

    fn record(&mut self, goal_id: Option<String>, event_type: &str, detail: &str) {
        let now = unix_ms();
        let id = stable_hash_id("internal_event", &format!("{}|{}|{}|{}", event_type, goal_id.as_deref().unwrap_or(""), detail, self.revision));
        self.history.push(InternalCognitionEvent { id, goal_id, event_type: event_type.to_string(), detail: detail.to_string(), at: now });
        if self.history.len() > MAX_INTERNAL_HISTORY { self.history.remove(0); }
        self.revision = self.revision.saturating_add(1);
        self.updated_at = now;
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCognitionImportInput {
    pub persisted_state: Option<RustInternalCognitionState>,
    pub reason: Option<String>,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCognitionImportReceipt {
    pub accepted: bool,
    pub imported: bool,
    pub authority: String,
    pub reason: String,
    pub state: RustInternalCognitionState,
}

#[cfg(test)]
pub fn import_internal_cognition(input: InternalCognitionImportInput, current: &RustInternalCognitionState) -> InternalCognitionImportReceipt {
    if current.initialized {
        return InternalCognitionImportReceipt { accepted: true, imported: false, authority: "rust_v7_pass_9".to_string(), reason: "rust_internal_cognition_already_initialized".to_string(), state: current.clone() };
    }
    let mut state = input.persisted_state.unwrap_or_else(RustInternalCognitionState::new_initialized);
    state.initialized = true;
    state.schema_version = 1;
    state.updated_at = unix_ms();
    InternalCognitionImportReceipt { accepted: true, imported: true, authority: "rust_v7_pass_9".to_string(), reason: input.reason.unwrap_or_else(|| "pass8_internal_cognition_restore".to_string()), state }
}
