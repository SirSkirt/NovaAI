"use strict";

/* =========================================================
   Nova V6.2.16 — TEACHING SYNTHESIS STRATEGIES

   Purpose:
   - keep V5.2's general cognitive-program synthesis;
   - recognize that lessons can teach several *forms* of cognition, not only
     callable operations;
   - compile supported guidance into declarative operations, invariants,
     selection policies, ordering policies, state transformations, validation
     policies, realization policies, behavior policies, learning policies, and
     memory policies;
   - keep examples, sample responses, practice material, reports, and other
     teaching evidence out of executable cognition;
   - never generate/evaluate JavaScript or bypass external authorization.

   Nova still owns the final interpretation and decision. Learned policies are
   extensions of Nova's brain, not independent advisors.
   ========================================================= */

const NOVA521_VERSION = "6.2.16";
const NOVA521_TEACHING_SYNTHESIS_SCHEMA = 2;

const NOVA521_SEED_COGNITIVE_FORMS = new Set([
  "operation_program",
  "invariant",
  "selection_policy",
  "ordering_policy",
  "state_transformation",
  "validation_policy",
  "realization_policy",
  "behavior_policy",
  "learning_policy",
  "memory_policy"
]);

/* ---------------------------------------------------------
   GENERAL POLICY OPERATIONS

   These are deliberately generic. They do not contain lesson answers.
   Lessons may compose them into learned cognition.
   --------------------------------------------------------- */


function v521Clone(value){
  try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(_){return value;}}
}
function v521Canonical(value){
  try{return canonical(String(value??""));}catch(_){return String(value??"").trim().toLowerCase();}
}
function v521Clean(value){
  try{return normalizeWhitespace(String(value??""));}catch(_){return String(value??"").replace(/\s+/g," ").trim();}
}
function v521Unique(values){
  const out=[];const seen=new Set();
  for(const value of values||[]){
    const key=typeof value==="object"?JSON.stringify(value):v521Canonical(value);
    if(!key||seen.has(key))continue;seen.add(key);out.push(value);
  }
  return out;
}
function v521PolicySignature(value){return v521Canonical(JSON.stringify(value||{}));}
function v521Workspace(ctx){
  if(ctx?.workspace)return ctx.workspace;
  try{if(typeof V460_ACTIVE_WORKSPACE==="object"&&V460_ACTIVE_WORKSPACE)return V460_ACTIVE_WORKSPACE;}catch(_){}
  return null;
}
function v521EnsurePolicyState(ctx){
  ctx.policyState ||= {invariants:[],ordering:[],checks:[],violations:[]};
  const ws=v521Workspace(ctx);
  if(ws){
    ws.moduleState ||= {};
    ws.moduleState.learnedPolicies ||= {invariants:[],ordering:[],checks:[],violations:[]};
  }
  return {local:ctx.policyState,persistent:ws?.moduleState?.learnedPolicies||null,workspace:ws};
}
function v521PushUniquePolicy(list,item){
  if(!Array.isArray(list))return;
  const sig=v521PolicySignature(item);
  if(!list.some(x=>v521PolicySignature(x)===sig))list.push(v521Clone(item));
}
function v521StructuredEqual(a,b){
  if(a===b)return true;
  try{return JSON.stringify(a)===JSON.stringify(b);}catch(_){return v521Canonical(a)===v521Canonical(b);}
}
function v521ReadPath(obj,path){return typeof v502GetPath==="function"?v502GetPath(obj,path):undefined;}
function v521WritePath(obj,path,value){return typeof v502SetPath==="function"?v502SetPath(obj,path,value):false;}

/* Current generic VM operation registrations. These extend the one interpreter
   directly; they do not replace or wrap v502ExecuteStep. */
