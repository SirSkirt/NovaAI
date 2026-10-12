"use strict";

/* =========================================================
   Nova V6.2.16 — PROVISIONAL COGNITIVE ABSTRACTION

   Fourth staged piece of the developmental-brain architecture:
   - inspect learned semantic relationships and executable cognitive programs
     for recurring structure;
   - form persistent *provisional* abstractions with explicit evidence rather
     than requiring every reusable higher-order term to be hard-coded;
   - keep the original claims/rules authoritative and preserve their source
     provenance;
   - require multiple distinct instances before proposing an abstraction;
   - never turn an abstraction candidate into a fact, executable program, or
     response-path behavior merely because a pattern was noticed.

   V6.2 DEVELOPMENTAL BOUNDARY:
   Experience-shaped abstractions may create PROVISIONAL entries in the same open
   category graph used by the current mind. They remain non-authoritative until
   later learning supplies enough support to validate/use them.
   ========================================================= */

const NOVA554_VERSION="6.2.16";
const NOVA554_ABSTRACTION_SCHEMA=2;
const NOVA554_ABSTRACTION_MIGRATION_SCHEMA=1;
const NOVA554_MIN_RELATION_INSTANCES=3;
const NOVA554_MIN_COGNITIVE_INSTANCES=3;
const NOVA554_HISTORY_LIMIT=80;
const NOVA554_EPISODE_SCHEMA=1;
const NOVA554_MAX_EPISODES=240;
const NOVA554_CONSOLIDATE_EVERY=8;

