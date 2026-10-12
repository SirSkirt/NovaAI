"use strict";
// Neural Nova v0.1: browser is UI only; every displayed activation comes from Rust.
const $ = id => document.getElementById(id);
const invoke = (name, args={}) => {
  if (!window.__TAURI__?.core?.invoke) {
    throw new Error("The Rust brain requires the installed Tauri app. A normal browser cannot train this model.");
  }
  return window.__TAURI__.core.invoke(name,args);
};
const eventAPI = () => window.__TAURI__?.event;
const layerColors = ["#268bce","#946bdb","#e2a246","#34b785"];
const neuronCounts = [1024,512,512,128];
const gridColumns = [16,16,16,8];
let latestTrace=null;
let latestStatus=null;
let training=false;
let activeTrainingMode="text";
let lastDialogueQuestion="";
let pendingInspect=0;
let canvasMetrics=null;
const canvas=$("brainCanvas");
const ctx=canvas.getContext("2d");

function setState(label,kind="ready"){
  $("brainStateText").textContent=label;
  $("brainState").className="pill "+(kind==="training"?"running":kind==="error"?"offline":"");
}
// The diagnostics area is intentionally always available at the bottom of the app.
// Every caught JS, Promise, Tauri command, and training error is shown here.
const diagnosticEntries=[];
let errorCount=0;
let lastBackendError=null;
function errorText(error){
  if(typeof error==="string")return error;
  if(error instanceof Error)return error.stack||error.message;
  try{return JSON.stringify(error);}catch(_){return String(error);}
}
function logDiagnostic(severity,source,message){
  const entry={
    time:new Date().toLocaleTimeString(),
    severity,
    source,
    message:errorText(message)
  };
  diagnosticEntries.push(entry);
  if(diagnosticEntries.length>150) diagnosticEntries.shift();
  if(severity==="ERROR")errorCount++;
  const area=$("diagnosticLog");
  if(area) {
    area.textContent=diagnosticEntries.map(item=>
      "["+item.time+"] ["+item.severity+"] ["+item.source+"] "+item.message
    ).join("\n\n");
    area.scrollTop=area.scrollHeight;
  }
  const summary=$("diagnosticCount");
  if(summary)summary.textContent=errorCount+" error"+(errorCount===1?"":"s")+" · "+diagnosticEntries.length+" entries";
  const panel=$("diagnostics");
  if(panel)panel.classList.toggle("has-errors",errorCount>0);
  if(severity==="ERROR"){
    if(panel)panel.open=true;
    console.error("[Neural Nova]["+source+"]",entry.message);
  }
}
function setError(error,source="UI"){logDiagnostic("ERROR",source,error);}
function clearError(){} // Preserve earlier errors until the user clears the debugger.
function safeRun(task,source="UI"){
  return Promise.resolve().then(task).catch(error=>setError(error,source));
}
window.addEventListener("error",event=>{
  logDiagnostic("ERROR","JavaScript",event.message+
    (event.filename?" @ "+event.filename+":"+event.lineno+":"+event.colno:""));
});
window.addEventListener("unhandledrejection",event=>{
  logDiagnostic("ERROR","Unhandled promise",event.reason);
});
function clearDiagnostics(){
  diagnosticEntries.length=0;errorCount=0;lastBackendError=null;
  $("diagnosticLog").textContent="No diagnostics recorded in this session.";
  $("diagnosticCount").textContent="0 errors · 0 entries";
  $("diagnostics").classList.remove("has-errors");
}
$("clearDiagnostics").addEventListener("click",clearDiagnostics);
$("copyDiagnostics").addEventListener("click",()=>safeRun(async()=>{
  const report="Neural Nova v0.1.2 diagnostic report\n"+
    "Architecture: 1024 → 512 → 512 → 128 / 64-character context\n"+
    "Status: "+JSON.stringify(latestStatus)+"\n"+
    diagnosticEntries.map(item=>"["+item.time+"] ["+item.severity+"] ["+item.source+"] "+item.message).join("\n\n");
  await navigator.clipboard.writeText(report);
  logDiagnostic("INFO","Debugger","Diagnostic report copied.");
},"Debugger"));