if(typeof v502RegisterOperation==="function"){
  v502RegisterOperation("declare_invariant",(args,ctx)=>{
    const state=v521EnsurePolicyState(ctx),invariant={id:args.id||`invariant_${Math.random().toString(36).slice(2,8)}`,type:args.type||"structured_preservation",paths:v521Unique(args.paths||[args.path].filter(Boolean)),phases:v521Unique(args.phases||[ctx.phase].filter(Boolean)),enforcement:args.enforcement||"observe",reason:args.reason||null,source:args.source||"learned_cognition",baseline:{}};
    for(const path of invariant.paths)invariant.baseline[path]=v521Clone(v521ReadPath(ctx,path));
    const before=state.local.invariants.length;v521PushUniquePolicy(state.local.invariants,invariant);if(state.persistent)v521PushUniquePolicy(state.persistent.invariants,invariant);if(state.workspace?.integratedCognition){state.workspace.integratedCognition.learnedPolicies||={invariants:[],ordering:[],checks:[],violations:[]};v521PushUniquePolicy(state.workspace.integratedCognition.learnedPolicies.invariants,invariant);}return state.local.invariants.length!==before;
  });
  v502RegisterOperation("declare_ordering",(args,ctx)=>{
    const state=v521EnsurePolicyState(ctx),ordering={id:args.id||`ordering_${Math.random().toString(36).slice(2,8)}`,before:v521Unique(args.before||[]),after:v521Unique(args.after||[]),enforcement:args.enforcement||"require_before",reason:args.reason||null,source:args.source||"learned_cognition"};
    const before=state.local.ordering.length;v521PushUniquePolicy(state.local.ordering,ordering);if(state.persistent)v521PushUniquePolicy(state.persistent.ordering,ordering);if(state.workspace?.integratedCognition){state.workspace.integratedCognition.learnedPolicies||={invariants:[],ordering:[],checks:[],violations:[]};v521PushUniquePolicy(state.workspace.integratedCognition.learnedPolicies.ordering,ordering);}return state.local.ordering.length!==before;
  });
  v502RegisterOperation("assert_paths_present",(args,ctx)=>{
    const state=v521EnsurePolicyState(ctx),paths=v521Unique(args.paths||[]),missing=paths.filter(path=>{const value=v521ReadPath(ctx,path);return value===undefined||value===null||(Array.isArray(value)&&!value.length);}),check={type:"paths_present",paths,passed:missing.length===0,missing,reason:args.reason||null,phase:ctx.phase||null};
    const sig=v521PolicySignature(check),before=state.local.checks.length;if(!state.local.checks.some(x=>v521PolicySignature(x)===sig))state.local.checks.push(check);if(state.persistent&&!state.persistent.checks.some(x=>v521PolicySignature(x)===sig))state.persistent.checks.push(v521Clone(check));if(missing.length){if(!state.local.violations.some(x=>v521PolicySignature(x)===sig))state.local.violations.push(check);if(state.persistent&&!state.persistent.violations.some(x=>v521PolicySignature(x)===sig))state.persistent.violations.push(v521Clone(check));}return state.local.checks.length!==before;
  });
  v502RegisterOperation("compose_structures",(args,ctx)=>{const parts=[];for(const path of args.sourcePaths||[]){const value=v521ReadPath(ctx,path);if(value==null)continue;if(Array.isArray(value))parts.push(...value.map(v521Clone));else parts.push(v521Clone(value));}return v521WritePath(ctx,args.outputPath||"composedStructure",{kind:args.kind||"composed_structure",members:parts,memberCount:parts.length,preserveMembers:true});});
  v502RegisterOperation("preserve_structure",(args,ctx)=>{ctx.__nova620Protected||={};const key=args.key||args.path;if(!key)return false;let changed=false;if(!Object.prototype.hasOwnProperty.call(ctx.__nova620Protected,key)){ctx.__nova620Protected[key]=v521Clone(v521ReadPath(ctx,args.path));changed=true;}const ws=v521Workspace(ctx);if(ws){ws.moduleState||={};ws.moduleState.protectedStructures||={};if(!Object.prototype.hasOwnProperty.call(ws.moduleState.protectedStructures,key)){ws.moduleState.protectedStructures[key]=v521Clone(v521ReadPath(ctx,args.path));changed=true;}}return changed;});
  v502RegisterOperation("restore_structure",(args,ctx)=>{const key=args.key||args.path,ws=v521Workspace(ctx),value=ctx.__nova620Protected?.[key]??ws?.moduleState?.protectedStructures?.[key];return value===undefined?false:v521WritePath(ctx,args.path,v521Clone(value));});
}





