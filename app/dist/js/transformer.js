"use strict";
// Transformer UI is only a transport/view: all model predictions and training run in Rust.
(()=>{
const $=id=>document.getElementById(id);
const invoke=(name,args={})=>{
  const native=window.__TAURI__?.core?.invoke;
  if(!native)throw Error("Install the Tauri application to use the Rust Transformer.");
  return native(name,args);
};
const report=(level,message)=>{
  if(typeof logDiagnostic==="function")logDiagnostic(level,"Transformer",message);
  else console.log(level,message);
};
const safe=async(fn)=>{try{await fn();}catch(e){report("ERROR",e.stack||String(e));}};
let running=false,initialized=false,lastError="";
function setRunning(value){
  running=Boolean(value);
  for(const [id,disabled] of [
    ["transformerTrain",running],["transformerStop",!running],
    ["transformerReset",running],["transformerSend",running||!initialized]
  ])$(id).disabled=disabled;
  $("transformerState").textContent=running?"Transformer training…":initialized?"Transformer ready":"Loading Transformer…";
}
function display(s){
  if(!s)return;
  initialized=true;
  $("transformerParams").textContent=s.parameters.toLocaleString();
  $("transformerSteps").textContent=s.step.toLocaleString();
  $("transformerLoss").textContent=Number.isFinite(s.heldoutLoss)?s.heldoutLoss.toFixed(4):"—";
  $("transformerAccuracy").textContent=Number.isFinite(s.heldoutAccuracy)?(s.heldoutAccuracy*100).toFixed(1)+"%":"—";
  if(s.total){
    const percent=100*s.completed/s.total;
    $("transformerProgress").textContent="Epoch "+s.epoch+"/"+s.epochs+" · "+s.completed+"/"+s.total+" examples";
    $("transformerPercent").textContent=percent.toFixed(1)+"%";
    $("transformerFill").style.width=percent+"%";
  }
  if(s.error&&lastError!==s.error){lastError=s.error;report("ERROR",s.error);}
  setRunning(s.training);
}
function log(message){
  $("transformerLog").textContent=message+"\n"+$("transformerLog").textContent;
}
function attention(items){
  const parent=$("transformerAttention");
  parent.replaceChildren();
  if(!items?.length){parent.textContent="No attention observed.";return;}
  const largest=Math.max(...items.map(item=>item.weight),1e-9);
  for(const item of items){
    const element=document.createElement("span");
    element.className="attention-token";
    element.textContent=item.character;
    element.title="Token "+item.index+" · "+(item.weight*100).toFixed(2)+"% attention";
    element.style.background="rgba(37,188,155,"+(0.12+0.78*item.weight/largest).toFixed(3)+")";
    parent.append(element);
  }
}
function bubble(role,message){
  const box=$("transformerTranscript");
  box.querySelector(".chat-placeholder")?.remove();
  const line=document.createElement("div");
  line.className="dialogue-line "+(role==="user"?"from-user":"from-nova");
  const heading=document.createElement("strong");heading.textContent=role==="user"?"You":"Nova (Transformer)";
  const body=document.createElement("div");body.textContent=message||"(Predicted end of reply)";
  line.append(heading,body);box.append(line);box.scrollTop=box.scrollHeight;
}
function setHelp(){
  $("transformerDatasetHelp").textContent=$("transformerMode").value==="dialogue"?
   "Trains reply characters from Dialogue School. Whole Q&A examples are held out for validation.":
   "Trains next characters from Training Studio text (120–14,000 ASCII bytes). The last 10% is held out.";
}
$("transformerMode").addEventListener("change",setHelp);
$("transformerTrain").addEventListener("click",()=>safe(async()=>{
  const mode=$("transformerMode").value;
  const text=(mode==="dialogue"?$("dialogueData"):$("trainingText")).value;
  const epochs=Number($("transformerEpochs").value);
  if(!Number.isInteger(epochs)||epochs<1||epochs>20)throw Error("Choose 1–20 epochs.");
  if(!text.trim())throw Error("Lesson data is empty.");
  $("transformerLog").textContent="";
  $("transformerProgress").textContent="Preparing "+mode+" training…";
  $("transformerFill").style.width="0%";
  $("transformerPercent").textContent="0%";
  setRunning(true);
  try{
    await invoke("transformer_train",{lesson:{mode,text,epochs}});
    report("INFO","Transformer "+mode+" learning started.");
    await poll();
  }catch(e){setRunning(false);throw e;}
}));
$("transformerStop").addEventListener("click",()=>safe(async()=>{
  if(await invoke("transformer_stop"))log("Stop requested; checkpoint is saved after the epoch.");
}));
$("transformerReset").addEventListener("click",()=>safe(async()=>{
  if(!confirm("Reset only Transformer weights, optimizer and checkpoint? Legacy Nova remains intact."))return;
  display(await invoke("transformer_reset"));
  $("transformerTranscript").replaceChildren();
  $("transformerAttention").textContent="Transformer reset.";
  log("Weights and optimizer state reset.");
}));
$("transformerSend").addEventListener("click",()=>safe(async()=>{
  const message=$("transformerMessage").value.trim();
  if(!/^[\x20-\x7e]{1,180}$/.test(message))throw Error("Messages must contain 1–180 printable ASCII characters.");
  if(running)throw Error("Finish training first.");
  $("transformerSend").disabled=true;
  bubble("user",message);
  try{
    const result=await invoke("transformer_chat",{message,maxChars:128,temperature:0.5});
    bubble("nova",result.response);
    attention(result.attention);
    display(result.status);
    $("transformerMessage").value="";
  }finally{$("transformerSend").disabled=running;}
}));
$("transformerMessage").addEventListener("keydown",e=>{if(e.key==="Enter"){e.preventDefault();$("transformerSend").click();}});
$("transformerClear").addEventListener("click",()=>{
  $("transformerTranscript").replaceChildren();
  report("INFO","Cleared displayed conversation; no weight changes.");
});
let polling=false;
async function poll(){
  if(polling)return;polling=true;
  try{
    const status=await invoke("transformer_status");
    const wasRunning=running;
    display(status);
    if(wasRunning&&!status.training)log(status.error?"Training failed.":"Training finished; checkpoint saved.");
  }catch(e){report("ERROR",e);}finally{polling=false;}
}
async function boot(){
  setHelp();
  if(!window.__TAURI__?.core?.invoke){$("transformerState").textContent="Tauri required";return;}
  try{
    const event=window.__TAURI__?.event;
    if(event?.listen){
      await event.listen("transformer-progress",evt=>{
        const p=evt.payload;
        $("transformerSteps").textContent=p.step.toLocaleString();
        $("transformerProgress").textContent="Epoch "+p.epoch+"/"+p.epochs+" · "+p.completed+" examples";
      });
      await event.listen("transformer-epoch",evt=>{
        const p=evt.payload;
        log("Epoch "+p.epoch+"/"+p.epochs+" · loss "+p.trainLoss.toFixed(4)+
          " · held-out "+p.validationLoss.toFixed(4)+" · accuracy "+(p.validationAccuracy*100).toFixed(1)+"%");
        $("transformerLoss").textContent=p.validationLoss.toFixed(4);
        $("transformerAccuracy").textContent=(p.validationAccuracy*100).toFixed(1)+"%";
      });
      await event.listen("transformer-done",evt=>{
        setRunning(false);
        log(evt.payload.cancelled?"Stopped, checkpoint saved.":"Training complete, checkpoint saved.");
        void poll();
      });
      await event.listen("transformer-error",evt=>{setRunning(false);report("ERROR",evt.payload);void poll();});
    }
  }catch(e){report("WARN","Native training events unavailable; periodic status polling remains enabled: "+e);}
  try{
    display(await invoke("transformer_boot"));
    report("INFO","Custom causal Transformer successfully initialized.");
  }catch(e){report("ERROR","Transformer startup: "+e);$("transformerState").textContent="Startup failed";}
  setInterval(()=>{if(running)void poll();},1000);
}
document.addEventListener("DOMContentLoaded",()=>{void boot();});
})();
