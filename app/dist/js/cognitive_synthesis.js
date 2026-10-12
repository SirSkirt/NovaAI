"use strict";

/* =========================================================
   Nova V6.2.16 — GENERAL COGNITIVE PROGRAM SYNTHESIS

   This is the current synthesizer for Nova's generic learned-module kernel.
   Stored Only means that the current reusable cognitive substrate cannot yet
   express the learned transformation; it never triggers an older responder.

   Safety / architecture boundaries:
   - learned programs remain declarative data;
   - no eval, Function, generated JavaScript, filesystem, network, app control,
     or permission changes are available to learned cognition;
   - lesson evidence/examples cannot become executable merely because synthesis
     becomes more capable;
   - learned programs may compose only whitelisted, reusable mental operations;
   - Nova's central executive remains the decision owner.
   ========================================================= */

const NOVA520_VERSION = "6.2.16";
const NOVA520_SYNTHESIS_SCHEMA_VERSION = 5;

/* ---------------------------------------------------------
   FUNDAMENTAL MENTAL INSTRUCTION SET

   These are deliberately lower-level than curriculum ideas.  They manipulate
   structured turn-scoped cognitive state.  A lesson should normally construct
   a program from these instead of requiring another sentence-shaped primitive.
   --------------------------------------------------------- */
if(typeof NOVA502_MODULE_PHASES === "object" && NOVA502_MODULE_PHASES?.add){
  for(const phase of ["learning","validation","planning","language"]) NOVA502_MODULE_PHASES.add(phase);
}
if(typeof NOVA502_MODULE_OPS === "object" && NOVA502_MODULE_OPS?.add){
  for(const op of [
    "copy_field","remove_field","compare_values","count_items","branch","for_each",
    "unique_list","intersect_list","preserve_field","restore_field","ensure_field_equals",
    "score_candidates","select_best_candidate","mark_ambiguity_by_score"
  ]) NOVA502_MODULE_OPS.add(op);
}

function v520Clone(value){
  try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(_){return value;}}
}
function v520Canonical(value){
  try{return canonical(String(value??""));}catch(_){return String(value??"").trim().toLowerCase();}
}
function v520Clean(value){
  try{return normalizeWhitespace(String(value??""));}catch(_){return String(value??"").replace(/\s+/g," ").trim();}
}
function v520Array(value){return Array.isArray(value)?value:(value==null?[]:[value]);}
function v520Unique(values){
  const out=[];const seen=new Set();
  for(const value of values||[]){
    const key=typeof value==="object"?JSON.stringify(value):v520Canonical(value);
    if(seen.has(key))continue;seen.add(key);out.push(value);
  }
  return out;
}
function v520Compare(a,b){
  if(typeof a==="number"&&typeof b==="number")return a===b?"equal":a<b?"less_than":"greater_than";
  const ca=v520Canonical(a), cb=v520Canonical(b);
  if(ca===cb)return "equal";
  if(ca.includes(cb)&&cb)return "contains";
  if(cb.includes(ca)&&ca)return "contained_by";
  return "not_equal";
}
function v520Read(spec,ctx){
  if(spec && typeof spec==="object" && !Array.isArray(spec)){
    if(Object.prototype.hasOwnProperty.call(spec,"path"))return typeof v502GetPath==="function"?v502GetPath(ctx,spec.path):undefined;
    if(Object.prototype.hasOwnProperty.call(spec,"value"))return spec.value;
  }
  return spec;
}
function v520Predicate(test,ctx){
  if(!test)return true;
  if(Array.isArray(test))return test.every(item=>v520Predicate(item,ctx));
  if(test.all)return v520Array(test.all).every(item=>v520Predicate(item,ctx));
  if(test.any)return v520Array(test.any).some(item=>v520Predicate(item,ctx));
  if(test.not)return !v520Predicate(test.not,ctx);
  const type=v520Canonical(test.type||"truthy").replace(/\s+/g,"_");
  const current=typeof v502GetPath==="function"?v502GetPath(ctx,test.field||test.path):undefined;
  if(type==="field_exists")return current!==undefined&&current!==null;
  if(type==="field_missing")return current===undefined||current===null;
  if(type==="truthy")return Boolean(current);
  if(type==="falsy")return !current;
  if(type==="field_equals")return v520Canonical(current)===v520Canonical(test.value);
  if(type==="field_not_equals")return v520Canonical(current)!==v520Canonical(test.value);
  if(type==="field_in")return (test.values||[]).some(value=>v520Canonical(value)===v520Canonical(current));
  if(type==="field_not_in")return !(test.values||[]).some(value=>v520Canonical(value)===v520Canonical(current));
  if(type==="count_compare"){
    const count=Array.isArray(current)?current.length:Number(current||0);const target=Number(test.value||0);
    const op=test.operator||test.compare||">=";
    if(op===">")return count>target;if(op==="<")return count<target;if(op==="<=")return count<=target;if(op==="===")return count===target;if(op==="!=")return count!==target;
    return count>=target;
  }
  if(typeof v502ConditionMatches==="function")return v502ConditionMatches(test,ctx);
  return false;
}
function v520ExecuteNested(program,ctx){
  for(const child of program||[]) if(typeof v502ExecuteStep==="function")v502ExecuteStep(child,ctx);
}

/* Validation and operation execution are owned directly by the current
   generic module kernel in cognitive_modules.js. Synthesis describes programs;
   it does not wrap or replace the interpreter. */

/* ---------------------------------------------------------
   SYNTHESIS DIAGNOSTICS
   --------------------------------------------------------- */
