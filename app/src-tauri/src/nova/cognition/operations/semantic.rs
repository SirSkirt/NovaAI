use serde_json::{json, Value};

use crate::nova::cognition::path::{append_path, canon, get_path, get_path_cloned, resolve_template, set_path, value_text};

fn append_semantic(ctx: &mut Value, bucket: &str, value: Value) -> bool {
    append_path(ctx, &format!("canonicalSemanticState.{bucket}"), value, true)
}

fn words(value: &str) -> std::collections::BTreeSet<String> {
    value.split(|c: char| !c.is_alphanumeric()).map(canon).filter(|w| w.len()>1).collect()
}

fn resolve_concept(args: &Value, ctx: &Value) -> Option<Value> {
    let explicit = args.get("concept").or_else(|| args.get("value")).map(|v| resolve_template(v,ctx))
        .or_else(|| args.get("fromPath").and_then(Value::as_str).and_then(|p|get_path_cloned(ctx,p)))
        .unwrap_or_else(|| get_path_cloned(ctx,"rawText").unwrap_or(Value::Null));
    if explicit.get("id").is_some() { return Some(explicit); }
    let text = canon(&value_text(Some(&explicit)));
    let query = words(&text);
    let concepts = get_path(ctx,"knowledge.concepts")?.as_array()?;
    let mut best: Option<(i64,Value)> = None;
    for concept in concepts {
        let labels = std::iter::once(concept.get("key")).chain(std::iter::once(concept.get("name"))).flatten()
            .chain(concept.get("aliases").and_then(Value::as_array).into_iter().flatten());
        let mut score=0i64;
        for label in labels {
            let label_text=canon(&value_text(Some(label)));
            if label_text==text { score=score.max(100); }
            let overlap=words(&label_text).intersection(&query).count() as i64;
            score=score.max(overlap*20);
        }
        if score>best.as_ref().map(|x|x.0).unwrap_or(0) { best=Some((score,concept.clone())); }
    }
    best.filter(|(score,_)|*score>=18).map(|(_,concept)|concept)
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool,String> {
    if let Some(bucket)=op.strip_prefix("semantic_add_") {
        let bucket=match bucket {"frame"=>"frames","entity"=>"entities","event"=>"events","relation"=>"relations","proposition"=>"propositions","request"=>"requests","unknown_slot"=>"unknownSlots","candidate_set"=>"candidateSets","evidence"=>"evidence","conclusion"=>"conclusions",_=>return Err("not_semantic".to_string())};
        let resolved=resolve_template(args,ctx);
        let normalized=super::semantic_records::normalize(bucket,resolved,ctx)?;
        return Ok(append_semantic(ctx,bucket,normalized));
    }
    match op {
        "create_structure" => {
            let value = resolve_template(args.get("value").or_else(||args.get("structure")).unwrap_or(&json!({})),ctx);
            Ok(append_path(ctx,args.get("listPath").and_then(Value::as_str).unwrap_or("canonicalSemanticState.structures"),value,args.get("unique").and_then(Value::as_bool).unwrap_or(true)))
        }
        "query_structures" => {
            let list=get_path_cloned(ctx,args.get("listPath").and_then(Value::as_str).unwrap_or("canonicalSemanticState.structures")).and_then(|v|v.as_array().cloned()).unwrap_or_default();
            let match_spec=args.get("match").and_then(Value::as_object);
            let out:Vec<_>=list.into_iter().filter(|item|match_spec.is_none_or(|spec|spec.iter().all(|(path,expected)|canon(&value_text(get_path(item,path)))==canon(&value_text(Some(&resolve_template(expected,ctx))))))).collect();
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("queryResults"),Value::Array(out)))
        }
        "link_structures" => {
            let relation=json!({"subject":resolve_template(args.get("subject").unwrap_or(&Value::Null),ctx),"subjectId":null,"predicate":resolve_template(args.get("predicate").unwrap_or(&Value::String("related_to".into())),ctx),"object":resolve_template(args.get("object").unwrap_or(&Value::Null),ctx),"objectId":null,"qualifiers":resolve_template(args.get("qualifiers").unwrap_or(&json!({})),ctx),"source":"rust_learned_module"});
            let bucket=args.get("bucket").and_then(Value::as_str).unwrap_or("relations");
            let normalized=super::semantic_records::normalize(bucket,relation,ctx)?;
            Ok(append_semantic(ctx,bucket,normalized))
        }
        "create_goal" | "register_unresolved_goal" => {
            let raw=resolve_template(args.get("goal").unwrap_or(args),ctx);
            let id=value_text(raw.get("id"));
            let need_type=value_text(raw.get("type").or_else(||raw.get("goalType")));
            let mut goal=json!({
                "id": if id.is_empty(){format!("rust_module_goal_{}",crate::nova::mind::unix_ms())}else{id},
                "type": if need_type.is_empty(){"learned_goal"}else{need_type.as_str()},
                "description": raw.get("description").or_else(||raw.get("label")).cloned().unwrap_or(Value::Null),
                "goalId": raw.get("goalId").cloned().unwrap_or(Value::Null),
                "requestId": raw.get("requestId").cloned().unwrap_or(Value::Null),
                "requiredCapabilities": raw.get("requiredCapabilities").cloned().unwrap_or_else(||json!([])),
                "attributes": raw.clone()
            });
            if let (Some(goal_object), Some(raw_object)) = (goal.as_object_mut(), raw.as_object()) {
                for (key, value) in raw_object {
                    if !goal_object.contains_key(key) { goal_object.insert(key.clone(), value.clone()); }
                }
            }
            Ok(append_semantic(ctx,"unresolved",goal))
        }
        "resolve_goal" | "resolve_unresolved_goal" => {
            let goals=get_path_cloned(ctx,"canonicalSemanticState.unresolved").and_then(|v|v.as_array().cloned()).unwrap_or_default();
            let id=args.get("id").or_else(||args.get("goalId")).map(|v|value_text(Some(&resolve_template(v,ctx))));
            let kind=args.get("type").or_else(||args.get("goalType")).map(|v|canon(&value_text(Some(&resolve_template(v,ctx)))));
            let next:Vec<_>=goals.iter().filter(|goal|{
                if id.as_ref().is_some_and(|wanted|value_text(goal.get("id"))==*wanted){return false;}
                if kind.as_ref().is_some_and(|wanted|canon(&value_text(goal.get("type")))==*wanted){return false;}
                true
            }).cloned().collect();
            if next.len()==goals.len(){Ok(false)}else{Ok(set_path(ctx,"canonicalSemanticState.unresolved",Value::Array(next)))}
        }
        "knowledge_resolve_concept" => {
            let Some(concept)=resolve_concept(args,ctx) else{return Ok(false)};
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.focusConcept"),concept))
        }
        "knowledge_collect_relations" => {
            let Some(concept)=resolve_concept(args,ctx) else{return Ok(false)};
            let id=value_text(concept.get("id"));
            let claims=get_path(ctx,"knowledge.claims").and_then(Value::as_array).cloned().unwrap_or_default();
            let edges:Vec<_>=claims.into_iter().filter(|claim|value_text(claim.get("subjectId"))==id||value_text(claim.get("objectConceptId"))==id).collect();
            let mut changed=set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.focusRelations"),Value::Array(edges));
            if let Some(path)=args.get("conceptOutputPath").and_then(Value::as_str){changed=set_path(ctx,path,concept)||changed;}
            Ok(changed)
        }
        "knowledge_select_relevant_relations" => {
            let list=get_path_cloned(ctx,args.get("listPath").and_then(Value::as_str).unwrap_or("moduleState.focusRelations")).and_then(|v|v.as_array().cloned()).unwrap_or_default();
            let goal_text=get_path(ctx,"canonicalSemanticState.unresolved").map(Value::to_string).unwrap_or_default();let goal=words(&goal_text);
            let mut scored:Vec<_>=list.into_iter().map(|mut edge|{let overlap=words(&edge.to_string()).intersection(&goal).count() as f64;edge.as_object_mut().map(|o|o.insert("relevanceScore".into(),json!(overlap*4.0+2.0)));edge}).collect();
            scored.sort_by(|a,b|b.get("relevanceScore").and_then(Value::as_f64).partial_cmp(&a.get("relevanceScore").and_then(Value::as_f64)).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(args.get("limit").and_then(Value::as_u64).unwrap_or(12) as usize);
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.relevantRelations"),Value::Array(scored)))
        }
        "knowledge_form_supported_proposition" => {
            let list=get_path(ctx,args.get("listPath").and_then(Value::as_str).unwrap_or("moduleState.relevantRelations")).and_then(Value::as_array);
            let Some(edge)=list.and_then(|v|v.get(args.get("index").and_then(Value::as_u64).unwrap_or(0) as usize)).cloned() else{return Ok(false)};
            let prop=json!({"id":format!("rust_module_prop_{}",crate::nova::mind::unix_ms()),"frameId":"frame_discourse","type":"supported_knowledge_relation","subject":value_text(edge.get("subject")),"subjectId":edge.get("subjectId").cloned(),"predicate":value_text(edge.get("predicate")),"object":value_text(edge.get("object")),"objectId":edge.get("objectConceptId").cloned(),"polarity":value_text(edge.get("polarity")),"confidence":edge.get("confidence").and_then(Value::as_f64).unwrap_or(0.5),"source":"rust_learned_module","evidence":edge.get("evidence").cloned().unwrap_or(json!([])),"support":{"claimId":edge.get("id").cloned()}});
            let mut changed=set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.supportedProposition"),prop.clone());if args.get("commit").and_then(Value::as_bool).unwrap_or(false){changed=append_semantic(ctx,"propositions",prop)||changed;}Ok(changed)
        }
        "select_labeled_role" => {
            let item=args.get("item").map(|v|resolve_template(v,ctx)).or_else(||args.get("itemPath").and_then(Value::as_str).and_then(|p|get_path_cloned(ctx,p))).or_else(||get_path_cloned(ctx,"instructionalItem"));let Some(item)=item else{return Ok(false)};let role=value_text(item.get("role").or_else(||item.get("label")).or_else(||item.get("type")));if role.is_empty(){return Ok(false)};Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.selectedInstructionRole"),json!({"role":role,"label":item.get("label"),"scope":item.get("scope"),"source":"explicit_label"})))
        }
        "evaluate_support_conflict" => {
            let mut values=Vec::new();
            if let Some(paths)=args.get("paths").and_then(Value::as_array){for path in paths.iter().filter_map(Value::as_str){if let Some(value)=get_path_cloned(ctx,path){if let Some(items)=value.as_array(){values.extend(items.clone())}else{values.push(value)}}}}
            if values.is_empty(){for path in ["canonicalSemanticState.propositions","canonicalSemanticState.evidence"]{if let Some(items)=get_path(ctx,path).and_then(Value::as_array){values.extend(items.clone())}}}
            let mut groups:std::collections::BTreeMap<String,(Vec<Value>,Vec<Value>)>=std::collections::BTreeMap::new();
            for item in values{let key=canon(&format!("{}|{}|{}",value_text(item.get("subjectId").or_else(||item.get("subject"))),value_text(item.get("predicate").or_else(||item.get("type"))),value_text(item.get("objectId").or_else(||item.get("object")).or_else(||item.get("value")))));if key.is_empty(){continue;}let polarity=canon(&value_text(item.get("polarity").or_else(||item.get("status"))));let group=groups.entry(key).or_default();if matches!(polarity.as_str(),"negative"|"false"|"rejected"|"contradicted"){group.1.push(item)}else{group.0.push(item)}}
            let result=groups.into_iter().map(|(key,(support,conflict))|json!({"key":key,"hasConflict":!support.is_empty()&&!conflict.is_empty(),"support":support,"conflict":conflict})).collect();
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.supportConflict"),Value::Array(result)))
        },
        "inspect_cognitive_status" => {
            let status = get_path_cloned(ctx,"cognitiveStatus").unwrap_or(json!({}));
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.cognitiveStatus"),status))
        },
        _ => Err("not_semantic".to_string()),
    }
}
