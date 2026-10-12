use serde::{Deserialize, Serialize};

use super::knowledge::KnowledgeSnapshot;
use super::requests::construct_and_resolve_requests;
use super::types::CanonicalCognitiveState;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticExecutiveReport {
    pub cycles: usize,
    pub requests_constructed: usize,
    pub requests_resolved: usize,
    pub conclusions_produced: usize,
    pub active_concepts_after_reasoning: usize,
    pub stalled: bool,
}

fn digest(state: &CanonicalCognitiveState) -> (usize, usize, usize, usize, usize) {
    (
        state.active_concepts.len(),
        state.requests.len(),
        state.unresolved.len(),
        state.propositions.len(),
        state.conclusions.len(),
    )
}

/// Current Rust-owned semantic executive for the migrated reasoning slice.
/// It repeatedly reconsiders the same canonical state as new concepts,
/// requests, propositions, or conclusions appear. It does not call the old
/// JavaScript semantic/request implementation and does not contain response
/// templates for particular curriculum questions.
pub fn run_semantic_executive(
    state: &mut CanonicalCognitiveState,
    knowledge: &KnowledgeSnapshot,
) -> SemanticExecutiveReport {
    const MAX_CYCLES: usize = 4;
    let initial_requests = state.requests.len();
    let initial_conclusions = state.conclusions.len();
    let mut previous = digest(state);
    let mut cycles = 0usize;
    let mut stalled = false;

    for cycle in 1..=MAX_CYCLES {
        cycles = cycle;
        construct_and_resolve_requests(state, knowledge);
        let now = digest(state);
        if now == previous {
            stalled = true;
            break;
        }
        previous = now;

        // A supported Rust conclusion satisfies the migrated semantic goal.
        // Additional cognitive-module work remains a later Rust migration and
        // is not layered over this semantic result here.
        if state
            .conclusions
            .iter()
            .rev()
            .any(|conclusion| conclusion.rust_authoritative && conclusion.surface.is_some())
        {
            break;
        }
    }

    SemanticExecutiveReport {
        cycles,
        requests_constructed: state.requests.len().saturating_sub(initial_requests),
        requests_resolved: state
            .requests
            .iter()
            .filter(|request| request.status == "resolved")
            .count(),
        conclusions_produced: state.conclusions.len().saturating_sub(initial_conclusions),
        active_concepts_after_reasoning: state.active_concepts.len(),
        stalled,
    }
}