function v520SynthesisStore(){
  const memory=typeof v43Memory==="function"?v43Memory():{};
  if(!memory.cognitiveSynthesis||memory.cognitiveSynthesis.version!==NOVA520_SYNTHESIS_SCHEMA_VERSION){
    const prior=memory.cognitiveSynthesis||{};
    memory.cognitiveSynthesis={version:NOVA520_SYNTHESIS_SCHEMA_VERSION,lastPass:prior.lastPass||null,history:Array.isArray(prior.history)?prior.history:[],sourceConstructions:Array.isArray(prior.sourceConstructions)?prior.sourceConstructions:[],capabilityGraph:prior.capabilityGraph||null,createdAt:prior.createdAt||Date.now(),updatedAt:Date.now()};
  }
  return memory.cognitiveSynthesis;
}
function v520CapabilityTermsForModule(module){
  const out=[];for(const value of [...(module?.capabilities||[]),...(module?.produces||[])]){const key=v520Canonical(value);if(key&&!out.includes(key))out.push(key);}const form=v520Canonical(module?.cognitiveForm||"");if(form)out.push(`cognitive_form:${form}`);return v520Unique(out);
}
function v520RuleCapabilityNeeds(rule){
  const out=[];for(const value of [...(rule?.synthesis?.missingCapabilities||[]),...(rule?.synthesis?.missingOperations||[])]){const key=v520Canonical(value);if(key&&!["unmapped cognitive state transformation","unmapped_cognitive_state_transformation"].includes(key))out.push(key);}const form=v520Canonical(rule?.cognitiveForm||"");if(form)out.push(`cognitive_form:${form}`);const category=v520Canonical(rule?.learnedCategory||"");if(category)out.push(`learned_category:${category}`);return v520Unique(out);
}
function v520CapabilitySignature(){
  const ops=[...(typeof NOVA502_MODULE_OPS==="object"?NOVA502_MODULE_OPS:[])].map(v520Canonical).sort(),modules=[];try{for(const module of v502ModuleStore()?.modules||[])if(module?.enabled!==false)modules.push(...v520CapabilityTermsForModule(module));}catch(_){ }
  return v520Canonical(JSON.stringify({ops,modules:v520Unique(modules).sort()}));
}
function v520VocabularySignature(){
  const ops=[...(typeof NOVA502_MODULE_OPS==="object"?NOVA502_MODULE_OPS:[])].sort();
  const phases=[...(typeof NOVA502_MODULE_PHASES==="object"?NOVA502_MODULE_PHASES:[])].sort();
  return v520Canonical(JSON.stringify({ops,phases,capabilities:v520CapabilitySignature()}));
}
function v520RebuildCapabilityGraph(rules=null){
  const inv=rules||((typeof v500CognitiveRuleInventory==="function"?v500CognitiveRuleInventory():{rules:[]})?.rules||[]),producers={},waiters={};
  try{for(const module of v502ModuleStore()?.modules||[]){if(module?.enabled===false)continue;for(const capability of v520CapabilityTermsForModule(module)){producers[capability] ||= [];producers[capability].push({moduleId:module.id,moduleNumber:module.moduleNumber||null,name:module.name});}}}catch(_){ }
  for(const rule of inv||[]){if(rule?.sourceType!=="learned"||["executable","module_executable"].includes(rule?.status))continue;for(const capability of v520RuleCapabilityNeeds(rule)){waiters[capability] ||= [];waiters[capability].push({ruleId:rule.id,directive:v520Clean(rule.directive||"").slice(0,180),sourceName:rule.sourceName||null});}}
  const graph={version:NOVA520_SYNTHESIS_SCHEMA_VERSION,signature:v520CapabilitySignature(),producerCapabilityCount:Object.keys(producers).length,waitingCapabilityCount:Object.keys(waiters).length,producers,waiters,updatedAt:Date.now()};const store=v520SynthesisStore();store.capabilityGraph=v520Clone(graph);store.updatedAt=Date.now();return graph;
}
function v520AttachDiagnostic(rule,diagnostic){
  if(!rule)return diagnostic;rule.synthesis={version:NOVA520_SYNTHESIS_SCHEMA_VERSION,vocabularySignature:v520VocabularySignature(),...v520Clone(diagnostic),attemptedAt:Date.now()};rule.updatedAt=Date.now();return rule.synthesis;
}
function v520Diagnostic(rule,status,extra={}){
  const missingOperations=extra.missingOperations||[],missingCapabilities=v520Unique([...(extra.missingCapabilities||[]),...missingOperations.filter(value=>!["unmapped cognitive state transformation","unmapped_cognitive_state_transformation"].includes(v520Canonical(value))),...(rule?.cognitiveForm?[`cognitive_form:${v520Canonical(rule.cognitiveForm)}`]:[]),...(rule?.learnedCategory?[`learned_category:${v520Canonical(rule.learnedCategory)}`]:[])]);
  return v520AttachDiagnostic(rule,{status,domain:rule?.domain||"general",scope:rule?.scope||"general",directive:rule?.directive||"",...v520Clone(extra),requiredOperations:extra.requiredOperations||[],availableOperations:extra.availableOperations||[],missingOperations,missingCapabilities,reason:extra.reason||null,moduleId:extra.moduleId||null,program:extra.program||null});
}

