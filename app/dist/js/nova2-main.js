"use strict";
/* Unified teacher workspace: the ONLY active mind is the Claude Bicameral Nova2 Rust engine.
   No legacy hybrid model commands, checkpoints, or runtime states are used. */
(()=>{
const $=id=>document.getElementById(id);
const invoke=(name,args={})=>window.__TAURI__?.core?.invoke(name,args)??Promise.reject(Error("Installed Nova application required"));
const safe=async fn=>{try{return await fn();}catch(e){console.error("Neural Nova Bicameral",e);$("trainingStage").textContent="Error: "+String(e);$("brainStateText").textContent="Error: "+String(e);}};
const changeTab=name=>{
  for(const n of document.querySelectorAll(".nav-tab")){let ok=n.dataset.tab===name;n.classList.toggle("selected",ok);n.setAttribute("aria-selected",String(ok));n.tabIndex=ok?0:-1;}
  for(const p of document.querySelectorAll(".tab-pane"))p.hidden=p.id!=="pane-"+name;
};
document.querySelectorAll(".nav-tab").forEach(n=>{
  n.addEventListener("click",()=>changeTab(n.dataset.tab));
  n.addEventListener("keydown",e=>{if(!["ArrowRight","ArrowLeft","Home","End"].includes(e.key))return;
    e.preventDefault();let a=[...document.querySelectorAll(".nav-tab")],i=a.indexOf(n);
    let to=e.key==="Home"?0:e.key==="End"?a.length-1:(i+(e.key==="ArrowRight"?1:a.length-1))%a.length;
    a[to].focus();changeTab(a[to].dataset.tab);
  });
});
let running=false,history=[],lastQuestion="",candidate=false,zipBytes=null,zipBundle=null;
let importedTarget=null;
let lessonBank={};
function printStatus(s){
  if(!s)return;
  running=s.training;
  $("totalParams").textContent=Number(s.parameters).toLocaleString();
  $("transformerParams").textContent=Number(s.transformerParameters).toLocaleString();
  $("denseParams").textContent=Number(s.denseParameters).toLocaleString();
  $("trainingSteps").textContent=Number(s.training?s.activeStep??s.step:s.step).toLocaleString();
  $("brainStateText").textContent=s.training?"Nova is learning":s.error?"Nova error":"Nova ready";
  $("brainState").className="status-pill"+(s.training?" running":s.error?" error":"");
  $("trainingStage").textContent=s.error||s.stage||"Ready";
  $("checkpointText").textContent=s.trained?".nnova2 weights saved":"Fresh .nnova2 brain";
  $("memoryStatus").textContent=history.length+"/12 turns · "+s.words+" word units";
  $("heldoutLoss").textContent=Number.isFinite(s.heldoutLoss)?s.heldoutLoss.toFixed(4):"—";
  $("heldoutAccuracy").textContent=Number.isFinite(s.heldoutAccuracy)?(100*s.heldoutAccuracy).toFixed(1)+"%":"—";
  $("lessonSplit").textContent="Training / held-out: "+(s.trainLessons||0)+" / "+(s.validationLessons||0)+" answer groups";
  $("trainingSpeed").textContent=s.tokensPerSecond>0?"Throughput: "+s.tokensPerSecond.toFixed(2)+" samples/s":"Throughput: —";
  $("bestEpoch").textContent="Best epoch: "+(s.bestEpoch||"baseline")+(Number.isFinite(s.bestHeldoutLoss)?" · "+s.bestHeldoutLoss.toFixed(4):"");
  $("trainingProgress").textContent=s.training?"Epoch "+s.epoch+"/"+s.epochs+" · "+s.completed+"/"+s.total:"Last lesson: "+(s.lesson||"none");
  let f=s.total?Math.min(100,100*s.completed/s.total):0;
  $("trainingFill").style.width=f.toFixed(1)+"%";$("trainingPercent").textContent=f.toFixed(1)+"%";
  $("startTraining").disabled=running;$("stopTraining").disabled=!running;$("resetBrain").disabled=running;
  $("sendMessage").disabled=running;$("startMassLessons").disabled=running||!zipBundle;$("stopMassLessons").disabled=!running;
  $("targetedStart").disabled=running||s.candidateReady;
  $("targetedStop").disabled=!running;
  $("targetedAccept").disabled=running||!s.candidateReady;
  $("targetedDiscard").disabled=running||!s.candidateReady;
  candidate=!!s.candidateReady;
  $("targetedStage").textContent=s.candidateReady?"Ready to accept "+s.candidateFocus+" candidate; epoch "+s.candidateEpoch:s.stage||"No active targeted candidate";
  $("targetedCandidateLoss").textContent=Number.isFinite(s.candidateLoss)?s.candidateLoss.toFixed(4):"—";
  $("targetedProgress").textContent=s.training?"Training epoch "+s.epoch+"/"+s.epochs:s.candidateReady?"Candidate ready":"No active experiment";
  $("targetedPercent").textContent=f.toFixed(1)+"%";$("targetedFill").style.width=f.toFixed(1)+"%";
  $("foundationTrainingStatus").textContent=$("foundationTrainingEnabled").checked?"Compare four pathway candidates; select lowest independent held-out loss.":"Standard joint neural training.";
}
async function refresh(){printStatus(await invoke("nova2_status"));}
function mode(){return $("lessonMode").value;}
function setText(v){$("lessonData").value=v;$("lessonCount").textContent=v.length+" characters";}
function saveText(){lessonBank[mode()]=$("lessonData").value;try{localStorage.setItem("nova2-course-"+mode(),lessonBank[mode()]);}catch(_){}}
$("lessonMode").addEventListener("change",e=>{setText(lessonBank[e.target.value]||"");$("lessonHint").textContent="Existing structured Dialogue, Understanding, Multi-turn Conversation and Reasoning lessons are supported.";});
$("lessonData").addEventListener("input",()=>{saveText();$("lessonCount").textContent=$("lessonData").value.length+" characters";});
$("clearLesson").addEventListener("click",()=>{setText("");saveText();});
$("sampleLesson").addEventListener("click",()=>{setText("[Dialogue Lesson]\nGreetings and friendly conversation.\n[examples]\nhello  Hello!\ngood morning  Good morning!\n[/examples]\n[/Dialogue Lesson]");$("lessonMode").value="unified";saveText();});
async function readLocalFile(input,max=50000){let f=input.files?.[0];if(!f)return null;if(f.size>max)throw Error("File is too large");return f.text();}
$("lessonFile").addEventListener("change",()=>safe(async()=>{const text=await readLocalFile($("lessonFile"));if(text!==null){setText(text);saveText();}}));
if(/Android/i.test(navigator.userAgent)){
  $("browserLessonFileLabel").hidden=true;$("lessonFile").hidden=true;$("nativeLessonFile").hidden=false;
  $("browserMassLessonPicker").hidden=true;$("massLessonZip").hidden=true;$("nativeMassLessonZip").hidden=false;
}
async function androidBytes(){
  let d=window.__TAURI__?.dialog,fs=window.__TAURI__?.fs;
  if(!d?.open||!fs?.readFile)throw Error("Android Files document picker is unavailable");
  let uri=await d.open({multiple:false,directory:false,pickerMode:"document"});
  if(!uri)return null;
  let v=await fs.readFile(uri);return v instanceof Uint8Array?v:new Uint8Array(v);
}
$("nativeLessonFile").addEventListener("click",()=>safe(async()=>{
  const bytes=await androidBytes();if(!bytes)return;if(bytes.length>50000)throw Error("TXT is over 50KB");
  const text=new TextDecoder().decode(bytes);if(/[^\x00-\x7f]/.test(text))throw Error("Training text must be ASCII");
  setText(text);saveText();
}));
$("addLesson").addEventListener("click",()=>{
  const q=$("teacherQuestion").value.trim(),a=$("teacherAnswer").value.trim();
  if(!q||!a)return;
  const prior=lessonBank.dialogue||"";
  lessonBank.dialogue=prior+(prior?"\n":"")+q+"\t"+a;
  $("lessonMode").value="dialogue";setText(lessonBank.dialogue);saveText();
  $("teacherQuestion").value="";$("teacherAnswer").value="";
});
function educationOptions(){return{
 foundationTraining:$("foundationTrainingEnabled").checked,
 foundationZipScope:$("foundationZipScope").value,
 adaptiveLearningRate:$("adaptiveTraining").checked,
 earlyStopping:$("earlyStopping").checked,
 protectPrevious:$("retentionGuardEnabled").checked,
 rehearsal:$("rehearsalEnabled").checked,
 retentionLossTolerance:Number($("retentionTolerance").value)
};}
function trainArgs(requireText=true){const text=$("lessonData").value,epochs=Number($("trainingEpochs").value),learningRate=Number($("trainingRate").value);
 if(requireText&&!text.trim())throw Error("Paste a lesson");if(!(epochs>=1&&epochs<=60))throw Error("Choose 1-60 epochs");
 if(!Number.isFinite(learningRate)||learningRate<=0)throw Error("Invalid learning rate");
 return {mode:mode(),text,epochs,learningRate,options:educationOptions()};}
async function trainingStart(name,args){
 await window.NovaTrainingBackground?.start();
 try{printStatus(await invoke(name,args));}catch(e){await window.NovaTrainingBackground?.stop();throw e;}
}
$("startTraining").addEventListener("click",()=>safe(async()=>{const a=trainArgs();await trainingStart("nova2_train",a);}));
$("stopTraining").addEventListener("click",()=>safe(async()=>{await invoke("nova2_stop");await refresh();}));
$("resetBrain").addEventListener("click",()=>safe(async()=>{
 if(!confirm("Erase Nova's active .nnova2 training and start fresh? Export first if you want a backup."))return;
 printStatus(await invoke("nova2_reset"));history=[];renderHistory();
}));
$("foundationTrainingEnabled").addEventListener("change",()=>safe(refresh));
$("massLessonZip").addEventListener("change",async()=>{zipBundle=null;zipBytes=null;$("startMassLessons").disabled=true;$("massLessonsStatus").textContent="ZIP selected. Validate first.";});
$("nativeMassLessonZip").addEventListener("click",()=>safe(async()=>{zipBytes=await androidBytes();zipBundle=null;$("massLessonsStatus").textContent=zipBytes?"ZIP selected; validate it.":"No ZIP selected";}));
$("previewMassLessons").addEventListener("click",()=>safe(async()=>{
 if(!zipBytes){let f=$("massLessonZip").files?.[0];if(!f)throw Error("Select a ZIP");zipBytes=new Uint8Array(await f.arrayBuffer());}
 if(!zipBytes.length||zipBytes.length>4*1024*1024)throw Error("ZIP must be 1B to 4MiB");
 zipBundle=await invoke("nova2_zip_preview",{bytes:Array.from(zipBytes),mode:"auto"});
 $("massLessonsStatus").textContent=zipBundle.lessons.length+" lessons validated.";
 $("massLessonsLog").textContent=zipBundle.lessons.map((x,i)=>(i+1)+". "+x.name+" ("+x.mode+")").join("\n");
 $("startMassLessons").disabled=false;
}));
$("startMassLessons").addEventListener("click",()=>safe(async()=>{
 if(!zipBundle||!zipBytes)throw Error("Validate ZIP first");
 const args=trainArgs(false);delete args.text;
 if(!confirm("Teach "+zipBundle.lessons.length+" ZIP lessons to Nova's .nnova2 brain?"))return;
 await trainingStart("nova2_zip_train",{bytes:Array.from(zipBytes),mode:"auto",epochs:args.epochs,learningRate:args.learningRate,options:args.options});
}));
$("stopMassLessons").addEventListener("click",()=>safe(async()=>{await invoke("nova2_stop");await refresh();}));
const ctx=$("chatTranscript");function bubble(role,v){
 let el=document.createElement("div");el.className="chat-bubble "+(role==="user"?"user":"nova");
 const name=document.createElement("strong");name.textContent=role==="user"?"You":"Nova";
 const text=document.createElement("div");text.textContent=v||"(No reply)";el.append(name,text);ctx.append(el);ctx.scrollTop=ctx.scrollHeight;
}
function renderHistory(){ctx.replaceChildren();for(const t of history){bubble("user",t.user);bubble("nova",t.assistant);}}
$("sendMessage").addEventListener("click",()=>safe(async()=>{
 const message=$("chatMessage").value.trim();if(!message)throw Error("Enter a message");
 lastQuestion=message;
 const r=await invoke("nova2_chat",{message,maxTokens:Number($("chatLength").value),
    temperature:Number($("chatTemperature").value),sampling:$("chatSampling").value,
    contextTokens:Number($("chatContext").value)});
 $("chatMessage").value="";
 history=await invoke("nova2_conversation");renderHistory();
 $("correctReply").disabled=false;
 $("chatContextReport").textContent="KV cache: "+r.generatedTokens+" generated tokens; "+Number(r.cacheAttentionKeys).toLocaleString()+" attention keys read. Step "+r.step+".";
 await refresh();
}));
$("chatMessage").addEventListener("keydown",e=>{if(e.key==="Enter"){$("sendMessage").click();}});
$("clearChat").addEventListener("click",()=>safe(async()=>{await invoke("nova2_new_chat");history=[];renderHistory();await refresh();}));
$("correctReply").addEventListener("click",()=>{changeTab("teach");$("lessonMode").value="dialogue";setText(lessonBank.dialogue||"");
 $("teacherQuestion").value=lastQuestion;$("teacherAnswer").value="";$("teacherQuestion").closest("details").open=true;$("teacherAnswer").focus();
});
$("chatTemperature").addEventListener("input",()=>{$("chatTempValue").textContent=$("chatTemperature").value;});
$("compareContext").addEventListener("click",()=>safe(async()=>{
 const msg=lastQuestion||$("chatMessage").value.trim();if(!msg)throw Error("Enter a question first");
 const result=await invoke("nova2_compare_context",{message:msg,maxTokens:Number($("chatLength").value)});
 $("chatComparison").hidden=false;$("chatComparison").textContent="128 tokens: "+result.short+"\n256 tokens: "+result.long;
}));
function targetStatus(s){
 printStatus(s);
 $("targetedWarnings").textContent=s.candidateReady?
   "A complete staged "+s.candidateFocus+" model is ready. Only acceptance changes live Nova.":
   "The active .nnova2 brain stays untouched until you accept a candidate.";
}
$("targetedCopyTeach").addEventListener("click",()=>{$("targetedLessonData").value=$("lessonData").value;});
$("targetedLessonFile").addEventListener("change",()=>safe(async()=>{let v=await readLocalFile($("targetedLessonFile"));if(v!==null)$("targetedLessonData").value=v;}));
$("targetedStart").addEventListener("click",()=>safe(async()=>{
 if(candidate)throw Error("Accept or discard your candidate first");
 if($("targetedSource").value==="imported"&&!importedTarget)throw Error("Stage an .nnova2 checkpoint first");
 const lessonText=$("targetedLessonData").value.trim();
 if(!lessonText)throw Error("Provide targeted lesson material");
 await trainingStart("nova2_targeted_train",{focusName:$("targetedFocus").value,
  skillName:$("targetedSkill").value,lessonText,epochs:Number($("targetedEpochs").value),
  learningRate:Number($("targetedRate").value),source:$("targetedSource").value});
 await refresh();
}));
$("targetedStop").addEventListener("click",()=>safe(async()=>{await invoke("nova2_stop");await refresh();}));
$("targetedDiscard").addEventListener("click",()=>safe(async()=>{targetStatus(await invoke("nova2_targeted_discard"));}));
$("targetedAccept").addEventListener("click",()=>safe(async()=>{
 if(!confirm("Install the entire focused-trained .nnova2 candidate? Your current brain will be backed up."))return;
 targetStatus(await invoke("nova2_targeted_accept"));history=[];renderHistory();
}));
$("targetedReturnTeach").addEventListener("click",()=>changeTab("teach"));
$("targetedFocus").addEventListener("change",()=>{$("targetedFocusHint").textContent="Only "+$("targetedFocus").value+" weights receive gradients; all three neural pathways participate in the forward pass.";});
$("targetedImport").addEventListener("click",()=>safe(async()=>{
 let data=importedTarget;
 if(!data){const f=$("targetedImportFile").files?.[0];if(!f)throw Error("Choose a .nnova2 checkpoint first");data=new Uint8Array(await f.arrayBuffer());}
 printStatus(await invoke("nova2_targeted_stage",{bytes:Array.from(data)}));
 importedTarget=data;$("targetedSource").value="imported";
 $("targetedImportStatus").textContent="Imported .nnova2 staged independently; the active brain is unchanged.";
}));
const CK=256*1024;let androidCheckpoint=null;
if(/Android/i.test(navigator.userAgent)){
 $("importCheckpointFileLabel").hidden=true;$("importCheckpointFile").hidden=true;$("importCheckpointNative").hidden=false;
 $("targetedBrowserImportLabel").hidden=true;$("targetedImportFile").hidden=true;$("targetedNativeImport").hidden=false;
}
$("importCheckpointNative").addEventListener("click",()=>safe(async()=>{
 androidCheckpoint=await androidBytes();
 if(androidCheckpoint)$("checkpointTransferStatus").textContent="Loaded "+androidCheckpoint.length+" bytes.";
}));
$("targetedNativeImport").addEventListener("click",()=>safe(async()=>{
 importedTarget=await androidBytes();
 $("targetedImportStatus").textContent=importedTarget?"Selected .nnova2 file (not yet staged).":"No file selected.";
}));
$("importCheckpointFile").addEventListener("change",()=>{
 $("importCheckpoint").disabled=!$("importCheckpointFile").files.length;
});
$("exportCheckpoint").addEventListener("click",()=>safe(async()=>{
 let info=await invoke("nova2_checkpoint_export_begin");
 try{
  let bytes=new Uint8Array(info.bytes),at=0;
  while(at<bytes.length){let part=await invoke("nova2_checkpoint_export_chunk",{offset:at});
   if(!part?.length)throw Error("Incomplete checkpoint download");
   bytes.set(part,at);at+=part.length;
  }
  if(/Android/i.test(navigator.userAgent)){
    let uri=await window.__TAURI__?.dialog?.save({defaultPath:"nova-step-"+info.step+".nnova2",
      filters:[{name:"Nova-2 checkpoint",extensions:["nnova2"]}]});
    if(!uri)return;
    let result=await invoke("nova2_checkpoint_export_android",{destinationUri:uri});
    $("checkpointTransferStatus").textContent="Verified Android .nnova2 export: "+result.bytes+" bytes.";
  }else{
    let blob=new Blob([bytes],{type:"application/octet-stream"});
    let url=URL.createObjectURL(blob),a=document.createElement("a");a.href=url;a.download="nova-step-"+info.step+".nnova2";a.click();setTimeout(()=>URL.revokeObjectURL(url),5000);
    $("checkpointTransferStatus").textContent="Exported "+bytes.length+" bytes.";
  }
 }finally{await invoke("nova2_checkpoint_export_cancel").catch(()=>{});}
}));
$("importCheckpoint").addEventListener("click",()=>safe(async()=>{
 let data=androidCheckpoint||new Uint8Array(await $("importCheckpointFile").files[0].arrayBuffer());
 if(!data?.length||data.length>24*1024*1024)throw Error("Invalid .nnova2 file");
 if(!confirm("Import this .nnova2 checkpoint as Nova's active brain? The current brain will be backed up."))return;
 await invoke("nova2_checkpoint_import_begin",{bytes:data.length});
 try{
  for(let n=0;n<data.length;n+=CK)await invoke("nova2_checkpoint_import_append",{chunk:Array.from(data.slice(n,n+CK))});
  printStatus(await invoke("nova2_checkpoint_import_finish"));history=[];renderHistory();
  $("checkpointTransferStatus").textContent="Active Nova .nnova2 checkpoint restored.";
 }finally{await invoke("nova2_checkpoint_import_cancel").catch(()=>{});}
}));
function stopBackground(){void window.NovaTrainingBackground?.stop().catch(()=>{});}
const events=window.__TAURI__?.event;
if(events?.listen){
 void events.listen("nova2-progress",({payload:s})=>{
  printStatus(s);let box=$("epochHistory");box.textContent=String(s.stage)+"\n"+box.textContent.slice(0,5000);
 });
 void events.listen("nova2-done",()=>{void refresh();stopBackground();});
 void events.listen("nova2-error",({payload:e})=>{$("trainingStage").textContent=String(e);void refresh();stopBackground();});
}
for(const m of ["unified","dialogue","text","conversation","reasoning"]){
 try{lessonBank[m]=localStorage.getItem("nova2-course-"+m)||"";}catch(_){lessonBank[m]="";}
}
setText(lessonBank.unified);
void safe(async()=>{printStatus(await invoke("nova2_boot"));history=await invoke("nova2_conversation");renderHistory();document.dispatchEvent(new Event("neural-nova-booted"));});
setInterval(()=>{if(running)void refresh().catch(console.error);},1200);
})();
