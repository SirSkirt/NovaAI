"use strict";

/* =========================================================
   Nova V7 — RUST MIND BRIDGE (PASS 8)

   Rust owns semantic/learning cognition, developmental experience reflection, category mutation, consolidation, internal goals/free thought, teaching intent, and response realization.
   JavaScript is a UI/persistence mirror at this boundary. It may bootstrap legacy state once, but it never replaces initialized Rust cognitive authority.
   ========================================================= */

const nova7RustMindBridge = {
  available: false,
  initialized: false,
  activeTurnId: null,
  activeRawText: null,
  activeCanonicalState: null,
  activeSemanticReport: null,
  activeCognitiveReport: null,
  activeRealizationReport: null,
  activeEventKind: null,
  snapshot: null,
  lastReceipt: null,
  learningReceipt: null,
  lastError: null,
  updatedAt: null
};

function nova7RustMindInvokeAvailable(){
  return Boolean(window.__TAURI__?.core?.invoke);
}

function nova7RustMindClone(value){
  try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(__){return value;}}
}

function nova7RustMindLegacyId(prefix,value){
  let h=2166136261;for(const ch of String(value||"")){h^=ch.charCodeAt(0);h=Math.imul(h,16777619);}
  return `${prefix}_${(h>>>0).toString(36)}`;
}

function nova7RustMindStatus(){
  return {
    available: nova7RustMindBridge.available,
    initialized: nova7RustMindBridge.initialized,
    activeTurnId: nova7RustMindBridge.activeTurnId,
    activeSemanticReport: nova7RustMindClone(nova7RustMindBridge.activeSemanticReport),
    activeCognitiveReport: nova7RustMindClone(nova7RustMindBridge.activeCognitiveReport),
    activeRealizationReport: nova7RustMindClone(nova7RustMindBridge.activeRealizationReport),
    activeEventKind: nova7RustMindBridge.activeEventKind,
    snapshot: nova7RustMindBridge.snapshot,
    lastReceipt: nova7RustMindBridge.lastReceipt,
    learningReceipt: nova7RustMindClone(nova7RustMindBridge.learningReceipt),
    lastError: nova7RustMindBridge.lastError,
    updatedAt: nova7RustMindBridge.updatedAt
  };
}

function nova7RustMindKnowledgeSnapshot(){
  try{
    if(typeof v450KnowledgeStore!=="function")return {concepts:[],claims:[],sourceRevision:null};
    const store=v450KnowledgeStore()||{};
    return {
      concepts:(store.concepts||[]).map(concept=>({
        id:String(concept.id||""),
        key:String(concept.key||""),
        name:String(concept.name||concept.key||""),
        aliases:(concept.aliases||[]).map(String),
        kinds:(concept.kinds||[]).map(String),
        domainKeys:(concept.domainKeys||[]).map(String),
        evidence:nova7RustMindClone(concept.evidence||[]),
        subjectEligible:Boolean(concept.subjectEligible),
        instructional:Boolean(concept.instructional),
        createdAt:Number(concept.createdAt||0)||null,
        updatedAt:Number(concept.updatedAt||0)||null
      })).filter(concept=>concept.id&&concept.key),
      claims:(store.claims||[]).map(claim=>({
        id:String(claim.id||""),
        subjectId:String(claim.subjectId||""),
        subjectKey:String(claim.subjectKey||""),
        subject:String(claim.subject||""),
        predicate:String(claim.predicate||""),
        object:String(claim.object??""),
        objectConceptId:claim.objectConceptId?String(claim.objectConceptId):null,
        polarity:String(claim.polarity||"positive"),
        confidence:Number(claim.confidence??0.5),
        qualifiers:nova7RustMindClone(claim.qualifiers||{}),
        evidence:nova7RustMindClone(claim.evidence||[])
      })).filter(claim=>claim.id&&claim.subjectId&&claim.predicate),
      guidance:(store.guidance||[]).map(item=>({
        id:String(item.id||""),domain:String(item.domain||"general"),scope:String(item.scope||"local"),
        target:String(item.target||"general"),strength:String(item.strength||"recommended"),
        polarity:String(item.polarity||"positive"),directive:String(item.directive||""),
        conditions:nova7RustMindClone(item.conditions||[]),evidence:nova7RustMindClone(item.evidence||[])
      })).filter(item=>item.id&&item.directive),
      examples:(store.examples||[]).map(item=>({
        id:String(item.id||""),demonstratesKey:item.demonstratesKey?String(item.demonstratesKey):null,
        content:String(item.content||""),sequence:nova7RustMindClone(item.sequence||[]),
        evidence:nova7RustMindClone(item.evidence||[])
      })).filter(item=>item.id&&item.content),
      procedures:(store.procedures||[]).map(item=>({
        id:String(item.id||""),conceptId:item.conceptId?String(item.conceptId):null,
        conceptKey:String(item.conceptKey||""),name:String(item.name||""),
        interactionMode:String(item.interactionMode||"single_turn"),roles:(item.roles||[]).map(String),
        sequence:nova7RustMindClone(item.sequence||[]),requirements:(item.requirements||[]).map(String),
        mechanisms:(item.mechanisms||[]).map(String),optionalMechanisms:(item.optionalMechanisms||[]).map(String),
        purpose:item.purpose?String(item.purpose):null,confidence:Number(item.confidence??0.86),
        evidence:nova7RustMindClone(item.evidence||[])
      })).filter(item=>item.id&&item.name),
      sourceRevision:Number(store.updatedAt||0)||null
    };
  }catch(error){
    console.warn("Nova V7 could not bridge semantic memory into Rust:",error);
    return {concepts:[],claims:[],sourceRevision:null};
  }
}


