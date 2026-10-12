"use strict";
// Opt-in diagnostic controls: this file never changes neural weights.
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
  ?? Promise.reject(Error("This diagnostic requires the installed Native Nova app"));
let state={enabled:false,path:null,lines:0,lastError:null};
function status(s){
  if(s)state=s;
  $("startDeepLog").disabled=Boolean(state.enabled);
  $("stopDeepLog").disabled=!state.enabled;
  const failure=state.lastError?" · LOG FAILURE: "+state.lastError:"";
  $("deepLogStatus").textContent=(state.enabled?"Recording":"Recording stopped")+
    " · "+state.lines+" events · "+
    (state.path||"No log created")+
    failure;
  $("deepLogStatus").style.color=state.lastError?"#ff99a9":"";
}
async function act(fn){
  try{await fn();}
  catch(error){
    $("deepLogStatus").textContent="Diagnostic error: "+String(error);
    $("deepLogStatus").style.color="#ff99a9";
    console.error("Neural Nova diagnostic",error);
  }
}
$("startDeepLog").addEventListener("click",()=>act(async()=>{
  if(state.path&&!confirm("Start a NEW JSONL recording? The previous log stays on disk, but the log viewer switches to the new file."))return;
  status(await invoke("nova_log_start"));
}));
$("stopDeepLog").addEventListener("click",()=>act(async()=>status(await invoke("nova_log_stop"))));
$("copyDeepLog").addEventListener("click",()=>act(async()=>{
  const contents=await invoke("nova_log_read");
  await navigator.clipboard.writeText(contents);
  status(await invoke("nova_log_status"));
  $("deepLogStatus").textContent+=" · JSONL copied; review before sharing.";
}));
$("saveDeepLog").addEventListener("click",()=>act(async()=>{
  const contents=await invoke("nova_log_read");
  const blob=new Blob([contents],{type:"application/x-ndjson;charset=utf-8"});
  const url=URL.createObjectURL(blob);
  try{
    const link=document.createElement("a");
    link.href=url;
    link.download="neural-nova-diagnostic-"+new Date().toISOString().replace(/[:.]/g,"-")+".jsonl";
    document.body.append(link);
    link.click();link.remove();
    $("deepLogStatus").textContent="Requested JSONL save. Local original: "+(state.path||"unknown")+
      ". If Android blocks the download, use Copy JSONL instead.";
  }finally{
    // Retain URL until the click is queued by the WebView.
    setTimeout(()=>URL.revokeObjectURL(url),1500);
  }
}));
const fmt=x=>Number(x||0).toFixed(5);
$("runDeepProbe").addEventListener("click",()=>act(async()=>{
  const prompts=$("deepProbePrompts").value;
  const output=$("deepProbeOutput");
  output.textContent="Inspecting identical learned weights with and without episodic memory...";
  const window=Number($("chatContext").value);
  const answer=await invoke("hybrid_diagnostic_probe",{
    prompts,contextChars:window,
    useConversationMemory:$("deepProbeMemory").checked
  });
  const rows=[
    "READ-ONLY NEURAL DIAGNOSTICS",
    "Transformer step "+answer.transformerStep+" · Dense step "+answer.denseStep+
      " · word units "+answer.vocabulary,
    "Weights fingerprint: "+answer.weightsFingerprint,
    "Conversation-memory mode selected: "+answer.withConversationMemory,
    "",
  ];
  for(const probe of answer.probes){
    if(probe.error){rows.push(probe.prompt+": ERROR "+probe.error);continue;}
    const top=p=>p.top.slice(0,5).map(c=>{
      const name=c.code===10?"END":String.fromCharCode(c.code);
      return name+" "+(100*c.probability).toFixed(2)+"%";
    }).join(", ");
    rows.push(
      "PROMPT: "+probe.prompt,
      "Token IDs: "+JSON.stringify(probe.tokenIds),
      "Memory slots: "+probe.memorySlots+" · attention positions: "+probe.attention.length,
      "Memory ON first char: "+top(probe.firstWithMemory),
      "Memory OFF first char: "+top(probe.firstWithoutMemory),
      "Memory effect (total variation): "+(100*probe.memoryTotalVariation).toFixed(3)+"%",
      "Bridge projection norm: "+fmt(probe.bridge.bridgeNorm)+
        " · Transformer norm: "+fmt(probe.bridge.transformerNorm),
      "Bridge ablation difference: "+(100*probe.bridge.bridgeAblationTotalVariation).toFixed(3)+"%",
      "Greedy (32 chars): "+JSON.stringify(probe.greedy32),
      ""
    );
  }
  output.textContent=rows.join("\n");
  status(await invoke("nova_log_status"));
}));
async function ready(){
  status(await invoke("nova_log_status"));
}
document.addEventListener("neural-nova-booted",()=>act(ready),{once:true});
})();
