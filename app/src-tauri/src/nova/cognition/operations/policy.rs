use serde_json::{json, Value};

use crate::nova::cognition::path::{append_path, canon, get_path, get_path_cloned, resolve_template, set_path, value_text};

fn policy_bucket(op: &str) -> Option<&'static str> {
    match op {
        "declare_invariant" => Some("moduleState.policyStore.invariants"),
        "declare_ordering" => Some("moduleState.policyStore.orderings"),
        "declare_reasoning_policy" => Some("canonicalSemanticState.epistemic.reasoningPolicies"),
        "register_cognitive_constraint" => Some("canonicalSemanticState.epistemic.constraintPolicies"),
        "declare_learned_predicate" => Some("moduleState.policyStore.predicates"),
        "declare_state_transition" => Some("moduleState.policyStore.transitions"),
        "declare_preference_policy" => Some("moduleState.policyStore.preferences"),
        "declare_task_resource_policy" => Some("moduleState.policyStore.resourcePolicies"),
        "declare_fidelity_policy" => Some("moduleState.policyStore.fidelityPolicies"),
        "declare_representation_invariant" => Some("moduleState.policyStore.representationInvariants"),
        "declare_stop_condition" => Some("canonicalSemanticState.epistemic.stopPolicies"),
        _ => None,
    }
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool,String> {
    if let Some(path)=policy_bucket(op) {
        let mut spec=resolve_template(args.get("spec").unwrap_or(args),ctx);
        if let Some(object)=spec.as_object_mut(){object.insert("source".into(),Value::String("rust_learned_module".into()));}
        return Ok(append_path(ctx,path,spec,true));
    }
    match op {
        "assert_paths_present" => {
            let paths=args.get("paths").and_then(Value::as_array).cloned().unwrap_or_default();let missing:Vec<_>=paths.iter().filter_map(Value::as_str).filter(|p|get_path(ctx,p).is_none()).map(|p|Value::String(p.into())).collect();
            let present = missing.is_empty();
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.pathAssertion"),json!({"present":present,"missing":missing})))
        }
        "compose_structures" => {
            let mut members=Vec::new();for path in args.get("sourcePaths").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str){if let Some(value)=get_path_cloned(ctx,path){if let Some(items)=value.as_array(){members.extend(items.clone())}else{members.push(value)}}}
            let member_count = members.len();
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("composedStructure"),json!({"kind":args.get("kind").and_then(Value::as_str).unwrap_or("composed_structure"),"memberCount":member_count,"members":members,"preserveMembers":true})))
        }
        "mark_epistemic_state" => {
            let key=args.get("key").or_else(||args.get("propositionId")).map(|v|value_text(Some(&resolve_template(v,ctx)))).filter(|s|!s.is_empty()).unwrap_or_else(||"current".into());
            let value = resolve_template(args,ctx);
            Ok(set_path(ctx,&format!("canonicalSemanticState.epistemic.propositions.{key}"),value))
        }
        "register_evidence_relation" => {
            let value = resolve_template(args,ctx);
            Ok(append_path(ctx,"canonicalSemanticState.epistemic.evidenceRelations",value,true))
        }
        "summarize_evidence_support" => {
            let items=get_path_cloned(ctx,"canonicalSemanticState.epistemic.evidenceRelations").and_then(|v|v.as_array().cloned()).unwrap_or_default();let supporting=items.iter().filter(|x|canon(&value_text(x.get("relation")))=="supports").count();let contradicting=items.iter().filter(|x|canon(&value_text(x.get("relation")))=="contradicts").count();
            let count = items.len();
            let available = !items.is_empty();
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("epistemicSupport"),json!({"count":count,"supporting":supporting,"contradicting":contradicting,"available":available,"relations":items})))
        }
        "update_belief_state" => {
            let summary=get_path_cloned(ctx,args.get("supportPath").and_then(Value::as_str).unwrap_or("epistemicSupport")).unwrap_or(json!({}));let status=args.get("status").and_then(Value::as_str).map(str::to_string).unwrap_or_else(||{let s=summary.get("supporting").and_then(Value::as_u64).unwrap_or(0);let c=summary.get("contradicting").and_then(Value::as_u64).unwrap_or(0);if s>0&&c==0{"supported".into()}else if c>0&&s==0{"contradicted".into()}else{"uncertain".into()}});let key=args.get("key").and_then(Value::as_str).unwrap_or("current");
            Ok(set_path(ctx,&format!("canonicalSemanticState.epistemic.beliefs.{key}"),json!({"status":status,"support":summary,"source":"rust_learned_module"})))
        }
        "record_cognitive_attempt" => {
            let value = resolve_template(args,ctx);
            Ok(append_path(ctx,"canonicalSemanticState.epistemic.attempts",value,false))
        }
        "compare_attempt_progress" => {
            let attempts=get_path(ctx,"canonicalSemanticState.epistemic.attempts").and_then(Value::as_array);let count=attempts.map_or(0,Vec::len);let repeated=count>=2&&attempts.and_then(|v|v.get(count-1)).map(|v|v.to_string())==attempts.and_then(|v|v.get(count-2)).map(|v|v.to_string());Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("attemptProgress"),json!({"count":count,"sameMethod":repeated,"changed":!repeated,"repeatWithoutNewProgress":repeated})))
        }
        "derive_semantic_binding" => {
            let value=args.get("value").or_else(||args.get("fromPath").and_then(Value::as_str).and_then(|p|get_path(ctx,p))).cloned().unwrap_or(Value::Null);
            let resolved = resolve_template(&value,ctx);
            Ok(set_path(ctx,args.get("outputPath").and_then(Value::as_str).unwrap_or("semanticBinding"),resolved))
        }
        "resolve_relative_discourse" | "require_contextual_resolution" | "mark_ambiguity_by_score" | "select_information_action" => Err(format!("unsupported_operation:{op}")),
        _ => Err(format!("unsupported_operation:{op}")),
    }
}
