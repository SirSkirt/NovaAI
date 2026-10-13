"use strict";
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
  ?? Promise.reject(Error("Installed native Neural Nova required"));
let bundle=null,mode=null,active=false,stopping=false,at=0,success=0,failures=0;
let nativeZipBytes=null,nativeZipName=null;
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
  nativeZipBytes=null;nativeZipName=null;bundle=null;readyButtons();
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
  bundle=result;mode=currentMode();at=0;success=0;failures=0;
  $("massLessonsLog").textContent="";
  result.lessons.forEach((lesson,i)=>addLog((i+1)+". "+lesson.name+" · "+
    lesson.trainingSamples+" training / "+lesson.heldoutSamples+" held-out targets"));
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
async function runQueue(){
  if(active||!bundle)throw Error("Validate a ZIP before starting.");
  if(mode!==currentMode())throw Error("Lesson type changed. Validate the ZIP again.");
  const started=await invoke("hybrid_status");
  if(started.training)throw Error("Another training session is already running.");
  if(!confirm("Teach "+bundle.lessons.length+" "+mode+" lessons one at a time? Each lesson updates the ONE active joint neural checkpoint."))return;
  const cfg={
    adaptiveLearningRate:$("adaptiveTraining").checked,
    earlyStopping:$("earlyStopping").checked,
    learningRate:Number($("trainingRate").value),
    patience:6,retentionTestId:$("retentionBenchmark").value||null,
    rehearsal:$("rehearsalEnabled").checked,
    protectPrevious:$("retentionGuardEnabled").checked,
    retentionLossTolerance:Number($("retentionTolerance").value)
  };
  const epochs=Number($("trainingEpochs").value);
  if(!Number.isInteger(epochs)||epochs<1||epochs>60)throw Error("Choose 1–60 epochs.");
  active=true;stopping=false;at=0;success=0;failures=0;
  readyButtons();
  try{
    await window.NovaTrainingBackground?.start();
    for(const lesson of bundle.lessons){
      if(stopping)break;
      view("Lesson "+(at+1)+"/"+bundle.lessons.length+": "+lesson.name+" · training");
      addLog("START "+(at+1)+"/"+bundle.lessons.length+": "+lesson.name);
      try{
        // Ensure progress and completion listeners are installed before
        // starting native Rust work, including tiny lessons.
        const watch=await nativeDone();
        try{await invoke("hybrid_train",{lesson:{mode,text:lesson.text,epochs},options:cfg});}
        catch(e){watch.cancel();throw e;}
        const result=await watch.finished;
        if(result.cancelled||stopping){
          stopping=true;addLog("STOPPED: "+lesson.name+" · best available checkpoint kept");
          break;
        }
        if(!result.bestEpoch){
          failures++;stopping=true;
          addLog("NO ACCEPTED EPOCH "+lesson.name+
            " · prior brain restored; stopped queue to avoid silently skipping a lesson");
          break;
        }
        success++;
        addLog("COMPLETE "+lesson.name+" · selected epoch "+result.bestEpoch);
      }catch(error){
        failures++;
        addLog("FAILED "+lesson.name+" · "+String(error));
        stopping=true;break;
      }finally{at++;}
    }
  }finally{
    active=false;readyButtons();
    view((stopping?"Stopped":"Finished")+" · "+success+" taught, "+failures+
      " failed, "+(bundle.lessons.length-at)+" not started. Checkpoint saved after each accepted lesson.");
  }
}
async function safe(task){
  try{await task();}
  catch(error){view("Error: "+String(error));console.error("Nova bulk lesson queue",error);}
}
$("massLessonZip").addEventListener("change",()=>{
  nativeZipBytes=null;nativeZipName=null;
  bundle=null;readyButtons();view("ZIP selected. Validate before starting.");
});
$("nativeMassLessonZip").addEventListener("click",()=>safe(pickNativeZip));
$("lessonMode").addEventListener("change",()=>{
  if(!active){bundle=null;readyButtons();view("Lesson mode changed; revalidate ZIP.");}
});
$("previewMassLessons").addEventListener("click",()=>safe(preview));
$("startMassLessons").addEventListener("click",()=>safe(runQueue));
$("stopMassLessons").addEventListener("click",()=>safe(async()=>{
  stopping=true;
  view("Stopping current lesson. The trainer will restore its best checkpoint.");
  await invoke("hybrid_stop");
}));
$("stopTraining").addEventListener("click",()=>{if(active)stopping=true;});
window.NovaMassLearning={get active(){return active;}};
readyButtons();
})();