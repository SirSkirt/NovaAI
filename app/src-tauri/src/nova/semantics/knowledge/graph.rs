use std::collections::{HashSet, VecDeque};

use super::lexical::{canonical, overlap_score, phrase_present, words};
use super::types::{ConceptMatch, KnowledgeConcept, KnowledgeEdge, KnowledgeSnapshot};
use crate::nova::ids::ConceptId;
use crate::nova::semantics::types::ConceptActivation;

impl KnowledgeSnapshot {
    pub fn concept_by_id(&self, id: &ConceptId) -> Option<&KnowledgeConcept> {
        self.concepts.iter().find(|concept| &concept.id == id)
    }

    pub fn concept_by_key(&self, key: &str) -> Option<&KnowledgeConcept> {
        let wanted = canonical(key);
        self.concepts
            .iter()
            .find(|concept| canonical(&concept.key) == wanted)
    }

    pub fn resolve_concept(&self, query: &str) -> Option<ConceptMatch> {
        let requested = canonical(query);
        let requested_words: HashSet<String> = words(query).into_iter().collect();
        let mut best: Option<ConceptMatch> = None;
        for concept in &self.concepts {
            for alias in std::iter::once(&concept.name).chain(concept.aliases.iter()) {
                let alias_key = canonical(alias);
                if alias_key.is_empty() {
                    continue;
                }
                let alias_words: HashSet<String> = words(alias).into_iter().collect();
                let overlap = requested_words.intersection(&alias_words).count();
                let score = if alias_key == requested {
                    100.0
                } else if phrase_present(query, alias) {
                    92.0 + alias_words.len().min(7) as f32
                } else if !requested.is_empty()
                    && (alias_key.contains(&requested) || requested.contains(&alias_key))
                {
                    78.0
                } else if overlap > 0 {
                    20.0 * overlap as f32
                        + if overlap == alias_words.len().min(requested_words.len()) {
                            12.0
                        } else {
                            0.0
                        }
                } else {
                    0.0
                };
                if score <= 0.0 {
                    continue;
                }
                if best.as_ref().map(|old| score > old.score).unwrap_or(true) {
                    best = Some(ConceptMatch {
                        concept: concept.clone(),
                        score,
                        matched_alias: alias.clone(),
                    });
                }
            }
        }
        best.filter(|hit| hit.score >= 24.0)
    }

    pub fn concepts_in_text(&self, text: &str, limit: usize) -> Vec<ConceptMatch> {
        let query_words: HashSet<String> = words(text).into_iter().collect();
        let mut hits = Vec::new();
        for concept in &self.concepts {
            let mut best_alias: Option<(String, f32)> = None;
            for alias in std::iter::once(&concept.name).chain(concept.aliases.iter()) {
                let alias_words: HashSet<String> = words(alias).into_iter().collect();
                let score = if phrase_present(text, alias) {
                    70.0 + alias_words.len().min(10) as f32 * 3.0
                } else {
                    let overlap = query_words.intersection(&alias_words).count();
                    if overlap > 0 && overlap == alias_words.len() {
                        50.0 + overlap as f32 * 4.0
                    } else {
                        0.0
                    }
                };
                if score <= 0.0 {
                    continue;
                }
                if best_alias
                    .as_ref()
                    .map(|(_, old)| score > *old)
                    .unwrap_or(true)
                {
                    best_alias = Some((alias.clone(), score));
                }
            }
            if let Some((matched_alias, score)) = best_alias {
                hits.push(ConceptMatch {
                    concept: concept.clone(),
                    score,
                    matched_alias,
                });
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(limit);
        hits
    }

    pub fn activate_concepts(&self, text: &str, limit: usize) -> Vec<ConceptActivation> {
        self.concepts_in_text(text, limit)
            .into_iter()
            .map(|hit| ConceptActivation {
                concept_id: hit.concept.id,
                key: hit.concept.key,
                label: hit.concept.name,
                strength: (hit.score / 100.0).clamp(0.0, 1.0),
                source: "rust_semantic_memory".to_string(),
                matched_surface: Some(hit.matched_alias),
            })
            .collect()
    }

    pub fn collect_edges(
        &self,
        root: &ConceptId,
        max_depth: usize,
        limit: usize,
    ) -> Vec<KnowledgeEdge> {
        let depth_limit = max_depth.clamp(1, 4);
        let mut queue = VecDeque::from([(root.clone(), 0usize)]);
        let mut seen_nodes = HashSet::from([root.clone()]);
        let mut seen_claims = HashSet::new();
        let mut out = Vec::new();

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= depth_limit || out.len() >= limit {
                continue;
            }
            for claim in &self.claims {
                let (next, direction) = if claim.subject_id == current {
                    (claim.object_concept_id.clone(), "outgoing")
                } else if claim.object_concept_id.as_ref() == Some(&current) {
                    (Some(claim.subject_id.clone()), "incoming")
                } else {
                    continue;
                };
                if !seen_claims.insert(claim.id.clone()) {
                    continue;
                }
                out.push(KnowledgeEdge {
                    claim: claim.clone(),
                    depth: depth + 1,
                    direction: direction.to_string(),
                    relevance: 0.0,
                });
                if let Some(next_id) = next {
                    if seen_nodes.insert(next_id.clone()) {
                        queue.push_back((next_id, depth + 1));
                    }
                }
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    pub fn select_relevant_edges(
        &self,
        mut edges: Vec<KnowledgeEdge>,
        goal_text: &str,
        limit: usize,
    ) -> Vec<KnowledgeEdge> {
        let goal_terms: HashSet<String> = words(goal_text).into_iter().collect();
        for edge in &mut edges {
            let claim = &edge.claim;
            let edge_terms: HashSet<String> = words(&format!(
                "{} {} {} {}",
                claim.subject, claim.predicate, claim.object, claim.polarity
            ))
            .into_iter()
            .collect();
            let overlap = overlap_score(&goal_terms, &edge_terms);
            let direct_bonus = if edge.depth == 1 { 4.0 } else { 0.0 };
            let structural_penalty = if matches!(claim.predicate.as_str(), "part_of" | "topic_context") {
                -5.0
            } else {
                0.0
            };
            edge.relevance = claim.confidence.max(0.1) * 4.0
                + direct_bonus
                + overlap
                + structural_penalty
                - (edge.depth.saturating_sub(1) as f32 * 1.25);
        }
        edges.sort_by(|a, b| b.relevance.total_cmp(&a.relevance));
        edges.truncate(limit.max(1));
        edges
    }

    pub fn relation_concept_is_present_in(&self, text: &str) -> bool {
        self.concepts_in_text(text, 12).iter().any(|hit| {
            let labels = std::iter::once(&hit.concept.name).chain(hit.concept.aliases.iter());
            labels.into_iter().any(|label| {
                canonical(label)
                    .split_whitespace()
                    .any(|word| word == "relation" || word == "relationship")
            })
        })
    }
}