function updateStatus(status){
  if(!status)return;
  latestStatus=status;
  $("parameterTotal").textContent=status.parameters.toLocaleString();
  $("neuronTotal").textContent=status.dimensions.reduce((a,b)=>a+b,0).toLocaleString();
  $("stepCount").textContent=status.steps.toLocaleString();
  $("lossValue").textContent=Number.isFinite(status.lastLoss)?status.lastLoss.toFixed(4):"—";
  $("validationLoss").textContent=Number.isFinite(status.validationLoss)?status.validationLoss.toFixed(4):"—";
  $("validationAccuracy").textContent=Number.isFinite(status.validationAccuracy)?(status.validationAccuracy*100).toFixed(1)+"%":"—";
  if(activeTrainingMode==="dialogue"){
    $("dialogueValidationLoss").textContent=Number.isFinite(status.validationLoss)?status.validationLoss.toFixed(4):"—";
    $("dialogueValidationAccuracy").textContent=Number.isFinite(status.validationAccuracy)?(status.validationAccuracy*100).toFixed(1)+"%":"—";
  }
  if(!training) setState(status.trained?"Weights loaded":"Untrained");
}
function charLabel(code){
  if(code===10)return "↵ LF";
  if(code===9)return "⇥ TAB";
  if(code===32)return "SPACE";
  if(code<32||code===127)return "0x"+code.toString(16).padStart(2,"0");
  return String.fromCharCode(code);
}
function renderPredictions(predictions){
  const box=$("predictions");
  box.replaceChildren();
  for(const p of predictions){
    const item=document.createElement("span");
    item.className="prediction";
    const strong=document.createElement("strong");
    strong.textContent=charLabel(p.code);
    item.append(strong,document.createTextNode((p.probability*100).toFixed(1)+"%"));
    box.append(item);
  }
}
function neuronPosition(layer,index,width,height){
  const groups=4;
  const centerX=width*(layer+0.5)/groups;
  const columns=gridColumns[layer];
  const rows=Math.ceil(neuronCounts[layer]/columns);
  const laneWidth=Math.min(120,width/groups-10);
  const gapX=laneWidth/(columns+1);
  const availableHeight=height-85;
  const gapY=availableHeight/(rows+1);
  return {x:centerX+(index%columns-(columns-1)/2)*gapX,
          y:51+(Math.floor(index/columns)+1)*gapY};
}
function blend(hex,alpha){
  const red=parseInt(hex.slice(1,3),16),green=parseInt(hex.slice(3,5),16),blue=parseInt(hex.slice(5,7),16);
  return "rgba("+red+","+green+","+blue+","+alpha+")";
}
function redraw(){
  const rect=canvas.getBoundingClientRect();
  const width=rect.width,height=rect.height;
  if(!width||!height)return;
  const dpr=Math.min(window.devicePixelRatio||1,2);
  if(canvas.width!==Math.round(width*dpr)||canvas.height!==Math.round(height*dpr)){
    canvas.width=Math.round(width*dpr);
    canvas.height=Math.round(height*dpr);
  }
  ctx.setTransform(dpr,0,0,dpr,0,0);
  ctx.clearRect(0,0,width,height);
  canvasMetrics={width,height};
  const trace=latestTrace;
  for(let layer=0;layer<4;layer++){
    const x=width*(layer+0.5)/4;
    ctx.fillStyle="#a8b8d5";
    ctx.font="600 12px system-ui";
    ctx.textAlign="center";
    ctx.fillText(["Input","Hidden 1","Hidden 2","Output"][layer],x,24);
    ctx.fillStyle="#8190aa";
    ctx.font="11px system-ui";
    ctx.fillText(neuronCounts[layer].toLocaleString()+" neurons",x,height-12);
    if(layer<3){
      ctx.strokeStyle="rgba(120,142,180,.10)";
      ctx.lineWidth=1;
      ctx.beginPath();ctx.moveTo(x+width/8,38);ctx.lineTo(x+width/8,height-37);ctx.stroke();
    }
  }
  if(!trace){
    ctx.textAlign="center";ctx.font="13px system-ui";ctx.fillStyle="#99a9c2";
    ctx.fillText("Waiting for the first network inspection…",width/2,height/2);
    return;
  }
  const peak=trace.layers.map(layer=>Math.max(0.00001,...layer.map(Math.abs)));
  // Every dot represents one real neuron. Links are only the strongest displayed
  // contributions, not all 853,120 weights.
  for(const link of trace.links){
    if(Math.abs(link.contribution)<1e-8)continue;
    const a=neuronPosition(link.transition,link.from,width,height);
    const b=neuronPosition(link.transition+1,link.to,width,height);
    const magnitude=Math.min(1,Math.abs(link.contribution)*2);
    ctx.strokeStyle=link.weight<0?blend("#ec778a",0.12+magnitude*.30):blend("#79c9bb",0.12+magnitude*.30);
    ctx.lineWidth=.7+magnitude*.9;
    ctx.beginPath();ctx.moveTo(a.x,a.y);ctx.lineTo(b.x,b.y);ctx.stroke();
  }
  trace.layers.forEach((values,layer)=>{
    const p=peak[layer];
    const base=layerColors[layer];
    values.forEach((value,index)=>{
      const xy=neuronPosition(layer,index,width,height);
      const activation=Math.max(0,Math.abs(value)/p);
      const alpha=.17+.83*Math.pow(activation,.57);
      const radius=layer===0?Math.max(.95,width/710):layer===3?Math.max(1.7,width/600):Math.max(1.3,width/640);
      ctx.fillStyle=blend(base,alpha);
      ctx.beginPath();ctx.arc(xy.x,xy.y,radius+(activation*.65),0,Math.PI*2);ctx.fill();
    });
  });
}
function showTrace(trace){
  if(!trace)return;
  latestTrace=trace;
  renderPredictions(trace.topPredictions);
  redraw();
}
function inspectClick(event){
  if(!latestTrace||!canvasMetrics)return;
  const r=canvas.getBoundingClientRect(),width=canvasMetrics.width,height=canvasMetrics.height;
  const x=event.clientX-r.left,y=event.clientY-r.top;
  let closest=null;
  for(let layer=0;layer<4;layer++){
    if(Math.abs(x-width*(layer+.5)/4)>width/8)continue;
    const n=neuronCounts[layer];
    const columns=gridColumns[layer];
    const rows=Math.ceil(n/columns);
    const laneWidth=Math.min(120,width/4-10);
    const gapX=laneWidth/(columns+1);
    const gapY=(height-85)/(rows+1);
    const centerX=width*(layer+.5)/4;
    const column=Math.round((x-centerX)/gapX+(columns-1)/2);
    const row=Math.round((y-51)/gapY-1);
    if(column<0||column>=columns||row<0||row>=rows)continue;
    const index=row*columns+column;
    if(index>=n)continue;
    const pos=neuronPosition(layer,index,width,height);
    const distance=Math.hypot(x-pos.x,y-pos.y);
    if(!closest||distance<closest.distance)closest={layer,index,distance};
  }
  if(!closest||closest.distance>24)return;
  const v=latestTrace.layers[closest.layer][closest.index];
  const label=["Input","Hidden 1","Hidden 2","Output"][closest.layer];
  const connection=latestTrace.links.find(link=>(link.transition===closest.layer&&link.from===closest.index)||
    (link.transition===closest.layer-1&&link.to===closest.index));
  $("neuronDetail").textContent=label+" / neuron "+closest.index+" · activation "+v.toFixed(6)+
    (connection?" · sampled weight "+connection.weight.toFixed(5)+" · contribution "+connection.contribution.toFixed(5):"");
}
async function inspect(){
  clearError();
  const text=$("probeInput").value;
  showTrace(await invoke("neural_inspect",{text}));
}
let timer;
$("probeInput").addEventListener("input",()=>{
  clearTimeout(timer);
  const version=++pendingInspect;
  timer=setTimeout(()=>safeRun(async()=>{
    const result=await invoke("neural_inspect",{text:$("probeInput").value});
    if(version===pendingInspect)showTrace(result);
  }),360);
});
$("inspectBtn").addEventListener("click",()=>safeRun(inspect));
$("generateBtn").addEventListener("click",()=>safeRun(async()=>{
  clearError();
  const button=$("generateBtn");
  button.disabled=true;
  try{
    const result=await invoke("neural_generate",{
      prompt:$("probeInput").value,
      length:Number($("lengthInput").value),
      temperature:Number($("temperatureInput").value)
    });
    $("generatedOutput").textContent=result.text;
    showTrace(result.trace);
    updateStatus(result.status);
  }finally{button.disabled=false;}
}));
function setTraining(active){
  training=active;
  $("startTrain").disabled=active;
  $("stopTrain").disabled=!active;
  $("resetBtn").disabled=active;
  $("trainDialogues").disabled=active;
  $("sendDialogue").disabled=active;
  $("dialogueStatus").textContent=active&&activeTrainingMode==="dialogue"?"Dialogue training in progress…":
    active?"Text training in progress…":"No dialogue lesson running.";
  setState(active?"Training":latestStatus?.trained?"Weights loaded":"Untrained",active?"training":"ready");
}
function updateProgressFromStatus(status){
  if(!status.totalExamples)return;
  const percent=100*(status.completedExamples||0)/status.totalExamples;
  $("progressPercent").textContent=percent.toFixed(1)+"%";
  $("progressFill").style.width=percent.toFixed(2)+"%";
  $("progressText").textContent="Epoch "+status.currentEpoch+"/"+status.totalEpochs+
    " · "+status.completedExamples.toLocaleString()+"/"+status.totalExamples.toLocaleString()+" examples";
}
let polling=false;
async function pollBrainStatus(){
  if(polling||!window.__TAURI__?.core?.invoke)return;
  polling=true;
  try{
    const status=await invoke("neural_status");
    const wasTraining=training;
    if(status.training!==training)setTraining(status.training);
    updateStatus(status);
    if(status.training||wasTraining)updateProgressFromStatus(status);
    if(status.lastError && status.lastError!==lastBackendError){
      lastBackendError=status.lastError;
      setError(status.lastError,"Rust training worker");
    }
    if(wasTraining&&!status.training){
      appendLog(status.lastError?"Training ended with an error.":"Training finished or stopped. Checkpoint saved.");
      logDiagnostic(status.lastError?"WARN":"INFO","Training",
        status.lastError?"Training failed.":"Training is no longer running.");
    }
  } catch(error){
    setError(error,"Status polling");
  } finally{polling=false;}
}
function updateProgress(value){
  const percent=value.totalExamples?100*value.completedExamples/value.totalExamples:0;
  $("progressPercent").textContent=percent.toFixed(1)+"%";
  $("progressFill").style.width=percent.toFixed(2)+"%";
  $("progressText").textContent="Epoch "+value.epoch+"/"+value.epochs+" · "+value.completedExamples.toLocaleString()+"/"+value.totalExamples.toLocaleString()+" examples";
  $("lossValue").textContent=value.loss.toFixed(4);
  $("stepCount").textContent=value.steps.toLocaleString();
  showTrace(value.trace);
}
function appendLog(message){
  const log=$("trainingLog");
  log.textContent=message+"\n"+log.textContent;
}
$("corpusFile").addEventListener("change",()=>safeRun(async()=>{
  const file=$("corpusFile").files[0];
  if(!file)return;
  if(file.size>250000){throw new Error("Corpus file exceeds 250,000 bytes. Try a smaller selection.");}
  const text=await file.text();
  if(!/^[\x00-\x7F]*$/.test(text))throw new Error("This version accepts ASCII only. Convert Unicode punctuation before training.");
  $("trainingText").value=text;
  updateCorpusSize();
}));
function updateCorpusSize(){
  const text=$("trainingText").value;
  const ascii=/^[\x00-\x7F]*$/.test(text);
  $("corpusSize").textContent=text.length.toLocaleString()+(ascii?" ASCII bytes":" characters (contains non-ASCII)");
}
const lesson=[
  "A fox wandered into the forest, searching for a warm place to sleep.",
  "Although the weather was cold, the children enjoyed their walk to school.",
  "When the clock struck midnight, every window in the old house lit up.",
  "The scientist compared two experiments before writing her conclusion.",
  "If the power goes out, the emergency generator starts automatically.",
  "The musician practiced every morning because she wanted to improve.",
  "After the storm passed, a bright rainbow appeared over the mountains.",
  "Nova reads each character and predicts what character should come next.",
  "The baker opened the shop early, while the town was still sleeping.",
  "Because the bridge was closed, the driver chose another route.",
  "A small robot carried supplies from the workshop to the laboratory.",
  "Whenever Maya found a new book, she shared the story with her friends."
];
$("loadLesson").addEventListener("click",()=>{
  if(training)return;
  $("trainingText").value=lesson.join("\n");
  updateCorpusSize();
  logDiagnostic("INFO","Training data","Loaded a varied sentence-pattern lesson for the 64-character model.");
});
$("trainingText").addEventListener("input",updateCorpusSize);
$("startTrain").addEventListener("click",()=>safeRun(async()=>{
  activeTrainingMode="text";
  clearError();
  const text=$("trainingText").value;
  const epochs=Number($("epochInput").value);
  if(!/^[\x00-\x7F]*$/.test(text))throw new Error("Corpus must be ASCII text for v0.1.2.");
  if(text.length<40||text.length>250000)throw new Error("Training text must have between 40 and 250,000 bytes.");
  if(!Number.isInteger(epochs)||epochs<1||epochs>25)throw new Error("Choose 1–25 epochs.");
  $("trainingLog").textContent="";
  $("progressText").textContent="Submitting training request to Rust…";
  $("progressPercent").textContent="0%";
  $("progressFill").style.width="0%";
  setTraining(true); // Set before invoking so a quick completion event cannot race with UI state.
  try{
    await invoke("neural_train",{request:{text,epochs}});
    logDiagnostic("INFO","Training","Rust accepted "+text.length+" ASCII bytes over "+epochs+" epoch(s).");
    await pollBrainStatus(); // Also works when live Tauri events are unavailable.
  }catch(error){
    setTraining(false);
    throw error;
  }
},"Start training"));
$("stopTrain").addEventListener("click",()=>safeRun(async()=>{
  const stopping=await invoke("neural_stop");
  if(stopping){
    appendLog("Stop requested — waiting for the current batch to finish.");
    logDiagnostic("INFO","Training","Cancellation requested.");
  }
},"Stop training"));
$("resetBtn").addEventListener("click",()=>safeRun(async()=>{
  if(!confirm("Reset Neural Nova's weights and delete its separate experiment checkpoint? Original Nova will not be affected."))return;
  clearError();
  updateStatus(await invoke("neural_reset"));
  latestTrace=null;
  $("generatedOutput").textContent="Neural brain reset; weights are random again.";
  appendLog("New random network initialized.");
  await inspect();
}));

