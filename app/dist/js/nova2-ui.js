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
$("n2ZipTrain").addEventListener("click",()=>void safe(async()=>{
    const file=$("n2Zip").files?.[0];
    if(!file)throw Error("Select a ZIP file containing your lesson TXT files.");
    if(file.size>4*1024*1024)throw Error("ZIP must be under 4 MiB.");
    const bytes=Array.from(new Uint8Array(await file.arrayBuffer()));
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