function nova7RustNormalizeModule(module, fallback={}){
  if(!module||typeof module!=="object")return null;
  const id=String(module.id||fallback.id||"").trim();if(!id)return null;
  return {
    id,
    moduleNumber:Number.isInteger(module.moduleNumber)?module.moduleNumber:(Number.isInteger(fallback.moduleNumber)?fallback.moduleNumber:null),
    name:String(module.name||fallback.name||id),
    domain:String(module.domain||fallback.domain||"reasoning"),
    phase:String(module.phase||fallback.phase||module.domain||"reasoning"),
    categories:(module.categories||fallback.categories||[]).map(String),
    conditions:nova7RustMindClone(module.conditions||fallback.conditions||[{type:"always"}]),
    program:nova7RustMindClone(module.program||fallback.program||[]),
    requires:(module.requires||fallback.requires||[]).map(String),
    produces:(module.produces||fallback.produces||[]).map(String),
    capabilities:(module.capabilities||fallback.capabilities||[]).map(String),
    priority:Number(module.priority??fallback.priority??0),
    enabled:module.enabled!==false,
    sourceType:String(module.sourceType||fallback.sourceType||"learned"),
    sourceName:String(module.sourceName||fallback.sourceName||"conversation"),
    sourceId:module.sourceId?String(module.sourceId):(fallback.sourceId?String(fallback.sourceId):null),
    cognitiveForm:module.cognitiveForm?String(module.cognitiveForm):(fallback.cognitiveForm?String(fallback.cognitiveForm):null),
    evidence:nova7RustMindClone(module.evidence||fallback.evidence||[])
  };
}

function nova7RustMindCognitionSnapshot(){
  try{
    const store=typeof v502ModuleStore==="function"?v502ModuleStore():null;if(!store)return {modules:[],capabilityGraph:{producers:[],waiters:[]},experienceRoutes:[],sourceRevision:null};
    const modules=(store.modules||[]).map(module=>nova7RustNormalizeModule(module)).filter(Boolean);
    const experienceRoutes=(store.experienceIndex?.routes||[]).map(route=>({id:String(route.id||""),conceptIds:[],conceptTerms:(route.conceptTerms||[]).map(String),goalTerms:(route.goalTerms||[]).map(String),relationTerms:(route.relationTerms||[]).map(String),fallbackTerms:(route.fallbackTerms||route.topicTerms||[]).map(String),moduleNumbers:(route.moduleNumbers||[]).map(Number).filter(Number.isFinite),moduleStrengths:nova7RustMindClone(route.moduleStrengths||{}),contributors:[],successes:Number(route.successes||0),failures:Number(route.failures||0),confidence:Number(route.confidence??0.55)})).filter(route=>route.id);
    return {modules,capabilityGraph:{producers:[],waiters:[]},experienceRoutes,sourceRevision:Number(store.updatedAt||0)||null};
  }catch(error){console.warn("Nova V7 could not bridge learned cognitive modules into Rust:",error);return {modules:[],capabilityGraph:{producers:[],waiters:[]},experienceRoutes:[],sourceRevision:null};}
}

