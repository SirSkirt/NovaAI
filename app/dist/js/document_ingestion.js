"use strict";

/* =========================================================
   Nova V6.2.16 — STRUCTURED LESSON + TEST DOCUMENT INGESTION

   One current intake path distinguishes teaching from evaluation before either
   reaches semantic memory or cognitive synthesis. Test answer keys are kept out
   of the cognition being tested. No legacy responder or parallel test brain is
   used: test questions run through the same current V6.2 mind.
   ========================================================= */

const NOVA534_VERSION="6.2.16";
const NOVA534_INGESTION_SCHEMA=3;

function v534Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v534Canonical(v){try{return canonical(String(v??""));}catch(_){return v534Clean(v).toLowerCase();}}
function v534Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v534NormalizeLabel(raw){return v534Canonical(String(raw||"").replace(/_/g," ")).replace(/\s+/g," ");}
function v534Stem(name){return String(name||"document").replace(/^.*[\\/]/,"").replace(/\.[^.]+$/,"");}

function v534SourceIdentity(name,sourceType="learned"){
  const raw=String(name||"document"),stem=v534Stem(raw),spaced=stem.replace(/[_-]+/g," ").replace(/\s+/g," ").trim();
  if(v534Canonical(sourceType)==="conversation"||v534Canonical(raw)==="conversation")return {kind:"conversation",lessonNumber:null,displayName:"Conversation",sourceName:raw};
  let m=spaced.match(/\btest\s+lesson\s*(\d+)\b/i);if(m)return {kind:"test",lessonNumber:Number(m[1]),displayName:`Test Lesson ${Number(m[1])}`,sourceName:raw};
  m=spaced.match(/\blesson\s*(\d+)\b/i);if(m){const before=spaced.slice(0,m.index||0);const kind=/\btest\b/i.test(before)?"test":"lesson";return {kind,lessonNumber:Number(m[1]),displayName:`${kind==="test"?"Test Lesson":"Lesson"} ${Number(m[1])}`,sourceName:raw};}
  if(/^test\b/i.test(spaced))return {kind:"test",lessonNumber:null,displayName:"Test",sourceName:raw};
  if(/^lesson\b/i.test(spaced))return {kind:"lesson",lessonNumber:null,displayName:"Lesson",sourceName:raw};
  return {kind:"document",lessonNumber:null,displayName:"Document",sourceName:raw};
}
function v534SourceDisplayName(name,sourceType="learned"){return v534SourceIdentity(name,sourceType).displayName;}

