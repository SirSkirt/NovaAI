"use strict";
(()=>{
const $=id=>document.getElementById(id);
const invoke=(command,args={})=>window.__TAURI__?.core?.invoke(command,args)
 ?? Promise.reject(Error("Launch the installed Neural Nova app"));
const android=/Android/i.test(navigator.userAgent);
const maxChunk=256*1024;
let busy=false;
let nativeImport=null;
let nativeImportLabel="";
const report=text=>$("checkpointTransferStatus").textContent=text;
const importAvailable=()=>android?Boolean(nativeImport):Boolean($("importCheckpointFile").files.length);

if(android){
  // Android WebView may not have access to storage-provider File objects.
  // Choose readable SAF documents through the native dialog/fs plugins.
  $("importCheckpointFileLabel").hidden=true;
  $("importCheckpointFileLabel").style.display="none";
  $("importCheckpointFile").hidden=true;
  $("importCheckpointNative").hidden=false;
}
async function safe(callback){
  if(busy)return;
  busy=true;
  $("exportCheckpoint").disabled=true;
  $("importCheckpoint").disabled=true;
  $("importCheckpointNative").disabled=true;
  try{await callback();}
  catch(error){
    report("Checkpoint error: "+String(error));
    console.error("Checkpoint transfer",error);
  }finally{
    busy=false;
    $("exportCheckpoint").disabled=false;
    $("importCheckpointNative").disabled=false;
    $("importCheckpoint").disabled=!importAvailable();
  }
}
$("importCheckpointFile").addEventListener("change",()=>{
  $("importCheckpoint").disabled=!importAvailable()||busy;
  if(!android&&$("importCheckpointFile").files.length){
    report("Selected "+$("importCheckpointFile").files[0].name+
      " · restore will replace current weights.");
  }
});
$("importCheckpointNative").addEventListener("click",()=>safe(async()=>{
  const dialog=window.__TAURI__?.dialog,fs=window.__TAURI__?.fs;
  if(!dialog?.open||!fs?.readFile){
    throw Error("Android Files picker unavailable. Install the latest Nova APK.");
  }
  const uri=await dialog.open({multiple:false,directory:false,pickerMode:"document"});
  if(uri===null||uri===undefined)return;
  if(typeof uri!=="string"||!uri.startsWith("content://")){
    throw Error("Android Files returned an invalid checkpoint document URI");
  }
  report("Reading checkpoint from Android Files…");
  const bytes=await fs.readFile(uri);
  const copied=bytes instanceof Uint8Array?bytes:new Uint8Array(bytes);
  if(copied.length<1||copied.length>24*1024*1024){
    throw Error("Checkpoint must be between 1 byte and 24 MiB");
  }
  nativeImport=copied;
  nativeImportLabel="Selected Android Files checkpoint ("+copied.length+" bytes)";
  report(nativeImportLabel+" · restore will replace current weights.");
}));
$("exportCheckpoint").addEventListener("click",()=>safe(async()=>{
  const result=await invoke("hybrid_checkpoint_export_begin");
  if(!Number.isInteger(result.bytes)||result.bytes<1||result.bytes>24*1024*1024){
    throw Error("Checkpoint is empty or exceeds the 24 MiB export limit");
  }
  const filename="neural-nova-step-"+result.step+".nhyb";
  try{
    if(android){
      // A WebView Blob download can silently fail on Android. Instead request
      // ACTION_CREATE_DOCUMENT via Tauri's native dialog, then have Rust call
      // the Java ContentResolver OutputStream and verify every saved byte.
      const dialog=window.__TAURI__?.dialog;
      if(!dialog?.save){
        throw Error("Android Save to Files is unavailable. Install the latest Nova APK.");
      }
      report("Choose a folder and filename for your full neural checkpoint…");
      const destination=await dialog.save({
        defaultPath:filename,
        filters:[{name:"Neural Nova checkpoint",extensions:["nhyb"]}],
      });
      if(destination===null||destination===undefined){
        report("Checkpoint export cancelled; Nova's active neural weights were not changed.");
        return;
      }
      if(typeof destination!=="string"||!destination.startsWith("content://")){
        throw Error("Android Files did not return a document destination");
      }
      report("Writing and verifying "+result.bytes+" checkpoint bytes…");
      const saved=await invoke("hybrid_checkpoint_export_android",{
        destinationUri:destination
      });
      if(!saved.verified||saved.bytes!==result.bytes){
        throw Error("Android checkpoint verification failed; do not rely on this backup");
      }
      report("Export complete: "+filename+" · "+saved.bytes+
        " bytes saved and SHA-256 verified in Android Files.");
      return;
    }
    // Desktop: keep existing browser-triggered download behavior.
    const chunks=[];let total=0;
    while(total<result.bytes){
      const arr=await invoke("hybrid_checkpoint_export_chunk",{offset:total});
      if(!arr.length||arr.length>maxChunk||arr.length+total>result.bytes){
        throw Error("Checkpoint download interrupted");
      }
      chunks.push(new Uint8Array(arr));total+=arr.length;
      report("Exporting joint brain · "+Math.floor(100*total/result.bytes)+"%");
    }
    const blob=new Blob(chunks,{type:"application/octet-stream"});
    const href=URL.createObjectURL(blob);
    try{
      const link=document.createElement("a");
      link.href=href;link.download=filename;
      document.body.append(link);link.click();link.remove();
      report("Requested download: "+filename+" ("+result.bytes+" bytes).");
    }finally{setTimeout(()=>URL.revokeObjectURL(href),3000);}
  }finally{
    await invoke("hybrid_checkpoint_export_cancel").catch(()=>{});
  }
}));
$("importCheckpoint").addEventListener("click",()=>safe(async()=>{
  const file=$("importCheckpointFile").files[0];
  if(android?!nativeImport:(!file||!file.name.toLowerCase().endsWith(".nhyb"))){
    throw Error("Select a .nhyb checkpoint first");
  }
  const size=android?nativeImport.length:file.size;
  if(size===0||size>24*1024*1024)throw Error("Checkpoint must be 1 byte–24 MiB");
  const state=await invoke("hybrid_status");
  if(state.training)throw Error("Stop neural training before importing a checkpoint");
  const name=android?nativeImportLabel:file.name;
  if(!confirm("Replace Nova's active joint neural checkpoint with "+name+
    "? The current checkpoint will be backed up. Chat history is cleared; test archive remains."))return;
  const input=android?nativeImport:new Uint8Array(await file.arrayBuffer());
  await invoke("hybrid_checkpoint_import_begin",{bytes:input.length});
  let done=0;
  try{
    while(done<input.length){
      const chunk=Array.from(input.subarray(done,done+maxChunk));
      await invoke("hybrid_checkpoint_import_append",{chunk});
      done+=chunk.length;
      report("Validating imported checkpoint · "+Math.floor(100*done/input.length)+"%");
    }
    const restored=await invoke("hybrid_checkpoint_import_finish");
    report("Restored whole joint brain at step "+restored.transformerStep+
      " · "+restored.tokenizerWords+" learned words. Reload to refresh Chat and Visualize.");
    nativeImport=null;
    nativeImportLabel="";
  }finally{
    await invoke("hybrid_checkpoint_import_cancel").catch(()=>{});
  }
}));
})();