function v520IsDanglingLessonHeader(text){
  const raw=String(text||"").trim();
  return /^(?:when|before|after|while)\b[^.!?]{0,120}:\s*$/i.test(raw) || /^(?:when processing|before finalizing|after processing)\b[^.!?]{0,100}$/i.test(raw);
}
function v520GuidanceFeatures(rule){
  const raw=v520Clean(rule?.directive||"");const t=v520Canonical(raw);const scope=v520Canonical(rule?.scope||"");const domain=v520Canonical(rule?.domain||"general");
  return {
    raw,t,scope,domain,
    negative:/\b(?:do not|don't|does not|should not|must not|never|avoid|not always|cannot|can't)\b/.test(t),
    preserve:/\b(?:preserve|preserved|keep|kept|retain|retained|do not replace|don't replace|remain|maintain)\b/.test(t),
    compare:/\b(?:compare|versus|vs\.?|stronger|weaker|more|less|nearest|distant|ambiguous|possible interpretations?)\b/.test(t),
    reference:/\b(?:pronoun|referent|antecedent|reference|listener|addressee|speaker)\b/.test(`${scope} ${t}`),
    coordination:/\b(?:coordinate|coordinated|multiple|both|separate actions?|merge)\b/.test(`${scope} ${t}`),
    sourceWording:/\bsource wording\b/.test(t),
    validation:/\b(?:validation|validator|requirements?)\b/.test(`${domain} ${scope} ${t}`),
    lessonEligibility:/\b(?:sentence|statement|material|example)\b.{0,60}\b(?:lesson|teaching)\b|\blesson\b.{0,60}\b(?:executable rule|cognitive rule|stored)\b/.test(t),
    originalUtterance:/\b(?:original|exact)\s+(?:current\s+)?utterance\b|\bexact original\b/.test(t),
    nested:/\b(?:outer relation|outer request|nested proposition|embedded proposition|detach|independent task|modifier)\b/.test(`${scope} ${t}`),
    scopedContext:/\b(?:scope|scoped|frame|inside|outside|outer|inner|boundary|ends?|leak|inherit|local|quoted|quotation)\b/.test(`${scope} ${t}`),
    participantScope:/\b(?:participant|speaker|addressee|listener|first-person|first person|second-person|second person|pronoun|mapping|reference)\b/.test(`${scope} ${t}`),
    ambiguity:/\b(?:ambiguous|ambiguity|two or more interpretations|multiple interpretations|reasonably possible)\b/.test(t),
    semanticCompatibility:/\b(?:semantic|meaning|relationship expressed|make sense|grammar)\b/.test(t)
  };
}

function v520ReferenceStrategyModule(rule,features){
  // Only synthesize a ranking policy when the teaching actually describes a
  // candidate-selection strategy.  This is one reusable program family, not a
  // per-pronoun function.
  if(!features.reference)return null;
  const strategySignal=features.compare||features.ambiguity||features.semanticCompatibility||/\bcontext\b/.test(features.t);
  if(!strategySignal)return null;
  // The current generic candidate representation can score locality, recency,
  // person/type and non-conflicting metadata.  It cannot yet evaluate arbitrary
  // predicate/world relationships, so explicitly relationship-dependent lessons
  // remain diagnostic rather than being falsely marked executable.
  if(/\brelationship(?:s)?\b/.test(features.t)&&!/(?:context|grammar|meaning|nearest|distant|ambigu)/.test(features.t))return null;
  const criteria=[{kind:"local",weight:3},{kind:"recent",weight:2},{kind:"person",weight:1}];
  const program=[
    {op:"collect_candidates",args:{sources:["local","discourse"]}},
    {op:"score_candidates",args:{listPath:"candidates",criteria}},
    {op:"select_best_candidate",args:{listPath:"candidates",outputPath:"selectedCandidate"}}
  ];
  if(features.ambiguity)program.push({op:"mark_ambiguity_by_score",args:{listPath:"candidates",outputPath:"ambiguous",tolerance:0.25,confidence:0.45}});
  // Only assign when not ambiguous. First/second-person participant mappings are
  // deliberately left to their participant frame instead of being overwritten.
  const assignTests=[{type:"field_not_in",field:"mention.surface",values:["i","me","my","mine","you","your","yours","we","us","our","ours"]}];
  if(features.ambiguity)assignTests.push({type:"field_equals",field:"ambiguous",value:false});
  program.push({op:"branch",args:{if:{all:assignTests},then:[{op:"assign_reference",args:{confidence:0.82}}]}});
  return {name:"Synthesized contextual reference selection",domain:"reference",phase:"reference",conditions:[{type:"field_exists",field:"mention.surface"}],program,priority:64};
}

function v520KernelSynthesizeRule(rule){
  if(!rule||rule.sourceType!=="learned")return {module:null,diagnostic:null};
  const f=v520GuidanceFeatures(rule);
  if(v520IsDanglingLessonHeader(rule.directive))return {module:null,diagnostic:{status:"teaching_artifact",reason:"dangling lesson/setup header has no executable consequent",requiredOperations:[]}};

  // Learning policy: base hygiene already rejects examples/tests. A learned rule
  // can make future ingestion stricter, but can never re-admit evidence the hard
  // hygiene layer rejected.
  if(f.lessonEligibility && /\b(?:not every|should not|must not|do not|don't)\b/.test(f.t)){
    const module={name:"Synthesized instructional-role learning gate",domain:"learning",phase:"learning",conditions:[{type:"always"}],program:[{op:"branch",args:{if:{type:"field_not_in",field:"unit.role",values:["instruction","important","guidance","rule","principle","behavior","meaning","validation rule","constraint","warning"]},then:[{op:"set_field",args:{path:"decision.eligible",value:false}}]}}],priority:80};
    return {module,diagnostic:{status:"synthesized",requiredOperations:["branch","set_field"],program:module.program}};
  }

  // Memory policy: preserve exact source evidence wording. The memory hook exposes
  // `source.text` and `evidence.text`, so the learned policy can enforce the copy.
  if(f.sourceWording && f.preserve){
    const module={name:"Synthesized source-wording preservation",domain:"memory",phase:"memory",conditions:[{type:"field_exists",field:"source.text"}],program:[{op:"copy_field",args:{fromPath:"source.text",toPath:"evidence.text"}}],priority:78};
    return {module,diagnostic:{status:"synthesized",requiredOperations:["copy_field"],program:module.program}};
  }

  // Validation policy: validators may only report requirements already present in
  // the goal/contract. `intersect_list` is a generic collection operation.
  if(f.validation && f.negative && /\b(?:invent|add|create)\b.{0,35}\b(?:new )?requirements?\b/.test(f.t)){
    const module={name:"Synthesized validation requirement boundary",domain:"validation",phase:"validation",conditions:[{type:"field_exists",field:"result"}],program:[
      {op:"intersect_list",args:{targetPath:"result.missingRequired",allowedPath:"baseline.requirements"}},
      {op:"intersect_list",args:{targetPath:"result.missing",allowedPath:"baseline.requirements"}}
    ],priority:86};
    return {module,diagnostic:{status:"synthesized",requiredOperations:["intersect_list"],program:module.program}};
  }

  // Architectural/language invariant taught by lesson: learned cognition can
  // reinforce preservation of the raw utterance but is not the sole thing that
  // protects it.
  if(f.originalUtterance && f.preserve){
    const module={name:"Synthesized original-utterance preservation",domain:"interpretation",phase:"interpretation",conditions:[{type:"field_exists",field:"immutable.rawInput"}],program:[{op:"ensure_field_equals",args:{sourcePath:"immutable.rawInput",targetPath:"workspace.rawInput"}}],priority:94};
    return {module,diagnostic:{status:"synthesized",requiredOperations:["ensure_field_equals"],program:module.program}};
  }

  // Coordination is a general list/structure-preservation problem. Reuse the
  // existing declarative operation even when an older classifier put the rule in
  // the wrong domain because of a word like "merge".
  if(f.coordination && /\b(?:do not|don't|avoid|preserve|keep|separate|not merge|merge)\b/.test(f.t)){
    const module={name:"Synthesized coordinated-action preservation",domain:"composition",phase:"composition",conditions:[{type:"contains_any",values:[" and ",","]}],program:[{op:"set_field",args:{path:"moduleState.preserveRequests",value:true}}],priority:67};
    return {module,diagnostic:{status:"synthesized",requiredOperations:["set_field"],program:module.program}};
  }

  const reference=v520ReferenceStrategyModule(rule,f);if(reference)return {module:reference,diagnostic:{status:"synthesized",requiredOperations:v520Unique(reference.program.map(x=>x.op)),program:reference.program}};

  // Explain *why* synthesis stopped. These labels name low-level missing CPU
  // abilities, not a desired lesson-shaped JavaScript function.
  const missing=[];
  if(f.scopedContext&&f.participantScope)missing.push("scoped_context_binding");
  else if(f.scopedContext)missing.push("scoped_context_policy");
  else if(f.nested)missing.push("nested_semantic_frame_manipulation");
  if(f.reference&&/\brelationship(?:s)?\b/.test(f.t))missing.push("evaluate_arbitrary_relationship_fit");
  if(f.reference&&/\baddressee\b|\blistener can change\b/.test(f.t))missing.push("derive_and_update_discourse_role_from_semantics");
  if(f.ambiguity&&!missing.length)missing.push("candidate_support_measurement");
  if(f.domain==="realization"&&/\breference-resolution check\b/.test(f.t))missing.push("cross_phase_precondition_check");
  /* Refine the old catch-all diagnostic into reusable cognitive capability
     families. These describe what kind of state operation is missing; they do
     not name a lesson, prompt, or desired answer. */
  if(!missing.length&&/\b(?:concept|meaning|definition|stored knowledge|learned knowledge)\b/.test(f.t)&&/\b(?:use|retrieve|find|connect|compose|combine|reason|explain|answer|apply|understand)\b/.test(f.t))missing.push("concept_graph_navigation");
  if(!missing.length&&/\b(?:label|labeled|labelled|teaching role|item role)\b/.test(f.t)&&/\b(?:read|use|determine|authoritative|role)\b/.test(f.t))missing.push("structured_role_access");
  if(!missing.length&&/\b(?:order|ordered|sequence|steps?)\b/.test(f.t)&&/\b(?:preserve|keep|separate|maintain|matter)\b/.test(f.t))missing.push("ordered_structure_preservation");
  if(!missing.length&&/\b(?:contradiction|conflict|support|evidence)\b/.test(f.t)&&/\b(?:check|compare|evaluate|accept|reject)\b/.test(f.t))missing.push("support_conflict_evaluation");
  if(!missing.length&&/\b(?:stored|compiled|applied|executable|execution state|status)\b/.test(f.t)&&/\b(?:report|inspect|check|distinguish)\b/.test(f.t))missing.push("cognitive_status_inspection");
  if(!missing.length&&/\b(?:original utterance|source wording|source evidence|exact source)\b/.test(f.t)&&/\b(?:read|keep|retrieve|preserve|available)\b/.test(f.t))missing.push("source_evidence_access");
  if(!missing.length)missing.push("unmapped_cognitive_state_transformation");
  return {module:null,diagnostic:{status:"missing_fundamental_capability",reason:"no safe general program can yet implement the learned state transformation",requiredOperations:[],missingOperations:v520Unique(missing)}};
}


/* ---------------------------------------------------------
   OPEN SYNTHESIS STRATEGY REGISTRY

   Current synthesis strategies cooperate through one dispatcher instead of
   replacing/wrapping v520SynthesizeRule.  Strategies can add support for new
   learned transformations while the synthesizer remains one current system.
   Strategy names are implementation organization only; learned category names
   remain arbitrary data and do not need to be registered here.
   --------------------------------------------------------- */
const NOVA620_SYNTHESIS_STRATEGIES=[];
function v520RegisterSynthesisStrategy(name,priority,fn){
  if(typeof fn!=="function")return false;
  const key=v520Canonical(name||`strategy_${NOVA620_SYNTHESIS_STRATEGIES.length}`);
  const existing=NOVA620_SYNTHESIS_STRATEGIES.find(x=>x.key===key);
  if(existing){existing.priority=Number(priority||0);existing.fn=fn;}
  else NOVA620_SYNTHESIS_STRATEGIES.push({key,name:String(name||key),priority:Number(priority||0),fn});
  NOVA620_SYNTHESIS_STRATEGIES.sort((a,b)=>b.priority-a.priority);return true;
}
function v520SynthesisAttemptSummary(name,result,note=null){return {strategy:name,status:result?.module?"candidate_module":result?.diagnostic?.status||"no_change",reason:result?.diagnostic?.reason||note||null,missingOperations:v520Clone(result?.diagnostic?.missingOperations||[]),requiredOperations:v520Clone(result?.diagnostic?.requiredOperations||[]),moduleName:result?.module?.name||null};}
function v520SynthesizeRule(rule){
  let result=v520KernelSynthesizeRule(rule)||{module:null,diagnostic:null};const attempts=[v520SynthesisAttemptSummary("general synthesis kernel",result)];
  for(const strategy of NOVA620_SYNTHESIS_STRATEGIES){
    try{
      const beforeModule=result?.module?.name||null,beforeStatus=result?.diagnostic?.status||null;const next=strategy.fn(rule,result);
      if(next&&typeof next==="object")result=next;
      if((result?.module?.name||null)!==beforeModule||(result?.diagnostic?.status||null)!==beforeStatus)attempts.push(v520SynthesisAttemptSummary(strategy.name,result));
    }catch(error){attempts.push({strategy:strategy.name,status:"strategy_error",reason:error?.message||String(error),missingOperations:[],requiredOperations:[],moduleName:null});console.warn(`Nova synthesis strategy ${strategy.name} failed:`,error);}
  }
  result ||= {module:null,diagnostic:null};result.diagnostic={...(result.diagnostic||{}),attempts};
  if(result?.module){result.module.categories=v520Unique([...(result.module.categories||[]),result.module.category,result.module.domain,result.module.phase,result.module.cognitiveForm].filter(Boolean));}
  return result;
}

/* Learned programs are themselves reusable synthesis capabilities.  A later
   lesson can explicitly name or categorize an existing program and invoke it,
   allowing new learning to make older Stored Only guidance executable without
   adding a JavaScript operation for that lesson. */
function v520ReusableModuleFor(label){
  const wanted=v520Canonical(label);if(!wanted||typeof v502ModuleStore!=="function")return null;
  return (v502ModuleStore().modules||[]).find(m=>{if(!m.enabled)return false;if(wanted.startsWith("cognitive_form:"))return v520Canonical(m.cognitiveForm||"")===wanted.slice("cognitive_form:".length);if(wanted.startsWith("learned_category:"))return (m.categories||[]).some(v=>v520Canonical(v)===wanted.slice("learned_category:".length));return [m.id,m.name,m.domain,m.phase,m.cognitiveForm,...(m.categories||[]),...(m.capabilities||[]),...(m.produces||[])].some(v=>v520Canonical(v)===wanted);})||null;
}
function v520ReusableProgramStrategy(rule,current){
  if(current?.module||!rule)return current;
  /* Reuse is intentionally exact. Broad labels such as "general", "reasoning",
     or a lesson scope do not prove that another module implements this rule.
     Earlier V6.2 builds could falsely mark cognition executable through those
     broad matches. A reusable program must now be explicitly named by a learned
     primitive/category or exactly satisfy a recorded missing capability. */
  const candidates=[];
  const primitive=v520Canonical(rule.primitive||"");
  if(primitive&&primitive!=="learned guidance"&&primitive!=="learned_guidance")candidates.push(rule.primitive);
  if(rule.learnedCategory)candidates.push(rule.learnedCategory);
  for(const missing of [...(rule?.synthesis?.missingOperations||[]),...(rule?.synthesis?.missingCapabilities||[])])if(missing&&v520Canonical(missing)!=="unmapped cognitive state transformation"&&v520Canonical(missing)!=="unmapped_cognitive_state_transformation")candidates.push(missing);
  for(const label of v520Unique(candidates)){
    const reusable=v520ReusableModuleFor(label);if(!reusable)continue;
    const module={
      name:`Composed learned program: ${v520Clean(rule.directive||label).slice(0,80)}`,
      domain:rule.domain||"reasoning",phase:rule.domain||"reasoning",
      categories:v520Unique([rule.domain,rule.learnedCategory,reusable.name,...(reusable.categories||[])].filter(Boolean)),
      conditions:[{type:"always"}],priority:60,
      requires:v520Clone(reusable.requires||[]),produces:v520Clone(reusable.produces||[]),
      program:[{op:"invoke_module",args:{id:reusable.id,respectConditions:true,firstOnly:true}}]
    };
    return {module,diagnostic:{status:"synthesized",reason:"composed from an explicitly matched reusable cognitive program",requiredOperations:["invoke_module"],reusedModuleId:reusable.id,program:module.program}};
  }
  return current;
}
v520RegisterSynthesisStrategy("reuse learned cognitive programs",5,v520ReusableProgramStrategy);

/* ---------------------------------------------------------
   SOURCE-LEVEL CONSTRUCTION SYNTHESIS

   A coherent lesson can teach a transformation across definitions, procedures,
   positive demonstrations, and negative demonstrations. Compiling every line in
   isolation loses that relationship. This source-level pass induces reusable
   construction modules from the lesson's demonstrated input/output structure.

   The induction logic is domain-general: it knows teaching demonstration labels
   (Input, Requested action, Selected content, Output), quoted surface regions,
   positive/negative exemplars, and generic canonical-state operations. It does
   not know any curriculum action such as repeat, paraphrase, explain, remember,
   or any fixed English trigger phrase.
   --------------------------------------------------------- */
function v520StableKey(text){let h=2166136261;for(const ch of String(text||"")){h^=ch.charCodeAt(0);h=Math.imul(h,16777619);}return (h>>>0).toString(36);}
function v520DemoLabel(label){return v520Canonical(label).replace(/[_-]+/g," ").replace(/\s+/g," ").trim();}
const NOVA6210_DEMO_INPUT_LABELS=new Set(["input","speaker","request","prompt","utterance"]);
const NOVA6210_DEMO_ACTION_LABELS=new Set(["requested action","action","expected action"]);
const NOVA6210_DEMO_SELECTED_LABELS=new Set(["selected content","selected linguistic content","target content","selected text"]);
const NOVA6210_DEMO_OUTPUT_LABELS=new Set(["output","response","suitable response","correct output","correct selected output","expected response"]);
const NOVA6210_DEMO_NONINPUT_LABELS=new Set(["original","output","response","suitable response","correct output","correct selected output","expected response","incorrect output","selected content","selected linguistic content","target content","requested action","action","expected action"]);
function v520QuotedParts(text){const raw=String(text||""),out=[];for(const rx of [/"([^"\n]*)"/g,/“([^”\n]*)”/g,/'([^'\n]*)'/g,/‘([^’\n]*)’/g]){let m;while((m=rx.exec(raw)))out.push({text:m[1]??"",start:m.index,end:m.index+m[0].length});}return out.sort((a,b)=>a.start-b.start);}
function v520DemoCue(input){let raw=String(input||"");for(const part of [...v520QuotedParts(raw)].sort((a,b)=>b.start-a.start))raw=raw.slice(0,part.start)+" "+raw.slice(part.end);return v520Canonical(raw).replace(/[^a-z0-9]+/g," ").replace(/\s+/g," ").trim();}
function v520DemoTokens(text){const stop=new Set(["the","a","an","this","that","these","those","to","of","and","or","in","on","for","with","from","as","is","are","be","selected","content","supplied","current","requested","action","words","sentence","request","perform"]);return v520Canonical(text).split(/[^a-z0-9]+/).filter(x=>x.length>2&&!stop.has(x));}
function v520DemoSimilarity(a,b){const A=new Set(v520DemoTokens(a)),B=new Set(v520DemoTokens(b));if(!A.size||!B.size)return 0;let common=0;for(const token of A)if(B.has(token))common++;return common/new Set([...A,...B]).size;}
function v520ParseTeachingDemo(block){
  const raw=String(block?.text||"").trim();if(!raw)return null;const lines=raw.split(/\n+/).map(v520Clean).filter(Boolean),fields={},unlabeled=[];
  for(const line of lines){const m=line.match(/^([A-Za-z][A-Za-z _-]{0,40})\s*:\s*(.*)$/);if(!m){unlabeled.push(line);continue;}const label=v520DemoLabel(m[1]),value=v520Clean(m[2]);if(NOVA6210_DEMO_INPUT_LABELS.has(label)&&!fields.input)fields.input=value;else if(NOVA6210_DEMO_ACTION_LABELS.has(label)&&!fields.action)fields.action=value;else if(NOVA6210_DEMO_SELECTED_LABELS.has(label)&&!fields.selected)fields.selected=value;else if(NOVA6210_DEMO_OUTPUT_LABELS.has(label)&&!fields.output)fields.output=value;else if(!NOVA6210_DEMO_NONINPUT_LABELS.has(label)&&!fields.input&&v520QuotedParts(line).length)fields.input=line;
  }
  if(!fields.input){const first=unlabeled[0]||lines[0]||"",m=first.match(/^([A-Za-z][A-Za-z _-]{0,40})\s*:/),label=m?v520DemoLabel(m[1]):"";if(first&&v520QuotedParts(first).length&&!NOVA6210_DEMO_NONINPUT_LABELS.has(label))fields.input=first;}
  if(!fields.input)return null;const quotes=v520QuotedParts(fields.input),cue=v520DemoCue(fields.input);if(!cue)return null;
  let payload=null;if(fields.selected){const selected=v520Clean(fields.selected).replace(/^['"“”‘’]|['"“”‘’]$/g,"");const match=quotes.find(q=>v520Canonical(q.text)===v520Canonical(selected));if(match)payload=match.text;else payload=selected;}
  if(!payload&&fields.output){const output=v520Clean(fields.output).replace(/^['"“”‘’]|['"“”‘’]$/g,"");const match=quotes.find(q=>v520Canonical(q.text)===v520Canonical(output));if(match)payload=match.text;}
  const output=fields.output?v520Clean(fields.output).replace(/^['"“”‘’]|['"“”‘’]$/g,""):null;
  const outputEqualsPayload=Boolean(output&&payload&&v520Canonical(output)===v520Canonical(payload));
  return {blockId:block.id,role:block.role,scope:block.scope||"general",input:fields.input,cue,action:fields.action||null,selected:fields.selected||null,output,payload,quotedPayload:Boolean(payload&&quotes.some(q=>v520Canonical(q.text)===v520Canonical(payload))),outputEqualsPayload};
}
function v520SourceConstructionReport(source,report){const store=v520SynthesisStore();store.sourceConstructions ||= [];const key=source?.id||v520Canonical(source?.name||"document"),row={sourceId:source?.id||null,sourceName:source?.name||"document",...v520Clone(report),compiledAt:Date.now()};const i=store.sourceConstructions.findIndex(x=>(x.sourceId||v520Canonical(x.sourceName))===key);if(i>=0)store.sourceConstructions[i]=row;else store.sourceConstructions.push(row);store.sourceConstructions=store.sourceConstructions.slice(-80);store.updatedAt=Date.now();return row;}
function v520SynthesizeSourceCognition(source,providedStructure=null,meta={},options={}){
  const storeCandidate=options.store!==false;
  if(!source?.text||(storeCandidate&&typeof v502StoreModule!=="function"))return {status:"skipped",reason:"source_or_module_store_unavailable"};
  const structure=providedStructure||((typeof v534ParseDocumentStructure==="function")?v534ParseDocumentStructure(source.text,{sourceName:source.name||"document"}):null);if(!structure?.blocks)return {status:"skipped",reason:"structured_source_unavailable"};
  const positives=(structure.blocks||[]).filter(b=>b.role==="example").map(v520ParseTeachingDemo).filter(Boolean),negatives=(structure.blocks||[]).filter(b=>b.role==="negative_example").map(block=>({demo:v520ParseTeachingDemo(block),text:v520Canonical(block.text||"")})).filter(row=>row.demo&&/\b(?:not|isn't|is not|rather than|instead of|does not|doesn't)\b/.test(row.text)).map(row=>row.demo);
  const transforms=positives.filter(d=>d.outputEqualsPayload&&d.quotedPayload);if(transforms.length<2)return v520SourceConstructionReport(source,{status:"no_demonstrated_constructor",reason:"fewer than two demonstrations showed a selected quoted payload becoming the response",positiveDemonstrations:positives.length,negativeDemonstrations:negatives.length});
  const scopeCounts=new Map();for(const d of transforms)scopeCounts.set(d.scope,(scopeCounts.get(d.scope)||0)+1);const familyName=[...scopeCounts.entries()].sort((a,b)=>b[1]-a[1])[0]?.[0]||source.name||"learned construction";
  const actionLabels=v520Unique(transforms.map(d=>d.action).filter(Boolean));const actionLabel=v520Clean(actionLabels[0]||familyName).replace(/[.!?]+$/g,"");const familyKey=`learned_family_${v520StableKey(`${source.id||source.name}|${familyName}|payload_to_response`)}`;
  const familyTokens=new Set(v520DemoTokens([familyName,...actionLabels].join(" ")));const anchorCues=transforms.map(d=>d.cue);
  const qualifies=d=>{if(d.outputEqualsPayload)return true;if(d.action&&v520DemoTokens(d.action).some(token=>familyTokens.has(token)))return true;if(v520DemoTokens(d.scope).some(token=>familyTokens.has(token)))return true;return anchorCues.some(cue=>v520DemoSimilarity(cue,d.cue)>=0.5);};
  const positiveCues=v520Unique(positives.filter(d=>v520QuotedParts(d.input).length>0&&qualifies(d)).map(d=>d.cue)).slice(0,80),negativeCues=v520Unique(negatives.filter(d=>v520QuotedParts(d.input).length>0).map(d=>d.cue)).slice(0,80);if(positiveCues.length<2)return v520SourceConstructionReport(source,{status:"no_demonstrated_constructor",reason:"not enough coherent positive surface demonstrations were available",familyName,transformDemonstrations:transforms.length});
  const familyCondition={type:"learned_surface_family",familyId:familyKey,familyName,positiveCues,negativeCues,payloadMode:"quoted_region",captureName:"selectedContent",threshold:0.58,negativeMargin:0.08};
  const requestId=`request_${familyKey}`,goalId=`goal_${familyKey}`;
  const recognizer={
    id:`${familyKey}_recognizer`,name:`Learned construction — ${familyName}`,domain:"semantic",phase:"interpretation",categories:["semantic","language","learned construction",familyName],priority:112,cognitiveForm:"state_transformation",conditions:[familyCondition],
    requires:["surface observation","learned construction exemplars"],produces:["action request","selected linguistic content","action goal"],
    synthesisIntent:{kind:"learned_construction_family",familyId:familyKey,familyName,actionLabel,transform:"selected_payload_to_response",sourceId:source.id||null},
    program:[
      {op:"semantic_add_request",args:{id:requestId,type:"action_request",action:actionLabel,actionFamily:familyKey,constructionFamily:familyName,selectedContent:{kind:"linguistic_content",text:{fromCapture:"selectedContent"},scopeId:{fromCapture:"selectedContentScopeId"},source:"current_utterance"},evidence:{matchedCue:{fromCapture:"matchedCue"},sourceName:source.name||"document"}}},
      {op:"create_structure",args:{value:{id:`content_${familyKey}`,kind:"selected_linguistic_content",category:familyName,text:{fromCapture:"selectedContent"},scopeId:{fromCapture:"selectedContentScopeId"},source:"current_utterance"},unique:true}},
      {op:"resolve_goal",args:{id:"goal_interpret_current_utterance"}},
      {op:"create_goal",args:{goal:{id:goalId,type:"action_request",action:actionLabel,actionFamily:familyKey,requestId,description:`perform learned action: ${actionLabel}`}}},
      {op:"activate_category",args:{name:familyName}}
    ]
  };
  const performer={
    id:`${familyKey}_performer`,name:`Learned action realization — ${familyName}`,domain:"behavior",phase:"realization",categories:["behavior","realization","learned construction",familyName],priority:106,cognitiveForm:"behavior_policy",conditions:[familyCondition,{type:"list_item_matches",listPath:"canonicalSemanticState.requests",match:{actionFamily:familyKey}}],
    requires:["learned action request","selected linguistic content"],produces:["intended response content"],
    synthesisIntent:{kind:"learned_construction_family",familyId:familyKey,familyName,actionLabel,transform:"selected_payload_to_response",sourceId:source.id||null},
    program:[
      {op:"select_matching_item",args:{listPath:"canonicalSemanticState.requests",match:{actionFamily:familyKey},outputPath:"moduleState.activeLearnedRequest"}},
      {op:"copy_field",args:{fromPath:"moduleState.activeLearnedRequest.selectedContent.text",toPath:"moduleState.intendedResponseContent"}},
      {op:"emit_surface",args:{text:{fromPath:"moduleState.intendedResponseContent"}}},
      {op:"resolve_goal",args:{id:goalId}}
    ]
  };
  if(!storeCandidate){
    return {...v520SourceConstructionReport(source,{status:"candidate_modules",familyId:familyKey,familyName,actionLabel,transform:"selected_payload_to_response",positiveDemonstrations:positiveCues.length,negativeDemonstrations:negativeCues.length,anchorDemonstrations:transforms.length,positiveCues,negativeCues}),candidateModules:[recognizer,performer]};
  }
  const stored=[];for(const module of [recognizer,performer]){const result=v502StoreModule(module,{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null,text:`source-level learned construction: ${familyName}`});if(!result?.stored)return v520SourceConstructionReport(source,{status:"synthesis_rejected",reason:(result?.validation?.errors||["module_store_rejected"]).join(", "),familyName,familyId:familyKey});stored.push(result.module);}
  return v520SourceConstructionReport(source,{status:"synthesized",familyId:familyKey,familyName,actionLabel,transform:"selected_payload_to_response",positiveDemonstrations:positiveCues.length,negativeDemonstrations:negativeCues.length,anchorDemonstrations:transforms.length,moduleIds:stored.map(m=>m.id),moduleNumbers:stored.map(m=>m.moduleNumber),positiveCues,negativeCues});
}

function v520AttemptRuleSynthesis(rule){
  const attempt=v520SynthesizeRule(rule);const available=[...(typeof NOVA502_MODULE_OPS==="object"?NOVA502_MODULE_OPS:[])];
  if(!attempt.module){
    const d=v520Diagnostic(rule,attempt.diagnostic?.status||"missing_fundamental_capability",{...(attempt.diagnostic||{}),availableOperations:available});
    return {upgraded:false,rule,module:null,diagnostic:d};
  }
  const resolvedCapabilities=v520Unique([...(attempt.diagnostic?.resolvedMissingOperations||[]),...(attempt.diagnostic?.capabilities||[])]).filter(cap=>!new Set(["unmapped cognitive state transformation","unmapped_cognitive_state_transformation"]).has(v520Canonical(cap)));
  if(resolvedCapabilities.length){attempt.module.capabilities=v520Unique([...(attempt.module.capabilities||[]),...resolvedCapabilities]);attempt.module.produces=v520Unique([...(attempt.module.produces||[]),...resolvedCapabilities]);}
  const validation=typeof v502ValidateModule==="function"?v502ValidateModule(attempt.module):{valid:false,errors:["validator_unavailable"]};
  if(!validation.valid){
    const missing=validation.errors.filter(e=>/^unsupported_op:/.test(e)).map(e=>e.split(":")[1]);
    const d=v520Diagnostic(rule,"missing_fundamental_capability",{...(attempt.diagnostic||{}),reason:validation.errors.join(", "),missingOperations:missing,availableOperations:available,program:attempt.module.program});
    return {upgraded:false,rule,module:null,diagnostic:d};
  }
  const stored=typeof v502StoreModule==="function"?v502StoreModule(attempt.module,{sourceType:"learned",sourceName:rule.sourceName,sourceId:rule.sourceId,text:rule.directive}):{stored:false};
  if(!stored.stored){
    const d=v520Diagnostic(rule,"synthesis_rejected",{...(attempt.diagnostic||{}),reason:(stored.validation?.errors||[]).join(", ")||"module_store_rejected",availableOperations:available,program:attempt.module.program});
    return {upgraded:false,rule,module:null,diagnostic:d};
  }
  rule.status="module_executable";rule.moduleId=stored.module.id;rule.updatedAt=Date.now();
  const d=v520Diagnostic(rule,"synthesized",{...(attempt.diagnostic||{}),moduleId:stored.module.id,availableOperations:available,program:attempt.module.program});
  return {upgraded:true,rule,module:stored.module,diagnostic:d};
}

function v520GeneralSynthesisPass(options={}){
  if(window.__TAURI__?.core?.invoke)return {skipped:true,authority:"rust_v7_pass_8",reason:"Native cognitive synthesis is Rust-owned; the JavaScript synthesizer is browser-only."};
  const inv=typeof v500CognitiveRuleInventory==="function"?v500CognitiveRuleInventory():{rules:[]},rules=inv.rules||[],resultByRule=new Map();let upgraded=0,artifacts=0,demotedFalseReuse=0;
  const vocabulary=v520VocabularySignature(),maxRounds=Math.max(1,Math.min(8,Number(options.maxRounds||5)));

  /* Remove only the obsolete broad-reuse products.  Useful current modules stay
     authoritative; their capabilities can participate in the fixed-point pass. */
  for(const rule of rules){
    if(rule.sourceType!=="learned"||!["executable","module_executable"].includes(rule.status))continue;
    const obsoleteBroadReuse=Number(rule?.synthesis?.version||0)<NOVA520_SYNTHESIS_SCHEMA_VERSION&&v520Canonical(rule?.synthesis?.reason||"")==="composed from an already learned reusable cognitive program";
    if(!obsoleteBroadReuse)continue;
    try{if(rule.moduleId&&typeof v502ModuleStore==="function"){const ms=v502ModuleStore(),before=ms.modules.length;ms.modules=ms.modules.filter(module=>module.id!==rule.moduleId);if(ms.modules.length!==before){ms.referenceIndex=null;if(typeof v622RebuildReferenceIndex==="function")v622RebuildReferenceIndex(ms);}}}catch(_){ }
    rule.status="stored_only";rule.moduleId=null;rule.updatedAt=Date.now();demotedFalseReuse++;
  }

  const roundReports=[];
  for(let round=1;round<=maxRounds;round++){
    let roundUpgraded=0,roundScanned=0;
    for(const rule of rules){
      if(rule.sourceType!=="learned"||["executable","module_executable"].includes(rule.status))continue;
      if(round===1&&!options.force&&rule.synthesis?.version===NOVA520_SYNTHESIS_SCHEMA_VERSION&&rule.synthesis?.vocabularySignature===vocabulary){resultByRule.set(rule.id,{ruleId:rule.id,directive:rule.directive,status:rule.synthesis.status,missingOperations:rule.synthesis.missingOperations||[],moduleId:rule.moduleId||null,skipped:true});continue;}
      roundScanned++;
      const result=v520AttemptRuleSynthesis(rule),row={ruleId:rule.id,directive:rule.directive,status:result.diagnostic?.status,missingOperations:result.diagnostic?.missingOperations||[],requiredOperations:result.diagnostic?.requiredOperations||[],moduleId:result.module?.id||null,round};resultByRule.set(rule.id,row);
      if(result.upgraded){upgraded++;roundUpgraded++;}
      if(result.diagnostic?.status==="teaching_artifact")artifacts++;
    }
    roundReports.push({round,scanned:roundScanned,upgraded:roundUpgraded,moduleCount:typeof v502ModuleStore==="function"?(v502ModuleStore().modules||[]).length:null});
    if(roundUpgraded===0)break;
  }

  let removedTeachingArtifacts=0;
  if(artifacts&&typeof v500CognitiveStore==="function"){
    const cognitive=v500CognitiveStore(),artifactIds=new Set([...resultByRule.values()].filter(r=>r.status==="teaching_artifact").map(r=>r.ruleId)),before=cognitive.rules.length;
    cognitive.rules=cognitive.rules.filter(r=>!artifactIds.has(r.id));removedTeachingArtifacts=before-cognitive.rules.length;if(removedTeachingArtifacts)cognitive.updatedAt=Date.now();
  }
  const currentRules=typeof v500CognitiveRuleInventory==="function"?(v500CognitiveRuleInventory().rules||[]):rules,remainingRules=currentRules.filter(rule=>rule.sourceType==="learned"&&!["executable","module_executable"].includes(rule.status));
  const results=[...resultByRule.values()];
  const capabilityCounts={};for(const rule of remainingRules)for(const missing of v520RuleCapabilityNeeds(rule)){const key=v520Canonical(missing||"unknown")||"unknown";capabilityCounts[key]=(capabilityCounts[key]||0)+1;}
  const capabilityGraph=v520RebuildCapabilityGraph(currentRules);
  const report={version:NOVA520_VERSION,scanned:results.length,upgraded,demotedFalseReuse,teachingArtifacts:artifacts,removedTeachingArtifacts,remaining:remainingRules.length,rounds:roundReports,missingCapabilityCounts:capabilityCounts,capabilityGraphSummary:{signature:capabilityGraph.signature,producerCapabilityCount:capabilityGraph.producerCapabilityCount,waitingCapabilityCount:capabilityGraph.waitingCapabilityCount},results,ranAt:Date.now(),reason:options.reason||"general_synthesis_pass"};
  const store=v520SynthesisStore();store.lastPass=report;store.history.push(report);store.history=store.history.slice(-30);store.updatedAt=Date.now();return report;
}

function v520ReconsiderCapabilities(capabilities=[],options={}){
  const wanted=new Set(v520Unique(capabilities).map(v520Canonical).filter(Boolean));if(!wanted.size)return {scanned:0,upgraded:0,reason:"no_capabilities"};const inv=typeof v500CognitiveRuleInventory==="function"?v500CognitiveRuleInventory():{rules:[]};let scanned=0,upgraded=0;
  for(const rule of inv.rules||[]){if(rule.sourceType!=="learned"||["executable","module_executable"].includes(rule.status))continue;const needs=v520RuleCapabilityNeeds(rule);if(!needs.some(need=>wanted.has(v520Canonical(need))))continue;scanned++;const result=v520AttemptRuleSynthesis(rule);if(result.upgraded)upgraded++;}
  const graph=v520RebuildCapabilityGraph(inv.rules||[]);return {scanned,upgraded,capabilities:[...wanted],graph:{producerCapabilityCount:graph.producerCapabilityCount,waitingCapabilityCount:graph.waitingCapabilityCount},reason:options.reason||"capability_reconsideration",ranAt:Date.now()};
}

/* V6.2 synthesis is invoked explicitly by the current learning path and by
   v502UpgradeStoredGuidance(). It does not replace lesson, memory, semantic,
   validation, or executive functions from older architectures. */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA520_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const id="general_cognitive_synthesis",entry={id,verb:"synthesize",object:"new declarative cognitive programs from learned guidance and reusable mental operations",detail:"I compose generic cognitive operations and previously learned programs. Missing capability diagnostics remain explicit when the current substrate cannot yet express a learned transformation."};
    const existing=V41_SELF_MODEL.capabilities.find(c=>c.id===id);if(existing)Object.assign(existing,entry);else V41_SELF_MODEL.capabilities.push(entry);
  }
}catch(_){ }

/* Run one migration pass after the inherited semantic-composition primitives are present. */
try{if(!window.__TAURI__?.core?.invoke)v520GeneralSynthesisPass({reason:"browser_startup_reconsideration",force:true});}catch(_){ }