function nova7RustMindLearningRules(){
  try{
    const inventory=typeof v500CognitiveRuleInventory==="function"?v500CognitiveRuleInventory():{rules:[]};
    return (inventory.rules||[]).filter(rule=>rule?.sourceType==="learned").map(rule=>({
      id:String(rule.id||""),directive:String(rule.directive||""),domain:String(rule.domain||"general"),primitive:String(rule.primitive||"learned_guidance"),scope:String(rule.scope||"general"),evidenceRole:String(rule.evidenceRole||"instruction"),
      cognitiveForm:rule.cognitiveForm?String(rule.cognitiveForm):null,learnedCategory:rule.learnedCategory?String(rule.learnedCategory):null,categories:(rule.categories||[]).map(String),strength:String(rule.strength||"recommended"),polarity:String(rule.polarity||"positive"),
      status:String(rule.status||"stored_only"),moduleId:rule.moduleId?String(rule.moduleId):null,sourceType:String(rule.sourceType||"learned"),sourceName:String(rule.sourceName||"conversation"),sourceDisplayName:rule.sourceDisplayName?String(rule.sourceDisplayName):null,sourceKind:rule.sourceKind?String(rule.sourceKind):null,sourceLessonNumber:Number.isFinite(Number(rule.sourceLessonNumber))?Number(rule.sourceLessonNumber):null,sourceId:rule.sourceId?String(rule.sourceId):null,
      evidence:nova7RustMindClone(rule.evidence||[]),parameters:nova7RustMindClone(rule.parameters||{}),synthesis:nova7RustMindClone(rule.synthesis||{}),learnedAt:Number(rule.learnedAt||0)||null,updatedAt:Number(rule.updatedAt||0)||null
    })).filter(rule=>rule.id&&rule.directive);
  }catch(error){console.warn("Nova V7 could not bridge learned cognitive rules into Rust:",error);return [];}
}

function nova7RustMindSynthesisCandidates(rules){
  const out=[];
  for(const rule of rules||[]){
    if(["module_executable","executable"].includes(String(rule.status||"")))continue;
    let attempt=null;
    try{
      // Migration frontend only: the legacy teaching translator may propose a
      // declarative candidate, but it does not store it or decide executability.
      // Rust Pass 6 is the authority that validates and adopts/rejects it.
      const original=(typeof v500CognitiveRuleInventory==="function"?v500CognitiveRuleInventory().rules:[]).find(row=>row.id===rule.id);
      if(original&&typeof v520SynthesizeRule==="function")attempt=v520SynthesizeRule(original);
    }catch(error){attempt={module:null,diagnostic:{status:"candidate_translation_error",reason:String(error?.message||error)}};}
    let module=null;
    if(attempt?.module){
      module=nova7RustNormalizeModule(attempt.module,{
        id:`rust_candidate_${rule.id}`,
        name:`Synthesized cognition for ${rule.id}`,
        domain:rule.domain,phase:rule.domain,categories:rule.categories,
        sourceType:"learned",sourceName:rule.sourceName,sourceId:rule.sourceId,cognitiveForm:rule.cognitiveForm,
        evidence:rule.evidence
      });
    }
    out.push({ruleId:rule.id,module,diagnostic:nova7RustMindClone(attempt?.diagnostic||rule.synthesis||{})});
  }
  return out;
}


function nova7RustMindSourceCandidates(){
  const modules=[];
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;
    for(const source of memory?.sources||[]){
      try{
        const instructional=typeof v534InstructionSource==="function"?v534InstructionSource(source):/lesson|curriculum|teaching/i.test(String(source?.name||""));
        if(!instructional||!source?.text||typeof v520SynthesizeSourceCognition!=="function")continue;
        const structure=typeof v534ParseDocumentStructure==="function"?v534ParseDocumentStructure(source.text,{sourceName:source.name||"document",sourceId:source.id||null}):null;
        const proposed=v520SynthesizeSourceCognition(source,structure,{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null},{store:false});
        for(const module of proposed?.candidateModules||[]){const normalized=nova7RustNormalizeModule(module,{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null});if(normalized)modules.push(normalized);}
      }catch(error){console.warn("Nova V7 could not translate one source-level construction candidate:",error);}
    }
  }catch(error){console.warn("Nova V7 could not bridge source-level synthesis candidates:",error);}
  return modules;
}

function nova7RustMindPersistedLearningState(){
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;
    const persisted=memory?.rustLearningState;
    return persisted?.initialized?nova7RustMindClone(persisted):null;
  }catch(_){return null;}
}

