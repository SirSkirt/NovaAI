"use strict";

/* =========================================================
   Nova V7 Pass 9 — UI-TRIGGERED RUST REINGESTION / BROWSER MAINTENANCE

   Rebuilds retained teaching sources through the current authoritative mind.
   Native Tauri re-ingests raw sources through the Rust teaching authority established in Pass 6 and retained by Pass 7; the older JavaScript
   rebuild exists only for browser migration/development. No prior compiled
   module is restored as a compatibility fallback.
   ========================================================= */

const NOVA620_MAINTENANCE_VERSION="7.0.0-alpha.9.3.1";
const NOVA620_RELEARN_SCHEMA=2;

function v620MaintClone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v620MaintCanon(v){try{return canonical(String(v??""));}catch(_){return String(v??"").trim().toLowerCase();}}
function v620InstructionSources(){let memory=null;try{memory=v43Memory();}catch(_){return [];}return (memory.sources||[]).filter(source=>source?.id&&source?.text&&!source.sourceTextTruncated&&(typeof v534InstructionSource==="function"?v534InstructionSource(source):true));}

function v620SourceCognitionSnapshot(sources){
  const ids=new Set((sources||[]).map(s=>s?.id).filter(Boolean)),names=new Set((sources||[]).map(s=>v620MaintCanon(s?.name)).filter(Boolean));let rules=[];try{rules=v500CognitiveRuleInventory()?.rules||[];}catch(_){ }
  const rows=rules.filter(r=>r?.sourceType==="learned"&&((r.sourceId&&ids.has(r.sourceId))||(!r.sourceId&&names.has(v620MaintCanon(r.sourceName))))).map(r=>({sourceId:r.sourceId||null,sourceName:r.sourceName||null,directive:String(r.directive||""),status:r.status||null,executable:r.status==="module_executable",moduleId:r.moduleId||null,key:`${r.sourceId||v620MaintCanon(r.sourceName)}::${v620MaintCanon(r.directive||"")}`}));
  const byKey=new Map();for(const row of rows){const old=byKey.get(row.key);if(!old||(!old.executable&&row.executable))byKey.set(row.key,row);}const unique=[...byKey.values()];return {total:unique.length,executable:unique.filter(x=>x.executable).length,storedOnly:unique.filter(x=>!x.executable).length,rules:unique};
}
function v620CompareCognitionSnapshots(before,after){const a=new Map((before?.rules||[]).map(x=>[x.key,x])),b=new Map((after?.rules||[]).map(x=>[x.key,x])),executableRegressions=[],newlyExecutable=[];for(const [key,row] of a){const next=b.get(key);if(row.executable&&(!next||!next.executable))executableRegressions.push({sourceName:row.sourceName,directive:row.directive,beforeStatus:row.status,afterStatus:next?.status||"missing"});}for(const [key,row] of b){const prev=a.get(key);if(row.executable&&(!prev||!prev.executable))newlyExecutable.push({sourceName:row.sourceName,directive:row.directive,beforeStatus:prev?.status||"missing",afterStatus:row.status});}return {before:{total:before?.total||0,executable:before?.executable||0,storedOnly:before?.storedOnly||0},after:{total:after?.total||0,executable:after?.executable||0,storedOnly:after?.storedOnly||0},executableRegressions,newlyExecutable,regressionCount:executableRegressions.length,newlyExecutableCount:newlyExecutable.length};}

function v620StripSourceBackedSemanticProducts(sourceIds){
  if(typeof v450KnowledgeStore!=="function")return {skipped:true,reason:"semantic_store_unavailable"};const ids=new Set(sourceIds||[]),store=v450KnowledgeStore(),report={claimsRemoved:0,guidanceRemoved:0,examplesRemoved:0,proceduresRemoved:0,conceptsRemoved:0,evidenceRemoved:0};
  const strip=name=>{const keep=[];for(const item of store[name]||[]){const ev=Array.isArray(item.evidence)?item.evidence:[],targeted=ev.filter(e=>ids.has(e?.sourceId));if(!targeted.length){keep.push(item);continue;}const remaining=ev.filter(e=>!ids.has(e?.sourceId));report.evidenceRemoved+=targeted.length;if(remaining.length){item.evidence=remaining;keep.push(item);}else report[`${name}Removed`]++;}store[name]=keep;};
  for(const name of ["claims","guidance","examples","procedures"])strip(name);
  const referenced=new Set();for(const c of store.claims||[]){if(c.subjectId)referenced.add(c.subjectId);if(c.objectConceptId)referenced.add(c.objectConceptId);}for(const p of store.procedures||[])if(p.conceptId)referenced.add(p.conceptId);
  const concepts=[];for(const concept of store.concepts||[]){const ev=Array.isArray(concept.evidence)?concept.evidence:[],targeted=ev.filter(e=>ids.has(e?.sourceId));if(!targeted.length){concepts.push(concept);continue;}const remaining=ev.filter(e=>!ids.has(e?.sourceId));report.evidenceRemoved+=targeted.length;if(remaining.length||referenced.has(concept.id)){concept.evidence=remaining;concepts.push(concept);}else report.conceptsRemoved++;}store.concepts=concepts;store.learnedSourceIds=(store.learnedSourceIds||[]).filter(id=>!ids.has(id));return report;
}

