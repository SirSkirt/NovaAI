use serde_json::{json, Value};

use super::bootstrap::clause_for_sentence;
use super::knowledge::{canonical, KnowledgeSnapshot};
use super::types::{CanonicalCognitiveState, EntityNode, EventNode, Proposition, RelationEdge, SurfaceSentence};
use crate::nova::ids::{EntityId, EventId, FrameId, PropositionId, RelationId};

fn frame_for_scope(_state: &CanonicalCognitiveState, scope_id: &str) -> FrameId {
    if scope_id == "scope_discourse" {
        FrameId("frame_discourse".to_string())
    } else {
        FrameId(format!("frame_{}", scope_id.trim_start_matches("scope_")))
    }
}

pub fn entity_by_label(state: &CanonicalCognitiveState, label: &str, frame_id: Option<&FrameId>) -> Option<EntityNode> {
    let wanted = canonical(label);
    state
        .entities
        .iter()
        .chain(state.discourse_context.entities.iter())
        .find(|entity| {
            canonical(&entity.label) == wanted
                && frame_id.map(|frame| &entity.frame_id == frame).unwrap_or(true)
        })
        .or_else(|| {
            state
                .entities
                .iter()
                .chain(state.discourse_context.entities.iter())
                .find(|entity| canonical(&entity.label) == wanted)
        })
        .cloned()
}

pub fn ensure_entity(
    state: &mut CanonicalCognitiveState,
    label: &str,
    frame_id: &FrameId,
    source: &str,
    knowledge: &KnowledgeSnapshot,
) -> EntityNode {
    if let Some(existing) = entity_by_label(state, label, Some(frame_id)) {
        return existing;
    }
    let clean = label.trim().trim_end_matches(|c: char| matches!(c, '.' | '!' | '?')).to_string();
    let key = canonical(&clean);
    let concept_id = knowledge.resolve_concept(&clean).map(|hit| hit.concept.id);
    let node = EntityNode {
        id: EntityId(format!("rust_entity_{}_{}", frame_id.0, key.replace(' ', "_"))),
        identity_key: format!("{}|{}", frame_id.0, key),
        frame_id: frame_id.clone(),
        label: clean,
        // Capitalization is useful surface evidence, but it does not teach Nova
        // the school concept of a proper name. Name-hood remains learnable.
        name: None,
        kind: "discourse_referent".to_string(),
        concept_id,
        source: source.to_string(),
        attributes: json!({"structuralOnly": true}),
    };
    state.entities.push(node.clone());
    node
}

fn ensure_relation(state: &mut CanonicalCognitiveState, relation: RelationEdge) {
    let duplicate = state.relations.iter().any(|existing| {
        existing.frame_id == relation.frame_id
            && existing.relation_type == relation.relation_type
            && existing.subject_id == relation.subject_id
            && existing.object_id == relation.object_id
            && existing.event_id == relation.event_id
    });
    if !duplicate {
        state.relations.push(relation);
    }
}

fn ensure_event(state: &mut CanonicalCognitiveState, event: EventNode) -> EventNode {
    if let Some(existing) = state.events.iter().find(|existing| existing.identity_key == event.identity_key) {
        return existing.clone();
    }
    state.events.push(event.clone());
    event
}

