"use strict";

/* =========================================================
   Nova V6.2.16 — GENERAL TEACHING SYNTHESIS FOUNDATIONS

   Purpose:
   - expand the safe cognitive instruction set with general mechanisms for
     predicates, state transitions, preference/fallback policies, task resource
     policies, fidelity policies, and representation-shape invariants;
   - let lessons define how cognition works without encoding curriculum answers
     in JavaScript;
   - distinguish operational teaching from curriculum metadata;
   - rebuild already-learned source-backed cognition once after upgrade so old
     Stored Only guidance can benefit from the current synthesizer without the
     lesson being taught again.

   Learned cognition remains declarative data. This file does not add eval,
   generated JavaScript, filesystem, network, application control, or permission
   operations to Nova's cognitive VM.
   ========================================================= */

const NOVA533_VERSION="6.2.16";
const NOVA533_SYNTHESIS_SCHEMA=2;
const NOVA533_MIGRATION_SCHEMA=2;

function v533Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(_){return v;}}}
function v533Canonical(v){try{return canonical(String(v??""));}catch(_){return String(v??"").trim().toLowerCase();}}
function v533Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v533Unique(xs){const out=[],seen=new Set();for(const x of xs||[]){const k=typeof x==="object"?JSON.stringify(x):v533Canonical(x);if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}
function v533Read(ctx,path){return typeof v502GetPath==="function"?v502GetPath(ctx,path):undefined;}
function v533Write(ctx,path,value){return typeof v502SetPath==="function"?v502SetPath(ctx,path,value):false;}
function v533State(ctx){return ctx?.cognitiveState||ctx?.canonicalSemanticState||ctx?.workspace?.cognitiveState||ctx?.workspace?.integratedCognition||ctx?.integratedCognition||null;}
function v533PolicyRoot(ctx){
  const state533=v533State(ctx);
  if(state533){
    state533.learnedSynthesis ||= {};
    const root=state533.learnedSynthesis;
    root.predicates ||= [];root.transitions ||= [];root.preferences ||= [];root.resourcePolicies ||= [];root.fidelityPolicies ||= [];root.representationInvariants ||= [];root.evaluations ||= [];
    return root;
  }
  ctx.learnedSynthesis ||= {predicates:[],transitions:[],preferences:[],resourcePolicies:[],fidelityPolicies:[],representationInvariants:[],evaluations:[]};
  return ctx.learnedSynthesis;
}
function v533PushUnique(list,item,keys=[]){
  if(!Array.isArray(list))return item;
  const sig=(x)=>keys.length?keys.map(k=>v533Canonical(x?.[k])).join("|"):v533Canonical(JSON.stringify(x||{}));
  const s=sig(item);const existing=list.find(x=>sig(x)===s);if(existing)return existing;list.push(v533Clone(item));return item;
}
function v533TextAtoms(text){
  const stop=new Set(["a","an","the","is","are","was","were","be","been","being","to","of","for","from","with","when","if","then","that","this","it","and","or","not","may","can","should","must","will","would"]);
  return v533Unique(v533Canonical(text).split(/[^a-z0-9_]+/).filter(x=>x.length>2&&!stop.has(x))).slice(0,24);
}
function v533ScopeName(rule,fallback="learned_property"){
  const s=v533Canonical(rule?.scope||"").replace(/[^a-z0-9]+/g,"_").replace(/^_+|_+$/g,"");
  return s&&s!=="general"?s:fallback;
}
function v533DeclarativePredicateSpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);const scope=v533ScopeName(rule,"learned_predicate");
  let conditionText="";
  const when=raw.match(/\bwhen\b\s+(.+)$/i);const ifm=raw.match(/\bif\b\s+(.+)$/i);
  if(when)conditionText=v533Clean(when[1]);else if(ifm)conditionText=v533Clean(ifm[1]);
  return {name:scope,definedBy:raw,conditionText,conditionAtoms:v533TextAtoms(conditionText),sourceRole:rule?.evidenceRole||null,scope:rule?.scope||"general",kind:"learned_predicate_definition"};
}
function v533StateTransitionSpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);const trigger=(raw.match(/\bwhen\b\s+(.+?)(?:,|\bthen\b|$)/i)||[])[1]||null;
  const target=v533ScopeName(rule,"cognitive_state");
  let action="revise";
  if(/\b(?:preserve|remain|keep|retain)\b/.test(t))action="preserve";
  else if(/\b(?:remove|retract|discard)\b/.test(t))action="remove";
  else if(/\b(?:mark|set)\b/.test(t))action="set";
  else if(/\b(?:update|revise|change|reconsider)\b/.test(t))action="revise";
  return {target,triggerText:v533Clean(trigger||""),triggerAtoms:v533TextAtoms(trigger||""),action,directive:raw,scope:rule?.scope||"general",kind:"learned_state_transition"};
}
function v533PreferenceSpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);let preferred=null,fallback=null,condition=null;
  let m=raw.match(/\bprefer\s+(.+?)(?:\s+(?:over|to|when|if)\s+(.+)|$)/i);if(m){preferred=v533Clean(m[1]);fallback=v533Clean(m[2]||"");}
  m=raw.match(/^(.+?)\s+may\s+still\s+be\s+useful\s+when\s+(.+?)\s+(?:is|are)\s+unavailable\.?$/i);if(m){fallback=v533Clean(m[1]);condition=v533Clean(m[2])+" unavailable";}
  const when=raw.match(/\bwhen\b\s+(.+)$/i);if(!condition&&when)condition=v533Clean(when[1]);
  return {preferred, fallback:fallback||v533ScopeName(rule,"fallback_option"),conditionText:condition||"",conditionAtoms:v533TextAtoms(condition||""),directive:raw,scope:rule?.scope||"general",kind:"learned_preference_policy"};
}
function v533ResourcePolicySpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);const resources=[];
  for(const [rx,name] of [[/\bmemory\b/,"memory"],[/\bsource material\b|\bsource\b/,"source_material"],[/\bweb\b/,"web"],[/\bfile(?:s|system)?\b/,"files"],[/\bself[- ]knowledge\b|\bself model\b/,"self_model"]])if(rx.test(t))resources.push(name);
  return {resources:v533Unique(resources),mode:/\b(?:must not|do not|don't|never|forbid)\b/.test(t)?"forbid":/\b(?:only|closed[- ]book)\b/.test(t)?"restrict":"explicit_if_material",conditionText:(raw.match(/\b(?:if|when)\b\s+(.+)$/i)||[])[1]||"",directive:raw,scope:rule?.scope||"general",kind:"learned_task_resource_policy"};
}
function v533FidelitySpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);let mode="semantic";
  if(/\b(?:exact|verbatim|source)\s+wording\b/.test(t)&& !/\bnot always\b/.test(t))mode="verbatim_when_required";
  if(/\b(?:not always|required only|may choose natural wording|natural wording)\b/.test(t))mode="semantic_unless_verbatim_required";
  if(/\bpreserve\b.{0,25}\b(?:meaning|semantic|roles?|frame|relationships?)\b/.test(t))mode="semantic_preservation";
  return {mode,conditionText:(raw.match(/\b(?:if|when|unless)\b\s+(.+)$/i)||[])[1]||"",directive:raw,scope:rule?.scope||"general",kind:"learned_fidelity_policy"};
}
function v533RepresentationSpec(rule){
  const raw=v533Clean(rule?.directive||"");const t=v533Canonical(raw);let mode="preserve_required_structure";
  if(/\b(?:complexity|structure|frames?|relationships?|roles?)\b/.test(t))mode="source_semantic_shape_fidelity";
  if(/\b(?:do not|don't|should not|must not)\b.{0,30}\b(?:flatten|collapse|detach|replace)\b/.test(t))mode="prevent_structural_loss";
  return {mode,directive:raw,scope:rule?.scope||"general",requiredAtoms:v533TextAtoms(raw),kind:"learned_representation_invariant"};
}

/* ---------------------------------------------------------
   GENERAL SAFE VM OPERATIONS

   Registered into the one current V6.2 interpreter. Learned categories remain
   data; these are generic modifiers that learned programs may compose.
   --------------------------------------------------------- */
if(typeof v502RegisterOperation==="function"){
  v502RegisterOperation("declare_learned_predicate",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.predicates.length,spec={id:args.id||`pred_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.predicates,spec,["name","definedBy"]);return root.predicates.length!==before;});
  v502RegisterOperation("evaluate_learned_predicate",(args,ctx)=>{const root=v533PolicyRoot(ctx),name=v533Canonical(args.name||args.predicate),spec=root.predicates.find(x=>v533Canonical(x.name)===name),text=v533Canonical(args.contextText??(args.contextPath?v533Read(ctx,args.contextPath):"")??""),atoms=spec?.conditionAtoms||[],matched=Boolean(spec)&&(!atoms.length||atoms.every(a=>text.includes(a))),result={predicate:name,known:Boolean(spec),matched,spec:v533Clone(spec||null)};const changed=args.outputPath?v533Write(ctx,args.outputPath,result):false;const sig=JSON.stringify(result),last=root.evaluations.at(-1);if(!last||JSON.stringify({predicate:last.predicate,known:last.known,matched:last.matched,spec:last.spec})!==sig){root.evaluations.push({...result,evaluatedAt:Date.now()});root.evaluations=root.evaluations.slice(-100);return true;}return changed;});
  v502RegisterOperation("declare_state_transition",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.transitions.length,spec={id:args.id||`transition_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.transitions,spec,["target","directive"]);return root.transitions.length!==before;});
  v502RegisterOperation("apply_declared_transition",(args,ctx)=>{const root=v533PolicyRoot(ctx),target=v533Canonical(args.target),spec=root.transitions.find(x=>v533Canonical(x.target)===target||v533Canonical(x.id)===target);if(!spec)return false;const statePath=args.statePath||`learnedState.${spec.target}`,current=v533Read(ctx,statePath),evidence=args.valueFromPath?v533Read(ctx,args.valueFromPath):args.value,next={previous:v533Clone(current),value:v533Clone(evidence??current),action:spec.action||"revise",reason:spec.directive||null};return v533Write(ctx,statePath,next);});
  v502RegisterOperation("declare_preference_policy",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.preferences.length,spec={id:args.id||`pref_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.preferences,spec,["directive","scope"]);return root.preferences.length!==before;});
  v502RegisterOperation("select_preferred_available",(args,ctx)=>{const root=v533PolicyRoot(ctx),options=(args.optionsPath?v533Read(ctx,args.optionsPath):args.options)||[],list=Array.isArray(options)?options:[],policy=root.preferences.find(x=>!args.policyId||x.id===args.policyId)||root.preferences.at(-1);let selected=null;if(policy?.preferred)selected=list.find(x=>v533Canonical(typeof x==="object"?(x.id||x.name||x.label):x)===v533Canonical(policy.preferred));if(!selected&&policy?.fallback)selected=list.find(x=>v533Canonical(typeof x==="object"?(x.id||x.name||x.label):x)===v533Canonical(policy.fallback));if(!selected)selected=list[0]||null;return args.outputPath?v533Write(ctx,args.outputPath,v533Clone(selected)):v533Write(ctx,"selectedPreferred",v533Clone(selected));});
  v502RegisterOperation("declare_task_resource_policy",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.resourcePolicies.length,spec={id:args.id||`resource_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.resourcePolicies,spec,["directive","scope"]);const state=v533State(ctx);if(state){state.taskResourcePolicies ||= [];v533PushUnique(state.taskResourcePolicies,spec,["directive","scope"]);}return root.resourcePolicies.length!==before;});
  v502RegisterOperation("declare_fidelity_policy",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.fidelityPolicies.length,spec={id:args.id||`fidelity_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.fidelityPolicies,spec,["directive","scope"]);const state=v533State(ctx);if(state){state.fidelityPolicies ||= [];v533PushUnique(state.fidelityPolicies,spec,["directive","scope"]);}return root.fidelityPolicies.length!==before;});
  v502RegisterOperation("declare_representation_invariant",(args,ctx)=>{const root=v533PolicyRoot(ctx),before=root.representationInvariants.length,spec={id:args.id||`shape_${Math.random().toString(36).slice(2,8)}`,...v533Clone(args.spec||{}),declaredAt:Date.now()};v533PushUnique(root.representationInvariants,spec,["directive","scope"]);return root.representationInvariants.length!==before;});
  v502RegisterOperation("check_representation_invariant",(args,ctx)=>{const source=args.sourcePath?v533Read(ctx,args.sourcePath):null,target=args.targetPath?v533Read(ctx,args.targetPath):null,sourceCount=source&&typeof source==="object"?Object.keys(source).length:source==null?0:1,targetCount=target&&typeof target==="object"?Object.keys(target).length:target==null?0:1,result={sourceCount,targetCount,preservesNontrivialShape:sourceCount<=1||targetCount>0};return v533Write(ctx,args.outputPath||"representationInvariantCheck",result);});
}

/* ---------------------------------------------------------
   LESSON ROLE HYGIENE
   --------------------------------------------------------- */
function v533CurriculumMetadataRole(role){return ["subject","topic","purpose","end","heading","title","summary","lesson_objective","assessment_criterion"].includes(v533Canonical(role).replace(/\s+/g,"_"));}
function v533CurriculumMetadataText(text){const t=v533Canonical(text);return /^(?:subject|topic|purpose|end)\s*:/.test(t)||/^this lesson (?:teaches|covers|introduces|is about)\b/.test(t)||/^the (?:goal|purpose) of this lesson\b/.test(t);}
/* Operational teaching may legitimately live under definition/relationship
   sections. Only capture definitions/relationships that define a predicate,
   transition, fallback/preference, resource policy, fidelity rule, or structural
   invariant. Ordinary factual definitions remain semantic knowledge only. */
function v533OperationalTeachingKind(text,role="",scope="",domain=""){
  const raw=v533Clean(text),t=v533Canonical(raw),r=v533Canonical(role),s=v533Canonical(scope),d=v533Canonical(domain);
  if(v533CurriculumMetadataRole(r)||v533CurriculumMetadataText(raw))return null;
  if(["example","examples","negative_example","practice","question","answer"].includes(r))return null;
  if((r==="definition"||r==="relationship"||r==="relationships"||r==="instruction"||r==="warning"||r==="constraint"||r==="procedure_step")&&/\b(?:is|are)\b.{0,35}\bwhen\b|\btestable\b.{0,20}\bwhen\b/.test(t))return "predicate";
  if(/\bmeans?\s+(?:revising|updating|changing|reconsidering|retracting)\b/.test(t)||/\bwhen\b.{0,90}\b(?:changes?|appears?|arrives?|fails?|succeeds?)\b.{0,90}\b(?:revise|update|change|reconsider|retract|preserve|mark|set)\b/.test(t)||/\b(?:should|must)\b.{0,50}\b(?:revise|update|change|reconsider|retract)\b/.test(t))return "state_transition";
  if(/\bprefer\b|\bmay still be useful when\b|\bfallback\b|\bwhen .+ unavailable\b/.test(t))return "preference";
  if(/\b(?:memory|source material|information source|source)\b/.test(t)&&/\b(?:consult|use|used|allowed|permitted|explicitly defined|closed[- ]book|open[- ]book)\b/.test(t))return "resource_policy";
  if((d==="realization"||/\b(?:wording|verbatim|exact wording|natural wording|semantic meaning|semantic frame)\b/.test(t))&&/\b(?:preserve|required|not always|may choose|exact|wording|meaning)\b/.test(t))return "fidelity_policy";
  if(/\b(?:complexity|structure|frame|frames|semantic roles?|relationships?)\b/.test(`${s} ${t}`)&&/\b(?:follow|preserve|retain|maintain|flatten|collapse|detach|replace|combine|composition)\b/.test(t))return "representation_invariant";
  return null;
}
function v533DomainForKind(kind,rule={}){
  if(kind==="resource_policy")return "memory";
  if(kind==="fidelity_policy")return "realization";
  if(kind==="representation_invariant")return "interpretation";
  return rule.domain||"reasoning";
}
function v533ModuleForOperationalTeaching(rule,kind){
  const raw=v533Clean(rule?.directive||"");const domain=v533DomainForKind(kind,rule);const phase=kind==="fidelity_policy"?"realization":kind==="resource_policy"?"memory":kind==="representation_invariant"?"interpretation":"reasoning";
  if(kind==="predicate"){
    const spec=v533DeclarativePredicateSpec(rule);return {name:"Synthesized learned predicate",domain,phase,priority:74,conditions:[{type:"always"}],cognitiveForm:"operation_program",policySpec:spec,program:[{op:"declare_learned_predicate",args:{spec}}]};
  }
  if(kind==="state_transition"){
    const spec=v533StateTransitionSpec(rule);return {name:"Synthesized general state transition",domain,phase,priority:75,conditions:[{type:"always"}],cognitiveForm:"state_transformation",policySpec:spec,program:[{op:"declare_state_transition",args:{spec}}]};
  }
  if(kind==="preference"){
    const spec=v533PreferenceSpec(rule);return {name:"Synthesized conditional preference policy",domain,phase,priority:73,conditions:[{type:"always"}],cognitiveForm:"selection_policy",policySpec:spec,program:[{op:"declare_preference_policy",args:{spec}}]};
  }
  if(kind==="resource_policy"){
    const spec=v533ResourcePolicySpec(rule);return {name:"Synthesized task resource policy",domain,phase,priority:82,conditions:[{type:"always"}],cognitiveForm:"memory_policy",policySpec:spec,program:[{op:"declare_task_resource_policy",args:{spec}}]};
  }
  if(kind==="fidelity_policy"){
    const spec=v533FidelitySpec(rule);return {name:"Synthesized response fidelity policy",domain,phase,priority:84,conditions:[{type:"always"}],cognitiveForm:"realization_policy",policySpec:spec,program:[{op:"declare_fidelity_policy",args:{spec}}]};
  }
  if(kind==="representation_invariant"){
    const spec=v533RepresentationSpec(rule);return {name:"Synthesized representation-shape invariant",domain,phase,priority:88,conditions:[{type:"always"}],cognitiveForm:"invariant",policySpec:spec,program:[{op:"declare_representation_invariant",args:{spec}}]};
  }
  return null;
}

/* Register a structural synthesis strategy with the current synthesizer. */
function v533GeneralTeachingSynthesisStrategy(rule,current){
  const kind=v533OperationalTeachingKind(rule?.directive||"",rule?.evidenceRole||"",rule?.scope||"",rule?.domain||""),role=v533Canonical(rule?.evidenceRole||"");
  if(kind&&["predicate","preference","resource_policy","fidelity_policy","representation_invariant"].includes(kind)&&["definition","relationship","relationships"].includes(role)){
    const module=v533ModuleForOperationalTeaching(rule,kind);if(module){const required=v533Unique((module.program||[]).map(x=>x.op));return {module,diagnostic:{status:"synthesized",cognitiveForm:module.cognitiveForm,teachingKind:kind,requiredOperations:required,program:v533Clone(module.program),resolvedMissingOperations:[]}};}
  }
  if(current?.module||current?.diagnostic?.status==="teaching_artifact"||!kind)return current;
  const module=v533ModuleForOperationalTeaching(rule,kind);if(!module)return current;const required=v533Unique((module.program||[]).map(x=>x.op));
  return {module,diagnostic:{status:"synthesized",cognitiveForm:module.cognitiveForm,teachingKind:kind,requiredOperations:required,program:v533Clone(module.program),resolvedMissingOperations:current?.diagnostic?.missingOperations||[]}};
}
if(typeof v520RegisterSynthesisStrategy==="function")v520RegisterSynthesisStrategy("general operational teaching",60,v533GeneralTeachingSynthesisStrategy);

/* ---------------------------------------------------------
   CURRENT SOURCE REBUILD / MAINTENANCE

   Explicit APIs only. Nothing here replaces v450 learning, the cognitive
   compiler, the module VM, or the executive.
   --------------------------------------------------------- */
function v533InstructionSource(source){
  try{if(typeof v534InstructionSource==="function")return v534InstructionSource(source);}catch(_){ }
  if(!source?.text)return false;const name=v533Canonical(source.name||""),head=v533Canonical(String(source.text).slice(0,1600));
  return /lesson|curriculum|teaching/.test(name)||/\b(?:concept|definition|guidance|procedure|warning)\s*:/.test(head);
}
function v533RebuildSourceBackedCognition(options={}){
  if(window.__TAURI__?.core?.invoke)return {version:NOVA533_VERSION,recompiledSources:0,skipped:true,authority:"rust_v7_pass_8",reason:"Native source-backed cognition is rebuilt by Rust teaching ingestion, not the retired JavaScript compiler."};
  let memory=null;try{memory=v43Memory();}catch(_){return {version:NOVA533_VERSION,recompiledSources:0,errors:["memory_unavailable"]};}
  const sources=Array.isArray(options.sources)?options.sources:(memory.sources||[]).filter(v533InstructionSource),errors=[],reports=[];let recompiledSources=0;
  for(const source of sources){
    try{if(typeof v500RemoveSourceCognition==="function")v500RemoveSourceCognition(source);const report=typeof v534CompileDocumentCognition==="function"?v534CompileDocumentCognition(source,{synthesize:false}):(typeof v500CompileCognitionRulesFromText==="function"?v500CompileCognitionRulesFromText(source.text||"",{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null,synthesize:false}):null);reports.push({sourceId:source.id||null,sourceName:source.name||"document",report:v533Clone(report)});recompiledSources++;}
    catch(error){errors.push(`${source.name||source.id||"document"}:${error?.message||error}`);}
  }
  let synthesis=null;try{
    if(window.__TAURI__?.core?.invoke)synthesis={pendingRustSync:true,authority:"rust_v7_pass_8",reason:"source rebuild compiled candidates only; Rust owns executability"};
    else if(typeof v520GeneralSynthesisPass==="function")synthesis=v520GeneralSynthesisPass({reason:"browser_source_rebuild",force:true});
  }catch(error){errors.push(`synthesis:${error?.message||error}`);}
  return {version:NOVA533_VERSION,recompiledSources,sourceCount:sources.length,reports,synthesis:v533Clone(synthesis),errors,completedAt:Date.now()};
}
function v533PostLoadSynthesisMaintenance(options={}){
  if(window.__TAURI__?.core?.invoke)return {version:NOVA533_VERSION,skipped:true,reason:"rust_v7_pass_8_synthesis_authority"};
  let memory=null;try{memory=v43Memory();}catch(_){return {version:NOVA533_VERSION,skipped:true,reason:"memory_unavailable"};}
  const prior=memory.generalTeachingSynthesisMaintenance;if(!options.force&&prior?.version===NOVA533_VERSION)return {version:NOVA533_VERSION,skipped:true,reason:"already_current"};
  const sourceConstructions=[];for(const source of (memory.sources||[]).filter(v533InstructionSource)){try{if(typeof v520SynthesizeSourceCognition==="function"){const structure=typeof v534ParseDocumentStructure==="function"?v534ParseDocumentStructure(source.text||"",{sourceName:source.name||"document",sourceId:source.id||null}):null;sourceConstructions.push(v533Clone(v520SynthesizeSourceCognition(source,structure,{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null})));}}catch(error){sourceConstructions.push({sourceId:source.id||null,sourceName:source.name||"document",status:"error",reason:error?.message||String(error)});}}
  let synthesis=null;try{if(typeof v520GeneralSynthesisPass==="function")synthesis=v520GeneralSynthesisPass({reason:"v6.2_post_load_reconsideration",force:true});}catch(error){synthesis={error:String(error?.message||error)};}
  const report={version:NOVA533_VERSION,sourceConstructions,synthesis:v533Clone(synthesis),updatedAt:Date.now()};memory.generalTeachingSynthesisMaintenance=report;return report;
}

try{if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){V41_SELF_MODEL.identity.version=NOVA533_VERSION;V41_SELF_MODEL.capabilities ||= [];const entry={id:"general_teaching_synthesis",verb:"compose",object:"learned predicates, transitions, preferences, resource policies, fidelity policies, and structural invariants from generic modifiers",detail:"These are reusable starting forms, not a closed list of categories. New learned category labels remain ordinary data."},old=V41_SELF_MODEL.capabilities.find(c=>c.id===entry.id);if(old)Object.assign(old,entry);else V41_SELF_MODEL.capabilities.push(entry);}}catch(_){ }

console.info("Nova V6.2.16 general teaching synthesis loaded.");
