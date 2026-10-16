"use strict";
/* Nova Messenger: inference, conversations, model import/export, read-only brain.
   There are deliberately no training commands or teacher interface here. */
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
  ?? Promise.reject(Error("Open the installed Nova app to use the neural runtime"));
const on=(id,name,fn)=>$(id).addEventListener(name,fn);
const isAndroid=/Android/i.test(navigator.userAgent);
const CK=256*1024;
let busy=false,ready=false,history=[],lastPrompt="";
const messageEl=$("chatMessage"),sendEl=$("sendMessage");
function setMessage(text,error=false){
  $("chatContextReport").textContent=String(text);
  $("chatContextReport").classList.toggle("error-message",!!error);
}
function showStatus(data){
  ready=!!data?.trained;
  $("brainStateText").textContent=ready?"Nova ready · step "+Number(data.step||0).toLocaleString():"Import checkpoint";
  $("brainState").classList.toggle("error",!ready);
  $("modelNotice").hidden=ready;
  $("modelNotice").textContent=ready?"":"No trained checkpoint loaded. Open Checkpoints and import a .nnova2 file.";
  $("memoryStatus").textContent=history.length+"/12 turns · "+(data?.words||0)+" word units";
  sendEl.disabled=busy||!ready;
  messageEl.disabled=busy||!ready;
  $("exportCheckpoint").disabled=busy||!ready;
}
function renderHistory(){
  const log=$("chatTranscript");log.replaceChildren();
  if(!history.length){
    const placeholder=document.createElement("div");placeholder.className="chat-placeholder";
    placeholder.textContent=ready?"You can talk to Nova now. The response comes from the active Bicameral checkpoint.":"Waiting for a trained checkpoint…";
    log.append(placeholder);return;
  }
  for(const turn of history){
    for(const [who,text] of [["user",turn.user],["nova",turn.assistant]]){
      const bubble=document.createElement("article");bubble.className="chat-bubble "+who;
      const title=document.createElement("strong");title.textContent=who==="user"?"You":"Nova";
      const content=document.createElement("div");content.textContent=text||"(No text generated)";
      bubble.append(title,content);log.append(bubble);
    }
  }
  log.scrollTop=log.scrollHeight;
}
function trace(response,input){
  const nodes=$("novaTraceNodes");nodes.replaceChildren();
  for(const [label,value] of [
    ["Message",input],
    ["Checkpoint","Step "+Number(response.step||0).toLocaleString()],
    ["Generated",response.generatedTokens+" tokens"],
    ["KV cache",Number(response.cacheAttentionKeys||0).toLocaleString()+" attention-key reads"],
    ["Reasoning trace","Not available from the current Nova-2 model"]
  ]){
    const li=document.createElement("li"),strong=document.createElement("strong"),span=document.createElement("span");
    strong.textContent=label;span.textContent=String(value);li.append(strong,span);nodes.append(li);
  }
}
async function boot(){
  try{
    const s=await invoke("nova2_boot");
    history=await invoke("nova2_conversation");
    showStatus(s);renderHistory();
    if(ready)setMessage("Nova is ready. "+s.stage);
    else setMessage(s.stage||"Import a trained .nnova2 checkpoint to begin.");
  }catch(error){ready=false;showStatus(null);renderHistory();setMessage("Nova initialization failed: "+error,true);}
}
async function send(){
  if(busy||!ready)return;
  const message=messageEl.value.trim();
  if(!message)return;
  if(/[^\x00-\x7f]/.test(message)){
    setMessage("This checkpoint currently accepts ASCII input only. Remove special characters and retry.",true);return;
  }
  busy=true;sendEl.disabled=true;messageEl.disabled=true;
  lastPrompt=message;setMessage("Nova is generating a reply…");
  try{
    const result=await invoke("nova2_chat",{
      message,maxTokens:Number($("chatLength").value),
      temperature:Number($("chatTemperature").value),
      contextTokens:Number($("chatContext").value),
      sampling:$("chatSampling").value
    });
    history=await invoke("nova2_conversation");
    messageEl.value="";renderHistory();trace(result,message);
    setMessage(result.generatedTokens+" generated tokens · "+Number(result.cacheAttentionKeys||0).toLocaleString()+" KV attention-key reads");
    $("inspectMessage").value=message;
    if($("autoInspectBrain").checked)void inspect(message);
  }catch(error){setMessage("Could not generate a reply: "+error,true);}
  finally{busy=false;sendEl.disabled=!ready;messageEl.disabled=!ready;messageEl.focus();}
}
on("sendMessage","click",()=>{void send();});
on("chatMessage","keydown",e=>{if(e.key==="Enter"&&!e.shiftKey){e.preventDefault();void send();}});
on("clearChat","click",async()=>{
  if(busy)return;
  try{await invoke("nova2_new_chat");history=[];renderHistory();$("novaTraceNodes").replaceChildren();setMessage("Started a new conversation.");}
  catch(error){setMessage("Could not clear conversation: "+error,true);}
});
on("chatTemperature","input",()=>{$("chatTempValue").textContent=$("chatTemperature").value;});
on("openModelTools","click",()=>{
  const details=$("checkpointTools");details.open=true;
  details.scrollIntoView({behavior:"smooth",block:"center"});
});
const rail=$("novaBrainRail"),backdrop=$("brainBackdrop");
function showBrain(open){
  rail.classList.toggle("rail-open",!!open);
  backdrop.hidden=!open;
  $("toggleBrain").setAttribute("aria-expanded",String(!!open));
  if(open)rail.scrollTop=0;
}
on("toggleBrain","click",()=>showBrain(!rail.classList.contains("rail-open")));
on("closeBrain","click",()=>showBrain(false));
on("brainBackdrop","click",()=>showBrain(false));
document.addEventListener("keydown",e=>{if(e.key==="Escape")showBrain(false);});
const ns="http://www.w3.org/2000/svg";
function dot(group,x,y,r,fill,caption,opacity){
  const node=document.createElementNS(ns,"circle");
  for(const [k,v] of Object.entries({cx:x,cy:y,r,fill,opacity}))node.setAttribute(k,String(v));
  const tip=document.createElementNS(ns,"title");tip.textContent=caption;node.append(tip);group.append(node);
}
let inspectorRequest=0;
async function inspect(input){
  if(!ready)return;
  const message=(input||$("inspectMessage").value||lastPrompt).trim();
  if(!message)return;
  if(/[^\x00-\x7f]/.test(message)){setMessage("Inspector requires ASCII input.",true);return;}
  $("inspectMessage").value=message;
  const request=++inspectorRequest,button=$("inspectBrain");button.disabled=true;
  $("visualizerDetail").textContent="Measuring neural output sensitivity…";
  try{
    const result=await invoke("nova2_inspect",{message,contextChars:Number($("chatContext").value)});
    if(request!==inspectorRequest)return;
    const sensitivity=result.attention||[];
    $("attentionCount").textContent=String(sensitivity.length);
    $("neuronCount").textContent="Weight samples";
    const left=$("leftActivity"),right=$("rightActivity");
    left.replaceChildren();right.replaceChildren();
    const max=Math.max(1e-8,...sensitivity.map(t=>Number(t.weight)||0));
    sensitivity.forEach((t,i)=>{
      const intensity=Math.min(1,(Number(t.weight)||0)/max);
      dot(left,155+(i%5)*44,108+Math.floor(i/5)*49,4+5*intensity,"#78b9dc",
        t.character+" · removal effect "+Number(t.weight).toPrecision(3),0.35+0.6*intensity);
    });
    const weights=(result.denseLayers||[]).flat(),peak=Math.max(1e-7,...weights.map(x=>Math.abs(Number(x)||0)));
    weights.slice(0,35).forEach((w,i)=>{
      const intensity=Math.min(1,Math.abs(Number(w)||0)/peak);
      dot(right,468+(i%7)*27,104+Math.floor(i/7)*48,3+4*intensity,"#baa5d5",
        "Sampled dense weight "+i+" = "+Number(w).toPrecision(3),0.25+0.7*intensity);
    });
    $("bridgeFibers").setAttribute("opacity",result.interconnectUsed?"0.67":"0.16");
    const tiles=$("attentionTiles");tiles.replaceChildren();
    for(const t of sensitivity){
      const span=document.createElement("span");span.className="attention-tile";
      span.textContent=t.character;span.title="Output shift: "+Number(t.weight).toPrecision(4);tiles.append(span);
    }
    if(!sensitivity.length)tiles.textContent="No samples were returned.";
    const pred=$("predictions");pred.replaceChildren();
    for(const v of result.topPredictions||[]){
      const el=document.createElement("span");el.className="prediction-token";
      const id=Number(v.code),token=id===10?"↵":id===32?"SPACE":id>=32&&id<127?String.fromCharCode(id):"#"+id;
      el.textContent=token+" · "+(100*Number(v.probability)).toFixed(2)+"%";pred.append(el);
    }
    $("visualizerDetail").textContent="Sampled output sensitivity, Dense weights and pathway ablation. The schematic is not an exact neuron map.";
    $("brainMemoryInfo").textContent=result.method||"Read-only neural measurements.";
  }catch(error){if(request===inspectorRequest)$("visualizerDetail").textContent="Could not inspect model: "+error;}
  finally{if(request===inspectorRequest)button.disabled=false;}
}
on("inspectBrain","click",()=>{void inspect();});
if(isAndroid){$("importCheckpointNative").hidden=false;$("importCheckpointFileLabel").hidden=true;}
let selectedBytes=null;
on("importCheckpointFile","change",async()=>{
  selectedBytes=null;const file=$("importCheckpointFile").files?.[0];
  if(!file)return;
  if(file.size>24*1024*1024){$("checkpointTransferStatus").textContent="Checkpoint exceeds the 24 MiB limit.";return;}
  selectedBytes=new Uint8Array(await file.arrayBuffer());
  $("importCheckpoint").disabled=false;
  $("checkpointTransferStatus").textContent="Selected "+file.name+" ("+selectedBytes.length+" bytes).";
});
on("importCheckpointNative","click",async()=>{
  try{
    const api=window.__TAURI__;
    if(!api?.dialog?.open||!api?.fs?.readFile)throw Error("Android Files picker unavailable");
    const uri=await api.dialog.open({multiple:false,directory:false,pickerMode:"document"});
    if(!uri)return;
    selectedBytes=await api.fs.readFile(uri);
    if(selectedBytes.length>24*1024*1024)throw Error("Checkpoint exceeds the 24 MiB limit");
    $("importCheckpoint").disabled=false;
    $("checkpointTransferStatus").textContent="Loaded "+selectedBytes.length+" bytes from Android Files.";
  }catch(error){selectedBytes=null;$("importCheckpoint").disabled=true;$("checkpointTransferStatus").textContent="Could not select checkpoint: "+error;}
});
on("importCheckpoint","click",async()=>{
  if(busy||!selectedBytes?.length)return;
  if(!confirm("Import this checkpoint as Nova's active neural brain? A backup of the previous checkpoint will be saved."))return;
  busy=true;$("importCheckpoint").disabled=true;sendEl.disabled=true;messageEl.disabled=true;
  try{
    await invoke("nova2_checkpoint_import_begin",{bytes:selectedBytes.length});
    for(let offset=0;offset<selectedBytes.length;offset+=CK){
      await invoke("nova2_checkpoint_import_append",{chunk:Array.from(selectedBytes.slice(offset,offset+CK))});
      $("checkpointTransferStatus").textContent="Importing "+Math.min(offset+CK,selectedBytes.length).toLocaleString()+" / "+selectedBytes.length.toLocaleString()+" bytes…";
    }
    const state=await invoke("nova2_checkpoint_import_finish");
    selectedBytes=null;history=[];ready=!!state.trained;showStatus(state);renderHistory();
    $("checkpointTransferStatus").textContent="Checkpoint installed · step "+Number(state.step).toLocaleString();
    setMessage("Nova is ready to chat with the imported checkpoint.");
  }catch(error){$("checkpointTransferStatus").textContent="Import failed: "+error;setMessage(String(error),true);}
  finally{await invoke("nova2_checkpoint_import_cancel").catch(()=>{});busy=false;showStatus(await invoke("nova2_status").catch(()=>({trained:ready})));$("importCheckpoint").disabled=!selectedBytes?.length;}
});
on("exportCheckpoint","click",async()=>{
  if(busy||!ready)return;
  busy=true;$("exportCheckpoint").disabled=true;
  try{
    const info=await invoke("nova2_checkpoint_export_begin");
    if(isAndroid){
      const uri=await window.__TAURI__?.dialog?.save({defaultPath:"nova-step-"+info.step+".nnova2",
        filters:[{name:"Nova checkpoint",extensions:["nnova2"]}]});
      if(!uri){$("checkpointTransferStatus").textContent="Export cancelled.";return;}
      const saved=await invoke("nova2_checkpoint_export_android",{destinationUri:uri});
      $("checkpointTransferStatus").textContent=saved.verified?"Verified export: "+saved.bytes.toLocaleString()+" bytes":"Checkpoint saved (verification unavailable).";
    }else{
      const chunks=[];for(let offset=0;offset<info.bytes;offset+=CK){
        const bytes=await invoke("nova2_checkpoint_export_chunk",{offset});
        if(!bytes?.length)throw Error("Incomplete checkpoint export");chunks.push(new Uint8Array(bytes));
      }
      const blob=new Blob(chunks,{type:"application/octet-stream"});
      const link=document.createElement("a");link.href=URL.createObjectURL(blob);
      link.download="nova-step-"+info.step+".nnova2";link.click();
      setTimeout(()=>URL.revokeObjectURL(link.href),5000);
      $("checkpointTransferStatus").textContent="Exported "+blob.size.toLocaleString()+" bytes.";
    }
  }catch(error){$("checkpointTransferStatus").textContent="Export failed: "+error;}
  finally{await invoke("nova2_checkpoint_export_cancel").catch(()=>{});busy=false;$("exportCheckpoint").disabled=!ready;}
});
void boot();
})();
