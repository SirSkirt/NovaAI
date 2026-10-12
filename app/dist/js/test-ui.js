"use strict";
// Teacher controls only: answers stay in test metadata and never become
// additional model input. Every actual reply comes from Rust hybrid inference.
(()=>{
const $=id=>document.getElementById(id);
const invoke=(cmd,args={})=>window.__TAURI__?.core?.invoke(cmd,args)
  ?? Promise.reject(Error("Launch the installed app for native test execution"));
let archive={definitions:[],reports:[]};
let currentRun=null,active=false,paused=false;
function state(message,error=false){
  $("testRunStatus").textContent=message;
  $("testRunStatus").style.color=error?"#ff9eae":"";
}
async function runSafely(work){
  try{return await work();}catch(error){
    state(String(error),true);console.error("Neural Nova test laboratory",error);return null;
  }
}
function testControls(){
  $("testStart").disabled=active;
  $("testSave").disabled=active;
  $("testDelete").disabled=active;
  $("testSaved").disabled=active;
  $("testPause").disabled=!active||paused;
  $("testResume").disabled=!active||!paused;
  $("testStop").disabled=!active;
}
async function refresh(preserveSelection=true){
  const previous=preserveSelection?$("testSaved").value:"";
  archive=await invoke("hybrid_test_archive");
  const selection=$("testSaved");selection.replaceChildren();
  const empty=document.createElement("option");empty.value="";empty.textContent="Select a saved test";
  selection.append(empty);
  for(const item of archive.definitions){
    const o=document.createElement("option");o.value=item.id;
    o.textContent=item.name+" · "+item.cases.length+" questions";
    selection.append(o);
  }
  if(archive.definitions.some(x=>x.id===previous))selection.value=previous;
  if(!currentRun && archive.reports.length){
    currentRun=archive.reports[archive.reports.length-1];
    if(currentRun.status==="running"||currentRun.status==="paused")
      currentRun={...currentRun,status:"interrupted"};
    render(currentRun);
  }
  testControls();
}
const selections=[
  ["testQuestionsFile","testQuestions"],["testAnswersFile","testAnswers"],
  ["testUnseenQuestionsFile","testUnseenQuestions"],["testUnseenAnswersFile","testUnseenAnswers"]
];
for(const [fileId,textId] of selections){
  $(fileId).addEventListener("change",()=>runSafely(async()=>{
    const file=$(fileId).files[0];if(!file)return;
    if(file.size>60_000)throw Error("TXT files must be 60 KB or smaller");
    const content=await file.text();
    if(!/^[\x00-\x7f]*$/.test(content))throw Error("Nova currently supports ASCII TXT content only");
    $(textId).value=content;
    state("Imported "+file.name+" into "+textId+". Save the test when ready.");
  }));
}
function definitionContents(d){
  $("testName").value=d.name;
  $("testQuestions").value=d.cases.map(c=>c.question).join("\n");
  $("testAnswers").value=d.cases.map(c=>c.expected).join("\n");
  $("testUnseenQuestions").value=d.unseen.map(c=>c.question).join("\n");
  $("testUnseenAnswers").value=d.unseen.map(c=>c.expected).join("\n");
}
$("testSaved").addEventListener("change",()=>{
  const d=archive.definitions.find(d=>d.id===$("testSaved").value);
  if(d){definitionContents(d);state("Loaded "+d.name+".");}
});
async function saveDefinition(){
  const name=$("testName").value.trim();
  if(!name)throw Error("Give the test a name.");
  const definition=await invoke("hybrid_test_save",{
    name,questionsText:$("testQuestions").value,answersText:$("testAnswers").value,
    unseenQuestionsText:$("testUnseenQuestions").value,
    unseenAnswersText:$("testUnseenAnswers").value
  });
  await refresh(false);
  $("testSaved").value=definition.id;
  state("Saved "+definition.cases.length+" questions and "+definition.unseen.length+
    " untouched generalization questions.");
  return definition;
}
$("testSave").addEventListener("click",()=>runSafely(saveDefinition));
$("testDelete").addEventListener("click",()=>runSafely(async()=>{
  const id=$("testSaved").value;
  if(!id)throw Error("Select a test to delete.");
  if(!confirm("Delete the saved test definition? Completed test history will be preserved."))return;
  await invoke("hybrid_test_delete",{testId:id});
  await refresh(false);
  state("Saved test deleted; previous results remain.");
}));
function options(){
  return {
    attempts:Number($("testAttempts").value),
    corrections:$("testCorrections").checked,
    learningRate:Number($("testRate").value),
    maxChars:Number($("testReplyLength").value),
    contextChars:Number($("testContext").value),
    deterministic:$("testSampling").value==="greedy",
    seed:$("testSampling").value==="seeded"?42:null,
    retention:$("testRetention").checked,
    generalization:$("testGeneralization").checked
  };
}
$("testStart").addEventListener("click",()=>runSafely(async()=>{
  if(active)return;
  let selected=archive.definitions.find(d=>d.id===$("testSaved").value);
  const edited=selected && (
    $("testName").value!==selected.name ||
    $("testQuestions").value.trim()!==selected.cases.map(c=>c.question).join("\n") ||
    $("testAnswers").value.trim()!==selected.cases.map(c=>c.expected).join("\n") ||
    $("testUnseenQuestions").value.trim()!==selected.unseen.map(c=>c.question).join("\n") ||
    $("testUnseenAnswers").value.trim()!==selected.unseen.map(c=>c.expected).join("\n")
  );
  if(!selected||edited)selected=await saveDefinition();
  const settings=options();
  if(!Number.isInteger(settings.attempts)||settings.attempts<1||settings.attempts>5)
    throw Error("Choose 1–5 attempts.");
  if(settings.generalization && !selected.unseen.length)
    throw Error("Import untouched generalization questions or switch that option off.");
  const runId=await invoke("hybrid_test_start",{testId:selected.id,options:settings});
  active=true;paused=false;currentRun=null;testControls();
  state("Running "+selected.name+" · baseline for ALL questions first · "+runId);
}));
$("testPause").addEventListener("click",()=>runSafely(async()=>{
  await invoke("hybrid_test_pause");paused=true;testControls();
  state("Pause requested. Nova will pause at the next safe training/generation boundary.");
}));
$("testResume").addEventListener("click",()=>runSafely(async()=>{
  await invoke("hybrid_test_resume");paused=false;testControls();state("Resuming neural test.");
}));
$("testStop").addEventListener("click",()=>runSafely(async()=>{
  await invoke("hybrid_test_stop");paused=false;state("Stopping safely after the current operation.");
}));
const cell=(name,value)=>{
  const element=document.createElement("div");
  const label=document.createElement("span");label.textContent=name;
  const strong=document.createElement("strong");strong.textContent=String(value);
  element.append(label,strong);return element;
};
function renderQuestion(result,index,unseen=false,run=currentRun){
  const detail=document.createElement("details");detail.className="test-question";
  const summary=document.createElement("summary");
  const attempts=result.attempts||[];
  const latest=attempts[attempts.length-1];
  summary.textContent=(unseen?"Unseen ":"Q")+(index+1)+". "+result.question+" — "+
    (latest?(latest.teacherGrade||latest.automatic):"Waiting");
  detail.append(summary);
  const contents=document.createElement("div");contents.className="test-question-body";
  const answer=document.createElement("p");answer.textContent="Expected: "+result.expected;
  contents.append(answer);
  if(result.retention){
    const retention=document.createElement("p");
    retention.textContent="Retention: "+result.retention.automatic+" — "+result.retention.response;
    contents.append(retention);
  }
  for(let i=0;i<attempts.length;i++){
    const a=attempts[i];
    const entry=document.createElement("div");entry.className="test-attempt";
    const title=document.createElement("strong");
    title.textContent="Attempt "+(i+1)+" · "+a.automatic+" · step "+a.modelStep+
      " · "+a.sampling;
    const output=document.createElement("pre");output.textContent=a.response||"(empty response)";
    const correction=document.createElement("p");
    correction.textContent=a.gradientSteps
      ?"Corrective updates: "+a.gradientSteps+" · loss "+Number(a.correctionLoss).toFixed(4)
      :"No neural correction applied";
    entry.append(title,output,correction);
    if(!unseen && run.status!=="running" && run.status!=="paused"){
      const review=document.createElement("div");review.className="row wrap";
      const select=document.createElement("select");
      for(const [value,label] of [["correct","Correct"],["incorrect","Incorrect"],["needsReview","Needs Teacher Review"]]){
        const op=document.createElement("option");op.value=value;op.textContent=label;
        select.append(op);
      }
      select.value=a.teacherGrade||a.automatic;select.setAttribute("aria-label","Teacher grade");
      const button=document.createElement("button");button.className="secondary compact";button.type="button";
      button.textContent="Save teacher grade";
      button.addEventListener("click",()=>runSafely(async()=>{
        await invoke("hybrid_test_review",{
          runId:run.id,index,attempt:i,teacherGrade:select.value
        });
        await refresh();
        const newRecord=archive.reports.find(x=>x.id===run.id);
        if(newRecord){currentRun=newRecord;render(currentRun);}
        state("Teacher override recorded alongside automatic assessment.");
      }));
      review.append(select,button);entry.append(review);
    }
    contents.append(entry);
  }
  detail.append(contents);return detail;
}
function render(report){
  currentRun=report;
  const first=report.questions.length;
  const accuracy=(n,d)=>d?(100*n/d).toFixed(1)+"%":"—";
  $("testOverview").open=true;
  $("testOverviewSummary").textContent=report.name+" · "+report.status+
    " · "+report.questions.filter(q=>q.attempts.length).length+"/"+first+" questions started";
  const stats=$("testOverviewStats");stats.replaceChildren(
    cell("Initial, untouched",accuracy(report.initialCorrect,first)),
    cell("Latest post-correction",accuracy(report.finalCorrect,first)),
    cell("Correct after correction",report.corrected),
    cell("Still incorrect",report.stillIncorrect),
    cell("Corrective updates",report.trainingUpdates),
    cell("Model steps",report.startStep+" → "+report.endStep)
  );
  if(report.config.generalization){
    stats.append(cell("Unseen baseline",report.baselineUnseenCorrect===null?"Pending":
      accuracy(report.baselineUnseenCorrect,report.unseen.length)));
    stats.append(cell("Unseen after training",report.finalUnseenCorrect===null?"Pending":
      accuracy(report.finalUnseenCorrect,report.unseen.length)));
  }
  const list=$("testResultList");list.replaceChildren();
  report.questions.forEach((q,i)=>list.append(renderQuestion(q,i,false,report)));
  if(report.unseen.length&&report.config.generalization){
    const title=document.createElement("h4");title.textContent="Untouched generalization set";
    list.append(title);
    report.unseen.forEach((q,i)=>list.append(renderQuestion(q,i,true,report)));
  }
  const completed=report.questions.reduce((n,q)=>n+q.attempts.length,0);
  state(report.status+" · "+completed+" attempts recorded · "+
    report.trainingUpdates+" corrective gradient steps"+(report.error?" · "+report.error:""));
}
async function setup(){
  if(!window.__TAURI__?.core?.invoke)return;
  await refresh();
  const events=window.__TAURI__?.event;
  if(events?.listen){
    await events.listen("hybrid-test-progress",({payload})=>{
      currentRun=payload;active=payload.status==="running";testControls();render(payload);
    });
    await events.listen("hybrid-test-done",({payload})=>{
      active=false;paused=false;testControls();render(payload);
      void refresh();
    });
  }
}
document.addEventListener("neural-nova-booted",()=>runSafely(setup),{once:true});
})();