function nova7RustMindLearningSyncInput(reason="pass6_sync"){
  const cognition=nova7RustMindCognitionSnapshot();
  // Pass 8: initialized Rust learning state remains authoritative; JavaScript proposes no live brain data.
  // Experience routes are now reflected and consolidated by Rust as well.
  if(nova7RustMindBridge.snapshot?.learningState?.initialized){
    return {rules:[],existingModules:[],candidates:[],sourceCandidates:[],experienceRoutes:[],sourceRevision:null,reason};
  }
  // Prefer a verbatim persisted Rust state from a prior Pass 6 run. JavaScript
  // stores this as data only; Rust reconstructs and revalidates its own runtime.
  const persisted=nova7RustMindPersistedLearningState();
  if(persisted){
    // Opaque persistence transport: do not unpack Rust cognition into a new JS
    // proposal catalog. Rust rehydrates and revalidates its own state.
    return {persistedState:nova7RustMindClone(persisted),rules:[],existingModules:[],candidates:[],sourceCandidates:[],experienceRoutes:nova7RustMindClone(persisted.experienceRoutes||[]),sourceRevision:persisted.sourceRevision??null,reason:`${reason}:restore_rust_learning_state`};
  }
  // Pass 5 and older upgrade only: import the legacy JS stores once. After this
  // succeeds, Pass 6 persists the Rust state and never asks JS to regenerate it.
  const rules=nova7RustMindLearningRules();
  return {rules,existingModules:cognition.modules,candidates:nova7RustMindSynthesisCandidates(rules),sourceCandidates:nova7RustMindSourceCandidates(),experienceRoutes:cognition.experienceRoutes,sourceRevision:cognition.sourceRevision,reason:`${reason}:one_time_legacy_js_import`};
}

function nova7RustMindApplyLearningMirror(receipt){
  const rustState=receipt?.state;if(!rustState)return;
  try{
    const cognitive=typeof v500CognitiveStore==="function"?v500CognitiveStore():null;
    if(cognitive){
      const byId=new Map((rustState.rules||[]).map(rule=>[String(rule.id),rule]));
      for(const rule of cognitive.rules||[]){const authoritative=byId.get(String(rule.id));if(!authoritative)continue;rule.status=authoritative.status||"stored_only";rule.moduleId=authoritative.moduleId||null;rule.synthesis=nova7RustMindClone(authoritative.synthesis||{});rule.updatedAt=Date.now();}
      cognitive.lastCompilation={...(cognitive.lastCompilation||{}),rustAuthority:"rust_v7_pass_8",rustSynthesis:nova7RustMindClone(receipt.report||null),updatedAt:Date.now()};cognitive.updatedAt=Date.now();
    }
    const modules=typeof v502ModuleStore==="function"?v502ModuleStore():null;
    if(modules){modules.modules=nova7RustMindClone(rustState.modules||[]);modules.referenceIndex=null;modules.updatedAt=Date.now();if(typeof v622RebuildReferenceIndex==="function")v622RebuildReferenceIndex(modules);}
    const memory=typeof v43Memory==="function"?v43Memory():null;
    if(memory){
      // Persistence mirror only. This is not a JS cognitive store and is never
      // executed by JavaScript. It preserves Rust-generated candidates across
      // process restarts until full persistence ownership migrates to Rust.
      memory.rustLearningState=nova7RustMindClone(rustState);
      memory.cognitiveSynthesis ||= {};memory.cognitiveSynthesis.version=7;memory.cognitiveSynthesis.authority="rust_v7_pass_8";memory.cognitiveSynthesis.capabilityGraph=nova7RustMindClone(rustState.capabilityGraph||{});memory.cognitiveSynthesis.lastPass=nova7RustMindClone(receipt.report||null);memory.cognitiveSynthesis.updatedAt=Date.now();
    }
  }catch(error){console.warn("Nova V7 could not mirror Rust learning state for persistence/UI:",error);}
}

async function nova7RustMindSyncLearning(){
  throw new Error("Pass 9 retired JavaScript learning synchronization. Rust NovaMind owns learning state.");
}


function nova7RustMindLegacySemanticSources(){
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;
    return (memory?.sources||[]).filter(source=>source?.id&&source?.text).map(source=>({
      id:String(source.id),name:String(source.name||"document"),displayName:String(source.displayName||source.name||"Document"),
      sourceType:String(source.sourceType||"learned"),sourceRole:String(source.sourceRole||source.classification?.type||"knowledge"),
      documentKind:String(source.documentKind||"document"),lessonNumber:Number.isFinite(Number(source.lessonNumber))?Number(source.lessonNumber):null,
      text:String(source.text||""),learnedAt:Number(source.learnedAt||source.createdAt||Date.now())
    }));
  }catch(_){return [];}
}

function nova7RustMindPersistedSemanticState(){
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;
    const persisted=memory?.rustSemanticMemoryState;
    return persisted&&persisted.initialized?nova7RustMindClone(persisted):null;
  }catch(_){return null;}
}

