"use strict";
(()=>{
  const $=id=>document.getElementById(id);
  const invoke=(name,args={})=>window.__TAURI__?.core?.invoke(name,args);
  let records=[],sessionId="",busy=false;
  const fmt=(v)=>Number.isFinite(v)?Number(v).toFixed(5):"—";
  function paint(){
    const record=records.find(x=>String(x.epoch)===$("observerEpoch").value);
    if(!record)return;
    const p=record.pathways||{},metrics=record.metrics||{};
    $("observerDenseEffect").textContent=fmt(p.fullVsNoDenseTotalVariation);
    $("observerExchangeEffect").textContent=fmt(p.fullVsNoReciprocalExchangeTotalVariation);
    $("observerBridgeRms").textContent=fmt(p.bridgeRms);
    $("observerStep").textContent=String(record.epoch)+" / "+String(record.step);
    $("observerMetrics").textContent="Epoch "+record.epoch+" · "+record.source+
      " / "+record.objective+" · "+(metrics.selected?"checkpoint selected":"checkpoint not selected")+
      " · "+JSON.stringify(metrics);
    $("observerReplies").textContent=(record.replies||[]).map(r=>
      "["+r.split+" · "+r.category+(r.promptSeenInTraining?" · repeated training prompt":"")+(r.independentPrompt?" · novel prompt":"")+"]\n"+
      "Prompt: "+r.prompt+"\n"+
      "Expected (teacher scoring only): "+r.expected+(r.expectedTruncated?"…":"")+"\n"+
      "Nova generated: "+JSON.stringify(r.generated)+
      (r.endedWithNewline?" [ended]":" [limit reached]")+"\n"+
      "First character: "+(r.matchedFirstCharacter?"match":"mismatch")+
      " · correct prefix: "+r.correctPrefixCharacters+
      " · complete: "+(r.completeCorrect?"yes":"no")+
      " · conversation memory turns: "+r.memoryTurns
    ).join("\n\n")||"This lesson has no supervised full-answer samples.";
    const ablations=record.ablationComparisons||[];
    $("observerAblations").textContent=ablations.map(c=>
      "["+c.category+(c.independentPrompt?" · novel prompt":" · overlaps training")+"]\n"+
      "Prompt: "+c.prompt+"\nExpected: "+c.expected+
      (c.truncatedAnswer?"… (truncated)":"")+"\n"+
      c.variants.map(v=>v.mode+": "+JSON.stringify(v.generated)+
        (v.ended?" [ended]":" [limit]")+" · prefix "+v.correctPrefixCharacters+
        " · expected-answer mean NLL "+fmt(v.teacherForcedMeanNll)).join("\n")+"\n"+
      "Full vs no-dense NLL benefit: "+fmt(c.benefitOfFullVsNoDenseNll)+
      " · full vs no-exchange NLL benefit: "+fmt(c.benefitOfFullVsNoExchangeNll)+
      " (positive = full is better)"
    ).join("\n\n")||"No three-way results recorded (older sessions cannot be backfilled).";
    const integrity=record.heldoutIntegrity;
    $("observerIntegrity").textContent=integrity?
      "Held-out audit: "+integrity.independentPromptCount+"/"+integrity.totalReplies+
        " distinct questions from training, "+integrity.samePromptAsTraining+
        " repeats. "+integrity.warning:
      "Older session: no held-out prompt-overlap audit available.";
    $("observerClipping").textContent=Number.isFinite(metrics.transformerMeanClipScale)?
      "Global gradient threshold 1.0 · average scaling: Transformer "+
        fmt(metrics.transformerMeanClipScale)+", Dense "+fmt(metrics.denseMeanClipScale)+
        " · clipped supervised batch fractions: Transformer "+
        fmt(metrics.transformerClipFraction)+", Dense "+fmt(metrics.denseClipFraction)+
        ". Adam weight updates do not scale linearly with this factor.":
      "Clipping-scale diagnostics not recorded in this epoch or training mode.";
    $("observerActivity").textContent=JSON.stringify({
      weightsFingerprint:record.weightFingerprint,
      pathwaySignals:p,
      denseNeuronCounts:(record.brainSnapshot?.denseLayers||[]).map(x=>x.length),
      lastAttentionTokens:(record.brainSnapshot?.attention||[]).slice(-24),
      limitations:record.limitations
    },null,2);
    if(record.brainSnapshot && window.NovaVisualObserver?.replay){
      window.NovaVisualObserver.replay(record.brainSnapshot);
    }
    $("observerStatus").textContent="Saved epoch "+record.epoch+" of "+(records.length-1)+
      ". Green connection = measured output difference, not decoded thoughts.";
  }
  async function loadSession(id,newest){
    sessionId=id;
    const data=await invoke("nova_observer_read",{sessionId:id});
    records=Array.isArray(data)?data.filter(x=>x.type==="epoch"):[];
    const menu=$("observerEpoch"),old=menu.value;
    menu.replaceChildren();
    for(const item of records){
      const opt=document.createElement("option");
      opt.value=String(item.epoch);
      opt.textContent="Epoch "+item.epoch+(item.epoch===0?" (baseline)":"");
      menu.append(opt);
    }
    if(records.length){
      menu.value=!newest&&records.some(x=>String(x.epoch)===old)?
        old:String(records[records.length-1].epoch);
      paint();
    }else{
      $("observerStatus").textContent="No completed epochs saved in this session.";
    }
  }
  async function refresh(latest=false){
    if(busy||!window.__TAURI__?.core?.invoke)return;
    busy=true;
    try{
      const sessions=await invoke("nova_observer_sessions");
      const menu=$("observerSession"),previous=latest?"":menu.value;
      menu.replaceChildren();
      for(const id of sessions){
        const option=document.createElement("option");
        option.value=id;option.textContent=id;menu.append(option);
      }
      if(!sessions.length){
        $("observerStatus").textContent="No recorded epochs yet. Start unified or targeted training.";
        return;
      }
      const chosen=sessions.includes(previous)?previous:sessions[0];
      menu.value=chosen;
      await loadSession(chosen,latest||sessionId!==chosen);
    }catch(error){
      $("observerStatus").textContent="Activity read error: "+String(error);
    }finally{busy=false;}
  }
  $("observerSession").addEventListener("change",()=>loadSession($("observerSession").value,true)
    .catch(e=>$("observerStatus").textContent="Activity read error: "+String(e)));
  $("observerEpoch").addEventListener("change",paint);
  $("observerRefresh").addEventListener("click",()=>refresh(false));
  document.addEventListener("neural-nova-booted",()=>{void refresh(true);});
  if(window.__TAURI__?.event?.listen){
    void window.__TAURI__.event.listen("nova-observer-epoch",({payload})=>{
      if(payload.error)$("observerStatus").textContent="Activity save error (training continued): "+payload.error;
      else void refresh(true);
    }).catch(e=>$("observerStatus").textContent="Activity listener error: "+String(e));
  }
})();