/* Preserve cognitive form metadata when a declarative module is persisted. */


/* ---------------------------------------------------------
   TEACHING EVIDENCE HYGIENE
   --------------------------------------------------------- */
function v521EvidenceOnlyScope(scope){
  const s=v521Canonical(scope).replace(/[_-]+/g," ");
  if(!s)return false;
  return /^(?:example|examples|practice|practice question|practice questions|quiz|test question|test questions|sample|sample response|sample output|expected response|expected answer|incorrect interpretation|negative example)(?:\s+\d+)?\b/.test(s)
    || /\bsemantic representation example(?:\s+\d+)?\b/.test(s)
    || /^honest learning report(?:\s+for\b.*)?$/.test(s)
    || /^demonstration(?:\s+\d+)?\b/.test(s);
}
function v521EvidenceOnlyRole(role){
  const r=v521Canonical(role).replace(/[_-]+/g," ");
  return /^(?:example|examples|practice|question|answer|expected response|sample|demonstration|incorrect interpretation|negative example)$/.test(r);
}

function v521RemoveTeachingEvidenceCognition(){
  if(typeof v500CognitiveStore!=="function")return {removedRules:0,removedModules:0};
  const store=v500CognitiveStore();const removed=[];
  for(const rule of store.rules||[]){
    if(rule?.sourceType!=="learned")continue;
    if(v521EvidenceOnlyScope(rule.scope)||v521EvidenceOnlyRole(rule.evidenceRole))removed.push(rule);
  }
  if(!removed.length)return {removedRules:0,removedModules:0};
  const ids=new Set(removed.map(r=>r.id));const moduleIds=new Set(removed.map(r=>r.moduleId).filter(Boolean));
  const signatures=new Set(removed.map(r=>`${r.sourceName||""}|${v521Canonical(r.directive||"")}`));
  const beforeRules=store.rules.length;store.rules=store.rules.filter(r=>!ids.has(r.id));store.updatedAt=Date.now();
  let removedModules=0;
  try{
    const modules=v502ModuleStore();const before=modules.modules.length;
    modules.modules=modules.modules.filter(m=>{
      if(moduleIds.has(m.id))return false;
      const evidence=(m.evidence||[]).map(v521Canonical);
      return ![...signatures].some(sig=>{const [sourceName,directive]=sig.split("|");return sourceName===(m.sourceName||"")&&evidence.includes(directive);});
    });
    removedModules=before-modules.modules.length;if(removedModules)modules.updatedAt=Date.now();
  }catch(_){}
  return {removedRules:beforeRules-store.rules.length,removedModules};
}

/* ---------------------------------------------------------
   COGNITIVE-FORM CLASSIFICATION
   --------------------------------------------------------- */
function v521ClassifyCognitiveForm(rule){
  const explicit=v521Clean(rule?.cognitiveForm||rule?.learnedCategory||rule?.category||"");if(explicit)return explicit;
  const t=v521Canonical(rule?.directive||"");const d=v521Canonical(rule?.domain||"general");
  if(d==="learning")return "learning_policy";
  if(d==="memory")return "memory_policy";
  if(d==="validation")return "validation_policy";
  if(d==="behavior")return "behavior_policy";
  if(/\b(?:before|after|prior to|only after)\b/.test(t)&&/\b(?:wording|realiz|template|plan|meaning|semantic target|viewpoint|interpret|reason|retriev|validat)\b/.test(t))return "ordering_policy";
  if(d==="realization")return "realization_policy";
  if(/\b(?:preserve|keep|retain|maintain|do not replace|don't replace|should not replace|must not replace|do not collapse|don't collapse|should not collapse|do not detach|don't detach|should not detach|do not switch|don't switch|should not switch)\b/.test(t))return "invariant";
  if(d==="reference"&&/\b(?:candidate|nearest|distant|ambiguous|referent|antecedent|support|rank|choose|select)\b/.test(t))return "selection_policy";
  if(/\b(?:combine|combines|composition|compose|composes|update|updates|change|changes|map|maps|transform|transforms|convert|converts|reduce|reduces|resolve|resolves)\b/.test(t))return "state_transformation";
  return "operation_program";
}