function nova7RustMindApplySemanticMirror(rustState){
  if(!rustState)return;
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return;
    const knowledge=rustState.knowledge||{};
    // Exact Rust state mirror for durable restart recovery. JavaScript stores
    // these bytes as payload data; it does not execute or reinterpret them.
    memory.rustSemanticMemoryState=nova7RustMindClone(rustState);
    memory.knowledgeStore ||= {};
    const store=memory.knowledgeStore;
    store.version=typeof NOVA450_SCHEMA_VERSION!=="undefined"?NOVA450_SCHEMA_VERSION:(store.version||7);
    store.authority="rust_v7_pass_8_mirror";
    store.concepts=nova7RustMindClone(knowledge.concepts||[]);
    store.claims=nova7RustMindClone(knowledge.claims||[]);
    store.guidance=nova7RustMindClone(knowledge.guidance||[]);
    store.examples=nova7RustMindClone(knowledge.examples||[]);
    store.procedures=nova7RustMindClone(knowledge.procedures||[]);
    store.learnedSourceIds=nova7RustMindClone(rustState.learnedSourceIds||[]);
    store.lastLearningReport=nova7RustMindClone(rustState.lastLearning||null);
    store.updatedAt=Number(rustState.updatedAt||Date.now());
    for(const source of rustState.sources||[]){
      const existing=(memory.sources||[]).find(row=>String(row.id)===String(source.id));
      const mirror={id:String(source.id),name:String(source.name||"document"),displayName:String(source.displayName||source.name||"Document"),text:String(source.text||""),sourceType:String(source.sourceType||"learned"),sourceRole:String(source.sourceRole||"instruction"),documentKind:String(source.documentKind||"document"),lessonNumber:source.lessonNumber??null,learnedAt:Number(source.learnedAt||Date.now()),rustAuthority:"rust_v7_pass_8"};
      if(existing)Object.assign(existing,mirror);else{memory.sources ||= [];memory.sources.push(mirror);}
    }
  }catch(error){console.warn("Nova V7 could not mirror Rust semantic memory for persistence/UI:",error);}
}

async function nova7RustMindImportSemanticMemory(){
  throw new Error("Pass 9 retired JavaScript semantic-memory import. Rust NovaMind owns semantic memory hydration.");
}


async function nova7RustMindParseSource(source,{kindHint=null,sourceType="learned"}={}){
  if(!nova7RustMindBridge.available||!nova7RustMindBridge.initialized)throw new Error("Rust NovaMind is required for teaching-source parsing.");
  const input={sourceId:source?.id?String(source.id):null,sourceName:String(source?.name||"document"),text:String(source?.text||""),sourceType:String(sourceType||source?.sourceType||"learned"),kindHint:kindHint||null};
  return await window.__TAURI__.core.invoke("nova_mind_parse_source",{input});
}

async function nova7RustMindIngestSource(source,{kindHint="lesson",sourceType="learned"}={}){
  if(!nova7RustMindBridge.available||!nova7RustMindBridge.initialized)throw new Error("Rust NovaMind is required for teaching ingestion.");
  const input={sourceId:source?.id?String(source.id):null,sourceName:String(source?.name||"document"),text:String(source?.text||""),sourceType:String(sourceType||source?.sourceType||"learned"),kindHint:kindHint||null};
  const receipt=await window.__TAURI__.core.invoke("nova_mind_ingest_source",{input});
  if(!window.__TAURI__?.core?.invoke){
    nova7RustMindApplySemanticMirror(receipt?.semanticState);
    nova7RustMindApplyLearningMirror({state:receipt?.learningState,report:receipt?.synthesisReport});
  }
  nova7RustMindBridge.learningReceipt={state:nova7RustMindClone(receipt?.learningState),report:nova7RustMindClone(receipt?.synthesisReport),authority:"rust_v7_pass_9"};
  nova7RustMindBridge.snapshot=await window.__TAURI__.core.invoke("nova_mind_snapshot");
  nova7RustMindApplySnapshotMirrors(nova7RustMindBridge.snapshot);
  nova7RustMindBridge.lastError=null;nova7RustMindBridge.updatedAt=Date.now();
  return receipt;
}

async function nova7RustTeacherRecordAttempt(input){
  if(!nova7RustMindBridge.available||!nova7RustMindBridge.initialized)throw new Error("Rust NovaMind is required for teacher test history.");
  const receipt=await window.__TAURI__.core.invoke("nova_teacher_record_attempt",{input});
  nova7RustMindBridge.snapshot=await window.__TAURI__.core.invoke("nova_mind_snapshot");
  nova7RustMindBridge.updatedAt=Date.now();
  return receipt;
}

async function nova7RustTeacherFinalizeRun(input){
  if(!nova7RustMindBridge.available||!nova7RustMindBridge.initialized)throw new Error("Rust NovaMind is required to finalize teacher test evidence.");
  const receipt=await window.__TAURI__.core.invoke("nova_teacher_finalize_run",{input});
  nova7RustMindBridge.snapshot=await window.__TAURI__.core.invoke("nova_mind_snapshot");
  nova7RustMindBridge.updatedAt=Date.now();
  return receipt;
}

