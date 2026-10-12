"use strict";

/* =========================================================
   Nova V7 Pass 4 — LEARNED MODULE CATALOG / BROWSER MIGRATION KERNEL

   This file remains the learned-program catalog and synthesis vocabulary. In native Tauri turns, execution is authoritative in Rust Pass 4; the JavaScript interpreter below is retained only for browser/localhost migration tests and is never a native fallback.

   Design rules:
   - one interpreter; extensions register capabilities instead of replacing it;
   - bootstrap categories are starting organization, not a closed ontology;
   - learned categories are ordinary data and require no JavaScript registration;
   - modules are composed from generic state/relationship/control operations;
   - stable numbered module descriptors remain structural addresses; an experience-built index learns useful routes only after successful cognition;
   - learned modules may invoke other learned modules, so later learning can make
     earlier Stored Only knowledge executable without adding lesson-specific code;
   - semantic changes are operations in this same cognitive kernel, not a second
     semantic mind layered in front of reasoning.
   ========================================================= */

const NOVA502_MODULE_SCHEMA_VERSION=3;
const NOVA620_VERSION="6.2.16";
const NOVA502_MODULE_PHASES=new Set([
  "interpretation","reference","composition","reasoning","memory","planning",
  "validation","behavior","realization","learning","language","mental_state"
]);
const NOVA502_MODULE_OPS=new Set();
const NOVA620_OPERATION_HANDLERS=new Map();
const NOVA620_CONDITION_HANDLERS=new Map();
const NOVA620_MAX_INVOKE_DEPTH=12;

/* Compact module descriptors + experience-built cognitive routes.

   Every module keeps a tiny persistent descriptor (#388 — semantic, names),
   but descriptors are NOT Nova's learned index.  The experience index starts
   empty.  On a semantically novel turn Nova explores the current module store
   broadly.  After a genuinely supported response, the current mind reflects on
   which modules actually changed cognition and records that successful route.

   A learned route guides attention on later similar problems.  It never closes
   the door to other cognition: if the route stalls, the same current executive
   broadens back into exploration and can revise the route after success. */
const NOVA622_REFERENCE_SCHEMA=3;
const NOVA622_EXPERIENCE_SCHEMA=2;
const NOVA622_MAX_EXPERIENCE_ROUTES=600;
const NOVA622_MAX_ROUTE_MODULES=48;
const NOVA622_MAX_WORKING_SET=220;
const NOVA622_ROUTE_MIN_CONFIDENCE=0.12;
const NOVA622_ROUTE_FAILURE_DECAY=0.08;
const NOVA622_REFERENCE_STOP_WORDS=new Set([
  "a","an","and","as","at","by","for","from","in","into","is","it","of","on","or","the","to","with",
  "module","cognitive","cognition","learned","current","generic","general"
]);
const NOVA622_TOPIC_STOP_WORDS=new Set([
  ...NOVA622_REFERENCE_STOP_WORDS,
  "what","who","why","how","which","where","when","whose","whom","does","do","did","done","are","was","were","be","been","being",
  "can","could","would","should","will","may","might","please","tell","explain","answer","question","questions","say","said","this","that","these","those","me","you","your"
]);
let NOVA622_RUNTIME_INDEX_REVISION=-1;
const NOVA622_MODULE_BY_NUMBER=new Map();
const NOVA622_MODULE_BY_ID=new Map();
const NOVA622_MODULE_BY_NAME=new Map();
const NOVA622_MODULES_BY_CATEGORY=new Map();
const NOVA622_REFERENCE_BY_NUMBER=new Map();

function v502Clone(value){try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(__){return value;}}}
function v502Canonical(value){try{return canonical(String(value??""));}catch(_){return String(value??"").trim().toLowerCase();}}
function v502Clean(value){try{return normalizeWhitespace(String(value??""));}catch(_){return String(value??"").replace(/\s+/g," ").trim();}}
function v502SafeId(value){return v502Canonical(value||"item").replace(/[^a-z0-9]+/g,"_").replace(/^_+|_+$/g,"").slice(0,80)||"item";}
function v502Unique(values,keyFn=x=>typeof x==="object"?JSON.stringify(x):v502Canonical(x)){const out=[],seen=new Set();for(const value of values||[]){if(value==null)continue;let key="";try{key=keyFn(value);}catch(_){key=String(value);}if(!key||seen.has(key))continue;seen.add(key);out.push(value);}return out;}
function v502PathParts(path){const unsafe=new Set(["__proto__","prototype","constructor"]);return String(path||"").split(".").map(x=>x.trim()).filter(Boolean).filter(x=>!unsafe.has(x));}
function v502GetPath(obj,path){let cur=obj;for(const part of v502PathParts(path)){if(cur==null)return undefined;cur=cur[part];}return cur;}
function v502Same(a,b){try{return JSON.stringify(a)===JSON.stringify(b);}catch(_){return a===b;}}
function v502SetPath(obj,path,value){const parts=v502PathParts(path);if(!parts.length)return false;let cur=obj;for(let i=0;i<parts.length-1;i++){const key=parts[i];if(!cur[key]||typeof cur[key]!=="object")cur[key]={};cur=cur[key];}const key=parts.at(-1);if(v502Same(cur[key],value))return false;cur[key]=value;return true;}
function v502DeletePath(obj,path){const parts=v502PathParts(path);if(!parts.length)return false;let cur=obj;for(let i=0;i<parts.length-1;i++){cur=cur?.[parts[i]];if(!cur)return false;}const key=parts.at(-1);if(!Object.prototype.hasOwnProperty.call(cur,key))return false;delete cur[key];return true;}
function v502AppendPath(obj,path,value,{unique=false}={}){let list=v502GetPath(obj,path);if(!Array.isArray(list)){v502SetPath(obj,path,[]);list=v502GetPath(obj,path);}if(!Array.isArray(list))return false;if(unique&&list.some(x=>v502Same(x,value)))return false;list.push(value);return true;}

/* ---------------------------------------------------------
   OPEN CATEGORY GRAPH
   --------------------------------------------------------- */
function v620EnsureCategoryStore(){
  const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return {schema:1,version:NOVA620_VERSION,categories:[],relations:[]};
  const prior=memory.cognitiveCategories||{};
  if(prior.schema!==1)memory.cognitiveCategories={schema:1,version:NOVA620_VERSION,categories:Array.isArray(prior.categories)?prior.categories:[],relations:Array.isArray(prior.relations)?prior.relations:[],createdAt:prior.createdAt||Date.now(),updatedAt:Date.now()};
  memory.cognitiveCategories.version=NOVA620_VERSION;return memory.cognitiveCategories;
}
function v620RegisterCategory(name,meta={}){
  if(window.__TAURI__?.core?.invoke&&typeof nova7RustMindBridge!=="undefined"&&nova7RustMindBridge?.initialized){const label=v502Clean(name),key=v502Canonical(label),store=v620EnsureCategoryStore();return (store.categories||[]).find(x=>v502Canonical(x.name||x.key)===key)||null;}
  const label=v502Clean(name);if(!label)return null;const store=v620EnsureCategoryStore(),key=v502Canonical(label);let category=(store.categories||[]).find(x=>v502Canonical(x.name)===key);
  if(!category){category={id:`cat_${v502SafeId(label)}_${Math.random().toString(36).slice(2,7)}`,name:label,origin:meta.origin||"learned",status:meta.status||"ACTIVE",traits:[],evidence:[],activationCount:0,successfulUseCount:0,strength:Number(meta.strength??0.5),createdAt:Date.now(),updatedAt:Date.now()};store.categories.push(category);}
  category.activationCount=Number(category.activationCount||0)+Number(meta.activationIncrement||0);category.successfulUseCount=Number(category.successfulUseCount||0)+Number(meta.successIncrement||0);
  if(meta.strength!=null)category.strength=Math.max(0,Math.min(1,Number(meta.strength)));else if(category.strength==null)category.strength=0.5;
  category.traits=v502Unique([...(category.traits||[]),...(meta.traits||[])],x=>v502Canonical(typeof x==="object"?JSON.stringify(x):x));category.evidence=v502Unique([...(category.evidence||[]),...(Array.isArray(meta.evidence)?meta.evidence:meta.evidence?[meta.evidence]:[])],x=>typeof x==="object"?JSON.stringify(x):v502Canonical(x)).slice(-80);category.updatedAt=Date.now();store.updatedAt=Date.now();return category;
}
function v620CategoryStore(){const store=v620EnsureCategoryStore();for(const name of NOVA502_MODULE_PHASES)v620RegisterCategory(name,{origin:"bootstrap"});return store;}
function v620RelateCategories(subject,predicate,object,meta={}){
  if(window.__TAURI__?.core?.invoke&&typeof nova7RustMindBridge!=="undefined"&&nova7RustMindBridge?.initialized){const store=v620EnsureCategoryStore(),s=v502Canonical(subject),p=v502Canonical(predicate||"related_to"),o=v502Canonical(object);return (store.relations||[]).find(x=>v502Canonical(x.subject)===s&&v502Canonical(x.predicate)===p&&v502Canonical(x.object)===o)||null;}
  const store=v620CategoryStore(),a=v620RegisterCategory(subject,{origin:meta.origin||"learned"}),b=v620RegisterCategory(object,{origin:meta.origin||"learned"});if(!a||!b)return null;
  const rel={subjectId:a.id,subject:a.name,predicate:v502Clean(predicate||"related_to"),objectId:b.id,object:b.name,origin:meta.origin||"learned",evidence:v502Clone(meta.evidence||null)};
  const existing=(store.relations||[]).find(x=>x.subjectId===rel.subjectId&&v502Canonical(x.predicate)===v502Canonical(rel.predicate)&&x.objectId===rel.objectId);
  if(existing){existing.strength=Math.max(0,Math.min(1,Number(existing.strength??0.5)+Number(meta.reinforce||0)));existing.reinforcements=Number(existing.reinforcements||0)+Number(meta.reinforce?1:0);if(meta.reinforce)existing.lastReinforcedAt=Date.now();if(meta.evidence)existing.evidence=v502Clone(meta.evidence);existing.updatedAt=Date.now();store.updatedAt=Date.now();return existing;}
  const stored={id:`catrel_${Math.random().toString(36).slice(2,8)}`,...rel,strength:Math.max(0,Math.min(1,Number(meta.strength??0.5))),reinforcements:Number(meta.reinforce?1:0),lastReinforcedAt:meta.reinforce?Date.now():null,createdAt:Date.now(),updatedAt:Date.now()};store.relations.push(stored);store.relations=store.relations.slice(-2000);store.updatedAt=Date.now();return stored;
}
function v620ReinforceExistingCategoryRelations(names,amount=0.025){
  if(window.__TAURI__?.core?.invoke&&typeof nova7RustMindBridge!=="undefined"&&nova7RustMindBridge?.initialized)return 0;
  const wanted=new Set((names||[]).map(v502Canonical).filter(Boolean)),store=v620CategoryStore();let changed=0;if(!wanted.size)return 0;
  for(const category of store.categories||[]){if(!wanted.has(v502Canonical(category.name)))continue;category.activationCount=Number(category.activationCount||0)+1;category.successfulUseCount=Number(category.successfulUseCount||0)+1;category.strength=Math.min(1,Number(category.strength??0.5)+Math.max(0,Number(amount||0))*0.5);category.updatedAt=Date.now();changed++;}
  if(wanted.size>=2)for(const rel of store.relations||[]){if(!wanted.has(v502Canonical(rel.subject))||!wanted.has(v502Canonical(rel.object)))continue;const before=Number(rel.strength??0.5);rel.strength=Math.min(1,before+Math.max(0,Number(amount||0)));rel.reinforcements=Number(rel.reinforcements||0)+1;rel.lastReinforcedAt=Date.now();rel.updatedAt=Date.now();if(rel.strength!==before)changed++;}
  if(changed)store.updatedAt=Date.now();return changed;
}

/* ---------------------------------------------------------
   MODULE STORE / DESCRIPTORS / EXPERIENCE INDEX
   --------------------------------------------------------- */
