"use strict";
/* Education and measurement UI for the single active Bicameral .nnova2 brain.
   Read-only training history is persisted by native Rust, including when JS sleeps. */
(()=>{
const $=id=>document.getElementById(id);
const invoke=(name,args={})=>window.__TAURI__?.core?.invoke(name,args)
    ?? Promise.reject(Error("Installed Neural Nova app required"));
let history=[],lastSeen=0,diagnostics=[],recording=false;
const fmt=n=>Number.isFinite(n)?n.toFixed(4):"Not sampled";
const set=(id,value)=>{const node=$(id);if(node)node.textContent=value;};
function log(level,message,data){
  let entry={time:new Date().toISOString(),level,message};
  if(data)entry.measurements=data;
  if(diagnostics.length>400)diagnostics.shift();
  diagnostics.push(entry);
  set("diagnosticCount",diagnostics.filter(x=>x.level==="error").length+
      " errors · "+diagnostics.length+" entries");
  set("diagnosticLog",diagnostics.map(x=>x.time+" ["+x.level+"] "+x.message).join("\n"));
  if(recording)set("deepLogStatus","Recording: "+diagnostics.length+" events (in this UI session). Native epoch records are saved separately.");
}
function selected(){
  const name=$("observerSession").value,idx=Number($("observerEpoch").value);
  return history.filter(e=>e.lesson===name)[idx];
}
function showEpoch(){
  const e=selected();
  if(!e){set("observerStatus","No saved Nova-2 candidate epochs yet.");return;}
  const records=history.filter(x=>x.lesson===e.lesson);
  set("observerStatus",records.length+" saved candidate epochs for "+e.lesson);
  set("observerDenseEffect",fmt(e.noDenseTv));
  set("observerExchangeEffect",fmt(e.noExchangeTv));
  set("observerBridgeRms","Not sampled");
  set("observerStep","Epoch "+e.epoch+" / step "+e.step);
  set("observerMetrics","Attempt "+e.attempt+" · "+e.focus+
    " · training loss "+fmt(e.trainLoss)+" · held-out "+fmt(e.heldoutLoss)+
    " · held-out token accuracy "+(100*e.heldoutAccuracy).toFixed(1)+"%"+
    " · learning rate "+e.learningRate.toPrecision(3)+
    (e.selected?" · IMPROVED":" · not selected")+
    (e.retentionPassed?" · retention eligible":" · retention regression blocked"));
  set("observerReplies",e.reply!=null?
    "Held-out prompt · first 12 generated tokens:\n"+e.reply+
    "\n\nTeacher expected response (scoring only):\n"+e.expected:
    "No autonomous generation sampled at this candidate epoch. Read-only generation and pathway probes are sampled after a lesson selects a checkpoint.");
  set("observerAblations",Number.isFinite(e.noDenseTv)?
    "Same weights, same prompt:\nFull vs disabled Dense, total variation "+fmt(e.noDenseTv)+
    "\nFull vs disabled reciprocal exchange, total variation "+fmt(e.noExchangeTv)+
    "\nThese are output changes, not proof of improved answers.":"No interconnect ablation was performed at this intermediate candidate epoch.");
  set("observerIntegrity",(e.validationOverlap||0)+" identical training/held-out prompts detected in this lesson's sampled groups.");
  set("observerClipping","Gradient norm and clipping telemetry are not exposed by this neural reference. No fabricated values.");
  set("observerActivity",JSON.stringify(e,null,2));
}
function refreshSelectors(){
  const session=$("observerSession"),epoch=$("observerEpoch");
  const oldSession=session.value,oldEpoch=epoch.value;
  let names=[...new Set(history.map(x=>x.lesson))];
  session.replaceChildren();
  names.forEach(n=>{const o=document.createElement("option");o.value=n;o.textContent=n;session.append(o);});
  if(names.includes(oldSession))session.value=oldSession;
  epoch.replaceChildren();
  let records=history.filter(x=>x.lesson===session.value);
  records.forEach((r,i)=>{
    const o=document.createElement("option");o.value=String(i);
    o.textContent="Attempt "+r.attempt+" · "+r.focus+" · epoch "+r.epoch;
    epoch.append(o);
  });
  if(oldSession===session.value&&Number(oldEpoch)<records.length&&oldEpoch!=="")epoch.value=oldEpoch;
  else if(records.length)epoch.value=String(records.length-1);
  showEpoch();
}
async function refreshObserver(){
  const found=await invoke("nova2_observer_read");
  if(!Array.isArray(found))throw Error("Invalid neural epoch archive");
  history=found;lastSeen=found.length;refreshSelectors();
}
async function refreshRehearsal(){
  const archive=await invoke("nova2_rehearsal_info");
  set("rehearsalBankStatus",archive.count+"/"+archive.capacity+
    " previous lesson sources archived"+(archive.count?" · "+archive.names.slice(-5).join(", "):""));
}
async function updateBenchmarks(){
  const archive=await invoke("nova2_test_archive");
  const sel=$("retentionBenchmark"),old=sel.value;sel.replaceChildren();
  const empty=document.createElement("option");empty.value="";empty.textContent="No prior-skill benchmark";
  sel.append(empty);
  for(const def of archive.definitions||[]){
    const o=document.createElement("option");o.value=def.id;
    o.textContent=def.name+" · "+def.cases.length+" questions";sel.append(o);
  }
  if([...(archive.definitions||[])].some(d=>d.id===old))sel.value=old;
  set("retentionStatus","Teacher-only saved tests may be used for independent scoring. Rehearsal retention uses previous lesson held-out loss.");
}
function guard(fn){return async()=>{try{await fn();}catch(e){log("error",String(e));console.error("Bicameral school UI",e);}};}
$("observerRefresh").addEventListener("click",guard(refreshObserver));
$("observerSession").addEventListener("change",refreshSelectors);
$("observerEpoch").addEventListener("change",showEpoch);
$("archivePriorLesson").addEventListener("click",guard(async()=>{
  const name=$("priorLessonName").value.trim(),mode=$("priorLessonMode").value,
        text=$("priorLessonData").value;
  await invoke("nova2_rehearsal_add",{name,mode,text});await refreshRehearsal();
  log("info","Archived prior source: "+name);
}));
$("clearRehearsal").addEventListener("click",guard(async()=>{
  if(!confirm("Clear the prior lesson archive? The trained .nnova2 weights are untouched."))return;
  await invoke("nova2_rehearsal_clear");await refreshRehearsal();
  log("info","Prior lesson sources cleared");
}));
$("priorLessonFile").addEventListener("change",guard(async()=>{
  const file=$("priorLessonFile").files[0];if(!file)return;
  if(file.size>50_000)throw Error("Prior TXT exceeds 50KB");
  $("priorLessonData").value=await file.text();
}));
$("clearDiagnostics").addEventListener("click",()=>{diagnostics=[];set("diagnosticLog","No diagnostics recorded in this session.");set("diagnosticCount","0 errors · 0 entries");});
$("copyDiagnostics").addEventListener("click",guard(async()=>navigator.clipboard.writeText($("diagnosticLog").textContent)));
$("startDeepLog").addEventListener("click",()=>{recording=true;$("startDeepLog").disabled=true;$("stopDeepLog").disabled=false;log("info","Started UI diagnostic recording");});
$("stopDeepLog").addEventListener("click",()=>{recording=false;$("startDeepLog").disabled=false;$("stopDeepLog").disabled=true;log("info","Stopped UI diagnostic recording");});
$("copyDeepLog").addEventListener("click",guard(async()=>navigator.clipboard.writeText(diagnostics.map(x=>JSON.stringify(x)).join("\n"))));
$("saveDeepLog").addEventListener("click",()=>{
  const blob=new Blob([diagnostics.map(x=>JSON.stringify(x)).join("\n")],{type:"application/x-ndjson"});
  const url=URL.createObjectURL(blob),a=document.createElement("a");a.href=url;
  a.download="nova2-training-ui-log.jsonl";a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
});
$("runDeepProbe").addEventListener("click",()=>set("deepProbeOutput","Deep model internals are not available from Claude's reference API. Use Training Activity for actual held-out scores and selected-checkpoint pathway ablations."));
const event=window.__TAURI__?.event;
if(event?.listen){
  void event.listen("nova2-progress",({payload:p})=>{
    if(!p)return;
    log("training",String(p.stage||"Training"),{
      lesson:p.lesson,epoch:p.epoch,optimizerStep:p.activeStep,
      heldoutLoss:p.heldoutLoss,heldoutAccuracy:p.heldoutAccuracy,
      trainLoss:p.loss,samplesCompleted:p.completed,planned:p.total});
    if(p.observations!==undefined&&p.observations!==lastSeen)
      void refreshObserver().catch(e=>log("error",String(e)));
  });
  void event.listen("nova2-done",()=>{log("info","Training worker finished");void refreshObserver();void refreshRehearsal();});
  void event.listen("nova2-error",({payload:e})=>log("error",String(e)));
  void event.listen("nova2-test-done",()=>void updateBenchmarks());
}
document.addEventListener("neural-nova-booted",()=>{void refreshObserver().catch(console.error);
  void refreshRehearsal().catch(console.error);void updateBenchmarks().catch(console.error);});
})();
