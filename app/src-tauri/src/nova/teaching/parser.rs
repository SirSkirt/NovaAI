pub(crate) use crate::nova::ids::stable_source_id;

use super::types::{ParsedTeachingSource, SourceIdentity, TeachingBlock, TeachingSourceInput};

pub const RUST_TEACHING_PARSER_SCHEMA: u32 = 1;

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

pub(crate) fn canonical(value: &str) -> String {
    clean(value).to_ascii_lowercase().replace('_', " " ).replace('-', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stem(name: &str) -> String {
    let base = name.rsplit(|ch| ch == '/' || ch == '\\').next().unwrap_or(name);
    base.rsplit_once('.').map(|(left, _)| left).unwrap_or(base).to_string()
}

fn number_after_phrase(text: &str, phrase: &str) -> Option<u64> {
    let lower = text.to_ascii_lowercase();
    let index = lower.find(phrase)?;
    let rest = &text[index + phrase.len()..];
    let digits = rest.chars().skip_while(|ch| ch.is_whitespace()).take_while(|ch| ch.is_ascii_digit()).collect::<String>();
    if digits.is_empty() { None } else { digits.parse().ok() }
}

pub fn source_identity(name: &str, source_type: &str, kind_hint: Option<&str>) -> SourceIdentity {
    let raw = if name.trim().is_empty() { "document" } else { name };
    if canonical(source_type) == "conversation" || canonical(raw) == "conversation" {
        return SourceIdentity { kind: "conversation".to_string(), lesson_number: None, display_name: "Conversation".to_string(), source_name: raw.to_string() };
    }
    if let Some(hint) = kind_hint.map(canonical).filter(|hint| matches!(hint.as_str(), "lesson" | "test" | "document")) {
        let lesson_number = number_after_phrase(&canonical(&stem(raw)), "lesson ");
        let display_name = match (hint.as_str(), lesson_number) {
            ("lesson", Some(number)) => format!("Lesson {number}"),
            ("test", Some(number)) => format!("Test Lesson {number}"),
            ("lesson", None) => "Lesson".to_string(),
            ("test", None) => "Test".to_string(),
            _ => "Document".to_string(),
        };
        return SourceIdentity { kind: hint, lesson_number, display_name, source_name: raw.to_string() };
    }
    let spaced = canonical(&stem(raw));
    if let Some(number) = number_after_phrase(&spaced, "test lesson ") {
        return SourceIdentity { kind: "test".to_string(), lesson_number: Some(number), display_name: format!("Test Lesson {number}"), source_name: raw.to_string() };
    }
    if let Some(number) = number_after_phrase(&spaced, "lesson ") {
        let test_at = spaced.find("test");
        let lesson_at = spaced.find("lesson");
        let kind = if matches!((test_at, lesson_at), (Some(test), Some(lesson)) if test < lesson) { "test" } else { "lesson" };
        return SourceIdentity { kind: kind.to_string(), lesson_number: Some(number), display_name: if kind == "test" { format!("Test Lesson {number}") } else { format!("Lesson {number}") }, source_name: raw.to_string() };
    }
    if spaced.starts_with("test") {
        return SourceIdentity { kind: "test".to_string(), lesson_number: None, display_name: "Test".to_string(), source_name: raw.to_string() };
    }
    if spaced.starts_with("lesson") {
        return SourceIdentity { kind: "lesson".to_string(), lesson_number: None, display_name: "Lesson".to_string(), source_name: raw.to_string() };
    }
    SourceIdentity { kind: "document".to_string(), lesson_number: None, display_name: "Document".to_string(), source_name: raw.to_string() }
}

fn label_role(label: &str) -> Option<&'static str> {
    match canonical(label).as_str() {
        "subject" | "topic" | "purpose" | "title" | "heading" | "summary" | "end" => Some("metadata"),
        "lesson" | "lesson material" | "teaching" | "teaching material" => Some("section_lesson"),
        "test" | "tests" | "test material" | "testing" | "testing material" => Some("section_test"),
        "test instruction" | "test instructions" => Some("test_instruction"),
        "expected answer" | "expected response" => Some("test_expected_answer"),
        "expected behavior" | "expected behaviour" | "expected action" | "expected result" => Some("test_expected_behavior"),
        "expected reasoning" => Some("test_expected_reasoning"),
        "rubric" | "criteria" => Some("test_rubric"),
        "concept" => Some("concept"),
        "category" => Some("category"),
        "parent category" => Some("category_relation"),
        "definition" | "meaning" => Some("definition"),
        "relationship" | "relationships" => Some("relationship"),
        "guidance" | "rule" | "rules" | "principle" | "important" => Some("guidance"),
        "behavior" => Some("behavior"),
        "warning" | "warnings" => Some("warning"),
        "constraint" | "constraints" => Some("constraint"),
        "correction" => Some("correction"),
        "validation rule" => Some("validation_rule"),
        "procedure" => Some("procedure"),
        "step" | "steps" => Some("procedure_step"),
        "structure" | "pattern" => Some("structure"),
        "example" | "examples" | "response" | "text" => Some("example"),
        "practice" => Some("practice"),
        "question" => Some("question"),
        "answer" => Some("answer"),
        "reason" | "why it works" | "why this works" => Some("explanation"),
        "not" | "not enough" | "incorrect interpretation" | "overgeneralization" | "undergeneralization" => Some("negative_example"),
        "source" | "classification" => Some("evidence"),
        _ => None,
    }
}

fn split_label(line: &str) -> Option<(String, String, String)> {
    let (left, right) = line.split_once(':')?;
    if left.trim().is_empty() || left.trim().len() > 50 { return None; }
    let role = label_role(left.trim())?;
    Some((canonical(left), role.to_string(), right.trim().to_string()))
}

fn strip_outer(value: &str) -> String {
    let value = value.trim();
    let pairs = [('[' , ']'), ('(', ')'), ('{', '}'), ('"', '"'), ('\'', '\''), ('“', '”'), ('‘', '’')];
    for (left, right) in pairs {
        if value.starts_with(left) && value.ends_with(right) && value.chars().count() >= 2 {
            return value.chars().skip(1).take(value.chars().count().saturating_sub(2)).collect::<String>().trim().to_string();
        }
    }
    value.to_string()
}

fn heading_text(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        let heading = trimmed.trim_start_matches('#').trim();
        return (!heading.is_empty()).then(|| clean(heading));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("section ") {
        if let Some((_, right)) = trimmed.split_once(':') { return Some(clean(right)); }
    }
    if matches!(canonical(trimmed).as_str(), "lesson" | "lesson material" | "teaching" | "teaching material" | "test" | "test material" | "testing" | "testing material") {
        return Some(clean(trimmed));
    }
    None
}

fn divider(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && (trimmed.chars().all(|ch| ch == '-') || trimmed.chars().all(|ch| ch == '=') || trimmed.chars().all(|ch| ch == '_'))
}

fn section_kind_from_heading(value: &str) -> Option<&'static str> {
    let value = canonical(value);
    if value.starts_with("test") || value.starts_with("testing") { Some("test") }
    else if value.starts_with("lesson") || value.starts_with("teaching") { Some("lesson") }
    else { None }
}

