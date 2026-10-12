use serde_json::{json, Value};

use crate::nova::semantics::knowledge::{
    KnowledgeExample, KnowledgeGuidance, KnowledgeProcedure, RustSemanticMemoryState, SemanticLearningCounts,
    SemanticLearningReport, SemanticSourceRecord,
};
use crate::nova::mind::unix_ms;

use super::parser::{canonical, stable_source_id};
use super::types::{ParsedTeachingSource, TeachingBlock, TeachingSourceInput};

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(|ch: char| matches!(ch, '.' | ';' | ':' | '"' | '\'' | '“' | '”')).trim().to_string()
}

fn source_role(parsed: &ParsedTeachingSource) -> &'static str {
    if parsed.document_kind == "lesson" || parsed.blocks.iter().any(|block| matches!(block.role.as_str(), "concept" | "definition" | "guidance" | "procedure" | "procedure_step")) { "instruction" } else { "knowledge" }
}

fn domain_name(parsed: &ParsedTeachingSource) -> String {
    parsed.blocks.iter().find(|block| block.role == "metadata" && block.label == "topic" && !block.text.trim().is_empty()).map(|block| clean(&block.text))
        .or_else(|| parsed.blocks.iter().find(|block| block.role == "concept" && !block.text.trim().is_empty()).map(|block| clean(&block.text)))
        .unwrap_or_else(|| parsed.source_display_name.clone())
}

fn sentence_list(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let sentence = clean(&current);
            if !sentence.is_empty() { out.push(sentence); }
            current.clear();
        }
    }
    let tail = clean(&current);
    if !tail.is_empty() { out.push(tail); }
    out
}

fn normalized_subject(value: &str) -> String {
    let value = clean(value);
    let lower = value.to_ascii_lowercase();
    for article in ["a ", "an ", "the "] {
        if lower.starts_with(article) && value.len() > article.len() {
            return clean(&value[article.len()..]);
        }
    }
    value
}

fn split_relation(text: &str, phrase: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    let index = lower.find(phrase)?;
    let left = normalized_subject(&text[..index]);
    let right = clean(&text[index + phrase.len()..]);
    if left.is_empty() || right.is_empty() { None } else { Some((left, right)) }
}

fn parse_claim(text: &str, scope: &str) -> Option<(String, String, String, Option<String>, f32)> {
    let text = clean(text);
    if text.is_empty() { return None; }
    for (phrase, predicate) in [
        (" refers to ", "refers_to"), (" means ", "means"), (" is a kind of ", "is_a"), (" is a type of ", "is_a"),
        (" is a form of ", "is_a"), (" belongs to ", "part_of"), (" is part of ", "part_of"), (" depends on ", "depends_on"),
        (" relies on ", "relies_on"), (" connects to ", "connects_to"), (" relates to ", "relates_to"),
    ] {
        if let Some((subject, object)) = split_relation(&text, phrase) {
            return Some((subject, predicate.to_string(), object.clone(), Some(object), 0.94));
        }
    }

    for (phrase, predicate) in [
        (" uses ", "use"), (" contains ", "contain"), (" includes ", "include"), (" supports ", "support"),
        (" stores ", "store"), (" provides ", "provide"), (" creates ", "create"), (" changes ", "change"),
        (" teaches ", "teach"), (" helps ", "help"), (" organizes ", "organize"), (" represents ", "represent"),
        (" describes ", "describe"), (" defines ", "define"), (" allows ", "allow"), (" requires ", "require"),
    ] {
        if let Some((subject, object)) = split_relation(&text, phrase) {
            return Some((subject, predicate.to_string(), object, None, 0.88));
        }
    }

    for phrase in [" is ", " are "] {
        if let Some((subject, mut object)) = split_relation(&text, phrase) {
            let lowered = object.to_ascii_lowercase();
            let predicate = if lowered.starts_with("a ") || lowered.starts_with("an ") {
                object = object.split_once(' ').map(|(_, right)| right.to_string()).unwrap_or(object);
                "is_a"
            } else { "is" };
            return Some((subject, predicate.to_string(), object, None, 0.9));
        }
    }

    let scope = clean(scope);
    if !scope.is_empty() && canonical(&scope) != "lesson" && canonical(&scope) != "document" {
        return Some((scope, "means".to_string(), text, None, 0.78));
    }
    None
}

