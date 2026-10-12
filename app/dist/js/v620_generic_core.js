"use strict";

/* =========================================================
   Nova V7 Pass 9 — NATIVE UI COORDINATOR / BROWSER DEVELOPMENT RUNTIME

   One authoritative live turn. In Tauri, Rust now owns semantic memory, native teaching parsing/translation, the migrated semantic/request/concept slice, semantic-goal executive, learned cognitive synthesis, and learned cognitive-module execution. This JavaScript file coordinates UI-facing turn completion and retains a browser-only migration runtime for non-Tauri development; it is not a native fallback brain.

   There is no older response fallback and no duplicate JavaScript semantic
   constructor behind Rust. The canonical Rust state is adopted directly and
   the remaining learned cognition works on that same state.
   ========================================================= */

const NOVA620_CORE_VERSION="7.0.0-alpha.9.3.1";
const NOVA620_CORE_SCHEMA=3;
const NOVA620_MAX_CYCLES=16;
const NOVA620_TERMINALS=Object.freeze({SUPPORTED:"SUPPORTED",BLOCKED:"BLOCKED",EXHAUSTED:"EXHAUSTED"});
const NOVA620_REASONS=Object.freeze({
  LEARNED:"LEARNING_COMPLETED",
  GOAL_SATISFIED:"COGNITIVE_GOAL_SATISFIED",
  MISSING_OPERATION:"MISSING_COGNITIVE_OPERATION",
  NO_INTERPRETATION:"SEMANTIC_INTERPRETATION_INCOMPLETE",
  INTERNAL_ERROR:"CURRENT_CORE_INTERNAL_ERROR"
});
let V620_ACTIVE_WORKSPACE=null;

