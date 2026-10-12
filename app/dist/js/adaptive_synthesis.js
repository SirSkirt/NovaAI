"use strict";

/* =========================================================
   Nova V6.2.16 — ADAPTIVE GENERIC SYNTHESIS

   Purpose:
   - deepen the V5.2/V5.2.1 synthesizer without adding lesson-shaped functions;
   - provide a few reusable structured-state operations that can unlock many
     retained cognitive lessons;
   - turn broad missing-capability diagnostics into declarative learned programs;
   - keep genuinely underspecified guidance Stored Only rather than pretending it
     is executable;
   - keep all learned programs inside the safe declarative cognitive VM.
   ========================================================= */

const NOVA522_VERSION = "6.2.16";
const NOVA522_ADAPTIVE_SYNTHESIS_SCHEMA = 2;

function v522Clone(value){
  try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(_){return value;}}
}
function v522Canonical(value){
  try{return canonical(String(value??""));}catch(_){return String(value??"").trim().toLowerCase();}
}
function v522Clean(value){
  try{return normalizeWhitespace(String(value??""));}catch(_){return String(value??"").replace(/\s+/g," ").trim();}
}
function v522Unique(values){
  const out=[];const seen=new Set();
  for(const value of values||[]){
    const key=typeof value==="object"?JSON.stringify(value):v522Canonical(value);
    if(!key||seen.has(key))continue;seen.add(key);out.push(value);
  }
  return out;
}
function v522ReadPath(obj,path){return typeof v502GetPath==="function"?v502GetPath(obj,path):undefined;}
function v522WritePath(obj,path,value){return typeof v502SetPath==="function"?v502SetPath(obj,path,value):false;}
function v522Primitive(value){
  if(value==null)return null;
  if(typeof value==="string"||typeof value==="number"||typeof value==="boolean")return value;
  if(typeof value==="object")return value.id??value.label??value.entity??value.name??value.value??value.target??null;
  return null;
}
function v522WalkStructure(value,visitor,path="",depth=0,seen=new Set()){
  if(value==null||depth>8)return;
  if(typeof value!=="object")return;
  if(seen.has(value))return;seen.add(value);
  if(Array.isArray(value)){
    value.forEach((item,index)=>{visitor(item,String(index),`${path}[${index}]`,value);v522WalkStructure(item,visitor,`${path}[${index}]`,depth+1,seen);});
    return;
  }
  for(const [key,item] of Object.entries(value)){
    const childPath=path?`${path}.${key}`:key;
    visitor(item,key,childPath,value);
    v522WalkStructure(item,visitor,childPath,depth+1,seen);
  }
}
function v522SemanticRoots(ctx,paths=[]){
  const roots=[];
  const add=(value,path)=>{if(value&&typeof value==="object"&&!roots.some(x=>x.value===value))roots.push({value,path});};
  for(const path of paths||[])add(v522ReadPath(ctx,path),path);
  if(!roots.length){
    for(const path of [
      "semanticComposition","interpretation","workspace.semanticComposition",
      "workspace.interpretation","workspace.novaMeaning","workspace.requestFrame",
      "workspace.integratedCognition.semanticState"
    ])add(v522ReadPath(ctx,path),path);
  }
  return roots;
}
function v522FindSemanticBindings(ctx,roleNames=[],sourcePaths=[]){
  const roles=new Set((roleNames||[]).map(v522Canonical));const matches=[];
  for(const root of v522SemanticRoots(ctx,sourcePaths)){
    v522WalkStructure(root.value,(item,key,path)=>{
      const k=v522Canonical(key).replace(/\s+/g,"_");
      if(!roles.has(k))return;
      const primitive=v522Primitive(item);
      if(primitive==null||primitive==="")return;
      matches.push({role:k,value:v522Clone(item),primitive,path:`${root.path}.${path}`});
    });
  }
  return matches;
}
function v522NormalizeParticipant(value,role="listener"){
  if(value&&typeof value==="object"){
    const id=value.id??value.entityId??value.participantId??value.value??value.label??value.name;
    const label=value.label??value.name??value.entity??value.value??id;
    if(id!=null||label!=null)return {id:String(id??label),label:String(label??id),role};
  }
  const primitive=v522Primitive(value);
  if(primitive==null)return null;
  return {id:String(primitive),label:String(primitive),role};
}
function v522StructureContains(structure,target){
  const needle=v522Canonical(v522Primitive(target));if(!needle||!structure)return false;
  let found=false;
  v522WalkStructure(structure,(item,key)=>{
    if(found)return;
    const p=v522Primitive(item);if(p!=null&&v522Canonical(p)===needle)found=true;
    if(v522Canonical(key)===needle)found=true;
  });
  return found;
}
function v522ExpectedTraits(structure){
  const expected={type:null,gender:null,number:null};
  if(!structure||typeof structure!=="object")return expected;
  v522WalkStructure(structure,(item,key)=>{
    const k=v522Canonical(key);const p=v522Canonical(v522Primitive(item));
    if(!p)return;
    if(!expected.type&&/(?:expected_?type|entity_?type|participant_?type|type)$/.test(k))expected.type=p;
    if(!expected.gender&&/gender$/.test(k))expected.gender=p;
    if(!expected.number&&/(?:number|grammatical_?number)$/.test(k))expected.number=p;
  });
  return expected;
}