async function nova7RustTeacherUpdateFeedback(input){
  if(!nova7RustMindBridge.available||!nova7RustMindBridge.initialized)throw new Error("Rust NovaMind is required for teacher feedback.");
  const receipt=await window.__TAURI__.core.invoke("nova_teacher_update_feedback",{input});
  nova7RustMindBridge.snapshot=await window.__TAURI__.core.invoke("nova_mind_snapshot");
  nova7RustMindBridge.updatedAt=Date.now();
  return receipt;
}

function nova7RustMindAssistantLabel(){
  try{return String(APP_CONFIG?.assistantName||V41_SELF_MODEL?.identity?.name||"Nova");}catch(_){return "Nova";}
}

function nova7RustMindConversationContext(){
  return []; // Browser compatibility only; native Pass 9 context selection lives in Rust.
}



function nova7RustMindPersistedDevelopmentState(){
  try{const memory=typeof v43Memory==="function"?v43Memory():null;const persisted=memory?.rustDevelopmentState;return persisted?.initialized?nova7RustMindClone(persisted):null;}catch(_){return null;}
}
function nova7RustMindLegacyDevelopmentInput(){
  try{
    const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return {legacyCategories:[],legacyRelations:[],legacyEpisodes:[],legacyAbstractions:[]};
    const graph=memory.cognitiveCategories||{},episodes=memory.developmentEpisodes||{},abstractions=memory.cognitiveAbstractions||{};
    const now=Date.now();
    const legacyCategories=(graph.categories||[]).map(row=>({id:String(row.id||`legacy_category_${String(row.name||"").toLowerCase().replace(/[^a-z0-9]+/g,"_")}`),name:String(row.name||row.label||""),key:String(row.key||row.name||row.label||"").toLowerCase().trim(),origin:String(row.origin||"legacy_js_import"),status:String(row.status||"ACTIVE"),traits:nova7RustMindClone(row.traits||[]),evidence:nova7RustMindClone(row.evidence||[]),support:Number(row.support||1),createdAt:Number(row.createdAt||now),updatedAt:Number(row.updatedAt||now)})).filter(row=>row.name);
    const legacyRelations=(graph.relations||[]).map(row=>({id:String(row.id||nova7RustMindLegacyId("legacy_catrel",`${row.subject||""}|${row.predicate||"related_to"}|${row.object||""}`)),subject:String(row.subject||""),predicate:String(row.predicate||"related_to"),object:String(row.object||""),origin:String(row.origin||"legacy_js_import"),evidence:Array.isArray(row.evidence)?nova7RustMindClone(row.evidence):(row.evidence?[nova7RustMindClone(row.evidence)]:[]),support:Number(row.support||1),createdAt:Number(row.createdAt||now),updatedAt:Number(row.updatedAt||now)})).filter(row=>row.subject&&row.object);
    const legacyEpisodes=(episodes.episodes||[]).map(row=>({id:String(row.id||nova7RustMindLegacyId("legacy_episode",`${row.turnId||"legacy"}|${row.promptHash||row.promptPreview||""}`)),turnId:String(row.turnId||row.id||"legacy"),promptHash:String(row.promptHash||"legacy"),promptPreview:String(row.promptPreview||""),supported:Boolean(row.supported),conceptTerms:(row.conceptTerms||[]).map(String),goalTerms:(row.goalTerms||[]).map(String),relationTerms:(row.relationTerms||[]).map(String),contributors:(row.contributors||[]).filter(c=>Number.isInteger(c.number??c.moduleNumber)).map(c=>({moduleId:String(c.moduleId||`legacy_module_${c.number??c.moduleNumber}`),moduleNumber:Number(c.number??c.moduleNumber),strength:Number(c.strength||.5),stateChangeCount:Number(c.deltaScore||c.stateChangeCount||1)})),experienceRouteIds:row.experienceRouteId?[String(row.experienceRouteId)]:(row.experienceRouteIds||[]).map(String),cycles:Number(row.cycles||0),createdAt:Number(row.createdAt||now)}));
    const legacyAbstractions=(abstractions.abstractions||[]).map(row=>({id:String(row.id||nova7RustMindLegacyId("legacy_abstraction",row.signature||row.generalTerm||row.name||"legacy")),signature:String(row.signature||row.id||"legacy"),kind:String(row.kind||"experience_pattern"),status:String(row.status||"PROVISIONAL"),generalTerm:String(row.generalTerm||row.name||"provisional pattern"),description:String(row.description||"Migrated provisional abstraction."),moduleNumbers:(row.template?.moduleNumbers||row.moduleNumbers||[]).map(Number).filter(Number.isFinite),conceptTerms:(row.template?.conceptTerms||row.conceptTerms||[]).map(String),instances:Number(row.support?.instances||row.instances||1),confidence:Number(row.confidence||.55),evidence:nova7RustMindClone(row.evidence||[]),createdAt:Number(row.createdAt||now),updatedAt:Number(row.updatedAt||now)}));
    return {legacyCategories,legacyRelations,legacyEpisodes,legacyAbstractions};
  }catch(error){console.warn("Nova V7 could not prepare one-time developmental migration:",error);return {legacyCategories:[],legacyRelations:[],legacyEpisodes:[],legacyAbstractions:[]};}
}
function nova7RustMindApplyDevelopmentMirror(rustState){
  if(!rustState)return;try{const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return;memory.rustDevelopmentState=nova7RustMindClone(rustState);memory.cognitiveCategories={schema:1,version:"7.0.0-alpha.8",authority:"rust_v7_pass_8_mirror",categories:nova7RustMindClone(rustState.categories||[]),relations:nova7RustMindClone(rustState.relations||[]),updatedAt:Date.now()};memory.developmentEpisodes={schema:1,version:"7.0.0-alpha.8",authority:"rust_v7_pass_8_mirror",episodes:nova7RustMindClone(rustState.episodes||[]),sinceConsolidation:Number(rustState.sinceConsolidation||0),lastConsolidation:nova7RustMindClone(rustState.lastConsolidation||null),updatedAt:Date.now()};memory.cognitiveAbstractions={schema:2,version:"7.0.0-alpha.8",authority:"rust_v7_pass_8_mirror",abstractions:nova7RustMindClone(rustState.abstractions||[]),lastPass:nova7RustMindClone(rustState.lastConsolidation||null),updatedAt:Date.now()};}catch(error){console.warn("Nova V7 could not mirror Rust developmental state:",error);}
}
async function nova7RustMindImportDevelopment(){
  throw new Error("Pass 9 retired JavaScript developmental-state import. Rust NovaMind owns developmental hydration.");
}