// Training examples use tab-separated lines. We show each user prompt and
// desired response before committing gradient updates; none are a lookup table.
const originalDialogueLessons=$("dialogueData").value;
function addCurriculumPair(user,assistant){
  const question=(user||"").trim(), answer=(assistant||"").trim();
  if(!question||!answer)throw new Error("Both the question and ideal response are required.");
  if(!/^[\x20-\x7e]*$/.test(question)||!/^[\x20-\x7e]*$/.test(answer)){
    throw new Error("Dialogue examples must contain only printable ASCII characters.");
  }
  if(question.length>180||answer.length>128){
    throw new Error("Question limit 180 ASCII bytes; response limit 128.");
  }
  const row=question+"\t"+answer;
  const current=$("dialogueData").value.trim();
  if(current.split("\n").includes(row)){
    logDiagnostic("WARN","Dialogue School","That exact lesson already exists.");
    return;
  }
  $("dialogueData").value=(current?current+"\n":"")+row;
  logDiagnostic("INFO","Dialogue School","Added one new supervised response example.");
}
$("addDialoguePair").addEventListener("click",()=>safeRun(()=>{
  addCurriculumPair($("teacherQuestion").value,$("teacherAnswer").value);
  $("teacherAnswer").value="";
},"Add dialogue lesson"));
$("restoreDialogueSample").addEventListener("click",()=>safeRun(()=>{
  if(training)throw new Error("Wait until training stops before changing the lesson set.");
  if(!confirm("Restore the initial dialogue lesson examples? Your current lesson text will be replaced."))return;
  $("dialogueData").value=originalDialogueLessons;
  logDiagnostic("INFO","Dialogue School","Restored the built-in example curriculum.");
},"Restore dialogue lessons"));
$("trainDialogues").addEventListener("click",()=>safeRun(async()=>{
  if(training)throw new Error("Another training session is already running.");
  const text=$("dialogueData").value;
  const epochs=Number($("dialogueEpochs").value);
  if(!Number.isInteger(epochs)||epochs<1||epochs>25)throw new Error("Use 1–25 training epochs.");
  if(!text.trim())throw new Error("Add at least one dialogue example.");
  if(/[^\x00-\x7F]/.test(text))throw new Error("Dialogue curriculum must be ASCII for v0.1.3.");
  activeTrainingMode="dialogue";
  setTraining(true);
  $("progressFill").style.width="0%";
  $("progressPercent").textContent="0%";
  $("dialogueValidationLoss").textContent="—";
  $("dialogueValidationAccuracy").textContent="—";
  $("trainingLog").textContent="";
  $("progressText").textContent="Starting response-supervised dialogue training…";
  try{
    await invoke("neural_train_dialogues",{request:{text,epochs}});
    logDiagnostic("INFO","Dialogue School","Training request accepted by Rust: "+text.split("\n").length+" lesson lines, "+epochs+" epochs.");
    await pollBrainStatus();
  }catch(error){setTraining(false);throw error;}
},"Train dialogues"));
function addTranscriptLine(role,text){
  const output=$("dialogueTranscript");
  const placeholder=output.querySelector(".chat-placeholder");
  if(placeholder)placeholder.remove();
  const line=document.createElement("div");
  line.className="dialogue-line "+(role==="user"?"from-user":"from-nova");
  const title=document.createElement("strong");
  title.textContent=role==="user"?"You":"Nova (neural)";
  const content=document.createElement("div");
  content.textContent=text||"(Nova generated an immediate end-of-reply character)";
  line.append(title,content);
  output.append(line);
  output.scrollTop=output.scrollHeight;
}
$("sendDialogue").addEventListener("click",()=>safeRun(async()=>{
  const message=$("dialogueMessage").value.trim();
  if(!message)throw new Error("Enter a message before asking Nova.");
  if(training)throw new Error("Finish training first.");
  if(!/^[\x20-\x7E]+$/.test(message)||message.length>180){
    throw new Error("Messages must be printable ASCII, 1–180 characters.");
  }
  $("sendDialogue").disabled=true;
  lastDialogueQuestion=message;
  $("teacherQuestion").value=message;
  $("correctDialogue").disabled=false;
  addTranscriptLine("user",message);
  try{
    const result=await invoke("neural_chat",{message,length:128,temperature:0.45});
    addTranscriptLine("nova",result.text);
    showTrace(result.trace);
    updateStatus(result.status);
    $("dialogueMessage").value="";
  }finally{$("sendDialogue").disabled=training;}
},"Talk to Nova"));
$("dialogueMessage").addEventListener("keydown",event=>{
  if(event.key==="Enter"){event.preventDefault();$("sendDialogue").click();}
});
$("correctDialogue").addEventListener("click",()=>safeRun(()=>{
  if(!lastDialogueQuestion)throw new Error("Ask Nova a question first.");
  $("teacherQuestion").value=lastDialogueQuestion;
  $("teacherAnswer").focus();
  $("teacherAnswer").scrollIntoView({block:"center",behavior:"smooth"});
  logDiagnostic("INFO","Teacher","Type a corrected response and click Add lesson, then Train dialogue lessons.");
},"Correct reply"));
$("clearDialogue").addEventListener("click",()=>{
  $("dialogueTranscript").replaceChildren();
  lastDialogueQuestion="";
  $("correctDialogue").disabled=true;
  logDiagnostic("INFO","Chat","Cleared displayed chat history. Neural weights were not changed.");
});