function v620CoreClone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v620CoreClean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v620CoreCanon(v){try{return canonical(String(v??""));}catch(_){return v620CoreClean(v).toLowerCase();}}
function v620CoreId(prefix){return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2,8)}`;}
function v620CoreUnique(xs,key=x=>typeof x==="object"?JSON.stringify(x):String(x)){const out=[],seen=new Set();for(const x of xs||[]){if(x==null)continue;let k="";try{k=key(x);}catch(_){k=String(x);}if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}

/* ---------------------------------------------------------
   LOSSLESS / NON-SEMANTIC SURFACE OBSERVATION

   These observations say what was physically present in the input. They do not
   decide that any word means hypothetical, question, cause, emotion, etc.
   --------------------------------------------------------- */
function v620QuoteRanges(rawText){
  const raw=String(rawText??""),out=[];for(const [rx,marker] of [[/"([^"\n]*)"/g,'"'],[/“([^”\n]*)”/g,'“'],[/'([^'\n]*)'/g,"'"]]){let m;while((m=rx.exec(raw))){const inner=m[1]??"",offset=m[0].indexOf(inner);out.push({id:v620CoreId("quote"),text:inner,start:m.index+offset,end:m.index+offset+inner.length,fullStart:m.index,fullEnd:m.index+m[0].length,marker});}}return out.sort((a,b)=>a.fullStart-b.fullStart);
}
function v620SurfaceSegments(rawText,quotes){const raw=String(rawText??""),out=[];let pos=0;for(const q of quotes||[]){if(q.fullStart>pos)out.push({kind:"outer_text",text:raw.slice(pos,q.fullStart),start:pos,end:q.fullStart});out.push({kind:"quoted_text",text:q.text,start:q.start,end:q.end,quoteId:q.id});pos=q.fullEnd;}if(pos<raw.length)out.push({kind:"outer_text",text:raw.slice(pos),start:pos,end:raw.length});return out.filter(x=>x.text!=="");}
function v620SurfaceSentences(rawText){const raw=String(rawText??""),out=[],rx=/[^.!?]+[.!?]?/g;let m;while((m=rx.exec(raw))){const full=m[0],leading=(full.match(/^\s*/)||[""])[0].length,trailing=(full.match(/\s*$/)||[""])[0].length,text=full.slice(leading,Math.max(leading,full.length-trailing));if(!text)continue;const start=m.index+leading;out.push({id:v620CoreId("sentence"),text,start,end:start+text.length,terminal:(text.match(/[.!?]+$/)||[])[0]||null});}return out;}
function v620SurfaceTokens(rawText){const raw=String(rawText??""),out=[],rx=/[A-Za-z][A-Za-z0-9_'’-]*|\d+(?:\.\d+)?|[^\sA-Za-z0-9]/g;let m;while((m=rx.exec(raw))){const surface=m[0];out.push({surface,start:m.index,end:m.index+surface.length,word:/^[A-Za-z]/.test(surface),capitalized:/^[A-Z][A-Za-z0-9_'’-]*$/.test(surface)});}return out;}
function v620ObserveUtterance(rawText){
  const raw=String(rawText??""),quotes=v620QuoteRanges(raw),sentences=v620SurfaceSentences(raw),tokens=v620SurfaceTokens(raw),punctuation=[];for(let i=0;i<raw.length;i++)if(/[^\w\s]/.test(raw[i]))punctuation.push({surface:raw[i],index:i});
  return {version:NOVA620_CORE_VERSION,raw,normalizedForDisplay:v620CoreClean(raw),quotes,segments:v620SurfaceSegments(raw,quotes),sentences,tokens,punctuation,boundaries:{colonIndices:[...raw.matchAll(/:/g)].map(m=>m.index),lineBreakIndices:[...raw.matchAll(/\n/g)].map(m=>m.index)},note:"Surface evidence only. No lexical item, punctuation mark, capitalization pattern, or quote is assigned semantic meaning here."};
}

/* ---------------------------------------------------------
   BOOTSTRAP PARTICIPANT FRAME

   This is starting machinery, not a closed semantic ontology. More participant
   structures and reference conventions can be learned as ordinary modules.
   --------------------------------------------------------- */
function v620ParticipantFrame(){
  const nova=(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL?.identity?.name)||"Nova";
  return {speaker:{id:"user",label:"user",role:"speaker"},listener:{id:"assistant",label:nova,role:"addressee"},group:{id:"user+assistant",label:`user and ${nova}`,role:"group"}};
}
function v627ObservedScopes(surface){
  const raw=String(surface?.raw??"");const root={id:"scope_discourse",kind:"discourse_scope",parentId:null,start:0,end:raw.length,boundaryKind:"root",participantFrame:null,policies:{},source:"bootstrap_surface_scope"};const out=[root];
  for(const segment of surface?.segments||[]){if(segment?.kind!=="quoted_text")continue;out.push({id:`scope_${segment.quoteId||v620CoreId("region")}`,kind:"bounded_surface_scope",parentId:root.id,start:Number(segment.start||0),end:Number(segment.end||0),boundaryKind:segment.kind,participantFrame:null,policies:{},source:"observed_surface_boundary"});}
  return out;
}
function v627ScopeAt(scopes,index){return (scopes||[]).filter(scope=>scope.id!=="scope_discourse"&&Number(index)>=Number(scope.start)&&Number(index)<Number(scope.end)).sort((a,b)=>(Number(a.end)-Number(a.start))-(Number(b.end)-Number(b.start)))[0]||(scopes||[]).find(scope=>scope.id==="scope_discourse")||null;}
function v620BootstrapDeictics(rawText,participants,surface=null,scopes=null){
  const raw=String(rawText??""),out=[],observed=surface||v620ObserveUtterance(raw),scopeList=scopes||v627ObservedScopes(observed),rx=/\b(?:I|me|my|mine|myself|you|your|yours|yourself|yourselves|we|us|our|ours|ourselves)\b/gi;let m;
  while((m=rx.exec(raw))){const key=v620CoreCanon(m[0]);let grammaticalPerson="first",role="speaker",participant=participants.speaker;if(["you","your","yours","yourself","yourselves"].includes(key)){grammaticalPerson="second";role="listener";participant=participants.listener;}else if(["we","us","our","ours","ourselves"].includes(key)){grammaticalPerson="first_plural";role="group";participant=participants.group;}const scope=v627ScopeAt(scopeList,m.index),isRoot=!scope||scope.id==="scope_discourse";out.push({id:v620CoreId("ref"),surface:m[0],index:m.index,end:m.index+m[0].length,grammaticalPerson,scopeId:scope?.id||"scope_discourse",resolvedEntity:isRoot?(participant?.label||participant?.id):null,participantRole:isRoot?role:null,resolutionSource:isRoot?"bootstrap_participant_frame":"scoped_deictic_unresolved",locked:isRoot});
  }
  return out;
}

function v620CreateCanonicalState(rawText,observation=null){
  const surface=observation||v620ObserveUtterance(rawText),participants=v620ParticipantFrame();
  let mentalState=null,activities=[];try{if(typeof v556CurrentMindState==="function")mentalState=v620CoreClone(v556CurrentMindState());}catch(_){ }try{if(typeof v555ActiveActivities==="function")activities=v620CoreClone(v555ActiveActivities());}catch(_){ }
  let discourseContext={entities:[],events:[],relations:[],propositions:[]};try{const prior=(typeof state!=="undefined"?state:null)?.context?.recentSemanticDiscourse;if(prior&&typeof prior==="object")discourseContext=v620CoreClone(prior);}catch(_){ }
  const scopes=v627ObservedScopes(surface);const rootScope=scopes.find(scope=>scope.id==="scope_discourse");if(rootScope)rootScope.participantFrame=v620CoreClone(participants);
  return {version:NOVA620_CORE_VERSION,schemaVersion:NOVA620_CORE_SCHEMA,id:v620CoreId("mind"),source:{raw:String(rawText??""),immutable:true},surface:v620CoreClone(surface),participants,scopes,activeScopeId:"scope_discourse",references:v620BootstrapDeictics(rawText,participants,surface,scopes),frames:[],entities:[],events:[],relations:[],propositions:[],requests:[],unknownSlots:[],candidateSets:[],constraints:[],evidence:[],conclusions:[],structures:[],categories:[],activeCategories:[],mentalState,activities,discourseContext,cognitiveOutcome:null,learnedState:{},moduleState:{},epistemic:{propositions:{},evidenceRelations:[],beliefs:{},attempts:[],reasoningPolicies:[],constraintPolicies:[],stopPolicies:[],unresolved:[]},unresolved:[{id:"goal_interpret_current_utterance",type:"semantic_interpretation",description:"construct enough meaning and cognition from the current utterance to determine what to do"}],status:"UNRESOLVED",createdAt:Date.now(),updatedAt:Date.now(),semanticOwner:"javascript_browser_only"};
}
function v700AdoptRustCanonicalState(raw,seed){
  const mind=v620CoreClone(seed?.canonicalState||{}),now=Date.now();
  let mentalState=null,activities=[];try{if(typeof v556CurrentMindState==="function")mentalState=v620CoreClone(v556CurrentMindState());}catch(_){ }try{if(typeof v555ActiveActivities==="function")activities=v620CoreClone(v555ActiveActivities());}catch(_){ }
  mind.version ||= "7.0.0-rust-authoritative-mind-pass-9";mind.schemaVersion ||= 4;mind.id ||= v620CoreId("rust_mind");mind.source={raw:String(raw??""),immutable:true,owner:"rust_v7_pass_9"};
  mind.surface ||= v620ObserveUtterance(raw);mind.participants ||= v620ParticipantFrame();mind.scopes ||= [];mind.activeScopeId ||= "scope_discourse";mind.references ||= [];mind.frames ||= [];mind.entities ||= [];mind.events ||= [];mind.relations ||= [];mind.propositions ||= [];mind.requests ||= [];mind.unknownSlots ||= [];mind.candidateSets ||= [];mind.constraints ||= [];mind.evidence ||= [];mind.conclusions ||= [];mind.structures ||= [];mind.categories ||= [];mind.activeCategories ||= [];mind.activeConcepts ||= [];mind.discourseContext ||= {entities:[],events:[],relations:[],propositions:[]};mind.semanticTrace ||= [];mind.cognitiveTrace ||= [];
  mind.mentalState=mentalState;mind.activities=activities;mind.cognitiveOutcome ??= null;mind.learnedState ||= {};mind.moduleState ||= {};mind.epistemic ||= {propositions:{},evidenceRelations:[],beliefs:{},attempts:[],reasoningPolicies:[],constraintPolicies:[],stopPolicies:[],unresolved:[]};mind.unresolved ||= [];mind.status ||= "UNRESOLVED";mind.createdAt ||= now;mind.updatedAt=now;mind.semanticOwner="rust_v7_pass_9";mind.rustSemanticReport=v620CoreClone(seed?.semanticReport||null);mind.rustCognitiveReport=v620CoreClone(seed?.cognitiveReport||null);return mind;
}
function v620CreateWorkspace(rawText){
  const raw=String(rawText??"");let rustSeed=null;try{if(typeof nova7RustMindSemanticSeed==="function")rustSeed=nova7RustMindSemanticSeed(raw);}catch(_){ }
  const nativeRustExpected=Boolean(typeof nova7RustMindBridge==="object"&&nova7RustMindBridge?.available);
  const mind=rustSeed?v700AdoptRustCanonicalState(raw,rustSeed):v620CreateCanonicalState(raw,v620ObserveUtterance(raw)),observation=v620CoreClone(mind.surface||v620ObserveUtterance(raw));
  return {version:NOVA620_CORE_VERSION,id:v620CoreId("turn"),rawInput:raw,immutableRawInput:raw,utteranceObservation:observation,canonicalSemanticState:mind,cognitiveState:mind,participants:mind.participants,moduleState:mind.moduleState,activeCategories:mind.activeCategories,appliedCognitiveModules:[],rustSemanticSeed:Boolean(rustSeed),rustSemanticReport:v620CoreClone(rustSeed?.semanticReport||null),rustCognitiveReport:v620CoreClone(rustSeed?.cognitiveReport||null),rustRealizationReport:v620CoreClone(rustSeed?.realizationReport||null),rustEventKind:String(rustSeed?.eventKind||"cognition"),nativeRustExpected,executive:{version:NOVA620_CORE_VERSION,architecture:rustSeed?"rust_pass_9_authoritative_mind":nativeRustExpected?"rust_native_unavailable":"javascript_browser_migration_runtime",legacyFallbackAllowed:false,cycles:[],terminalState:null,reasonCode:null,details:{semanticOwner:rustSeed?"rust_v7_pass_9":nativeRustExpected?"rust_required_but_unavailable":"javascript_browser_only"},startedAt:Date.now(),endedAt:null},createdAt:Date.now()};
}

function v620MindDigest(mind,moduleState){
  const compact={scopes:mind.scopes,activeScopeId:mind.activeScopeId,frames:mind.frames,entities:mind.entities,events:mind.events,relations:mind.relations,propositions:mind.propositions,requests:mind.requests,unknownSlots:mind.unknownSlots,candidateSets:mind.candidateSets,constraints:mind.constraints,evidence:mind.evidence,conclusions:mind.conclusions,structures:mind.structures,cognitiveOutcome:mind.cognitiveOutcome,unresolved:mind.unresolved,activeCategories:mind.activeCategories,learnedState:mind.learnedState,moduleState};
  try{return JSON.stringify(compact);}catch(_){return String(Date.now());}
}
function v620ModuleContext(ws,extra={}){const mind=ws.canonicalSemanticState;let recentEntities=[];try{recentEntities=(typeof state!=="undefined"?state:null)?.context?.recentEntities||[];}catch(_){ }return {version:NOVA620_CORE_VERSION,phase:"cognition",rawText:ws.rawInput,input:ws.rawInput,surface:ws.rawInput,utteranceObservation:ws.utteranceObservation,workspace:ws,canonicalSemanticState:mind,cognitiveState:mind,participants:mind.participants,references:mind.references,recentEntities,discourseContext:mind.discourseContext||{},moduleState:ws.moduleState,learnedState:mind.learnedState,structures:mind.structures,cognitiveOutcome:mind.cognitiveOutcome,unresolved:mind.unresolved,activeCategories:mind.activeCategories,constraints:mind.constraints,scopes:mind.scopes,activeScopeId:mind.activeScopeId,internalGoal:ws.internalGoal||null,executionMode:ws.executionMode||"USER_TURN",...extra};}
function v620ExtractSurface(ws){
  const conclusions=[...(ws.canonicalSemanticState.conclusions||[])].reverse(),rustResolved=conclusions.find(c=>c?.rustAuthoritative===true&&typeof (c?.surface??c?.response??c?.text)==="string"&&String(c?.surface??c?.response??c?.text).trim());
  if(rustResolved)return String(rustResolved.surface??rustResolved.response??rustResolved.text).trim();
  const direct=ws.moduleState?.surfaceResponse??ws.canonicalSemanticState?.moduleState?.surfaceResponse;if(typeof direct==="string"&&direct.trim())return direct.trim();
  for(const c of conclusions){const s=c?.surface??c?.response??c?.text;if(typeof s==="string"&&s.trim())return s.trim();}return null;
}
function v620SetTerminal(ws,state620,reasonCode,details={}){ws.executive.terminalState=state620;ws.executive.reasonCode=reasonCode;ws.executive.details=v620CoreClone(details);ws.executive.endedAt=Date.now();ws.canonicalSemanticState.status=state620;ws.canonicalSemanticState.updatedAt=Date.now();return ws.executive;}

/* ---------------------------------------------------------
   TERMINAL-STATE COGNITION EXPOSURE

   A stalled current mind does not jump to a canned chat sentence. Before the
   executive terminates, it writes a truthful cognitive-outcome structure into
   the same shared state and gives current learned behavior/realization modules
   one ordinary chance to respond to it. This is not a fallback responder: no
   surface text is supplied here, and no older Nova is invoked.
   --------------------------------------------------------- */
function v629ExposeCognitiveOutcome(ws){
  const mind=ws?.canonicalSemanticState;if(!mind)return null;
  const hasMeaning=[mind.frames,mind.entities,mind.events,mind.relations,mind.propositions,mind.requests,mind.unknownSlots,mind.candidateSets,mind.structures,mind.conclusions].some(x=>Array.isArray(x)&&x.length);
  const requests=Array.isArray(mind.requests)?mind.requests:[],unresolved=Array.isArray(mind.unresolved)?mind.unresolved:[];
  const outcome={
    id:"current_cognitive_outcome",
    type:"cognitive_outcome",
    state:hasMeaning?"blocked_with_partial_meaning":"interpretation_incomplete",
    meaningConstructed:Boolean(hasMeaning),
    requestIdentified:requests.length>0,
    unresolvedGoalCount:unresolved.length,
    unresolvedGoalTypes:v620CoreUnique(unresolved.map(goal=>goal?.type).filter(Boolean),v620CoreCanon),
    responseProduced:false,
    executionMode:ws.executionMode||"USER_TURN",
    source:"current_executive_stall"
  };
  mind.cognitiveOutcome=outcome;
  mind.activeCategories=v620CoreUnique([...(mind.activeCategories||[]),"communication","realization","self_report"],v620CoreCanon);
  const existing=(mind.structures||[]).find(item=>item?.id===outcome.id);if(existing)Object.assign(existing,v620CoreClone(outcome));else{mind.structures ||= [];mind.structures.push(v620CoreClone(outcome));}
  mind.updatedAt=Date.now();return outcome;
}

/* ---------------------------------------------------------
   ITERATIVE CENTRAL EXECUTIVE

   There is no fixed interpretation -> reasoning -> memory -> realization pass.
   Any applicable learned module may change the shared state. The executive then
   reconsiders the state until a response is produced, no further progress is
   possible, or the bounded cycle limit is reached.
   --------------------------------------------------------- */
function v620RunCurrentMind(ws){
  // Native Tauri cognition is authoritative after Pass 4. A missing Rust receipt
  // is a native-runtime failure, not permission to wake the JavaScript brain.
  if(ws?.nativeRustExpected&&!ws?.rustSemanticSeed){
    v620SetTerminal(ws,NOVA620_TERMINALS.BLOCKED,NOVA620_REASONS.INTERNAL_ERROR,{rustRequired:true,rustUnavailable:true});
    return {surface:null,references:[],considered:[],applied:[],changed:[],totalProgress:0,cycles:0,rustRuntimeUnavailable:true};
  }
  if(ws?.rustSemanticSeed){
    const realization=ws.rustRealizationReport||{},surface=typeof realization.surface==="string"&&realization.surface.trim()?realization.surface.trim():null,cog=ws.rustCognitiveReport||{},normalizeRow=row=>({number:row?.moduleNumber??null,id:row?.moduleId??null,name:row?.name||row?.moduleId||"Rust module",phase:row?.phase||"reasoning",categories:v620CoreClone(row?.categories||[]),progress:Boolean(row?.progress),deltaScore:Number(row?.deltaScore||0),error:row?.error||null,attentionWeight:Number(row?.attentionWeight||0),sourceType:"rust_runtime"}),applied=(cog.applied||[]).map(normalizeRow),changed=(cog.changed||[]).map(normalizeRow);
    const rustRouteIds=v620CoreClone(cog.experienceRouteIds||[]),rustSearchMode=String(cog.searchMode||"");
    ws.cognitiveSearch={mode:rustSearchMode,routeIds:rustRouteIds,attemptedRouteIds:rustRouteIds.slice(),globalExplored:rustSearchMode.includes("explore_all"),forceBroad:false,routeFailureRecorded:false,workingSet:changed.map(row=>row.number).filter(Number.isInteger),updatedAt:Date.now()};
    const details={cycles:Number(cog.cycles||0),considered:Number(cog.modulesConsidered||0),applied:Number(cog.modulesApplied||0),totalProgress:Number(cog.progressCount||0),rustSemanticGoalResolved:Boolean(ws.rustSemanticReport?.suggestedSurface),rustCognitiveRuntime:true,searchMode:rustSearchMode,experienceRouteIds:rustRouteIds,broadenedAfterExperiencedRoute:Boolean(cog.broadenedAfterExperienceRoute),stalled:Boolean(cog.stalled),outcomeExposed:Boolean(cog.outcomeExposed),unsupportedOperations:v620CoreClone(cog.unsupportedOperations||[]),missingCapabilities:v620CoreClone(cog.missingCapabilities||[]),awakenableStoredOnlyRuleIds:v620CoreClone(cog.awakenableStoredOnlyRuleIds||[])};
    details.developmentalAuthority="rust_v7_pass_9";
    details.realizationAuthority="rust_v7_pass_9";
    const terminal=String(realization.terminalState||"EXHAUSTED"),reason=String(realization.reasonCode||"NO_INTERPRETATION");
    if(surface&&ws.canonicalSemanticState.cognitiveOutcome)ws.canonicalSemanticState.cognitiveOutcome.responseProduced=true;
    if(terminal==="SUPPORTED")ws.canonicalSemanticState.unresolved=[];
    v620SetTerminal(ws,terminal,reason,details);
    return {surface,references:[],considered:applied,applied,changed,totalProgress:details.totalProgress,cycles:details.cycles,rustCognitiveReport:v620CoreClone(cog),rustRealizationReport:v620CoreClone(realization)};
  }

  // Browser-only development path. This is not reachable as a Tauri fallback.
  const allRelevant=new Map(),allConsidered=new Map(),allApplied=new Map(),allChanged=new Map();let totalProgress=0,lastDigest=v620MindDigest(ws.canonicalSemanticState,ws.moduleState),stalled=false,outcomeExposed=false;
  for(let cycle=1;cycle<=NOVA620_MAX_CYCLES;cycle++){
    const ctx=v620ModuleContext(ws),reports=[];let cycleProgress=0;
    const globalReport=typeof v502RunCognitiveModules==="function"?v502RunCognitiveModules(null,ctx):{references:[],considered:[],applied:[],changed:[],progressCount:0};reports.push(globalReport);
    for(const mention of ws.canonicalSemanticState.references||[]){const mentionCtx=v620ModuleContext(ws,{mention});const report=v502RunCognitiveModules(null,mentionCtx);reports.push(report);}
    const cycleRelevant=new Map();for(const report of reports){cycleProgress+=Number(report.progressCount||0);for(const ref of report.references||[]){cycleRelevant.set(ref.number,ref);allRelevant.set(ref.number,ref);}for(const row of report.considered||[])allConsidered.set(row.id,row);for(const row of report.applied||[])allApplied.set(row.id,row);for(const row of report.changed||[])allChanged.set(row.id,row);}
    const surface=v620ExtractSurface(ws),digest=v620MindDigest(ws.canonicalSemanticState,ws.moduleState),stateChanged=digest!==lastDigest;totalProgress+=cycleProgress;ws.executive.cycles.push({cycle,relevantModules:cycleRelevant.size,modulesReportingProgress:cycleProgress,stateChanged,surfaceProduced:Boolean(surface),unresolved:(ws.canonicalSemanticState.unresolved||[]).length});const details={cycles:cycle,relevant:allRelevant.size,considered:allConsidered.size,applied:allApplied.size,totalProgress};
    if(surface){ws.canonicalSemanticState.unresolved=[];v620SetTerminal(ws,NOVA620_TERMINALS.SUPPORTED,NOVA620_REASONS.GOAL_SATISFIED,details);return {surface,references:[...allRelevant.values()],considered:[...allConsidered.values()],applied:[...allApplied.values()],changed:[...allChanged.values()],totalProgress,cycles:cycle};}
    if(!stateChanged){if(!outcomeExposed){v629ExposeCognitiveOutcome(ws);outcomeExposed=true;lastDigest=v620MindDigest(ws.canonicalSemanticState,ws.moduleState);continue;}stalled=true;break;}lastDigest=digest;
  }
  const mind=ws.canonicalSemanticState,hasMeaning=[mind.frames,mind.entities,mind.events,mind.relations,mind.propositions,mind.requests,mind.structures,mind.conclusions].some(x=>Array.isArray(x)&&x.length),details={stalled,considered:allConsidered.size,applied:allApplied.size,totalProgress,cycles:ws.executive.cycles.length};v620SetTerminal(ws,hasMeaning?NOVA620_TERMINALS.BLOCKED:NOVA620_TERMINALS.EXHAUSTED,hasMeaning?NOVA620_REASONS.MISSING_OPERATION:NOVA620_REASONS.NO_INTERPRETATION,details);return {surface:null,references:[...allRelevant.values()],considered:[...allConsidered.values()],applied:[...allApplied.values()],changed:[...allChanged.values()],totalProgress,cycles:ws.executive.cycles.length};
}

function v620LooksInstructional(rawText,ws=null){
  if(window.__TAURI__?.core?.invoke)return ws?.rustEventKind==="teaching"?{type:"teaching",authority:"rust_v7_pass_9"}:null;
  try{const teaching=typeof v450DetectTeachingAct==="function"?v450DetectTeachingAct(rawText):null;if(teaching)return teaching;}catch(_){ }
  return null;
}
function v620LearningResponse(learning,synthesis){
  const learnedFacts=Number(learning?.report?.learnedUnits??learning?.report?.learnedCount??0),upgraded=Number(synthesis?.upgraded||0),remaining=Number(synthesis?.remaining||0);
  if(upgraded>0&&remaining>0)return "I learned from that. There are still parts of it I do not know how to use yet.";
  if(upgraded>0)return "I learned from that.";
  if(remaining>0)return "I learned from that, but there is still part of it I do not know how to use yet.";
  return learnedFacts>0?"I learned from that.":"I understood that as teaching.";
}
async function v620LearnCurrentMind(ws,teaching){
  let learning=null,cognitiveLearning=null,synthesis=null,freeThought=null;
  const beforeRules=window.__TAURI__?.core?.invoke?[]:(typeof v621RuleSnapshot==="function"?v621RuleSnapshot():[]);
  if(!(window.__TAURI__?.core?.invoke))throw new Error("This Nova build no longer has a JavaScript learning brain. Conversation learning requires the Rust Nova mind.");
  if(typeof nova7RustMindIngestSource!=="function")throw new Error("Rust Pass 9 conversation ingestion is unavailable; refusing to fall back to JavaScript learning.");
  const rustTeaching=await nova7RustMindIngestSource({id:`conversation_${Date.now()}_${Math.random().toString(36).slice(2,8)}`,name:"conversation",text:ws.rawInput,classification:{type:"teaching"},type:"teaching"},{kindHint:"conversation",sourceType:"conversation"});
  const semanticReport=rustTeaching?.semanticReport||null,counts=semanticReport?.counts||{};
  learning={authority:"rust_v7_pass_9",report:{...semanticReport,learnedUnits:Object.values(counts).reduce((sum,value)=>sum+Number(value||0),0)}};
  cognitiveLearning={authority:"rust_v7_pass_9",parsed:v620CoreClone(rustTeaching?.parsed||null),learningState:v620CoreClone(rustTeaching?.learningState||null)};
  synthesis=rustTeaching?.synthesisReport||null;
  const realization=rustTeaching?.realizationReport||{},response=String(realization.surface||"").trim()||null;if(response)ws.canonicalSemanticState.conclusions.push({type:"learning_result",surface:response,rustAuthoritative:true,source:"rust_v7_pass_9",learning:v620CoreClone(learning?.report||learning||null),cognitiveLearning:v620CoreClone(cognitiveLearning||null),synthesis:v620CoreClone(synthesis||null)});ws.canonicalSemanticState.unresolved=[];v620SetTerminal(ws,String(realization.terminalState||NOVA620_TERMINALS.SUPPORTED),String(realization.reasonCode||NOVA620_REASONS.LEARNED),{synthesis:v620CoreClone(synthesis||null),realizationAuthority:"rust_v7_pass_9"});
  ws.executive.details.internalGoalAuthority="rust_v7_pass_9";
  try{if(typeof novaSaveDurableCheckpoint==="function")await novaSaveDurableCheckpoint("conversation_learning");else saveState();}catch(error){console.error("Conversation-learning durable checkpoint failed:",error);}
  return response;
}

function v620CreateInternalGoalWorkspace(goal){
  if(window.__TAURI__?.core?.invoke)throw new Error("JavaScript internal-goal workspace is retired on native Nova: Rust Pass 9 owns internal goals.");
  const description=v620CoreClean(goal?.description||goal?.target||"internal goal"),ws=v620CreateWorkspace(description);ws.executionMode="INTERNAL_GOAL";ws.internalGoal=v620CoreClone(goal);ws.executive.origin="SELF_GENERATED_GOAL";ws.executive.visibleResponseRequired=false;const mind=ws.canonicalSemanticState;mind.source={raw:description,immutable:true,origin:"self_generated_goal",goalId:goal?.id||null};mind.structures.push({kind:"internal_goal",goalId:goal?.id||null,activityId:goal?.activityId||null,goalType:goal?.goalType||"self_generated_goal",description,origin:goal?.origin||"SELF_GENERATED",generationBasis:v620CoreClone(goal?.generationBasis||null),evidence:v620CoreClone(goal?.evidence||[])});mind.activeCategories=v620CoreUnique([...(mind.activeCategories||[]),"reasoning",goal?.goalType||"self_generated_goal"],v620CoreCanon);mind.unresolved=[{id:`internal_${goal?.id||v620CoreId("goal")}`,type:goal?.goalType||"self_generated_goal",description,goalId:goal?.id||null,activityId:goal?.activityId||null,origin:"self_generated"}];return ws;
}
async function v620RunInternalGoal(goal,options={}){
  if(window.__TAURI__?.core?.invoke)throw new Error("JavaScript internal-goal execution is retired on native Nova: Rust Pass 9 owns free thought through the Rust executive.");
  if(!goal)return null;const prior=V620_ACTIVE_WORKSPACE,ws=v620CreateInternalGoalWorkspace(goal);V620_ACTIVE_WORKSPACE=ws;let report=null;try{report=v620RunCurrentMind(ws);const terminal=ws.executive.terminalState,status=terminal===NOVA620_TERMINALS.SUPPORTED?NOVA555_GOAL_STATUS.SUPPORTED:terminal===NOVA620_TERMINALS.BLOCKED?NOVA555_GOAL_STATUS.BLOCKED:NOVA555_GOAL_STATUS.EXHAUSTED;if(typeof v555SetGoalTerminal==="function")v555SetGoalTerminal(goal.id,status,{reasonCode:ws.executive.reasonCode||terminal,result:{internal:true,surface:report.surface||null,totalProgress:report.totalProgress,cycles:report.cycles,options:v620CoreClone(options)}});try{state.context ||= {};state.context.lastFreeThoughtTurn={version:NOVA620_CORE_VERSION,goalId:goal.id,activityId:goal.activityId||null,executive:v620CoreClone(ws.executive),canonicalState:v620CoreClone(ws.canonicalSemanticState),at:Date.now()};saveState();}catch(_){ }return {goal:v620CoreClone(typeof v555GoalById==="function"?v555GoalById(goal.id):goal),workspace:ws,report};}finally{V620_ACTIVE_WORKSPACE=prior;}
}

function v623CompactDiscourseSnapshot(mind){
  if(!mind)return {entities:[],events:[],relations:[],propositions:[]};
  const prior=mind.discourseContext||{},merge=(a,b,key)=>v620CoreUnique([...(a||[]),...(b||[])],key).slice(-80);
  const entities=merge(prior.entities,mind.entities,x=>x.identityKey||`${x.frameId||""}|${v620CoreCanon(x.label||x.name||x.id)}`);
  const events=merge(prior.events,mind.events,x=>x.identityKey||`${x.frameId||""}|${x.predicateKey||v620CoreCanon(x.predicateSurface)}|${x.leftArgumentId||x.actorId||""}|${x.rightArgumentId||x.objectId||""}`);
  const relations=merge(prior.relations,mind.relations,x=>`${x.frameId||""}|${x.type||x.predicate||""}|${x.subjectId||x.subject||""}|${x.objectId||x.object||""}|${x.eventId||""}`);
  const propositions=merge(prior.propositions,mind.propositions,x=>`${x.frameId||""}|${x.type||""}|${x.eventId||""}|${v620CoreCanon(x.text||"")}`);
  return {entities,events,relations,propositions,updatedAt:Date.now()};
}
function v623CurrentThoughtSnapshot(ws){
  const mind=ws?.canonicalSemanticState||{};
  if(window.__TAURI__?.core?.invoke){const internal=nova7RustMindBridge?.snapshot?.internalCognition||null,currentGoal=(internal?.goals||[]).filter(goal=>goal.status==="QUEUED"||goal.status==="BLOCKED").at(-1)||null;return {version:NOVA620_CORE_VERSION,authority:"rust_v7_pass_9",input:ws?.rawInput||null,currentGoal:v620CoreClone(currentGoal),currentMindState:null,currentActivity:null,executionMode:"RUST_CURRENT_MIND",terminalState:ws?.executive?.terminalState||null,reasonCode:ws?.executive?.reasonCode||null,cognitiveOutcome:v620CoreClone(mind.cognitiveOutcome||null),updatedAt:Date.now()};}
  let currentGoal=(mind.unresolved||[])[0]||null,currentActivity=null,currentMindState=mind.mentalState||null;
  try{if(typeof v555ActivityStore==="function"){const store=v555ActivityStore(),activeGoal=(store.goals||[]).find(g=>g.status===NOVA555_GOAL_STATUS?.ACTIVE)||null;if(activeGoal)currentGoal=v620CoreClone(activeGoal);if(activeGoal?.activityId&&typeof v555ActivityById==="function")currentActivity=v555ActivityById(activeGoal.activityId);if(!currentActivity&&typeof v555ActiveActivities==="function")currentActivity=v555ActiveActivities()[0]||null;}}catch(_){ }
  try{if(typeof v556CurrentMindState==="function")currentMindState=v556CurrentMindState()||currentMindState;}catch(_){ }
  return {version:NOVA620_CORE_VERSION,input:ws?.rawInput||null,currentGoal:v620CoreClone(currentGoal),currentMindState:v620CoreClone(currentMindState),currentActivity:v620CoreClone(currentActivity),executionMode:ws?.executionMode||"USER_TURN",terminalState:ws?.executive?.terminalState||null,reasonCode:ws?.executive?.reasonCode||null,cognitiveOutcome:v620CoreClone(mind.cognitiveOutcome||null),updatedAt:Date.now()};
}

function v620RecordInspectorState(ws,response){
  try{state.context ||= {};state.context.lastGenericCoreTurn={version:NOVA620_CORE_VERSION,input:ws.rawInput,executive:v620CoreClone(ws.executive),modules:{relevant:ws.executive.details?.relevant||0,relevantReferences:v620CoreClone(ws.executive.details?.relevantReferences||[]),considered:ws.executive.details?.considered||0,applied:ws.executive.details?.applied||0,totalProgress:ws.executive.details?.totalProgress||0},response};state.context.lastUtteranceObservation=v620CoreClone(ws.utteranceObservation);state.context.lastUtteranceMetadata=v620CoreClone(ws.utteranceObservation);state.context.lastCanonicalSemanticState=v620CoreClone(ws.canonicalSemanticState);state.context.currentThought=v623CurrentThoughtSnapshot(ws);state.context.recentSemanticDiscourse=v623CompactDiscourseSnapshot(ws.canonicalSemanticState);state.context.lastSemanticFrame=null;state.context.lastResponsePlan=null;saveState();}catch(_){ }
}

function v628ThinkingTrace(ws,report){
  if(typeof nova4313Thought!=="function")return;
  const mind=ws?.canonicalSemanticState||{};
  const trace=Array.isArray(mind?.cognitiveTrace)?mind.cognitiveTrace:[];
  const kindFor=stage=>({
    observe:"observe",
    interpretation:"interpretation",
    reference:"reference",
    context:"context",
    relationship:"relationship",
    goal:"goal",
    cognition:"cognition",
    unresolved:"unresolved",
    conclusion:"conclusion"
  }[String(stage||"")]||"reasoning");

  // Native V7 Thinking is rendered only from Rust-produced meaning-level
  // cognitive events. Technical semantic/runtime telemetry is recorded in the
  // separate Developer Trace and must not leak back into this panel.
  if(ws?.rustSemanticSeed&&trace.length){
    for(const event of trace){if(event?.text)nova4313Thought(String(event.text),kindFor(event.stage));}
    return;
  }

  // Browser-only migration path: stay concise and meaning-level. This is not a
  // native fallback and does not recreate the old developer telemetry narration.
  const requests=mind.requests||[],unresolved=mind.unresolved||[],conclusions=mind.conclusions||[];
  for(const reference of (mind.references||[]).slice(0,4)){
    if(reference?.resolvedEntity)nova4313Thought(`I resolved "${reference.surface}" to ${reference.resolvedEntity}.`,"reference");
    else nova4313Thought(`I left the reference "${reference?.surface||"?"}" unresolved.`,"reference");
  }
  for(const request of requests.slice(0,3)){
    nova4313Thought(`I treated the current request as ${String(request?.type||request?.requestType||"a request").replace(/_/g," ")} about ${request?.target||"the current message"}.`,"planning");
  }
  if(unresolved.length)nova4313Thought(`I still had not resolved ${String(unresolved[0]?.description||unresolved[0]?.type||"the remaining goal").replace(/_/g," ")}.`,"limitation");
  const conclusion=[...conclusions].reverse().find(item=>typeof (item?.surface??item?.response??item?.text)==="string");
  if(conclusion)nova4313Thought(`From that interpretation, I concluded: ${String(conclusion.surface??conclusion.response??conclusion.text)}`,"conclusion");
  else if(report?.surface)nova4313Thought(`From that interpretation, I concluded: ${report.surface}`,"conclusion");
}

async function v620GenerateAssistantReply(rawText){
  if(window.__TAURI__?.core?.invoke){
    const seed=nova7RustMindSemanticSeed(rawText);
    if(!seed)throw new Error("Rust NovaMind receipt is required; Pass 9 has no JavaScript cognition fallback.");
    let realization=seed.realizationReport||null;
    let cognitiveReport=seed.cognitiveReport||null;
    let canonical=seed.canonicalState||null;
    if(seed.eventKind==="teaching"){
      const taught=await nova7RustMindIngestSource({name:"conversation",text:String(rawText??"")},{kindHint:"conversation",sourceType:"conversation"});
      realization=taught?.realizationReport||realization;
      const snapshot=nova7RustMindBridge?.snapshot||null;
      canonical=snapshot?.cognitiveState||canonical;
      cognitiveReport=snapshot?.lastCognitiveReport||cognitiveReport;
    }
    try{
      if(typeof nova4313BeginThinking==="function")nova4313BeginThinking(String(rawText??""));
      for(const event of canonical?.cognitiveTrace||[]){if(event?.text&&typeof nova4313Thought==="function")nova4313Thought(String(event.text),String(event.stage||"reasoning"));}
      if(typeof nova700RecordDeveloperTrace==="function"){
        const ws={rawInput:String(rawText??""),canonicalSemanticState:canonical,rustSemanticSeed:true,rustSemanticReport:seed.semanticReport||null,rustCognitiveReport:cognitiveReport||null,executive:{terminalState:realization?.terminalState||null,reasonCode:realization?.reasonCode||null,startedAt:Date.now(),details:{authority:"rust_v7_pass_9"}}};
        nova700RecordDeveloperTrace(ws,{surface:realization?.surface||null},realization?.surface||null);
      }
      if(typeof nova4313CompleteThinking==="function")nova4313CompleteThinking();
    }catch(_){ }
    return typeof realization?.surface==="string"&&realization.surface.trim()?realization.surface.trim():null;
  }
  const ws=v620CreateWorkspace(rawText);V620_ACTIVE_WORKSPACE=ws;
  try{
    try{if(typeof nova4313BeginThinking==="function")nova4313BeginThinking(ws.rawInput);}catch(_){ }
    const teaching=v620LooksInstructional(ws.rawInput,ws);let response,report=null;
    if(teaching){response=await v620LearnCurrentMind(ws,teaching);try{if(typeof nova4313Thought==="function")nova4313Thought("I connected the teaching to my current learned structures and updated what the lesson changed.","learning");}catch(_){ }}
    else{report=v620RunCurrentMind(ws);response=report.surface||null;try{v628ThinkingTrace(ws,report);}catch(_){ }}
    try{if(typeof nova700RecordDeveloperTrace==="function")nova700RecordDeveloperTrace(ws,report,response);}catch(_){ }v620RecordInspectorState(ws,response);try{if(typeof nova4313CompleteThinking==="function")nova4313CompleteThinking();}catch(_){ }return response;
  }catch(error){v620SetTerminal(ws,NOVA620_TERMINALS.BLOCKED,NOVA620_REASONS.INTERNAL_ERROR,{message:error?.message||String(error)});try{if(typeof nova4313Thought==="function")nova4313Thought("My reasoning was interrupted before I could produce a response.","limitation");}catch(_){ }const response=null;try{if(typeof nova700RecordDeveloperTrace==="function")nova700RecordDeveloperTrace(ws,{surface:null,error:error?.message||String(error)},response);}catch(_){ }v620RecordInspectorState(ws,response);try{if(typeof nova4313CompleteThinking==="function")nova4313CompleteThinking();}catch(_){ }return response;}
  finally{V620_ACTIVE_WORKSPACE=null;}
}

/* The live cut. No previous generator is retained or invoked. */
var generateAssistantReply=v620GenerateAssistantReply;

/* ---------------------------------------------------------
   CURRENT ARCHITECTURE REGRESSION TESTS
   --------------------------------------------------------- */
async function v620RunRegressionTests(){
  const results=[];const test=async(name,fn)=>{let passed=false,error=null,reason=null;try{const r=await fn();passed=Boolean(r===true||r?.passed);reason=typeof r==="object"?r?.reason:null;}catch(e){error=e?.message||String(e);}results.push({name,passed,error,reason});};
  if(window.__TAURI__?.core?.invoke){
    let snapshot=null;try{snapshot=await window.__TAURI__.core.invoke("nova_mind_snapshot");}catch(error){results.push({name:"Rust NovaMind snapshot is available",passed:false,error:error?.message||String(error),reason:null});}
    await test("Pass 9 native turn authority is Rust",()=>snapshot?.turnLifecycleOwner==="rust_v7_pass_9"&&snapshot?.migrationPhase==="rust_authoritative");
    await test("Pass 9 has no pending JavaScript brain subsystem",()=>Object.values(snapshot?.subsystemOwnership?.owners||{}).every(owner=>String(owner).startsWith("rust_")));
    await test("Pass 9 persistent mind orchestration is Rust-owned",()=>["persistent_mind_payload_orchestration","persistent_mind_serialization","persistent_mind_hydration","persistent_mind_recovery","conversation_history_authority"].every(key=>snapshot?.subsystemOwnership?.owners?.[key]==="rust_v7_pass_9"));
    await test("Pass 9 Rust owns conversation history",()=>Array.isArray(snapshot?.conversationHistory));
    await test("Pass 9 learning state retains synthesis candidates",()=>Array.isArray(snapshot?.learningState?.candidates));
    await test("Native JavaScript free thought refuses execution",async()=>{try{await v621RunFreeThoughtTick();return false;}catch(error){return String(error?.message||error).includes("Rust Pass");}});
    await test("Native JavaScript semantic learner is retired",()=>{try{v450LearnConversation("Learn this: probe",{teaching:{type:"teaching"}});return false;}catch(error){return String(error?.message||error).includes("Rust Pass");}});
    await test("No legacy response fallback is available",()=>v620CreateWorkspace("probe").executive.legacyFallbackAllowed===false);
    const passed=results.filter(x=>x.passed).length;return {version:NOVA620_CORE_VERSION,passed,failed:results.length-passed,total:results.length,results,architecture:"rust_pass_9_authoritative_mind",ranAt:Date.now()};
  }
  const savedState=v620CoreClone(typeof state!=="undefined"?state:null);const cleanupModules=[];const storeModule=m=>{const r=v502StoreModule(m,{sourceType:"test",sourceName:"V7 Pass 8 browser-only regression"});if(r?.module)cleanupModules.push(r.module.id);return r;};
  const teach=(name,text)=>{const source={id:`test_source_${Math.random().toString(36).slice(2,8)}`,name,text,classification:{type:"guidance"},type:"guidance"};const semantic=typeof v450LearnDocument==="function"?v450LearnDocument(source,{currentCore:true,skipCognitiveCompile:true}):null;const cognitive=typeof v534CompileDocumentCognition==="function"?v534CompileDocumentCognition(source,{synthesize:false}):null;const synthesis=typeof v520GeneralSynthesisPass==="function"?v520GeneralSynthesisPass({reason:"v7_pass_4_browser_regression_teaching",force:true}):null;return {semantic,cognitive,synthesis};};
  const run=text=>{const ws=v620CreateWorkspace(text),report=v620RunCurrentMind(ws);return {ws,report,response:report.surface||null};};
  const answerKey=value=>v620CoreCanon(String(value||"").replace(/[.!?]+$/g,""));
  const foundationLesson=`NOVA LESSON — GENERAL SEMANTIC RELATIONSHIPS\nSECTION 1: SEMANTIC STRUCTURE\nA semantic structure preserves relationships in an utterance.\nSECTION 2: ACTOR\nAn actor is the participant filling the acting role in an event.\nSECTION 3: ACTION\nAn action is an event relation connecting participants.\nSECTION 4: OBJECT\nAn object is a participant related to an event through the object role.\nSECTION 5: QUESTION FRAME\nA question frame represents requested information.\nSECTION 6: UNKNOWN SLOT\nAn unknown slot preserves the role whose value is requested.\nSECTION 7: ANSWER RELATION\nAn answer relation identifies which relation must be resolved.\nSECTION 8: NAME\nA name can identify a discourse entity.\nSECTION 9: SEMANTIC FRAME\nA semantic frame provides context for related propositions.`;
  try{
    /* Isolate the regression run in volatile state only. Do not call resetState(),
       saveState(), or native persistence from the test harness. */
    state.messages=[];state.facts=[];state.inferredFacts=[];state.context={lastEntity:"user",lastTopic:null,lastIntent:null,lastQuestionRelation:null,recentEntities:[],currentTopic:null,topicStack:[],salience:{},turn:0,lastSemanticFrame:null,lastResolvedReferences:[],lastParseConfidence:0};v43EnsureCognition();v502ModuleStore();teach("Nova Regression Semantic Foundations Lesson.txt",foundationLesson);
    await test("V7 Pass 8 keeps one current generator while Rust owns native semantics, learned-module execution, and learning executability",()=>generateAssistantReply===v620GenerateAssistantReply);
    await test("generic acknowledgement fallback is not part of the current foundation",()=>!(v502ModuleStore().modules||[]).some(m=>m.id==="core_generic_conversation_realization")&&!NOVA502_MODULE_OPS.has("conversation_acknowledge_understood"));
    await test("partial semantic construction cannot certify understanding by itself",()=>{const key=`partial${Math.random().toString(36).slice(2,8)}`;storeModule({name:`partial meaning ${key}`,categories:["semantic",key],priority:130,conditions:[{type:"contains_any",values:[key]}],program:[{op:"semantic_add_entity",args:{id:`entity_${key}`,label:key,type:"probe_entity"}}]});const r=run(`${key} unresolved`);return r.response===null&&r.ws.executive.terminalState===NOVA620_TERMINALS.BLOCKED&&r.ws.canonicalSemanticState.cognitiveOutcome?.meaningConstructed===true;});
    await test("surface observation still contains no lexical meaning table",()=>{const source=v620ObserveUtterance.toString();return !/Suppose|Assume|Imagine|Who|Which person|hypothetical|question_word/.test(source)&&v620ObserveUtterance("Suppose Mira opened it?").note.includes("No lexical item");});
    await test("migrated semantic built-ins and operations are absent from the JavaScript brain",()=>{const modules=v502ModuleStore().modules||[];return !modules.some(m=>["core_generic_semantic_construction","core_generic_relational_reasoning"].includes(m.id))&&!NOVA502_MODULE_OPS.has("semantic_construct_relational_surface")&&!NOVA502_MODULE_OPS.has("reason_resolve_relational_request");});
    await test("a Rust canonical conclusion remains authoritative in Pass 8 UI coordination",()=>{const seed={canonicalState:{version:"7.0.0-rust-authoritative-mind-pass-9",schemaVersion:3,id:"probe",sourceRaw:"probe",surface:{quotes:[],segments:[],sentences:[],tokens:[]},participants:v620ParticipantFrame(),scopes:[],activeScopeId:"scope_discourse",references:[],frames:[],entities:[],events:[],relations:[],propositions:[],requests:[],unknownSlots:[],candidateSets:[],constraints:[],evidence:[],conclusions:[{surface:"Rust answer.",rustAuthoritative:true}],structures:[],categories:[],activeCategories:[],activeConcepts:[],discourseContext:{entities:[],events:[],relations:[],propositions:[]},semanticTrace:[],unresolved:[],status:"SUPPORTED"},semanticReport:{suggestedSurface:"Rust answer."}},mind=v700AdoptRustCanonicalState("probe",seed),ws={canonicalSemanticState:mind,moduleState:{surfaceResponse:"JS answer."}};return mind.semanticOwner==="rust_v7_pass_9"&&v620ExtractSurface(ws)==="Rust answer.";});
    await test("Thinking renders Rust meaning-level cognitiveTrace and excludes developer semantic telemetry",()=>{nova4313BeginThinking("trace split probe");const seed={canonicalState:{version:"7.0.0-rust-authoritative-mind-pass-9",schemaVersion:4,id:"trace_probe",sourceRaw:"trace split probe",surface:{quotes:[],segments:[],sentences:[],tokens:[]},participants:v620ParticipantFrame(),conversationContext:[],scopes:[],activeScopeId:"scope_discourse",references:[],frames:[],entities:[],events:[],relations:[],propositions:[],requests:[],unknownSlots:[],candidateSets:[],constraints:[],evidence:[],conclusions:[],structures:[],categories:[],activeCategories:[],activeConcepts:[],discourseContext:{entities:[],events:[],relations:[],propositions:[]},semanticTrace:[{stage:"request_reasoning",detail:"The Rust semantic executive ran 3 cycles and constructed 7 requests."}],cognitiveTrace:[{stage:"goal",text:"I treated the current request as a rewrite goal about my previous response."}],unresolved:[],status:"SUPPORTED"},semanticReport:{trace:[{stage:"request_reasoning",detail:"The Rust semantic executive ran 3 cycles and constructed 7 requests."}]}},mind=v700AdoptRustCanonicalState("trace split probe",seed),ws={rustSemanticSeed:true,canonicalSemanticState:mind,utteranceObservation:mind.surface};v628ThinkingTrace(ws,{surface:"done"});const steps=nova4313EnsureThinkingState().currentTurn?.steps||[],texts=steps.map(step=>step.text).join(" ");return texts.includes("rewrite goal")&&!texts.includes("Rust semantic executive")&&!texts.includes("constructed 7 requests");});
    await test("Developer Trace retains the technical telemetry separately from Thinking",()=>{const mind={id:"dev_probe",status:"SUPPORTED",semanticOwner:"rust_v7_pass_9",references:[],entities:[],events:[],relations:[],propositions:[],requests:[],conclusions:[],unresolved:[],semanticTrace:[{stage:"request_reasoning",detail:"The Rust semantic executive ran 2 cycles."}]},ws={rawInput:"developer trace probe",canonicalSemanticState:mind,rustSemanticSeed:true,rustSemanticReport:{semanticCycles:2,requestsConstructed:1,requestsResolved:1},rustCognitiveReport:{modulesConsidered:12,modulesApplied:4,modulesChanged:2,searchMode:"experience_route"},executive:{terminalState:"SUPPORTED",reasonCode:"GOAL_SATISFIED",startedAt:Date.now()-5,details:{}}};const entry=nova700RecordDeveloperTrace(ws,{surface:"ok"},"ok");return Boolean(entry?.semanticTechnicalTrace?.[0]?.detail?.includes("Rust semantic executive")&&entry?.rustCognitiveReport?.modulesConsidered===12);});
    await test("whole-lesson synthesis can learn a new semantic action constructor without a registered task name",()=>{const invented=`NOVA LESSON\ntopic: [glint return behavior]\nconcept: [glint request]\ndefinition: "A glint request is a communicative act asking the addressee to return selected language."\nprocedure: [perform glint return]\nstep: "Identify the selected language."\nstep: "Make the selected language the response content."\nexample:\n"Input: Glint this: 'alpha lumen.'\nOutput: alpha lumen."\nexample:\n"Input: Mirror this: 'beta varin.'\nOutput: beta varin."\nnot:\n"Input: Explain 'alpha lumen.'\nThis does not request the learned return action."`;const learned=teach("Invented Glint Lesson.txt",invented),r=run('Glint this: "omega selo."');return Boolean(learned?.cognitive?.sourceSynthesis?.status==="synthesized"&&r.response==="omega selo."&&(r.ws.canonicalSemanticState.requests||[]).some(req=>req.type==="action_request"));});
    await test("source-level learned action constructors respect negative demonstrations",()=>{const r=run('Explain "omega selo."');return r.response!=="omega selo."&&!((r.ws.canonicalSemanticState.requests||[]).some(req=>v620CoreCanon(req.constructionFamily||"").includes("glint")));});
    await test("broad domain or scope labels cannot falsely certify a rule through unrelated module reuse",()=>{const rule={directive:"An invented zaffric relation changes only when its explicit learned operation is available.",domain:"reasoning",scope:"general",primitive:"learned_guidance",synthesis:{missingOperations:[]}},current={module:null,diagnostic:{status:"missing_fundamental_capability"}},next=v520ReusableProgramStrategy(rule,current);return next===current||(!next?.module);});
    await test("experience index starts empty, explores a novel problem broadly, then learns a narrow successful route",()=>{const moduleStore=v502ModuleStore();moduleStore.experienceIndex=null;v622EnsureExperienceIndex(moduleStore);const key=`relevant${Math.random().toString(36).slice(2,7)}`;const target=storeModule({name:`semantic names ${key}`,categories:["semantic","names"],priority:140,conditions:[{type:"contains_any",values:[key]}],program:[{op:"set_field",args:{path:"learnedState.indexProbe",value:key}},{op:"emit_surface",args:{text:"route learned"}}]});for(let i=0;i<300;i++)storeModule({name:`unrelated_${i}_${Math.random().toString(36).slice(2,5)}`,categories:[`unrelated_${i}`],conditions:[{type:"contains_any",values:[`never_match_${i}`]}],program:[{op:"set_field",args:{path:`learnedState.unrelated_${i}`,value:true}}]});const before=v622CognitiveMapSummary().experience.totalRoutes,firstWs=v620CreateWorkspace(`${key} first experience`),first=v620RunCurrentMind(firstWs),afterFirst=v622CognitiveMapSummary().experience.totalRoutes,secondWs=v620CreateWorkspace(`${key} similar experience`),second=v620RunCurrentMind(secondWs);return before===0&&first.surface==="route learned"&&firstWs.cognitiveSearch.globalExplored===true&&first.considered.length>250&&afterFirst===1&&second.surface==="route learned"&&secondWs.cognitiveSearch.mode==="experience_route"&&second.considered.length<45&&second.references.some(x=>x.number===target.module.moduleNumber);});
    await test("an experienced route can broaden when it stops making progress and learn the newly useful module",()=>{const store=v502ModuleStore();store.experienceIndex=null;v622EnsureExperienceIndex(store);const topic=`cobalt${Math.random().toString(36).slice(2,7)}`,alpha=`alpha${Math.random().toString(36).slice(2,6)}`,beta=`beta${Math.random().toString(36).slice(2,6)}`;const a=storeModule({name:`route a ${topic}`,categories:["probe"],priority:140,conditions:[{type:"contains_any",values:[alpha]}],program:[{op:"emit_surface",args:{text:"A"}}]}),b=storeModule({name:`route b ${topic}`,categories:["probe"],priority:140,conditions:[{type:"contains_any",values:[beta]}],program:[{op:"emit_surface",args:{text:"B"}}]});const first=run(`${topic} ${alpha}`),second=run(`${topic} ${beta}`),route=v622CognitiveMapSummary().experience.routes[0];return first.response==="A"&&second.response==="B"&&second.ws.executive.cycles.some(c=>c.broadenedAfterExperiencedRoute===true)&&(route?.moduleNumbers||[]).includes(a.module.moduleNumber)&&(route?.moduleNumbers||[]).includes(b.module.moduleNumber);});
    await test("learned modules remain reusable primitives for later learned modules",()=>{const key=`reuse${Math.random().toString(36).slice(2,7)}`,base=storeModule({name:`base ${key}`,categories:[key],conditions:[{type:"always"}],program:[{op:"set_field",args:{path:"learnedState.reused",value:key}}]}),caller=storeModule({name:`caller ${key}`,categories:["test"],priority:100,conditions:[{type:"always"}],program:[{op:"invoke_module",args:{number:base.module.moduleNumber}}]});const ws=v620CreateWorkspace("reuse probe"),ctx=v620ModuleContext(ws);v502RunCognitiveModules("test",ctx);return caller.stored&&ws.canonicalSemanticState.learnedState.reused===key;});
    await test("Current Thought snapshot exposes goal, mind state, and activity fields",()=>{const thought=v623CurrentThoughtSnapshot(v620CreateWorkspace("thought probe"));return Boolean(thought&&Object.prototype.hasOwnProperty.call(thought,"currentGoal")&&Object.prototype.hasOwnProperty.call(thought,"currentMindState")&&Object.prototype.hasOwnProperty.call(thought,"currentActivity"));});
    await test("blocked turns have no hardcoded chat fallback",()=>{const key=`nofallback${Math.random().toString(36).slice(2,8)}`;storeModule({name:`partial no-fallback ${key}`,categories:["semantic",key],priority:130,conditions:[{type:"contains_any",values:[key]}],program:[{op:"semantic_add_entity",args:{id:`entity_${key}`,label:key,type:"probe_entity"}}]});const r=run(`${key} unresolved`);return r.response===null&&r.ws.executive.terminalState===NOVA620_TERMINALS.BLOCKED&&typeof v620FailureResponse==="undefined";});
    await test("a stalled mind exposes its cognitive outcome to current learned realization before terminating",()=>{const key=`outcome${Math.random().toString(36).slice(2,8)}`;storeModule({name:`partial outcome ${key}`,categories:["semantic",key],priority:130,conditions:[{type:"contains_any",values:[key]}],program:[{op:"semantic_add_entity",args:{id:`entity_${key}`,label:key,type:"probe_entity"}}]});const probe=storeModule({name:`learned outcome response ${key}`,categories:["realization","communication",key],priority:120,conditions:[{type:"field_equals",field:"cognitiveOutcome.state",value:"blocked_with_partial_meaning"}],program:[{op:"emit_surface",args:{text:"learned outcome response"}}]});const r=run(`${key} unresolved`);return Boolean(probe.stored&&r.response==="learned outcome response"&&r.ws.canonicalSemanticState.cognitiveOutcome?.responseProduced===true&&r.ws.executive.terminalState===NOVA620_TERMINALS.SUPPORTED);});
    await test("uninterpreted turns keep the technical state internal when no learned response is produced",()=>{const r=run("qxzv ungrounded probe");const thought=v623CurrentThoughtSnapshot(r.ws);return r.response===null&&thought.reasonCode===NOVA620_REASONS.NO_INTERPRETATION&&r.ws.canonicalSemanticState.cognitiveOutcome?.state==="interpretation_incomplete";});
    await test("learning acknowledgements do not expose synthesis implementation language",()=>{const reply=v620LearningResponse({report:{learnedUnits:1}},{upgraded:0,remaining:2});return !/Stored Only|general machinery|cognitive item|executable cognition|synthesis/i.test(reply);});
    await test("structural quote scopes prevent outer participant leakage before learned quote cognition runs",()=>{const ws=v620CreateWorkspace('The example says "I opened it."');const inner=(ws.canonicalSemanticState.references||[]).find(r=>v620CoreCanon(r.surface)==="i");return Boolean(inner&&inner.scopeId!=="scope_discourse"&&inner.resolvedEntity==null&&inner.locked===false&&inner.resolutionSource==="scoped_deictic_unresolved");});
    await test("generic scoped-context synthesis can upgrade quotation participant-frame guidance",()=>{const directives=[
      "Quotation marks clarify the content boundary but do not replace the teaching label.",
      "Resolve pronouns inside quoted speech from the quoted participant frame when that frame is known.",
      "Do not automatically resolve quoted first-person or second-person forms from the outer conversation frame.",
      "Do not leak outer participant mappings into a quote without evidence.",
      "Do not keep quoted participant mappings active after the quote ends.",
      "Resolve first-person and second-person references inside the quote from that quoted frame.",
      "First-person realization consistency means Nova normally refers to herself with first-person forms when Nova is the response speaker and no contrast requires third-person naming."
    ];const created=directives.map((directive,i)=>v500StoreCognitiveRule({directive,domain:"general",scope:"general"},{sourceType:"learned",sourceName:"V6.2.16 scoped-context regression",sourceId:`scope_reg_${i}`}));v520GeneralSynthesisPass({reason:"v6.2.16_scoped_context_regression",force:true});return created.every(rule=>v500CognitiveStore().rules.find(r=>r.id===rule.id)?.status==="module_executable");});
    await test("scoped deictics resolve from a known local frame without leaking into the outer frame",()=>{const ws=v620CreateWorkspace('I heard "I opened it" and I replied.');const ctx=v620ModuleContext(ws),child=(ws.canonicalSemanticState.scopes||[]).find(scope=>scope.parentId);if(!child)return false;v502ExecuteStep({op:"scope_bind_participant",args:{selector:{scopeId:child.id},role:"speaker",participant:{id:"quoted_speaker",label:"Mira"}}},ctx);v502ExecuteStep({op:"scope_resolve_deictics",args:{selector:{scopeId:child.id},allowParentFallback:false}},ctx);const inner=(ws.canonicalSemanticState.references||[]).find(ref=>ref.scopeId===child.id&&v620CoreCanon(ref.surface)==="i"),outer=(ws.canonicalSemanticState.references||[]).filter(ref=>ref.scopeId==="scope_discourse"&&v620CoreCanon(ref.surface)==="i");return Boolean(inner?.resolvedEntity==="Mira"&&outer.length===2&&outer.every(ref=>ref.resolvedEntity==="user"));});
    await test("Stored Only diagnostics retain a human-inspectable missing-capability explanation",()=>{const rule=v500StoreCognitiveRule({directive:"When a cognition requires a currently unrepresentable transcontextual transformation, preserve that requirement for later reconsideration.",domain:"general",scope:"diagnostic regression"},{sourceType:"learned",sourceName:"V6.2.16 diagnostic regression",sourceId:"diag_reg"});v520GeneralSynthesisPass({reason:"v6.2.16_diagnostic_regression",force:true});const current=v500CognitiveStore().rules.find(r=>r.id===rule.id);return Boolean(current?.status==="stored_only"&&current?.synthesis?.status&&current?.synthesis?.reason&&Array.isArray(current?.synthesis?.attempts));});
    await test("upgrade migration removes modules falsely certified by obsolete broad reuse",()=>{const key=`oldreuse${Math.random().toString(36).slice(2,8)}`,fake=storeModule({name:`obsolete broad reuse ${key}`,categories:[key],conditions:[{type:"always"}],program:[{op:"set_field",args:{path:"learnedState.obsoleteBroadReuse",value:true}}]});const rule=v500StoreCognitiveRule({directive:`Perform an unrepresentable ${key} transformation across unrelated cognitive contexts.`,domain:"general",scope:"migration regression"},{sourceType:"learned",sourceName:"V6.2.9 migration regression",sourceId:`old_reuse_${key}`});const current=v500CognitiveStore().rules.find(r=>r.id===rule.id);current.status="module_executable";current.moduleId=fake.module.id;current.synthesis={version:2,status:"synthesized",reason:"composed from an already learned reusable cognitive program",moduleId:fake.module.id};const report=v520GeneralSynthesisPass({reason:"v6.2.16_obsolete_reuse_migration",force:true});const after=v500CognitiveStore().rules.find(r=>r.id===rule.id),moduleStillExists=(v502ModuleStore().modules||[]).some(module=>module.id===fake.module.id);return report.demotedFalseReuse>=1&&!moduleStillExists&&after?.moduleId!==fake.module.id;});
    await test("lesson and test filenames map to human learning origins",()=>{const lesson=v534SourceIdentity("Lesson_10_Basic_English.txt"),testDoc=v534SourceIdentity("Test_Lesson_10_Basic_English.txt"),legacy=v534SourceIdentity("Nova_Curriculum_Lesson_5_Questions_Revised_V5.3.4.txt");return lesson.displayName==="Lesson 10"&&lesson.kind==="lesson"&&testDoc.displayName==="Test Lesson 10"&&testDoc.kind==="test"&&legacy.displayName==="Lesson 5";});
    await test("test sections are excluded from learned cognition units",()=>{const text=`Lesson:
