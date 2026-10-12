mod common;
mod concept;
mod general;
mod introspection;
mod relational;

use super::bootstrap::sentence_kind;
use super::knowledge::KnowledgeSnapshot;
use super::types::{CanonicalCognitiveState, SurfaceSentence};

pub fn construct_and_resolve_requests(
    state: &mut CanonicalCognitiveState,
    knowledge: &KnowledgeSnapshot,
) -> usize {
    let questions: Vec<SurfaceSentence> = state
        .surface
        .sentences
        .iter()
        .filter(|sentence| sentence.terminal.as_deref() == Some("?") || matches!(sentence_kind(state, &sentence.id), Some("question") | Some("request")))
        .cloned()
        .collect();
    let mut handled = 0usize;

    // Represented/nested requests are built first so the outer turn can inspect
    // them as semantic objects without executing them as the outer request.
    let mut ordered = questions;
    ordered.sort_by_key(|sentence| {
        if sentence.scope_id == "scope_discourse" {
            1
        } else {
            0
        }
    });

    for sentence in ordered {
        if relational::resolve_direct_relational_question(state, &sentence, knowledge) {
            handled += 1;
            continue;
        }
        if introspection::resolve_meta_relation_question(state, &sentence, knowledge) {
            handled += 1;
            continue;
        }
        if concept::resolve_concept_question(state, &sentence, knowledge) {
            handled += 1;
            continue;
        }
        if general::resolve_general_knowledge_goal(state, &sentence, knowledge) {
            handled += 1;
        }
    }
    handled
}