/// Build structural left-link-right assertions from the small English bootstrap.
/// This is language substrate, not learned English knowledge: it preserves the
/// utterance shape without assigning human semantic-role names or verb meanings.
pub fn construct_assertions(state: &mut CanonicalCognitiveState, knowledge: &KnowledgeSnapshot) -> usize {
    let clauses = state.english_bootstrap.clauses.clone();
    let mut created = 0usize;

    for clause in clauses.into_iter().filter(|clause| clause.utterance_kind == "statement") {
        if clause.left_text.trim().is_empty() || clause.link_text.trim().is_empty() || clause.right_text.trim().is_empty() {
            continue;
        }
        let frame_id = frame_for_scope(state, &clause.scope_id);
        let left = ensure_entity(state, &clause.left_text, &frame_id, "rust_basic_english_left_group", knowledge);
        let right = ensure_entity(state, &clause.right_text, &frame_id, "rust_basic_english_right_group", knowledge);
        let predicate_surface = clause.link_text.trim().to_string();
        let predicate_key = canonical(&predicate_surface);
        let event = ensure_event(
            state,
            EventNode {
                id: EventId(format!("rust_event_{}_{}_{}", frame_id.0, predicate_key.replace(' ', "_"), created)),
                identity_key: format!("{}|{}|{}|{}|{}|{}", frame_id.0, predicate_key, left.id.0, right.id.0, clause.negative, clause.operator_text.as_deref().unwrap_or("")),
                frame_id: frame_id.clone(),
                predicate_surface: predicate_surface.clone(),
                predicate_key: predicate_key.clone(),
                left_argument_id: Some(left.id.clone()),
                right_argument_id: Some(right.id.clone()),
                // Structural argument positions are substrate primitives. Learned
                // cognition may later teach human role concepts and map them here.
                actor_id: None,
                object_id: None,
                roles: json!({
                    "leftArgument": left.id.0.clone(),
                    "rightArgument": right.id.0.clone(),
                    "negative": clause.negative,
                    "operator": clause.operator_text.clone(),
                    "coordinator": clause.coordinator.clone()
                }),
                source: "rust_basic_english_structural_relation".to_string(),
            },
        );

        ensure_relation(
            state,
            RelationEdge {
                id: RelationId(format!("rust_rel_left_{}", event.id.0)),
                frame_id: frame_id.clone(),
                relation_type: "left_argument_of".to_string(),
                subject_id: Some(left.id.0.clone()),
                object_id: Some(event.id.0.clone()),
                event_id: Some(event.id.clone()),
                subject: Some(left.label.clone()),
                object: Some(predicate_surface.clone()),
                source: "rust_basic_english_structural_relation".to_string(),
                attributes: json!({}),
            },
        );
        ensure_relation(
            state,
            RelationEdge {
                id: RelationId(format!("rust_rel_right_{}", event.id.0)),
                frame_id: frame_id.clone(),
                relation_type: "right_argument_of".to_string(),
                subject_id: Some(right.id.0.clone()),
                object_id: Some(event.id.0.clone()),
                event_id: Some(event.id.clone()),
                subject: Some(right.label.clone()),
                object: Some(predicate_surface.clone()),
                source: "rust_basic_english_structural_relation".to_string(),
                attributes: json!({}),
            },
        );

        let proposition = Proposition {
            id: PropositionId(format!("rust_prop_{}", event.id.0)),
            frame_id: frame_id.clone(),
            proposition_type: "structural_relation".to_string(),
            subject: Some(left.label.clone()),
            subject_id: Some(left.id.0.clone()),
            predicate: Some(predicate_surface.clone()),
            object: Some(right.label.clone()),
            object_id: Some(right.id.0.clone()),
            text: Some(format!("{} {} {}", left.label, predicate_surface, right.label)),
            polarity: if clause.negative { "negative" } else { "positive" }.to_string(),
            confidence: clause.confidence,
            source: "rust_basic_english_structural_relation".to_string(),
            evidence: Vec::new(),
            support: json!({
                "sentenceId": clause.sentence_id,
                "englishBootstrap": BASIC_ENGLISH_SUPPORT,
                "inverted": clause.inverted,
                "operator": clause.operator_text,
                "coordinator": clause.coordinator
            }),
            attributes: Value::Null,
        };
        if !state.propositions.iter().any(|item| item.id == proposition.id) {
            state.propositions.push(proposition);
        }
        created += 1;
    }
    created
}

const BASIC_ENGLISH_SUPPORT: &str = "structural_only_not_learned_grammar";

pub fn frame_id_for_scope(state: &CanonicalCognitiveState, scope_id: &str) -> FrameId {
    frame_for_scope(state, scope_id)
}

pub fn clause_for_question<'a>(state: &'a CanonicalCognitiveState, sentence: &SurfaceSentence) -> Option<&'a super::types::EnglishClauseObservation> {
    clause_for_sentence(state, &sentence.id)
}