function v622ReferenceTokens(...values){
  const out=[],seen=new Set();
  const add=value=>{
    if(value==null)return;
    if(Array.isArray(value)){for(const item of value)add(item);return;}
    if(typeof value==="object"){
      for(const [key,item] of Object.entries(value)){add(key);if(typeof item!=="object")add(item);}
      return;
    }
    const raw=v502Canonical(value);
    if(!raw)return;
    if(raw.length<=64&&!NOVA622_REFERENCE_STOP_WORDS.has(raw)&&!seen.has(raw)){seen.add(raw);out.push(raw);}
    for(const token of raw.split(/[^a-z0-9]+/g)){
      if(token.length<2||NOVA622_REFERENCE_STOP_WORDS.has(token)||seen.has(token))continue;
      seen.add(token);out.push(token);
    }
  };
  for(const value of values)add(value);
  return out;
}
function v622SurfaceCueTerms(module){
  const cues=[];
  for(const condition of module?.conditions||[]){
    const type=v502Canonical(condition?.type||"").replace(/\s+/g,"_");
    if(type==="surface_pattern"){
      const literal=String(condition.pattern||condition.value||"").replace(/\{[^}]+\}/g," ");
      cues.push(...v622ReferenceTokens(literal));continue;
    }
    if(type==="learned_surface_family")cues.push(...v622ReferenceTokens((condition.positiveCues||[]).slice(0,24)));
  }
  return v502Unique(cues,v502Canonical).slice(0,12);
}
function v622ModuleReferenceLabels(module){
  const labels=v502Unique([...(module?.categories||[]),module?.domain,module?.phase,module?.cognitiveForm].filter(Boolean),v502Canonical).slice(0,2);
  if(labels.length<2){for(const token of v622ReferenceTokens(module?.name||"")){if(labels.some(x=>v502Canonical(x)===token))continue;labels.push(token);if(labels.length>=2)break;}}
  return labels.length?labels:["uncategorized"];
}
function v622ModuleReferenceTerms(module){
  const conditionTerms=[];
  for(const condition of module?.conditions||[]){
    conditionTerms.push(condition,condition?.type,condition?.field,condition?.path,condition?.sourcePath,condition?.goalType,condition?.category,condition?.kind,condition?.values);
    if(typeof condition?.value!=="object")conditionTerms.push(condition?.value);
    if(typeof condition?.pattern==="string")conditionTerms.push(String(condition.pattern).replace(/\{[^}]+\}/g," "));
  }
  return v502Unique(v622ReferenceTokens(module?.categories,module?.domain,module?.phase,module?.cognitiveForm,module?.name,module?.requires,module?.produces,module?.capabilities,conditionTerms),v502Canonical).slice(0,18);
}
function v622EnsureModuleNumber(store,module){
  if(Number.isInteger(module?.moduleNumber)&&module.moduleNumber>0){store.nextModuleNumber=Math.max(Number(store.nextModuleNumber||1),module.moduleNumber+1);return module.moduleNumber;}
  let next=Math.max(1,Number(store.nextModuleNumber||1));module.moduleNumber=next;store.nextModuleNumber=next+1;return next;
}
function v622BuildReference(module){return {number:module.moduleNumber,id:module.id,labels:v622ModuleReferenceLabels(module),terms:v622ModuleReferenceTerms(module),cues:v622SurfaceCueTerms(module),priority:Number(module.priority||0),sourceType:module.sourceType||"learned"};}
function v622ReferenceLine(reference,prefix="Relevant"){if(!reference)return "";return `${prefix} - #${reference.number} - ${(reference.labels||[]).join(", ")||"uncategorized"}`;}
function v622RefreshRuntimeLookup(store){
  NOVA622_MODULE_BY_NUMBER.clear();NOVA622_MODULE_BY_ID.clear();NOVA622_MODULE_BY_NAME.clear();NOVA622_MODULES_BY_CATEGORY.clear();NOVA622_REFERENCE_BY_NUMBER.clear();
  for(const module of store?.modules||[]){
    if(!Number.isInteger(module.moduleNumber)||module.moduleNumber<=0)continue;
    NOVA622_MODULE_BY_NUMBER.set(module.moduleNumber,module);NOVA622_MODULE_BY_ID.set(v502Canonical(module.id),module);NOVA622_MODULE_BY_NAME.set(v502Canonical(module.name),module);
    for(const category of v502Unique([...(module.categories||[]),module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical)){
      const key=v502Canonical(category);if(!NOVA622_MODULES_BY_CATEGORY.has(key))NOVA622_MODULES_BY_CATEGORY.set(key,[]);const list=NOVA622_MODULES_BY_CATEGORY.get(key);if(!list.some(x=>x.moduleNumber===module.moduleNumber))list.push(module);
    }
  }
  for(const list of NOVA622_MODULES_BY_CATEGORY.values())list.sort((a,b)=>(b.priority||0)-(a.priority||0));
  for(const ref of store?.referenceIndex?.references||[])NOVA622_REFERENCE_BY_NUMBER.set(ref.number,ref);
  NOVA622_RUNTIME_INDEX_REVISION=Number(store?.referenceIndex?.revision||0);
}
function v622RebuildReferenceIndex(store){
  if(!store)return null;let maxNumber=0;
  for(const module of store.modules||[]){module.categories=v502Unique([...(module.categories||[]),module.category,module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical);if(!module.phase)module.phase=module.categories[0]||"reasoning";module.version=NOVA620_VERSION;if(Number.isInteger(module.moduleNumber)&&module.moduleNumber>0)maxNumber=Math.max(maxNumber,module.moduleNumber);}
  store.nextModuleNumber=Math.max(Number(store.nextModuleNumber||1),maxNumber+1);for(const module of store.modules||[])v622EnsureModuleNumber(store,module);
  const priorRevision=Number(store.referenceIndex?.revision||0);store.referenceIndex={schema:NOVA622_REFERENCE_SCHEMA,kind:"module_descriptors",revision:priorRevision+1,moduleCount:(store.modules||[]).length,references:(store.modules||[]).map(v622BuildReference),updatedAt:Date.now()};
  store.updatedAt=Date.now();v622RefreshRuntimeLookup(store);return store.referenceIndex;
}
function v622EnsureReferenceIndex(store){const idx=store?.referenceIndex;if(!idx||idx.schema!==NOVA622_REFERENCE_SCHEMA||idx.moduleCount!==(store.modules||[]).length)return v622RebuildReferenceIndex(store);if(NOVA622_RUNTIME_INDEX_REVISION!==Number(idx.revision||0))v622RefreshRuntimeLookup(store);return idx;}
function v622EnsureExperienceIndex(store){
  if(!store)return {schema:NOVA622_EXPERIENCE_SCHEMA,routes:[]};
  const prior=store.experienceIndex||{},oldRoutes=Array.isArray(prior.routes)?prior.routes:[];
  if(!store.experienceIndex||prior.schema!==NOVA622_EXPERIENCE_SCHEMA){
    store.experienceIndex={schema:NOVA622_EXPERIENCE_SCHEMA,routes:oldRoutes.map(route=>({
      ...v502Clone(route),
      conceptTerms:v502Unique(route.conceptTerms||[],v502Canonical),
      relationTerms:v502Unique(route.relationTerms||[],v502Canonical),
      fallbackTerms:v502Unique(route.fallbackTerms||route.topicTerms||[],v502Canonical),
      goalTerms:v502Unique(route.goalTerms||[],v502Canonical),
      moduleStrengths:{...(route.moduleStrengths||{})},
      confidence:Math.max(NOVA622_ROUTE_MIN_CONFIDENCE,Math.min(1,Number(route.confidence??0.55))),
      failures:Number(route.failures||0),broadenings:Number(route.broadenings||0)
    })),revision:Number(prior.revision||0),createdAt:prior.createdAt||Date.now(),updatedAt:Date.now(),note:"Built only from successful current-mind experience; semantic signatures and contribution strengths develop from actual traces."};
  }
  const experience=store.experienceIndex,valid=new Set((store.modules||[]).map(module=>module.moduleNumber));
  for(const route of experience.routes||[]){
    route.moduleNumbers=v502Unique((route.moduleNumbers||[]).filter(number=>valid.has(number)),Number).slice(0,NOVA622_MAX_ROUTE_MODULES);
    route.moduleStrengths ||= {};for(const key of Object.keys(route.moduleStrengths)){if(!valid.has(Number(key)))delete route.moduleStrengths[key];else route.moduleStrengths[key]=Math.max(0.05,Math.min(1,Number(route.moduleStrengths[key]||0.5)));}
    for(const number of route.moduleNumbers)if(route.moduleStrengths[number]==null)route.moduleStrengths[number]=0.5;
    route.confidence=Math.max(NOVA622_ROUTE_MIN_CONFIDENCE,Math.min(1,Number(route.confidence??0.55)));
  }
  experience.routes=(experience.routes||[]).filter(route=>(route.moduleNumbers||[]).length).slice(-NOVA622_MAX_EXPERIENCE_ROUTES);return experience;
}
function v622IndexModule(store,module){
  let idx=store?.referenceIndex;if(!idx||idx.schema!==NOVA622_REFERENCE_SCHEMA)idx=v622RebuildReferenceIndex(store);else if(NOVA622_RUNTIME_INDEX_REVISION!==Number(idx.revision||0))v622RefreshRuntimeLookup(store);v622EnsureModuleNumber(store,module);const ref=v622BuildReference(module),existing=NOVA622_REFERENCE_BY_NUMBER.get(module.moduleNumber);
  if(existing)Object.assign(existing,ref);else{idx.references.push(ref);NOVA622_REFERENCE_BY_NUMBER.set(ref.number,ref);}idx.moduleCount=(store.modules||[]).length;idx.revision=Number(idx.revision||0)+1;idx.updatedAt=Date.now();NOVA622_RUNTIME_INDEX_REVISION=idx.revision;
  NOVA622_MODULE_BY_NUMBER.set(module.moduleNumber,module);NOVA622_MODULE_BY_ID.set(v502Canonical(module.id),module);NOVA622_MODULE_BY_NAME.set(v502Canonical(module.name),module);
  for(const category of v502Unique([...(module.categories||[]),module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical)){const key=v502Canonical(category);if(!NOVA622_MODULES_BY_CATEGORY.has(key))NOVA622_MODULES_BY_CATEGORY.set(key,[]);const list=NOVA622_MODULES_BY_CATEGORY.get(key);if(!list.some(x=>x.moduleNumber===module.moduleNumber))list.push(module);list.sort((a,b)=>(b.priority||0)-(a.priority||0));}
  return ref;
}
function v622ResolveModuleReference(wanted){const store=v502ModuleStore();v622EnsureReferenceIndex(store);const raw=v502Clean(wanted);if(!raw)return null;const numberMatch=raw.match(/^#?(\d+)$/);if(numberMatch)return NOVA622_MODULE_BY_NUMBER.get(Number(numberMatch[1]))||null;const key=v502Canonical(raw);return NOVA622_MODULE_BY_ID.get(key)||NOVA622_MODULE_BY_NAME.get(key)||(NOVA622_MODULES_BY_CATEGORY.get(key)||[])[0]||null;}

function v622KnownSurfaceConceptTerms(ctx={}){
  const tokens=(ctx?.utteranceObservation?.tokens||[]).filter(token=>token?.word).map(token=>v502Clean(token.surface)).filter(Boolean),found=[],seen=new Set();if(!tokens.length)return found;
  for(let width=Math.min(3,tokens.length);width>=1;width--){for(let i=0;i<=tokens.length-width;i++){const phrase=tokens.slice(i,i+width).join(" "),key=v502Canonical(phrase);if(!key||seen.has(key)||NOVA622_TOPIC_STOP_WORDS.has(key))continue;let hit=null;try{if(typeof v450ResolveConcept==="function")hit=v450ResolveConcept(phrase,{minScore:70});}catch(_){ }if(hit?.concept){const conceptKey=v502Canonical(hit.concept.key||hit.concept.name);if(conceptKey&&!seen.has(conceptKey)){seen.add(conceptKey);found.push(conceptKey);if(found.length>=24)return found;}}}}
  return found;
}
function v622ExperienceSignature(ctx={}){
  const concepts=[],goals=[],relations=[],fallback=[];const conceptSeen=new Set(),goalSeen=new Set(),relationSeen=new Set(),fallbackSeen=new Set();
  const add=(bucket,seen,value,stop=null,limit=24)=>{for(const token of v622ReferenceTokens(value)){const key=v502Canonical(token);if(!key||seen.has(key)||(stop&&stop.has(key)))continue;seen.add(key);bucket.push(key);if(bucket.length>=limit)return;}};
  const mind=ctx?.canonicalSemanticState||ctx?.cognitiveState||{};
  add(concepts,conceptSeen,mind.activeCategories,NOVA622_TOPIC_STOP_WORDS,24);
  for(const entity of mind.entities||[]){let resolved=null;try{resolved=v622ResolveKnowledgeConceptValue(entity?.label||entity?.name||"",70);}catch(_){ }add(concepts,conceptSeen,resolved?.key||resolved?.name||entity?.type||entity?.kind,NOVA622_TOPIC_STOP_WORDS,24);}
  for(const event of mind.events||[])add(relations,relationSeen,[event?.predicateKey,event?.type],NOVA622_TOPIC_STOP_WORDS,18);
  for(const relation of mind.relations||[])add(relations,relationSeen,[relation?.type,relation?.predicate],NOVA622_TOPIC_STOP_WORDS,18);
  for(const slot of mind.unknownSlots||[])add(relations,relationSeen,[slot?.role,slot?.kind,slot?.type],NOVA622_TOPIC_STOP_WORDS,18);
  for(const request of mind.requests||[]){add(goals,goalSeen,[request?.type,request?.action,request?.actionFamily,request?.requestedRole,request?.answerRelation],null,16);add(concepts,conceptSeen,[request?.constructionFamily,request?.topic],NOVA622_TOPIC_STOP_WORDS,24);}
  for(const conclusion of mind.conclusions||[])add(goals,goalSeen,conclusion?.type,null,16);for(const unresolved of mind.unresolved||[])add(goals,goalSeen,[unresolved?.type,unresolved?.goalType],null,16);
  for(const concept of v622KnownSurfaceConceptTerms(ctx))add(concepts,conceptSeen,concept,NOVA622_TOPIC_STOP_WORDS,24);
  if(!concepts.length){for(const token of ctx?.utteranceObservation?.tokens||[]){if(token?.word)add(fallback,fallbackSeen,token.surface,NOVA622_TOPIC_STOP_WORDS,24);}}
  return {conceptTerms:concepts.slice(0,24),goalTerms:goals.slice(0,16),relationTerms:relations.slice(0,18),fallbackTerms:fallback.slice(0,24),topics:(concepts.length?concepts:fallback).slice(0,24),goals:goals.slice(0,16)};
}
function v622ExperienceRouteScore(route,signature){
  const overlap=(a,b)=>{const A=new Set((a||[]).map(v502Canonical));let n=0;for(const value of b||[])if(A.has(v502Canonical(value)))n++;return n;};
  const conceptOverlap=overlap(route?.conceptTerms,signature.conceptTerms),goalOverlap=overlap(route?.goalTerms,signature.goalTerms),relationOverlap=overlap(route?.relationTerms,signature.relationTerms),fallbackOverlap=overlap(route?.fallbackTerms||route?.topicTerms,signature.fallbackTerms);
  if(!conceptOverlap&&!goalOverlap&&!relationOverlap&&!fallbackOverlap)return 0;let score=conceptOverlap*14+goalOverlap*10+relationOverlap*6+fallbackOverlap*2;score+=Math.min(5,Number(route?.successes||0)*0.25)*Math.max(.25,Number(route?.confidence??.55));return score;
}
function v622FindExperienceRoutes(ctx={},limit=3){
  const store=v502ModuleStore(),experience=v622EnsureExperienceIndex(store),signature=v622ExperienceSignature(ctx),ranked=[];
  for(const route of experience.routes||[]){if(Number(route.confidence??.55)<NOVA622_ROUTE_MIN_CONFIDENCE)continue;const score=v622ExperienceRouteScore(route,signature);if(score<=0)continue;ranked.push({route,score});}
  ranked.sort((a,b)=>b.score-a.score||Number(b.route.confidence||0)-Number(a.route.confidence||0)||Number(b.route.successes||0)-Number(a.route.successes||0)||Number(b.route.updatedAt||0)-Number(a.route.updatedAt||0));return {signature,routes:ranked.slice(0,limit)};
}
function v622RefsForNumbers(numbers,prefix="Relevant",strengths={}){
  const out=[];for(const number of v502Unique(numbers||[],Number)){const module=NOVA622_MODULE_BY_NUMBER.get(number),ref=NOVA622_REFERENCE_BY_NUMBER.get(number);if(!module||module.enabled===false||!ref)continue;out.push({...v502Clone(ref),experienceWeight:Number(strengths?.[number]??strengths?.[String(number)]??0),line:v622ReferenceLine(ref,prefix)});}return out;
}
function v622ModuleIsMentionSpecific(module){if(!module)return false;if(v502Canonical(module.domain)==="reference"||v502Canonical(module.phase)==="reference")return true;try{return /\bmention(?:\.|\")/.test(JSON.stringify(module.conditions||[]));}catch(_){return false;}}
function v622SearchState(ctx){const ws=ctx?.workspace;if(!ws)return {mode:"stateless",workingSet:[]};ws.cognitiveSearch ||= {mode:null,routeIds:[],attemptedRouteIds:[],globalExplored:false,mentionExplored:{},workingSet:[],forceBroad:false,routeFailureRecorded:false,createdAt:Date.now()};return ws.cognitiveSearch;}
function v622StructuralSelectorReferences(selectorKey){
  const refs=[];for(const ref of NOVA622_REFERENCE_BY_NUMBER.values()){const module=NOVA622_MODULE_BY_NUMBER.get(ref.number);if(!module||module.enabled===false)continue;const exact=[...(ref.labels||[]),...(ref.terms||[])].some(x=>v502Canonical(x)===selectorKey);if(exact)refs.push({...v502Clone(ref),score:100,line:v622ReferenceLine(ref)});}return refs.sort((a,b)=>Number(b.priority||0)-Number(a.priority||0)||a.number-b.number);
}
function v622ColdExplorationReferences(ctx={}){
  const store=v502ModuleStore();v622EnsureReferenceIndex(store);const search=v622SearchState(ctx),isMention=Boolean(ctx?.mention),foundation=(store.modules||[]).filter(module=>module.enabled!==false&&module.sourceType==="built_in").map(module=>module.moduleNumber);
  if(isMention){
    const key=String(ctx.mention?.id||ctx.mention?.index||ctx.mention?.surface||"mention");
    if(!search.mentionExplored[key]){search.mentionExplored[key]=true;search.mode="explore_mention";const numbers=(store.modules||[]).filter(module=>module.enabled!==false&&v622ModuleIsMentionSpecific(module)).map(module=>module.moduleNumber);return v622RefsForNumbers(v502Unique([...foundation,...numbers],Number),"Exploring");}
    const working=(search.workingSet||[]).filter(number=>v622ModuleIsMentionSpecific(NOVA622_MODULE_BY_NUMBER.get(number)));search.mode="working_set";return v622RefsForNumbers(v502Unique([...foundation,...working],Number),"Working");
  }
  if(search.forceBroad||!search.globalExplored){search.forceBroad=false;search.globalExplored=true;search.mode="explore_all";return v622RefsForNumbers((store.modules||[]).filter(module=>module.enabled!==false).map(module=>module.moduleNumber),"Exploring");}
  search.mode="working_set";return v622RefsForNumbers(v502Unique([...foundation,...(search.workingSet||[])],Number),"Working");
}
function v622RelevantModuleReferences(ctx={},selector=null,options={}){
  const store=v502ModuleStore();v622EnsureReferenceIndex(store);v622EnsureExperienceIndex(store);const selectorKey=v502Canonical(selector||"");
  if(selectorKey)return v622StructuralSelectorReferences(selectorKey).slice(0,Math.max(1,Number(options.limit||80)));
  const search=v622SearchState(ctx);
  if(!search.forceBroad){const experience=v622FindExperienceRoutes(ctx,3);if(experience.routes.length){const numbers=[],routeIds=[],strengths={};for(const item of experience.routes){routeIds.push(item.route.id);numbers.push(...(item.route.moduleNumbers||[]));for(const [number,value] of Object.entries(item.route.moduleStrengths||{}))strengths[number]=Math.max(Number(strengths[number]||0),Number(value||0));item.route.uses=Number(item.route.uses||0)+1;item.route.lastUsedAt=Date.now();}for(const module of store.modules||[])if(module.enabled!==false&&module.sourceType==="built_in")numbers.push(module.moduleNumber);search.mode="experience_route";search.routeIds=routeIds;search.attemptedRouteIds=v502Unique([...(search.attemptedRouteIds||[]),...routeIds],String);search.routeStrengths=strengths;return v622RefsForNumbers(v502Unique(numbers,Number),"Experienced",strengths);}}
  search.routeIds=[];search.routeStrengths={};return v622ColdExplorationReferences(ctx);
}
function v622RecordTurnWorkingSet(ctx,applied=[],changed=[]){
  const search=v622SearchState(ctx);if(!ctx?.workspace)return search;const numbers=new Set(search.workingSet||[]);for(const row of changed||[])if(Number.isInteger(row?.number))numbers.add(row.number);
  const produced=new Set();for(const row of changed||[]){const module=NOVA622_MODULE_BY_NUMBER.get(row.number);for(const term of v622ReferenceTokens(module?.produces||[]))produced.add(term);}
  if(produced.size){for(const module of NOVA622_MODULE_BY_NUMBER.values()){if(module.enabled===false)continue;const needs=v622ReferenceTokens(module.requires||[]);if(needs.some(term=>produced.has(term)))numbers.add(module.moduleNumber);}}
  for(const module of NOVA622_MODULE_BY_NUMBER.values())if(module.sourceType==="built_in")numbers.add(module.moduleNumber);
  search.workingSet=[...numbers].slice(-NOVA622_MAX_WORKING_SET);search.lastApplied=(applied||[]).filter(row=>row?.progress).map(row=>row.number);search.lastChanged=(changed||[]).map(row=>row.number);search.updatedAt=Date.now();return search;
}
function v622QuickHash(value){let h=2166136261;for(const ch of String(value||"")){h^=ch.charCodeAt(0);h=Math.imul(h,16777619);}return (h>>>0).toString(36);}
function v622CognitiveFingerprint(ctx={}){const mind=ctx?.canonicalSemanticState||ctx?.cognitiveState||{},buckets=["frames","entities","events","relations","propositions","requests","unknownSlots","candidateSets","constraints","evidence","conclusions","structures","activeCategories","unresolved"];const parts=[];for(const name of buckets){const list=Array.isArray(mind[name])?mind[name]:[];parts.push(`${name}:${list.length}:${v622QuickHash(JSON.stringify(list.slice(-3)))}`);}parts.push(`surface:${v622QuickHash(ctx?.moduleState?.surfaceResponse||"")}`);return parts.join("|");}
function v622ContributionDelta(before,after,progress=false){if(before===after)return progress?1:0;const A=String(before||"").split("|"),B=String(after||"").split("|");let changed=0;for(let i=0;i<Math.max(A.length,B.length);i++)if(A[i]!==B[i])changed++;return Math.max(1,changed);}
function v622ReflectSuccessfulRoute(ws,report={}){
  if(window.__TAURI__?.core?.invoke)return null;
  if(!ws||ws.executionMode==="INTERNAL_GOAL"||!report?.surface)return null;const store=v502ModuleStore(),experience=v622EnsureExperienceIndex(store),ctx={workspace:ws,utteranceObservation:ws.utteranceObservation,canonicalSemanticState:ws.canonicalSemanticState,cognitiveState:ws.canonicalSemanticState},signature=v622ExperienceSignature(ctx);
  const contributors=(report.changed||[]).filter(row=>Number.isInteger(row?.number)&&row?.progress!==false),moduleNumbers=v502Unique(contributors.map(row=>row.number),Number).slice(0,NOVA622_MAX_ROUTE_MODULES);if(!moduleNumbers.length)return null;
  let best=null,bestScore=0;const attemptedIds=new Set(ws.cognitiveSearch?.attemptedRouteIds||ws.cognitiveSearch?.routeIds||[]);if(attemptedIds.size){for(const route of experience.routes||[]){if(!attemptedIds.has(route.id))continue;const score=v622ExperienceRouteScore(route,signature);if(score>bestScore){best=route;bestScore=score;}}}
  if(!best){for(const route of experience.routes||[]){const score=v622ExperienceRouteScore(route,signature);if(score>bestScore){best=route;bestScore=score;}}}
  if(!best||(!attemptedIds.has(best.id)&&bestScore<10)){best={id:`route_${Date.now()}_${Math.random().toString(36).slice(2,7)}`,conceptTerms:signature.conceptTerms.slice(0,18),relationTerms:signature.relationTerms.slice(0,12),fallbackTerms:signature.fallbackTerms.slice(0,12),topicTerms:signature.topics.slice(0,18),goalTerms:signature.goalTerms.slice(0,10),moduleNumbers:[],moduleStrengths:{},confidence:.55,successes:0,failures:0,uses:0,broadenings:0,origin:"successful_current_mind_reflection",createdAt:Date.now(),updatedAt:Date.now()};experience.routes.push(best);}
  best.conceptTerms=v502Unique([...(best.conceptTerms||[]),...signature.conceptTerms],v502Canonical).slice(0,24);best.relationTerms=v502Unique([...(best.relationTerms||[]),...signature.relationTerms],v502Canonical).slice(0,18);best.fallbackTerms=v502Unique([...(best.fallbackTerms||[]),...signature.fallbackTerms],v502Canonical).slice(0,18);best.topicTerms=v502Unique([...(best.topicTerms||[]),...signature.topics],v502Canonical).slice(0,24);best.goalTerms=v502Unique([...(best.goalTerms||[]),...signature.goalTerms],v502Canonical).slice(0,14);best.moduleNumbers=v502Unique([...(best.moduleNumbers||[]),...moduleNumbers],Number).slice(0,NOVA622_MAX_ROUTE_MODULES);best.moduleStrengths ||= {};
  for(const row of contributors){const contribution=Math.min(1,.35+Math.min(8,Number(row.deltaScore||1))*.07),prior=Number(best.moduleStrengths[row.number]??.45);best.moduleStrengths[row.number]=Number(Math.min(1,prior*.72+contribution*.28).toFixed(3));}
  best.successes=Number(best.successes||0)+1;best.confidence=Number(Math.min(1,Number(best.confidence??.55)+.06).toFixed(3));best.lastSuccessfulPrompt=v502Clean(ws.rawInput).slice(0,180);best.lastSuccessfulModuleNumbers=moduleNumbers;best.lastContribution=v502Clone(contributors.map(row=>({number:row.number,deltaScore:Number(row.deltaScore||1)})));best.updatedAt=Date.now();experience.routes=experience.routes.slice(-NOVA622_MAX_EXPERIENCE_ROUTES);experience.revision=Number(experience.revision||0)+1;experience.updatedAt=Date.now();store.updatedAt=Date.now();
  try{v620ReinforceExistingCategoryRelations(signature.conceptTerms,.02);}catch(_){ }return v502Clone(best);
}
function v622RecordRouteFailure(ws,reason="route_stalled"){
  if(window.__TAURI__?.core?.invoke)return null;
  const search=ws?.cognitiveSearch;if(!search?.routeIds?.length||search.routeFailureRecorded)return null;const store=v502ModuleStore(),experience=v622EnsureExperienceIndex(store),rows=[];
  for(const id of search.routeIds){const route=(experience.routes||[]).find(row=>row.id===id);if(!route)continue;route.failures=Number(route.failures||0)+1;route.broadenings=Number(route.broadenings||0)+Number(reason==="broadened_after_stall");route.confidence=Number(Math.max(NOVA622_ROUTE_MIN_CONFIDENCE,Number(route.confidence??.55)-NOVA622_ROUTE_FAILURE_DECAY).toFixed(3));for(const key of Object.keys(route.moduleStrengths||{}))route.moduleStrengths[key]=Number(Math.max(.05,Number(route.moduleStrengths[key]||.5)*.97).toFixed(3));route.lastFailureReason=reason;route.lastFailedAt=Date.now();route.updatedAt=Date.now();rows.push(v502Clone(route));}
  if(rows.length){search.routeFailureRecorded=true;experience.revision=Number(experience.revision||0)+1;experience.updatedAt=Date.now();store.updatedAt=Date.now();}return rows;
}
function v622CognitiveMapSummary(){
  const store=v502ModuleStore(),descriptors=v622EnsureReferenceIndex(store),experience=v622EnsureExperienceIndex(store);return {schema:NOVA622_REFERENCE_SCHEMA,total:descriptors.references?.length||0,nextModuleNumber:store.nextModuleNumber||1,lastRun:v502Clone(store.lastRun||null),references:v502Clone((descriptors.references||[]).map(ref=>({number:ref.number,labels:ref.labels,line:v622ReferenceLine(ref)}))),experience:{schema:experience.schema,totalRoutes:experience.routes?.length||0,routes:v502Clone((experience.routes||[]).slice().sort((a,b)=>Number(b.updatedAt||0)-Number(a.updatedAt||0)).slice(0,80))}};
}

/* ---------------------------------------------------------
   V7 PASS 2 — RUST-MIGRATED SEMANTIC FOUNDATION

   The old JavaScript built-ins for generic relational semantic construction
   and relational request resolution were removed when those responsibilities
   moved to Rust. Saved copies are deleted from the current module store rather
   than retained as compatibility cognition.
   --------------------------------------------------------- */
function v623EnsureFoundationModules(store){
  if(!store||!Array.isArray(store.modules))return false;
  const rustMigrated=new Set(["core_generic_semantic_construction","core_generic_relational_reasoning","core_generic_conversation_realization"]);
  const before=store.modules.length;
  store.modules=store.modules.filter(module=>!(module?.sourceType==="built_in"&&rustMigrated.has(module?.id)));
  const changed=store.modules.length!==before;if(changed)store.referenceIndex=null;return changed;
}

function v502ModuleStore(){
  const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return {version:NOVA502_MODULE_SCHEMA_VERSION,modules:[],nextModuleNumber:1,referenceIndex:{schema:NOVA622_REFERENCE_SCHEMA,revision:0,moduleCount:0,references:[]},experienceIndex:{schema:NOVA622_EXPERIENCE_SCHEMA,routes:[]}};const prior=memory.cognitiveModules||{};
  if(prior.version!==NOVA502_MODULE_SCHEMA_VERSION){
    memory.cognitiveModules={version:NOVA502_MODULE_SCHEMA_VERSION,appVersion:NOVA620_VERSION,modules:Array.isArray(prior.modules)?prior.modules:[],nextModuleNumber:Number(prior.nextModuleNumber||1),referenceIndex:null,experienceIndex:null,lastCompilation:prior.lastCompilation||null,lastRun:prior.lastRun||null,createdAt:prior.createdAt||Date.now(),updatedAt:Date.now()};
    const migrated=memory.cognitiveModules;for(const module of migrated.modules||[]){module.categories=v502Unique([...(module.categories||[]),module.category,module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical);if(!module.phase)module.phase=module.categories[0]||"reasoning";module.version=NOVA620_VERSION;for(const category of module.categories)v620RegisterCategory(category,{origin:module.sourceType==="built_in"?"bootstrap":"learned",evidence:(module.evidence||[]).slice(0,4)});}v622RebuildReferenceIndex(migrated);
  }
  const store=memory.cognitiveModules;store.appVersion=NOVA620_VERSION;v623EnsureFoundationModules(store);v622EnsureReferenceIndex(store);v622EnsureExperienceIndex(store);return store;
}
function v502RegisterOperation(name,handler){const op=v502Canonical(name).replace(/\s+/g,"_");if(!op||typeof handler!=="function")return false;NOVA502_MODULE_OPS.add(op);NOVA620_OPERATION_HANDLERS.set(op,handler);return true;}
function v502RegisterCondition(name,handler){const key=v502Canonical(name).replace(/\s+/g,"_");if(!key||typeof handler!=="function")return false;NOVA620_CONDITION_HANDLERS.set(key,handler);return true;}
function v502ReadValue(spec,ctx){if(spec&&typeof spec==="object"&&!Array.isArray(spec)){if(Object.prototype.hasOwnProperty.call(spec,"fromPath"))return v502Clone(v502GetPath(ctx,spec.fromPath));if(Object.prototype.hasOwnProperty.call(spec,"path"))return v502Clone(v502GetPath(ctx,spec.path));if(Object.prototype.hasOwnProperty.call(spec,"fromCapture"))return v502Clone(ctx?.captures?.[spec.fromCapture]??ctx?.patternCaptures?.[spec.fromCapture]);if(Object.prototype.hasOwnProperty.call(spec,"value"))return v502Clone(spec.value);}return v502Clone(spec);}
function v620ResolveTemplate(value,ctx){
  if(Array.isArray(value))return value.map(x=>v620ResolveTemplate(x,ctx));
  if(value&&typeof value==="object"){if(Object.keys(value).length===1&&Object.prototype.hasOwnProperty.call(value,"fromPath"))return v502Clone(v502GetPath(ctx,value.fromPath));if(Object.keys(value).length===1&&Object.prototype.hasOwnProperty.call(value,"fromCapture"))return v502Clone(ctx?.captures?.[value.fromCapture]??ctx?.patternCaptures?.[value.fromCapture]);const out={};for(const [k,v] of Object.entries(value))out[k]=v620ResolveTemplate(v,ctx);return out;}
  if(typeof value==="string")return value.replace(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g,(_,path)=>{const v=v502GetPath(ctx,path);return v==null?"":String(v);}).replace(/\{\%\s*([A-Za-z0-9_.-]+)\s*\%\}/g,(_,name)=>{const v=ctx?.captures?.[name]??ctx?.patternCaptures?.[name];return v==null?"":String(v);});
  return value;
}
function v620PatternToRegex(pattern){const raw=String(pattern||"");let src="",last=0;const rx=/\{([A-Za-z][A-Za-z0-9_-]*)\}/g;let m;const esc=s=>s.replace(/[.*+?^${}()|[\]\\]/g,"\\$&").replace(/\s+/g,"\\s+");while((m=rx.exec(raw))){src+=esc(raw.slice(last,m.index));src+=`(?<${m[1]}>.+?)`;last=m.index+m[0].length;}src+=esc(raw.slice(last));return new RegExp(`^\\s*${src}\\s*$`,`i`);}
function v620MatchSurfacePattern(pattern,text){try{const m=String(text??"").match(v620PatternToRegex(pattern));return m?{...(m.groups||{}),__matched:m[0]}:null;}catch(_){return null;}}

function v502ConditionMatches(condition,ctx){const c=condition||{type:"always"},type=v502Canonical(c.type||"always").replace(/\s+/g,"_"),handler=NOVA620_CONDITION_HANDLERS.get(type);return handler?Boolean(handler(c,ctx)):false;}
function v502ModuleMatches(module,ctx){return (module.conditions||[{type:"always"}]).every(c=>v502ConditionMatches(c,ctx));}
function v502ValidateModule(module){
  const errors=[];if(!module||typeof module!=="object")return {valid:false,errors:["module_not_object"]};if(!module.name)errors.push("missing_name");const categories=v502Unique([...(module.categories||[]),module.category,module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical);if(!categories.length)errors.push("missing_category");if(!Array.isArray(module.program)||!module.program.length)errors.push("missing_program");
  const walk=(program,path="program")=>{for(let i=0;i<(program||[]).length;i++){const step=program[i]||{},op=v502Canonical(step.op).replace(/\s+/g,"_");if(!NOVA502_MODULE_OPS.has(op))errors.push(`unsupported_op:${step.op||"missing"}`);for(const p of [step?.args?.path,step?.args?.fromPath,step?.args?.toPath,step?.args?.listPath,step?.args?.targetPath,step?.args?.sourcePath,step?.args?.outputPath])if(p&&/(?:^|\.)(?:__proto__|prototype|constructor)(?:\.|$)/.test(String(p)))errors.push(`unsafe_path:${path}.${i}`);if(op==="branch"){walk(step?.args?.then||step.then||[],`${path}.${i}.then`);walk(step?.args?.else||step.else||[],`${path}.${i}.else`);}if(op==="for_each")walk(step?.args?.program||step.program||[],`${path}.${i}.program`);}};walk(module.program||[]);
  for(const c of module.conditions||[])if(!NOVA620_CONDITION_HANDLERS.has(v502Canonical(c?.type||"always").replace(/\s+/g,"_")))errors.push(`unsupported_condition:${c?.type||"missing"}`);return {valid:!errors.length,errors:v502Unique(errors,String)};
}
function v502ModuleSignature(module){return v502Canonical(JSON.stringify({name:module.name,categories:v502Unique([...(module.categories||[]),module.category,module.domain,module.phase,module.cognitiveForm].filter(Boolean),v502Canonical).sort(),conditions:module.conditions||[],program:module.program||[]}));}
function v502StoreModule(module,meta={}){
  const categories=v502Unique([...(module?.categories||[]),module?.category,module?.domain,module?.phase,module?.cognitiveForm].filter(Boolean),v502Canonical),preparedInput={...module,categories,phase:module?.phase||categories[0]||"reasoning"},validation=v502ValidateModule(preparedInput);if(!validation.valid)return {stored:false,validation,module:null};
  const store=v502ModuleStore(),prepared={id:module.id||`cmod_${v502SafeId(module.name)}_${Math.random().toString(36).slice(2,7)}`,version:NOVA620_VERSION,name:v502Clean(module.name),domain:module.domain||categories[0]||"learned",phase:preparedInput.phase,categories,conditions:v502Clone(module.conditions||[{type:"always"}]),program:v502Clone(module.program||[]),requires:v502Clone(module.requires||[]),produces:v502Clone(module.produces||[]),capabilities:v502Clone(module.capabilities||[]),priority:Number.isFinite(module.priority)?module.priority:50,enabled:module.enabled!==false,sourceType:meta.sourceType||module.sourceType||"learned",sourceName:meta.sourceName||module.sourceName||"conversation",sourceId:meta.sourceId||module.sourceId||null,evidence:v502Unique([...(module.evidence||[]),v502Clean(meta.text||module.directive||"")].filter(Boolean),x=>typeof x==="object"?JSON.stringify(x):v502Canonical(x)),cognitiveForm:module.cognitiveForm||null,policySpec:v502Clone(module.policySpec||null),synthesisIntent:v502Clone(module.synthesisIntent||null),learnedAt:module.learnedAt||Date.now(),updatedAt:Date.now()};
  prepared.signature=v502ModuleSignature(prepared);const existing=store.modules.find(x=>x.signature===prepared.signature&&x.sourceType===prepared.sourceType);
  if(existing){existing.enabled=true;existing.updatedAt=Date.now();existing.categories=v502Unique([...(existing.categories||[]),...categories],v502Canonical);existing.evidence=v502Unique([...(existing.evidence||[]),...prepared.evidence],x=>typeof x==="object"?JSON.stringify(x):v502Canonical(x)).slice(-80);existing.capabilities=v502Unique([...(existing.capabilities||[]),...(prepared.capabilities||[])],v502Canonical);existing.produces=v502Unique([...(existing.produces||[]),...(prepared.produces||[])],v502Canonical);for(const c of existing.categories)v620RegisterCategory(c,{origin:existing.sourceType==="built_in"?"bootstrap":"learned",evidence:prepared.evidence});v622IndexModule(store,existing);return {stored:true,existing:true,validation,module:existing};}
  v622EnsureModuleNumber(store,prepared);store.modules.push(prepared);let trimmed=false;if(store.modules.length>1500){store.modules=store.modules.slice(-1500);trimmed=true;}store.updatedAt=Date.now();for(const c of categories)v620RegisterCategory(c,{origin:prepared.sourceType==="built_in"?"bootstrap":"learned",evidence:prepared.evidence});if(trimmed)v622RebuildReferenceIndex(store);else v622IndexModule(store,prepared);return {stored:true,existing:false,validation,module:prepared};
}

/* ---------------------------------------------------------
   CONDITION VOCABULARY — structural, not curriculum-specific
   --------------------------------------------------------- */
v502RegisterCondition("always",()=>true);
v502RegisterCondition("field_exists",(c,ctx)=>{const v=v502GetPath(ctx,c.field||c.path);return v!==undefined&&v!==null;});
v502RegisterCondition("field_missing",(c,ctx)=>{const v=v502GetPath(ctx,c.field||c.path);return v===undefined||v===null;});
v502RegisterCondition("field_equals",(c,ctx)=>v502Canonical(v502GetPath(ctx,c.field||c.path))===v502Canonical(c.value));
v502RegisterCondition("field_not_equals",(c,ctx)=>v502Canonical(v502GetPath(ctx,c.field||c.path))!==v502Canonical(c.value));
v502RegisterCondition("field_in",(c,ctx)=>(c.values||[]).some(v=>v502Canonical(v)===v502Canonical(v502GetPath(ctx,c.field||c.path))));
v502RegisterCondition("field_not_in",(c,ctx)=>!(c.values||[]).some(v=>v502Canonical(v)===v502Canonical(v502GetPath(ctx,c.field||c.path))));
v502RegisterCondition("contains_any",(c,ctx)=>{const raw=v502Canonical(c.sourcePath?v502GetPath(ctx,c.sourcePath):(ctx?.rawText||ctx?.input||""));return(c.values||[]).some(v=>raw.includes(v502Canonical(v)));});
v502RegisterCondition("contains_all",(c,ctx)=>{const raw=v502Canonical(c.sourcePath?v502GetPath(ctx,c.sourcePath):(ctx?.rawText||ctx?.input||""));return(c.values||[]).every(v=>raw.includes(v502Canonical(v)));});
v502RegisterCondition("surface_in",(c,ctx)=>(c.values||[]).some(v=>v502Canonical(v)===v502Canonical(ctx?.mention?.surface??ctx?.surface??ctx?.rawText)));
v502RegisterCondition("surface_pattern",(c,ctx)=>{const text=c.sourcePath?v502GetPath(ctx,c.sourcePath):(ctx?.surface??ctx?.rawText??"");const captures=v620MatchSurfacePattern(c.pattern,text);if(!captures)return false;ctx.patternCaptures={...(ctx.patternCaptures||{}),...captures};ctx.captures={...(ctx.captures||{}),...captures};return true;});

/* A learned surface family is a compact, data-driven recognizer synthesized from
   positive and negative teaching demonstrations. The condition contains learned
   exemplars as data; it does not assign any built-in meaning to words such as
   repeat, explain, remember, joke, or any future learned action. */
function v6210FamilyCue(text){return v502Canonical(String(text??"")).replace(/[^a-z0-9]+/g," ").replace(/\s+/g," ").trim();}
function v6210FamilyTokens(text){return new Set(v6210FamilyCue(text).split(" ").filter(token=>token.length>1&&!NOVA622_REFERENCE_STOP_WORDS.has(token)));}
function v6210FamilySimilarity(a,b){const ca=v6210FamilyCue(a),cb=v6210FamilyCue(b);if(!ca||!cb)return 0;if(ca===cb)return 1;const A=v6210FamilyTokens(ca),B=v6210FamilyTokens(cb);if(!A.size||!B.size)return 0;let common=0;for(const token of A)if(B.has(token))common++;const union=new Set([...A,...B]).size;let score=common/Math.max(1,union);const aa=[...A],bb=[...B];if(aa[0]&&aa[0]===bb[0])score+=0.12;if(ca.includes(cb)||cb.includes(ca))score+=0.18;return Math.min(1,score);}
function v6210ObservedOuterCue(ctx){const obs=ctx?.utteranceObservation||ctx?.canonicalSemanticState?.surface||ctx?.cognitiveState?.surface||null;if(obs?.segments?.length)return (obs.segments||[]).filter(segment=>segment?.kind!=="quoted_text").map(segment=>segment?.text||"").join(" ");return ctx?.rawText??ctx?.input??ctx?.surface??"";}
function v6210LearnedFamilyMatch(c,ctx){
  const current=v6210ObservedOuterCue(ctx),positives=(c.positiveCues||[]).filter(Boolean),negatives=(c.negativeCues||[]).filter(Boolean);if(!positives.length)return null;
  let positiveScore=0,matchedCue=null;for(const cue of positives){const score=v6210FamilySimilarity(current,cue);if(score>positiveScore){positiveScore=score;matchedCue=cue;}}
  let negativeScore=0;for(const cue of negatives)negativeScore=Math.max(negativeScore,v6210FamilySimilarity(current,cue));
  const threshold=Number(c.threshold??0.58),margin=Number(c.negativeMargin??0.08);if(positiveScore<threshold||negativeScore>=positiveScore-margin)return null;
  const obs=ctx?.utteranceObservation||ctx?.canonicalSemanticState?.surface||ctx?.cognitiveState?.surface||{};const quotes=Array.isArray(obs?.quotes)?obs.quotes:[];const payloadMode=v502Canonical(c.payloadMode||"").replace(/\s+/g,"_");
  if(payloadMode==="quoted_region"&&!quotes.length)return null;
  const captureName=c.captureName||"selectedContent",captures={matchedCue,matchedFamily:c.familyId||c.familyName||null,familyScore:positiveScore};
  if(payloadMode==="quoted_region"){const chosen=quotes.length===1?quotes[0]:quotes[0];captures[captureName]=chosen?.text??"";captures[`${captureName}ScopeId`]=(ctx?.canonicalSemanticState?.scopes||[]).find(scope=>scope?.start===chosen?.start&&scope?.end===chosen?.end)?.id||null;captures[`${captureName}Start`]=chosen?.start??null;captures[`${captureName}End`]=chosen?.end??null;captures[`${captureName}All`]=quotes.map(q=>q.text);}
  return captures;
}
 v502RegisterCondition("learned_surface_family",(c,ctx)=>{const captures=v6210LearnedFamilyMatch(c,ctx);if(!captures)return false;ctx.patternCaptures={...(ctx.patternCaptures||{}),...captures};ctx.captures={...(ctx.captures||{}),...captures};return true;});
v502RegisterCondition("goal_exists",(c,ctx)=>{const state=ctx?.canonicalSemanticState||ctx?.cognitiveState||ctx?.workspace?.canonicalSemanticState||ctx;return(state?.unresolved||[]).some(g=>!c.goalType||v502Canonical(g.type)===v502Canonical(c.goalType));});
v502RegisterCondition("category_active",(c,ctx)=>{const wanted=v502Canonical(c.category);return(ctx?.activeCategories||[]).some(x=>v502Canonical(x)===wanted);});
v502RegisterCondition("structure_exists",(c,ctx)=>{const list=v502GetPath(ctx,c.listPath||"canonicalSemanticState.structures")||[];return Array.isArray(list)&&list.some(x=>(!c.kind||v502Canonical(x.kind)===v502Canonical(c.kind))&&(!c.category||v502Canonical(x.category)===v502Canonical(c.category)));});
function v6210ItemMatches(item,match,ctx){for(const [path,expectedSpec] of Object.entries(match||{})){const expected=v620ResolveTemplate(expectedSpec,ctx),actual=v502GetPath(item,path);if(Array.isArray(expected)){if(!expected.some(value=>v502Canonical(value)===v502Canonical(actual)))return false;continue;}if(typeof expected==="object"&&expected!==null){if(!v502Same(actual,expected))return false;continue;}if(v502Canonical(actual)!==v502Canonical(expected))return false;}return true;}
v502RegisterCondition("list_item_matches",(c,ctx)=>{const list=v502GetPath(ctx,c.listPath)||[];return Array.isArray(list)&&list.some(item=>v6210ItemMatches(item,c.match||{},ctx));});
v502RegisterCondition("scope_exists",(c,ctx)=>{const state=v620SemanticTarget(ctx)||ctx,list=state?.scopes||[];return Array.isArray(list)&&list.some(scope=>(!c.scopeId||scope.id===c.scopeId)&&(!c.kind||v502Canonical(scope.kind)===v502Canonical(c.kind))&&(!c.boundaryKind||v502Canonical(scope.boundaryKind)===v502Canonical(c.boundaryKind))&&(!c.nonRoot||scope.parentId));});

/* ---------------------------------------------------------
   GENERIC MODIFIERS / COGNITIVE INSTRUCTION SET
   --------------------------------------------------------- */
function v620SemanticTarget(ctx){return ctx?.canonicalSemanticState||ctx?.cognitiveState||ctx?.workspace?.canonicalSemanticState||ctx?.workspace?.cognitiveState||null;}
function v620SemanticAppend(ctx,bucket,value){const target=v620SemanticTarget(ctx);if(!target)return false;target[bucket] ||= [];const resolved=v620ResolveTemplate(value,ctx);if(target[bucket].some(x=>v502Same(x,resolved)))return false;target[bucket].push(resolved);target.updatedAt=Date.now();return true;}
function v620Compare(a,b){if(typeof a==="number"&&typeof b==="number")return a===b?"equal":a<b?"less_than":"greater_than";const ca=v502Canonical(a),cb=v502Canonical(b);if(ca===cb)return"equal";if(ca&&cb&&ca.includes(cb))return"contains";if(ca&&cb&&cb.includes(ca))return"contained_by";return"not_equal";}

v502RegisterOperation("set_field",(args,ctx)=>v502SetPath(ctx,args.path,v620ResolveTemplate(args.value,ctx)));
v502RegisterOperation("append_field",(args,ctx)=>v502AppendPath(ctx,args.path,v620ResolveTemplate(args.value,ctx),{unique:Boolean(args.unique)}));
v502RegisterOperation("copy_field",(args,ctx)=>v502SetPath(ctx,args.toPath||args.path,v502Clone(v502GetPath(ctx,args.fromPath))));
v502RegisterOperation("select_matching_item",(args,ctx)=>{const list=v502GetPath(ctx,args.listPath)||[];if(!Array.isArray(list))return false;const item=list.find(candidate=>v6210ItemMatches(candidate,args.match||{},ctx));if(!item)return false;return v502SetPath(ctx,args.outputPath||"selectedItem",v502Clone(item));});
v502RegisterOperation("remove_field",(args,ctx)=>v502DeletePath(ctx,args.path));
v502RegisterOperation("compare_values",(args,ctx)=>v502SetPath(ctx,args.outputPath||"comparison",{left:v502ReadValue(args.left??{fromPath:args.leftPath},ctx),right:v502ReadValue(args.right??{fromPath:args.rightPath},ctx),relation:v620Compare(v502ReadValue(args.left??{fromPath:args.leftPath},ctx),v502ReadValue(args.right??{fromPath:args.rightPath},ctx))}));
v502RegisterOperation("count_items",(args,ctx)=>{const v=v502GetPath(ctx,args.listPath),count=Array.isArray(v)?v.length:(v==null?0:1);return v502SetPath(ctx,args.outputPath||"count",count);});
v502RegisterOperation("branch",(args,ctx,step)=>{let changed=false;const test=args.if||args.condition,program=(typeof v520Predicate==="function"?v520Predicate(test,ctx):v502ConditionMatches(test,ctx))?(args.then||step.then||[]):(args.else||step.else||[]);for(const child of program)changed=v502ExecuteStep(child,ctx)||changed;return changed;});
v502RegisterOperation("for_each",(args,ctx,step)=>{const list=Array.isArray(v502GetPath(ctx,args.listPath))?v502GetPath(ctx,args.listPath):[],localPath=args.localPath||`locals.${args.as||"item"}`,old=v502Clone(v502GetPath(ctx,localPath)),had=v502GetPath(ctx,localPath)!==undefined;let changed=false;for(let i=0;i<list.length;i++){v502SetPath(ctx,localPath,list[i]);v502SetPath(ctx,`${localPath}Index`,i);for(const child of args.program||step.program||[])changed=v502ExecuteStep(child,ctx)||changed;}if(had)v502SetPath(ctx,localPath,old);else v502DeletePath(ctx,localPath);return changed;});
v502RegisterOperation("unique_list",(args,ctx)=>{const path=args.outputPath||args.listPath||args.path;return v502SetPath(ctx,path,v502Unique(v502GetPath(ctx,args.listPath||args.path)||[]));});
v502RegisterOperation("intersect_list",(args,ctx)=>{const current=Array.isArray(v502GetPath(ctx,args.targetPath))?v502GetPath(ctx,args.targetPath):[],allowed=Array.isArray(v502GetPath(ctx,args.allowedPath))?v502GetPath(ctx,args.allowedPath):[],keys=new Set(allowed.map(v=>v502Canonical(typeof v==="object"?(v.id||v.name||v.label||JSON.stringify(v)):v))),next=current.filter(v=>keys.has(v502Canonical(typeof v==="object"?(v.id||v.name||v.label||JSON.stringify(v)):v)));return v502SetPath(ctx,args.targetPath,next);});
v502RegisterOperation("preserve_field",(args,ctx)=>{ctx.__nova620Preserved||={};if(!args.path||Object.prototype.hasOwnProperty.call(ctx.__nova620Preserved,args.path))return false;ctx.__nova620Preserved[args.path]=v502Clone(v502GetPath(ctx,args.path));return true;});
v502RegisterOperation("restore_field",(args,ctx)=>{if(!args.path||!ctx.__nova620Preserved||!Object.prototype.hasOwnProperty.call(ctx.__nova620Preserved,args.path))return false;return v502SetPath(ctx,args.path,v502Clone(ctx.__nova620Preserved[args.path]));});
v502RegisterOperation("preserve_structure",(args,ctx)=>{ctx.preservedStructures||={};const key=args.key||args.path;if(!key||Object.prototype.hasOwnProperty.call(ctx.preservedStructures,key))return false;ctx.preservedStructures[key]=v502Clone(v502GetPath(ctx,args.path));return true;});
v502RegisterOperation("restore_structure",(args,ctx)=>{const key=args.key||args.path;if(!key||!ctx.preservedStructures||!Object.prototype.hasOwnProperty.call(ctx.preservedStructures,key))return false;return v502SetPath(ctx,args.path,v502Clone(ctx.preservedStructures[key]));});
v502RegisterOperation("ensure_field_equals",(args,ctx)=>v502SetPath(ctx,args.targetPath||args.path,v502Clone(v502GetPath(ctx,args.sourcePath))));
v502RegisterOperation("add_constraint",(args,ctx)=>{const value=v620ResolveTemplate(args.value,ctx);ctx.constraints ||= [];if(ctx.constraints.some(x=>v502Same(x,value)))return false;ctx.constraints.push(value);return true;});
v502RegisterOperation("mark_uncertain",(args,ctx)=>{const before=v502Clone(ctx.uncertainty||null);ctx.uncertainty={status:"uncertain",confidence:Number(args.confidence??0.45),reason:v620ResolveTemplate(args.reason||null,ctx)};if(ctx.mention){ctx.mention.uncertain=true;ctx.mention.confidence=Math.min(Number(ctx.mention.confidence??0.5),Number(args.confidence??0.45));}return !v502Same(before,ctx.uncertainty);});


/* ---------------------------------------------------------
   GENERAL CONNECTION / SUPPORT PRIMITIVES

   These operations are deliberately lower-level than curriculum concepts.
   They let learned cognition open a concept in persistent semantic memory,
   inspect its connected relationships, measure support, preserve nested
   contexts, and state explicit preconditions.  They do not know question
   words, lesson numbers, or domain-specific answers.
   --------------------------------------------------------- */
function v622KnowledgeStoreSafe(){try{return typeof v450KnowledgeStore==="function"?v450KnowledgeStore():null;}catch(_){return null;}}
function v622ResolveKnowledgeConceptValue(value,minScore=18){
  const raw=v502Clean(value);if(!raw)return null;
  try{if(typeof v450ResolveConcept==="function"){const hit=v450ResolveConcept(raw,{minScore});if(hit?.concept)return {id:hit.concept.id,key:hit.concept.key,name:hit.concept.name,aliases:v502Clone(hit.concept.aliases||[]),score:Number(hit.score||0)};}}catch(_){ }
  const store=v622KnowledgeStoreSafe();if(!store)return null;const words=new Set(v622ReferenceTokens(raw)),ranked=[];
  for(const concept of store.concepts||[]){let score=0;for(const label of [concept.name,...(concept.aliases||[])]){const key=v502Canonical(label);if(key===v502Canonical(raw))score=Math.max(score,100);const overlap=v622ReferenceTokens(label).filter(word=>words.has(word)).length;if(overlap)score=Math.max(score,overlap*20);}if(score)ranked.push({concept,score});}
  ranked.sort((a,b)=>b.score-a.score);const hit=ranked[0];return hit?{id:hit.concept.id,key:hit.concept.key,name:hit.concept.name,aliases:v502Clone(hit.concept.aliases||[]),score:hit.score}:null;
}
function v622KnowledgeConceptFromArgs(args,ctx){
  const explicit=args.concept??args.value??(args.fromPath?v502GetPath(ctx,args.fromPath):null)??v502GetPath(ctx,args.conceptPath||"");
  if(explicit&&typeof explicit==="object"&&explicit.id)return explicit;
  return v622ResolveKnowledgeConceptValue(explicit??ctx?.rawText??ctx?.input??"",Number(args.minScore??18));
}
function v622KnowledgeEdges(concept,{direction="both",depth=1,limit=80}={}){
  const store=v622KnowledgeStoreSafe();if(!store||!concept?.id)return [];
  const maxDepth=Math.max(1,Math.min(3,Number(depth||1))),out=[],queue=[{id:concept.id,depth:0}],seenNodes=new Set([concept.id]),seenEdges=new Set();
  while(queue.length&&out.length<limit){const current=queue.shift();if(current.depth>=maxDepth)continue;
    for(const claim of store.claims||[]){let nextId=null,edgeDirection=null;
      if(direction!=="incoming"&&claim.subjectId===current.id){nextId=claim.objectConceptId||null;edgeDirection="outgoing";}
      else if(direction!=="outgoing"&&claim.objectConceptId===current.id){nextId=claim.subjectId||null;edgeDirection="incoming";}else continue;
      const sig=claim.id||`${claim.subjectId}|${claim.predicate}|${claim.object}`;if(seenEdges.has(sig))continue;seenEdges.add(sig);
      out.push({id:claim.id||sig,subjectId:claim.subjectId,subject:claim.subject,predicate:claim.predicate,object:claim.object,objectConceptId:claim.objectConceptId||null,polarity:claim.polarity||"positive",confidence:Number(claim.confidence??0.5),qualifiers:v502Clone(claim.qualifiers||{}),evidence:v502Clone(claim.evidence||[]),depth:current.depth+1,direction:edgeDirection});
      if(nextId&&!seenNodes.has(nextId)){seenNodes.add(nextId);queue.push({id:nextId,depth:current.depth+1});}
      if(out.length>=limit)break;
    }
  }
  return out;
}
v502RegisterOperation("knowledge_resolve_concept",(args,ctx)=>{const concept=v622KnowledgeConceptFromArgs(args,ctx);if(!concept)return false;return v502SetPath(ctx,args.outputPath||"moduleState.focusConcept",v502Clone(concept));});
v502RegisterOperation("knowledge_collect_relations",(args,ctx)=>{const concept=v622KnowledgeConceptFromArgs(args,ctx);if(!concept)return false;const edges=v622KnowledgeEdges(concept,{direction:v502Canonical(args.direction||"both"),depth:Number(args.depth||1),limit:Number(args.limit||80)});let changed=v502SetPath(ctx,args.outputPath||"moduleState.focusRelations",edges);if(args.conceptOutputPath)changed=v502SetPath(ctx,args.conceptOutputPath,v502Clone(concept))||changed;return changed;});
v502RegisterOperation("knowledge_select_relevant_relations",(args,ctx)=>{
  const list=v502GetPath(ctx,args.listPath||"moduleState.focusRelations")||[];if(!Array.isArray(list))return false;const goalValue=args.goal??(args.goalPath?v502GetPath(ctx,args.goalPath):null)??(v620SemanticTarget(ctx)?.unresolved||[]).map(g=>[g.type,g.description,g.goalType]).flat();const goalTerms=new Set(v622ReferenceTokens(goalValue));
  const scored=list.map(edge=>{let score=Number(edge.confidence||0)*2+Math.max(0,4-Number(edge.depth||1));const edgeTerms=v622ReferenceTokens(edge.predicate,edge.object,edge.subject,edge.qualifiers);let overlap=0;for(const term of edgeTerms)if(goalTerms.has(term))overlap++;score+=overlap*4;return {...v502Clone(edge),relevanceScore:score,goalOverlap:overlap};}).sort((a,b)=>Number(b.relevanceScore||0)-Number(a.relevanceScore||0));
  return v502SetPath(ctx,args.outputPath||"moduleState.relevantRelations",scored.slice(0,Math.max(1,Number(args.limit||12))));
});
v502RegisterOperation("knowledge_form_supported_proposition",(args,ctx)=>{
  const list=v502GetPath(ctx,args.listPath||"moduleState.relevantRelations")||[];const edge=Array.isArray(list)?list[Number(args.index||0)]:null;if(!edge)return false;
  const proposition={id:`knowledge_prop_${Math.random().toString(36).slice(2,8)}`,type:"supported_knowledge_relation",subject:edge.subject,subjectId:edge.subjectId,predicate:edge.predicate,object:edge.object,objectConceptId:edge.objectConceptId||null,polarity:edge.polarity||"positive",confidence:Number(edge.confidence??0.5),support:{claimId:edge.id,evidence:v502Clone(edge.evidence||[]),depth:edge.depth||1},source:"persistent_semantic_graph"};
  let changed=v502SetPath(ctx,args.outputPath||"moduleState.supportedProposition",proposition);if(args.commit===true)changed=v620SemanticAppend(ctx,"propositions",proposition)||changed;return changed;
});
v502RegisterOperation("select_labeled_role",(args,ctx)=>{
  const item=args.item??(args.itemPath?v502GetPath(ctx,args.itemPath):null)??ctx?.unit??ctx?.instructionalItem;if(!item||typeof item!=="object")return false;
  const role=v502Clean(item.role||item.label||item.type||"");if(!role)return false;return v502SetPath(ctx,args.outputPath||"moduleState.selectedInstructionRole",{role,label:item.label||null,scope:item.scope||null,source:"explicit_label"});
});
v502RegisterOperation("preserve_ordered_items",(args,ctx)=>{
  const list=v502GetPath(ctx,args.listPath||"");if(!Array.isArray(list))return false;const copy=list.map((item,index)=>({index,value:v502Clone(item)}));return v502SetPath(ctx,args.outputPath||"moduleState.orderedItems",copy);
});
v502RegisterOperation("evaluate_support_conflict",(args,ctx)=>{
  const values=[];for(const path of args.paths||[args.listPath].filter(Boolean)){const value=v502GetPath(ctx,path);if(Array.isArray(value))values.push(...value);else if(value!=null)values.push(value);}
  if(!values.length){const semantic=v620SemanticTarget(ctx)||{};values.push(...(semantic.propositions||[]),...(semantic.evidence||[]));}const groups=new Map();
  for(const item of values){const key=v502Canonical(item?.key||item?.signature||`${item?.subjectId||item?.subject||""}|${item?.predicate||item?.type||""}|${item?.objectId||item?.object||item?.value||""}`),polarity=v502Canonical(item?.polarity||item?.status||"positive");if(!key)continue;if(!groups.has(key))groups.set(key,{key,support:[],conflict:[]});const row=groups.get(key);if(["negative","false","rejected","contradicted"].includes(polarity))row.conflict.push(v502Clone(item));else row.support.push(v502Clone(item));}
  const result=[...groups.values()].map(row=>({...row,hasConflict:Boolean(row.support.length&&row.conflict.length)}));return v502SetPath(ctx,args.outputPath||"moduleState.supportConflict",result);
});
v502RegisterOperation("inspect_cognitive_status",(args,ctx)=>{
  let rules=[];try{rules=typeof v500CognitiveRuleInventory==="function"?(v500CognitiveRuleInventory()?.rules||[]):[];}catch(_){ }const summary={total:rules.length,executable:rules.filter(r=>r.status==="module_executable").length,storedOnly:rules.filter(r=>r.status!=="module_executable").length,bySource:{}};for(const rule of rules){const key=rule.sourceDisplayName||rule.sourceName||"conversation";summary.bySource[key] ||= {total:0,executable:0,storedOnly:0};summary.bySource[key].total++;if(rule.status==="module_executable")summary.bySource[key].executable++;else summary.bySource[key].storedOnly++;}return v502SetPath(ctx,args.outputPath||"moduleState.cognitiveStatus",summary);
});
v502RegisterOperation("retrieve_source_evidence",(args,ctx)=>{
  const candidates=[args.fromPath?v502GetPath(ctx,args.fromPath):null,ctx?.immutable?.rawInput,ctx?.workspace?.rawInput,ctx?.rawText,ctx?.source?.text].filter(value=>value!=null);if(!candidates.length)return false;return v502SetPath(ctx,args.outputPath||"moduleState.sourceEvidence",v502Clone(candidates[0]));
});

v502RegisterOperation("measure_candidate_support",(args,ctx)=>{
  const listPath=args.listPath||"candidates",list=(v502GetPath(ctx,listPath)||[]).map(candidate=>({...candidate})),semantic=v620SemanticTarget(ctx)||{},roots=[...(semantic.relations||[]),...(semantic.events||[]),...(semantic.propositions||[]),...(semantic.evidence||[])];
  for(const candidate of list){const labels=v502Unique([candidate.entity,candidate.label,candidate.name,candidate.id].filter(Boolean),v502Canonical),reasons=[...(candidate.supportReasons||[])];let support=Number(candidate.support||candidate.score||0);for(const root of roots){let text="";try{text=v502Canonical(JSON.stringify(root));}catch(_){text=v502Canonical(root);}if(labels.some(label=>label&&text.includes(v502Canonical(label)))){support+=Number(args.matchWeight??1);reasons.push("represented_relationship_support");}}candidate.support=support;candidate.score=Math.max(Number(candidate.score||0),support);candidate.supportReasons=v502Unique(reasons,v502Canonical);}list.sort((a,b)=>Number(b.support||0)-Number(a.support||0));return v502SetPath(ctx,args.outputPath||listPath,list);
});
v502RegisterOperation("require_preconditions",(args,ctx)=>{
  const paths=v502Unique(args.paths||[],String),missing=paths.filter(path=>{const value=v502GetPath(ctx,path);return value===undefined||value===null||(args.nonEmpty===true&&Array.isArray(value)&&!value.length);});let changed=v502SetPath(ctx,args.outputPath||"moduleState.preconditions",{required:paths,missing,satisfied:missing.length===0});
  if(missing.length&&args.registerUnresolved===true){const target=v620SemanticTarget(ctx)||ctx;target.unresolved ||= [];const label=args.label||"required cognitive precondition";if(!target.unresolved.some(g=>v502Canonical(g.description||g.label)===v502Canonical(label))){target.unresolved.push({id:`precondition_${Math.random().toString(36).slice(2,8)}`,type:"cognitive_precondition",description:label,missingPaths:v502Clone(missing),source:"learned_precondition"});changed=true;}}return changed;
});
v502RegisterOperation("scope_create_child",(args,ctx)=>{
  const state=v620SemanticTarget(ctx);if(!state)return false;state.scopes ||= [];const parentSource=args.parentId??(args.parentPath?{fromPath:args.parentPath}:(state.activeScopeId||"scope_discourse")),parentId=v620ResolveTemplate(parentSource,ctx),kind=v502Clean(v620ResolveTemplate(args.kind||"nested_scope",ctx)),identity=v502Clean(v620ResolveTemplate(args.identity||args.id||"",ctx));
  let existing=identity?state.scopes.find(scope=>scope.id===identity):null;if(existing){if(args.outputPath)v502SetPath(ctx,args.outputPath,v502Clone(existing));return false;}const scope={id:identity||`scope_${Math.random().toString(36).slice(2,8)}`,kind,parentId:parentId||"scope_discourse",boundaryKind:v620ResolveTemplate(args.boundaryKind||null,ctx),participantFrame:v502Clone(v620ResolveTemplate(args.participantFrame||null,ctx)),policies:v502Clone(v620ResolveTemplate(args.policies||{},ctx)),source:args.source||"learned_nested_scope"};state.scopes.push(scope);state.updatedAt=Date.now();if(args.outputPath)v502SetPath(ctx,args.outputPath,v502Clone(scope));return true;
});
v502RegisterOperation("scope_set_active",(args,ctx)=>{const state=v620SemanticTarget(ctx);if(!state)return false;const id=v620ResolveTemplate(args.scopeId??(args.fromPath?{fromPath:args.fromPath}:null),ctx);if(!id||(state.scopes||[]).every(scope=>scope.id!==id))return false;const before=state.activeScopeId;state.activeScopeId=id;return before!==id;});
v502RegisterOperation("scope_restore_parent",(args,ctx)=>{const state=v620SemanticTarget(ctx);if(!state)return false;const current=(state.scopes||[]).find(scope=>scope.id===state.activeScopeId),parent=current?.parentId;if(!parent)return false;state.activeScopeId=parent;return true;});

/* Generic scoped-context operations. Surface boundaries create scope records;
   learned cognition can add policies/bindings and resolve references within those
   scopes. These operations know nothing about the semantic meaning of quotation,
   hypotheticals, stories, beliefs, or any other particular scope type. */
function v627ScopeState(ctx){return v620SemanticTarget(ctx)||ctx;}
function v627ScopeMatches(scope,selector={}){if(!scope)return false;if(selector.scopeId&&scope.id!==selector.scopeId)return false;if(selector.nonRoot&&!scope.parentId)return false;if(selector.kind&&v502Canonical(scope.kind)!==v502Canonical(selector.kind))return false;if(selector.boundaryKind&&v502Canonical(scope.boundaryKind)!==v502Canonical(selector.boundaryKind))return false;return true;}
function v627ScopeById(ctx,id){return (v627ScopeState(ctx)?.scopes||[]).find(scope=>scope.id===id)||null;}
function v627ScopeReferences(ctx,selector={}){const state=v627ScopeState(ctx),refs=state?.references||[];return refs.filter(ref=>v627ScopeMatches(v627ScopeById(ctx,ref.scopeId||"scope_discourse"),selector));}
v502RegisterOperation("scope_apply_policy",(args,ctx)=>{const state=v627ScopeState(ctx),selector=args.selector||{},policy=v620ResolveTemplate(args.policy||{},ctx);let changed=false;for(const scope of state?.scopes||[]){if(!v627ScopeMatches(scope,selector))continue;scope.policies ||= {};for(const [key,value] of Object.entries(policy)){if(v502Same(scope.policies[key],value))continue;scope.policies[key]=v502Clone(value);changed=true;}}if(changed)state.updatedAt=Date.now();return changed;});
v502RegisterOperation("scope_bind_participant",(args,ctx)=>{const state=v627ScopeState(ctx),selector=args.selector||{},role=v502Canonical(args.role||"");if(!state||!role)return false;const value=v620ResolveTemplate(args.participant??(args.fromPath?{fromPath:args.fromPath}:null),ctx);if(value==null)return false;let changed=false;for(const scope of state.scopes||[]){if(!v627ScopeMatches(scope,selector))continue;scope.participantFrame ||= {};const normalized=typeof value==="object"?v502Clone(value):{id:String(value),label:String(value),role};normalized.role ||= role;if(!v502Same(scope.participantFrame[role],normalized)){scope.participantFrame[role]=normalized;changed=true;}}if(changed)state.updatedAt=Date.now();return changed;});
v502RegisterOperation("scope_enforce_isolation",(args,ctx)=>{const selector=args.selector||{nonRoot:true},state=v627ScopeState(ctx);if(!state)return false;let changed=false;for(const ref of v627ScopeReferences(ctx,selector)){const scope=v627ScopeById(ctx,ref.scopeId);const role=ref.grammaticalPerson==="second"?"listener":ref.grammaticalPerson==="first_plural"?"group":"speaker",local=scope?.participantFrame?.[role]||null;if(local)continue;if(ref.resolvedEntity!=null||ref.participantRole!=null||ref.locked===true){ref.resolvedEntity=null;ref.participantRole=null;ref.locked=false;ref.resolutionSource="scoped_deictic_unresolved";changed=true;}}if(changed)state.updatedAt=Date.now();return changed;});
v502RegisterOperation("scope_resolve_deictics",(args,ctx)=>{const selector=args.selector||{},state=v627ScopeState(ctx);if(!state)return false;let changed=false;for(const ref of v627ScopeReferences(ctx,selector)){const scope=v627ScopeById(ctx,ref.scopeId),role=ref.grammaticalPerson==="second"?"listener":ref.grammaticalPerson==="first_plural"?"group":"speaker";let frame=scope?.participantFrame||null,participant=frame?.[role]||null;if(!participant&&args.allowParentFallback===true){let parent=v627ScopeById(ctx,scope?.parentId);while(parent&&!participant){participant=parent.participantFrame?.[role]||null;parent=v627ScopeById(ctx,parent.parentId);}}if(!participant)continue;const target=participant.label||participant.name||participant.id;if(ref.resolvedEntity!==target||ref.participantRole!==role||ref.resolutionSource!=="learned_scoped_frame"){ref.resolvedEntity=target;ref.participantRole=role;ref.resolutionSource="learned_scoped_frame";ref.locked=true;changed=true;}}if(changed)state.updatedAt=Date.now();return changed;});
v502RegisterOperation("perspective_subject",(args,ctx)=>{const state=v627ScopeState(ctx),claim=ctx?.claim||v502GetPath(ctx,args.claimPath||"claim"),ownerId=claim?.qualifiers?.semanticOwnerId??claim?.semanticOwnerId??v502GetPath(ctx,args.ownerPath||"");if(!ownerId)return false;const root=(state?.scopes||[]).find(scope=>scope.id===(args.scopeId||state?.activeScopeId||"scope_discourse"))||(state?.scopes||[]).find(scope=>scope.id==="scope_discourse"),speaker=root?.participantFrame?.speaker||state?.participants?.speaker,listener=root?.participantFrame?.listener||state?.participants?.listener;let surface=null;if(String(ownerId)===String(speaker?.id))surface=args.firstPerson||"I";else if(String(ownerId)===String(listener?.id))surface=args.secondPerson||"you";else surface=claim?.subjectLabel||claim?.subject||ownerId;return v502SetPath(ctx,args.outputPath||"surfaceSubject",surface);});

/* Generic participant/candidate operations. Participant identity is bootstrap
   structure; language-to-participant mappings can still be taught as modules. */
v502RegisterOperation("resolve_participant",(args,ctx)=>{if(!ctx.mention)return false;const participant=v502GetPath(ctx,`participants.${args.role||"listener"}`);if(!participant)return false;const target=participant.label||participant.name||participant.id,before=ctx.mention.resolvedEntity;ctx.mention.resolvedEntity=target;ctx.mention.participantRole=args.role||"listener";ctx.mention.confidence=Math.max(Number(ctx.mention.confidence||0),Number(args.confidence??0.99));ctx.mention.resolutionSource="learned_module";return before!==target;});
v502RegisterOperation("collect_candidates",(args,ctx)=>{const semantic=v620SemanticTarget(ctx),out=[];const add=(value,source,extra={})=>{const label=typeof value==="string"?value:(value?.label||value?.name||value?.surface||value?.id);if(!label||out.some(x=>v502Canonical(x.entity)===v502Canonical(label)))return;out.push({entity:label,source,...extra});};for(const e of semantic?.entities||[])add(e,"current_semantic",{type:e.type||e.kind||null});for(const e of ctx?.recentEntities||ctx?.workspace?.recentEntities||[])add(e,"recent_discourse");for(const role of args.includeParticipants||[])add(ctx?.participants?.[role],role);return v502SetPath(ctx,args.outputPath||"candidates",out);});
v502RegisterOperation("filter_candidates",(args,ctx)=>{const path=args.listPath||"candidates",before=v502GetPath(ctx,path)||[];let list=before;if(args.type)list=list.filter(c=>!c.type||v502Canonical(c.type)===v502Canonical(args.type));if(args.source)list=list.filter(c=>v502Canonical(c.source)===v502Canonical(args.source));if(args.excludeSources)list=list.filter(c=>!args.excludeSources.some(s=>v502Canonical(s)===v502Canonical(c.source)));return v502SetPath(ctx,args.outputPath||path,list);});
v502RegisterOperation("score_candidates",(args,ctx)=>{const path=args.listPath||"candidates",list=(v502GetPath(ctx,path)||[]).map(x=>({...x}));for(const item of list){let score=Number(item.score||0);for(const criterion of args.criteria||[]){const kind=v502Canonical(criterion.kind),weight=Number(criterion.weight??1);if(kind==="source_equals"&&v502Canonical(item.source)===v502Canonical(criterion.value))score+=weight;if(kind==="field_equals"&&v502Canonical(item[criterion.field])===v502Canonical(criterion.value))score+=weight;}item.score=score;}list.sort((a,b)=>Number(b.score||0)-Number(a.score||0));return v502SetPath(ctx,args.outputPath||path,list);});
v502RegisterOperation("select_best_candidate",(args,ctx)=>{const list=v502GetPath(ctx,args.listPath||"candidates")||[],best=list[0]||null;let changed=v502SetPath(ctx,args.outputPath||"selectedCandidate",v502Clone(best));if(best&&args.assignToMention&&ctx.mention){const before=ctx.mention.resolvedEntity;ctx.mention.resolvedEntity=best.entity||best.label||best.name;ctx.mention.confidence=Number(args.confidence??0.82);changed=before!==ctx.mention.resolvedEntity||changed;}return changed;});
v502RegisterOperation("assign_reference",(args,ctx)=>{const best=ctx.selectedCandidate||(ctx.candidates||[])[0];if(!best||!ctx.mention)return false;const before=ctx.mention.resolvedEntity;ctx.mention.resolvedEntity=best.entity||best.label||best.name;ctx.mention.confidence=Number(args.confidence??0.9);ctx.mention.resolutionSource="learned_module";return before!==ctx.mention.resolvedEntity;});

/* ---------------------------------------------------------
   V7 PASS 2 OWNERSHIP BOUNDARY

   Surface-to-semantic construction, request construction, concept activation,
   relationship traversal, proposition composition, and the migrated request
   resolver now live in src-tauri/src/nova/semantics/. They are intentionally
   absent here so JavaScript cannot silently become a second semantic brain.
   Learned modules may still manipulate the shared canonical structures using
   the generic semantic_add_* operations below until their runtime migrates.
   --------------------------------------------------------- */

/* Canonical semantic structures are just state structures operated on by the
   same reasoning kernel. No lexical meaning is encoded here. */
for(const [op,bucket] of [["semantic_add_frame","frames"],["semantic_add_entity","entities"],["semantic_add_event","events"],["semantic_add_relation","relations"],["semantic_add_proposition","propositions"],["semantic_add_request","requests"],["semantic_add_unknown_slot","unknownSlots"],["semantic_add_candidate_set","candidateSets"],["semantic_add_evidence","evidence"],["semantic_add_conclusion","conclusions"]])v502RegisterOperation(op,(args,ctx)=>v620SemanticAppend(ctx,bucket,args));
v502RegisterOperation("create_structure",(args,ctx)=>{const path=args.listPath||"canonicalSemanticState.structures",value=v620ResolveTemplate(args.value??args.structure??{},ctx);return v502AppendPath(ctx,path,value,{unique:args.unique!==false});});
v502RegisterOperation("query_structures",(args,ctx)=>{const list=Array.isArray(v502GetPath(ctx,args.listPath||"canonicalSemanticState.structures"))?v502GetPath(ctx,args.listPath||"canonicalSemanticState.structures"):[],match=args.match||{},out=list.filter(item=>Object.entries(match).every(([k,v])=>v502Canonical(v502GetPath(item,k))===v502Canonical(v620ResolveTemplate(v,ctx))));return v502SetPath(ctx,args.outputPath||"queryResults",v502Clone(out));});
v502RegisterOperation("link_structures",(args,ctx)=>{const relation={subject:v620ResolveTemplate(args.subject??{fromPath:args.subjectPath},ctx),predicate:v620ResolveTemplate(args.predicate||"related_to",ctx),object:v620ResolveTemplate(args.object??{fromPath:args.objectPath},ctx),qualifiers:v620ResolveTemplate(args.qualifiers||{},ctx)};return v620SemanticAppend(ctx,args.bucket||"relations",relation);});
v502RegisterOperation("create_goal",(args,ctx)=>{const target=v620SemanticTarget(ctx)||ctx;target.unresolved ||= [];const goal=v620ResolveTemplate(args.goal||args,ctx);return v502AppendPath(target,"unresolved",goal,{unique:true});});
v502RegisterOperation("resolve_goal",(args,ctx)=>{const target=v620SemanticTarget(ctx)||ctx,list=target.unresolved||[],id=v620ResolveTemplate(args.id||args.goalId||null,ctx),type=v620ResolveTemplate(args.type||args.goalType||null,ctx),description=v620ResolveTemplate(args.description||null,ctx),next=list.filter(g=>{if(id&&v502Canonical(g.id)===v502Canonical(id))return false;if(type&&v502Canonical(g.type)===v502Canonical(type))return false;if(description&&v502Canonical(g.description)===v502Canonical(description))return false;return true;});if(next.length===list.length)return false;target.unresolved=next;return true;});
v502RegisterOperation("emit_surface",(args,ctx)=>v502SetPath(ctx,args.outputPath||"moduleState.surfaceResponse",String(v620ResolveTemplate(args.text??args.value??"",ctx))));
v502RegisterOperation("register_category",(args,ctx)=>{const name=v620ResolveTemplate(args.name??args.category,ctx),before=(v620CategoryStore().categories||[]).length,category=v620RegisterCategory(name,{origin:args.origin||"learned",traits:v620ResolveTemplate(args.traits||[],ctx),evidence:v620ResolveTemplate(args.evidence||ctx?.module?.evidence||[],ctx)});if(args.outputPath)v502SetPath(ctx,args.outputPath,v502Clone(category));return (v620CategoryStore().categories||[]).length!==before;});
v502RegisterOperation("relate_categories",(args,ctx)=>{const store=v620CategoryStore(),before=(store.relations||[]).length,rel=v620RelateCategories(v620ResolveTemplate(args.subject,ctx),v620ResolveTemplate(args.predicate||"related_to",ctx),v620ResolveTemplate(args.object,ctx),{origin:args.origin||"learned",evidence:v620ResolveTemplate(args.evidence||ctx?.module?.evidence||null,ctx)});if(args.outputPath)v502SetPath(ctx,args.outputPath,v502Clone(rel));return (store.relations||[]).length!==before;});
v502RegisterOperation("activate_category",(args,ctx)=>{ctx.activeCategories ||= [];const name=v502Clean(v620ResolveTemplate(args.name??args.category,ctx));if(!name||ctx.activeCategories.some(x=>v502Canonical(x)===v502Canonical(name)))return false;ctx.activeCategories.push(name);v620RegisterCategory(name,{origin:"learned",activationIncrement:1});return true;});
v502RegisterOperation("invoke_module",(args,ctx)=>{const depth=Number(ctx.__nova620InvokeDepth||0);if(depth>=NOVA620_MAX_INVOKE_DEPTH)return false;const wanted=v620ResolveTemplate(args.number??args.id??args.name??args.category??"",ctx);if(wanted==null||wanted==="")return false;const module=v622ResolveModuleReference(wanted);if(!module||module.enabled===false||module===ctx.module)return false;const prior=ctx.module;ctx.__nova620InvokeDepth=depth+1;ctx.module=module;let changed=false;try{if(!v502ModuleMatches(module,ctx))return false;for(const step of module.program||[])changed=v502ExecuteStep(step,ctx)||changed;return changed;}finally{ctx.module=prior;ctx.__nova620InvokeDepth=depth;}});

function v502ExecuteStep(step,ctx){const op=v502Canonical(step?.op).replace(/\s+/g,"_"),handler=NOVA620_OPERATION_HANDLERS.get(op);if(!handler)throw new Error(`Unsupported cognitive operation: ${step?.op||"missing"}`);return Boolean(handler(step?.args||{},ctx,step));}
function v502RunCognitiveModules(categoryOrPhase=null,ctx={}){
  const store=v502ModuleStore(),considered=[],applied=[],changed=[],references=v622RelevantModuleReferences(ctx,categoryOrPhase);const refByNumber=new Map(references.map(ref=>[ref.number,ref]));const modules=references.map(ref=>NOVA622_MODULE_BY_NUMBER.get(ref.number)).filter(Boolean).sort((a,b)=>Number(refByNumber.get(b.moduleNumber)?.experienceWeight||0)-Number(refByNumber.get(a.moduleNumber)?.experienceWeight||0)||(b.priority||0)-(a.priority||0));
  for(const module of modules){const ref=NOVA622_REFERENCE_BY_NUMBER.get(module.moduleNumber),attentionWeight=Number(refByNumber.get(module.moduleNumber)?.experienceWeight||0);considered.push({number:module.moduleNumber,id:module.id,name:module.name,reference:v622ReferenceLine(ref),phase:module.phase,categories:v502Clone(module.categories||[]),sourceType:module.sourceType,sourceName:module.sourceName,attentionWeight});const oldCaptures=ctx.captures,oldPattern=ctx.patternCaptures;ctx.captures={};ctx.patternCaptures={};let matches=false;try{matches=v502ModuleMatches(module,ctx);}catch(error){applied.push({number:module.moduleNumber,id:module.id,name:module.name,error:String(error?.message||error),progress:false,attentionWeight,deltaScore:0});ctx.captures=oldCaptures;ctx.patternCaptures=oldPattern;continue;}if(!matches){ctx.captures=oldCaptures;ctx.patternCaptures=oldPattern;continue;}let progress=false;ctx.module=module;const before=v622CognitiveFingerprint(ctx);try{for(const step of module.program||[])progress=v502ExecuteStep(step,ctx)||progress;}catch(error){const after=v622CognitiveFingerprint(ctx),deltaScore=v622ContributionDelta(before,after,progress);applied.push({number:module.moduleNumber,id:module.id,name:module.name,phase:module.phase,categories:v502Clone(module.categories||[]),sourceType:module.sourceType,sourceName:module.sourceName,progress,error:String(error?.message||error),attentionWeight,deltaScore});ctx.captures=oldCaptures;ctx.patternCaptures=oldPattern;continue;}const after=v622CognitiveFingerprint(ctx),deltaScore=v622ContributionDelta(before,after,progress),row={number:module.moduleNumber,id:module.id,name:module.name,reference:v622ReferenceLine(ref),phase:module.phase,categories:v502Clone(module.categories||[]),sourceType:module.sourceType,sourceName:module.sourceName,progress,attentionWeight,deltaScore};applied.push(row);if(progress)changed.push(row);ctx.captures=oldCaptures;ctx.patternCaptures=oldPattern;}
  delete ctx.module;const search=v622RecordTurnWorkingSet(ctx,applied,changed);store.lastRun={selector:categoryOrPhase||null,searchMode:search?.mode||null,experienceRouteIds:v502Clone(search?.routeIds||[]),relevant:references.map(x=>x.line),considered:considered.length,applied:v502Clone(applied),changed:v502Clone(changed),ranAt:Date.now()};store.updatedAt=Date.now();return {ctx,references,considered,applied,changed,progressCount:changed.length,searchMode:search?.mode||null,experienceRouteIds:v502Clone(search?.routeIds||[])};
}

/* Existing Stored Only cognition is reconsidered by the current synthesizer.
   This function remains as a stable API name because saved state and inspector
   code may call it; it contains no older execution path. */
function v502UpgradeStoredGuidance(){if(window.__TAURI__?.core?.invoke)return {upgraded:0,pending:true,authority:"rust_v7_pass_8"};return typeof v520GeneralSynthesisPass==="function"?v520GeneralSynthesisPass({reason:"browser_generic_module_reconsideration",force:true}):{upgraded:0,pending:true};}
function v502CognitiveModuleInventory(){const store=v502ModuleStore(),modules=(store.modules||[]).slice();return {version:NOVA620_VERSION,total:modules.length,learned:modules.filter(m=>m.sourceType==="learned").length,builtIn:modules.filter(m=>m.sourceType==="built_in").length,enabled:modules.filter(m=>m.enabled).length,byCategory:modules.reduce((out,m)=>{for(const c of m.categories||[]){out[c]=(out[c]||0)+1;}return out;},{}),modules,moduleMap:v622CognitiveMapSummary(),lastCompilation:store.lastCompilation,lastRun:store.lastRun,categories:v502Clone(v620CategoryStore())};}

try{if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){V41_SELF_MODEL.capabilities ||= [];const id="generic_cognitive_modules";const entry={id,verb:"compose",object:"new learned cognitive programs and categories from generic modifiers",detail:"My learned modules are persistent declarative cognition with stable numbered descriptors. In native V7 Pass 4, Rust executes them; the JavaScript interpreter remains only for browser migration tests. My experience-built cognitive index grows from successful reasoning traces instead of being prefilled from module metadata."};const existing=V41_SELF_MODEL.capabilities.find(x=>x.id===id);if(existing)Object.assign(existing,entry);else V41_SELF_MODEL.capabilities.push(entry);}}catch(_){ }

console.info("Nova V7 Pass 4 learned-module catalog loaded — native execution is Rust; JavaScript execution is browser/testing only.");