/* ---------------------------------------------------------
   GENERAL RUNTIME OPERATIONS

   V6.2 registers capabilities with the one current cognitive VM. No executor
   replacement and no fallback interpreter are introduced here.
   --------------------------------------------------------- */
if(typeof v502RegisterOperation === "function"){
  v502RegisterOperation("derive_semantic_binding",(args,ctx)=>{
    const roleNames=v522Unique(args.semanticRoles||[args.semanticRole,args.role].filter(Boolean));
    const matches=v522FindSemanticBindings(ctx,roleNames,args.sourcePaths||[]);
    let changed=false;
    if(args.matchesPath)changed=v522WritePath(ctx,args.matchesPath,matches)||changed;
    const chosen=matches[0]||null;if(!chosen)return changed;
    const role=args.targetRole||args.role||"listener",participant=v522NormalizeParticipant(chosen.value,role);if(!participant)return changed;
    const targets=v522Unique(args.targetPaths||[args.targetPath||`participants.${role}`]);
    for(const path of targets)changed=v522WritePath(ctx,path,v522Clone(participant))||changed;
    if(ctx.workspace){
      if(role==="listener"){
        if(ctx.workspace.referenceResolution?.participants)changed=v522WritePath(ctx.workspace,"referenceResolution.participants.listener",v522Clone(participant))||changed;
        if(ctx.workspace.interpretation?.participants)changed=v522WritePath(ctx.workspace,"interpretation.participants.listener",v522Clone(participant))||changed;
      }else if(role==="speaker"){
        if(ctx.workspace.referenceResolution?.participants)changed=v522WritePath(ctx.workspace,"referenceResolution.participants.speaker",v522Clone(participant))||changed;
        if(ctx.workspace.interpretation?.participants)changed=v522WritePath(ctx.workspace,"interpretation.participants.speaker",v522Clone(participant))||changed;
      }
    }
    changed=v522WritePath(ctx,"semanticBinding",{role,semanticRole:chosen.role,participant:v522Clone(participant),sourcePath:chosen.path})||changed;
    return changed;
  });

  v502RegisterOperation("score_candidates_by_relationship",(args,ctx)=>{
    const listPath=args.listPath||"candidates",list=Array.isArray(v522ReadPath(ctx,listPath))?v522ReadPath(ctx,listPath):[];
    const roots=v522SemanticRoots(ctx,args.relationshipPaths||[]).map(x=>x.value);
    const expected=roots.map(v522ExpectedTraits).reduce((acc,item)=>({type:acc.type||item.type,gender:acc.gender||item.gender,number:acc.number||item.number}),{type:null,gender:null,number:null});
    const scored=list.map(candidate=>{
      let score=Number(candidate?.score||0);const reasons=[...(candidate?.scoreReasons||[])];
      for(const root of roots){if(v522StructureContains(root,candidate)){score+=Number(args.explicitWeight??3);reasons.push("explicit_relationship_member");break;}}
      for(const field of ["type","gender","number"]){if(!expected[field]||!candidate?.[field])continue;if(v522Canonical(expected[field])===v522Canonical(candidate[field])){score+=Number(args.compatibleWeight??1.5);reasons.push(`${field}_relationship_fit`);}else{score-=Number(args.conflictPenalty??2);reasons.push(`${field}_relationship_conflict`);}}
      return {...candidate,score,scoreReasons:v522Unique(reasons),relationshipFit:{expected:v522Clone(expected)}};
    }).sort((a,b)=>Number(b.score||0)-Number(a.score||0));
    return v522WritePath(ctx,args.outputPath||listPath,scored);
  });

  v502RegisterOperation("resolve_relative_discourse",(args,ctx)=>{
    const relation=v522Canonical(args.relation||"previous"),role=args.role?v522Canonical(args.role):null;
    const all=Array.isArray(globalThis.state?.messages)?globalThis.state.messages:[],filtered=all.filter(message=>!role||v522Canonical(message?.role)===role);
    let selected=null;if(["previous","preceding","last","most_recent","earlier"].includes(relation)){const offset=Math.max(1,Number(args.offset||1));selected=filtered.at(-offset)||null;}else if(["current","latest"].includes(relation))selected=filtered.at(-1)||null;
    return selected?v522WritePath(ctx,args.outputPath||"relativeDiscourse.selected",v522Clone(selected)):false;
  });

  v502RegisterOperation("require_contextual_resolution",(args,ctx)=>{
    if(!ctx.mention)return false;
    const accepted=v522Unique(args.acceptedSources||["participant","learned_module","learned_quote_frame","discourse","context","semantic_binding"]),source=v522Canonical(ctx.mention.resolutionSource||"");
    const hasContext=accepted.some(item=>source.includes(v522Canonical(item)))||Boolean(ctx.selectedCandidate||ctx.quoteFrame||ctx.semanticBinding);
    if(hasContext)return false;
    const before=JSON.stringify({u:ctx.mention.uncertain,c:ctx.mention.confidence,r:ctx.contextualResolutionRequired});
    ctx.mention.uncertain=true;ctx.mention.confidence=Math.min(Number(ctx.mention.confidence??0.5),Number(args.confidence??0.45));ctx.contextualResolutionRequired=true;
    return before!==JSON.stringify({u:ctx.mention.uncertain,c:ctx.mention.confidence,r:ctx.contextualResolutionRequired});
  });
}