function nova7RustMindApplyInternalMirror(rustState){
  if(!rustState)return;try{const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return;memory.rustInternalCognitionState=nova7RustMindClone(rustState);memory.freeThought={schema:1,version:"7.0.0-alpha.8",authority:"rust_v7_pass_8_mirror",goals:nova7RustMindClone(rustState.goals||[]),history:nova7RustMindClone(rustState.history||[]),updatedAt:Date.now()};}catch(error){console.warn("Nova V7 could not mirror Rust internal cognition:",error);}
}
async function nova7RustMindImportInternalCognition(){
  throw new Error("Pass 9 retired JavaScript internal-cognition import. Rust NovaMind owns internal cognition hydration.");
}


function nova7RustMindApplySnapshotMirrors(snapshot){
  if(!snapshot)return;
  if(window.__TAURI__?.core?.invoke)return; // Pass 9 native JavaScript has no brain mirrors.
  try{if(snapshot.semanticMemory)nova7RustMindApplySemanticMirror(snapshot.semanticMemory);if(snapshot.learningState)nova7RustMindApplyLearningMirror({state:snapshot.learningState,report:snapshot.learningState.lastSynthesis||null});if(snapshot.developmentState)nova7RustMindApplyDevelopmentMirror(snapshot.developmentState);if(snapshot.internalCognition)nova7RustMindApplyInternalMirror(snapshot.internalCognition);}catch(error){console.warn("Nova V7 browser snapshot mirror failed:",error);}
}

async function nova7RustMindInitialize(){
  nova7RustMindBridge.available = nova7RustMindInvokeAvailable();
  nova7RustMindBridge.lastError = null;
  if(!nova7RustMindBridge.available){nova7RustMindBridge.initialized=false;nova7RustMindBridge.updatedAt=Date.now();return nova7RustMindStatus();}
  try{
    // Pass 9 bootstrap restores NovaMind in Rust before this bridge initializes.
    // JavaScript does not import, synchronize, or reconstruct brain state.
    nova7RustMindBridge.snapshot = await window.__TAURI__.core.invoke("nova_mind_snapshot");
    nova7RustMindBridge.initialized = Boolean(nova7RustMindBridge.snapshot);
  }catch(error){nova7RustMindBridge.initialized=false;nova7RustMindBridge.lastError=String(error?.message||error);}
  nova7RustMindBridge.updatedAt=Date.now();
  return nova7RustMindStatus();
}

