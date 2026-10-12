use std::collections::HashSet;

use super::lexical::{canonical, overlap_score, words};
use super::types::{KnowledgeEvidenceItem, KnowledgeSnapshot};
use crate::nova::ids::ConceptId;
use crate::nova::semantics::types::ConceptActivation;

impl KnowledgeSnapshot {
    pub fn evidence_items(&self) -> Vec<KnowledgeEvidenceItem> {
        let mut out = Vec::new();
        for claim in &self.claims {
            if matches!(claim.predicate.as_str(), "part_of" | "topic_context") {
                continue;
            }
            out.push(KnowledgeEvidenceItem {
                id: claim.id.clone(),
                kind: "claim".to_string(),
                concept_id: Some(claim.subject_id.clone()),
                concept_key: Some(claim.subject_key.clone()),
                subject: Some(claim.subject.clone()),
                predicate: Some(claim.predicate.clone()),
                text: format!(
                    "{} {} {}",
                    claim.subject,
                    claim.predicate.replace('_', " "),
                    claim.object
                ),
                polarity: claim.polarity.clone(),
                confidence: claim.confidence,
                relevance: 0.0,
                evidence: claim.evidence.clone(),
                attributes: serde_json::json!({
                    "object": claim.object.clone(),
                    "objectConceptId": claim.object_concept_id.clone(),
                    "qualifiers": claim.qualifiers.clone()
                }),
            });
        }
        for guidance in &self.guidance {
            let concept = self.concept_by_key(&guidance.target);
            out.push(KnowledgeEvidenceItem {
                id: guidance.id.clone(),
                kind: "guidance".to_string(),
                concept_id: concept.map(|item| item.id.clone()),
                concept_key: concept.map(|item| item.key.clone()),
                subject: Some(guidance.target.clone()),
                predicate: Some("guidance".to_string()),
                text: guidance.directive.clone(),
                polarity: guidance.polarity.clone(),
                confidence: match guidance.strength.as_str() {
                    "required" | "must" => 0.98,
                    "strong" => 0.94,
                    _ => 0.88,
                },
                relevance: 0.0,
                evidence: guidance.evidence.clone(),
                attributes: serde_json::json!({
                    "domain": guidance.domain.clone(),
                    "scope": guidance.scope.clone(),
                    "conditions": guidance.conditions.clone()
                }),
            });
        }
        for procedure in &self.procedures {
            let text = procedure
                .purpose
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| procedure.name.clone());
            out.push(KnowledgeEvidenceItem {
                id: procedure.id.clone(),
                kind: "procedure".to_string(),
                concept_id: procedure.concept_id.clone(),
                concept_key: Some(procedure.concept_key.clone()),
                subject: Some(procedure.name.clone()),
                predicate: Some("procedure".to_string()),
                text,
                polarity: "positive".to_string(),
                confidence: procedure.confidence.max(0.1),
                relevance: 0.0,
                evidence: procedure.evidence.clone(),
                attributes: serde_json::json!({
                    "interactionMode": procedure.interaction_mode.clone(),
                    "requirements": procedure.requirements.clone(),
                    "sequence": procedure.sequence.clone()
                }),
            });
            for (index, step) in procedure.sequence.iter().enumerate() {
                let content = step
                    .get("content")
                    .and_then(|value| value.as_str())
                    .or_else(|| step.as_str())
                    .unwrap_or("")
                    .trim();
                if content.is_empty() {
                    continue;
                }
                out.push(KnowledgeEvidenceItem {
                    id: format!("{}#step{}", procedure.id, index + 1),
                    kind: "procedure_step".to_string(),
                    concept_id: procedure.concept_id.clone(),
                    concept_key: Some(procedure.concept_key.clone()),
                    subject: Some(procedure.name.clone()),
                    predicate: Some("procedure_step".to_string()),
                    text: content.to_string(),
                    polarity: "positive".to_string(),
                    confidence: procedure.confidence.max(0.1),
                    relevance: 0.0,
                    evidence: procedure.evidence.clone(),
                    attributes: serde_json::json!({"order": index + 1}),
                });
            }
        }
        for example in &self.examples {
            let concept = example
                .demonstrates_key
                .as_deref()
                .and_then(|key| self.concept_by_key(key));
            let is_teacher_feedback = example.evidence.iter().any(|row| {
                row.get("sourceType")
                    .and_then(|value| value.as_str())
                    .map(|value| value == "teacher_feedback")
                    .unwrap_or(false)
            });
            let kind = if is_teacher_feedback { "teacher_feedback" } else { "example" };
            out.push(KnowledgeEvidenceItem {
                id: example.id.clone(),
                kind: kind.to_string(),
                concept_id: concept.map(|item| item.id.clone()),
                concept_key: example.demonstrates_key.clone(),
                subject: concept.map(|item| item.name.clone()),
                predicate: Some(kind.to_string()),
                text: example.content.clone(),
                polarity: "positive".to_string(),
                confidence: if is_teacher_feedback { 0.92 } else { 0.72 },
                relevance: 0.0,
                evidence: example.evidence.clone(),
                attributes: serde_json::json!({
                    "sequence": example.sequence.clone(),
                    "nonExecutable": is_teacher_feedback
                }),
            });
        }
        out
    }

    pub fn rank_evidence(
        &self,
        question: &str,
        active_concepts: &[ConceptActivation],
        focus_concept: Option<&ConceptId>,
        limit: usize,
    ) -> Vec<KnowledgeEvidenceItem> {
        let query_terms: HashSet<String> = words(question).into_iter().collect();
        let active_ids: HashSet<ConceptId> = active_concepts
            .iter()
            .map(|activation| activation.concept_id.clone())
            .collect();
        let active_keys: HashSet<String> = active_concepts
            .iter()
            .map(|activation| canonical(&activation.key))
            .collect();

        let mut candidates = self.evidence_items();
        for item in &mut candidates {
            let item_terms: HashSet<String> = words(&format!(
                "{} {} {} {}",
                item.subject.clone().unwrap_or_default(),
                item.predicate.clone().unwrap_or_default(),
                item.text,
                item.concept_key.clone().unwrap_or_default()
            ))
            .into_iter()
            .collect();
            let lexical = overlap_score(&query_terms, &item_terms);
            let predicate_bonus = item
                .predicate
                .as_deref()
                .filter(|predicate| {
                    !matches!(
                        *predicate,
                        "definition" | "relationship" | "guidance" | "procedure" | "procedure_step" | "teacher_feedback" | "example"
                    )
                })
                .map(|predicate| {
                    let predicate_terms: HashSet<String> = words(predicate).into_iter().collect();
                    if !predicate_terms.is_empty() && predicate_terms.iter().all(|term| query_terms.contains(term)) {
                        10.0
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);
            let concept_bonus = if item
                .concept_id
                .as_ref()
                .map(|id| active_ids.contains(id))
                .unwrap_or(false)
                || item
                    .concept_key
                    .as_ref()
                    .map(|key| active_keys.contains(&canonical(key)))
                    .unwrap_or(false)
            {
                // Active concepts are an attention prior, not an authority. A
                // different learned concept with much better goal-relevant
                // evidence must still be able to win and open itself.
                7.0
            } else {
                0.0
            };
            let focus_bonus = if focus_concept
                .zip(item.concept_id.as_ref())
                .map(|(focus, item_id)| focus == item_id)
                .unwrap_or(false)
            {
                5.0
            } else {
                0.0
            };
            let kind_bias = match item.kind.as_str() {
                "claim" => 4.0,
                "guidance" => 3.0,
                "procedure" => 1.5,
                "procedure_step" => 2.5,
                // An example is evidence of a principle, not the default rule itself.
                "teacher_feedback" => 1.0,
                "example" => -3.0,
                _ => 0.0,
            };
            let polarity_bonus = if item.polarity == "negative" && canonical(question).contains("not") {
                2.0
            } else {
                0.0
            };
            item.relevance = lexical
                + predicate_bonus
                + concept_bonus
                + focus_bonus
                + kind_bias
                + polarity_bonus
                + item.confidence.max(0.1) * 4.0;
        }
        candidates.retain(|item| item.relevance >= 8.0);
        candidates.sort_by(|a, b| b.relevance.total_cmp(&a.relevance));
        candidates.truncate(limit.max(1));
        candidates
    }

    pub fn infer_concepts_from_evidence(
        &self,
        evidence: &[KnowledgeEvidenceItem],
        already_active: &[ConceptActivation],
        limit: usize,
    ) -> Vec<ConceptActivation> {
        let mut seen: HashSet<ConceptId> = already_active
            .iter()
            .map(|activation| activation.concept_id.clone())
            .collect();
        let mut out = Vec::new();
        for item in evidence {
            let id = match &item.concept_id {
                Some(id) if seen.insert(id.clone()) => id,
                _ => continue,
            };
            let concept = match self.concept_by_id(id) {
                Some(concept) => concept,
                None => continue,
            };
            out.push(ConceptActivation {
                concept_id: concept.id.clone(),
                key: concept.key.clone(),
                label: concept.name.clone(),
                strength: (item.relevance / 40.0).clamp(0.25, 0.85),
                source: "rust_goal_evidence_activation".to_string(),
                matched_surface: None,
            });
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}