async function v620RelearnAllKnowledge(){
  const memory=v43Memory(),sources=v620InstructionSources(),report={schema:NOVA620_RELEARN_SCHEMA,version:NOVA620_MAINTENANCE_VERSION,startedAt:Date.now(),sourceCount:sources.length,recompiledSources:0,reindexedSemanticSources:0,errors:[],warnings:[]},before=v620SourceCognitionSnapshot(sources);
  if(window.__TAURI__?.core?.invoke){
    if(typeof nova7RustMindIngestSource!=="function"){report.status="RUST_MIND_REQUIRED";report.errors.push("Rust Pass 9 teaching ingestion is unavailable; JavaScript relearn is disabled on native Nova.");report.completedAt=Date.now();memory.lastManualRelearn=v620MaintClone(report);try{saveState();}catch(_){ }return report;}
    if(!sources.length){report.status="NO_FULL_LESSON_SOURCES";report.completedAt=Date.now();memory.lastManualRelearn=v620MaintClone(report);try{saveState();}catch(_){ }return report;}
    let lastReceipt=null;
    for(const source of sources){
      try{lastReceipt=await nova7RustMindIngestSource(source,{kindHint:"lesson",sourceType:"learned"});report.reindexedSemanticSources++;report.recompiledSources++;}
      catch(error){report.errors.push(`${source.name||source.id}:rust_ingestion:${error?.message||error}`);}
    }
    report.synthesis=v620MaintClone(lastReceipt?.synthesisReport||null);
    report.rustAuthority="rust_v7_pass_9";
    report.relearnMode="rust_authoritative_reingestion";
    report.status=report.errors.length?"COMPLETED_WITH_WARNINGS":"COMPLETED";
    report.completedAt=Date.now();memory.lastManualRelearn=v620MaintClone(report);try{saveState();}catch(_){ }return report;
  }
  if(!sources.length){report.status="NO_FULL_LESSON_SOURCES";report.completedAt=Date.now();memory.lastManualRelearn=v620MaintClone(report);try{saveState();}catch(_){ }return report;}
  const ids=sources.map(s=>s.id);try{report.semanticCleanup=v620MaintClone(v620StripSourceBackedSemanticProducts(ids));}catch(error){report.errors.push(`semantic_cleanup:${error?.message||error}`);}
  for(const source of sources){
    try{if(typeof v500RemoveSourceCognition==="function")v500RemoveSourceCognition(source);}catch(error){report.errors.push(`${source.name||source.id}:cognition_cleanup:${error?.message||error}`);}
    try{if(typeof v450LearnDocument==="function"){let semanticSource=source;try{if(typeof v534ParseDocumentStructure==="function"&&typeof v534FilterStructure==="function"&&typeof v534StructureText==="function"){const identity=typeof v534SourceIdentity==="function"?v534SourceIdentity(source.name||"document","learned"):null,structure=v534ParseDocumentStructure(source.text,{sourceName:source.name||"document",sourceType:"learned",identity}),lessonStructure=v534FilterStructure(structure,"lesson"),lessonText=v534StructureText(lessonStructure);if(lessonText)semanticSource={...source,text:lessonText};}}catch(_){ }v450LearnDocument(semanticSource,{reindex:true,relearn:true,currentCore:true,skipCognitiveCompile:true});report.reindexedSemanticSources++;}}catch(error){report.errors.push(`${source.name||source.id}:semantic:${error?.message||error}`);}
    try{if(typeof v534CompileDocumentCognition==="function")v534CompileDocumentCognition(source,{synthesize:false});else if(typeof v500CompileCognitionRulesFromText==="function")v500CompileCognitionRulesFromText(source.text,{sourceType:"learned",sourceName:source.name||"document",sourceId:source.id||null,synthesize:false});report.recompiledSources++;}catch(error){report.errors.push(`${source.name||source.id}:cognition:${error?.message||error}`);}
  }
  try{
    if(window.__TAURI__?.core?.invoke&&typeof nova7RustMindSyncLearning==="function"){const rustLearning=await nova7RustMindSyncLearning("pass6_manual_relearn");report.synthesis=v620MaintClone(rustLearning?.report||null);}
    else report.synthesis=v620MaintClone(typeof v520GeneralSynthesisPass==="function"?v520GeneralSynthesisPass({reason:"browser_manual_relearn",force:true}):null);
  }catch(error){report.errors.push(`synthesis:${error?.message||error}`);}
  try{report.abstractions=v620MaintClone(typeof v554RefreshCognitiveAbstractions==="function"?v554RefreshCognitiveAbstractions({reason:"v6.2_manual_relearn",force:true}):null);}catch(error){report.errors.push(`abstraction:${error?.message||error}`);}
  const after=v620SourceCognitionSnapshot(sources);report.cognitionComparison=v620CompareCognitionSnapshots(before,after);report.priorExecutablePreservation={attempted:report.cognitionComparison.regressionCount,restored:0,reason:"CURRENT_MIND_REBUILD_NO_LEGACY_EXECUTABLE_RESTORE"};
  if(report.cognitionComparison.regressionCount)report.warnings.push(`${report.cognitionComparison.regressionCount} previously executable learned directive(s) could not be rebuilt by the current V6.2 mind. They remain visible instead of being restored through an older implementation.`);
  report.status=(report.errors.length||report.warnings.length)?"COMPLETED_WITH_WARNINGS":"COMPLETED";report.completedAt=Date.now();memory.lastManualRelearn=v620MaintClone(report);try{saveState();}catch(_){ }return report;
}

try{if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){V41_SELF_MODEL.identity.version=NOVA620_MAINTENANCE_VERSION;V41_SELF_MODEL.capabilities ||= [];const entry={id:"manual_source_relearn",verb:"rebuild",object:"derived semantic knowledge and learned cognition from retained teaching sources through the current mind",detail:"Relearn never restores an older compiled executable as a fallback; missing current capability stays visible."},old=V41_SELF_MODEL.capabilities.find(c=>c.id===entry.id);if(old)Object.assign(old,entry);else V41_SELF_MODEL.capabilities.push(entry);}}catch(_){ }

console.info("Nova V7 Pass 9 maintenance UI loaded — native relearn only requests Rust re-ingestion; JavaScript recompilation is browser-only development code.");
