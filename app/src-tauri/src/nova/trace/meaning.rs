use std::collections::BTreeSet;

use serde_json::json;

use crate::nova::cognition::CognitiveRuntimeReport;
use crate::nova::semantics::{CanonicalCognitiveState, DiscourseSnapshot};
use crate::nova::semantics::types::CognitiveTraceEvent;

fn human_key(value: &str) -> String {
    value.replace('_', " ").trim().to_string()
}

fn entity_label(state: &CanonicalCognitiveState, id: &str) -> String {
    state.entities.iter()
        .find(|entity| entity.id.0 == id)
        .map(|entity| entity.name.clone().unwrap_or_else(|| entity.label.clone()))
        .unwrap_or_else(|| id.to_string())
}

fn prior_ids(discourse: &DiscourseSnapshot) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    ids.extend(discourse.entities.iter().map(|item| item.id.0.clone()));
    ids.extend(discourse.events.iter().map(|item| item.id.0.clone()));
    ids.extend(discourse.relations.iter().map(|item| item.id.0.clone()));
    ids.extend(discourse.propositions.iter().map(|item| item.id.0.clone()));
    ids
}

fn has_explicit_context_link(state: &CanonicalCognitiveState, discourse: &DiscourseSnapshot) -> bool {
    if !state.english_bootstrap.context_links.is_empty() { return true; }
    let prior = prior_ids(discourse);
    if prior.is_empty() { return false; }
    state.relations.iter().any(|relation| {
        relation.subject_id.as_ref().is_some_and(|id| prior.contains(id))
            || relation.object_id.as_ref().is_some_and(|id| prior.contains(id))
            || relation.event_id.as_ref().is_some_and(|id| prior.contains(&id.0))
    }) || state.propositions.iter().any(|proposition| {
        proposition.subject_id.as_ref().is_some_and(|id| prior.contains(id))
            || proposition.object_id.as_ref().is_some_and(|id| prior.contains(id))
    })
}

fn push(events: &mut Vec<CognitiveTraceEvent>, stage: &str, text: String, source_ids: Vec<String>, data: serde_json::Value) {
    if text.trim().is_empty() { return; }
    events.push(CognitiveTraceEvent {
        stage: stage.to_string(),
        text,
        source_ids,
        data,
    });
}