function v521AnnotateModule(module,rule,form,policySpec=null){
  if(!module)return module;
  module.cognitiveForm=form;
  module.categories=v521Unique([...(module.categories||[]),rule?.domain,rule?.scope,rule?.learnedCategory,form].filter(Boolean));
  module.policySpec=policySpec? v521Clone(policySpec):null;
  module.synthesisIntent={directive:rule?.directive||"",scope:rule?.scope||"general",domain:rule?.domain||module.domain||module.phase||"general"};
  return module;
}

function v521PolicyModule(rule,form,spec){
  const domain=spec.domain||rule.domain||"reasoning";const phase=spec.phase||domain;
  const module={
    name:spec.name||`Synthesized ${String(form).replace(/_/g," ")}`,
    domain,phase,priority:Number(spec.priority||75),conditions:spec.conditions||[{type:"always"}],
    program:spec.program||[],cognitiveForm:form,policySpec:spec.policySpec||null
  };
  return v521AnnotateModule(module,rule,form,spec.policySpec||null);
}

/* ---------------------------------------------------------
   DEEPER GENERAL SYNTHESIS
   --------------------------------------------------------- */
function v521TeachingSynthesisStrategy(rule,current){
    const form=v521ClassifyCognitiveForm(rule);const raw=v521Clean(rule?.directive||"");const t=v521Canonical(raw);const scope=v521Canonical(rule?.scope||"");const domain=v521Canonical(rule?.domain||"general");

    if(v521EvidenceOnlyScope(rule?.scope)||v521EvidenceOnlyRole(rule?.evidenceRole)){
      return {module:null,diagnostic:{status:"teaching_artifact",reason:"lesson material is evidence/example/report rather than persistent cognitive guidance",requiredOperations:[],cognitiveForm:form}};
    }

    const base=current||{module:null,diagnostic:null};
    if(base?.module){
      v521AnnotateModule(base.module,rule,form,base.module.policySpec||null);
      base.diagnostic={...(base.diagnostic||{}),cognitiveForm:form};
      return base;
    }
    if(base?.diagnostic?.status==="teaching_artifact")return {...base,diagnostic:{...base.diagnostic,cognitiveForm:form}};

    // Invariant: preserve the exact untouched input. V5.2 handled "do not
    // replace"; V5.2.2 also understands should/must-not wording.
    if(/\b(?:original|exact)\s+(?:current\s+)?utterance\b/.test(t)&&/\b(?:preserve|keep|retain|do not replace|don't replace|should not replace|must not replace)\b/.test(t)){
      const module=v521PolicyModule(rule,"invariant",{
        name:"Synthesized immutable-input invariant",domain:"interpretation",phase:"interpretation",priority:95,
        conditions:[{type:"field_exists",field:"immutable.rawInput"}],
        policySpec:{type:"path_unchanged",paths:["workspace.rawInput"],reason:"preserve exact original utterance"},
        program:[
          {op:"ensure_field_equals",args:{sourcePath:"immutable.rawInput",targetPath:"workspace.rawInput"}},
          {op:"declare_invariant",args:{type:"path_unchanged",paths:["workspace.rawInput"],enforcement:"restore",reason:"preserve exact original utterance"}}
        ]
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"invariant",requiredOperations:["ensure_field_equals","declare_invariant"],program:module.program}};
    }

    // Nested/embedded meaning is a structural invariant: the outer task remains
    // authoritative and the embedded proposition remains represented as its
    // object rather than being promoted to a separate task.
    if(/\b(?:outer relation|outer request|nested proposition|embedded proposition|detach|independent task|modifier)\b/.test(`${scope} ${t}`)){
      const module=v521PolicyModule(rule,"invariant",{
        name:"Synthesized nested-semantic-structure invariant",domain:"composition",phase:"composition",priority:91,
        conditions:[{type:"field_exists",field:"semanticComposition"}],
        policySpec:{type:"fields_coexist",paths:["semanticComposition.speechAct","semanticComposition.embeddedPropositions"],reason:"preserve outer task and nested proposition structure"},
        program:[
          {op:"set_field",args:{path:"semanticComposition.outerSpeechActAuthoritative",value:true}},
          {op:"declare_invariant",args:{type:"fields_coexist",paths:["semanticComposition.speechAct","semanticComposition.embeddedPropositions"],enforcement:"observe",reason:"preserve outer task and nested proposition structure"}}
        ]
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"invariant",requiredOperations:["set_field","declare_invariant"],program:module.program}};
    }

    // Frame composition: build a non-destructive composed structure out of any
    // semantic subframes already produced by Nova. The operation never invents
    // facts; it only combines existing structured members while preserving them.
    if(/\bframe composition\b/.test(`${scope} ${t}`)&&/\b(?:combine|combines|compose|multiple frames|connected events)\b/.test(t)){
      const module=v521PolicyModule(rule,"state_transformation",{
        name:"Synthesized semantic-frame composition",domain:"composition",phase:"composition",priority:78,
        conditions:[{type:"field_exists",field:"semanticComposition"}],
        program:[{op:"compose_structures",args:{sourcePaths:["semanticComposition.requestedAction","semanticComposition.evaluation","semanticComposition.embeddedPropositions"],outputPath:"semanticComposition.composedFrames",kind:"semantic_frame_composition"}}]
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"state_transformation",requiredOperations:["compose_structures"],program:module.program}};
    }

    // Realization invariants: surface wording is allowed to vary, but the
    // semantic claim/roles supplied to realization are protected as the source
    // of truth. Perspective operations may change pronouns without changing
    // ownership.
    if(domain==="realization"&&/\b(?:preserve the semantic frame|preserve semantic frame|semantic roles|pronouns encode semantic roles|preserve.*meaning|preserve.*relationship)\b/.test(t)){
      const program=[
        {op:"preserve_structure",args:{path:"claim",key:"realization_semantic_claim"}},
        {op:"declare_invariant",args:{type:"structured_preservation",paths:["claim"],enforcement:"observe",reason:"wording must preserve semantic meaning and roles"}}
      ];
      if(/\bpronouns?\b|\bsemantic roles?\b/.test(t))program.push({op:"perspective_subject"});
      const module=v521PolicyModule(rule,"realization_policy",{
        name:"Synthesized semantic-preserving realization policy",domain:"realization",phase:"realization",priority:92,
        conditions:[{type:"field_exists",field:"claim"}],
        policySpec:{type:"semantic_preservation",paths:["claim"],reason:"wording may vary but semantic ownership/roles remain authoritative"},program
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"realization_policy",requiredOperations:v521Unique(program.map(x=>x.op)),program}};
    }

    // Ordering policies teach *when* an already-available faculty may run.
    // They do not decide content. This supports lessons such as meaning before
    // wording or semantic target/viewpoint before template selection.
    if(form==="ordering_policy"){
      let before=[],after=[];
      if(/\bsemantic (?:target|meaning|response plan)\b/.test(t))before.push("semantic_meaning");
      if(/\b(?:viewpoint|perspective)\b/.test(t))before.push("response_viewpoint");
      if(/\b(?:template)\b/.test(t))after.push("template_selection");
      if(/\b(?:final wording|wording|realization|realizer)\b/.test(t))after.push("surface_wording");
      if(!before.length&&/\bmeaning\b/.test(t))before.push("semantic_meaning");
      if(!after.length)after.push("surface_wording");
      const ordering={before:v521Unique(before),after:v521Unique(after),reason:raw};
      const module=v521PolicyModule(rule,"ordering_policy",{
        name:"Synthesized cognition-ordering policy",domain:domain==="realization"?"realization":"planning",phase:domain==="realization"?"realization":"planning",priority:88,
        conditions:[{type:"always"}],policySpec:{type:"ordering",...ordering},
        program:[{op:"declare_ordering",args:{...ordering,enforcement:"require_before"}}]
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"ordering_policy",requiredOperations:["declare_ordering"],program:module.program}};
    }

    // Cross-phase preservation guidance becomes an invariant over Nova's shared
    // semantic state rather than a sentence-shaped function.
    if(/\bpreserve\b/.test(t)&&/\b(?:interpretation|reasoning|retrieval|planning|validation|final wording|realization)\b/.test(t)&&/\b(?:relationships?|semantic|meaning|roles?)\b/.test(t)){
      const phases=["interpretation","reasoning","memory","planning","validation","realization"].filter(p=>t.includes(p)|| (p==="memory"&&t.includes("retrieval")) || (p==="realization"&&t.includes("final wording")));
      const module=v521PolicyModule(rule,"invariant",{
        name:"Synthesized cross-phase semantic-preservation invariant",domain:"interpretation",phase:"interpretation",priority:89,
        conditions:[{type:"field_exists",field:"workspace"}],
        policySpec:{type:"cross_phase_structure_preservation",paths:["workspace.semanticComposition","workspace.interpretation"],phases,reason:raw},
        program:[{op:"declare_invariant",args:{type:"cross_phase_structure_preservation",paths:["workspace.semanticComposition","workspace.interpretation"],phases,enforcement:"observe",reason:raw}}]
      });
      return {module,diagnostic:{status:"synthesized",cognitiveForm:"invariant",requiredOperations:["declare_invariant"],program:module.program}};
    }

    // Make the fallback diagnostic explicitly say what *kind* of cognition was
    // understood even if the current CPU cannot yet synthesize it.
    return {module:null,diagnostic:{...(base?.diagnostic||{}),status:base?.diagnostic?.status||"missing_fundamental_capability",cognitiveForm:form,reason:base?.diagnostic?.reason||"the learned cognitive form is understood, but its required state transformation cannot yet be composed safely"}};

}
if(typeof v520RegisterSynthesisStrategy==="function")v520RegisterSynthesisStrategy("teaching policy synthesis",90,v521TeachingSynthesisStrategy);

/* Preserve the cognitive form in synthesis diagnostics. */


/* v520AttemptRuleSynthesis does not forward arbitrary diagnostic fields, so
   annotate the rule/module after each attempt. */


/* ---------------------------------------------------------
   POLICY AWARENESS + INTEGRATED EXECUTION
   --------------------------------------------------------- */
/* Ordering and other learned policies are ordinary module state in V6.2.
   The current executive evaluates learned modules directly; no V5 operation
   dispatcher is intercepted here. */

/* ---------------------------------------------------------
   SELF MODEL
   --------------------------------------------------------- */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA521_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const id="multi_form_cognitive_learning",entry={id,verb:"learn",object:"multiple forms and categories of cognition from lessons",detail:"Bootstrap forms help early teaching, but learned module categories are open data and are not restricted to this seed vocabulary."};
    const existing=V41_SELF_MODEL.capabilities.find(c=>c.id===id);if(existing)Object.assign(existing,entry);else V41_SELF_MODEL.capabilities.push(entry);
  }
}catch(_){ }

/* ---------------------------------------------------------
   STARTUP MIGRATION
   --------------------------------------------------------- */
try{
  const cleaned=v521RemoveTeachingEvidenceCognition();
  const pass=!window.__TAURI__?.core?.invoke&&typeof v520GeneralSynthesisPass==="function"?v520GeneralSynthesisPass({reason:"browser_multi_form_reconsideration",force:true}):null;
  const memory=typeof v43Memory==="function"?v43Memory():null;
  if(memory){memory.teachingSynthesis={version:NOVA521_TEACHING_SYNTHESIS_SCHEMA,appVersion:NOVA521_VERSION,lastCleanup:cleaned,lastSynthesis:pass,authority:window.__TAURI__?.core?.invoke?"rust_v7_pass_6":"browser_js",updatedAt:Date.now()};}
}catch(_){}