async function nova7RustMindResetDevelopmentalState(){
  throw new Error("Pass 9 retired partial developmental reset. Native reset is whole-mind Rust reset only.");
}


async function nova7RustMindBeginTurn(text,{sourceLabel=null}={}){
  if(!nova7RustMindBridge.available || !nova7RustMindBridge.initialized)return null;
  try{
    const raw=String(text??"");
    const receipt = await window.__TAURI__.core.invoke("nova_mind_begin_turn",{
      input:{
        text:raw,
        sourceLabel:sourceLabel||null
      }
    });
    nova7RustMindBridge.lastReceipt = receipt;
    nova7RustMindBridge.activeTurnId = receipt?.turnId || null;
    nova7RustMindBridge.activeRawText = raw;
    nova7RustMindBridge.activeCanonicalState = nova7RustMindClone(receipt?.canonicalState||null);
    nova7RustMindBridge.activeSemanticReport = nova7RustMindClone(receipt?.semanticReport||null);
    nova7RustMindBridge.activeCognitiveReport = nova7RustMindClone(receipt?.cognitiveReport||null);
    nova7RustMindBridge.activeRealizationReport = nova7RustMindClone(receipt?.realizationReport||null);
    nova7RustMindBridge.activeEventKind = receipt?.eventKind||"cognition";
    nova7RustMindBridge.lastError = null;
    nova7RustMindBridge.snapshot = await window.__TAURI__.core.invoke("nova_mind_snapshot");
    nova7RustMindBridge.updatedAt = Date.now();
    return receipt;
  }catch(error){
    nova7RustMindBridge.lastError = String(error?.message || error);
    nova7RustMindBridge.updatedAt = Date.now();
    return null;
  }
}

function nova7RustMindSemanticSeed(rawText){
  if(!nova7RustMindBridge.activeTurnId || !nova7RustMindBridge.activeCanonicalState)return null;
  if(String(rawText??"")!==String(nova7RustMindBridge.activeRawText??""))return null;
  return {
    canonicalState:nova7RustMindClone(nova7RustMindBridge.activeCanonicalState),
    semanticReport:nova7RustMindClone(nova7RustMindBridge.activeSemanticReport),
    cognitiveReport:nova7RustMindClone(nova7RustMindBridge.activeCognitiveReport),
    realizationReport:nova7RustMindClone(nova7RustMindBridge.activeRealizationReport),
    eventKind:nova7RustMindBridge.activeEventKind||"cognition",
    turnId:nova7RustMindBridge.activeTurnId
  };
}

async function nova7RustMindFinishTurn({responseProduced=false}={}){
  const turnId = nova7RustMindBridge.activeTurnId;
  if(!turnId || !nova7RustMindBridge.available || !nova7RustMindBridge.initialized)return null;
  try{
    const snapshot = await window.__TAURI__.core.invoke("nova_mind_finish_turn",{
      input:{turnId,responseProduced:Boolean(responseProduced)}
    });
    nova7RustMindBridge.snapshot = snapshot;
    nova7RustMindApplySnapshotMirrors(snapshot);
    nova7RustMindBridge.activeTurnId = null;
    nova7RustMindBridge.activeRawText = null;
    nova7RustMindBridge.activeCanonicalState = null;
    nova7RustMindBridge.activeSemanticReport = null;
    nova7RustMindBridge.activeCognitiveReport = null;
    nova7RustMindBridge.activeRealizationReport = null;
    nova7RustMindBridge.activeEventKind = null;
    nova7RustMindBridge.lastError = null;
    nova7RustMindBridge.updatedAt = Date.now();
    return snapshot;
  }catch(error){
    nova7RustMindBridge.lastError = String(error?.message || error);
    nova7RustMindBridge.updatedAt = Date.now();
    return null;
  }
}


try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version="7.0.0-alpha.9.3.1";
    V41_SELF_MODEL.architecture=[
      "Rust-owned turn lifecycle, semantic core, semantic-goal executive, learned cognitive-module runtime, synthesis authority, and Stored Only executability",
      "one canonical typed cognitive state",
      "open learned semantic graph",
      "Rust capability-gated learned module execution",
      "Rust-owned plastic experience-built cognitive index",
      "Rust-owned developmental category graph and consolidation",
      "Rust-owned internal goals/free thought through the current executive",
      "Rust-owned response realization and teaching-intent interpretation",
      "Rust-owned complete persistent mind and conversation continuity",
      "JavaScript UI over a read-only Rust mind snapshot",
      "trusted external capability boundary"
    ];
  }
}catch(_){ }

console.info("Nova V7 alpha 9.3 Rust bridge loaded — Rust owns the complete mind and teacher/test evidence; JavaScript is UI/IPC only on native Tauri.");