fn looks_structured(text: &str) -> bool {
    text.lines().filter(|line| split_label(line.trim()).is_some()).take(2).count() >= 2
}

#[derive(Default)]
struct OpenBlock {
    id: String,
    label: String,
    role: String,
    lines: Vec<String>,
    scope: String,
    line_start: usize,
    line_end: usize,
    inline: bool,
    section_kind: String,
}

pub fn parse_source(input: &TeachingSourceInput) -> ParsedTeachingSource {
    let source_type = input.source_type.as_deref().unwrap_or("learned");
    let identity = source_identity(&input.source_name, source_type, input.kind_hint.as_deref());
    let mut blocks = Vec::<TeachingBlock>::new();
    let mut current: Option<OpenBlock> = None;
    let mut heading = identity.display_name.clone();
    let mut topic: Option<String> = None;
    let mut concept: Option<String> = None;
    let mut category: Option<String> = None;
    let mut procedure: Option<String> = None;
    let mut section_kind = if identity.kind == "test" { "test".to_string() } else { "lesson".to_string() };

    let scope_now = |procedure: &Option<String>, concept: &Option<String>, category: &Option<String>, topic: &Option<String>, heading: &str| {
        procedure.clone().or_else(|| concept.clone()).or_else(|| category.clone()).or_else(|| topic.clone()).unwrap_or_else(|| heading.to_string())
    };

    let flush = |current: &mut Option<OpenBlock>, blocks: &mut Vec<TeachingBlock>, topic: &mut Option<String>, concept: &mut Option<String>, category: &mut Option<String>, procedure: &mut Option<String>| {
        let Some(open) = current.take() else { return; };
        let text = strip_outer(&open.lines.join("\n"));
        if text.is_empty() && !matches!(open.role.as_str(), "metadata" | "concept" | "category" | "procedure" | "test_instruction") { return; }
        let block = TeachingBlock {
            id: open.id,
            label: open.label.clone(),
            role: open.role.clone(),
            text: text.clone(),
            scope: open.scope,
            line_start: open.line_start,
            line_end: open.line_end,
            inline: open.inline,
            section_kind: open.section_kind,
        };
        blocks.push(block);
        if !text.is_empty() {
            match open.role.as_str() {
                "metadata" if open.label == "topic" => {
                    *topic = Some(text);
                    *concept = None;
                    *procedure = None;
                }
                "concept" => {
                    *concept = Some(text);
                    *procedure = None;
                }
                "category" => {
                    *category = Some(text);
                    *procedure = None;
                }
                "procedure" => {
                    *procedure = Some(text);
                }
                _ => {}
            }
        }
    };

    let normalized_text = input.text.replace("\r\n", "\n").replace('\r', "\n");
    for (index, raw) in normalized_text.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw.trim();
        if divider(trimmed) {
            flush(&mut current, &mut blocks, &mut topic, &mut concept, &mut category, &mut procedure);
            continue;
        }
        if let Some(new_heading) = heading_text(trimmed) {
            flush(&mut current, &mut blocks, &mut topic, &mut concept, &mut category, &mut procedure);
            if let Some(kind) = section_kind_from_heading(&new_heading) { section_kind = kind.to_string(); }
            heading = new_heading.clone(); topic = None; concept = None; category = None; procedure = None;
            blocks.push(TeachingBlock { id: format!("block_{}", blocks.len() + 1), label: "heading".to_string(), role: "metadata".to_string(), text: new_heading.clone(), scope: new_heading, line_start: line_no, line_end: line_no, inline: true, section_kind: section_kind.clone() });
            continue;
        }
        if let Some((label, role, inline)) = split_label(trimmed) {
            flush(&mut current, &mut blocks, &mut topic, &mut concept, &mut category, &mut procedure);
            if role == "section_lesson" || role == "section_test" {
                section_kind = if role == "section_test" { "test" } else { "lesson" }.to_string();
                if !inline.trim().is_empty() { heading = clean(&inline); }
                topic = None; concept = None; category = None; procedure = None;
                continue;
            }
            let scope = scope_now(&procedure, &concept, &category, &topic, &heading);
            current = Some(OpenBlock {
                id: format!("block_{}", blocks.len() + 1), label: label.clone(), role: role.clone(), lines: if inline.trim().is_empty() { Vec::new() } else { vec![inline.clone()] }, scope,
                line_start: line_no, line_end: line_no, inline: !inline.trim().is_empty(), section_kind: section_kind.clone(),
            });
            if role == "metadata" && label == "topic" && !inline.trim().is_empty() { topic = Some(strip_outer(&inline)); concept = None; procedure = None; }
            if role == "concept" && !inline.trim().is_empty() { concept = Some(strip_outer(&inline)); procedure = None; }
            if role == "category" && !inline.trim().is_empty() { category = Some(strip_outer(&inline)); procedure = None; }
            if role == "procedure" && !inline.trim().is_empty() { procedure = Some(strip_outer(&inline)); }
            continue;
        }
        if let Some(open) = current.as_mut() {
            open.lines.push(raw.to_string());
            open.line_end = line_no;
            continue;
        }
        if trimmed.is_empty() { continue; }
        current = Some(OpenBlock {
            id: format!("block_{}", blocks.len() + 1), label: "body".to_string(), role: "body".to_string(), lines: vec![raw.to_string()],
            scope: scope_now(&procedure, &concept, &category, &topic, &heading), line_start: line_no, line_end: line_no, inline: false, section_kind: section_kind.clone(),
        });
    }
    flush(&mut current, &mut blocks, &mut topic, &mut concept, &mut category, &mut procedure);

    let has_test_material = identity.kind == "test" || blocks.iter().any(|block| block.section_kind == "test" && matches!(block.role.as_str(), "question" | "test_instruction" | "test_expected_answer" | "test_expected_behavior" | "test_expected_reasoning" | "test_rubric"));
    let instructional = identity.kind == "lesson" || blocks.iter().any(|block| block.section_kind != "test" && matches!(block.role.as_str(),
        "concept" | "category" | "category_relation" | "definition" | "relationship" | "guidance" | "behavior" | "warning" | "constraint" | "correction" | "validation_rule" | "procedure" | "procedure_step" | "structure" | "practice" | "explanation"
    ));

    ParsedTeachingSource {
        schema_version: RUST_TEACHING_PARSER_SCHEMA,
        source_name: identity.source_name,
        source_display_name: identity.display_name,
        document_kind: identity.kind,
        lesson_number: identity.lesson_number,
        structured: looks_structured(&input.text),
        instructional,
        has_test_material,
        blocks,
    }
}
