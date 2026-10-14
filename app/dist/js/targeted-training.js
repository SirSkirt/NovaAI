"use strict";
// Targeted Training changes the optimizer mask, never routes Nova's answers.
(()=>{
const $=id=>document.getElementById(id);
const invoke=(cmd,args={})=>window.__TAURI__?.core?.invoke(cmd,args) ??
  Promise.reject(Error("Run the installed Neural Nova app"));
const CHUNK=256*1024;
let busy=false,lastRunning=false,current=null;
let androidCheckpointBytes=null,androidCheckpointName=null;
if(/Android/i.test(navigator.userAgent)){
  // Android content:// document grants may not be readable by WebView File APIs.
  // Native dialog + plugin-fs readFile is the same proven path as ZIP imports.
  $("targetedBrowserImportLabel").hidden=true;
  $("targetedBrowserImportLabel").style.display="none";
  $("targetedImportFile").hidden=true;
  $("targetedNativeImport").hidden=false;
}
$("targetedNativeImport").addEventListener("click",()=>safe(async()=>{
  const dialog=window.__TAURI__?.dialog,fs=window.__TAURI__?.fs;
  if(!dialog?.open||!fs?.readFile){
    throw Error("Native Android Files picker unavailable in this build");
  }
  const uri=await dialog.open({multiple:false,directory:false,pickerMode:"document"});
  if(uri===null||uri===undefined)return;
  if(typeof uri!=="string")throw Error("Unexpected Android document selection");
  const bytes=await fs.readFile(uri);
  const input=bytes instanceof Uint8Array?bytes:new Uint8Array(bytes);
  if(!input.length||input.length>24*1024*1024)throw Error("File must be 1 byte–24 MiB");
  androidCheckpointBytes=input;
  const last=decodeURIComponent(uri.split("/").pop()||"");
  androidCheckpointName=last.toLowerCase().endsWith(".nhyb")?last:"neural-nova-targeted-import.nhyb";
  $("targetedImportStatus").textContent="Selected Android document ("+input.length+
    " bytes). Tap Stage imported checkpoint to validate without replacing Nova.";
}));
function error(err){$("targetedStage").textContent="Targeted error: "+String(err);console.error(err);}
async function safe(fn){
  if(busy)return;
  busy=true;
  try{await fn();}catch(e){error(e);}
  finally{busy=false;}
}
const percent=n=>Number.isFinite(n)?(n*100).toFixed(1)+"%":"—";
const decimal=n=>Number.isFinite(n)?n.toFixed(4):"—";
const fraction=s=>s?s.completeCorrect+"/"+s.completeChecked:"—";
const archive=s=>s?s.archiveCompleteCorrect+"/"+s.archiveCompleteChecked:"—";
const ranking=s=>s&&Number.isFinite(s.contrastiveAccuracy)?percent(s.contrastiveAccuracy):"—";
function render(s){
  if(!s)return;
  current=s;
  $("targetedStage").textContent=s.error?"Error: "+s.error:(s.stage||"Ready for targeted practice");
  const active=Boolean(s.running);
  $("targetedStart").disabled=active||s.candidateReady;
  $("targetedStop").disabled=!active;
  $("targetedDiscard").disabled=active||!s.candidateReady;
  $("targetedAccept").disabled=active||!s.candidateReady||!s.bestEpoch;
  for(const id of ["targetedSource","targetedFocus","targetedSkill",
    "targetedLessonData","targetedEpochs","targetedRate","targetedImport",
    "targetedAllowRegression"]){$(id).disabled=active;}
  const complete=s.total?Math.min(100,100*s.completed/s.total):0;
  $("targetedPercent").textContent=complete.toFixed(1)+"%";
  $("targetedFill").style.width=complete.toFixed(1)+"%";
  $("targetedProgress").textContent=active?
    "Epoch "+s.epoch+"/"+s.epochs+" · "+s.completed+"/"+s.total+" targets":
    s.candidateReady?"Candidate ready: selected epoch "+s.bestEpoch:
    "No active targeted candidate";
  for(const [id,value] of [
    ["targetedBaselineLoss",s.baseline&&decimal(s.baseline.heldoutLoss)],
    ["targetedCandidateLoss",s.candidate&&decimal(s.candidate.heldoutLoss)],
    ["targetedBaselineReplies",s.baseline&&fraction(s.baseline)],
    ["targetedCandidateReplies",s.candidate&&fraction(s.candidate)],
    ["targetedBaselinePrefix",s.baseline&&percent(s.baseline.selfGeneratedPrefixAccuracy)],
    ["targetedCandidatePrefix",s.candidate&&percent(s.candidate.selfGeneratedPrefixAccuracy)],
    ["targetedBaselineArchive",s.baseline&&archive(s.baseline)],
    ["targetedCandidateArchive",s.candidate&&archive(s.candidate)],
    ["targetedBaselineContrastive",s.baseline&&ranking(s.baseline)],
    ["targetedCandidateContrastive",s.candidate&&ranking(s.candidate)]
  ])$(id).textContent=value||"—";
  $("targetedWarnings").textContent=s.candidateReady?
    (s.warnings.length?"Regression warnings: "+s.warnings.join("; ")+". ":"")+
      s.recommendation:
    "Training creates a staged copy; the live neural checkpoint is not overwritten.";
  if(s.importedReady){
    $("targetedImportStatus").textContent="Validated .nhyb source: "+s.importLabel+
      ". Live Nova has not been replaced.";
  }
  if(lastRunning&&!active){
    void invoke("hybrid_status").then(h=>{
      if(!h.training&&!window.NovaMassLearning?.active){
        return window.NovaTrainingBackground?.stop();
      }
    }).catch(()=>{});
  }
  lastRunning=active;
}
$("targetedImport").addEventListener("click",()=>safe(async()=>{
  const file=$("targetedImportFile").files[0];
  if(!androidCheckpointBytes &&
      (!file||!file.name.toLowerCase().endsWith(".nhyb"))){
    throw Error("Choose an .nhyb checkpoint to stage");
  }
  const size=androidCheckpointBytes?.length??file.size;
  if(size<1||size>24*1024*1024)throw Error("Checkpoint must be 1 byte–24 MiB");
  const h=await invoke("hybrid_status");
  if(h.training)throw Error("Finish training before importing a candidate");
  const bytes=androidCheckpointBytes||new Uint8Array(await file.arrayBuffer());
  const filename=androidCheckpointName||file.name;
  await invoke("targeted_import_begin",{bytes:bytes.length});
  try{
    let done=0;
    while(done<bytes.length){
      const chunk=Array.from(bytes.subarray(done,done+CHUNK));
      await invoke("targeted_import_append",{chunk});
      done+=chunk.length;
      $("targetedImportStatus").textContent="Staging checkpoint · "+
        Math.floor(100*done/bytes.length)+"%";
    }
    const s=await invoke("targeted_import_finish",{filename});
    $("targetedSource").value="imported";
    render(s);
  }finally{
    await invoke("targeted_import_cancel").catch(()=>{});
  }
}));
$("targetedCopyTeach").addEventListener("click",()=>{
  $("targetedLessonData").value=$("lessonData").value;
  $("targetedStage").textContent="Copied the Teach editor; only the selected block objective receives training.";
});
$("targetedLessonFile").addEventListener("change",()=>safe(async()=>{
  const file=$("targetedLessonFile").files[0];if(!file)return;
  if(file.size>50_000)throw Error("Lesson TXT exceeds 50,000 bytes");
  const value=await file.text();
  if(/[^\x00-\x7f]/.test(value))throw Error("Structured lessons require ASCII TXT");
  $("targetedLessonData").value=value;
}));
$("targetedStart").addEventListener("click",()=>safe(async()=>{
  if(current?.candidateReady)throw Error("Accept or discard the staged candidate first");
  const lessonText=$("targetedLessonData").value;
  if(!lessonText.trim()||lessonText.length>50_000||/[^\x00-\x7f]/.test(lessonText))
    throw Error("Provide a structured ASCII lesson below 50,000 characters");
  const epochs=Number($("targetedEpochs").value);
  if(!Number.isInteger(epochs)||epochs<1||epochs>30)throw Error("Choose 1–30 epochs");
  const source=$("targetedSource").value;
  if(source==="imported"&&!current?.importedReady)
    throw Error("Stage an imported checkpoint first");
  const learningRate=Number($("targetedRate").value);
  await window.NovaTrainingBackground?.start();
  try{
    await invoke("targeted_train",{
      source,focusName:$("targetedFocus").value,
      skillName:$("targetedSkill").value,lessonText,epochs,learningRate
    });
  }catch(e){
    await window.NovaTrainingBackground?.stop().catch(()=>{});
    throw e;
  }
  render(await invoke("targeted_status"));
}));
$("targetedStop").addEventListener("click",()=>safe(async()=>{
  await invoke("targeted_stop");
  $("targetedStage").textContent="Stop requested; the next batch boundary ends targeted training.";
}));
$("targetedDiscard").addEventListener("click",()=>safe(async()=>{
  if(!confirm("Discard the experimental candidate only? Live Nova will not change."))return;
  render(await invoke("targeted_discard"));
  $("targetedAllowRegression").checked=false;
}));
$("targetedAccept").addEventListener("click",()=>safe(async()=>{
  const s=await invoke("targeted_status");
  if(s.running||!s.candidateReady||!s.bestEpoch)
    throw Error("No improved, completed candidate is ready");
  const allowRegression=$("targetedAllowRegression").checked;
  if(s.warnings.length&&!allowRegression)
    throw Error("Review the regression warnings; explicitly enable override to accept anyway");
  let prompt="Install this "+s.focus+" targeted candidate (epoch "+s.bestEpoch+
    ") as Nova's whole active checkpoint?\nA backup will be created and chat history cleared.";
  if(s.warnings.length)prompt+="\nWARNING: "+s.warnings.join("; ")+
    "\nYou have explicitly approved possible forgetting.";
  if(!confirm(prompt))return;
  const result=await invoke("targeted_accept",{allowRegression});
  $("targetedAllowRegression").checked=false;
  render(result);
  $("targetedStage").textContent="Accepted. Reload Nova to refresh Chat and Visualize.";
}));
$("targetedReturnTeach").addEventListener("click",()=>$("tab-teach").click());
$("targetedFocus").addEventListener("change",()=>{
  const tips={
    dense:"Only dense weights update. Transformer and interconnect are frozen but supply neural gradients.",
    transformer:"Transformer language, attention, output, memory and recurrent reasoning weights update; dense and interconnect are frozen.",
    interconnect:"Only bridge and reciprocal neural query/response projections update. Both hemispheres remain computationally active.",
    joint:"Both hemispheres and all neural communication weights update through the same objective."
  };
  $("targetedFocusHint").textContent=tips[$("targetedFocus").value]+
    " The selected skill controls which example objectives provide gradients.";
});
setInterval(async()=>{
  if(!window.__TAURI__?.core?.invoke)return;
  try{render(await invoke("targeted_status"));}
  catch(e){if(!current)error(e);}
},900);
})();
