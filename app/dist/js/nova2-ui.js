"use strict";
/* Nova-2 workspace: native Claude bicameral Rust brain, separate from v1 controls. */
(()=>{
const $=id=>document.getElementById(id);
const invoke=(name,args={})=>{
    const t=window.__TAURI__?.core?.invoke;
    if(!t)throw Error("The native Neural Nova app is required.");
    return t(name,args);
};
let current=null,waiting=false;
function alertStatus(msg,isError=false){
    $("n2Status").textContent=(isError?"Error: ":"")+String(msg);
    if(isError)console.error("Nova-2:",msg);
}
function asLine(s){return String(s??"").trim();}
function print(obj){
    $("n2Progress").textContent=JSON.stringify(obj,null,2);
}
function display(s){
    current=s;
    $("n2Params").textContent=Number(s.parameters).toLocaleString();
    $("n2Left").textContent=Number(s.transformerParameters).toLocaleString();
    $("n2Right").textContent=Number(s.denseParameters).toLocaleString();
    $("n2Inter").textContent=Number(s.interconnectParameters).toLocaleString();
    $("n2Steps").textContent=Number(s.step).toLocaleString();
    alertStatus(s.error||s.stage,Boolean(s.error));
    $("n2Teach").disabled=s.training||waiting;
    $("n2ZipTrain").disabled=s.training||waiting;
    $("n2Stop").disabled=!s.training;
    $("n2Reset").disabled=s.training||waiting;
    $("n2Chat").disabled=s.training||waiting;
    print({stage:s.stage,lesson:s.lesson,epoch:s.epoch,epochs:s.epochs,
        completed:s.completed,total:s.total,optimizerStep:s.step,
        trainingLoss:s.loss,heldoutLoss:s.heldoutLoss,learnedWords:s.words,
        error:s.error});
    if(document.querySelector(".nav-tab.selected")?.dataset.tab==="nova2"){
        $("totalParams").textContent=Number(s.parameters).toLocaleString();
        $("transformerParams").textContent=Number(s.transformerParameters).toLocaleString();
        $("denseParams").textContent=Number(s.denseParameters).toLocaleString();
        $("trainingSteps").textContent=Number(s.step).toLocaleString();
    }
}
async function refresh(){display(await invoke("nova2_status"));}
async function safe(task){try{
    waiting=true;if(current)display(current);
    return await task();
}catch(e){alertStatus(e,true);}finally{waiting=false;if(current)display(current);}}
async function beginTraining(cmd,args){
    await window.NovaTrainingBackground?.start();
    try{return await invoke(cmd,args);}
    catch(error){
        await window.NovaTrainingBackground?.stop();
        throw error;
    }
}
function opts(){
    const epochs=Number($("n2Epochs").value);
    const learningRate=Number($("n2Rate").value);
    if(!Number.isInteger(epochs)||epochs<1||epochs>25)throw Error("Epochs must be 1–25.");
    if(!Number.isFinite(learningRate)||learningRate<0.000001||learningRate>0.01)
        throw Error("Peak learning rate must be between 0.000001 and 0.01.");
    return {epochs,learningRate};
}
function renderHistory(turns){
    $("n2ChatHistory").textContent=turns.length?
        turns.map(t=>"You: "+t.user+"\nNova-2: "+t.assistant).join("\n\n"):"No Nova-2 conversation yet.";
    $("n2ChatHistory").scrollTop=$("n2ChatHistory").scrollHeight;
}
async function reloadHistory(){renderHistory(await invoke("nova2_conversation"));}
$("n2Teach").addEventListener("click",()=>void safe(async()=>{
    const text=$("n2Text").value,mode=$("n2Mode").value;
    if(!text.trim())throw Error("Paste lesson text before starting.");
    display(await beginTraining("nova2_train",{mode,text,...opts()}));
}));
let nativeZip=null;
const onNativeAndroid=/Android/i.test(navigator.userAgent);
if(onNativeAndroid){
    $("n2Zip").hidden=true;
    $("n2NativeZip").hidden=false;
}
$("n2NativeZip").addEventListener("click",()=>void safe(async()=>{
    const dialog=window.__TAURI__?.dialog,fs=window.__TAURI__?.fs;
    if(!dialog?.open||!fs?.readFile)
        throw Error("Android Files ZIP picker is unavailable.");
    const uri=await dialog.open({multiple:false,directory:false,pickerMode:"document"});
    if(uri==null)return;
    if(typeof uri!=="string"||!uri.startsWith("content://"))
        throw Error("Android Files did not return a readable document URI.");
    const result=await fs.readFile(uri);
    const data=result instanceof Uint8Array?result:new Uint8Array(result);
    if(!data.length||data.length>4*1024*1024)
        throw Error("ZIP must be nonempty and under 4 MiB.");
    nativeZip=data;
    alertStatus("Selected ZIP via Android Files ("+nativeZip.length+" bytes).");
}));
$("n2ZipTrain").addEventListener("click",()=>void safe(async()=>{
    let bytes;
    if(onNativeAndroid){
        if(!nativeZip)throw Error("Choose a ZIP using Android Files first.");
        bytes=Array.from(nativeZip);
    }else{
        const file=$("n2Zip").files?.[0];
        if(!file)throw Error("Select a ZIP containing your lesson TXT files.");
        if(!file.size||file.size>4*1024*1024)
            throw Error("ZIP must be nonempty and under 4 MiB.");
        bytes=Array.from(new Uint8Array(await file.arrayBuffer()));
    }
    display(await beginTraining("nova2_zip_train",{bytes,mode:$("n2ZipMode").value,...opts()}));
}));
$("n2Stop").addEventListener("click",()=>void safe(async()=>{
    const active=await invoke("nova2_stop");
    alertStatus(active?"Stop requested. The current step will finish.":"No active training.");
}));
$("n2Reset").addEventListener("click",()=>void safe(async()=>{
    if(!window.confirm("Reset only Nova-2's brain and saved chat? This erases its v2 training. Existing .nhyb checkpoints are untouched."))return;
    display(await invoke("nova2_reset"));renderHistory([]);
}));
$("n2Chat").addEventListener("click",()=>void safe(async()=>{
    const message=asLine($("n2Prompt").value);
    if(!message)throw Error("Write a message to Nova-2.");
    const reply=await invoke("nova2_chat",{message,maxTokens:64});
    $("n2Prompt").value="";
    await reloadHistory();
    $("n2CacheStats").textContent="KV-cached generation: "+reply.generatedTokens+
        " output tokens, "+Number(reply.cacheAttentionKeys).toLocaleString()+
        " attention keys read, checkpoint step "+reply.step+".";
}));

const onAndroid=/Android/i.test(navigator.userAgent);
let importBytes=null;
const writeTransfer=text=>{$("n2Transfer").textContent=text;};
if(onAndroid){
    $("n2ImportNative").hidden=false;
    $("n2ImportFileLabel").hidden=true;
    $("n2ImportFile").hidden=true;
}
$("n2ImportFile").addEventListener("change",()=>{
    const file=$("n2ImportFile").files?.[0];
    writeTransfer(file?"Selected "+file.name+" ("+file.size+" bytes)":"No checkpoint selected.");
});
$("n2ImportNative").addEventListener("click",()=>void safe(async()=>{
    const d=window.__TAURI__?.dialog,fs=window.__TAURI__?.fs;
    if(!d?.open||!fs?.readFile)throw Error("Android Files document picker unavailable.");
    const uri=await d.open({multiple:false,directory:false,pickerMode:"document"});
    if(uri==null)return;
    if(typeof uri!=="string"||!uri.startsWith("content://"))
        throw Error("Android did not return a document URI.");
    const contents=await fs.readFile(uri);
    importBytes=contents instanceof Uint8Array?contents:new Uint8Array(contents);
    if(importBytes.length<1||importBytes.length>24*1024*1024)
        throw Error("Checkpoint exceeds 24 MiB transfer limit.");
    writeTransfer("Selected Android Files checkpoint: "+importBytes.length+" bytes.");
}));
$("n2Export").addEventListener("click",()=>void safe(async()=>{
    if(current?.training)throw Error("Finish Nova-2 training before exporting.");
    const out=await invoke("nova2_checkpoint_export_begin");
    const filename="neural-nova2-step-"+out.step+".nnova2";
    try{
        if(onAndroid){
            const d=window.__TAURI__?.dialog;
            if(!d?.save)throw Error("Android Save to Files unavailable.");
            const target=await d.save({defaultPath:filename,
                filters:[{name:"Neural Nova-2 checkpoint",extensions:["nnova2"]}]});
            if(target==null){writeTransfer("Export cancelled.");return;}
            if(typeof target!=="string"||!target.startsWith("content://"))
                throw Error("Invalid Android Files destination.");
            writeTransfer("Writing and verifying full Nova-2 checkpoint...");
            const saved=await invoke("nova2_checkpoint_export_android",{destinationUri:target});
            if(!saved.verified||saved.bytes!==out.bytes)
                throw Error("Checkpoint save verification failed.");
            writeTransfer("Export verified: "+filename+" ("+saved.bytes+" bytes).");
        }else{
            const chunks=[];let position=0;
            while(position<out.bytes){
                const part=await invoke("nova2_checkpoint_export_chunk",{offset:position});
                if(!part.length||part.length>256*1024)throw Error("Checkpoint chunk invalid.");
                chunks.push(new Uint8Array(part));position+=part.length;
                writeTransfer("Exporting "+Math.floor(position/out.bytes*100)+"%");
            }
            const link=document.createElement("a");
            const blob=new Blob(chunks,{type:"application/octet-stream"});
            const uri=URL.createObjectURL(blob);
            try{link.href=uri;link.download=filename;document.body.append(link);link.click();link.remove();}
            finally{setTimeout(()=>URL.revokeObjectURL(uri),3000);}
            writeTransfer("Download requested: "+filename+" ("+out.bytes+" bytes).");
        }
    }finally{await invoke("nova2_checkpoint_export_cancel").catch(()=>{});}
}));
$("n2Import").addEventListener("click",()=>void safe(async()=>{
    if(current?.training)throw Error("Finish training before importing.");
    const f=$("n2ImportFile").files?.[0];
    if(!onAndroid&&(!f||!f.name.toLowerCase().endsWith(".nnova2")))
        throw Error("Select a .nnova2 checkpoint first.");
    const source=onAndroid?importBytes:new Uint8Array(await f.arrayBuffer());
    if(!source||!source.length||source.length>24*1024*1024)
        throw Error("Checkpoint must be 1 byte–24 MiB.");
    if(!confirm("Restore this Nova-2 checkpoint? The current Nova-2 file will be backed up. The .nhyb brain is unaffected."))return;
    await invoke("nova2_checkpoint_import_begin",{bytes:source.length});
    try{
        for(let position=0;position<source.length;position+=256*1024){
            const chunk=Array.from(source.subarray(position,position+256*1024));
            await invoke("nova2_checkpoint_import_append",{chunk});
            writeTransfer("Importing "+Math.floor((position+chunk.length)/source.length*100)+"%");
        }
        display(await invoke("nova2_checkpoint_import_finish"));
        importBytes=null;
        await reloadHistory();
        writeTransfer("Nova-2 restored. Original .nhyb checkpoints untouched.");
    }finally{await invoke("nova2_checkpoint_import_cancel").catch(()=>{});}
}));

$("n2Prompt").addEventListener("keydown",e=>{
    if(e.key==="Enter"&&!e.shiftKey){e.preventDefault();$("n2Chat").click();}
});
$("n2NewChat").addEventListener("click",()=>void safe(async()=>{
    await invoke("nova2_new_chat");renderHistory([]);
}));
async function boot(){
    const listen=window.__TAURI__?.event?.listen;
    if(listen){
        await listen("nova2-progress",({payload})=>display(payload));
        await listen("nova2-error",({payload})=>{void window.NovaTrainingBackground?.stop();alertStatus(payload,true);});
        await listen("nova2-done",()=>{void window.NovaTrainingBackground?.stop();void safe(refresh);});
    }
    display(await invoke("nova2_boot"));
    await reloadHistory();
}
boot().catch(e=>alertStatus(e,true));
})();