/* ---------------------------------------------------------
   ADAPTIVE PROGRAM SYNTHESIS

   Important: the dispatch key is the *general missing capability* reported by
   the previous synthesizer. It is not an exact curriculum sentence.
   --------------------------------------------------------- */
function v522ScopePronoun(scope){
  const s=v522Canonical(scope).replace(/["“”'’]/g,"").trim();
  const valid=new Set(["i","me","my","mine","you","your","yours","we","us","our","ours","he","him","his","she","her","hers","they","them","their","theirs","it","its","this","that","these","those"]);
  if(valid.has(s))return s;
  const tokens=s.split(/[^a-z]+/).filter(Boolean);
  return tokens.find(token=>valid.has(token))||null;
}
function v522RoleSignals(text,scope){
  const t=`${v522Canonical(scope)} ${v522Canonical(text)}`;
  if(/\b(?:listener|addressee|recipient|person being spoken to|addressed participant)\b/.test(t))return {targetRole:"listener",semanticRoles:["addressee","listener","recipient"]};
  if(/\b(?:speaker|person speaking|source participant)\b/.test(t))return {targetRole:"speaker",semanticRoles:["speaker","source","actor"]};
  return null;
}
function v522AdaptiveModule(rule,base){
  const missing=new Set(base?.diagnostic?.missingOperations||[]);const raw=v522Clean(rule?.directive||"");const t=v522Canonical(raw);const scope=v522Canonical(rule?.scope||"");const form=typeof v521ClassifyCognitiveForm==="function"?v521ClassifyCognitiveForm(rule):"operation_program";

  if(missing.has("derive_and_update_discourse_role_from_semantics")){
    const role=v522RoleSignals(raw,scope);
    if(role){
      const module={
        name:"Synthesized semantic discourse-role binding",domain:"reference",phase:"interpretation",priority:87,
        conditions:[{type:"always"}],cognitiveForm:"state_transformation",
        policySpec:{type:"semantic_role_binding",targetRole:role.targetRole,semanticRoles:role.semanticRoles,reason:raw},
        program:[{op:"derive_semantic_binding",args:{targetRole:role.targetRole,semanticRoles:role.semanticRoles,targetPaths:[`participants.${role.targetRole}`,`workspace.interpretation.participants.${role.targetRole}`,`workspace.referenceResolution.participants.${role.targetRole}`]}}]
      };
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"state_transformation",requiredOperations:["derive_semantic_binding"],program:module.program,resolvedMissingOperations:[...missing]}};
    }
  }

  if(missing.has("candidate_support_measurement")){
    const module={
      name:"Synthesized evidence-weighted reference selection",domain:"reference",phase:"reference",priority:66,
      conditions:[{type:"field_exists",field:"mention.surface"}],cognitiveForm:"selection_policy",
      policySpec:{type:"candidate_support_selection",reason:raw},
      program:[
        {op:"collect_candidates",args:{sources:["local","discourse"]}},
        {op:"score_candidates",args:{listPath:"candidates",criteria:[{kind:"local",weight:1.5},{kind:"recent",weight:1}]}},
        {op:"score_candidates_by_relationship",args:{listPath:"candidates",relationshipPaths:["workspace.semanticComposition","workspace.interpretation","semanticComposition"]}},
        {op:"select_best_candidate",args:{listPath:"candidates",outputPath:"selectedCandidate"}},
        {op:"mark_ambiguity_by_score",args:{listPath:"candidates",outputPath:"ambiguous",tolerance:0.35,confidence:0.45}},
        {op:"branch",args:{if:{type:"field_equals",field:"ambiguous",value:false},then:[{op:"assign_reference",args:{confidence:0.82}}],else:[{op:"mark_uncertain",args:{confidence:0.45}}]}}
      ]
    };
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"selection_policy",requiredOperations:v522Unique(module.program.map(x=>x.op)),program:module.program,resolvedMissingOperations:[...missing]}};
  }

  if(missing.has("evaluate_arbitrary_relationship_fit")){
    const module={
      name:"Synthesized relationship-fit reference selection",domain:"reference",phase:"reference",priority:68,
      conditions:[{type:"field_exists",field:"mention.surface"}],cognitiveForm:"selection_policy",
      policySpec:{type:"relationship_fit_selection",reason:raw},
      program:[
        {op:"collect_candidates",args:{sources:["local","discourse","speaker","listener"]}},
        {op:"score_candidates",args:{listPath:"candidates",criteria:[{kind:"local",weight:1},{kind:"recent",weight:0.8}]}},
        {op:"score_candidates_by_relationship",args:{listPath:"candidates",relationshipPaths:["workspace.semanticComposition","workspace.interpretation","semanticComposition"],explicitWeight:3,compatibleWeight:1.5,conflictPenalty:2}},
        {op:"select_best_candidate",args:{listPath:"candidates",outputPath:"selectedCandidate"}},
        {op:"mark_ambiguity_by_score",args:{listPath:"candidates",outputPath:"ambiguous",tolerance:0.3,confidence:0.45}},
        {op:"branch",args:{if:{type:"field_equals",field:"ambiguous",value:false},then:[{op:"assign_reference",args:{confidence:0.84}}],else:[{op:"mark_uncertain",args:{confidence:0.45}}]}}
      ]
    };
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"selection_policy",requiredOperations:v522Unique(module.program.map(x=>x.op)),program:module.program,resolvedMissingOperations:[...missing]}};
  }


  if(missing.has("nested_semantic_frame_manipulation")){
    const module={
      name:"Synthesized nested-context preservation",domain:"composition",phase:"composition",priority:88,
      conditions:[{type:"scope_exists",nonRoot:true}],cognitiveForm:"state_transformation",
      categories:["scope","composition","nested context"],requires:["nested or bounded scope"],produces:["nested_semantic_frame_manipulation","preserved parent-child scope relation"],
      policySpec:{type:"nested_context_preservation",reason:raw},
      program:[
        {op:"scope_apply_policy",args:{selector:{nonRoot:true},policy:{preserveParentChildDistinction:true,inheritTaskAuthority:false,bindingLifetime:"scope"}}},
        {op:"scope_enforce_isolation",args:{selector:{nonRoot:true}}}
      ]
    };
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"state_transformation",requiredOperations:["scope_apply_policy","scope_enforce_isolation"],resolvedMissingOperations:["nested_semantic_frame_manipulation"],program:module.program,reason:"composed from the existing generic scope graph and isolation operations"}};
  }

  if(missing.has("cross_phase_precondition_check")){
    const paths=[];if(/\breference/.test(t))paths.push("canonicalSemanticState.references");if(/\bsemantic|meaning|interpret/.test(t))paths.push("canonicalSemanticState.propositions");if(/\brequest|goal/.test(t))paths.push("canonicalSemanticState.unresolved");if(!paths.length)paths.push("canonicalSemanticState");
    const module={
      name:"Synthesized cross-phase precondition policy",domain:rule?.domain||"validation",phase:rule?.domain||"validation",priority:84,
      conditions:[{type:"always"}],cognitiveForm:"validation_policy",categories:["precondition","validation","cross phase"],requires:paths,produces:["cross_phase_precondition_check"],
      policySpec:{type:"cross_phase_precondition",paths,reason:raw},program:[{op:"require_preconditions",args:{paths,nonEmpty:paths.some(path=>/references|propositions|unresolved/.test(path)),outputPath:"moduleState.crossPhasePreconditions"}}]
    };
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"validation_policy",requiredOperations:["require_preconditions"],resolvedMissingOperations:["cross_phase_precondition_check"],program:module.program}};
  }

  if(missing.has("scoped_context_binding")||missing.has("scoped_context_policy")){
    const quoted=/\b(?:quote|quoted|quotation)\b/.test(`${scope} ${t}`),boundaryKind=quoted?"quoted_text":null,selector={nonRoot:true,...(boundaryKind?{boundaryKind}:{})};
    const firstSecond=/\b(?:first-person|first person|second-person|second person|pronouns?|references?)\b/.test(t),participantSignal=firstSecond||/\b(?:participant|speaker|addressee|listener|mapping|outer conversation frame)\b/.test(t),negative=/\b(?:do not|don't|never|without|not automatically|should not|must not)\b/.test(t),boundaryRole=/\b(?:boundary)\b/.test(t)&&/\b(?:do not|does not|not|replace|determine)\b/.test(t),lifetime=/\b(?:after|ends?|outside|leak|keep .* active)\b/.test(t);
    const program=[];
    if(boundaryRole)program.push({op:"scope_apply_policy",args:{selector,policy:{boundaryDefinesRegion:true,roleAuthority:"independent"}}});
    if(participantSignal&&(negative||lifetime))program.push({op:"scope_apply_policy",args:{selector,policy:{inheritParticipantFrame:false,bindingLifetime:"scope"}}},{op:"scope_enforce_isolation",args:{selector}});
    if(firstSecond&&!negative)program.push({op:"scope_resolve_deictics",args:{selector,allowParentFallback:false}});
    if(!program.length)program.push({op:"scope_apply_policy",args:{selector,policy:{participantFrameScoped:true}}});
    const module={name:"Synthesized scoped-context participant policy",domain:"reference",phase:"reference",priority:97,conditions:[{type:"scope_exists",...selector}],cognitiveForm:"state_transformation",categories:["scope","reference","participant frame"],requires:["observed bounded scope"],produces:["scope-local participant policy","scope-local reference resolution"],policySpec:{type:"scoped_context_binding",reason:raw},program};
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"state_transformation",requiredOperations:v522Unique(program.map(x=>x.op)),program,resolvedMissingOperations:[...missing],reason:"composed from generic scope policy, scope isolation, and scoped deictic operations"}};
  }

  if(missing.has("concept_graph_navigation")){
    const program=[
      {op:"knowledge_resolve_concept",args:{fromPath:"rawText",outputPath:"moduleState.focusConcept",minScore:18}},
      {op:"knowledge_collect_relations",args:{conceptPath:"moduleState.focusConcept",depth:2,direction:"both",limit:80,outputPath:"moduleState.focusRelations"}},
      {op:"knowledge_select_relevant_relations",args:{listPath:"moduleState.focusRelations",outputPath:"moduleState.relevantRelations",limit:12}},
      {op:"knowledge_form_supported_proposition",args:{listPath:"moduleState.relevantRelations",outputPath:"moduleState.supportedProposition",commit:true}}
    ];
    const module={name:"Synthesized concept-connection reasoning",domain:"reasoning",phase:"reasoning",priority:79,conditions:[{type:"goal_exists"}],cognitiveForm:"operation_program",categories:["reasoning","concepts","semantic relationships"],requires:["persistent semantic concepts","current cognitive goal"],produces:["concept_graph_navigation","connected semantic relationships","supported proposition"],capabilities:["concept_graph_navigation","supported_proposition_construction"],policySpec:{type:"concept_connection_reasoning",reason:raw},program};
    return {module,diagnostic:{status:"synthesized",cognitiveForm:"operation_program",requiredOperations:program.map(step=>step.op),resolvedMissingOperations:["concept_graph_navigation"],capabilities:["concept_graph_navigation","supported_proposition_construction"],program,reason:"composed from generic concept lookup, relationship traversal, relevance selection, and supported proposition construction"}};
  }
  if(missing.has("structured_role_access")){const program=[{op:"select_labeled_role",args:{itemPath:"unit",outputPath:"moduleState.selectedInstructionRole"}}],module={name:"Synthesized labeled-structure role access",domain:rule?.domain||"learning",phase:rule?.domain||"learning",priority:76,conditions:[{type:"field_exists",field:"unit"}],cognitiveForm:"state_transformation",categories:["structure","role","learning"],requires:["labeled structured item"],produces:["structured_role_access"],program};return {module,diagnostic:{status:"synthesized",requiredOperations:["select_labeled_role"],resolvedMissingOperations:["structured_role_access"],program}};}
  if(missing.has("ordered_structure_preservation")){const program=[{op:"preserve_ordered_items",args:{listPath:"procedure.steps",outputPath:"moduleState.orderedProcedureSteps"}}],module={name:"Synthesized ordered-structure preservation",domain:rule?.domain||"reasoning",phase:rule?.domain||"reasoning",priority:72,conditions:[{type:"field_exists",field:"procedure.steps"}],cognitiveForm:"state_transformation",categories:["structure","sequence"],requires:["ordered items"],produces:["ordered_structure_preservation"],program};return {module,diagnostic:{status:"synthesized",requiredOperations:["preserve_ordered_items"],resolvedMissingOperations:["ordered_structure_preservation"],program}};}
  if(missing.has("support_conflict_evaluation")){const program=[{op:"evaluate_support_conflict",args:{outputPath:"moduleState.supportConflict"}}],module={name:"Synthesized support/conflict evaluation",domain:rule?.domain||"reasoning",phase:rule?.domain||"reasoning",priority:81,conditions:[{type:"always"}],cognitiveForm:"validation_policy",categories:["evidence","support","conflict"],requires:["represented evidence or propositions"],produces:["support_conflict_evaluation"],program};return {module,diagnostic:{status:"synthesized",requiredOperations:["evaluate_support_conflict"],resolvedMissingOperations:["support_conflict_evaluation"],program}};}
  if(missing.has("cognitive_status_inspection")){const program=[{op:"inspect_cognitive_status",args:{outputPath:"moduleState.cognitiveStatus"}}],module={name:"Synthesized cognitive-status inspection",domain:rule?.domain||"learning",phase:rule?.domain||"learning",priority:70,conditions:[{type:"always"}],cognitiveForm:"inspection_policy",categories:["learning","status","reflection"],produces:["cognitive_status_inspection"],program};return {module,diagnostic:{status:"synthesized",requiredOperations:["inspect_cognitive_status"],resolvedMissingOperations:["cognitive_status_inspection"],program}};}
  if(missing.has("source_evidence_access")){const program=[{op:"retrieve_source_evidence",args:{outputPath:"moduleState.sourceEvidence"}}],module={name:"Synthesized source-evidence access",domain:rule?.domain||"interpretation",phase:rule?.domain||"interpretation",priority:74,conditions:[{type:"always"}],cognitiveForm:"state_transformation",categories:["evidence","source"],produces:["source_evidence_access"],program};return {module,diagnostic:{status:"synthesized",requiredOperations:["retrieve_source_evidence"],resolvedMissingOperations:["source_evidence_access"],program}};}

  if(missing.has("unmapped_cognitive_state_transformation")){
    const quotedPronoun=(raw.match(/["“'‘]\s*(I|me|my|mine|you|your|yours|we|us|our|ours|he|him|his|she|her|hers|they|them|their|theirs|it|its|this|that|these|those)\s*["”'’]/i)||[])[1];
    const pronoun=v522ScopePronoun(scope)||v522ScopePronoun(quotedPronoun||"");

    // Negative pronoun guidance can become an enforceable contextual-evidence
    // policy without Nova pretending the lesson supplied a positive referent.
    if(pronoun && /\b(?:not|do not|don't|should not|must not|never)\b/.test(t) && /\b(?:word itself|surface|alone|only from|without context)\b/.test(t)){
      const module={
        name:"Synthesized contextual-reference evidence policy",domain:"reference",phase:"reference",priority:93,
        conditions:[{type:"surface_in",values:[pronoun]}],cognitiveForm:"invariant",
        policySpec:{type:"context_required_for_reference",surface:pronoun,reason:raw},
        program:[{op:"require_contextual_resolution",args:{confidence:0.45}},{op:"declare_invariant",args:{type:"context_required_for_reference",paths:["mention.resolutionSource"],enforcement:"observe",reason:raw}}]
      };
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"invariant",requiredOperations:["require_contextual_resolution","declare_invariant"],program:module.program,resolvedMissingOperations:[...missing]}};
    }

    // Relative discourse words are resolved against the conversation timeline,
    // not the response currently being generated. The operation itself is
    // generic over relation/role/offset.
    if(/\b(?:previous|preceding|earlier|last|most recent)\b/.test(`${scope} ${t}`) && /\b(?:response|message|turn|statement)\b/.test(t)){
      const role=/\b(?:response|assistant)\b/.test(t)?"assistant":null;
      const relation=/\b(?:previous|preceding|earlier|last|most recent)\b/.exec(`${scope} ${t}`)?.[0]||"previous";
      const module={
        name:"Synthesized relative-discourse selection",domain:"interpretation",phase:"interpretation",priority:82,
        conditions:[{type:"contains_any",values:["previous","preceding","earlier","last","most recent"]}],cognitiveForm:"state_transformation",
        policySpec:{type:"relative_discourse_selection",relation,role,reason:raw},
        program:[{op:"resolve_relative_discourse",args:{relation,role,offset:1,outputPath:"relativeDiscourse.selected"}}]
      };
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"state_transformation",requiredOperations:["resolve_relative_discourse"],program:module.program,resolvedMissingOperations:[...missing]}};
    }

    // Perspective consistency is already representable by Nova's existing
    // perspective-aware realization operation; the old synthesizer simply did
    // not connect this form of teaching to it.
    if(/\b(?:first-person|first person|perspective|viewpoint)\b/.test(`${scope} ${t}`) && /\b(?:consistent|consistency|herself|himself|itself|speaks about)\b/.test(t)){
      const module={
        name:"Synthesized perspective-consistency policy",domain:"realization",phase:"realization",priority:91,
        conditions:[{type:"field_exists",field:"claim.qualifiers.semanticOwnerId"}],cognitiveForm:"realization_policy",
        policySpec:{type:"perspective_consistency",reason:raw},
        program:[{op:"perspective_subject"},{op:"declare_invariant",args:{type:"perspective_consistency",paths:["claim.qualifiers.semanticOwnerId","surfaceSubject"],enforcement:"observe",reason:raw}}]
      };
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"realization_policy",requiredOperations:["perspective_subject","declare_invariant"],program:module.program,resolvedMissingOperations:[...missing]}};
    }

  }

  return null;
}

