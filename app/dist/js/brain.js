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
let pendingInspect=0;
let canvasMetrics=null;
const canvas=$("brainCanvas");
const ctx=canvas.getContext("2d");

function setState(label,kind="ready"){
  $("brainStateText").textContent=label;
  $("brainState").className="pill "+(kind==="training"?"running":kind==="error"?"offline":"");
}
function setError(error){
  $("errorMessage").hidden=false;
  $("errorMessage").textContent=typeof error==="string"?error:String(error);
}
function clearError(){$("errorMessage").hidden=true;}
function safeRun(task){return Promise.resolve().then(task).catch(setError);}
function updateStatus(status){
  if(!status)return;
  latestStatus=status;
  $("parameterTotal").textContent=status.parameters.toLocaleString();
  $("neuronTotal").textContent=status.dimensions.reduce((a,b)=>a+b,0).toLocaleString();
  $("stepCount").textContent=status.steps.toLocaleString();
  $("lossValue").textContent=Number.isFinite(status.lastLoss)?status.lastLoss.toFixed(4):"—";
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
  setState(active?"Training":"Ready",active?"training":"ready");
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
  if(file.size>100000){throw new Error("Corpus file exceeds 100,000 bytes. Use a smaller selection for this first experiment.");}
  const text=await file.text();
  if(!/^[\x00-\x7F]*$/.test(text))throw new Error("This version accepts ASCII only. Convert Unicode punctuation before training.");
  $("trainingText").value=text;
  updateCorpusSize();
}));
function updateCorpusSize(){
  $("corpusSize").textContent=$("trainingText").value.length.toLocaleString()+" ASCII bytes";
}
$("trainingText").addEventListener("input",updateCorpusSize);
$("startTrain").addEventListener("click",()=>safeRun(async()=>{
  clearError();
  const text=$("trainingText").value;
  const epochs=Number($("epochInput").value);
  if(!/^[\x00-\x7F]*$/.test(text))throw new Error("Corpus must be ASCII text for v0.1.");
  if(text.length<9||text.length>100000)throw new Error("Training text must have between 9 and 100,000 bytes.");
  if(!Number.isInteger(epochs)||epochs<1||epochs>25)throw new Error("Choose 1–25 epochs.");
  await invoke("neural_train",{request:{text,epochs}});
  setTraining(true);
  $("progressText").textContent="Starting neural training…";
  $("progressPercent").textContent="0%";
  $("progressFill").style.width="0%";
  $("trainingLog").textContent="";
}));
$("stopTrain").addEventListener("click",()=>safeRun(async()=>{
  const stopping=await invoke("neural_stop");
  if(stopping)appendLog("Stop requested — waiting for the current batch to finish.");
}));
$("resetBtn").addEventListener("click",()=>safeRun(async()=>{
  if(!confirm("Reset Neural Nova's weights and delete its separate experiment checkpoint? Original Nova will not be affected."))return;
  clearError();
  updateStatus(await invoke("neural_reset"));
  latestTrace=null;
  $("generatedOutput").textContent="Neural brain reset; weights are random again.";
  appendLog("New random network initialized.");
  await inspect();
}));
async function boot(){
  updateCorpusSize();
  canvas.addEventListener("click",inspectClick);
  new ResizeObserver(redraw).observe(canvas);
  window.addEventListener("resize",redraw);
  if(!window.__TAURI__){
    setState("Open in app","error");
    setError("Open Neural Nova as an installed Windows or Android app to use its native Rust network.");
    redraw();return;
  }
  const events=eventAPI();
  if(!events?.listen)throw new Error("Tauri event API unavailable.");
  await events.listen("neural-progress",event=>updateProgress(event.payload));
  await events.listen("neural-epoch",event=>{
    const p=event.payload;
    appendLog("Epoch "+p.epoch+"/"+p.epochs+" complete · mean loss "+p.loss.toFixed(4));
  });
  await events.listen("neural-done",async event=>{
    setTraining(false);
    appendLog(event.payload.cancelled?"Training stopped. Checkpoint saved.":"Training complete. Checkpoint saved.");
    updateStatus(await invoke("neural_status"));
    if(!event.payload.cancelled){
      $("progressFill").style.width="100%";
      $("progressPercent").textContent="100%";
    }
  });
  await events.listen("neural-error",async event=>{
    setTraining(false);setError(event.payload);
    updateStatus(await invoke("neural_status"));
  });
  updateStatus(await invoke("neural_boot"));
  await inspect();
}
document.addEventListener("DOMContentLoaded",()=>safeRun(boot));