fn guidance_from_block(block: &TeachingBlock, source_id: &str, source_name: &str, source_role: &str) -> Option<KnowledgeGuidance> {
    let directive = clean(&block.text);
    if directive.is_empty() { return None; }
    let lower = directive.to_ascii_lowercase();
    let negative = matches!(block.role.as_str(), "warning" | "constraint" | "negative_example") || lower.starts_with("do not ") || lower.starts_with("don't ") || lower.starts_with("avoid ") || lower.contains(" must not ") || lower.contains(" should not ");
    let strength = if lower.contains("must") || lower.starts_with("do not") || lower.starts_with("don't") { "required" } else if lower.contains("may") || lower.contains("can") || lower.contains("might") { "optional" } else { "recommended" };
    Some(KnowledgeGuidance {
        id: String::new(),
        domain: canonical(&block.scope).replace(' ', "_"),
        scope: block.scope.clone(),
        target: block.scope.clone(),
        strength: strength.to_string(),
        polarity: if negative { "negative".to_string() } else { "positive".to_string() },
        directive,
        conditions: Vec::new(),
        evidence: vec![RustSemanticMemoryState::evidence(source_id, source_name, source_role, &block.scope, &block.role, &block.text)],
    })
}

pub fn apply_semantic_learning(
    memory: &mut RustSemanticMemoryState,
    input: &TeachingSourceInput,
    parsed: &ParsedTeachingSource,
) -> SemanticLearningReport {
    memory.begin_learning_update();
    let now = unix_ms();
    let source_id = input.source_id.clone().filter(|id| !id.trim().is_empty()).unwrap_or_else(|| stable_source_id(&input.source_name, &input.text));
    let role = source_role(parsed);
    let domain = domain_name(parsed);
    let domain_evidence = RustSemanticMemoryState::evidence(&source_id, &input.source_name, role, &domain, "domain", &format!("Document domain: {domain}"));
    let domain_concept = memory.upsert_concept(&domain, &["domain".to_string()], &[input.source_name.clone()], role == "knowledge", role == "instruction", None, domain_evidence);
    let domain_key = domain_concept.as_ref().map(|concept| concept.key.clone());
    let before = (
        memory.knowledge.concepts.len(), memory.knowledge.claims.len(), memory.knowledge.procedures.len(), memory.knowledge.guidance.len(), memory.knowledge.examples.len()
    );

    for block in parsed.blocks.iter().filter(|block| block.section_kind != "test") {
        let evidence = RustSemanticMemoryState::evidence(&source_id, &input.source_name, role, &block.scope, &block.role, &block.text);
        match block.role.as_str() {
            "concept" | "category" => {
                let kind = if block.role == "category" { "learned_category" } else if role == "instruction" { "instructional_concept" } else { "topic" };
                if let Some(concept) = memory.upsert_concept(&block.text, &[kind.to_string()], &[], role == "knowledge", role == "instruction", domain_key.as_deref(), evidence.clone()) {
                    if let Some(domain_concept) = &domain_concept {
                        if concept.id != domain_concept.id {
                            let _ = memory.store_claim(&concept.name, "part_of", &domain_concept.name, Some(&domain_concept.name), "positive", 0.96, json!({}), evidence.clone());
                        }
                    }
                }
            }
            "definition" | "relationship" | "body" | "explanation" => {
                for sentence in sentence_list(&block.text) {
                    if let Some((subject, predicate, object, object_concept, confidence)) = parse_claim(&sentence, &block.scope) {
                        let _ = memory.store_claim(&subject, &predicate, &object, object_concept.as_deref(), "positive", confidence, json!({"teachingRole":block.role}), evidence.clone());
                    }
                }
            }
            "guidance" | "warning" | "constraint" | "behavior" | "correction" | "validation_rule" => {
                if let Some(item) = guidance_from_block(block, &source_id, &input.source_name, role) { memory.store_guidance(item, evidence); }
            }
            "example" | "practice" | "negative_example" => {
                let item = KnowledgeExample { id: String::new(), demonstrates_key: memory.concept_by_name(&block.scope).map(|concept| concept.key), content: clean(&block.text), sequence: Vec::new(), evidence: Vec::new() };
                if !item.content.is_empty() { memory.store_example(item, evidence); }
            }
            "procedure" => {
                let name = clean(&block.text);
                if !name.is_empty() {
                    let concept = memory.upsert_concept(&name, &["procedure".to_string()], &[], false, true, domain_key.as_deref(), evidence.clone());
                    let item = KnowledgeProcedure {
                        id: String::new(), concept_id: concept.as_ref().map(|row| row.id.clone()), concept_key: concept.as_ref().map(|row| row.key.clone()).unwrap_or_else(|| canonical(&name)), name,
                        interaction_mode: "single_turn".to_string(), roles: vec!["assistant".to_string()], sequence: Vec::new(), requirements: Vec::new(), mechanisms: Vec::new(), optional_mechanisms: Vec::new(), purpose: None, confidence: 0.86, evidence: Vec::new(),
                    };
                    memory.store_procedure(item, evidence);
                }
            }
            "procedure_step" => {
                let procedure_name = clean(&block.scope);
                if !procedure_name.is_empty() {
                    let concept = memory.upsert_concept(&procedure_name, &["procedure".to_string()], &[], false, true, domain_key.as_deref(), evidence.clone());
                    let mut sequence = memory.knowledge.procedures.iter().find(|row| canonical(&row.name) == canonical(&procedure_name)).map(|row| row.sequence.clone()).unwrap_or_default();
                    let step: Value = json!({"step": clean(&block.text), "sourceRole": block.role, "sourceId": source_id});
                    if !sequence.iter().any(|old| canonical(&old.to_string()) == canonical(&step.to_string())) { sequence.push(step); }
                    let item = KnowledgeProcedure {
                        id: String::new(), concept_id: concept.as_ref().map(|row| row.id.clone()), concept_key: concept.as_ref().map(|row| row.key.clone()).unwrap_or_else(|| canonical(&procedure_name)), name: procedure_name,
                        interaction_mode: "single_turn".to_string(), roles: vec!["assistant".to_string()], sequence, requirements: Vec::new(), mechanisms: Vec::new(), optional_mechanisms: Vec::new(), purpose: None, confidence: 0.86, evidence: Vec::new(),
                    };
                    memory.store_procedure(item, evidence);
                }
            }
            _ => {}
        }
    }

    memory.upsert_source(SemanticSourceRecord {
        id: source_id.clone(), name: input.source_name.clone(), display_name: parsed.source_display_name.clone(), source_type: input.source_type.clone().unwrap_or_else(|| "learned".to_string()), source_role: role.to_string(), document_kind: parsed.document_kind.clone(), lesson_number: parsed.lesson_number, text: input.text.clone(), learned_at: now,
    });

    let counts = SemanticLearningCounts {
        concepts: memory.knowledge.concepts.len().saturating_sub(before.0),
        claims: memory.knowledge.claims.len().saturating_sub(before.1),
        procedures: memory.knowledge.procedures.len().saturating_sub(before.2),
        guidance: memory.knowledge.guidance.len().saturating_sub(before.3),
        examples: memory.knowledge.examples.len().saturating_sub(before.4),
    };
    let report = SemanticLearningReport { source_id, source_name: input.source_name.clone(), source_role: role.to_string(), domain: Some(domain), counts, created_at: now, authority: "rust_v7_pass_6".to_string() };
    memory.finish_learning_update(report.clone());
    report
}
