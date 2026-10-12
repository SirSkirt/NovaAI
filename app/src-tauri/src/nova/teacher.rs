use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ids::stable_hash_id;
use super::mind::unix_ms;
use super::semantics::types::CognitiveTraceEvent;

pub const RUST_TEACHER_SCHEMA: u32 = 2;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeacherSpanAnnotation {
    pub id: String,
    pub target: String,
    pub start: usize,
    pub end: usize,
    pub style: String,
    pub note: Option<String>,
    pub created_at: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeacherAttemptRecord {
    pub id: String,
    pub run_id: String,
    pub suite_id: String,
    pub case_id: String,
    pub source_display_name: String,
    #[serde(default)]
    pub lesson_context: Option<String>,
    #[serde(default)]
    pub attempt_number: u32,
    pub question: String,
    pub original_answer: String,
    pub expected_answer: Option<String>,
    pub expected_behavior: Option<String>,
    pub automatic_status: String,
    pub automatic_reason: String,
    pub terminal_state: Option<String>,
    pub reason_code: Option<String>,
    #[serde(default)]
    pub cognitive_trace: Vec<CognitiveTraceEvent>,
    #[serde(default)]
    pub cognitive_context: Value,
    #[serde(default)]
    pub inspector_context: Value,
    #[serde(default)]
    pub teacher_points_awarded: Option<f32>,
    #[serde(default)]
    pub teacher_points_possible: Option<f32>,
    #[serde(default)]
    pub teacher_grade: Option<String>,
    #[serde(default)]
    pub teacher_credit_percent: Option<f32>,
    #[serde(default)]
    pub teacher_notes: String,
    #[serde(default)]
    pub annotations: Vec<TeacherSpanAnnotation>,
    #[serde(default)]
    pub evidence_released: bool,
    pub knowledge_id: String,
    #[serde(default)]
    pub provenance: Value,
    pub created_at: u128,
    pub updated_at: u128,
}

impl TeacherAttemptRecord {
    pub fn effective_credit_percent(&self) -> Option<f32> {
        self.teacher_credit_percent
            .or_else(|| match (self.teacher_points_awarded, self.teacher_points_possible) {
                (Some(awarded), Some(possible)) if possible > 0.0 => Some((awarded / possible * 100.0).clamp(0.0, 100.0)),
                _ => None,
            })
            .or_else(|| match self.automatic_status.as_str() {
                "pass" => Some(100.0),
                "fail" => Some(0.0),
                _ => None,
            })
    }

    pub fn is_partial_credit(&self) -> bool {
        self.effective_credit_percent().is_some_and(|value| value > 0.0 && value < 100.0)
    }

    pub fn knowledge_text(&self) -> String {
        let mut parts = vec![
            format!("Teacher feedback and test history from {}.", self.source_display_name),
            format!("Question: {}", self.question),
            format!("Nova's preserved original answer: {}", if self.original_answer.trim().is_empty() { "[no spoken answer]" } else { self.original_answer.trim() }),
        ];
        if let Some(expected) = self.expected_answer.as_deref().filter(|value| !value.trim().is_empty()) {
            parts.push(format!("Expected answer used for grading: {}", expected.trim()));
        } else if let Some(expected) = self.expected_behavior.as_deref().filter(|value| !value.trim().is_empty()) {
            parts.push(format!("Expected behavior used for grading: {}", expected.trim()));
        }
        if let Some(context) = self.lesson_context.as_deref().filter(|value| !value.trim().is_empty()) {
            parts.push(format!("Lesson context: {}.", context.trim()));
        }
        parts.push(format!("Attempt number: {}.", self.attempt_number.max(1)));
        if let (Some(awarded), Some(possible)) = (self.teacher_points_awarded, self.teacher_points_possible) {
            parts.push(format!("Teacher score: {awarded}/{possible}."));
        }
        if let Some(grade) = self.teacher_grade.as_deref().filter(|value| !value.trim().is_empty()) {
            parts.push(format!("Teacher grade: {}.", grade.trim()));
        }
        if let Some(credit) = self.effective_credit_percent() {
            parts.push(format!("Credit recorded: {:.1}%. Partial credit: {}.", credit, self.is_partial_credit()));
        }
        if !self.automatic_reason.trim().is_empty() {
            parts.push(format!("Automatic grading context: {}", self.automatic_reason.trim()));
        }
        if !self.teacher_notes.trim().is_empty() {
            parts.push(format!("Teacher notes: {}", self.teacher_notes.trim()));
        }
        if !self.cognitive_trace.is_empty() {
            let trace = self.cognitive_trace.iter()
                .map(|event| format!("{}: {}", event.stage.replace('_', " "), event.text.trim()))
                .collect::<Vec<_>>()
                .join(" | ");
            if !trace.trim().is_empty() {
                parts.push(format!("Nova's preserved reasoning trace for this attempt: {}", trace));
            }
        }
        for annotation in &self.annotations {
            let source = if annotation.target == "question" { &self.question } else { &self.original_answer };
            if let Some(excerpt) = utf16_excerpt(source, annotation.start, annotation.end) {
                let mut marked = format!("Teacher {} marked {} text: \"{}\".", annotation.style, annotation.target, excerpt);
                if let Some(note) = annotation.note.as_deref().filter(|value| !value.trim().is_empty()) {
                    marked.push_str(&format!(" Mark note: {}", note.trim()));
                }
                parts.push(marked);
            }
        }
        parts.push("This record is preserved teacher/test evidence and is non-executable knowledge.".to_string());
        parts.join(" ")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTeacherState {
    pub schema_version: u32,
    pub initialized: bool,
    pub revision: u64,
    #[serde(default)]
    pub attempts: Vec<TeacherAttemptRecord>,
    pub updated_at: u128,
}

impl RustTeacherState {
    pub fn new_initialized() -> Self {
        Self {
            schema_version: RUST_TEACHER_SCHEMA,
            initialized: true,
            revision: 0,
            attempts: Vec::new(),
            updated_at: unix_ms(),
        }
    }

    pub fn normalize(&mut self) {
        self.schema_version = RUST_TEACHER_SCHEMA;
        self.initialized = true;
        if self.attempts.len() > 240 {
            let keep_from = self.attempts.len() - 240;
            self.attempts.drain(0..keep_from);
        }
        for index in 0..self.attempts.len() {
            if self.attempts[index].attempt_number != 0 {
                continue;
            }
            let suite_id = self.attempts[index].suite_id.clone();
            let case_id = self.attempts[index].case_id.clone();
            let earlier = self.attempts[..index]
                .iter()
                .filter(|row| row.suite_id == suite_id && row.case_id == case_id)
                .count();
            self.attempts[index].attempt_number = earlier.saturating_add(1) as u32;
        }
        self.updated_at = unix_ms();
    }

    pub fn record_attempt(
        &mut self,
        input: RecordTeacherAttemptInput,
        cognitive_trace: Vec<CognitiveTraceEvent>,
        cognitive_context: Value,
        inspector_context: Value,
    ) -> Result<TeacherAttemptRecord, String> {
        if input.question.trim().is_empty() {
            return Err("Teacher test history requires the original question.".to_string());
        }
        if !self.initialized {
            *self = Self::new_initialized();
        }
        let now = unix_ms();
        let signature = format!(
            "{}|{}|{}|{}|{}|{}",
            input.run_id,
            input.suite_id,
            input.case_id,
            input.question,
            input.original_answer,
            self.revision
        );
        let id = stable_hash_id("teacher_attempt", &signature);
        let attempt_number = self.attempts.iter().filter(|row| row.suite_id == input.suite_id && row.case_id == input.case_id).count().saturating_add(1) as u32;
        let provenance = input.provenance;
        let provenance_run_id = provenance.get("runId").cloned().unwrap_or(Value::String(input.run_id.clone()));
        let record = TeacherAttemptRecord {
            knowledge_id: stable_hash_id("teacher_knowledge", &id),
            id,
            run_id: input.run_id,
            suite_id: input.suite_id,
            case_id: input.case_id,
            source_display_name: if input.source_display_name.trim().is_empty() { "Test".to_string() } else { input.source_display_name },
            lesson_context: input.lesson_context.filter(|value| !value.trim().is_empty()),
            attempt_number,
            question: input.question,
            original_answer: input.original_answer,
            expected_answer: input.expected_answer,
            expected_behavior: input.expected_behavior,
            automatic_status: input.automatic_status.trim().to_ascii_lowercase(),
            automatic_reason: input.automatic_reason,
            terminal_state: input.terminal_state,
            reason_code: input.reason_code,
            cognitive_trace,
            cognitive_context,
            inspector_context,
            teacher_points_awarded: None,
            teacher_points_possible: None,
            teacher_grade: None,
            teacher_credit_percent: None,
            teacher_notes: String::new(),
            annotations: Vec::new(),
            evidence_released: false,
            provenance: json!({
                "sourceType": "teacher_test_history",
                "sourceLabel": "Teacher",
                "nonExecutable": true,
                "runId": provenance_run_id,
                "uiMetadata": provenance
            }),
            created_at: now,
            updated_at: now,
        };
        self.attempts.push(record.clone());
        if self.attempts.len() > 240 {
            let excess = self.attempts.len() - 240;
            self.attempts.drain(0..excess);
        }
        self.revision = self.revision.saturating_add(1);
        self.updated_at = now;
        Ok(record)
    }

    pub fn update_feedback(&mut self, input: UpdateTeacherFeedbackInput) -> Result<TeacherAttemptRecord, String> {
        let index = self.attempts.iter().position(|row| row.id == input.attempt_id)
            .ok_or_else(|| "Teacher feedback target was not found in Rust test history.".to_string())?;

        let credit = match input.teacher_credit_percent {
            Some(value) => {
                if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                    return Err("Teacher credit must be between 0 and 100 percent.".to_string());
                }
                Some(value)
            }
            None => None,
        };

        let (points_awarded, points_possible) = match (input.teacher_points_awarded, input.teacher_points_possible) {
            (Some(awarded), Some(possible)) => {
                if !awarded.is_finite() || !possible.is_finite() || possible <= 0.0 || awarded < 0.0 || awarded > possible {
                    return Err("Teacher score must satisfy 0 <= awarded <= possible and possible must be greater than zero.".to_string());
                }
                (Some(awarded), Some(possible))
            }
            (None, None) => (None, None),
            _ => return Err("Teacher score needs both awarded and possible points, or neither.".to_string()),
        };

        if let (Some(percent), Some(awarded), Some(possible)) = (credit, points_awarded, points_possible) {
            let point_percent = awarded / possible * 100.0;
            if (percent - point_percent).abs() > 0.05 {
                return Err("Teacher percent and point score disagree; record one grading result, not two conflicting ones.".to_string());
            }
        }

        let grade = input.teacher_grade
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let notes = input.teacher_notes.trim().to_string();
        let annotations = {
            let current = &self.attempts[index];
            validate_annotations(&current.question, &current.original_answer, input.annotations)?
        };

        let now = unix_ms();
        let record = &mut self.attempts[index];
        record.teacher_credit_percent = credit;
        record.teacher_points_awarded = points_awarded;
        record.teacher_points_possible = points_possible;
        record.teacher_grade = grade;
        record.teacher_notes = notes;
        record.annotations = annotations;
        record.updated_at = now;

        let cloned = record.clone();
        self.revision = self.revision.saturating_add(1);
        self.updated_at = now;
        Ok(cloned)
    }

    pub fn release_run_evidence(&mut self, run_id: &str) -> Result<Vec<TeacherAttemptRecord>, String> {
        let run_id = run_id.trim();
        if run_id.is_empty() {
            return Err("Teacher run finalization requires a run id.".to_string());
        }
        let now = unix_ms();
        let mut matched = false;
        let mut released = Vec::new();
        for record in self.attempts.iter_mut().filter(|record| record.run_id == run_id) {
            matched = true;
            if record.evidence_released {
                continue;
            }
            record.evidence_released = true;
            record.updated_at = now;
            released.push(record.clone());
        }
        if !matched {
            return Err("Teacher run was not found in Rust test history.".to_string());
        }
        if !released.is_empty() {
            self.revision = self.revision.saturating_add(1);
            self.updated_at = now;
        }
        Ok(released)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordTeacherAttemptInput {
    pub run_id: String,
    pub suite_id: String,
    pub case_id: String,
    pub source_display_name: String,
    #[serde(default)]
    pub lesson_context: Option<String>,
    pub question: String,
    pub original_answer: String,
    pub expected_answer: Option<String>,
    pub expected_behavior: Option<String>,
    pub automatic_status: String,
    pub automatic_reason: String,
    pub terminal_state: Option<String>,
    pub reason_code: Option<String>,
    #[serde(default)]
    pub provenance: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateTeacherFeedbackInput {
    pub attempt_id: String,
    pub teacher_credit_percent: Option<f32>,
    pub teacher_points_awarded: Option<f32>,
    pub teacher_points_possible: Option<f32>,
    pub teacher_grade: Option<String>,
    pub teacher_notes: String,
    #[serde(default)]
    pub annotations: Vec<TeacherSpanAnnotation>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinalizeTeacherRunInput {
    pub run_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeacherRunFinalizeReceipt {
    pub accepted: bool,
    pub authority: String,
    pub run_id: String,
    pub released_count: usize,
    #[serde(default)]
    pub knowledge_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeacherFeedbackReceipt {
    pub accepted: bool,
    pub authority: String,
    pub record: TeacherAttemptRecord,
    pub knowledge_id: String,
}


fn utf16_excerpt(text: &str, start: usize, end: usize) -> Option<String> {
    if start >= end {
        return None;
    }
    let units = text.encode_utf16().collect::<Vec<_>>();
    if end > units.len() {
        return None;
    }
    Some(String::from_utf16_lossy(&units[start..end]))
}

fn validate_annotations(question: &str, answer: &str, annotations: Vec<TeacherSpanAnnotation>) -> Result<Vec<TeacherSpanAnnotation>, String> {
    let mut out = Vec::new();
    for mut annotation in annotations {
        let target = annotation.target.trim().to_ascii_lowercase();
        if !matches!(target.as_str(), "question" | "answer") {
            return Err("Teacher annotation target must be question or answer.".to_string());
        }
        let style = annotation.style.trim().to_ascii_lowercase();
        if !matches!(style.as_str(), "strike" | "highlight" | "underline") {
            return Err("Teacher annotation style must be strike, highlight, or underline.".to_string());
        }
        let target_text = if target == "question" { question } else { answer };
        let max = target_text.encode_utf16().count();
        if annotation.start >= annotation.end || annotation.end > max {
            return Err(format!("Teacher annotation range {}..{} is outside the preserved {} text.", annotation.start, annotation.end, target));
        }
        if annotation.id.trim().is_empty() {
            annotation.id = stable_hash_id("teacher_annotation", &format!("{}|{}|{}|{}", target, annotation.start, annotation.end, style));
        }
        annotation.target = target;
        annotation.style = style;
        annotation.note = annotation.note.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
        if annotation.created_at == 0 { annotation.created_at = unix_ms(); }
        out.push(annotation);
    }
    out.sort_by_key(|annotation| (annotation.target.clone(), annotation.start, annotation.end, annotation.style.clone()));
    out.dedup_by(|left, right| left.target == right.target && left.start == right.start && left.end == right.end && left.style == right.style);
    Ok(out)
}
