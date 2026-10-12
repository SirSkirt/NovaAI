use std::collections::{BTreeMap, BTreeSet};

use crate::nova::semantics::CanonicalCognitiveState;

use super::path::canon;
use super::types::{CognitiveModule, CognitiveRuntimeSnapshot, ExperienceRoute};

const MAX_MATCHED_ROUTES: usize = 3;
const MIN_ROUTE_CONFIDENCE: f32 = 0.12;

#[derive(Clone, Debug, Default)]
pub struct CognitiveSchedule<'a> {
    pub modules: Vec<&'a CognitiveModule>,
    pub mode: String,
    pub route_ids: Vec<String>,
    pub route_strengths: BTreeMap<u64, f32>,
}

#[derive(Clone, Debug, Default)]
struct ExperienceSignature {
    concepts: BTreeSet<String>,
    goals: BTreeSet<String>,
    relations: BTreeSet<String>,
    fallback: BTreeSet<String>,
}

fn add_term(bucket: &mut BTreeSet<String>, value: impl AsRef<str>) {
    for part in value
        .as_ref()
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
    {
        let key = canon(part);
        if key.len() > 1 {
            bucket.insert(key);
        }
    }
}

fn signature(state: &CanonicalCognitiveState) -> ExperienceSignature {
    let mut signature = ExperienceSignature::default();

    for concept in &state.active_concepts {
        add_term(&mut signature.concepts, &concept.key);
        add_term(&mut signature.concepts, &concept.label);
    }
    for category in &state.active_categories {
        add_term(&mut signature.concepts, category);
    }
    for event in &state.events {
        add_term(&mut signature.relations, &event.predicate_key);
    }
    for relation in &state.relations {
        add_term(&mut signature.relations, &relation.relation_type);
    }
    for slot in &state.unknown_slots {
        add_term(&mut signature.relations, &slot.role);
        add_term(&mut signature.relations, &slot.value_type);
    }
    for request in &state.requests {
        add_term(&mut signature.goals, &request.request_type);
        add_term(&mut signature.goals, &request.answer_relation);
        if let Some(target) = &request.target {
            add_term(&mut signature.concepts, target);
        }
    }
    for unresolved in &state.unresolved {
        add_term(&mut signature.goals, &unresolved.need_type);
    }
    for conclusion in &state.conclusions {
        add_term(&mut signature.goals, &conclusion.conclusion_type);
    }

    if signature.concepts.is_empty() {
        for token in &state.surface.tokens {
            if token.word {
                add_term(&mut signature.fallback, &token.surface);
            }
        }
    }

    signature
}

fn overlap(route_terms: &[String], signature_terms: &BTreeSet<String>) -> usize {
    route_terms
        .iter()
        .filter(|term| signature_terms.contains(&canon(term)))
        .count()
}

fn route_score(route: &ExperienceRoute, signature: &ExperienceSignature) -> f32 {
    let concept_overlap = overlap(&route.concept_terms, &signature.concepts) as f32;
    let goal_overlap = overlap(&route.goal_terms, &signature.goals) as f32;
    let relation_overlap = overlap(&route.relation_terms, &signature.relations) as f32;
    let fallback_overlap = if signature.concepts.is_empty() {
        overlap(&route.fallback_terms, &signature.fallback) as f32
    } else {
        0.0
    };

    if concept_overlap == 0.0 && goal_overlap == 0.0 && relation_overlap == 0.0 && fallback_overlap == 0.0 {
        return 0.0;
    }

    let base = concept_overlap * 14.0 + goal_overlap * 10.0 + relation_overlap * 6.0 + fallback_overlap * 2.0;
    let experience = (route.successes as f32 * 0.25).min(5.0) * route.confidence.max(0.25);
    base + experience
}

fn sort_modules<'a>(modules: &mut Vec<&'a CognitiveModule>, strengths: &BTreeMap<u64, f32>) {
    modules.sort_by(|left, right| {
        let left_strength = left.module_number.and_then(|number| strengths.get(&number)).copied().unwrap_or(0.0);
        let right_strength = right.module_number.and_then(|number| strengths.get(&number)).copied().unwrap_or(0.0);
        right_strength
            .partial_cmp(&left_strength)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.priority.partial_cmp(&left.priority).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| left.module_number.unwrap_or(u64::MAX).cmp(&right.module_number.unwrap_or(u64::MAX)))
    });
}

pub fn schedule<'a>(state: &CanonicalCognitiveState, snapshot: &'a CognitiveRuntimeSnapshot, force_broad: bool) -> CognitiveSchedule<'a> {
    let mut enabled: Vec<&CognitiveModule> = snapshot.modules.iter().filter(|module| module.enabled).collect();
    if force_broad || snapshot.experience_routes.is_empty() {
        let strengths = BTreeMap::new();
        sort_modules(&mut enabled, &strengths);
        return CognitiveSchedule {
            modules: enabled,
            mode: "explore_all".to_string(),
            route_ids: Vec::new(),
            route_strengths: strengths,
        };
    }

    let signature = signature(state);
    let mut ranked: Vec<(&ExperienceRoute, f32)> = snapshot
        .experience_routes
        .iter()
        .filter(|route| route.confidence >= MIN_ROUTE_CONFIDENCE)
        .filter_map(|route| {
            let score = route_score(route, &signature);
            (score > 0.0).then_some((route, score))
        })
        .collect();
    ranked.sort_by(|(left, left_score), (right, right_score)| {
        right_score
            .partial_cmp(left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.confidence.partial_cmp(&left.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| right.successes.cmp(&left.successes))
    });
    ranked.truncate(MAX_MATCHED_ROUTES);

    if ranked.is_empty() {
        let strengths = BTreeMap::new();
        sort_modules(&mut enabled, &strengths);
        return CognitiveSchedule {
            modules: enabled,
            mode: "explore_all".to_string(),
            route_ids: Vec::new(),
            route_strengths: strengths,
        };
    }

    let mut wanted = BTreeSet::new();
    let mut route_ids = Vec::new();
    let mut strengths = BTreeMap::new();
    for (route, _) in ranked {
        route_ids.push(route.id.clone());
        for number in &route.module_numbers {
            wanted.insert(*number);
            let key = number.to_string();
            let strength = route
                .module_strengths
                .as_object()
                .and_then(|values| values.get(&key))
                .and_then(|value| value.as_f64())
                .unwrap_or(0.5) as f32;
            strengths.entry(*number).and_modify(|current: &mut f32| *current = current.max(strength)).or_insert(strength);
        }
    }

    let mut selected: Vec<&CognitiveModule> = enabled
        .into_iter()
        .filter(|module| {
            module.source_type == "built_in"
                || module.module_number.is_some_and(|number| wanted.contains(&number))
        })
        .collect();
    sort_modules(&mut selected, &strengths);

    CognitiveSchedule {
        modules: selected,
        mode: "experience_route".to_string(),
        route_ids,
        route_strengths: strengths,
    }
}
