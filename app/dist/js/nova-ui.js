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
const conversationSample=[
"U: Hello, I am Mira.","N: Hello Mira.","U: What name did I tell you?","N: You told me Mira.",
"",
"U: My box is red.","N: The box is red.","U: What color is my box?","N: Your box is red.",
"",
"U: I like apples.","N: You like apples.","U: What fruit did I mention?","N: Apples.",
"",
"U: My dog is called Luna.","N: Luna is your dog.","U: What is my dog's name?","N: Luna.",
"",
"U: I am going home.","N: You are going home.","U: Where am I going?","N: Home.",
"",
"U: The tree is tall.","N: The tree is tall.","U: What did I say was tall?","N: The tree."
].join("\n");
const reasoningSample=[
"The amber key opens the amber door.\tWhich key opens the amber door?\tThe amber key.\tThe violet key.",
"The violet key opens the violet door.\tWhich key opens the violet door?\tThe violet key.\tThe amber key.",
"The silver box contains an apple.\tWhat is in the silver box?\tAn apple.\tA stone.",
"The green box contains a stone.\tWhat is in the green box?\tA stone.\tAn apple.",
"Mira is north of Tavik.\tWho is south of Mira?\tTavik.\tMira.",
"Tavik is south of Mira.\tWho is north of Tavik?\tMira.\tTavik.",
"The copper switch turns on the lamp.\tWhat turns on the lamp?\tThe copper switch.\tThe bell.",
"The bell rings when touched.\tWhat rings when touched?\tThe bell.\tThe copper switch."
].join("\n");
const datasets={dialogue:dialogueSample,text:textSample,conversation:conversationSample,reasoning:reasoningSample};
const TOKENIZER_BASELINE_KEY="neural-nova-tokenizer-experiment-v1";
let latestTokenizerProbe=null;

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
  $("inspectBrain").disabled=running||!initialized||busy;
  $("compareContext").disabled=running||!initialized||busy;
  for(const id of ["chatLength","chatTemperature","chatContext","chatSampling"]){
    $(id).disabled=running||busy||(id==="chatTemperature"&&$("chatSampling").value==="greedy");
  }
  for(const id of ["adaptiveTraining","earlyStopping","trainingRate","trainingEpochs"]){
    $(id).disabled=running;
  }
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
  $("checkpointText").textContent=s.trained?"Joint learned weights saved":"Fresh untrained weights";
  $("memoryStatus").textContent=(s.memoryTurns||0)+"/12 memory slots · "+(s.tokenizerWords||0)+" learned word units";
  $("brainMemoryInfo").textContent="Memory slots: "+(s.memoryTurns||0)+
    " / 12 · learned word units: "+(s.tokenizerWords||0)+
    ". The visualizer shows actual token attention and shared neural activations.";
  $("trainingStage").textContent=s.stage||"Idle";
  $("lessonSplit").textContent=(s.trainLessons||0)+" training / "+(s.validationLessons||0)+" held-out lessons";
  $("trainingSpeed").textContent=s.tokensPerSecond>0?"Throughput: "+s.tokensPerSecond.toFixed(1)+" tokens/s":"Throughput: —";
  $("retentionStatus").textContent=Number.isFinite(s.retentionAccuracy)
    ?"Earlier-skill greedy accuracy: "+(100*s.retentionAccuracy).toFixed(1)+
      "% (baseline "+(100*s.retentionBaseline).toFixed(1)+
      "%). When protection is on, any decline prevents selection of that checkpoint."
    :"Choose a saved teacher benchmark to protect specific earlier answers (optional).";
  $("bestEpoch").textContent=(s.bestEpoch>0?"Best epoch: "+s.bestEpoch:"Best epoch: baseline")+
    (Number.isFinite(s.bestHeldoutLoss)?" · loss "+s.bestHeldoutLoss.toFixed(4):"");

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
  if(!(mode in datasets))datasets[mode]=mode==="dialogue"?dialogueSample:mode==="conversation"?conversationSample:mode==="reasoning"?reasoningSample:textSample;
  activeMode=mode;
  $("lessonData").value=datasets[mode];
  $("lessonHint").textContent=mode==="dialogue"
    ?"Up to 1,024 dialogue pairs, 384 ASCII characters per reply; one TAB between question and answer. Validation holds out whole question groups."
    :mode==="conversation"
    ?"Multi-turn: write U: and N: lines, separate sessions with blank lines or ---. At least 5 independent sessions, each with 2+ turns. Held-out sessions test neural memory."
    :mode==="reasoning"
    ?"Reasoning: one TAB-separated PREMISE, QUESTION, SUPPORTED answer, and CONTRADICTED answer per line. At least five distinct problems. The shared brain learns a whole-answer preference in addition to ordinary text."
    :"120–50,000 ASCII bytes, with at least five distinct paragraphs (blank-line separated), sentences, or lines. Entire sections are independently held out by a stable split; no training context crosses into validation.";
  countData();
}
function countData(){
  const input=$("lessonData").value;
  $("lessonCount").textContent=activeMode==="dialogue"
    ?input.split("\n").filter(line=>line.includes("\t")&&line.trim()).length+" lesson pairs"
    :activeMode==="conversation"
      ?input.split(/\n\s*\n|\n---\n/).filter(block=>block.trim()).length+" conversation sessions"
      :new TextEncoder().encode(input).length+" bytes";
}
const TRAINER_SETTINGS_KEY="neural-nova-0.3.2-trainer-settings";
function describeSchedule(){
  const rate=$("trainingRate").value;
  const adaptive=$("adaptiveTraining").checked;
  const earlyStop=$("earlyStopping").checked;
  const lr=adaptive?"Starts at "+rate+"; may only decrease if validation stalls.":"Stays fixed at "+rate+" for every epoch.";
  const stop=earlyStop?" Early stopping independently watches validation loss (6 epochs without improvement).":
    " Training runs up to the maximum epochs unless you press Stop.";
  $("trainingScheduleHelp").textContent=lr+stop;
}
function saveSchedule(){
  describeSchedule();
  try{
    localStorage.setItem(TRAINER_SETTINGS_KEY,JSON.stringify({
      adaptiveLearningRate:$("adaptiveTraining").checked,
      earlyStopping:$("earlyStopping").checked,
      learningRate:$("trainingRate").value,
      rehearsal:$("rehearsalEnabled").checked,
      protectPrevious:$("retentionGuardEnabled").checked,
      retentionLossTolerance:Number($("retentionTolerance").value)
    }));
  }catch(_){}
}
function loadSchedule(){
  try{
    const p=JSON.parse(localStorage.getItem(TRAINER_SETTINGS_KEY)||"null");
    if(p && typeof p==="object"){
      if(typeof p.adaptiveLearningRate==="boolean")$("adaptiveTraining").checked=p.adaptiveLearningRate;
      if(typeof p.earlyStopping==="boolean")$("earlyStopping").checked=p.earlyStopping;
      if(["0.0001","0.0003","0.001","0.003","0.005"].includes(p.learningRate))$("trainingRate").value=p.learningRate;
      if(typeof p.rehearsal==="boolean")$("rehearsalEnabled").checked=p.rehearsal;
      if(typeof p.protectPrevious==="boolean")$("retentionGuardEnabled").checked=p.protectPrevious;
      if([0.05,0.10,0.20].includes(p.retentionLossTolerance))
        $("retentionTolerance").value=p.retentionLossTolerance.toFixed(2);
    }
  }catch(_){}
  describeSchedule();
}
for(const id of ["adaptiveTraining","earlyStopping","trainingRate",
  "rehearsalEnabled","retentionGuardEnabled","retentionTolerance"]){
  $(id).addEventListener("change",saveSchedule);
}
async function updateRehearsalBank(){
  const bank=await invoke("hybrid_rehearsal_info");
  $("rehearsalBankStatus").textContent=bank.names.length
    ?"Saved past lessons: "+bank.names.map((n,i)=>n+" ("+bank.modes[i]+")").join(" · ")
    :"No previous lessons saved for these weights. Future accepted lessons will be archived automatically.";
}
$("priorLessonFile").addEventListener("change",()=>safe(async()=>{
  const f=$("priorLessonFile").files[0];if(!f)return;
  const data=await f.text();
  if(data.length>50000||!/^[\x00-\x7f]*$/.test(data))throw Error("Prior TXT must be ASCII and at most 50 KB");
  $("priorLessonData").value=data;
  if(!$("priorLessonName").value)$("priorLessonName").value=f.name.replace(/\.txt$/i,"");
},"Prior lesson import"));
$("archivePriorLesson").addEventListener("click",()=>safe(async()=>{
  if(running)return;
  if(!confirm("Archive this previously taught lesson as neural rehearsal? Only its own training split will be replayed. The model is not trained by archiving."))return;
  await invoke("hybrid_rehearsal_add",{
    mode:$("priorLessonMode").value,
    text:$("priorLessonData").value,
    name:$("priorLessonName").value.trim()
  });
  await updateRehearsalBank();
  report("INFO","Rehearsal","Added previous lesson to teacher replay bank; no neural changes.");
},"Previous lesson"));
$("clearRehearsal").addEventListener("click",()=>safe(async()=>{
  if(running)return;
  if(!confirm("Clear past lesson replay data for Nova? Neural weights are NOT reset."))return;
  await invoke("hybrid_rehearsal_clear");
  await updateRehearsalBank();
},"Clear rehearsal"));
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
  $("lessonData").value=activeMode==="dialogue"?dialogueSample:activeMode==="conversation"?conversationSample:activeMode==="reasoning"?reasoningSample:textSample;
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
  if(running||window.NovaMassLearning?.active)return;
  const epochs=Number($("trainingEpochs").value);
  const text=$("lessonData").value;
  const learningRate=Number($("trainingRate").value);
  const adaptiveLearningRate=$("adaptiveTraining").checked;
  const earlyStopping=$("earlyStopping").checked;
  if(!Number.isInteger(epochs)||epochs<1||epochs>60)throw Error("Choose 1–60 epochs.");
  if(!text.trim())throw Error("Provide training data.");
  if(!/^[\x00-\x7f]*$/.test(text))throw Error("Training data contains characters outside ASCII.");
  $("epochHistory").textContent="No completed epochs yet.";
  $("trainingProgress").textContent="Preparing training…";$("trainingPercent").textContent="0%";
  $("trainingFill").style.width="0%";
  setRunning(true);
  try{
    await invoke("hybrid_train",{
      lesson:{mode:activeMode,text,epochs},
      options:{adaptiveLearningRate,earlyStopping,learningRate,patience:6,
        retentionTestId:$("retentionBenchmark").value||null,
        rehearsal:$("rehearsalEnabled").checked,
        protectPrevious:$("retentionGuardEnabled").checked,
        retentionLossTolerance:Number($("retentionTolerance").value)}
    });
    report("INFO","Training","Joint training started: "+activeMode+" · max "+epochs+
      " epochs · "+(adaptiveLearningRate?"adaptive":"fixed")+" LR "+learningRate+
      " · early stopping "+(earlyStopping?"on":"off")+".");
    await poll();
  }catch(e){setRunning(false);throw e;}
},"Training"));
$("runTokenizerProbe").addEventListener("click",()=>safe(async()=>{
  if(running||busy)throw Error("Finish training or generation before this investigation.");
  const output=$("tokenizerProbeOutput");
  output.textContent="Testing identical weights with and without new tokenizer units…";
  const result=await invoke("hybrid_tokenizer_probe",{
    mode:activeMode,lessonText:$("lessonData").value,
    probes:$("tokenizerProbePrompts").value,maxChars:48,
    contextChars:Number($("chatContext").value)
  });
  latestTokenizerProbe={
    mode:activeMode,lessonText:$("lessonData").value,
    prompts:$("tokenizerProbePrompts").value,
    window:Number($("chatContext").value),result,
  };
  const rows=[
    "EXPERIMENT A vs B — ZERO GRADIENT UPDATES",
    "New word units: "+result.addedWordUnits,
    "Weights identical: "+result.unchangedWeights,
    ""
  ];
  for(let i=0;i<result.samples.length;i+=2){
    const a=result.samples[i],b=result.samples[i+1];
    const top=sample=>sample.topPredictions.slice(0,5)
      .map(x=>(x.code===10?"NEWLINE":String.fromCharCode(x.code))+" "+(100*x.probability).toFixed(2)+"%").join(", ");
    rows.push("PROMPT: "+a.prompt);
    rows.push("A (checkpoint): step "+a.transformerStep+" / "+a.denseStep+
      ", "+a.vocabulary+" learned words, tokens "+JSON.stringify(a.tokenIds));
    rows.push("  Greedy: "+JSON.stringify(a.response)+" · top: "+top(a));
    rows.push("B (vocab only): step "+b.transformerStep+" / "+b.denseStep+
      ", "+b.vocabulary+" learned words, tokens "+JSON.stringify(b.tokenIds));
    rows.push("  Greedy: "+JSON.stringify(b.response)+" · top: "+top(b));
    rows.push("  Representation changed: "+(JSON.stringify(a.tokenIds)!==JSON.stringify(b.tokenIds))+
      " · response changed: "+(a.response!==b.response));
    rows.push("");
  }
  output.textContent=rows.join("\n");
  report("INFO","Tokenizer probe","Non-mutating A/B finished: "+result.addedWordUnits+
    " newly proposed units, "+result.samples.length/2+" prompts.");
},"Tokenizer-only diagnosis"));
$("saveTokenizerBaseline").addEventListener("click",()=>safe(async()=>{
  if(!latestTokenizerProbe)throw Error("Run the A/B experiment before saving a baseline.");
  const old=localStorage.getItem(TOKENIZER_BASELINE_KEY);
  if(old&&!confirm("Replace the previously saved tokenizer A/B baseline?"))return;
  localStorage.setItem(TOKENIZER_BASELINE_KEY,JSON.stringify({
    ...latestTokenizerProbe,createdAt:new Date().toISOString()
  }));
  report("INFO","Tokenizer diagnosis","Preserved the A/B baseline for comparison with the post-training checkpoint.");
},"Save tokenizer baseline"));
$("compareTokenizerStageC").addEventListener("click",()=>safe(async()=>{
  if(running||busy)throw Error("Finish training before measuring stage C.");
  const baseline=JSON.parse(localStorage.getItem(TOKENIZER_BASELINE_KEY)||"null");
  if(!baseline)throw Error("Save an A/B baseline BEFORE training a new lesson.");
  if(baseline.mode!==activeMode || baseline.lessonText!==$("lessonData").value ||
     baseline.prompts!==$("tokenizerProbePrompts").value ||
     baseline.window!==Number($("chatContext").value)){
    throw Error("Stage C requires the same lesson text, prompts, mode and window as the saved A/B baseline.");
  }
  const current=await invoke("hybrid_tokenizer_probe",{
    mode:activeMode,lessonText:$("lessonData").value,
    probes:baseline.prompts,maxChars:48,contextChars:baseline.window
  });
  const rows=["A / B / C CONTROLLED CHECKPOINT COMPARISON",
    "Baseline saved: "+baseline.createdAt,
    "A = original working model, B = extra tokenizer units only,",
    "C = current model after subsequent neural training.",
    "A/B both have zero additional gradient steps.",
    "C must have a different optimizer step for a meaningful post-training comparison.",""];
  for(let i=0;i<baseline.result.samples.length;i+=2){
    const old=baseline.result.samples[i],extended=baseline.result.samples[i+1];
    const now=current.samples[i];
    const summary=(label,item)=>label+" step "+item.transformerStep+
      ", vocabulary "+item.vocabulary+", IDs "+JSON.stringify(item.tokenIds)+
      ", greedy "+JSON.stringify(item.response)+
      ", first prediction "+(item.topPredictions[0]?String.fromCharCode(item.topPredictions[0].code)+" "+(item.topPredictions[0].probability*100).toFixed(2)+"%":"—");
    rows.push("PROMPT "+old.prompt,
      summary("A",old),summary("B",extended),summary("C",now),
      "Token shift A→B: "+(JSON.stringify(old.tokenIds)!==JSON.stringify(extended.tokenIds))+
      " | Answer A→B changed: "+(old.response!==extended.response)+
      " | Answer A→C changed: "+(old.response!==now.response),"");
  }
  $("tokenizerProbeOutput").textContent=rows.join("\n");
  report("INFO","Tokenizer diagnosis","Completed A/B/C comparison; baseline evidence remains saved.");
},"Tokenizer stage C"));
$("runBridgeProbe").addEventListener("click",()=>safe(async()=>{
  if(running||busy)throw Error("Finish training first.");
  const result=await invoke("hybrid_bridge_probe",{
    message:$("bridgeProbeMessage").value,
    contextChars:Number($("chatContext").value)
  });
  const formatted=(x)=>Number(x).toFixed(6);
  $("bridgeProbeOutput").textContent=[
    "READ-ONLY BRIDGE DIAGNOSTICS",
    "Transformer step: "+result.transformerStep+" · dense step: "+result.denseStep,
    "Transformer hidden norm (before bridge): "+formatted(result.transformerNorm),
    "Dense feature norm: "+formatted(result.denseNorm),
    "Projected bridge norm: "+formatted(result.projectedBridgeNorm),
    "Combined hidden norm: "+formatted(result.combinedNorm),
    "Bridge / Transformer norm ratio: "+formatted(result.bridgeToTransformerRatio),
    "Next-character distribution change (bridge ablation): "+
      (100*result.nextCharacterTotalVariation).toFixed(3)+"% total variation",
    "",
    "The ablation only zeroes the bridge input for a measurement. It does not",
    "train or edit Nova. A large ratio is a clue, not proof of a faulty bridge."
  ].join("\n");
  report("INFO","Bridge probe","Measured actual learned bridge magnitude and no-weight-change ablation.");
},"Neural bridge measurement"));
$("stopTraining").addEventListener("click",()=>safe(async()=>{
  if(await invoke("hybrid_stop"))report("INFO","Training","Stop requested; the current epoch checkpoint will be saved.");
},"Training"));
$("resetBrain").addEventListener("click",()=>safe(async()=>{
  if(!confirm("Reset BOTH hemispheres, the learned bridge, and the joint checkpoint? Independent older checkpoints are not deleted."))return;
  status(await invoke("hybrid_reset"));
  latestTrace=null;clearVisual();
  report("INFO","Brain","Reset unified model weights and optimizer.");
},"Reset"));
const CHAT_PREFS_KEY="neural-nova-0.4-chat-controls";
function chatOptions(){
  const maxChars=Number($("chatLength").value);
  const temperature=Number($("chatTemperature").value);
  const contextChars=Number($("chatContext").value);
  const mode=$("chatSampling").value;
  return {maxChars,temperature,contextChars,
    deterministic:mode==="greedy",
    seed:mode==="fixed"?42:null};
}
function saveChatOptions(){
  const mode=$("chatSampling").value;
  const context=Number($("chatContext").value);
  $("chatTempValue").textContent=Number($("chatTemperature").value).toFixed(1);
  $("chatTemperature").disabled=running||busy||mode==="greedy";
  $("chatExperimentNote").textContent=(context===256
    ?"Transformer: 256-character context (extra positions need training). Dense pathway: 64 characters. Compare using the same seed."
    :"Transformer: original 128-character context; dense pathway: 64 characters. The question can fall out during long replies.")+
    (mode==="greedy"?" Most likely: temperature does not affect results.":
     mode==="fixed"?" Seeded sampling (42) allows repeatable A/B comparisons.":
     " Random sampling can produce different replies without any new learning.");
  try{localStorage.setItem(CHAT_PREFS_KEY,JSON.stringify({
    maxChars:Number($("chatLength").value),temperature:Number($("chatTemperature").value),
    contextChars:context,mode
  }));}catch(_){}
}
function restoreChatOptions(){
  try{
    const data=JSON.parse(localStorage.getItem(CHAT_PREFS_KEY)||"null");
    if(data){
      if([32,64,128,192,256].includes(data.maxChars))$("chatLength").value=String(data.maxChars);
      if([128,256].includes(data.contextChars))$("chatContext").value=String(data.contextChars);
      if(typeof data.temperature==="number" && data.temperature>=0.2&&data.temperature<=2)
        $("chatTemperature").value=String(data.temperature);
      if(["random","fixed","greedy"].includes(data.mode))$("chatSampling").value=data.mode;
    }
  }catch(_){}
  saveChatOptions();
}
for(const id of ["chatLength","chatTemperature","chatContext","chatSampling"]){
  $(id).addEventListener(id==="chatTemperature"?"input":"change",saveChatOptions);
}
function explainContext(r){
  if(!r)return "No prompt retention information.";
  const kept=r.promptTokensRetained??0;
  const total=r.promptTokens??0;
  const note=kept===total?"All original prompt tokens retained.":
    kept===0?"Original prompt tokens have left the attention window.":
    "Original prompt partially truncated by attention window.";
  return "Transformer "+r.window+" tokens · "+r.activeTokens+" active · "+
    kept+"/"+total+" prompt tokens retained · "+r.memoryTurns+
    "/12 previous turns encoded as neural memory · "+r.generatedBytes+
    " characters generated · "+(r.termination==="learned-end"?"learned reply end":"length limit")+
    " · "+note;
}
function comparePanel(a,b){
  const root=$("chatComparison");
  root.hidden=false;
  root.replaceChildren();
  const heading=document.createElement("strong");
  heading.textContent="Same question · same weights · same sampling sequence";
  const grid=document.createElement("div");
  grid.className="comparison-grid";
  for(const [window,reply] of [[128,a],[256,b]]){
    const card=document.createElement("article");card.className="comparison-card";
    const label=document.createElement("strong");
    label.textContent=window+"-character window";
    const output=document.createElement("pre");
    output.textContent=reply.response||"(Predicted end of reply)";
    const detail=document.createElement("small");
    detail.textContent=explainContext(reply.contextReport);
    card.append(label,output,detail);grid.append(card);
  }
  const first=a.response,second=b.response;
  let matching=0;
  while(matching<first.length && matching<second.length&&first[matching]===second[matching])matching++;
  const summary=document.createElement("p");
  summary.className="muted";
  summary.textContent="Identical first "+matching+" characters. "+
    (first===second?"Both responses match exactly for this test. ":
     "Responses diverge at generated character "+(matching+1)+". ")+
    "Both trials use identical prior neural memory; newly learned token units and untrained 256-position embeddings still affect comparisons.";
  root.append(heading,grid,summary);
}
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
    const reply=await invoke("hybrid_chat",{message,...chatOptions()});
    bubble("nova",reply.response);
    lastReply=reply.response;
    $("correctReply").disabled=false;
    status(reply.status);
    showVisual(reply.trace);
    $("inspectMessage").value=message;
    $("chatContextReport").textContent=explainContext(reply.contextReport);
    report("INFO","Context",explainContext(reply.contextReport));
  }finally{busy=false;setRunning(running);}
},"Chat"));
$("compareContext").addEventListener("click",()=>safe(async()=>{
  const message=$("chatMessage").value.trim()||lastQuestion;
  if(!/^[\x20-\x7e]{1,180}$/.test(message))throw Error("Enter a printable ASCII message to compare.");
  if(running||busy)return;
  busy=true;setRunning(running);
  $("chatComparison").hidden=false;
  $("chatComparison").textContent="Comparing both context windows with matching sampling settings…";
  try{
    const settings=chatOptions();
    // Both calls use an identical pseudorandom seed; greedy ignores temperature.
    // Do not train or alter Nova's weights during the comparison.
    const base={...settings,seed:42};
    const old=await invoke("hybrid_chat",{message,...base,contextChars:128,commit:false});
    const extended=await invoke("hybrid_chat",{message,...base,contextChars:256,commit:false});
    comparePanel(old,extended);
    report("INFO","A/B context","Compared 128 and 256 character windows with fixed seed 42.");
  }finally{busy=false;setRunning(running);}
},"Context comparison"));
$("chatMessage").addEventListener("keydown",e=>{
  if(e.key==="Enter"){e.preventDefault();$("sendMessage").click();}
});
$("clearChat").addEventListener("click",()=>safe(async()=>{
  if(busy||running)return;
  if(!confirm("Start a new conversation? This clears the previous chat and its neural memory slots, but does not reset learned weights."))return;
  await invoke("hybrid_new_chat");
  transcript.replaceChildren();
  const placeholder=document.createElement("div");placeholder.className="chat-placeholder";
  placeholder.textContent="New conversation. Memory slots are empty; Nova's learned weights remain unchanged.";
  transcript.append(placeholder);
  lastQuestion="";lastReply="";$("correctReply").disabled=true;
  $("chatComparison").hidden=true;
  $("chatContextReport").textContent="New session: 0 / 12 neural memory slots.";
  await poll();
},"New conversation"));
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
    "Measured "+attention.length+" Transformer attention token positions and sampled activations from "+
    layers.reduce((sum,l)=>sum+l.length,0).toLocaleString()+
    " dense neurons. The center fibers show learned bridge signal magnitudes (32 channels). These are neural signals, not a literal brain scan.";
}
$("inspectBrain").addEventListener("click",()=>safe(async()=>{
  const message=$("inspectMessage").value.trim();
  if(!message)throw Error("Enter a message to inspect.");
  showVisual(await invoke("hybrid_inspect",{message,contextChars:Number($("chatContext").value)}));
},"Brain inspection"));
$("inspectMessage").addEventListener("keydown",e=>{if(e.key==="Enter"){e.preventDefault();$("inspectBrain").click();}});
$("copyDiagnostics").addEventListener("click",()=>safe(async()=>{
  const text="Neural Nova hybrid v0.7.1\n"+JSON.stringify(latestStatus)+"\n"+
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
      $("trainingStage").textContent=p.stage+" · learning rate "+p.learningRate.toFixed(5);
      $("trainingSpeed").textContent="Throughput: "+p.tokensPerSecond.toFixed(1)+" tokens/s";
    });
    await events.listen("hybrid-epoch",({payload:p})=>{
      history("Epoch "+p.epoch+"/"+p.epochs+" | train "+p.trainLoss.toFixed(4)+
        " | held-out "+p.validationLoss.toFixed(4)+" | accuracy "+(100*p.validationAccuracy).toFixed(1)+
        "% | "+(p.improved?"NEW BEST":"no improvement")+" | LR "+p.learningRate.toFixed(5)+
        (Number.isFinite(p.retentionAccuracy)?" | teacher skill "+(100*p.retentionAccuracy).toFixed(1)+"%":"")+
        (Number.isFinite(p.previousHeldoutLoss)?" | prior heldout "+
            p.previousHeldoutLoss.toFixed(4)+" (baseline "+p.previousBaselineLoss.toFixed(4)+")":"")+
        (p.guardAccepted===false?" | REJECTED for retention":"")+
        (p.rehearsalBatches?" | old batches "+p.rehearsalBatches:""));
      if(Number.isFinite(p.transformerGradientNorm)){
        history("Gradient norms (before clipping): Transformer "+
          p.transformerGradientNorm.toFixed(4)+" · Dense "+
          p.denseGradientNorm.toFixed(4)+" · Bridge "+
          p.bridgeGradientNorm.toFixed(4)+" · clipped batches Transformer "+
          (p.transformerClipFraction*100).toFixed(1)+"% / Dense "+
          (p.denseClipFraction*100).toFixed(1)+"%");
      }
      $("heldoutLoss").textContent=p.validationLoss.toFixed(4);
      $("heldoutAccuracy").textContent=(100*p.validationAccuracy).toFixed(1)+"%";
      $("bestEpoch").textContent="Best epoch: "+(p.bestEpoch||"baseline")+" · loss "+p.bestLoss.toFixed(4);
      $("trainingSpeed").textContent="Throughput: "+p.tokensPerSecond.toFixed(1)+" tokens/s";
    });
    await events.listen("hybrid-done",({payload:p})=>{
      setRunning(false);
      void updateRehearsalBank().catch(e=>report("WARN","Rehearsal archive",e));
      report("INFO","Training",p.cancelled?"Training stopped; best joint checkpoint restored.":
        p.earlyStopped?"Validation stalled; early stopped at best epoch "+p.bestEpoch+".":
        "Shared training finished; best validation checkpoint restored (epoch "+p.bestEpoch+").");
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
  loadSchedule();
  restoreChatOptions();
  modeChanged("dialogue");setRunning(false);
  if(!window.__TAURI__?.core?.invoke){setState("Installed app required","error");report("ERROR","Startup","Run the installed Windows or Android app for Rust neural learning.");return;}
  await listen();
  try{
    status(await invoke("hybrid_boot"));
    const turns=await invoke("hybrid_conversation");
    for(const turn of turns){bubble("user",turn.user);bubble("nova",turn.assistant);}
    report("INFO","Brain","Unified hybrid brain, trainable word units and persisted multi-turn neural memory loaded. "+turns.length+" stored turns.");
    document.dispatchEvent(new Event("neural-nova-booted"));
    void updateRehearsalBank().catch(e=>report("WARN","Rehearsal archive",e));
  }catch(e){setState("Startup error","error");report("ERROR","Startup",e);}
  setInterval(()=>{if(running)void poll();},1000);
}
void boot();
})();