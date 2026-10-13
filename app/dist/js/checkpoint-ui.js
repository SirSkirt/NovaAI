"use strict";
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
 ?? Promise.reject(Error("Launch the installed native app"));
let busy=false;
const report=s=>$("checkpointTransferStatus").textContent=s;
const maxChunk=256*1024;
async function safe(callback){
  if(busy)return;
  busy=true;
  $("exportCheckpoint").disabled=true;
  $("importCheckpoint").disabled=true;
  try{await callback();}
  catch(error){report("Checkpoint error: "+String(error));console.error("Checkpoint transfer",error);}
  finally{
    busy=false;
    $("exportCheckpoint").disabled=false;
    $("importCheckpoint").disabled=!$("importCheckpointFile").files.length;
  }
}
$("importCheckpointFile").addEventListener("change",()=>{
  $("importCheckpoint").disabled=!$("importCheckpointFile").files.length||busy;
  if($("importCheckpointFile").files.length){
    report("Selected "+$("importCheckpointFile").files[0].name+" · restore will replace current weights.");
  }
});
$("exportCheckpoint").addEventListener("click",()=>safe(async()=>{
  const result=await invoke("hybrid_checkpoint_export_begin");
  if(result.bytes>24*1024*1024)throw Error("Checkpoint is larger than permitted export size");
  const chunks=[];let total=0;
  while(total<result.bytes){
    const arr=await invoke("hybrid_checkpoint_export_chunk",{offset:total});
    if(!arr.length||arr.length>maxChunk)throw Error("Checkpoint download interrupted");
    chunks.push(new Uint8Array(arr));total+=arr.length;
    report("Exporting joint brain · "+Math.floor(100*total/result.bytes)+"%");
  }
  // This is one binary .nhyb, not a new checkpoint format.
  const blob=new Blob(chunks,{type:"application/octet-stream"});
  const href=URL.createObjectURL(blob);
  try{
    const link=document.createElement("a");
    link.href=href;link.download="neural-nova-step-"+result.step+".nhyb";
    document.body.append(link);link.click();link.remove();
    report("Requested download: neural-nova-step-"+result.step+
      ".nhyb ("+result.bytes+" bytes). Check Downloads; on mobile use the system download prompt.");
  }finally{setTimeout(()=>URL.revokeObjectURL(href),3000);}
}));
$("importCheckpoint").addEventListener("click",()=>safe(async()=>{
  const file=$("importCheckpointFile").files[0];
  if(!file||!file.name.toLowerCase().endsWith(".nhyb"))throw Error("Select a .nhyb checkpoint");
  if(file.size===0||file.size>24*1024*1024)throw Error("File must be 1 byte–24 MiB");
  const state=await invoke("hybrid_status");
  if(state.training)throw Error("Stop neural training before importing a checkpoint");
  if(!confirm("Replace Nova's active joint neural checkpoint with "+file.name+
    "? The current checkpoint will be backed up. Chat history is cleared; test archive remains."))return;
  const input=new Uint8Array(await file.arrayBuffer());
  await invoke("hybrid_checkpoint_import_begin",{bytes:input.length});
  let done=0;
  try{
    while(done<input.length){
      const chunk=Array.from(input.subarray(done,done+maxChunk));
      await invoke("hybrid_checkpoint_import_append",{chunk});
      done+=chunk.length;report("Validating imported checkpoint · "+Math.floor(100*done/input.length)+"%");
    }
    const result=await invoke("hybrid_checkpoint_import_finish");
    report("Restored whole joint brain at step "+result.transformerStep+
      " · "+result.tokenizerWords+" learned words. Reload the app to refresh Chat and Visualize.");
  }finally{
    await invoke("hybrid_checkpoint_import_cancel").catch(()=>{});
  }
}));
})();