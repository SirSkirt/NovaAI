use serde::{Deserialize, Serialize};

use super::mind::TurnOrigin;
use super::semantics::{CanonicalCognitiveState, SemanticTurnReport};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizationReport {
    pub authority: String,
    pub surface: Option<String>,
    pub terminal_state: String,
    pub reason_code: String,
    pub meaning_available: bool,
    pub visible_response_required: bool,
}

fn conclusion_surface(state: &CanonicalCognitiveState) -> Option<String> {
    state.conclusions.iter().rev().find_map(|conclusion| {
        conclusion.surface.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()).map(str::to_string)
    })
}

fn has_meaning(state: &CanonicalCognitiveState) -> bool {
    !state.frames.is_empty()
        || !state.entities.is_empty()
        || !state.events.is_empty()
        || !state.relations.is_empty()
        || !state.propositions.is_empty()
        || !state.requests.is_empty()
        || !state.unknown_slots.is_empty()
        || !state.candidate_sets.is_empty()
        || !state.structures.is_empty()
        || !state.conclusions.is_empty()
}

pub fn realize_current(
    state: &CanonicalCognitiveState,
    semantic_report: &SemanticTurnReport,
    origin: &TurnOrigin,
) -> RealizationReport {
    let semantic_surface = semantic_report.suggested_surface.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()).map(str::to_string);
    let surface = semantic_surface.or_else(|| conclusion_surface(state));
    let meaning_available = has_meaning(state);
    let visible_response_required = !matches!(origin, TurnOrigin::Internal);

    let (terminal_state, reason_code) = if surface.is_some() {
        ("SUPPORTED", "GOAL_SATISFIED")
    } else if matches!(origin, TurnOrigin::Internal) && state.unresolved.is_empty() && meaning_available {
        ("SUPPORTED", "INTERNAL_GOAL_SATISFIED")
    } else if meaning_available {
        ("BLOCKED", "MISSING_OPERATION")
    } else {
        ("EXHAUSTED", "NO_INTERPRETATION")
    };

    RealizationReport {
        authority: "rust_v7_pass_9".to_string(),
        surface,
        terminal_state: terminal_state.to_string(),
        reason_code: reason_code.to_string(),
        meaning_available,
        visible_response_required,
    }
}

pub fn realize_learning(learned_units: usize, upgraded: usize, stored_only: usize) -> RealizationReport {
    let surface = if learned_units == 0 && upgraded == 0 && stored_only == 0 {
        "I understood that as teaching.".to_string()
    } else if upgraded > 0 && stored_only > 0 {
        "I learned from that. There are still parts of it I do not know how to use yet.".to_string()
    } else if upgraded > 0 {
        "I learned from that.".to_string()
    } else if stored_only > 0 {
        "I learned from that, but there is still part of it I do not know how to use yet.".to_string()
    } else {
        "I learned from that.".to_string()
    };
    RealizationReport {
        authority: "rust_v7_pass_9".to_string(),
        surface: Some(surface),
        terminal_state: "SUPPORTED".to_string(),
        reason_code: "LEARNED".to_string(),
        meaning_available: true,
        visible_response_required: true,
    }
}