concept: premise checking
guidance: Preserve unsupported assumptions as unresolved.
Test:
question: Which hotel is Mira using?
expected answer: unknown
test instruction: verify the answer without accepting unsupported premises.`;const units=v534StructuredLessonUnits(text,{sourceName:"Lesson_10_Premises.txt"});return units.some(unit=>/preserve unsupported assumptions/i.test(unit.text))&&!units.some(unit=>/hotel|verify the answer|unknown/i.test(unit.text));});
    await test("curriculum tests ask one visible-turn question at a time before consulting the answer key",async()=>{const text=`Question: Suppose Mira opened the red box and Tavik opened the blue box. Which person opened the red box?
Expected Answer: Mira`;const source={id:"reg_test_lesson_10",name:"Test_Lesson_10_Semantics.txt",text},structure=v534ParseDocumentStructure(text,{sourceName:source.name,identity:v534SourceIdentity(source.name)}),suite=v534ExtractTestSuite(source,structure,v534SourceIdentity(source.name));const asked=[];const testRun=await v534RunInteractiveTestSuite(suite,{persist:false,askQuestion:async(question,meta)=>{asked.push({question,meta});return {response:"Mira",terminal:NOVA620_TERMINALS.SUPPORTED,reasonCode:NOVA620_REASONS.GOAL_SATISFIED};}});return suite.sourceDisplayName==="Test Lesson 10"&&asked.length===1&&asked[0].question===suite.cases[0].question&&asked[0].meta?.sourceLabel==="Teacher"&&!Object.prototype.hasOwnProperty.call(asked[0].meta||{},"expectedAnswer")&&testRun.total===1&&testRun.passed===1&&answerKey(testRun.results[0].response)==="mira";});
    await test("curriculum auto grading accepts harmless answer wrapping but sends ambiguous paraphrases to review",()=>{const wrapped=v534GradeTestCase({expectedAnswer:"Mira"},"The answer is Mira."),ambiguous=v534GradeTestCase({expectedAnswer:"A structured teaching source whose content has different roles such as concepts, definitions, guidance, examples, procedures, and tests."},"A lesson is material used for learning."),missing=v534GradeTestCase({expectedAnswer:"Mira"},null);return wrapped.status==="pass"&&wrapped.autoStatus==="pass"&&ambiguous.status==="review"&&missing.status==="fail";});
    await test("manual curriculum grading overrides the effective score without erasing the automatic grade",()=>{const row={id:"test_1",...v534GradeTestCase({expectedAnswer:"Mira"},"Tavik")},run={results:[null],passed:0,failed:0,review:0};run.results[0]=row;row.manualStatus="pass";v534RecountTestRun(run);return row.autoStatus==="fail"&&row.status==="pass"&&row.gradeSource==="manual"&&run.passed===1&&run.failed===0;});
    await test("generic concept primitives can open a learned concept and compose a supported proposition",()=>{const concept=v450GetOrCreateConcept("velorin",{fromClaim:true}),claim=v450StoreClaim({subject:"velorin",predicate:"is",object:"a structured probe concept"},{sourceName:"concept primitive regression",text:"velorin is a structured probe concept"});if(!concept||!claim)return false;const ws=v620CreateWorkspace("concept primitive probe"),ctx=v620ModuleContext(ws);ws.canonicalSemanticState.unresolved=[{type:"concept_description",description:"explain velorin"}];v502ExecuteStep({op:"knowledge_resolve_concept",args:{value:"velorin",outputPath:"moduleState.focusConcept"}},ctx);v502ExecuteStep({op:"knowledge_collect_relations",args:{conceptPath:"moduleState.focusConcept",depth:2,outputPath:"moduleState.focusRelations"}},ctx);v502ExecuteStep({op:"knowledge_select_relevant_relations",args:{listPath:"moduleState.focusRelations",goal:"explain velorin",outputPath:"moduleState.relevantRelations"}},ctx);v502ExecuteStep({op:"knowledge_form_supported_proposition",args:{listPath:"moduleState.relevantRelations",outputPath:"moduleState.supportedProposition",commit:true}},ctx);return ws.moduleState.focusConcept?.key==="velorin"&&ws.moduleState.supportedProposition?.object==="a structured probe concept";});
    await test("experience signatures prefer learned concepts over raw word matching when semantic memory can ground the surface",()=>{const key=`conceptsig${Math.random().toString(36).slice(2,7)}`;v450GetOrCreateConcept(key,{fromClaim:true});const ws=v620CreateWorkspace(`Please explain ${key}`),sig=v622ExperienceSignature({workspace:ws,utteranceObservation:ws.utteranceObservation,canonicalSemanticState:ws.canonicalSemanticState,cognitiveState:ws.canonicalSemanticState});return sig.conceptTerms.includes(v620CoreCanon(key))&&sig.fallbackTerms.length===0;});
    await test("experience routes retain per-module contribution strengths instead of treating every considered module as equally useful",()=>{const ms=v502ModuleStore();ms.experienceIndex=null;v622EnsureExperienceIndex(ms);const key=`strength${Math.random().toString(36).slice(2,7)}`,target=storeModule({name:`strength route ${key}`,categories:[key],priority:145,conditions:[{type:"contains_any",values:[key]}],program:[{op:"set_field",args:{path:"learnedState.strengthProbe",value:key}},{op:"emit_surface",args:{text:"strength ok"}}]});const r=run(key),route=v622CognitiveMapSummary().experience.routes[0],weight=Number(route?.moduleStrengths?.[target.module.moduleNumber]||0);return r.response==="strength ok"&&weight>0&&weight<=1&&route?.lastContribution?.some(row=>row.number===target.module.moduleNumber&&row.deltaScore>=1);});
    await test("Stored Only capability dependencies are represented as a producer/waiter graph for later reconsideration",()=>{const key=`dep${Math.random().toString(36).slice(2,7)}`,rule=v500StoreCognitiveRule({directive:`Preserve the ${key} transformation until its cognitive form exists.`,domain:"reasoning",scope:"dependency probe",cognitiveForm:key},{sourceType:"learned",sourceName:"dependency probe",sourceId:`dep_rule_${key}`});rule.status="stored_only";rule.synthesis={version:NOVA520_SYNTHESIS_SCHEMA_VERSION,status:"missing_fundamental_capability",missingCapabilities:[`cognitive_form:${key}`],missingOperations:[]};const module=storeModule({name:`producer ${key}`,domain:"reasoning",phase:"reasoning",categories:[key],cognitiveForm:key,conditions:[{type:"always"}],program:[{op:"set_field",args:{path:"learnedState.depProbe",value:true}}]});const graph=v520RebuildCapabilityGraph(v500CognitiveRuleInventory().rules);const cap=`cognitive_form:${v620CoreCanon(key)}`;return Boolean(module.stored&&(graph.producers?.[cap]||[]).some(x=>x.moduleId===module.module.id)&&(graph.waiters?.[cap]||[]).some(x=>x.ruleId===rule.id));});
    await test("development episodes preserve compact successful reasoning traces for later consolidation",()=>{const key=`episode${Math.random().toString(36).slice(2,7)}`,module=storeModule({name:`episode ${key}`,categories:[key],conditions:[{type:"always"}],program:[{op:"set_field",args:{path:"learnedState.episodeProbe",value:true}}]}).module,ws=v620CreateWorkspace(key);v620SetTerminal(ws,NOVA620_TERMINALS.SUPPORTED,NOVA620_REASONS.GOAL_SATISFIED,{});const ep=v554RecordDevelopmentEpisode(ws,{changed:[{number:module.moduleNumber,name:module.name,progress:true,deltaScore:3}],cycles:2});return Boolean(ep?.supported&&ep.contributors?.[0]?.number===module.moduleNumber&&ep.contributors?.[0]?.deltaScore===3&&v554EpisodeStore().episodes.some(x=>x.id===ep.id));});
    await test("repeated successful episodes can form a provisional experience abstraction without becoming executable authority",()=>{const es=v554EpisodeStore(),key=`absep${Math.random().toString(36).slice(2,7)}`,module=storeModule({name:`abstraction episode ${key}`,categories:[key],conditions:[{type:"always"}],program:[{op:"set_field",args:{path:"learnedState.absProbe",value:true}}]}).module;for(let i=0;i<3;i++)es.episodes.push({id:`ep_${key}_${i}`,promptHash:`${key}_${i}`,supported:true,goalTerms:["describe_concept"],conceptTerms:[key],relationTerms:[],contributors:[{number:module.moduleNumber,deltaScore:2}],createdAt:Date.now()+i});const candidates=v554ExperienceCandidates();const hit=candidates.find(x=>x.kind==="experience_pattern"&&x.template?.moduleNumbers?.includes(module.moduleNumber));return Boolean(hit&&hit.status==="PROVISIONAL"&&hit.support.instances>=3);});
    await test("catch-all unmapped synthesis is never advertised as a reusable learned primitive",()=>!(v502ModuleStore().modules||[]).some(module=>(module.produces||[]).some(value=>v620CoreCanon(value)==="unmapped_cognitive_state_transformation")||(module.capabilities||[]).some(value=>v620CoreCanon(value)==="unmapped_cognitive_state_transformation")));
    await test("no legacy response fallback is available",()=>v620CreateWorkspace("probe").executive.legacyFallbackAllowed===false);
  } finally {
    try{if(savedState&&typeof savedState==="object"&&typeof state!=="undefined"){for(const key of Object.keys(state))delete state[key];Object.assign(state,v620CoreClone(savedState));}}catch(_){ }
  }
  const passed=results.filter(x=>x.passed).length;return {version:NOVA620_CORE_VERSION,passed,failed:results.length-passed,total:results.length,results,architecture:"rust_pass_9_authoritative_mind",ranAt:Date.now()};
};

try{if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){V41_SELF_MODEL.identity.version=NOVA620_CORE_VERSION;V41_SELF_MODEL.architecture=["lossless surface observation","one canonical cognitive state","bootstrap participant frame","open learned category graph","generic cognitive module kernel","plastic experience-built cognitive index","iterative central executive","developmental consolidation","developmental learning and memory","trusted external capability boundary"];V41_SELF_MODEL.capabilities ||= [];const entry={id:"generic_module_cognitive_core",verb:"develop",object:"new cognitive connections and categories through teaching without older response fallbacks",detail:"Semantic structures, reasoning state, learned categories, and learned programs operate in one current state. My executive can start from cognitive routes learned from successful experience, broaden beyond those routes when progress stalls, and reflect on successful execution traces to improve future attention while repeatedly reconsidering the shared state as learned cognition changes it."},existing=V41_SELF_MODEL.capabilities.find(c=>c.id===entry.id);if(existing)Object.assign(existing,entry);else V41_SELF_MODEL.capabilities.push(entry);}}catch(_){ }

console.info("Nova V7 Pass 9 UI coordinator loaded — native Tauri cognition and persistence are Rust-owned; JavaScript coordinates UI/IPC only.");