const NOVA534_LABEL_ROLE=Object.freeze({
  "subject":"metadata","topic":"metadata","purpose":"metadata","title":"metadata","heading":"metadata","summary":"metadata","end":"metadata",
  "lesson":"section_lesson","lesson material":"section_lesson","teaching":"section_lesson","teaching material":"section_lesson",
  "test":"section_test","tests":"section_test","test material":"section_test","testing":"section_test","testing material":"section_test",
  "test instruction":"test_instruction","test instructions":"test_instruction","expected answer":"test_expected_answer","expected response":"test_expected_answer","expected behavior":"test_expected_behavior","expected behaviour":"test_expected_behavior","expected reasoning":"test_expected_reasoning","expected action":"test_expected_behavior","expected result":"test_expected_behavior","rubric":"test_rubric","criteria":"test_rubric",
  "concept":"concept","category":"category","parent category":"category_relation","definition":"definition","meaning":"definition","relationship":"relationship","relationships":"relationship",
  "guidance":"guidance","rule":"guidance","rules":"guidance","principle":"guidance","behavior":"behavior","warning":"warning","warnings":"warning","constraint":"constraint","constraints":"constraint","correction":"correction","important":"guidance","validation rule":"validation_rule",
  "procedure":"procedure","step":"procedure_step","steps":"procedure_step","structure":"structure","pattern":"structure",
  "example":"example","examples":"example","practice":"practice","question":"question","answer":"answer","reason":"explanation","why it works":"explanation","why this works":"explanation","not":"negative_example","not enough":"negative_example","incorrect interpretation":"negative_example","overgeneralization":"negative_example","undergeneralization":"negative_example","source":"evidence","classification":"evidence","response":"example","text":"example"
});
function v534LabelKey(raw){const n=v534NormalizeLabel(raw);if(Object.prototype.hasOwnProperty.call(NOVA534_LABEL_ROLE,n))return n;const base=n.replace(/\s+\d+$/," ").trim();return Object.prototype.hasOwnProperty.call(NOVA534_LABEL_ROLE,base)?base:n;}
function v534KnownLabel(raw){return Object.prototype.hasOwnProperty.call(NOVA534_LABEL_ROLE,v534LabelKey(raw));}
function v534RoleForLabel(raw){return NOVA534_LABEL_ROLE[v534LabelKey(raw)]||null;}
function v534StripOuterContainer(value){let text=String(value??"").trim();if(text.length<2)return text;for(const [a,b] of [["[","]"],["(",")"],["{","}"],["\"","\""],["'","'"],["“","”"],["‘","’"]]){if(text.startsWith(a)&&text.endsWith(b)){text=text.slice(a.length,text.length-b.length).trim();break;}}return text;}
function v534HeadingText(line){const raw=String(line||"").trim();let m=raw.match(/^#{1,6}\s+(.+)$/);if(m)return v534Clean(m[1]);m=raw.match(/^SECTION\s+\d+\s*:\s*(.+)$/i);if(m)return v534Clean(m[1]);if(/^(?:LESSON|TEACHING|TEST|TESTS|TESTING)(?:\s+(?:MATERIAL|SECTION|QUESTIONS?))?$/i.test(raw))return v534Clean(raw);m=raw.match(/^\d+\.\s+([A-Z0-9][A-Z0-9 '\-–—/&()]+)$/);return m?v534Clean(m[1]):null;}
function v534SectionKindFromHeading(value){const h=v534Canonical(value);if(/^(?:test|tests|testing)(?:\s+(?:material|section|questions?))?$/.test(h))return "test";if(/^(?:lesson|teaching)(?:\s+(?:material|section))?$/.test(h))return "lesson";return null;}
function v534DividerLine(line){return /^(?:={3,}|-{3,}|_{3,})$/.test(String(line||"").trim());}
function v534LabelMatch(line){const m=String(line||"").trim().match(/^([A-Za-z][A-Za-z0-9 _-]{0,50})\s*:\s*(.*)$/);if(!m||!v534KnownLabel(m[1]))return null;const key=v534LabelKey(m[1]);return {label:key,role:NOVA534_LABEL_ROLE[key],inline:String(m[2]||"")};}
function v534LooksStructuredDocument(text){let hits=0;for(const raw of String(text||"").replace(/\r\n?/g,"\n").split("\n")){if(v534LabelMatch(raw))hits++;if(hits>=2)return true;}return false;}

function v534ParseDocumentStructure(text,meta={}){
  const identity=meta.identity||v534SourceIdentity(meta.sourceName||"document",meta.sourceType||"learned");
  const lines=String(text||"").replace(/\r\n?/g,"\n").split("\n"),blocks=[];let current=null,heading=v534Clean(meta.scope||identity.displayName||"general")||"general",topic=null,concept=null,procedure=null,category=null;
  let sectionKind=identity.kind==="test"?"test":"lesson";
  const scopeNow=()=>procedure||concept||category||topic||heading||"general";
  function push(block){blocks.push({...block,sectionKind:block.sectionKind||sectionKind});}
  function flush(){if(!current)return;const content=v534StripOuterContainer(current.lines.join("\n").trim());current.text=content;current.scope=current.scope||scopeNow();current.sectionKind=current.sectionKind||sectionKind;if(content||["metadata","concept","category","procedure","test_instruction"].includes(current.role))push(current);if(current.role==="metadata"&&current.label==="topic"&&content){topic=content;concept=null;procedure=null;}if(current.role==="concept"&&content){concept=content;procedure=null;}if(current.role==="category"&&content){category=content;procedure=null;}if(current.role==="procedure"&&content)procedure=content;current=null;}
  for(let i=0;i<lines.length;i++){
    const raw=lines[i],trim=String(raw||"").trim();if(v534DividerLine(trim)){flush();continue;}
    const h=v534HeadingText(trim);if(h){flush();const section=v534SectionKindFromHeading(h);if(section)sectionKind=section;heading=h;topic=null;concept=null;category=null;procedure=null;push({id:`block_${blocks.length+1}`,label:"heading",role:"metadata",text:h,scope:h,lineStart:i+1,lineEnd:i+1,inline:true,sectionKind});continue;}
    const lm=v534LabelMatch(trim);if(lm){flush();const inline=v534StripOuterContainer(lm.inline.trim());if(lm.role==="section_lesson"||lm.role==="section_test"){sectionKind=lm.role==="section_test"?"test":"lesson";heading=inline||`${sectionKind} material`;topic=null;concept=null;category=null;procedure=null;if(inline)push({id:`block_${blocks.length+1}`,label:lm.label,role:"metadata",text:inline,scope:heading,lineStart:i+1,lineEnd:i+1,inline:true,sectionKind});continue;}current={id:`block_${blocks.length+1}`,label:lm.label,role:lm.role,lines:inline?[inline]:[],scope:scopeNow(),lineStart:i+1,lineEnd:i+1,inline:Boolean(inline),sectionKind};if(lm.role==="metadata"&&lm.label==="topic"&&inline)topic=inline;if(lm.role==="concept"&&inline)concept=inline;if(lm.role==="category"&&inline)category=inline;if(lm.role==="procedure"&&inline)procedure=inline;continue;}
    if(current){current.lines.push(raw);current.lineEnd=i+1;continue;}if(!trim)continue;current={id:`block_${blocks.length+1}`,label:"body",role:"body",lines:[raw],scope:scopeNow(),lineStart:i+1,lineEnd:i+1,inline:false,sectionKind};
  }
  flush();const counts=blocks.reduce((a,b)=>(a[b.role]=(a[b.role]||0)+1,a),{}),sectionCounts=blocks.reduce((a,b)=>(a[b.sectionKind]=(a[b.sectionKind]||0)+1,a),{});
  return {schema:NOVA534_INGESTION_SCHEMA,version:NOVA534_VERSION,sourceName:meta.sourceName||null,sourceDisplayName:identity.displayName,documentKind:identity.kind,lessonNumber:identity.lessonNumber,structured:v534LooksStructuredDocument(text),blocks,counts,sectionCounts,parsedAt:Date.now()};
}

function v534BlockSentences(text){return typeof v500LessonSentences==="function"?v500LessonSentences(text):String(text||"").split(/(?<=[.!?])\s+|\n+/).map(v534Clean).filter(Boolean);}
function v534BlockUnits(block){
  const role=block?.role,text=v534Clean(block?.text);if(!text)return [];
  if(["guidance","behavior","warning","constraint","validation_rule","correction","procedure_step"].includes(role)){const lines=String(block.text||"").split("\n").map(x=>v534Clean(x.replace(/^[-*•]\s*/,""))).filter(Boolean);return (lines.length>1?lines:[text]).map(t=>({text:t,scope:block.scope||"general",role,label:block.label,blockId:block.id,lineStart:block.lineStart,lineEnd:block.lineEnd,sectionKind:block.sectionKind}));}
  if(["definition","relationship","body"].includes(role))return v534BlockSentences(block.text).map(t=>({text:t,scope:block.scope||"general",role,label:block.label,blockId:block.id,lineStart:block.lineStart,lineEnd:block.lineEnd,sectionKind:block.sectionKind}));
  if(role==="category")return [{text,scope:block.scope||"general",role:"category",label:block.label,learnedCategory:text,blockId:block.id,sectionKind:block.sectionKind}];
  if(role==="category_relation")return [{text,scope:block.scope||"general",role:"category_relation",label:block.label,parentCategory:text,blockId:block.id,sectionKind:block.sectionKind}];
  return [];
}
function v534LooksLikeAssessmentProcedure(scope){const s=v534Canonical(scope||"");return /^(?:verify|test|assess|diagnose)\b/.test(s)||/\b(?:verify|test|assess)\s+lesson\s*\d*\s+learning\b/.test(s);}
function v534StructuredLessonUnits(text,meta={}){const structure=v534ParseDocumentStructure(text,meta),units=[];let activeCategory=null,parentCategory=null;for(const block of structure.blocks){if(block.sectionKind==="test")continue;if(block.role==="category"&&block.text){activeCategory=v534Clean(block.text);continue;}if(block.role==="category_relation"&&block.text){parentCategory=v534Clean(block.text);continue;}if(["metadata","concept","procedure","structure","example","practice","question","answer","negative_example","explanation","evidence","test_instruction","test_expected_answer","test_expected_behavior","test_expected_reasoning","test_rubric"].includes(block.role))continue;/* Older curriculum files sometimes put assessment instructions under a named
     procedure instead of a dedicated Test section. Preserve them as lesson/test
     evidence, but do not compile “verify my learning” into runtime cognition. */if(block.role==="procedure_step"&&v534LooksLikeAssessmentProcedure(block.scope))continue;for(const unit of v534BlockUnits(block))units.push({...unit,learnedCategory:unit.learnedCategory||activeCategory||null,parentCategory:unit.parentCategory||parentCategory||null});}return units;}

function v534InstructionSource(source){if(!source?.text)return false;const identity=v534SourceIdentity(source.name||"document","learned");if(identity.kind==="lesson")return true;if(identity.kind==="test")return false;if(v534LooksStructuredDocument(source.text))return true;const name=v534Canonical(source.name||""),head=v534Canonical(String(source.text).slice(0,1600));return /lesson|curriculum|teaching/.test(name)||/\b(?:concept|definition|guidance|procedure|warning)\s*:/.test(head);}
function v534FilterStructure(structure,sectionKind){const blocks=(structure?.blocks||[]).filter(b=>b.sectionKind===sectionKind);return {...v534Clone(structure),blocks,counts:blocks.reduce((a,b)=>(a[b.role]=(a[b.role]||0)+1,a),{}),sectionCounts:{[sectionKind]:blocks.length}};}
function v534StructureText(structure){return (structure?.blocks||[]).map(block=>{if(block.role==="metadata"&&block.label==="heading")return `\n${block.text}\n`;const label=block.label&&block.label!=="body"?`${block.label}: `:"";return `${label}${block.text||""}`;}).filter(Boolean).join("\n");}

function v534TestStore(){const memory=typeof v43Memory==="function"?v43Memory():null;if(!memory)return {version:NOVA534_VERSION,suites:[],lastRun:null};memory.curriculumTests ||= {schema:2,version:NOVA534_VERSION,suites:[],lastRun:null,updatedAt:Date.now()};memory.curriculumTests.schema=2;memory.curriculumTests.version=NOVA534_VERSION;memory.curriculumTests.suites ||= [];if(memory.curriculumTests.lastRun)v534NormalizeStoredTestRun(memory.curriculumTests.lastRun);for(const suite of memory.curriculumTests.suites||[]){if(suite?.lastRun)v534NormalizeStoredTestRun(suite.lastRun);}return memory.curriculumTests;}
function v534ExtractTestSuite(source,structure,identity=v534SourceIdentity(source?.name||"document")){
  const blocks=(structure?.blocks||[]).filter(b=>identity.kind==="test"||b.sectionKind==="test"),cases=[],instructions=[];let current=null;
  const finish=()=>{if(!current)return;if(current.question)cases.push(current);current=null;};
  for(const block of blocks){const text=v534Clean(block.text);if(!text)continue;if(block.role==="test_instruction"||block.role==="practice"){instructions.push(text);continue;}if(block.role==="question"){finish();current={id:`test_${cases.length+1}`,question:text,expectedAnswer:null,expectedBehavior:null,expectedReasoning:null,rubric:null,sourceLine:block.lineStart};continue;}if(!current)continue;if(block.role==="answer"||block.role==="test_expected_answer"){current.expectedAnswer=text;continue;}if(block.role==="test_expected_behavior"){current.expectedBehavior=text;continue;}if(block.role==="test_expected_reasoning"||block.role==="explanation"){current.expectedReasoning=text;continue;}if(block.role==="test_rubric"||block.role==="validation_rule"){current.rubric=text;continue;}}
  finish();return {id:`suite_${source?.id||Date.now()}`,sourceId:source?.id||null,sourceName:source?.name||"document",sourceDisplayName:identity.displayName,lessonNumber:identity.lessonNumber,kind:"test",instructions,cases,createdAt:Date.now(),updatedAt:Date.now()};
}
function v534NormalizeTestAnswer(value){return v534Canonical(String(value??"").replace(/[.!?]+$/g,"").replace(/^['"“”‘’]|['"“”‘’]$/g,"").trim());}
const NOVA534_GRADE_STOPWORDS=new Set(["a","an","the","is","are","was","were","be","been","being","to","of","in","on","at","for","from","with","and","or","that","this","it","its","as","by","what","which","who","when","where","why","how","should","would","could","can","do","does","did"]);
function v534GradeTokens(value){return v534NormalizeTestAnswer(value).split(/[^a-z0-9]+/i).map(v=>v.trim()).filter(v=>v&& !NOVA534_GRADE_STOPWORDS.has(v));}
function v534GradePolarity(value){const n=v534NormalizeTestAnswer(value);if(/^(?:no|not|never|false)\b/.test(n))return "negative";if(/^(?:yes|true|correct)\b/.test(n))return "positive";return null;}
function v534AutoGradeTestCase(testCase,response){
  const actual=v534NormalizeTestAnswer(response);
  if(!actual)return {autoStatus:"fail",autoPassed:false,autoReason:"Nova did not produce a spoken answer."};
  if(!testCase?.expectedAnswer)return {autoStatus:"review",autoPassed:null,autoReason:"No exact expected answer was supplied; compare Nova's attempt with the stored behavior or rubric."};
  const alternatives=String(testCase.expectedAnswer).split(/\s*\|\s*/).map(v534NormalizeTestAnswer).filter(Boolean);
  if(alternatives.includes(actual))return {autoStatus:"pass",autoPassed:true,autoReason:"Nova's answer matched an accepted answer after normalization."};
  for(const expected of alternatives){
    if(expected.length>=3&&actual.includes(expected))return {autoStatus:"pass",autoPassed:true,autoReason:"Nova included the complete expected answer with additional harmless wording."};
    const ep=v534GradePolarity(expected),ap=v534GradePolarity(actual);
    if(ep&&ap&&ep!==ap)return {autoStatus:"fail",autoPassed:false,autoReason:"Nova's answer has the opposite yes/no polarity from the expected answer."};
  }
  let best=0;
  for(const expected of alternatives){
    const e=[...new Set(v534GradeTokens(expected))],a=new Set(v534GradeTokens(actual));
    if(!e.length)continue;
    const covered=e.filter(token=>a.has(token)).length/e.length;
    best=Math.max(best,covered);
  }
  if(best>=0.78)return {autoStatus:"pass",autoPassed:true,autoReason:`Nova expressed most of the expected answer's key terms (${Math.round(best*100)}% coverage).`};
  if(best>=0.35)return {autoStatus:"review",autoPassed:null,autoReason:`Nova's wording overlaps the expected answer, but the automatic grader is not confident enough to decide (${Math.round(best*100)}% key-term coverage).`};
  const shortAnswer=alternatives.length>0&&alternatives.every(expected=>v534GradeTokens(expected).length<=2);
  if(shortAnswer)return {autoStatus:"fail",autoPassed:false,autoReason:"This test expects a short concrete answer and Nova did not provide an accepted answer."};
  return {autoStatus:"review",autoPassed:null,autoReason:"Nova's wording differs too much from the longer expected answer for a safe automatic decision. Manual review is required."};
}
function v534EffectiveGrade(result){
  const manual=result?.manualStatus;
  if(manual==="pass")return {status:"pass",passed:true,gradeSource:"manual"};
  if(manual==="fail")return {status:"fail",passed:false,gradeSource:"manual"};
  return {status:result?.autoStatus||result?.status||"review",passed:result?.autoPassed!==undefined?result.autoPassed:result?.passed??null,gradeSource:"automatic"};
}
function v534GradeTestCase(testCase,response){const auto=v534AutoGradeTestCase(testCase,response),effective=v534EffectiveGrade(auto);return {...auto,manualStatus:null,status:effective.status,passed:effective.passed,gradeSource:effective.gradeSource,reason:auto.autoReason};}
function v534NormalizeStoredTestRun(run){
  if(!run||!Array.isArray(run.results))return run;
  for(const result of run.results){
    if(!result.autoStatus)result.autoStatus=result.status||"review";
    if(result.autoPassed===undefined)result.autoPassed=result.passed??null;
    if(!result.autoReason)result.autoReason=result.reason||"Legacy automatic grade imported from an earlier test run.";
    if(result.manualStatus===undefined)result.manualStatus=null;
  }
  return v534RecountTestRun(run);
}
function v534RecountTestRun(run){
  const results=Array.isArray(run?.results)?run.results:[];
  for(const result of results){const e=v534EffectiveGrade(result);result.status=e.status;result.passed=e.passed;result.gradeSource=e.gradeSource;result.reason=result.manualStatus?`User marked this answer ${result.manualStatus==="pass"?"correct":"incorrect"}.`:result.autoReason||result.reason||"";}
  run.passed=results.filter(r=>r.passed===true).length;run.failed=results.filter(r=>r.passed===false).length;run.review=results.filter(r=>r.passed==null).length;run.updatedAt=Date.now();return run;
}
function v534PersistTestSuite(suite,run){const store=v534TestStore();v534RecountTestRun(run);store.lastRun=v534Clone(run);const i=store.suites.findIndex(x=>x.id===suite.id),saved={...v534Clone(suite),lastRun:v534Clone(run),updatedAt:Date.now()};if(i>=0)store.suites[i]=saved;else store.suites.push(saved);store.suites=store.suites.slice(-80);store.updatedAt=Date.now();try{saveState();}catch(_){ }return run;}
function v534SetManualTestGrade(suiteId,testCaseId,manualStatus){
  const allowed=new Set([null,"pass","fail"]);if(!allowed.has(manualStatus))return {ok:false,reason:"invalid_manual_status"};
  const store=v534TestStore(),run=store.lastRun;if(!run||run.suiteId!==suiteId)return {ok:false,reason:"run_not_found"};
  const result=(run.results||[]).find(r=>r.id===testCaseId);if(!result)return {ok:false,reason:"test_case_not_found"};
  if(!result.autoStatus){result.autoStatus=result.status||"review";result.autoPassed=result.passed??null;result.autoReason=result.reason||"Legacy automatic grade imported before manual review.";}
  result.manualStatus=manualStatus;v534RecountTestRun(run);
  const suite=(store.suites||[]).find(s=>s.id===suiteId);if(suite)suite.lastRun=v534Clone(run);store.lastRun=v534Clone(run);store.updatedAt=Date.now();try{saveState();}catch(_){ }
  return {ok:true,run:v534Clone(run),result:v534Clone(result)};
}

/*
 * Curriculum tests intentionally do NOT call the cognitive core directly.
 * The caller supplies an askQuestion callback that must send the question through
 * Nova's ordinary visible turn controller. Only the question and safe positional
 * metadata are passed to that callback; the expected answer/rubric remains private
 * to this evaluator until the visible Nova turn has completed.
 */
async function v534RunInteractiveTestSuite(suite,options={}){
  const askQuestion=options.askQuestion;if(typeof askQuestion!=="function")throw new Error("Interactive curriculum tests require the normal conversation turn callback.");
  const cases=Array.isArray(suite?.cases)?suite.cases:[],results=[];
  const startedAt=Date.now();
  const runId=`teacher_run_${startedAt}_${Math.random().toString(36).slice(2,8)}`;
  for(let index=0;index<cases.length;index++){
    const testCase=cases[index];let response=null,terminal=null,reasonCode=null,error=null;
    try{
      const attempt=await askQuestion(testCase.question,{index,total:cases.length,sourceDisplayName:suite?.sourceDisplayName||"Test",sourceLabel:"Teacher"});
      if(attempt&&typeof attempt==="object"&&!Array.isArray(attempt)){response=typeof attempt.response==="string"?attempt.response:null;terminal=attempt.terminal||null;reasonCode=attempt.reasonCode||null;}
      else response=typeof attempt==="string"?attempt:null;
    }catch(e){error=e?.message||String(e);}
    // The answer key is consulted only after the normal Nova turn above is complete.
    const grade=v534GradeTestCase(testCase,response);
    const result={...v534Clone(testCase),response,terminal,reasonCode,error,...grade,attemptedAt:Date.now(),teacherAttemptId:null};
    if(window.__TAURI__?.core?.invoke&&typeof nova7RustTeacherRecordAttempt==="function"){
      const receipt=await nova7RustTeacherRecordAttempt({
        runId,
        suiteId:String(suite?.id||"test_suite"),
        caseId:String(testCase?.id||`test_${index+1}`),
        sourceDisplayName:String(suite?.sourceDisplayName||"Test"),
        lessonContext:suite?.lessonNumber?`Lesson ${suite.lessonNumber}`:(suite?.sourceName?String(suite.sourceName):null),
        question:String(testCase?.question||""),
        originalAnswer:typeof response==="string"?response:"",
        expectedAnswer:testCase?.expectedAnswer?String(testCase.expectedAnswer):null,
        expectedBehavior:testCase?.expectedBehavior?String(testCase.expectedBehavior):null,
        automaticStatus:String(grade?.autoStatus||grade?.status||"review"),
        automaticReason:String(grade?.autoReason||grade?.reason||""),
        terminalState:terminal?String(terminal):null,
        reasonCode:reasonCode?String(reasonCode):null,
        provenance:{runId,index,total:cases.length,sourceDisplayName:String(suite?.sourceDisplayName||"Test"),lessonNumber:suite?.lessonNumber??null,sourceName:suite?.sourceName||null,provenance:"Teacher"}
      });
      result.teacherAttemptId=receipt?.record?.id||null;
    }
    results.push(result);
    if(typeof options.afterAttempt==="function")await options.afterAttempt(v534Clone(result),{index,total:cases.length,sourceDisplayName:suite?.sourceDisplayName||"Test"});
  }
  const run={runId,suiteId:suite?.id||null,sourceDisplayName:suite?.sourceDisplayName||"Test",total:results.length,passed:0,failed:0,review:0,results,startedAt,ranAt:Date.now(),mode:"visible_sequential_teacher_turns",teacherEvidenceReleased:0};
  v534RecountTestRun(run);
  if(window.__TAURI__?.core?.invoke&&typeof nova7RustTeacherFinalizeRun==="function"){
    const finalized=await nova7RustTeacherFinalizeRun({runId});
    run.teacherEvidenceReleased=Number(finalized?.releasedCount||0);
  }
  if(options.persist!==false)v534PersistTestSuite(suite,run);
  return run;
}
function v534FormatTestRun(run){if(!run)return "";const graded=Number(run.passed||0)+Number(run.failed||0),score=graded?`${run.passed}/${graded}`:`0/0`;const review=run.review?` • ${run.review} need review`:"";return `${run.sourceDisplayName||"Test"} complete. Automatic score: ${score} passed${review}. You can correct any grading mistakes in Brain Inspector without changing Nova's original answer.`; }
function v534TestSummary(){const store=v534TestStore();return {suites:(store.suites||[]).length,lastRun:v534Clone(store.lastRun||null),recentSuites:v534Clone((store.suites||[]).slice(-10).reverse())};}

function v534RegisterDeclaredCategories(text,meta={}){if(typeof v620RegisterCategory!=="function")return {categories:0,relations:0};const structure=v534ParseDocumentStructure(text,meta);let categories=0,relations=0,current=null;for(const block of structure.blocks){if(block.sectionKind==="test")continue;if(block.role==="category"&&block.text){const before=v620CategoryStore().categories.length;current=v620RegisterCategory(block.text,{origin:"learned",evidence:{sourceName:meta.sourceName||null,sourceId:meta.sourceId||null,line:block.lineStart}});if(v620CategoryStore().categories.length>before)categories++;continue;}if(block.role==="category_relation"&&block.text&&current&&typeof v620RelateCategories==="function"){const before=v620CategoryStore().relations.length;v620RelateCategories(current.name,"kind_of",block.text,{origin:"learned",evidence:{sourceName:meta.sourceName||null,sourceId:meta.sourceId||null,line:block.lineStart}});if(v620CategoryStore().relations.length>before)relations++;}}return {categories,relations};}

function v534CompileDocumentCognition(source,options={}){
  if(window.__TAURI__?.core?.invoke)throw new Error("v534CompileDocumentCognition is retired on native Nova: Rust Pass 6 owns document parsing and cognitive translation.");
  if(!source?.text)return {version:NOVA534_VERSION,skipped:true,reason:"missing_source_text"};const identity=options.identity||v534SourceIdentity(source.name||"document","learned"),meta={sourceType:"learned",sourceName:source.name||"document",sourceDisplayName:identity.displayName,sourceKind:identity.kind,sourceLessonNumber:identity.lessonNumber,sourceId:source.id||null,scope:options.scope||"general",synthesize:options.synthesize!==false,identity};
  const fullStructure=options.structure||v534ParseDocumentStructure(source.text,meta),structure=v534FilterStructure(fullStructure,"lesson"),lessonText=v534StructureText(structure),lessonSource={...source,text:lessonText};if(!structure.blocks.length)return {version:NOVA534_VERSION,skipped:true,reason:"no_lesson_material",structure:{structured:fullStructure.structured,blockCount:0,counts:{}}};
  const declared=v534RegisterDeclaredCategories(lessonText,meta),cognitive=typeof v500CompileCognitionRulesFromText==="function"?v500CompileCognitionRulesFromText(lessonText,meta):null;let sourceSynthesis=null;
  const nativeRustLearning=Boolean(window.__TAURI__?.core?.invoke);
  if(nativeRustLearning)sourceSynthesis={status:"deferred_to_rust_authority",reason:"Pass 6 native learning bypasses this retired JavaScript compiler; Rust owns parsing, semantic learning, translation, synthesis, and executability."};
  else try{if(typeof v520SynthesizeSourceCognition==="function")sourceSynthesis=v520SynthesizeSourceCognition(lessonSource,structure,meta);}catch(error){sourceSynthesis={status:"error",reason:error?.message||String(error)};}
  try{const memory=v43Memory();memory.lastDocumentIngestion={version:NOVA534_VERSION,sourceId:source.id||null,sourceName:source.name||"document",sourceDisplayName:identity.displayName,documentKind:identity.kind,lessonNumber:identity.lessonNumber,structured:fullStructure.structured,blockCount:fullStructure.blocks.length,counts:v534Clone(fullStructure.counts),sectionCounts:v534Clone(fullStructure.sectionCounts),declaredCategories:declared,sourceSynthesis:v534Clone(sourceSynthesis),compiledAt:Date.now()};}catch(_){ }
  return {version:NOVA534_VERSION,sourceDisplayName:identity.displayName,documentKind:identity.kind,structure:{structured:fullStructure.structured,blockCount:structure.blocks.length,counts:structure.counts,sectionCounts:fullStructure.sectionCounts},declaredCategories:declared,cognitive,sourceSynthesis};
}

/* ---------------------------------------------------------
   CURRENT DOCUMENT INTAKE
   --------------------------------------------------------- */
async function v620HandleDocumentFile(file,options={}){
  if(!file)throw new Error("No document was provided.");if(typeof v43TextLikeFile!=="function"||!v43TextLikeFile(file))return {status:"unsupported",response:`I can currently read text-based lesson and test files only. ${file.name||"That file"} needs a format reader that is not part of my current mind yet.`};
  if(!options.skipTurnIncrement&&typeof v43BeginHandledTurn==="function")v43BeginHandledTurn();const rawText=await file.text();if(rawText.length>500000)throw new Error("That document is larger than the current 500,000-character intake limit.");
  const native=Boolean(window.__TAURI__?.core?.invoke);let rustParsed=null;
  if(native){
    if(typeof nova7RustMindParseSource!=="function")throw new Error("Rust Pass 6 document parser is unavailable; refusing to let JavaScript decide native learning intent.");
    rustParsed=await nova7RustMindParseSource({name:file.name||"document",text:rawText},{kindHint:null,sourceType:"learned"});
  }
  // JavaScript structure parsing remains for Teacher/test presentation only. On
  // native Nova, Rust's parsed source decides whether brain learning is allowed.
  const jsIdentity=v534SourceIdentity(file.name||"document","learned"),identity=rustParsed?{kind:rustParsed.documentKind,lessonNumber:rustParsed.lessonNumber,displayName:rustParsed.sourceDisplayName,sourceName:rustParsed.sourceName}:jsIdentity,classification=typeof v43ClassifyDocument==="function"?v43ClassifyDocument(file.name||"document",rawText):{type:"unknown",confidence:0};const structure=v534ParseDocumentStructure(rawText,{sourceName:file.name||"document",sourceType:"learned",identity:jsIdentity});
  const lessonStructure=v534FilterStructure(structure,"lesson"),testStructure=v534FilterStructure(structure,"test"),lessonText=v534StructureText(lessonStructure),testText=v534StructureText(testStructure),hasLesson=lessonStructure.blocks.length>0,hasTest=rustParsed?Boolean(rustParsed.hasTestMaterial):testStructure.blocks.some(b=>b.role==="question"||b.role==="test_instruction"||b.role==="test_expected_answer"||b.role==="test_expected_behavior");
  const instructional=rustParsed?Boolean(rustParsed.instructional):identity.kind==="lesson"||identity.kind!=="test"&&hasLesson&&(/lesson|curriculum|teaching/.test(v534Canonical(file.name||""))||v534InstructionSource({name:file.name||"document",text:lessonText}));
  let source=null,semantic=null,cognitive=null,synthesis=null,upgrade=null,abstraction=null,testSuite=null,testRun=null;
  if(instructional&&identity.kind!=="test"){
    // Pass 6 replacement boundary: JavaScript does not assign brain identity to
    // a learned source. Rust receives the raw named source with no frontend ID,
    // derives the stable source ID, learns it, and mirrors that identity back for UI/persistence.
    if(!window.__TAURI__?.core?.invoke)return {status:"native_mind_required",response:"This Nova build no longer has a JavaScript learning brain. Lesson learning requires the Rust Nova mind."};
    if(typeof nova7RustMindIngestSource!=="function")throw new Error("Rust Pass 6 lesson ingestion is unavailable; refusing to fall back to JavaScript learning.");
    const rustTeaching=await nova7RustMindIngestSource({name:file.name||"document",text:rawText},{kindHint:rustParsed?.documentKind||identity.kind||null,sourceType:"learned"});
    const rustSourceId=String(rustTeaching?.sourceId||"");
    const rustSource=(rustTeaching?.semanticState?.sources||[]).find(row=>String(row?.id||"")===rustSourceId)||null;
    source=rustSource?{...v534Clone(rustSource),classification,purpose:"rust_pass6_source_ui_mirror",originalFileName:file.name||"document"}:{id:rustSourceId||null,name:file.name||"document",displayName:identity.displayName,text:rawText,classification,purpose:"rust_pass6_source_ui_mirror",documentKind:identity.kind,lessonNumber:identity.lessonNumber,originalFileName:file.name||"document",rustAuthority:"rust_v7_pass_8"};
    try{const memory=typeof v43Memory==="function"?v43Memory():null,mirrored=(memory?.sources||[]).find(row=>String(row?.id||"")===rustSourceId);if(mirrored){mirrored.classification=v534Clone(classification);mirrored.purpose="rust_pass6_source_ui_mirror";mirrored.originalFileName=file.name||"document";source=mirrored;}}catch(_){ }
    semantic=rustTeaching?.semanticReport||null;cognitive={authority:"rust_v7_pass_8",parsed:rustTeaching?.parsed||null};synthesis=rustTeaching?.synthesisReport||null;upgrade={authority:"rust_v7_pass_8",upgraded:Number(synthesis?.upgraded||0)};
    try{if(typeof v554RefreshCognitiveAbstractions==="function")abstraction=v554RefreshCognitiveAbstractions({reason:"rust_pass6_document_learning"});}catch(_){ }
  }
  if(identity.kind==="test"||hasTest){
    const testIdentity=identity.kind==="test"?identity:{kind:"test",lessonNumber:identity.lessonNumber,displayName:identity.lessonNumber?`Test Lesson ${identity.lessonNumber}`:`Test — ${identity.displayName}`,sourceName:file.name||"document"};
    const pending={id:`pending_test_${Date.now()}_${Math.random().toString(36).slice(2,7)}`,name:identity.kind==="test"?(file.name||"document"):`Test_${file.name||"document"}`,text:testText||rawText};
    testSuite=v534ExtractTestSuite(pending,testStructure.blocks.length?testStructure:structure,testIdentity);
    if(testSuite.cases.length){
      // Keep the suite transient until Nova has answered every visible Teacher turn.
      // In particular, do not archive the answer key into semantic/source memory first.
      testSuite={...testSuite,id:`suite_${Date.now()}_${Math.random().toString(36).slice(2,8)}`,sourceId:null,sourceName:pending.name,sourceDisplayName:testIdentity.displayName,pending:true};
    }
  }
  try{const c=typeof v43EnsureCognition==="function"?v43EnsureCognition():null;if(c)c.lastDocumentDecision={fileName:file.name||"document",displayName:identity.displayName,classification,decision:{action:identity.kind==="test"?"test":instructional&&testSuite?.cases?.length?"learn_then_test":instructional?"learn":"retain_for_instruction",reason:identity.kind==="test"?"filename identifies curriculum test":instructional?"lesson material identified":"document purpose is not explicit enough to treat as teaching"}};}catch(_){ }
  try{if(typeof novaSaveDurableCheckpoint==="function")await novaSaveDurableCheckpoint("document_ingestion");else saveState();}catch(error){console.error("Document-ingestion durable checkpoint failed:",error);}
  if(identity.kind==="test")return {status:testSuite?.cases?.length?"test_ready":"test_empty",source:null,classification,testSuite,testRun:null,response:null};
  if(!instructional&&!testSuite?.cases?.length){const stored=typeof v43StoreSource==="function"?v43StoreSource(file.name||"document",rawText,classification,{purpose:"current_document_intake"}):null;const response=`I stored ${identity.displayName} as source evidence, but I did not treat it as a lesson or a test because its role is not clear.`;return {status:"awaiting_instruction",source:stored,classification,response};}
  const counts=semantic?.counts||{},learnedTotal=Number(counts.concepts||0)+Number(counts.claims||0)+Number(counts.procedures||0)+Number(counts.guidance||0)+Number(counts.examples||0),executable=Number(synthesis?.upgraded||0),storedOnly=Number(synthesis?.storedOnly??synthesis?.remaining??0),categoryCount=Number(cognitive?.declaredCategories?.categories||0);
  const response=[instructional?`I learned from ${identity.displayName}.`:null,learnedTotal?`${learnedTotal} semantic item${learnedTotal===1?"":"s"} were added or updated.`:null,categoryCount?`${categoryCount} new declared categor${categoryCount===1?"y":"ies"} entered my open category graph.`:null,executable?`${executable} learned cognitive item${executable===1?"":"s"} became executable.`:null,storedOnly?`${storedOnly} cognitive item${storedOnly===1?" remains":"s remain"} Stored Only until later learning supplies the missing connections.`:null,null].filter(Boolean).join(" ");
  return {status:testSuite?.cases?.length?"learned_test_ready":"learned",source,classification,semantic,cognitive,synthesis,upgrade,abstraction,testSuite,testRun:null,response};
}

function v534PostLoadDocumentIngestionMaintenance(){try{const memory=v43Memory();memory.documentIngestion={schema:NOVA534_INGESTION_SCHEMA,version:NOVA534_VERSION,updatedAt:Date.now()};const store=v534TestStore();store.version=NOVA534_VERSION;return v534Clone(memory.documentIngestion);}catch(_){return {schema:NOVA534_INGESTION_SCHEMA,version:NOVA534_VERSION};}}
try{if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){V41_SELF_MODEL.identity.version=NOVA534_VERSION;V41_SELF_MODEL.capabilities ||= [];const entry={id:"structured_document_ingestion",verb:"distinguish",object:"lesson material, test material, examples, and answer keys before learning or evaluation",detail:"Lesson material can change cognition. Test answer keys remain separate until the same current mind has attempted each question."},old=V41_SELF_MODEL.capabilities.find(c=>c.id===entry.id);if(old)Object.assign(old,entry);else V41_SELF_MODEL.capabilities.push(entry);}}catch(_){ }
console.info("Nova V6.2.16 lesson/test document ingestion loaded.");
