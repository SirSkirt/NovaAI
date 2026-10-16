"use strict";
/* Chat-first Nova workspace. Rearranges existing controls without cloning the
   teacher or checkpoint DOM, so the native commands remain the only authority. */
(()=>{
  const $=id=>document.getElementById(id);
  const main=document.querySelector(".app-shell > main");
  const nav=document.querySelector(".app-shell > .nav-tabs");
  const chat=$("pane-chat"),teach=$("pane-teach"),targeted=$("pane-targeted"),visualize=$("pane-visualize");
  if(!main||!nav||!chat||!teach||!targeted||!visualize)return;
  nav.innerHTML='<button type="button" id="tab-chat" class="nav-tab selected" role="tab" aria-controls="pane-chat" aria-selected="true" data-tab="chat">Chat</button>'+
    '<button type="button" id="tab-train" class="nav-tab" role="tab" aria-controls="pane-train" aria-selected="false" data-tab="train" tabindex="-1">Train</button>';
  const train=document.createElement("section");
  train.id="pane-train";train.className="tab-pane train-workspace";train.setAttribute("role","tabpanel");
  train.setAttribute("aria-labelledby","tab-train");train.hidden=true;
  train.innerHTML='<div class="pane-title"><div><p class="eyebrow">TEACHER WORKSPACE · TEMPORARY</p><h2>Train Nova</h2></div>'+
    '<span class="subtle">Retained until Yendory Studio is ready</span></div>'+
    '<div class="train-subtabs" role="tablist" aria-label="Training tools">'+
    '<button type="button" role="tab" id="subtab-teach" data-train="teach" aria-controls="pane-teach" aria-selected="true" class="train-subtab selected">Lessons &amp; Foundation</button>'+
    '<button type="button" role="tab" id="subtab-targeted" data-train="targeted" aria-controls="pane-targeted" aria-selected="false" class="train-subtab">Targeted training</button>'+
    '<button type="button" role="tab" id="subtab-activity" data-train="activity" aria-controls="pane-activity" aria-selected="false" class="train-subtab">Training history</button></div>';
  const activity=document.createElement("section");
  activity.id="pane-activity";activity.className="train-pane";activity.setAttribute("role","tabpanel");
  activity.setAttribute("aria-labelledby","subtab-activity");activity.hidden=true;
  for(const [pane,id] of [[teach,"teach"],[targeted,"targeted"]]){
    pane.classList.remove("tab-pane");pane.classList.add("train-pane");pane.hidden=id!=="teach";
    pane.setAttribute("aria-labelledby","subtab-"+id);train.append(pane);
  }
  const records=visualize.querySelector('section[aria-label="Saved epoch observations"]');
  if(records)activity.append(records);
  train.append(activity);
  main.prepend(chat);main.append(train);chat.hidden=false;
  const title=chat.querySelector(".pane-title");
  title.innerHTML='<div><p class="eyebrow">ONE BICAMERAL MIND · LOCAL</p><h2>Conversation</h2></div>'+
    '<div class="messenger-actions"><button type="button" class="secondary compact" id="openModelTools" title="Open checkpoint import and export">Checkpoints</button>'+
    '<button type="button" class="secondary compact brain-toggle" id="toggleBrain" aria-controls="novaBrainRail" aria-expanded="false">Brain inspector</button></div>';
  const grid=document.createElement("div");grid.className="messenger-grid";
  const conversation=document.createElement("div");conversation.className="messenger-conversation";
  const card=chat.querySelector(".chat-card");
  if(!card)throw Error("Nova chat card was not found");
  conversation.append(card);
  // Mount conversation immediately: an unrelated inspector failure must never blank chat.
  grid.append(conversation);chat.append(grid);
  const intro=document.createElement("div");intro.className="conversation-intro";
  intro.innerHTML='<span class="mini-mark" aria-hidden="true">N</span><div><strong>Nova</strong><small>Direct conversation with the active .nnova2 brain</small></div>';
  card.prepend(intro);
  const notice=document.createElement("div");notice.id="modelNotice";
  notice.className="model-notice";notice.hidden=true;
  notice.textContent="No trained model is loaded. Open Checkpoints to import your .nnova2 file.";
  intro.after(notice);
  const transcript=$("chatTranscript");
  const tree=document.createElement("details");tree.id="novaThinkingTree";tree.className="thinking-tree";
  tree.innerHTML='<summary><span>Thinking tree <small>preview · inference evidence</small></span><span class="disclosure-mark" aria-hidden="true">⌄</span></summary>'+
    '<div class="thinking-intro">Nova-2 does not expose a step-by-step reasoning tree. Only measured prompt, generation and checkpoint metadata appear here; no thoughts are invented.</div>'+
    '<ol id="novaTraceNodes" class="trace-nodes"><li>No completed inference in this session yet.</li></ol>';
  transcript.after(tree);
  const settings=card.querySelector(".chat-settings");
  const advanced=document.createElement("details");advanced.className="generation-settings";
  advanced.innerHTML='<summary>Generation settings <span>temperature · context · decoding</span></summary>';
  settings.parentElement.insertBefore(advanced,settings);
  advanced.append(settings);
  const rail=document.createElement("aside");rail.id="novaBrainRail";rail.className="nova-brain-rail";
  rail.setAttribute("aria-label","Nova brain inspector");
  rail.innerHTML='<div class="rail-heading"><div><p class="eyebrow">LIVE / READ-ONLY</p><h3>Brain inspector</h3></div>'+
    '<button type="button" id="closeBrain" class="secondary compact rail-close">Close</button></div>'+
    '<label class="auto-inspect"><input type="checkbox" id="autoInspectBrain"> Inspect after each reply (may be slow)</label>'+
    '<p class="rail-caveat">The diagram is unchanged; visual overlays use real token sensitivity and sampled weights, not decoded thoughts.</p>';
  const diagram=visualize.querySelector(".brain-card");
  const controls=visualize.querySelector(".telemetry-card");
  if(diagram){
    const detail=diagram.querySelector("#visualizerDetail");
    if(detail)detail.textContent="Left: output sensitivity when a token is removed. Right: sampled learned Dense weights. Green: measurable interconnect influence. Dot positions are schematic, not neuron coordinates.";
    rail.append(diagram);
  }
  if(controls){
    const memory=controls.querySelector("#brainMemoryInfo");
    if(memory)memory.textContent="Choose Inspect to measure how the active checkpoint responds to a prompt.";
    const heading=controls.querySelector("h3");
    if(heading)heading.textContent="Inspect a prompt";
    rail.append(controls);
  }
  visualize.remove();
  grid.append(rail);
  const overlay=document.createElement("button");overlay.type="button";overlay.className="brain-backdrop";
  overlay.id="brainBackdrop";overlay.setAttribute("aria-label","Close brain inspector");overlay.hidden=true;
  chat.append(overlay);
  const showBrain=open=>{
    const active=!!open;rail.classList.toggle("rail-open",active);
    document.body.classList.toggle("brain-open",active);
    $("toggleBrain").setAttribute("aria-expanded",String(active));
    overlay.hidden=!active;
    if(active&&window.matchMedia("(max-width: 1120px)").matches)rail.scrollTop=0;
  };
  $("toggleBrain").addEventListener("click",()=>showBrain(!rail.classList.contains("rail-open")));
  $("closeBrain").addEventListener("click",()=>showBrain(false));
  overlay.addEventListener("click",()=>showBrain(false));
  document.addEventListener("keydown",event=>{if(event.key==="Escape")showBrain(false);});
  const showTrainSubtab=name=>{
    const selected=["teach","targeted","activity"].includes(name)?name:"teach";
    for(const b of train.querySelectorAll(".train-subtab")){
      const active=b.dataset.train===selected;b.classList.toggle("selected",active);
      b.setAttribute("aria-selected",String(active));b.tabIndex=active?0:-1;
    }
    for(const pane of train.querySelectorAll(".train-pane"))pane.hidden=pane.id!=="pane-"+selected;
  };
  for(const b of train.querySelectorAll(".train-subtab")){
    b.addEventListener("click",()=>showTrainSubtab(b.dataset.train));
    b.addEventListener("keydown",event=>{
      if(!["ArrowLeft","ArrowRight","Home","End"].includes(event.key))return;
      event.preventDefault();const tabs=[...train.querySelectorAll(".train-subtab")],i=tabs.indexOf(b);
      const k=event.key==="Home"?0:event.key==="End"?tabs.length-1:
        (i+(event.key==="ArrowRight"?1:tabs.length-1))%tabs.length;
      tabs[k].focus();showTrainSubtab(tabs[k].dataset.train);
    });
  }
  $("openModelTools").addEventListener("click",()=>{
    $("tab-train").click();showTrainSubtab("teach");
    const transfer=$("checkpointTransfer");
    if(transfer){transfer.open=true;transfer.scrollIntoView?.({behavior:"smooth",block:"center"});}
  });
  function traceReply(info){
    const nodes=$("novaTraceNodes");nodes.replaceChildren();
    const items=[
      ["Prompt",info.message?"Message submitted to the local Bicameral model":"Completed inference"],
      ["Checkpoint",Number.isFinite(info.step)?"Optimizer step "+Number(info.step).toLocaleString():"Step unavailable"],
      ["Generated output",Number.isFinite(info.generatedTokens)?info.generatedTokens+" generated tokens":"Generation count unavailable"],
      ["KV-cache",Number.isFinite(info.cacheAttentionKeys)?Number(info.cacheAttentionKeys).toLocaleString()+" attention-key reads":"Not measured"],
      ["Reasoning trace","Not exposed by the current Nova-2 architecture"]
    ];
    for(const [heading,content] of items){
      const li=document.createElement("li"),label=document.createElement("strong"),value=document.createElement("span");
      label.textContent=heading;value.textContent=content;li.append(label,value);nodes.append(li);
    }
  }
  const ns="http://www.w3.org/2000/svg";
  const dot=(group,x,y,size,fill,caption,opacity)=>{
    const node=document.createElementNS(ns,"circle");node.setAttribute("cx",x);node.setAttribute("cy",y);
    node.setAttribute("r",size);node.setAttribute("fill",fill);node.setAttribute("opacity",opacity);
    const tooltip=document.createElementNS(ns,"title");tooltip.textContent=caption;node.append(tooltip);group.append(node);
  };
  const text=(target,value)=>{const el=$(target);if(el)el.textContent=value;};
  let requestNumber=0;
  async function inspect(message){
    const clean=(message||$("inspectMessage").value||"").trim();
    if(!clean)return;
    $("inspectMessage").value=clean;
    const index=++requestNumber,button=$("inspectBrain");
    button.disabled=true;text("visualizerDetail","Measuring current checkpoint response; this may take a moment...");
    try{
      const api=window.__TAURI__?.core?.invoke;
      if(!api)throw Error("Native Nova runtime unavailable");
      const trace=await api("nova2_inspect",{message:clean,contextChars:Number($("chatContext").value)});
      if(index!==requestNumber)return;
      const sensitivity=trace.attention||[];
      text("attentionCount",String(sensitivity.length));
      text("neuronCount","Sampled");
      const left=$("leftActivity"),right=$("rightActivity");
      left.replaceChildren();right.replaceChildren();
      const max=Math.max(1e-8,...sensitivity.map(t=>Number(t.weight)||0));
      sensitivity.forEach((t,i)=>{
        const col=i%5,row=Math.floor(i/5);
        dot(left,155+col*44,108+row*49,4+5*Math.min(1,(Number(t.weight)||0)/max),
          "#78b9dc",t.character+" · removal effect "+Number(t.weight).toPrecision(3),0.35+0.6*Math.min(1,(Number(t.weight)||0)/max));
      });
      const weights=(trace.denseLayers||[]).flat();
      const largest=Math.max(1e-7,...weights.map(x=>Math.abs(Number(x)||0)));
      weights.slice(0,35).forEach((v,i)=>{
        const col=i%7,row=Math.floor(i/7),intensity=Math.min(1,Math.abs(Number(v)||0)/largest);
        dot(right,468+col*27,104+row*48,3+4*intensity,"#baa5d5",
          "Sampled Dense weight "+i+" = "+Number(v).toPrecision(3),0.25+0.7*intensity);
      });
      $("bridgeFibers").setAttribute("opacity",trace.interconnectUsed?"0.67":"0.16");
      const tiles=$("attentionTiles");tiles.replaceChildren();
      for(const item of sensitivity){
        const chip=document.createElement("span");chip.className="attention-tile";
        chip.textContent=item.character;chip.title="Output change when removed: "+Number(item.weight).toPrecision(4);
        tiles.append(chip);
      }
      if(!sensitivity.length)tiles.textContent="No token-sensitivity samples returned.";
      const predictions=$("predictions");predictions.replaceChildren();
      for(const item of (trace.topPredictions||[])){
        const chip=document.createElement("span");chip.className="prediction-token";
        const code=Number(item.code),label=code===10?"↵":code===32?"SPACE":code>=32&&code<127?String.fromCharCode(code):"#"+code;
        chip.textContent=label+" · "+(Number(item.probability)*100).toFixed(2)+"%";predictions.append(chip);
      }
      text("visualizerDetail","Observed: token-removal sensitivity, sampled Dense weights and interconnect ablation. Positions are schematic; no internal thoughts are decoded.");
      text("brainMemoryInfo",trace.method||"Read-only checkpoint measurements.");
    }catch(error){
      if(index===requestNumber)text("visualizerDetail","Inspector error: "+String(error));
    }finally{if(index===requestNumber)button.disabled=false;}
  }
  $("inspectBrain").addEventListener("click",()=>{void inspect();});
  document.addEventListener("nova-reply",event=>{
    const detail=event.detail||{};traceReply(detail);
    if($("autoInspectBrain").checked)void inspect(detail.message);
  });
  window.NovaMessenger={showTrainSubtab,traceReply,inspect};
  showTrainSubtab("teach");
})();
