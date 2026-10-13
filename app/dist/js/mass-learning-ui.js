"use strict";
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
  ?? Promise.reject(Error("Installed native Neural Nova required"));
let bundle=null,mode=null,active=false,stopping=false,at=0,success=0,failures=0;
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
async function preview(){
  if(active)throw Error("Finish the current lesson queue first.");
  const file=$("massLessonZip").files[0];
  if(!file)throw Error("Select a .zip containing .txt lessons.");
  if(!file.name.toLowerCase().endsWith(".zip")||file.size>4*1024*1024)
    throw Error("Choose a .zip smaller than 4 MiB.");
  const bytes=Array.from(new Uint8Array(await file.arrayBuffer()));
  const result=await invoke("hybrid_zip_preview",{bytes,mode:currentMode()});
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
  bundle=null;readyButtons();view("ZIP selected. Validate before starting.");
});
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