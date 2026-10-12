"use strict";

/* =========================================================
   Nova V6.2.16 — GENERIC LEARNED COGNITION STORE

   This file owns learned cognitive guidance as data. It does not patch the
   semantic learner, executive, response generator, or module interpreter.

   A teaching can be stored before Nova has enough reusable cognition to run
   it. The current synthesizer later attempts to compose an executable module
   from generic modifiers and already-learned programs.

   Important boundary:
   - categories/domains/forms are open labels, not an executable allow-list;
   - no English construction ("suppose", "who", etc.) is hardcoded here;
   - evidence/examples are not promoted into cognition merely because they
     contain imperative-looking words.
   ========================================================= */

const NOVA500_COGNITIVE_SCHEMA_VERSION=3;
const NOVA620_COGNITION_VERSION="6.2.16";

function v500Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v500Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v500Canon(v){try{return canonical(String(v??""));}catch(_){return v500Clean(v).toLowerCase();}}
function v500Unique(xs,key=x=>typeof x==="object"?JSON.stringify(x):v500Canon(x)){const out=[],seen=new Set();for(const x of xs||[]){if(x==null)continue;let k="";try{k=key(x);}catch(_){k=String(x);}if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}
function v500RuleSignature(rule){return v500Canon([rule?.sourceId||rule?.sourceName||"conversation",rule?.scope||"general",rule?.domain||"general",rule?.cognitiveForm||"",rule?.polarity||"positive",v500Clean(rule?.directive||"")].join("|"));}

function v500CognitiveStore(){
  const memory=typeof v43Memory==="function"?v43Memory():null;
  if(!memory)return {version:NOVA500_COGNITIVE_SCHEMA_VERSION,appVersion:NOVA620_COGNITION_VERSION,rules:[],recentRejected:[]};
  const prior=memory.cognitiveLearning||{};
  if(prior.version!==NOVA500_COGNITIVE_SCHEMA_VERSION){
    // Carry only user/source learned guidance. Old built-in rule inventories were
    // products of the previous closed cognition design and are intentionally not
    // migrated as executable authority into the generic-module mind.
    const learned=(prior.rules||[]).filter(r=>r?.sourceType==="learned").map(r=>({
      ...v500Clone(r),status:["module_executable"].includes(r?.status)?"module_executable":"stored_only",
      appVersion:NOVA620_COGNITION_VERSION,updatedAt:Date.now()
    }));
    memory.cognitiveLearning={version:NOVA500_COGNITIVE_SCHEMA_VERSION,appVersion:NOVA620_COGNITION_VERSION,rules:learned,lastCompilation:null,lastApplication:null,recentRejected:[],createdAt:prior.createdAt||Date.now(),updatedAt:Date.now(),migratedFromVersion:prior.version??null};
  }
  const store=memory.cognitiveLearning;store.appVersion=NOVA620_COGNITION_VERSION;store.rules ||= [];store.recentRejected ||= [];
  // A missing module can never stay "executable" merely because an older build
  // once marked it so. Current executability belongs to the current module store.
  let moduleIds=null;try{moduleIds=new Set((typeof v502ModuleStore==="function"?v502ModuleStore().modules:[]).map(m=>m.id));}catch(_){moduleIds=null;}
  for(const rule of store.rules){
    rule.status=(rule.moduleId&&moduleIds?.has(rule.moduleId))?"module_executable":"stored_only";
    rule.appVersion=NOVA620_COGNITION_VERSION;
  }
  store.updatedAt=Date.now();return store;
}

function v500RulePriority(rule){return (rule?.strength==="required"?20:10)+(Number(rule?.updatedAt||0)/1e15);}
function v500ActiveRule(domain,primitive){return v500CognitiveStore().rules.filter(r=>r.status==="module_executable"&&(!domain||v500Canon(r.domain)===v500Canon(domain))&&(!primitive||v500Canon(r.primitive)===v500Canon(primitive))).sort((a,b)=>v500RulePriority(b)-v500RulePriority(a))[0]||null;}
function v500RuleEnabled(domain,primitive){return Boolean(v500ActiveRule(domain,primitive));}
function v500Executable(domain,primitive){return Boolean(v500ActiveRule(domain,primitive));}

function v500RecordRuleApplication(rule,detail={}){if(!rule)return null;const item={ruleId:rule.id,domain:rule.domain||"general",primitive:rule.primitive||"learned_guidance",sourceType:rule.sourceType,sourceName:rule.sourceName||null,detail:v500Clone(detail),appliedAt:Date.now()};const store=v500CognitiveStore();store.lastApplication=item;return item;}

function v500StoreCognitiveRule(rule={},meta={}){
  const store=v500CognitiveStore(),directive=v500Clean(rule.directive||meta.text||"");if(!directive)return null;
  const prepared={
    id:rule.id||`cog_${Date.now()}_${Math.random().toString(36).slice(2,8)}`,
    appVersion:NOVA620_COGNITION_VERSION,
    domain:v500Clean(rule.domain||meta.domain||"general")||"general",
    primitive:v500Clean(rule.primitive||meta.primitive||"learned_guidance")||"learned_guidance",
    directive,
    scope:v500Clean(rule.scope||meta.scope||"general")||"general",
    evidenceRole:v500Clean(rule.evidenceRole||meta.evidenceRole||"instruction")||"instruction",
    cognitiveForm:rule.cognitiveForm||meta.cognitiveForm||null,
    learnedCategory:rule.learnedCategory||rule.category||meta.learnedCategory||meta.category||null,
    categories:v500Unique([...(rule.categories||[]),...(meta.categories||[])].filter(Boolean)),
    parentCategory:rule.parentCategory||meta.parentCategory||null,
    strength:rule.strength||meta.strength||"recommended",
    polarity:rule.polarity||meta.polarity||"positive",
    status:"stored_only",
    moduleId:null,
    sourceType:meta.sourceType||rule.sourceType||"learned",
    sourceName:meta.sourceName||rule.sourceName||"conversation",
    sourceDisplayName:meta.sourceDisplayName||rule.sourceDisplayName||(typeof v534SourceDisplayName==="function"?v534SourceDisplayName(meta.sourceName||rule.sourceName||"conversation",meta.sourceType||rule.sourceType||"learned"):null),
    sourceKind:meta.sourceKind||rule.sourceKind||null,
    sourceLessonNumber:meta.sourceLessonNumber??rule.sourceLessonNumber??null,
    sourceId:meta.sourceId||rule.sourceId||null,
    evidence:v500Unique([...(rule.evidence||[]),meta.text||directive].filter(Boolean)),
    parameters:v500Clone(rule.parameters||meta.parameters||{}),
    learnedAt:rule.learnedAt||Date.now(),updatedAt:Date.now()
  };
  prepared.signature=v500RuleSignature(prepared);
  const existing=store.rules.find(x=>x.signature===prepared.signature&&x.sourceType===prepared.sourceType);
  if(existing){existing.evidence=v500Unique([...(existing.evidence||[]),...(prepared.evidence||[])]);existing.categories=v500Unique([...(existing.categories||[]),...(prepared.categories||[])]);for(const key of ["cognitiveForm","learnedCategory","parentCategory"])if(prepared[key]&&!existing[key])existing[key]=prepared[key];existing.updatedAt=Date.now();return existing;}
  store.rules.push(prepared);store.rules=store.rules.slice(-1500);store.updatedAt=Date.now();return prepared;
}

function v500LessonSentences(text){return String(text||"").replace(/\r/g,"\n").split(/(?:\n+|(?<=[.!?])\s+)/).map(s=>v500Clean(s.replace(/^[-*•]\s*/,""))).filter(s=>s.length>=3);}

const NOVA620_DIRECT_COGNITIVE_ROLES=new Set(["guidance","rule","principle","behavior","warning","constraint","validation_rule","validation rule","procedure_step","procedure step","instruction","correction"]);
const NOVA620_EVIDENCE_ONLY_ROLES=new Set(["example","examples","practice","question","answer","negative_example","negative example","explanation","evidence","metadata","subject","topic","purpose","end"]);

function v501NormalizeRole(role){return v500Canon(role||"body").replace(/\s+/g,"_");}
function v501LessonUnits(text,meta={}){
  // Structured-document ownership, when available, is authoritative because it
  // distinguishes a lesson's instruction from examples about that instruction.
  try{if(typeof v534StructuredLessonUnits==="function"){const units=v534StructuredLessonUnits(text,meta);if(Array.isArray(units)&&units.length)return units;}}catch(_){ }
  const lines=String(text||"").replace(/\r/g,"\n").split("\n"),out=[];let role=v501NormalizeRole(meta.evidenceRole||"body"),scope=v500Clean(meta.scope||"general")||"general";
  const labels=new Set(["guidance","rule","principle","behavior","warning","constraint","procedure","step","definition","relationship","example","practice","question","answer","reason","topic","subject","purpose","correction","category","parent category"]);
  for(const rawLine of lines){
    const line=String(rawLine||"").trim();if(!line)continue;
    const m=line.match(/^([A-Za-z][A-Za-z _-]{0,40})\s*:\s*(.*)$/);if(m&&labels.has(v500Canon(m[1]))){const label=v500Canon(m[1]);role=label==="step"?"procedure_step":label.replace(/\s+/g,"_");if(["topic","subject"].includes(label)&&m[2])scope=v500Clean(m[2])||scope;if(m[2]&&!NOVA620_EVIDENCE_ONLY_ROLES.has(role))out.push({text:v500Clean(m[2]),scope,role,label});continue;}
    for(const sentence of v500LessonSentences(line))out.push({text:sentence,scope,role});
  }
  return out;
}

function v501RejectCognitiveSentence(sentence,meta={}){
  const raw=v500Clean(sentence),t=v500Canon(raw),role=v501NormalizeRole(meta.role||meta.evidenceRole||"body");
  if(!raw)return "empty";
  if(NOVA620_EVIDENCE_ONLY_ROLES.has(role))return `evidence_role:${role}`;
  if(/^(?:subject|topic|purpose|end|example|practice|question|answer|reason)\s*:/.test(t))return "lesson_metadata_or_evidence";
  if(/^this lesson\s+(?:teaches|covers|introduces|is about)\b/.test(t))return "lesson_description";
  return null;
}

function v500LooksNormative(text){const t=v500Canon(text);return /\b(?:should|must|never|avoid|prefer|preserve|keep|require|required|do not|don't|must not|should not|when|whenever|before|after|if)\b/.test(t)||/\b(?:use|store|retrieve|compare|select|record|resolve|maintain|change|update|create|group|relate)\b/.test(t);}
function v500RuleCandidates(sentence,meta={}){
  const raw=v500Clean(sentence),role=v501NormalizeRole(meta.role||meta.evidenceRole||"body");if(!raw||v501RejectCognitiveSentence(raw,{...meta,role}))return [];
  /* Structured definitions and relationships belong to semantic memory first.
     They are evidence that whole-lesson synthesis may use, but they should not
     become standalone executable rules merely because their wording happens to
     contain a verb such as “change”, “use”, or “relate”. Unstructured teaching
     can still create cognition from normative body text. */
  const direct=NOVA620_DIRECT_COGNITIVE_ROLES.has(role)||(role==="body"&&v500LooksNormative(raw));if(!direct)return [];
  const negative=/\b(?:do not|don't|never|avoid|must not|should not)\b/.test(v500Canon(raw));
  return [{domain:meta.domain||"general",primitive:"learned_guidance",directive:raw,scope:meta.scope||"general",evidenceRole:role,strength:/\b(?:must|always|required|never)\b/.test(v500Canon(raw))?"required":"recommended",polarity:negative?"negative":"positive",cognitiveForm:meta.cognitiveForm||null,learnedCategory:meta.learnedCategory||meta.category||null,parentCategory:meta.parentCategory||null}];
}

function v500CompileCognitionRulesFromText(text,meta={}){
  if(window.__TAURI__?.core?.invoke)throw new Error("v500CompileCognitionRulesFromText is retired on native Nova: Rust Pass 6 owns teaching-to-cognition translation.");
  const store=v500CognitiveStore(),before=new Set(store.rules.map(r=>r.id)),rejected=[],touched=[];
  for(const unit of v501LessonUnits(text,meta)){
    const reason=v501RejectCognitiveSentence(unit.text,{...meta,scope:unit.scope,role:unit.role});if(reason){rejected.push({text:unit.text,role:unit.role,reason});continue;}
    const candidates=v500RuleCandidates(unit.text,{...meta,scope:unit.scope,role:unit.role,evidenceRole:unit.role,domain:unit.domain,cognitiveForm:unit.cognitiveForm,learnedCategory:unit.learnedCategory,category:unit.category,parentCategory:unit.parentCategory});
    for(const candidate of candidates){const rule=v500StoreCognitiveRule(candidate,{...meta,text:unit.text,scope:unit.scope,evidenceRole:unit.role});if(rule)touched.push(rule.id);}
  }
  let synthesis=null;try{
    if(meta.synthesize!==false&&window.__TAURI__?.core?.invoke)synthesis={deferred:true,authority:"rust_v7_pass_8",reason:"native executability belongs to Rust learning sync"};
    else if(meta.synthesize!==false&&typeof v520GeneralSynthesisPass==="function")synthesis=v520GeneralSynthesisPass({reason:`browser_learning:${meta.sourceName||"conversation"}`,force:true});
  }catch(error){synthesis={error:String(error?.message||error)};}
  const sourceRules=store.rules.filter(r=>touched.includes(r.id)||(!before.has(r.id)&&r.sourceName===(meta.sourceName||"conversation")));
  const executable=sourceRules.filter(r=>r.status==="module_executable"),storedOnly=sourceRules.filter(r=>r.status!=="module_executable");
  const report={version:NOVA620_COGNITION_VERSION,sourceName:meta.sourceName||"conversation",sourceId:meta.sourceId||null,sourceType:meta.sourceType||"learned",rulesAdded:sourceRules.length,executableRules:executable.length,storedOnlyRules:storedOnly.length,rejectedFragments:rejected.length,executable:executable.map(r=>({id:r.id,domain:r.domain,scope:r.scope,status:r.status,moduleId:r.moduleId||null,evidenceRole:r.evidenceRole||null})),storedOnly:storedOnly.map(r=>({id:r.id,domain:r.domain,scope:r.scope,status:r.status,synthesis:v500Clone(r.synthesis||null),evidenceRole:r.evidenceRole||null})),rejected,synthesis,compiledAt:Date.now()};
  store.lastCompilation=report;store.recentRejected=rejected.slice(-80);store.updatedAt=Date.now();return report;
}

function v500RemoveSourceCognition(source={}){const store=v500CognitiveStore(),id=source.id||source.sourceId||null,name=v500Canon(source.name||source.sourceName||"");const matchesSource=item=>item?.sourceType==="learned"&&((id&&item.sourceId===id)||(!id&&name&&v500Canon(item.sourceName)===name));const removed=store.rules.filter(matchesSource),ids=new Set(removed.map(r=>r.id)),moduleIds=new Set(removed.map(r=>r.moduleId).filter(Boolean));store.rules=store.rules.filter(r=>!ids.has(r.id));let modulesRemoved=0;try{if(typeof v502ModuleStore==="function"){const ms=v502ModuleStore(),before=ms.modules.length;ms.modules=ms.modules.filter(m=>!moduleIds.has(m.id)&&!matchesSource(m));modulesRemoved=before-ms.modules.length;if(modulesRemoved){ms.referenceIndex=null;if(typeof v622RebuildReferenceIndex==="function")v622RebuildReferenceIndex(ms);}ms.updatedAt=Date.now();}}catch(_){ }store.updatedAt=Date.now();return {rulesRemoved:removed.length,modulesRemoved};}

function v500CognitiveRuleInventory(){const store=v500CognitiveStore(),rules=store.rules.slice().sort((a,b)=>(b.updatedAt||0)-(a.updatedAt||0));return {version:NOVA620_COGNITION_VERSION,schema:NOVA500_COGNITIVE_SCHEMA_VERSION,total:rules.length,executable:rules.filter(r=>r.status==="module_executable").length,storedOnly:rules.filter(r=>r.status!=="module_executable").length,rules,lastCompilation:v500Clone(store.lastCompilation),lastApplication:v500Clone(store.lastApplication),recentRejected:v500Clone(store.recentRejected||[])};}

console.info("Nova V6.2.16 generic learned cognition store loaded.");
