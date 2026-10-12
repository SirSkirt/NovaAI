"use strict";

/* =========================================================
   Nova V6.2.16 — GENERAL EPISTEMIC / CONTROL-FLOW SYNTHESIS

   This strategy set does NOT encode the answers from any reasoning curriculum.
   It expands Nova's cognitive CPU and program synthesizer so lessons can teach
   general reasoning policies using structured state, evidence relationships,
   condition/action control flow, unresolved goals, attempts, belief revision,
   and stopping conditions.

   Learned cognition remains declarative data executed by the safe cognitive VM.
   No eval/Function/filesystem/network/app-control capability is introduced here.
   ========================================================= */

const NOVA530_VERSION="6.2.16";
const NOVA530_EPISTEMIC_SCHEMA=2;

function v530Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(_){return v;}}}
function v530Canonical(v){try{return canonical(String(v??""));}catch(_){return String(v??"").trim().toLowerCase();}}
function v530Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v530Unique(xs){const out=[],seen=new Set();for(const x of xs||[]){const k=typeof x==="object"?JSON.stringify(x):v530Canonical(x);if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}
function v530Read(ctx,path){return typeof v502GetPath==="function"?v502GetPath(ctx,path):undefined;}
function v530Write(ctx,path,value){return typeof v502SetPath==="function"?v502SetPath(ctx,path,value):false;}
function v530Array(v){return Array.isArray(v)?v:(v==null?[]:[v]);}
function v530State(ctx){return ctx?.cognitiveState||ctx?.canonicalSemanticState||ctx?.workspace?.cognitiveState||ctx?.workspace?.integratedCognition||ctx?.integratedCognition||null;}
function v530EpistemicRoot(ctx){
  const state530=v530State(ctx);
  if(state530){
    state530.epistemic ||= {propositions:{},evidenceRelations:[],beliefs:{},attempts:[],reasoningPolicies:[],constraintPolicies:[],stopPolicies:[],updatedAt:Date.now()};
    state530.epistemic.constraintPolicies ||= [];
    return state530.epistemic;
  }
  ctx.epistemic ||= {propositions:{},evidenceRelations:[],beliefs:{},attempts:[],reasoningPolicies:[],constraintPolicies:[],stopPolicies:[],updatedAt:Date.now()};
  ctx.epistemic.constraintPolicies ||= [];
  return ctx.epistemic;
}
function v530PropositionKey(value){
  if(value&&typeof value==="object")value=value.id??value.text??value.claim??value.proposition??value.target??JSON.stringify(value);
  const key=v530Canonical(value||"current_proposition").replace(/[^a-z0-9]+/g,"_").replace(/^_+|_+$/g,"").slice(0,120);
  return key||"current_proposition";
}
function v530CurrentProposition(ctx,args={}){
  const explicit=args.proposition??(args.propositionPath?v530Read(ctx,args.propositionPath):undefined);
  if(explicit!=null)return explicit;
  return v530Read(ctx,"evaluation.claim.text")??v530Read(ctx,"semanticComposition.evaluation.claim.text")??v530Read(ctx,"workspace.semanticComposition.evaluation.claim.text")??v530Read(ctx,"workspace.requestFrame.explicitTarget")??"current_proposition";
}
function v530CurrentEvidence(ctx,args={}){
  const explicit=args.evidence??(args.evidencePath?v530Read(ctx,args.evidencePath):undefined);
  if(explicit!=null)return v530Array(explicit);
  return v530Array(v530Read(ctx,"evaluation.evidence")??v530Read(ctx,"semanticComposition.evaluation.evidence")??v530Read(ctx,"workspace.semanticComposition.evaluation.evidence")??[]);
}
function v530PolicyBucket(ctx){
  ctx.learnedPolicies ||= {};
  ctx.learnedPolicies.reasoning ||= [];
  const state530=v530State(ctx);
  if(state530){state530.learnedPolicies ||= {};state530.learnedPolicies.reasoning ||= [];}
  return {ctx:ctx.learnedPolicies.reasoning,state:state530?.learnedPolicies?.reasoning||null};
}
function v530PushPolicy(ctx,policy){
  const prepared={id:policy.id||`rp_${Math.random().toString(36).slice(2,8)}`,...v530Clone(policy),registeredAt:Date.now()};
  const buckets=v530PolicyBucket(ctx);
  if(!buckets.ctx.some(x=>x.id===prepared.id||JSON.stringify(x.spec)===JSON.stringify(prepared.spec)))buckets.ctx.push(prepared);
  if(buckets.state&&!buckets.state.some(x=>x.id===prepared.id||JSON.stringify(x.spec)===JSON.stringify(prepared.spec)))buckets.state.push(v530Clone(prepared));
  const root=v530EpistemicRoot(ctx);if(!root.reasoningPolicies.some(x=>x.id===prepared.id||JSON.stringify(x.spec)===JSON.stringify(prepared.spec)))root.reasoningPolicies.push(v530Clone(prepared));
  root.updatedAt=Date.now();return prepared;
}

/* ---------------------------------------------------------
   GENERAL VM OPERATIONS

   These operations register with the V6.2 cognitive kernel. They extend the
   current mind's instruction set; they do not replace an older executor.
   --------------------------------------------------------- */
if(typeof v502RegisterOperation==="function"){
  v502RegisterOperation("ensure_state_container",(args,ctx)=>{const path=args.path||"cognitiveState";return v530Read(ctx,path)==null?v530Write(ctx,path,args.kind==="list"?[]:{}):false;});
  v502RegisterOperation("declare_reasoning_policy",(args,ctx)=>{const before=v530EpistemicRoot(ctx).reasoningPolicies.length;v530PushPolicy(ctx,{type:args.type||"conditional_policy",form:args.form||null,spec:v530Clone(args.spec||{}),reason:args.reason||null,source:args.source||null});return v530EpistemicRoot(ctx).reasoningPolicies.length!==before;});
  v502RegisterOperation("register_cognitive_constraint",(args,ctx)=>{
    const root=v530EpistemicRoot(ctx),directive=args.directive||args.reason||"",target=args.target||"cognition";
    if((root.constraintPolicies||[]).some(x=>x.target===target&&v530Canonical(x.directive)===v530Canonical(directive)))return false;
    const constraint={id:args.id||`constraint_${Math.random().toString(36).slice(2,8)}`,target,directive,scope:args.scope||"general",polarity:args.polarity||"negative",phase:ctx?.phase||args.phase||null,source:args.source||null,at:Date.now()};
    root.constraintPolicies ||= [];root.constraintPolicies.push(v530Clone(constraint));ctx.constraints ||= [];if(!ctx.constraints.some(x=>v530Canonical(x.directive||x.reason)===v530Canonical(directive)))ctx.constraints.push(v530Clone(constraint));
    const state530=v530State(ctx);if(state530){state530.learnedConstraints ||= [];if(!state530.learnedConstraints.some(x=>v530Canonical(x.directive)===v530Canonical(directive)))state530.learnedConstraints.push(v530Clone(constraint));}return true;
  });
  v502RegisterOperation("mark_epistemic_state",(args,ctx)=>{
    const root=v530EpistemicRoot(ctx),prop=v530CurrentProposition(ctx,args),key=v530PropositionKey(prop),prior=root.propositions[key]||{id:key,proposition:v530Clone(prop),history:[]},status=args.statusFromPath?v530Read(ctx,args.statusFromPath):args.status||prior.status||"uncertain";
    const changed=prior.status!==status;prior.status=status;if(changed||!prior.history.length)prior.history.push({status,reason:args.reason||null,at:Date.now()});prior.updatedAt=Date.now();root.propositions[key]=prior;root.updatedAt=Date.now();if(args.outputPath)return v530Write(ctx,args.outputPath,v530Clone(prior))||changed;return changed;
  });
  v502RegisterOperation("register_unresolved_goal",(args,ctx)=>{
    const state530=v530State(ctx),label=args.label||(args.labelFromPath&&v530Read(ctx,args.labelFromPath))||String(v530CurrentProposition(ctx,args)),root=v530EpistemicRoot(ctx);let changed=false;
    if(state530){state530.unresolved ||= [];if(!state530.unresolved.some(x=>v530Canonical(x.label??x)===v530Canonical(label))){state530.unresolved.push({label,source:"learned_reasoning_policy",reason:args.reason||null});changed=true;}}
    root.unresolved ||= [];if(!root.unresolved.some(x=>v530Canonical(x.label)===v530Canonical(label))){root.unresolved.push({label,reason:args.reason||null,at:Date.now()});changed=true;}return changed;
  });
  v502RegisterOperation("resolve_unresolved_goal",(args,ctx)=>{
    const state530=v530State(ctx),label=args.label||(args.labelFromPath&&v530Read(ctx,args.labelFromPath))||String(v530CurrentProposition(ctx,args)),root=v530EpistemicRoot(ctx);let changed=false;
    if(state530?.unresolved){const before=state530.unresolved.length;state530.unresolved=state530.unresolved.filter(x=>v530Canonical(x.label??x)!==v530Canonical(label));changed=state530.unresolved.length!==before||changed;}
    const before=(root.unresolved||[]).length;root.unresolved=(root.unresolved||[]).filter(x=>v530Canonical(x.label)!==v530Canonical(label));return root.unresolved.length!==before||changed;
  });
  v502RegisterOperation("register_evidence_relation",(args,ctx)=>{
    const root=v530EpistemicRoot(ctx),prop=v530CurrentProposition(ctx,args),key=v530PropositionKey(prop),items=v530CurrentEvidence(ctx,args);let changed=false;
    for(const evidence of items){const signature=JSON.stringify({key,evidence,relation:args.relation||"supports",strength:args.strength??null,scope:args.scope??null});if(root.evidenceRelations.some(x=>JSON.stringify({key:x.propositionId,evidence:x.evidence,relation:x.relation,strength:x.strength??null,scope:x.scope??null})===signature))continue;root.evidenceRelations.push({id:`er_${Math.random().toString(36).slice(2,8)}`,propositionId:key,proposition:v530Clone(prop),evidence:v530Clone(evidence),relation:args.relation||"supports",strength:args.strength??null,scope:args.scope??null,source:args.source||null,at:Date.now()});changed=true;}
    root.evidenceRelations=root.evidenceRelations.slice(-500);root.updatedAt=Date.now();return changed;
  });
  v502RegisterOperation("summarize_evidence_support",(args,ctx)=>{const root=v530EpistemicRoot(ctx),prop=v530CurrentProposition(ctx,args),key=v530PropositionKey(prop);let relations=root.evidenceRelations.filter(x=>x.propositionId===key);if(!relations.length&&v530CurrentEvidence(ctx,args).length)relations=v530CurrentEvidence(ctx,args).map(e=>({relation:"available",evidence:e,strength:null,scope:null}));const summary={propositionId:key,count:relations.length,supporting:relations.filter(x=>x.relation==="supports").length,contradicting:relations.filter(x=>x.relation==="contradicts").length,available:relations.length>0,relations:v530Clone(relations)};return v530Write(ctx,args.outputPath||"epistemicSupport",summary);});
  v502RegisterOperation("update_belief_state",(args,ctx)=>{const root=v530EpistemicRoot(ctx),prop=v530CurrentProposition(ctx,args),key=v530PropositionKey(prop),prior=root.beliefs[key]||{id:key,proposition:v530Clone(prop),history:[]};let status=args.statusFromPath?v530Read(ctx,args.statusFromPath):args.status;if(!status){const summary=v530Read(ctx,args.supportPath||"epistemicSupport");if(summary?.supporting>0&&summary?.contradicting===0)status="supported";else if(summary?.contradicting>0&&summary?.supporting===0)status="contradicted";else status="uncertain";}const changed=prior.status!==status;prior.status=status;prior.support=v530Clone(v530Read(ctx,args.supportPath||"epistemicSupport")||null);if(changed||!prior.history.length)prior.history.push({status,reason:args.reason||null,at:Date.now()});prior.updatedAt=Date.now();root.beliefs[key]=prior;root.updatedAt=Date.now();return (args.outputPath?v530Write(ctx,args.outputPath,v530Clone(prior)):false)||changed;});
  v502RegisterOperation("record_cognitive_attempt",(args,ctx)=>{const root=v530EpistemicRoot(ctx),attempt={goal:args.goalPath?v530Read(ctx,args.goalPath):args.goal||v530State(ctx)?.currentGoal||null,method:args.methodPath?v530Read(ctx,args.methodPath):args.method||null,result:args.resultPath?v530Read(ctx,args.resultPath):args.result||null,stateSignature:args.statePath?JSON.stringify(v530Read(ctx,args.statePath)):null};const last=root.attempts.at(-1);if(last&&JSON.stringify({goal:last.goal,method:last.method,result:last.result,stateSignature:last.stateSignature})===JSON.stringify(attempt))return false;const stored={id:`attempt_${Math.random().toString(36).slice(2,8)}`,...v530Clone(attempt),at:Date.now()};root.attempts.push(stored);root.attempts=root.attempts.slice(-100);return args.outputPath?v530Write(ctx,args.outputPath,stored)||true:true;});
  v502RegisterOperation("compare_attempt_progress",(args,ctx)=>{const root=v530EpistemicRoot(ctx),attempts=root.attempts||[],latest=attempts.at(-1)||null,previous=attempts.at(-2)||null,sameMethod=Boolean(latest&&previous&&v530Canonical(latest.method)===v530Canonical(previous.method)),changed=Boolean(latest&&previous&&JSON.stringify(latest.result)!==JSON.stringify(previous.result)),summary={sameMethod,changed,repeatWithoutNewProgress:sameMethod&&!changed,count:attempts.length};return v530Write(ctx,args.outputPath||"attemptProgress",summary);});
  v502RegisterOperation("select_information_action",(args,ctx)=>{const state530=v530State(ctx),registry=state530?.operationRegistry||null,wanted=new Set(v530Array(args.faculties||args.faculty).map(v530Canonical)),forbidden=new Set(v530Array(args.exclude||[]).map(v530Canonical));let operations=(registry?.operations||[]).filter(x=>x.executable!==false);if(wanted.size)operations=operations.filter(x=>wanted.has(v530Canonical(x.faculty)));operations=operations.filter(x=>!forbidden.has(v530Canonical(x.id)));const candidates=operations.map(x=>({id:x.id,faculty:x.faculty,kind:x.kind,learned:Boolean(x.learned),description:x.description||x.name||""}));return v530Write(ctx,args.outputPath||"informationActions",candidates);});
  v502RegisterOperation("declare_stop_condition",(args,ctx)=>{const root=v530EpistemicRoot(ctx),candidate={type:args.type||"stop_condition",when:v530Clone(args.when||null),unless:v530Clone(args.unless||null),reason:args.reason||null};if(root.stopPolicies.some(x=>JSON.stringify({type:x.type,when:x.when,unless:x.unless,reason:x.reason})===JSON.stringify(candidate)))return false;const policy={...candidate,at:Date.now()};root.stopPolicies.push(policy);root.stopPolicies=root.stopPolicies.slice(-100);v530PushPolicy(ctx,{type:"stop_condition",spec:policy,reason:args.reason||null});return true;});
}

/* Predicate vocabulary is extensible through the same condition registry. */
if(typeof v502RegisterCondition==="function"){
  v502RegisterCondition("unresolved_exists",(test,ctx)=>{const state530=v530State(ctx),root=v530EpistemicRoot(ctx);return Boolean(state530?.unresolved?.length||root?.unresolved?.length);});
  v502RegisterCondition("epistemic_state_is",(test,ctx)=>{const root=v530EpistemicRoot(ctx),prop=v530CurrentProposition(ctx,test),key=v530PropositionKey(prop),status=root.propositions[key]?.status||root.beliefs[key]?.status;return v530Canonical(status)===v530Canonical(test.value);});
  v502RegisterCondition("evidence_available",(test,ctx)=>v530CurrentEvidence(ctx,test).length>0||v530EpistemicRoot(ctx).evidenceRelations.length>0);
  v502RegisterCondition("operation_available",(test,ctx)=>{const registry=v530State(ctx)?.operationRegistry||null;return (registry?.operations||[]).some(op=>(!test.faculty||v530Canonical(op.faculty)===v530Canonical(test.faculty))&&(!test.operationId||op.id===test.operationId)&&op.executable!==false);});
  v502RegisterCondition("attempt_repeated_without_progress",(_,ctx)=>{const a=v530EpistemicRoot(ctx).attempts||[],x=a.at(-1),y=a.at(-2);return Boolean(x&&y&&v530Canonical(x.method)===v530Canonical(y.method)&&JSON.stringify(x.result)===JSON.stringify(y.result));});
}

/* ---------------------------------------------------------
   GENERAL TEACHING ROLE HYGIENE
   --------------------------------------------------------- */
function v530LooksCurriculumMetadata(text,meta={}){
  const t=v530Canonical(text),role=v530Canonical(meta.role||meta.evidenceRole||"");
  if(["purpose","subject","topic","end"].includes(role))return true;
  if(/^this lesson (?:teaches|covers|introduces|is about)\b/.test(t))return true;
  if(/^the (?:purpose|goal|objective) (?:of this lesson )?is\b/.test(t))return true;
  if(/^lesson\s+\d+(?:\.\d+)?\s+is\s+complete\s+when\b/.test(t))return true;
  if(/^this lesson should apply beyond\b/.test(t))return true;
  return false;
}
function v530LooksConceptDefinition(text,meta={}){
  const t=v530Canonical(text),role=v530Canonical(meta.role||meta.evidenceRole||"");
  if(!["definition","relationship","relationships"].includes(role))return false;
  return !/\b(?:should|must|do not|don't|never|avoid|prefer|required)\b/.test(t);
}
/* ---------------------------------------------------------
   GENERAL EPISTEMIC POLICY SYNTHESIS

   This recognizes broad semantic roles (evidence, uncertainty, belief,
   attempts, alternatives, stopping, clarification) and compiles them into a
   declarative policy. It does not encode any particular lesson answer.
   --------------------------------------------------------- */
function v530EpistemicSignals(rule){
  const raw=v530Clean(rule?.directive||""),t=v530Canonical(raw),scopeText=v530Canonical(rule?.scope||""),domain=v530Canonical(rule?.domain||"general"),all=`${scopeText} ${t}`;
  const has=(rx)=>rx.test(all);
  return {
    raw,t,scopeText,domain,
    epistemic:has(/\b(?:evidence|uncertain|uncertainty|known|unknown|knowable|support|conclusion|inference|belief|observation|testable|testability|attempt|failed|failure|retry|method|alternative|contradiction|clarification|guess|reasoning loop|reasoning|epistemic|capability|stopping|stop)\b/),
    condition:/\b(?:if|when|unless|while|after|before)\b/.test(t),
    evidence:has(/\bevidence|observation|observed|support(?:ed|s|ing)?|contradict(?:s|ed|ion)?\b/),
    uncertainty:has(/\buncertain|uncertainty|unknown|do not know|don't know|insufficient\b/),
    belief:has(/\bbelief|conclusion|update|revise|reconsider|correction\b/),
    seek:/\b(?:obtain|acquire|seek|gather|retrieve|ask|observe|test)\b.{0,35}\b(?:evidence|information|answer|result)?\b|\bevidence-seeking\b/.test(t),
    attempt:/\b(?:attempt|retry|method|failed|failure)\b/.test(t),
    alternatives:/\b(?:alternative|explanation|interpretation|plausible)\b/.test(t),
    stop:/\b(?:stop|stopping condition|do not continue|don't continue|no reasonable|indefinitely)\b/.test(t),
    directness:has(/\bdirect evidence|indirect evidence|direct observation\b/),
    scopeBreadth:has(/\blocal|general|broader|unlimited|permanent|always|every|all|specific case|one successful|repeated\b/),
    capability:has(/\bcapabilit(?:y|ies)|available mechanisms?|available operations?\b/),
    clarification:/\b(?:clarification|clarify|asking)\b/.test(t),
    negative:/\b(?:do not|don't|should not|must not|never|avoid|not automatically|not always)\b/.test(t),
    normative:/\b(?:should|must|do not|don't|never|avoid|prefer|if|when|unless|may be useful|can be useful|useful when)\b/.test(t)
  };
}
function v530PolicySpec(rule,s){
  const effects=[];const conditions=[];
  if(s.uncertainty){conditions.push({type:"epistemic_or_information_state",value:"uncertain_or_insufficient"});if(/\b(?:mark|remain|preserve|keep)\b/.test(s.t)||s.negative)effects.push({type:"preserve_uncertainty"});}
  if(s.evidence){conditions.push({type:"evidence_context"});if(/\b(?:update|revise|reconsider|change)\b/.test(s.t))effects.push({type:"revise_belief_from_evidence"});if(/\b(?:stronger|claim more|more than|general|local|scope)\b/.test(s.t)||s.scopeBreadth)effects.push({type:"bound_conclusion_by_evidence"});}
  if(s.scopeBreadth&&s.negative&&!effects.some(x=>x.type==="bound_conclusion_by_evidence"))effects.push({type:"bound_conclusion_by_evidence"});
  if(s.seek)effects.push({type:"seek_relevant_information"});
  if(s.attempt){conditions.push({type:"attempt_context"});if(/\b(?:do not repeat|don't repeat|indefinitely)\b/.test(s.t))effects.push({type:"avoid_repeating_unchanged_failed_attempt"});else effects.push({type:"record_and_reconsider_attempt"});}
  if(s.alternatives)effects.push({type:"compare_plausible_alternatives"});
  if(s.stop)effects.push({type:"stop_when_resolved_or_no_progress"});
  if(s.clarification)effects.push({type:"clarify_only_for_material_ambiguity"});
  if(s.capability)effects.push({type:"inspect_available_cognition_or_safe_evidence_route"});
  if(/\b(?:unknown answer may remain unknown|remain unknown|remain uncertain)\b/.test(s.t))effects.push({type:"allow_unresolved_epistemic_state"});
  if(/\b(?:final response|answer after reasoning|communicate the conclusion)\b/.test(s.t))effects.push({type:"realize_conclusion_not_internal_trace"});
  return {kind:"epistemic_control_policy",conditions:v530Unique(conditions),effects:v530Unique(effects),sourceText:s.raw,scope:rule?.scope||"general"};
}
function v530ProgramForPolicy(rule,s,spec){
  const program=[{op:"declare_reasoning_policy",args:{type:"epistemic_control_policy",form:typeof v521ClassifyCognitiveForm==="function"?v521ClassifyCognitiveForm(rule):"operation_program",spec,reason:s.raw}}];
  for(const effect of spec.effects||[]){
    if(effect.type==="preserve_uncertainty")program.push({op:"mark_epistemic_state",args:{status:"uncertain",reason:s.raw}});
    else if(effect.type==="seek_relevant_information")program.push({op:"select_information_action",args:{faculties:["memory","reasoning","self","context"],outputPath:"epistemic.availableInformationActions"}});
    else if(effect.type==="revise_belief_from_evidence")program.push({op:"summarize_evidence_support",args:{outputPath:"epistemicSupport"}},{op:"update_belief_state",args:{supportPath:"epistemicSupport",reason:s.raw}});
    else if(effect.type==="avoid_repeating_unchanged_failed_attempt")program.push({op:"compare_attempt_progress",args:{outputPath:"attemptProgress"}},{op:"declare_stop_condition",args:{type:"avoid_unchanged_repeat",when:{type:"attempt_repeated_without_progress"},reason:s.raw}});
    else if(effect.type==="record_and_reconsider_attempt")program.push({op:"record_cognitive_attempt",args:{goalPath:"workspace.integratedCognition.currentGoal",resultPath:"result"}});
    else if(effect.type==="stop_when_resolved_or_no_progress")program.push({op:"declare_stop_condition",args:{type:"resolved_or_no_progress",reason:s.raw}});
    else if(effect.type==="bound_conclusion_by_evidence")program.push({op:"summarize_evidence_support",args:{outputPath:"epistemicSupport"}});
    else if(effect.type==="inspect_available_cognition_or_safe_evidence_route")program.push({op:"select_information_action",args:{outputPath:"epistemic.availableCapabilityEvidenceRoutes"}});
  }
  return program;
}
function v530CanSynthesizeEpistemic(rule,base){
  const s=v530EpistemicSignals(rule);if(!s.epistemic)return null;
  // Pure descriptive definitions/metadata remain semantic knowledge, not
  // executable cognition. Normative guidance and procedural directives qualify.
  if(!s.normative && !["reasoning","interpretation","validation","memory"].includes(s.domain) && !/\b(?:identify|check|determine|compare|list|retrieve|update|answer|choose|interpret|separate|ask|use|stop)\b/.test(s.t))return null;
  const spec=v530PolicySpec(rule,s);if(!(spec.effects||[]).length)return null;
  const form=typeof v521ClassifyCognitiveForm==="function"?v521ClassifyCognitiveForm(rule):"operation_program";
  const phase=s.clarification&&rule.domain==="interpretation"?"interpretation":"reasoning";
  const program=v530ProgramForPolicy(rule,s,spec);
  const module={name:"Synthesized general epistemic control policy",domain:phase==="interpretation"?"interpretation":"reasoning",phase,priority:73,conditions:[{type:"always"}],cognitiveForm:form,policySpec:spec,program};
  return {module,diagnostic:{status:"synthesized",cognitiveForm:form,requiredOperations:v530Unique(program.map(x=>x.op)),program,resolvedMissingOperations:base?.diagnostic?.missingOperations||[]}};
}

function v530ConstraintTarget(rule){
  const t=v530Canonical(`${rule?.scope||""} ${rule?.directive||""}`),d=v530Canonical(rule?.domain||"");
  if(d==="realization"||/\b(?:final response|response wording|wording|direct answer|internal machinery)\b/.test(t))return "realization";
  if(/\b(?:action|test|method|modify|authorization|external|internal|simulated|dangerous|safe)\b/.test(t))return "candidate_action";
  if(d==="memory")return "memory";if(d==="reference")return "reference";if(d==="interpretation")return "interpretation";if(d==="validation")return "validation";return "reasoning";
}
function v530SynthesizeNormativeConstraint(rule,base){
  const role=v530Canonical(rule?.evidenceRole||""),t=v530Canonical(rule?.directive||""),negative=rule?.polarity==="negative"||/\b(?:do not|don't|never|avoid|must not|should not)\b/.test(t);
  if(role!=="warning"&&!negative)return null;
  const phase=v530ConstraintTarget(rule)==="realization"?"realization":(rule?.domain&&["reference","interpretation","memory","validation","behavior","learning"].includes(rule.domain)?rule.domain:"reasoning");
  const target=v530ConstraintTarget(rule);
  const form=target==="realization"?"realization_policy":"invariant";
  const program=[{op:"register_cognitive_constraint",args:{target,directive:rule.directive,scope:rule.scope||"general",polarity:negative?"negative":"positive",source:rule.sourceName||null}}];
  const module={name:"Synthesized learned cognitive constraint",domain:phase,phase,priority:72,conditions:[{type:"always"}],cognitiveForm:form,policySpec:{kind:"cognitive_constraint",target,directive:rule.directive,scope:rule.scope||"general"},program};
  return {module,diagnostic:{status:"synthesized",cognitiveForm:form,requiredOperations:["register_cognitive_constraint"],program,resolvedMissingOperations:base?.diagnostic?.missingOperations||[]}};
}

function v530EpistemicSynthesisStrategy(rule,current){
  if(current?.module||current?.diagnostic?.status==="teaching_artifact")return current;
  return v530CanSynthesizeEpistemic(rule,current)||v530SynthesizeNormativeConstraint(rule,current)||current;
}
if(typeof v520RegisterSynthesisStrategy==="function")v520RegisterSynthesisStrategy("general epistemic synthesis",70,v530EpistemicSynthesisStrategy);

/* Learned epistemic programs execute directly in the current V6.2 module
   kernel; no legacy working-state/executive function is replaced here. */

/* Version/self model. */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA530_VERSION;V41_SELF_MODEL.capabilities ||= [];
    if(!V41_SELF_MODEL.capabilities.some(c=>c.id==="general_epistemic_program_synthesis"))V41_SELF_MODEL.capabilities.push({id:"general_epistemic_program_synthesis",verb:"synthesize",object:"reasoning policies from generic cognitive state and control-flow operations",detail:"I can learn condition/action reasoning policies involving uncertainty, evidence, belief revision, unresolved goals, attempts, information seeking, and stopping conditions without requiring a lesson-shaped JavaScript function."});
  }
}catch(_){ }

/* V6.2 current-system startup reconsideration. This calls the current synthesizer
   directly; it does not patch any learning or executive function. */
function v530PostLoadEpistemicMaintenance(options={}){
  if(window.__TAURI__?.core?.invoke)return {skipped:true,reason:"rust_v7_pass_8_synthesis_authority"};
  let pass=null;try{if(typeof v520GeneralSynthesisPass==="function")pass=v520GeneralSynthesisPass({reason:"v6.2_epistemic_reconsideration",force:Boolean(options.force)});}catch(error){pass={error:String(error?.message||error)};}
  try{const memory=v43Memory();memory.generalEpistemicSynthesis={schema:NOVA530_EPISTEMIC_SCHEMA,version:NOVA530_VERSION,lastSynthesis:v530Clone(pass),updatedAt:Date.now()};}catch(_){ }
  return pass;
}

console.info("Nova V6.2.16 general epistemic synthesis loaded.");