function v554Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v554Canon(v){try{return canonical(String(v??""));}catch(_){return String(v??"").trim().toLowerCase();}}
function v554Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v554Key(v){return v554Canon(v).replace(/[^a-z0-9]+/g,"_").replace(/^_+|_+$/g,"");}
function v554Unique(xs,keyFn=x=>typeof x==="object"?JSON.stringify(x):String(x)){const out=[],seen=new Set();for(const x of xs||[]){if(x==null)continue;const k=keyFn(x);if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}
function v554Readable(v){return v554Clean(String(v??"").replace(/_/g," "));}

function v554AbstractionStore(){
  const memory=v43Memory();
  if(!memory.cognitiveAbstractions||memory.cognitiveAbstractions.schema!==NOVA554_ABSTRACTION_SCHEMA){
    const old=memory.cognitiveAbstractions||{};
    memory.cognitiveAbstractions={
      schema:NOVA554_ABSTRACTION_SCHEMA,version:NOVA554_VERSION,
      abstractions:Array.isArray(old.abstractions)?old.abstractions:[],
      history:Array.isArray(old.history)?old.history.slice(-NOVA554_HISTORY_LIMIT):[],
      lastPass:old.lastPass||null,migration:old.migration||null,
      createdAt:old.createdAt||Date.now(),updatedAt:Date.now()
    };
  }
  memory.cognitiveAbstractions.version=NOVA554_VERSION;
  return memory.cognitiveAbstractions;
}

function v554EpisodeStore(){
  const memory=v43Memory(),prior=memory.developmentEpisodes||{};
  if(!memory.developmentEpisodes||prior.schema!==NOVA554_EPISODE_SCHEMA)memory.developmentEpisodes={schema:NOVA554_EPISODE_SCHEMA,version:NOVA554_VERSION,episodes:Array.isArray(prior.episodes)?prior.episodes.slice(-NOVA554_MAX_EPISODES):[],sinceConsolidation:Number(prior.sinceConsolidation||0),lastConsolidatedAt:prior.lastConsolidatedAt||null,lastConsolidation:prior.lastConsolidation||null,createdAt:prior.createdAt||Date.now(),updatedAt:Date.now()};
  memory.developmentEpisodes.version=NOVA554_VERSION;return memory.developmentEpisodes;
}
function v554PromptHash(text){let h=2166136261;for(const ch of String(text||"")){h^=ch.charCodeAt(0);h=Math.imul(h,16777619);}return (h>>>0).toString(36);}
function v554RecordDevelopmentEpisode(ws,report={}){
  if(window.__TAURI__?.core?.invoke)return null;
  if(!ws)return null;const store=v554EpisodeStore(),signature=typeof v622ExperienceSignature==="function"?v622ExperienceSignature({workspace:ws,utteranceObservation:ws.utteranceObservation,canonicalSemanticState:ws.canonicalSemanticState,cognitiveState:ws.canonicalSemanticState}):{conceptTerms:[],goalTerms:[],relationTerms:[]};
  const contributors=(report.changed||[]).filter(row=>Number.isInteger(row?.number)&&row?.progress!==false).map(row=>({number:row.number,deltaScore:Number(row.deltaScore||1),name:row.name||null})).sort((a,b)=>b.deltaScore-a.deltaScore).slice(0,24);
  const episode={id:`episode_${Date.now()}_${Math.random().toString(36).slice(2,7)}`,promptHash:v554PromptHash(ws.rawInput),promptPreview:v554Clean(ws.rawInput).slice(0,180),executionMode:ws.executionMode||"USER_TURN",terminalState:ws.executive?.terminalState||null,reasonCode:ws.executive?.reasonCode||null,supported:ws.executive?.terminalState==="SUPPORTED",conceptTerms:v554Clone(signature.conceptTerms||[]),goalTerms:v554Clone(signature.goalTerms||[]),relationTerms:v554Clone(signature.relationTerms||[]),contributors,experienceRouteId:report?.experienceRoute?.id||ws.executive?.details?.experienceRoute?.id||null,cycles:Number(report?.cycles||ws.executive?.cycles?.length||0),createdAt:Date.now()};
  store.episodes.push(episode);store.episodes=store.episodes.slice(-NOVA554_MAX_EPISODES);store.sinceConsolidation=Number(store.sinceConsolidation||0)+1;store.updatedAt=Date.now();return v554Clone(episode);
}
function v554SetOverlap(a,b){const A=new Set((a||[]).map(v554Canon)),B=new Set((b||[]).map(v554Canon));if(!A.size&&!B.size)return 1;if(!A.size||!B.size)return 0;let common=0;for(const x of A)if(B.has(x))common++;return common/new Set([...A,...B]).size;}
function v554ExperienceCandidates(){
  const episodes=v554EpisodeStore().episodes.filter(e=>e.supported&&e.contributors?.length),groups=new Map();
  for(const e of episodes){const goal=(e.goalTerms||[]).slice(0,2).join("+")||"general",mods=(e.contributors||[]).slice(0,4).map(x=>x.number).sort((a,b)=>a-b).join(",");if(!mods)continue;const key=`experience:${goal}:${mods}`;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(e);}
  const out=[];for(const [signature,items] of groups){const prompts=new Set(items.map(x=>x.promptHash)),concepts=v554Unique(items.flatMap(x=>x.conceptTerms||[]),v554Canon);if(items.length<3||prompts.size<2)continue;const goal=(items[0].goalTerms||[])[0]||"reasoning",modules=v554Unique(items.flatMap(x=>(x.contributors||[]).slice(0,4).map(c=>c.number)),Number);const confidence=Math.min(.93,.55+Math.min(.2,(items.length-3)*.04)+Math.min(.1,prompts.size*.02));out.push({signature,kind:"experience_pattern",status:"PROVISIONAL",generalTerm:`experienced ${v554Readable(goal)} pattern`,description:`Repeated successful cognitive episodes used a similar module route for ${v554Readable(goal)} goals.`,template:{goal,moduleNumbers:modules,conceptTerms:concepts.slice(0,12)},support:{instances:items.length,distinctPrompts:prompts.size,distinctConcepts:concepts.length},confidence:Number(confidence.toFixed(3)),evidence:items.slice(-12).map(e=>({type:"development_episode",id:e.id,promptHash:e.promptHash,contributors:e.contributors.slice(0,6)})),sourceKeys:[...prompts].map(x=>`episode:${x}`),createdAt:Date.now(),updatedAt:Date.now()});}
  return out;
}

function v554EvidenceSourceKeys(item){
  const out=[];
  if(item?.sourceId)out.push(`id:${item.sourceId}`);
  if(item?.sourceName)out.push(`name:${v554Canon(item.sourceName)}`);
  for(const e of item?.evidence||[]){
    if(e?.sourceId)out.push(`id:${e.sourceId}`);
    else if(e?.sourceName)out.push(`name:${v554Canon(e.sourceName)}`);
  }
  return v554Unique(out,String);
}

function v554ModuleById(id){try{return (v502ModuleStore()?.modules||[]).find(m=>m.id===id)||null;}catch(_){return null;}}
function v554ProgramSkeleton(rule){
  const module=rule?.moduleId?v554ModuleById(rule.moduleId):null;
  const program=module?.program||rule?.synthesis?.program||[];
  const ops=(program||[]).map(step=>v554Key(step?.op)).filter(Boolean);
  if(ops.length)return ops.join(">");
  if(rule?.primitive&&rule.primitive!=="unsupported_guidance")return `primitive:${v554Key(rule.primitive)}`;
  return "";
}

function v554RelationshipCandidates(){
  let claims=[];try{claims=v450KnowledgeStore()?.claims||[];}catch(_){claims=[];}
  const groups=new Map();
  for(const claim of claims){
    if(!claim?.predicate||claim?.object==null||!claim?.subject)continue;
    const predicate=v554Key(claim.predicate);if(!predicate)continue;
    const signature=`relation:${predicate}:${v554Key(claim.polarity||"positive")}`;
    if(!groups.has(signature))groups.set(signature,[]);groups.get(signature).push(claim);
  }
  const out=[];
  for(const [signature,items] of groups){
    const subjects=v554Unique(items.map(x=>v554Canon(x.subject)).filter(Boolean));
    const objects=v554Unique(items.map(x=>v554Canon(x.object)).filter(Boolean));
    const sourceKeys=v554Unique(items.flatMap(v554EvidenceSourceKeys));
    if(items.length<NOVA554_MIN_RELATION_INSTANCES||subjects.length<2||objects.length<2)continue;
    // Prefer independent lessons/sources. Four or more distinct instances can
    // still propose a pattern when an early curriculum establishes it in one
    // source, but confidence remains lower until another source supports it.
    if(sourceKeys.length<2&&items.length<4)continue;
    const predicate=items[0].predicate;
    const confidence=Math.min(.94,.55+Math.min(.18,(items.length-3)*.05)+Math.min(.12,(subjects.length-2)*.03)+Math.min(.09,sourceKeys.length*.03));
    out.push({
      signature,kind:"relationship_pattern",status:"PROVISIONAL",
      generalTerm:`${v554Readable(predicate)} relation`,
      description:`A recurring learned relationship uses ${v554Readable(predicate)} between varying subjects and objects.`,
      template:{subject:"$subject",predicate,object:"$object",polarity:items[0].polarity||"positive"},
      support:{instances:items.length,distinctSubjects:subjects.length,distinctObjects:objects.length,distinctSources:sourceKeys.length},
      confidence:Number(confidence.toFixed(3)),
      evidence:items.slice(0,12).map(x=>({type:"claim",id:x.id,subject:x.subject,predicate:x.predicate,object:x.object,sources:v554EvidenceSourceKeys(x)})),
      sourceKeys,createdAt:Date.now(),updatedAt:Date.now()
    });
  }
  return out;
}

function v554CognitiveCandidates(){
  let rules=[];try{rules=v500CognitiveRuleInventory()?.rules||[];}catch(_){rules=[];}
  const groups=new Map();
  for(const rule of rules){
    if(!["executable","module_executable"].includes(rule?.status))continue;
    const form=v554Key(rule?.synthesis?.cognitiveForm||rule?.domain||"");
    const skeleton=v554ProgramSkeleton(rule);if(!form||!skeleton)continue;
    const signature=`cognition:${v554Key(rule.domain||"general")}:${form}:${skeleton}`;
    if(!groups.has(signature))groups.set(signature,[]);groups.get(signature).push(rule);
  }
  const out=[];
  for(const [signature,items] of groups){
    if(items.length<NOVA554_MIN_COGNITIVE_INSTANCES)continue;
    const sourceKeys=v554Unique(items.flatMap(v554EvidenceSourceKeys));
    const scopes=v554Unique(items.map(x=>v554Canon(x.scope)).filter(Boolean));
    if(sourceKeys.length<2&&items.length<4)continue;
    const form=items[0]?.synthesis?.cognitiveForm||items[0]?.domain||"cognitive pattern";
    const skeleton=v554ProgramSkeleton(items[0]);const ops=skeleton.split(">").filter(Boolean);
    const termBase=v554Readable(form)||"cognitive";
    const confidence=Math.min(.95,.58+Math.min(.18,(items.length-3)*.05)+Math.min(.12,sourceKeys.length*.04)+Math.min(.07,scopes.length*.02));
    out.push({
      signature,kind:"cognitive_pattern",status:"PROVISIONAL",
      generalTerm:`${termBase} pattern`,
      description:`Several learned cognitive teachings share the same ${termBase} operation structure.`,
      template:{domain:items[0].domain||"general",cognitiveForm:form,operationSkeleton:ops},
      support:{instances:items.length,distinctScopes:scopes.length,distinctSources:sourceKeys.length},
      confidence:Number(confidence.toFixed(3)),
      evidence:items.slice(0,12).map(x=>({type:"cognitive_rule",id:x.id,directive:x.directive,scope:x.scope,sourceId:x.sourceId||null,sourceName:x.sourceName||null})),
      sourceKeys,createdAt:Date.now(),updatedAt:Date.now()
    });
  }
  return out;
}

function v554MergeAbstraction(existing,candidate){
  existing.kind=candidate.kind;existing.status=existing.status||"PROVISIONAL";
  existing.generalTerm=candidate.generalTerm;existing.description=candidate.description;
  existing.template=v554Clone(candidate.template);existing.support=v554Clone(candidate.support);
  existing.confidence=Math.max(Number(existing.confidence||0),Number(candidate.confidence||0));
  existing.evidence=v554Unique([...(existing.evidence||[]),...(candidate.evidence||[])],x=>`${x.type}|${x.id||JSON.stringify(x)}`).slice(-24);
  existing.sourceKeys=v554Unique([...(existing.sourceKeys||[]),...(candidate.sourceKeys||[])],String);
  existing.active=true;existing.updatedAt=Date.now();return existing;
}

function v554RefreshCognitiveAbstractions(options={}){
  if(window.__TAURI__?.core?.invoke)return {ran:false,retired:true,authority:"rust_v7_pass_8"};
  const store=v554AbstractionStore();
  const candidates=[...v554RelationshipCandidates(),...v554CognitiveCandidates(),...v554ExperienceCandidates()];
  const bySig=new Map((store.abstractions||[]).map(a=>[a.signature,a]));
  for(const old of store.abstractions||[])old.active=false;
  let created=0,updated=0;
  for(const candidate of candidates){
    const existing=bySig.get(candidate.signature);
    if(existing){v554MergeAbstraction(existing,candidate);updated++;}
    else{const made={id:`abstraction_${Date.now()}_${Math.random().toString(36).slice(2,8)}`,...candidate,active:true};store.abstractions.push(made);bySig.set(made.signature,made);created++;}
  }
  store.abstractions=store.abstractions.slice(-300);
  const active=store.abstractions.filter(a=>a.active!==false&&a.status==="PROVISIONAL");
  // V6.2: an abstraction may become a provisional category in the same open
  // category graph used by built-in and explicitly taught categories. It is
  // still non-authoritative and non-executable until later learning supports it.
  if(typeof v620RegisterCategory==="function"){
    for(const abstraction of active){
      const category=v620RegisterCategory(abstraction.generalTerm,{origin:"abstracted",status:"PROVISIONAL",traits:[{kind:abstraction.kind,template:v554Clone(abstraction.template),confidence:abstraction.confidence}],evidence:v554Clone(abstraction.evidence||[])});
      const parent=abstraction.template?.cognitiveForm||abstraction.template?.relation||abstraction.template?.goal||abstraction.kind;
      if(category&&parent&&typeof v620RelateCategories==="function")v620RelateCategories(category.name,"provisional_kind_of",parent,{origin:"abstracted",evidence:{abstractionId:abstraction.id,confidence:abstraction.confidence}});
    }
  }
  const report={schema:NOVA554_ABSTRACTION_SCHEMA,version:NOVA554_VERSION,reason:options.reason||"refresh",candidates:candidates.length,created,updated,active:active.length,relationshipPatterns:active.filter(a=>a.kind==="relationship_pattern").length,cognitivePatterns:active.filter(a=>a.kind==="cognitive_pattern").length,experiencePatterns:active.filter(a=>a.kind==="experience_pattern").length,ranAt:Date.now()};
  store.lastPass=v554Clone(report);store.history.push(v554Clone(report));store.history=store.history.slice(-NOVA554_HISTORY_LIMIT);store.updatedAt=Date.now();
  return report;
}

function v554ProvisionalAbstractions(){return (v554AbstractionStore().abstractions||[]).filter(a=>a.status==="PROVISIONAL"&&a.active!==false);}
function v554AbstractionSummary(){const s=v554AbstractionStore(),active=v554ProvisionalAbstractions();return {total:(s.abstractions||[]).length,active:active.length,relationshipPatterns:active.filter(a=>a.kind==="relationship_pattern").length,cognitivePatterns:active.filter(a=>a.kind==="cognitive_pattern").length,experiencePatterns:active.filter(a=>a.kind==="experience_pattern").length,lastPassAt:s.lastPass?.ranAt||null};}



function v554MergeExperienceRoutes(){
  if(window.__TAURI__?.core?.invoke)return {merged:0,pruned:0,retired:true,authority:"rust_v7_pass_8"};
  let store=null;try{store=v502ModuleStore();}catch(_){return {merged:0,pruned:0};}const experience=typeof v622EnsureExperienceIndex==="function"?v622EnsureExperienceIndex(store):store?.experienceIndex;if(!experience?.routes)return {merged:0,pruned:0};const routes=experience.routes.slice(),removed=new Set();let merged=0;
  for(let i=0;i<routes.length;i++){const a=routes[i];if(removed.has(a.id))continue;for(let j=i+1;j<routes.length;j++){const b=routes[j];if(removed.has(b.id))continue;const concept=v554SetOverlap(a.conceptTerms||a.topicTerms,b.conceptTerms||b.topicTerms),goal=v554SetOverlap(a.goalTerms,b.goalTerms);if(concept<.6||(goal<.5&&((a.goalTerms||[]).length||(b.goalTerms||[]).length)))continue;const keep=Number(a.successes||0)>=Number(b.successes||0)?a:b,drop=keep===a?b:a;keep.conceptTerms=v554Unique([...(keep.conceptTerms||[]),...(drop.conceptTerms||[])],v554Canon).slice(0,24);keep.relationTerms=v554Unique([...(keep.relationTerms||[]),...(drop.relationTerms||[])],v554Canon).slice(0,18);keep.fallbackTerms=v554Unique([...(keep.fallbackTerms||[]),...(drop.fallbackTerms||[])],v554Canon).slice(0,18);keep.goalTerms=v554Unique([...(keep.goalTerms||[]),...(drop.goalTerms||[])],v554Canon).slice(0,14);keep.moduleNumbers=v554Unique([...(keep.moduleNumbers||[]),...(drop.moduleNumbers||[])],Number).slice(0,48);keep.moduleStrengths ||= {};for(const [n,v] of Object.entries(drop.moduleStrengths||{}))keep.moduleStrengths[n]=Math.max(Number(keep.moduleStrengths[n]||0),Number(v||0));keep.successes=Number(keep.successes||0)+Number(drop.successes||0);keep.failures=Number(keep.failures||0)+Number(drop.failures||0);keep.uses=Number(keep.uses||0)+Number(drop.uses||0);keep.confidence=Math.max(Number(keep.confidence||.55),Number(drop.confidence||.55));keep.updatedAt=Date.now();removed.add(drop.id);merged++;}
  }
  experience.routes=routes.filter(route=>!removed.has(route.id));const before=experience.routes.length;experience.routes=experience.routes.filter(route=>!(Number(route.failures||0)>=5&&Number(route.uses||0)>=5&&Number(route.confidence||.55)<.15));const pruned=before-experience.routes.length;if(merged||pruned){experience.revision=Number(experience.revision||0)+1;experience.updatedAt=Date.now();store.updatedAt=Date.now();}return {merged,pruned};
}
function v554MaybeConsolidateDevelopment(options={}){
  if(window.__TAURI__?.core?.invoke)return {ran:false,retired:true,authority:"rust_v7_pass_8"};
  const episodes=v554EpisodeStore(),force=Boolean(options.force);if(!force&&Number(episodes.sinceConsolidation||0)<NOVA554_CONSOLIDATE_EVERY)return {ran:false,pending:NOVA554_CONSOLIDATE_EVERY-Number(episodes.sinceConsolidation||0)};
  const report={ran:true,version:NOVA554_VERSION,reason:options.reason||"periodic_developmental_consolidation",startedAt:Date.now(),episodesConsidered:(episodes.episodes||[]).length};
  try{report.routes=v554MergeExperienceRoutes();}catch(error){report.routes={error:String(error?.message||error)};}
  try{report.abstractions=v554RefreshCognitiveAbstractions({reason:"developmental_consolidation"});}catch(error){report.abstractions={error:String(error?.message||error)};}
  try{const current=typeof v520CapabilitySignature==="function"?v520CapabilitySignature():null,prior=episodes.lastCapabilitySignature||null;if(current&&current!==prior){if(window.__TAURI__?.core?.invoke)report.reconsideration={pendingRustSync:true,reason:"rust_v7_pass_8_synthesis_authority"};else if(typeof v520GeneralSynthesisPass==="function")report.reconsideration=v520GeneralSynthesisPass({reason:"browser_developmental_capability_change",force:false});}episodes.lastCapabilitySignature=current;}catch(error){report.reconsideration={error:String(error?.message||error)};}
  episodes.sinceConsolidation=0;episodes.lastConsolidatedAt=Date.now();report.completedAt=Date.now();episodes.lastConsolidation=v554Clone(report);episodes.updatedAt=Date.now();try{saveState();}catch(_){ }return report;
}

function v554PostLoadCognitiveAbstractionMaintenance(options={}){
  if(window.__TAURI__?.core?.invoke)return {firstRun:false,retired:true,authority:"rust_v7_pass_8"};
  const memory=v43Memory(),prior=memory.cognitiveAbstractionMigration;
  const firstRun=Boolean(options.force||prior?.version!==NOVA554_VERSION);let report=null;
  if(firstRun||options.refresh){try{report=v554RefreshCognitiveAbstractions({reason:firstRun?"v6.2_current_abstraction_refresh":"v6.2_post_load_refresh"});}catch(error){report={error:String(error?.message||error)};}}
  if(firstRun)memory.cognitiveAbstractionMigration={schema:NOVA554_ABSTRACTION_MIGRATION_SCHEMA,version:NOVA554_VERSION,ranAt:Date.now(),report:v554Clone(report)};
  try{saveState();}catch(_){ }
  return {firstRun,report,migration:v554Clone(memory.cognitiveAbstractionMigration||null)};
}

try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA554_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const entries=[
      {id:"provisional_cognitive_abstraction",verb:"abstract",object:"recurring structures across learned relationships and cognitive programs",detail:"Repeated experience can form provisional categories with explicit support and provenance; they are not developer-registered category names."},
      {id:"evidence_bounded_concept_formation",verb:"propose",object:"new internal categories from repeated learned structure and repeated successful experience",detail:"A noticed pattern remains provisional rather than silently becoming a fact or executable ability."},
      {id:"developmental_consolidation",verb:"consolidate",object:"recent cognitive episodes, successful routes, abstractions, and newly available capabilities",detail:"Periodic consolidation compresses repeated successful experience, merges redundant attention routes, and reconsiders Stored Only cognition when the current capability vocabulary has actually changed."}
    ];
    for(const entry of entries){const old=V41_SELF_MODEL.capabilities.find(x=>x.id===entry.id);if(old)Object.assign(old,entry);else V41_SELF_MODEL.capabilities.push(entry);}
  }
}catch(_){ }

try{v554AbstractionStore();}catch(_){ }
console.info("Nova V6.2.16 developmental abstraction and consolidation loaded.");
