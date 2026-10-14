"use strict";
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
  ?? Promise.reject(Error("Installed native Neural Nova required"));
let bundle=null,mode=null,selectedMode=null,active=false,stopping=false,at=0,success=0,skipped=0,failures=0;
let nativeZipBytes=null,nativeZipName=null,bundleBytes=null,lastQueueLesson=0,lastQueueCompleted=0,lastQueueSkipped=0,lastQueueAttempt=0;
const android=/Android/i.test(navigator.userAgent);
if(android){
  // Android WebView File.arrayBuffer() can throw NotReadableError for files
  // selected through Storage Access Framework. Let the native plugins hold
  // the transient document grant and read the bytes immediately.
  $("browserMassLessonPicker").hidden=true;
  $("browserMassLessonPicker").style.display="none";
  $("massLessonZip").hidden=true;
  $("nativeMassLessonZip").hidden=false;
}
const view=(message)=>{$("massLessonsStatus").textContent=message;};
function addLog(message){
  $("massLessonsLog").textContent+=(($("massLessonsLog").textContent==="No bulk lessons loaded.")?"":"\n")+message;
  $("massLessonsLog").scrollTop=$("massLessonsLog").scrollHeight;
}
function readyButtons(){
  $("startMassLessons").disabled=!bundle||active;
  $("previewMassLessons").disabled=active;
  $("stopMassLessons").disabled=!active;
}
const currentMode=()=>$("lessonMode").value;
async function pickNativeZip(){
  if(active)throw Error("Finish the current lesson before selecting another ZIP.");
  const dialog=window.__TAURI__?.dialog;
  const fs=window.__TAURI__?.fs;
  if(!dialog?.open||!fs?.readFile)
    throw Error("Native Android file picker unavailable; update to the new Neural Nova build.");
  // Android returns a content:// URI, not a normal filesystem path.
  // Tauri's dialog plugin grants access to selected documents for plugin-fs.
  const picked=await dialog.open({multiple:false,directory:false,pickerMode:"document"});
  // Do not rely on Android providers advertising a particular ZIP MIME type.
  // The shared Rust ZIP validator checks the actual archive bytes.
  if(picked===null || picked===undefined)return; // Android Back / Cancel.
  if(typeof picked!=="string")throw Error("Unexpected result from Android document picker.");
  nativeZipBytes=null;nativeZipName=null;bundleBytes=null;bundle=null;readyButtons();
  view("Reading ZIP with Android's native document access…");
  let data;
  try{data=await fs.readFile(picked);}
  catch(error){
    throw Error("Android could not read this document: "+String(error)+
      ". Try moving the ZIP into Downloads and selecting it from Files.");
  }
  if(!(data instanceof Uint8Array))data=new Uint8Array(data);
  if(data.byteLength===0||data.byteLength>4*1024*1024)
    throw Error("Selected ZIP must be between 1 byte and 4 MiB.");
  nativeZipBytes=data;
  nativeZipName=picked.split("/").pop()||"Selected ZIP";
  view("ZIP read successfully from Android Files ("+data.byteLength+
    " bytes). Tap Validate ZIP.");
}
async function preview(){
  if(active)throw Error("Finish the current lesson queue first.");
  let data=nativeZipBytes;
  if(!data){
    const file=$("massLessonZip").files[0];
    if(!file)throw Error("Select a .zip containing .txt lessons.");
    if(!file.name.toLowerCase().endsWith(".zip")||file.size>4*1024*1024)
      throw Error("Choose a .zip smaller than 4 MiB.");
    try{data=new Uint8Array(await file.arrayBuffer());}
    catch(error){
      if(error?.name==="NotReadableError")
        throw Error("The WebView cannot read this selection. Use Android's native ZIP picker instead.");
      throw error;
    }
  }
  if(data.byteLength===0||data.byteLength>4*1024*1024)
    throw Error("Selected ZIP must be between 1 byte and 4 MiB.");
  const result=await invoke("hybrid_zip_preview",{bytes:Array.from(data),mode:currentMode()});
  bundle=result;bundleBytes=data;mode=result.mode;selectedMode=currentMode();at=0;success=0;skipped=0;failures=0;
  $("massLessonsLog").textContent="";
  result.lessons.forEach((lesson,i)=>addLog((i+1)+". "+lesson.name+" · "+
    lesson.mode+" · "+lesson.trainingSamples+" training / "+lesson.heldoutSamples+" held-out targets"));
  view("Validated "+bundle.lessons.length+" "+mode+" lessons ("+result.totalBytes+
    " uncompressed bytes). Ready for sequential teaching.");
  readyButtons();
}
async function nativeDone(){
  const event=window.__TAURI__?.event;
  if(!event?.listen)throw Error("Native events are required for sequential learning.");
  let resolveDone,rejectDone,settled=false;
  const finished=new Promise((resolve,reject)=>{resolveDone=resolve;rejectDone=reject;});
  let unlistenA=()=>{},unlistenB=()=>{};
  const cleanup=()=>{unlistenA();unlistenB();};
  unlistenA=await event.listen("hybrid-done",({payload})=>{
    if(settled)return;settled=true;cleanup();resolveDone(payload);
  });
  try{
    unlistenB=await event.listen("hybrid-error",({payload})=>{
      if(settled)return;settled=true;cleanup();rejectDone(Error(String(payload)));
    });
  }catch(e){unlistenA();throw e;}
  return {finished,cancel:()=>{
    if(settled)return;
    settled=true;cleanup();
    // Finish the promise even when native training could not start.
    resolveDone({cancelled:true,bestEpoch:0});
  }};
}
// This is an Android-safe native Rust queue: JS events are only for display.
// Rust advances between lessons even if the WebView is paused with screen off.
function showQueueStatus(p){
  if(!p)return;
  active=!!p.active;
  at=p.current||0;success=p.completed||0;skipped=p.skipped||0;failures=p.failed||0;
  stopping=!!p.stopping;
  readyButtons();
  if(p.current>lastQueueLesson){
    addLog("START "+p.current+"/"+p.total+": "+p.lesson);
    lastQueueLesson=p.current;
    lastQueueAttempt=0;
  }
  if(p.attempt>lastQueueAttempt){
    const strategy=p.retryStrategy||"standard";
    addLog("ATTEMPT "+p.attempt+" · "+strategy+" · "+p.lesson);
    lastQueueAttempt=p.attempt;
  }
  if(p.completed>lastQueueCompleted){
    addLog("ACCEPTED "+p.completed+" lesson(s); best checkpoint retained");
    lastQueueCompleted=p.completed;
  }
  if((p.skipped||0)>lastQueueSkipped){
    addLog("SKIPPED "+p.skipped+" lesson(s): retries exhausted or no eligible improvement; accepted baseline retained");
    lastQueueSkipped=p.skipped;
  }
  const evidence=p.learningEvidence;
  const learned=evidence?(" · practice "+evidence.practiceCorrect+"/"+evidence.practiceChecked+
    ", held-out "+evidence.heldoutCorrect+"/"+evidence.heldoutChecked+
    " · "+evidence.finding):"";
  view("Native background ZIP queue · "+(p.stage||"Working")+
    " · lesson "+(p.current||0)+"/"+(p.total||0)+
    " · "+success+" accepted · "+skipped+" skipped"+
    learned+(p.error?" · ERROR: "+p.error:"")+
    (p.stopping?" · stopping":""));
}
async function recoverQueue(){
  try{
    const p=await invoke("hybrid_zip_status");
    showQueueStatus(p);
    if(!p.active && p.stage && p.stage!=="Idle"){
      // The Rust worker may have completed while JS was suspended.
      await window.NovaTrainingBackground?.stop();
    }
  }catch(e){console.warn("Neural Nova ZIP queue status",e);}
}
async function runQueue(){
  if(active||!bundle||!bundleBytes)throw Error("Validate a ZIP before starting.");
  if(selectedMode!==currentMode())throw Error("Legacy fallback type changed. Validate the ZIP again.");
  const started=await invoke("hybrid_status");
  if(started.training)throw Error("Another training session is already running.");
  const foundationScope=$("foundationZipScope").value;
  if(!confirm("Teach "+bundle.lessons.length+" "+mode+
    " lessons as ONE native background session? "+
    ($("foundationTrainingEnabled").checked?
      ("Foundation v2: "+(foundationScope==="entireZip"?"entire ZIP":"first five lessons")+
      ", up to 3 recovery retries per foundation lesson. "):"")+
    "Existing accepted checkpoints are retained between lessons."))return;
  const cfg={
    adaptiveLearningRate:$("adaptiveTraining").checked,
    earlyStopping:$("earlyStopping").checked,
    learningRate:Number($("trainingRate").value),
    patience:6,retentionTestId:$("retentionBenchmark").value||null,
    rehearsal:$("rehearsalEnabled").checked,
    protectPrevious:$("retentionGuardEnabled").checked,
    retentionLossTolerance:Number($("retentionTolerance").value),
    foundationTraining:$("foundationTrainingEnabled").checked,
    foundationZipScope:foundationScope
  };
  const epochs=Number($("trainingEpochs").value);
  if(!Number.isInteger(epochs)||epochs<1||epochs>60)throw Error("Choose 1–60 epochs.");
  active=true;stopping=false;at=0;success=0;skipped=0;failures=0;
  lastQueueLesson=0;lastQueueCompleted=0;lastQueueSkipped=0;lastQueueAttempt=0;
  readyButtons();
  try{
    await window.NovaTrainingBackground?.start();
    // Native validates the ZIP again; it never trusts the JS preview.
    await invoke("hybrid_zip_train",{bytes:Array.from(bundleBytes),
      mode:selectedMode,epochs,options:cfg});
    view("Native ZIP education started. You can switch apps or turn the screen off. "+
      "Android may still stop the process under extreme battery/memory pressure.");
    await recoverQueue();
  }catch(e){
    active=false;readyButtons();
    await window.NovaTrainingBackground?.stop();
    throw e;
  }
}
async function safe(task){
  try{await task();}
  catch(error){view("Error: "+String(error));console.error("Nova bulk lesson queue",error);}
}
$("massLessonZip").addEventListener("change",()=>{
  nativeZipBytes=null;nativeZipName=null;bundleBytes=null;
  bundle=null;readyButtons();view("ZIP selected. Structured lesson blocks are detected automatically. Validate before starting.");
});
$("nativeMassLessonZip").addEventListener("click",()=>safe(pickNativeZip));
$("lessonMode").addEventListener("change",()=>{
  if(!active){bundle=null;readyButtons();view("Legacy fallback changed; structured lesson blocks will still auto-detect. Revalidate ZIP.");}
});
$("previewMassLessons").addEventListener("click",()=>safe(preview));
$("startMassLessons").addEventListener("click",()=>safe(runQueue));
$("stopMassLessons").addEventListener("click",()=>safe(async()=>{
  stopping=true;
  view("Stopping the native ZIP session after the current lesson; the best eligible checkpoint will be restored.");
  await invoke("hybrid_zip_stop");
}));
$("stopTraining").addEventListener("click",()=>{if(active)stopping=true;});
window.NovaMassLearning={get active(){return active;}};
const events=window.__TAURI__?.event;
if(events?.listen){
  void events.listen("hybrid-zip-progress",({payload})=>showQueueStatus(payload))
    .catch(e=>console.error("ZIP status events",e));
  void events.listen("hybrid-zip-done",({payload})=>{
    showQueueStatus(payload);
    addLog(payload.error?"FAILED: "+payload.error:
      payload.stopping?"STOPPED: accepted checkpoints retained":
      "COMPLETE: "+payload.completed+" accepted, "+(payload.skipped||0)+
      " skipped (no eligible improvement; baseline retained)."+
      (payload.completed===0?" No lessons were learned in this ZIP run.":""));
    void window.NovaTrainingBackground?.stop();
  }).catch(e=>console.error("ZIP completion events",e));
}
document.addEventListener("neural-nova-booted",()=>{void recoverQueue();});
document.addEventListener("visibilitychange",()=>{
  if(!document.hidden)void recoverQueue();
});
readyButtons();
})();