function v522AdaptiveSynthesisStrategy(rule,current){
  if(current?.module||current?.diagnostic?.status==="teaching_artifact")return current;
  if(current?.diagnostic?.status==="missing_fundamental_capability")return v522AdaptiveModule(rule,current)||current;
  return current;
}
if(typeof v520RegisterSynthesisStrategy==="function")v520RegisterSynthesisStrategy("adaptive generic synthesis",80,v522AdaptiveSynthesisStrategy);

/* ---------------------------------------------------------
   VERSION / SELF-AWARENESS / MIGRATION
   --------------------------------------------------------- */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA522_VERSION;
    V41_SELF_MODEL.capabilities ||= [];
    if(!V41_SELF_MODEL.capabilities.some(c=>c.id==="adaptive_cognitive_synthesis"))V41_SELF_MODEL.capabilities.push({
      id:"adaptive_cognitive_synthesis",verb:"compose",object:"learned cognition from general structured-state operations",detail:"When older synthesis reports a broad missing capability, I can attempt a deeper program decomposition using reusable discourse, support, relationship, and state operations instead of requiring a lesson-shaped JavaScript function."
    });
  }
}catch(_){ }

/* Current-core version ownership lives in the V6.2 core. Historical
   capability/workspace functions are not replaced or wrapped here. */

function v522RunAdaptiveMigration(){
  const pass=typeof v520GeneralSynthesisPass==="function"?v520GeneralSynthesisPass({reason:"v5.2.2_adaptive_synthesis_migration",force:true}):null;
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;
    if(memory)memory.adaptiveSynthesis={version:NOVA522_ADAPTIVE_SYNTHESIS_SCHEMA,appVersion:NOVA522_VERSION,lastSynthesis:pass,updatedAt:Date.now()};
  }catch(_){ }
  try{saveState();}catch(_){ }
  return pass;
}

try{if(!window.__TAURI__?.core?.invoke)v522RunAdaptiveMigration();}catch(error){console.warn("Browser adaptive synthesis migration failed:",error);}