async function registerTrainingEvents(){
  const events=eventAPI();
  if(!events?.listen){
    logDiagnostic("WARN","Tauri events","Native event API unavailable; periodic status polling enabled.");
    return;
  }
  try{
    // These require the explicit core:event:allow-listen Tauri capability.
    await events.listen("neural-progress",event=>safeRun(async()=>{
      updateProgress(event.payload);
    },"Training progress event"));
    await events.listen("neural-epoch",event=>safeRun(async()=>{
      const p=event.payload;
      const dialogue=p.mode==="dialogue";
      const vl=Number.isFinite(p.validationLoss)?p.validationLoss.toFixed(4):"N/A";
      const va=Number.isFinite(p.validationAccuracy)?(p.validationAccuracy*100).toFixed(1)+"%":"N/A";
      const message=(dialogue?"Dialogue ":"Text ")+"epoch "+p.epoch+"/"+p.epochs+
        " · training loss "+p.loss.toFixed(4)+" · held-out loss "+vl+" · accuracy "+va;
      appendLog(message);
      if(dialogue){
        $("dialogueValidationLoss").textContent=vl;
        $("dialogueValidationAccuracy").textContent=va;
      }else{
        $("validationLoss").textContent=vl;
        $("validationAccuracy").textContent=va;
      }
      logDiagnostic("INFO","Training",message);
    },"Training epoch event"));
    await events.listen("neural-done",event=>safeRun(async()=>{
      setTraining(false);
      appendLog(event.payload.cancelled?"Training stopped. Checkpoint saved.":"Training complete. Checkpoint saved.");
      logDiagnostic("INFO","Training",event.payload.cancelled?"Training cancelled; checkpoint saved.":"Training complete; checkpoint saved.");
      updateStatus(await invoke("neural_status"));
      if(!event.payload.cancelled){
        $("progressFill").style.width="100%";
        $("progressPercent").textContent="100%";
      }
    },"Training completion event"));
    await events.listen("neural-error",event=>safeRun(async()=>{
      setTraining(false);
      setError(event.payload,"Rust training worker");
      updateStatus(await invoke("neural_status"));
    },"Training error event"));
    logDiagnostic("INFO","Tauri events","Subscribed to live training progress, epoch, completion and error events.");
  }catch(error){
    setError(error,"Tauri event permissions");
    logDiagnostic("WARN","Tauri events","Event listener registration failed; periodic status polling will continue.");
  }
}
async function boot(){
  updateCorpusSize();
  canvas.addEventListener("click",inspectClick);
  if(typeof ResizeObserver==="function")new ResizeObserver(redraw).observe(canvas);
  window.addEventListener("resize",redraw);
  redraw();
  if(!window.__TAURI__?.core?.invoke){
    setState("Open in app","error");
    setError("Open Neural Nova as an installed Windows or Android app to use its native Rust network.","Startup");
    return;
  }
  logDiagnostic("INFO","Startup","Neural Nova v0.1.3 Dialogue School initialized. Experimental 64-character context.");
  // Start the status fallback independently of event permissions; stalled event delivery
  // must never prevent backend state updates or hide an error.
  setInterval(()=>{if(training||latestStatus?.training)void pollBrainStatus();},1000);
  await registerTrainingEvents();
  try{
    const initial=await invoke("neural_boot");
    updateStatus(initial);
    setTraining(initial.training);
    logDiagnostic("INFO","Startup","Rust neural network loaded. "+initial.parameters.toLocaleString()+" parameters.");
  }catch(error){
    setState("Startup error","error");
    setError(error,"Rust startup");
  }
  try{await inspect();}
  catch(error){setError(error,"Initial brain inspection");}
}
document.addEventListener("DOMContentLoaded",()=>safeRun(boot,"Startup"));