/// Build the human-facing trace only from structures that the authoritative
/// Rust cognition path actually produced. This function does not run a second
/// interpreter or infer a hidden rationale; every line points back to canonical
/// state or the Rust cognitive-runtime report.
pub fn refresh_cognitive_trace(
    state: &mut CanonicalCognitiveState,
    discourse: &DiscourseSnapshot,
    cognitive: Option<&CognitiveRuntimeReport>,
) {
    let mut events = Vec::new();

    if !state.surface.quotes.is_empty() {
        let quoted = state.surface.quotes.iter().take(2).map(|quote| format!("\"{}\"", quote.text)).collect::<Vec<_>>().join("; ");
        push(
            &mut events,
            "observe",
            format!("I kept the quoted material separate from the surrounding message: {quoted}."),
            state.surface.quotes.iter().take(2).map(|quote| quote.id.clone()).collect(),
            json!({"quoteCount": state.surface.quotes.len()}),
        );
    }


    if !state.english_bootstrap.utterance_kind.is_empty() {
        let clauses = state.english_bootstrap.clauses.iter().take(2).map(|clause| {
            let operator = clause.operator_text.as_deref().map(|value| format!("{} ", value)).unwrap_or_default();
            let polarity = if clause.negative { " (with negation)" } else { "" };
            format!("{} [{}{}] {}{}", clause.left_text, operator, clause.link_text, clause.right_text, polarity)
        }).collect::<Vec<_>>();
        let detail = if clauses.is_empty() {
            format!("I read this as a {} and kept the whole wording available.", state.english_bootstrap.utterance_kind)
        } else {
            format!("I read this as a {} and grouped the wording as {}.", state.english_bootstrap.utterance_kind, clauses.join("; "))
        };
        push(
            &mut events,
            "interpretation",
            detail,
            Vec::new(),
            json!({"basicEnglish": state.english_bootstrap.clone()}),
        );
    }

    for reference in state.references.iter().take(4) {
        let text = match &reference.resolved_entity {
            Some(entity) => format!(
                "I understood \"{}\" as referring to {}.",
                reference.surface,
                entity
            ),
            None => format!(
                "I could not determine what \"{}\" referred to.",
                reference.surface
            ),
        };
        push(&mut events, "reference", text, vec![reference.id.clone()], json!({
            "surface": reference.surface.clone(),
            "resolvedEntity": reference.resolved_entity.clone(),
            "resolutionSource": reference.resolution_source.clone()
        }));
    }

    let prior_count = discourse.entities.len() + discourse.events.len() + discourse.relations.len() + discourse.propositions.len();
    let previous_assistant = state.conversation_context.iter().rev().find(|turn| turn.role == "assistant");
    if prior_count > 0 || previous_assistant.is_some() {
        let linked = has_explicit_context_link(state, discourse)
            || state.references.iter().any(|reference| reference.resolution_source.contains("context"));
        let context_text = if linked {
            "I connected this turn to something from earlier in the conversation.".to_string()
        } else if let Some(previous) = previous_assistant {
            let preview = previous.content.split_whitespace().take(18).collect::<Vec<_>>().join(" ");
            format!("My previous response was available (\"{}{}\"), but I did not connect this turn to it.", preview, if previous.content.split_whitespace().count() > 18 { "…" } else { "" })
        } else {
            "Earlier conversation was available to me, but I did not connect this turn to it.".to_string()
        };
        push(
            &mut events,
            "context",
            context_text,
            previous_assistant.map(|turn| vec![turn.id.clone()]).unwrap_or_default(),
            json!({"priorSemanticItemsAvailable": prior_count, "previousAssistantResponseAvailable": previous_assistant.is_some(), "explicitLink": linked}),
        );
    }

    if !state.active_concepts.is_empty() {
        let concepts = state.active_concepts.iter().take(4).map(|concept| concept.label.clone()).collect::<Vec<_>>();
        push(
            &mut events,
            "interpretation",
            format!("I connected this message with learned ideas about {}.", concepts.join(", ")),
            state.active_concepts.iter().take(4).map(|concept| concept.concept_id.0.clone()).collect(),
            json!({"concepts": concepts}),
        );
    }

    for event in state.events.iter().take(3) {
        let left = event.left_argument_id.as_ref().map(|id| entity_label(state, &id.0));
        let right = event.right_argument_id.as_ref().map(|id| entity_label(state, &id.0));
        let text = match (left, right) {
            (Some(left), Some(right)) => format!("I represented a relationship where {left} {} {right}.", event.predicate_surface),
            (Some(left), None) => format!("I represented an event where {left} {}.", event.predicate_surface),
            _ => format!("I represented an event around \"{}\".", event.predicate_surface),
        };
        push(&mut events, "relationship", text, vec![event.id.0.clone()], json!({"predicate": event.predicate_key}));
    }

    for relation in state.relations.iter().filter(|relation| relation.source != "rust_basic_english_structural_relation").take(3) {
        if relation.subject.is_none() && relation.object.is_none() { continue; }
        let subject = relation.subject.clone().or_else(|| relation.subject_id.as_ref().map(|id| entity_label(state, id))).unwrap_or_else(|| "something".to_string());
        let object = relation.object.clone().or_else(|| relation.object_id.as_ref().map(|id| entity_label(state, id))).unwrap_or_else(|| "something".to_string());
        push(
            &mut events,
            "relationship",
            format!("I connected {subject} to {object} through the learned relationship \"{}\".", human_key(&relation.relation_type)),
            vec![relation.id.0.clone()],
            json!({"relationType": relation.relation_type.clone()}),
        );
    }

    for request in state.requests.iter().take(3) {
        let target = request.target.clone().unwrap_or_else(|| "the current message".to_string());
        push(
            &mut events,
            "goal",
            format!(
                "I treated the current request as {} about {}, seeking {}.",
                human_key(&request.request_type),
                target,
                human_key(&request.answer_relation)
            ),
            vec![request.id.0.clone()],
            json!({
                "requestType": request.request_type.clone(),
                "target": request.target.clone(),
                "answerRelation": request.answer_relation.clone(),
                "status": request.status.clone()
            }),
        );
    }

    if let Some(report) = cognitive {
        if !report.changed.is_empty() {
            let mut themes = report.changed.iter()
                .flat_map(|row| row.categories.iter().cloned())
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>();
            themes.sort();
            themes.dedup();
            themes.truncate(5);
            let text = if themes.is_empty() {
                "I considered learned connections that changed my working understanding.".to_string()
            } else {
                format!("I considered learned connections about {} that changed my working understanding.", themes.join(", "))
            };
            push(
                &mut events,
                "consideration",
                text,
                report.changed.iter().take(4).map(|row| row.module_id.0.clone()).collect(),
                json!({"changedModuleCount": report.changed.len()}),
            );
        }
    }

    for unresolved in state.unresolved.iter().take(3) {
        push(
            &mut events,
            "unresolved",
            format!("I still had not resolved {}.", human_key(unresolved.description.as_deref().unwrap_or(&unresolved.need_type))),
            vec![unresolved.id.clone()],
            json!({"needType": unresolved.need_type.clone()}),
        );
    }

    if let Some(conclusion) = state.conclusions.iter().rev().find(|item| item.rust_authoritative) {
        if let Some(surface) = conclusion.surface.as_ref().filter(|value| !value.trim().is_empty()) {
            push(
                &mut events,
                "conclusion",
                format!("From that interpretation, I concluded: {surface}"),
                vec![conclusion.id.clone()],
                json!({"conclusionType": conclusion.conclusion_type.clone()}),
            );
        }
    } else if let Some(report) = cognitive {
        if let Some(surface) = report.surface_response.as_ref().filter(|value| !value.trim().is_empty()) {
            push(
                &mut events,
                "conclusion",
                format!("From that interpretation, I concluded: {surface}"),
                Vec::new(),
                json!({"source": "rust_cognitive_runtime"}),
            );
        }
    }

    state.cognitive_trace = events;
}
