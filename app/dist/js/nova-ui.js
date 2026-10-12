"use strict";
// Neural Nova UI: one backend for one jointly trainable brain.
// The visualizer displays real native activations/attention, not simulated activity.
(()=>{
const $=id=>document.getElementById(id);
const svgNS="http://www.w3.org/2000/svg";
const invoke=(name,args={})=>{
  const fn=window.__TAURI__?.core?.invoke;
  if(!fn)throw Error("Launch the installed Neural Nova app to use the Rust neural network.");
  return fn(name,args);
};
const dialogueSample=[
"hello nova\tHello!","hi nova\tHi there!","good morning\tGood morning!",
"good night\tSleep well!","what is your name\tMy name is Nova.",
"who are you\tI am Neural Nova.","what is rain\tRain is falling water.",
"what is snow\tSnow is frozen water.","what is a tree\tA tree is a plant.",
"what is an apple\tAn apple is a fruit.","what is a dog\tA dog is an animal.",
"what is a cat\tA cat is an animal.","what is two plus two\tFour.",
"how are you\tI am learning.","thank you\tYou are welcome.",
"can you learn\tI learn from examples.","goodbye nova\tGoodbye.",
"what color is the sky\tThe sky is often blue."
].join("\n");
const textSample=[
"The curious fox followed a lantern through the dark forest.",
"When the rain finally stopped, the children walked home together.",
"Although the road was wet, the driver approached the corner carefully.",
"Nova studies recurring patterns in different kinds of sentences.",
"Questions contain punctuation. Replies may contain several words.",
"A small neural network must encounter many examples to generalize.",
"The scientist measured the result before planning the next experiment."
].join("\n");
const datasets={dialogue:dialogueSample,text:textSample};
let activeMode="dialogue", initialized=false,running=false,busy=false,latestStatus=null,latestTrace=null;
let lastQuestion="",lastReply="";
const debugEntries=[];let errorCount=0;
function report(level,source,message){
  const item={time:new Date().toLocaleTimeString(),level,source,message:String(message?.stack||message)};
  debugEntries.push(item);if(debugEntries.length>180)debugEntries.shift();
  if(level==="ERROR")errorCount++;
  $("diagnosticLog").textContent=debugEntries.map(e=>"["+e.time+"] ["+e.level+"] ["+e.source+"] "+e.message).join("\n\n");
  $("diagnosticLog").scrollTop=$("diagnosticLog").scrollHeight;
  $("diagnosticCount").textContent=errorCount+" error"+(errorCount===1?"":"s")+" · "+debugEntries.length+" entries";
  $("diagnostics").classList.toggle("has-errors",errorCount>0);
  if(level==="ERROR"){$("diagnostics").open=true;console.error(source,message);}
}
async function safe(task,source="UI"){try{return await task();}catch(e){report("ERROR",source,e);return null;}}
window.addEventListener("error",e=>report("ERROR","JavaScript",e.message+(e.filename?" @"+e.filename+":"+e.lineno:"")));
window.addEventListener("unhandledrejection",e=>report("ERROR","Promise",e.reason));
function tab(name){
  for(const button of document.querySelectorAll(".nav-tab")){
    const selected=button.dataset.tab===name;
    button.classList.toggle("selected",selected);
    button.setAttribute("aria-selected",String(selected));
    button.tabIndex=selected?0:-1;
  }
  for(const panel of document.querySelectorAll(".tab-pane"))panel.hidden=panel.id!=="pane-"+name;
}
document.querySelectorAll(".nav-tab").forEach(button=>{
  button.addEventListener("click",()=>tab(button.dataset.tab));
  button.addEventListener("keydown",e=>{
    if(!["ArrowRight","ArrowLeft","Home","End"].includes(e.key))return;
    e.preventDefault();const tabs=[...document.querySelectorAll(".nav-tab")];
    const idx=tabs.indexOf(button);const next=e.key==="Home"?0:e.key==="End"?2:(idx+(e.key==="ArrowRight"?1:2))%3;
    tabs[next].focus();tab(tabs[next].dataset.tab);
  });
});
function setState(text,state="ok"){
  $("brainStateText").textContent=text;
  $("brainState").className="status-pill"+(state==="running"?" running":state==="error"?" error":"");
}
function setRunning(value){
  running=Boolean(value);
  $("startTraining").disabled=running||!initialized;
  $("stopTraining").disabled=!running;
  $("resetBrain").disabled=running||!initialized;
  $("sendMessage").disabled=running||!initialized||busy;
  $("inspectBrain").disabled=running||!initialized;
  setState(running?"Nova is learning":initialized?"Nova ready":"Loading…",running?"running":"ok");
}
function status(s){
  if(!s)return;
  latestStatus=s;initialized=true;
  $("totalParams").textContent=s.parameters.toLocaleString();
  $("transformerParams").textContent=s.transformerParameters.toLocaleString();
  $("denseParams").textContent=s.denseParameters.toLocaleString();
  $("trainingSteps").textContent=s.transformerStep.toLocaleString();
  $("heldoutLoss").textContent=Number.isFinite(s.heldoutLoss)?s.heldoutLoss.toFixed(4):"—";
  $("heldoutAccuracy").textContent=Number.isFinite(s.heldoutAccuracy)?(s.heldoutAccuracy*100).toFixed(1)+"%":"—";
  $("checkpointText").textContent=s.trained?"Learned weights saved / active":"Fresh untrained weights";
  if(s.total){
    const fraction=Math.min(1,s.completed/s.total);
    $("trainingFill").style.width=(fraction*100).toFixed(1)+"%";
    $("trainingPercent").textContent=(fraction*100).toFixed(1)+"%";
    $("trainingProgress").textContent="Epoch "+s.epoch+"/"+s.epochs+" · "+s.completed+"/"+s.total+" samples";
  }
  if(s.error && s.error!==status.lastError){status.lastError=s.error;report("ERROR","Rust",s.error);}
  setRunning(s.training);
}
function rememberDataset(){datasets[activeMode]=$("lessonData").value;try{localStorage.setItem("nova-hybrid-"+activeMode,datasets[activeMode]);}catch(_){}}
function modeChanged(mode){
  if(!datasets[mode])datasets[mode]=mode==="dialogue"?dialogueSample:textSample;
  activeMode=mode;
  $("lessonData").value=datasets[mode];
  $("lessonHint").textContent=mode==="dialogue"
    ?"One question and ideal reply per line, separated by a TAB. At least five distinct pairs; whole pairs are held out."
    :"120–14,000 ASCII bytes. Nova predicts each next character, with the final 10% held out.";
  countData();
}
function countData(){
  const input=$("lessonData").value;
  $("lessonCount").textContent=activeMode==="dialogue"
    ?input.split("\n").filter(line=>line.includes("\t")&&line.trim()).length+" lesson pairs"
    :new TextEncoder().encode(input).length+" bytes";
}
$("lessonMode").addEventListener("change",e=>{rememberDataset();modeChanged(e.target.value);});
$("lessonData").addEventListener("input",()=>{rememberDataset();countData();});
$("lessonFile").addEventListener("change",()=>safe(async()=>{
  const file=$("lessonFile").files[0];if(!file)return;
  const text=await file.text();
  if(!/^[\x00-\x7f]*$/.test(text))throw Error("The experimental tokenizer currently accepts ASCII text only.");
  $("lessonData").value=text;rememberDataset();countData();
  report("INFO","Lesson","Loaded "+file.name+".");
},"File import"));
$("sampleLesson").addEventListener("click",()=>{
  $("lessonData").value=activeMode==="dialogue"?dialogueSample:textSample;
  rememberDataset();countData();
});
$("clearLesson").addEventListener("click",()=>{$("lessonData").value="";rememberDataset();countData();});
$("addLesson").addEventListener("click",()=>safe(async()=>{
  const question=$("teacherQuestion").value.trim();
  const answer=$("teacherAnswer").value.trim();
  if(!question||!answer)throw Error("Supply both a question and ideal response.");
  if(!/^[\x20-\x7e]+$/.test(question+answer))throw Error("Lesson fields must contain printable ASCII characters.");
  if(activeMode!=="dialogue"){rememberDataset();$("lessonMode").value="dialogue";modeChanged("dialogue");}
  const existing=$("lessonData").value.trimEnd();
  $("lessonData").value=existing+(existing?"\n":"")+question+"\t"+answer;
  $("teacherQuestion").value="";$("teacherAnswer").value="";
  rememberDataset();countData();report("INFO","Teacher","Added correction to dialogue curriculum (training not yet started).");
},"Teacher"));
function history(line){
  const prev=$("epochHistory").textContent;
  $("epochHistory").textContent=line+"\n"+(prev==="No completed epochs yet."?"":prev);
}
$("startTraining").addEventListener("click",()=>safe(async()=>{
  if(running)return;
  const epochs=Number($("trainingEpochs").value);
  const text=$("lessonData").value;
  if(!Number.isInteger(epochs)||epochs<1||epochs>20)throw Error("Choose 1–20 epochs.");
  if(!text.trim())throw Error("Provide training data.");
  if(!/^[\x00-\x7f]*$/.test(text))throw Error("Training data contains characters outside ASCII.");
  $("epochHistory").textContent="No completed epochs yet.";
  $("trainingProgress").textContent="Preparing training…";$("trainingPercent").textContent="0%";
  $("trainingFill").style.width="0%";
  setRunning(true);
  try{
    await invoke("hybrid_train",{lesson:{mode:activeMode,text,epochs}});
    report("INFO","Training","Joint training started: "+activeMode+" · "+epochs+" epochs.");
    await poll();
  }catch(e){setRunning(false);throw e;}
},"Training"));
$("stopTraining").addEventListener("click",()=>safe(async()=>{
  if(await invoke("hybrid_stop"))report("INFO","Training","Stop requested; the current epoch checkpoint will be saved.");
},"Training"));
$("resetBrain").addEventListener("click",()=>safe(async()=>{
  if(!confirm("Reset BOTH hemispheres, the learned bridge, and the joint checkpoint? Independent older checkpoints are not deleted."))return;
  status(await invoke("hybrid_reset"));
  latestTrace=null;clearVisual();
  report("INFO","Brain","Reset unified model weights and optimizer.");
},"Reset"));
const transcript=$("chatTranscript");
function bubble(role,message){
  transcript.querySelector(".chat-placeholder")?.remove();
  const entry=document.createElement("div");entry.className="chat-line "+role;
  const name=document.createElement("strong");name.textContent=role==="user"?"You":"Nova";
  const content=document.createElement("div");content.textContent=message||"(Predicted end of reply)";
  entry.append(name,content);transcript.append(entry);
  transcript.scrollTop=transcript.scrollHeight;
}
$("sendMessage").addEventListener("click",()=>safe(async()=>{
  const message=$("chatMessage").value.trim();
  if(!/^[\x20-\x7e]{1,180}$/.test(message))throw Error("Message must be 1–180 printable ASCII characters.");
  if(running||busy)return;
  busy=true;setRunning(running);
  lastQuestion=message;lastReply="";
  bubble("user",message);
  $("chatMessage").value="";
  try{
    const reply=await invoke("hybrid_chat",{message,maxChars:128,temperature:0.6});
    bubble("nova",reply.response);
    lastReply=reply.response;
    $("correctReply").disabled=false;
    status(reply.status);
    showVisual(reply.trace);
    $("inspectMessage").value=message;
  }finally{busy=false;setRunning(running);}
},"Chat"));
$("chatMessage").addEventListener("keydown",e=>{
  if(e.key==="Enter"){e.preventDefault();$("sendMessage").click();}
});
$("clearChat").addEventListener("click",()=>{
  transcript.replaceChildren();
  const placeholder=document.createElement("div");placeholder.className="chat-placeholder";
  placeholder.textContent="Conversation display cleared. This version still uses a single-turn model context.";
  transcript.append(placeholder);lastQuestion="";lastReply="";$("correctReply").disabled=true;
});
$("correctReply").addEventListener("click",()=>{
  if(!lastQuestion)return;
  tab("teach");
  $("lessonMode").value="dialogue";rememberDataset();modeChanged("dialogue");
  $("teacherQuestion").value=lastQuestion;$("teacherAnswer").value="";
  const details=document.querySelector(".card .teacher-extra");
  details.open=true;$("teacherAnswer").focus();
});
function clearVisual(){
  $("leftActivity").replaceChildren();$("rightActivity").replaceChildren();
  $("attentionTiles").textContent="No attention observed.";
  $("predictions").textContent="Run an inspection to view predictions.";
  $("attentionCount").textContent="—";
  $("visualizerDetail").textContent="Generate or inspect a message to show measured neural activity.";
}
function neuron(x,y,magnitude,hue,label,detail){
  const c=document.createElementNS(svgNS,"circle");
  const level=Math.min(1,Math.max(0,magnitude));
  c.setAttribute("cx",x.toFixed(1));c.setAttribute("cy",y.toFixed(1));
  c.setAttribute("r",(1.7+level*4.5).toFixed(1));
  c.setAttribute("fill",hue);
  c.setAttribute("opacity",(.16+level*.82).toFixed(2));
  const title=document.createElementNS(svgNS,"title");title.textContent=detail;
  c.append(title);
  c.addEventListener("click",()=>$("visualizerDetail").textContent=detail);
  return c;
}
function nodeLocation(i,n,centerX){
  const angle=i*2.399963229728653;
  const radius=Math.sqrt((i+.5)/(n+.3));
  return {x:centerX+Math.cos(angle)*122*radius,y:207+Math.sin(angle)*119*radius};
}
function charName(code){return code===10?"↵":code===32?"SPACE":code<32?"0x"+code.toString(16):String.fromCharCode(code);}
function showVisual(trace){
  if(!trace)return;latestTrace=trace;
  const left=$("leftActivity"),right=$("rightActivity");
  left.replaceChildren();right.replaceChildren();
  const attention=trace.attention||[];
  const strongest=Math.max(1e-9,...attention.map(x=>x.weight));
  attention.forEach((data,i)=>{
    const p=nodeLocation(i,attention.length,245);
    const detail="Language attention · token "+data.index+" ("+data.character+") · "+(data.weight*100).toFixed(3)+"%";
    left.append(neuron(p.x,p.y,data.weight/strongest,"#65caff","attention",detail));
  });
  const layers=trace.denseLayers||[];
  // Display a deterministic subsample of actual neurons, not invented activation values.
  const dots=[];
  layers.forEach((layer,idx)=>{
    const peak=Math.max(1e-7,...layer.map(x=>Math.abs(x)));
    const maxPoints=idx===0?40:idx===3?32:52;
    const stride=Math.max(1,Math.ceil(layer.length/maxPoints));
    for(let i=0;i<layer.length;i+=stride) {
      dots.push({layer:idx,index:i,value:layer[i],level:Math.abs(layer[i])/peak});
    }
  });
  dots.forEach((data,i)=>{
    const p=nodeLocation(i,dots.length,555);
    const detail="Dense pathway · layer "+data.layer+" · neuron "+data.index+
      " · measured activation "+data.value.toFixed(6);
    right.append(neuron(p.x,p.y,data.level,"#c09aff","neuron",detail));
  });
  $("attentionCount").textContent=attention.length.toLocaleString();
  const tiles=$("attentionTiles");tiles.replaceChildren();
  attention.forEach(data=>{
    const span=document.createElement("span");
    span.className="attention-tile";span.textContent=data.character;
    span.title="Token "+data.index+" · "+(data.weight*100).toFixed(3)+"%";
    span.style.background="rgba(50,188,150,"+(.11+.8*data.weight/strongest).toFixed(3)+")";
    tiles.append(span);
  });
  const predictions=$("predictions");predictions.replaceChildren();
  for(const t of trace.topPredictions||[]){
    const chip=document.createElement("span");chip.className="prediction-token";
    const name=document.createElement("b");name.textContent=charName(t.code)+" ";
    chip.append(name,document.createTextNode((t.probability*100).toFixed(1)+"%"));
    predictions.append(chip);
  }
  const bridge=trace.bridgeSignal||[];
  const maximum=Math.max(1e-8,...bridge.map(Math.abs));
  $("bridgeFibers").querySelectorAll("path").forEach((path,i)=>{
    const start=Math.floor(i*bridge.length/5),end=Math.floor((i+1)*bridge.length/5);
    const mean=bridge.slice(start,end).reduce((sum,v)=>sum+Math.abs(v),0)/Math.max(1,end-start);
    const intensity=Math.min(1,mean/maximum);
    path.style.opacity=(.12+.85*intensity).toFixed(2);
    path.style.strokeWidth=(1+3*intensity).toFixed(2);
  });
  $("visualizerDetail").textContent=
    "Measured "+attention.length+" Transformer attention positions and sampled activations from "+
    layers.reduce((sum,l)=>sum+l.length,0).toLocaleString()+
    " dense neurons. The center fibers show learned bridge signal magnitudes (32 channels). This is a schematic, not a neurological map.";
}
$("inspectBrain").addEventListener("click",()=>safe(async()=>{
  const message=$("inspectMessage").value.trim();
  if(!message)throw Error("Enter a message to inspect.");
  showVisual(await invoke("hybrid_inspect",{message}));
},"Brain inspection"));
$("inspectMessage").addEventListener("keydown",e=>{if(e.key==="Enter"){e.preventDefault();$("inspectBrain").click();}});
$("copyDiagnostics").addEventListener("click",()=>safe(async()=>{
  const text="Neural Nova hybrid v0.3.0\n"+JSON.stringify(latestStatus)+"\n"+
    debugEntries.map(e=>"["+e.time+"] ["+e.level+"] ["+e.source+"] "+e.message).join("\n\n");
  await navigator.clipboard.writeText(text);
  report("INFO","Debugger","Copied diagnostic report.");
},"Debugger"));
$("clearDiagnostics").addEventListener("click",()=>{
  debugEntries.length=0;errorCount=0;$("diagnosticLog").textContent="No diagnostics recorded in this session.";
  $("diagnosticCount").textContent="0 errors · 0 entries";$("diagnostics").classList.remove("has-errors");
});
let polling=false;
async function poll(){
  if(polling)return;polling=true;
  try{const result=await invoke("hybrid_status");status(result);}
  catch(e){report("ERROR","Status polling",e);}
  finally{polling=false;}
}
async function listen(){
  const events=window.__TAURI__?.event;
  if(!events?.listen){report("WARN","Events","Native progress events unavailable; status polling remains enabled.");return;}
  try{
    await events.listen("hybrid-progress",({payload:p})=>{
      $("trainingProgress").textContent="Epoch "+p.epoch+"/"+p.epochs+" · "+p.completed+"/"+p.total+" samples";
      $("trainingPercent").textContent=(100*p.completed/p.total).toFixed(1)+"%";
      $("trainingFill").style.width=(100*p.completed/p.total)+"%";
      $("trainingSteps").textContent=p.step.toLocaleString();
    });
    await events.listen("hybrid-epoch",({payload:p})=>{
      history("Epoch "+p.epoch+"/"+p.epochs+" | train "+p.trainLoss.toFixed(4)+
        " | held-out "+p.validationLoss.toFixed(4)+" | accuracy "+(100*p.validationAccuracy).toFixed(1)+"%");
      $("heldoutLoss").textContent=p.validationLoss.toFixed(4);
      $("heldoutAccuracy").textContent=(100*p.validationAccuracy).toFixed(1)+"%";
    });
    await events.listen("hybrid-done",({payload:p})=>{
      setRunning(false);report("INFO","Training",p.cancelled?"Training stopped; checkpoint saved.":"Shared training completed; checkpoint saved.");
      void poll();
    });
    await events.listen("hybrid-error",({payload:p})=>{setRunning(false);report("ERROR","Rust training",p);void poll();});
    report("INFO","Events","Listening for unified Nova training progress, epoch, completion and errors.");
  }catch(e){report("ERROR","Native events",e);}
}
async function boot(){
  for(const mode of ["dialogue","text"]){
    try{const v=localStorage.getItem("nova-hybrid-"+mode);if(v!==null)datasets[mode]=v;}catch(_){}
  }
  modeChanged("dialogue");setRunning(false);
  if(!window.__TAURI__?.core?.invoke){setState("Installed app required","error");report("ERROR","Startup","Run the installed Windows or Android app for Rust neural learning.");return;}
  await listen();
  try{
    status(await invoke("hybrid_boot"));
    report("INFO","Brain","Unified trainable Transformer + dense brain and neural bridge initialized.");
  }catch(e){setState("Startup error","error");report("ERROR","Startup",e);}
  setInterval(()=>{if(running)void poll();},1000);
}
void boot();
})();