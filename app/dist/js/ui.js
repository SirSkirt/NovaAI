"use strict";

/* =========================================================
   Nova V7.0.0-alpha.9.3.1 — CURRENT UI

   Presentation only. This file does not replace, wrap, or extend reasoning.
   Native Thinking, Teacher Tools, and Brain Inspector render projections of the
   Rust-authoritative NovaMind; JavaScript does not maintain a shadow brain.
   ========================================================= */

function nova620UiClone(value){
  try{return structuredClone(value);}catch(_){try{return JSON.parse(JSON.stringify(value));}catch(__){return value;}}
}
function nova620UiJson(value){
  try{return JSON.stringify(value,null,2);}catch(_){return String(value??"");}
}
function nova620UiCount(value){return Array.isArray(value)?value.length:Number(value||0);}
function nova620UiCard(title,detail="",extra=""){
  return `<div class="brain-card"><div class="brain-card-title"><strong>${escapeHtml(String(title??""))}</strong></div>${detail?`<div class="brain-card-detail">${escapeHtml(String(detail))}</div>`:""}${extra||""}</div>`;
}
function nova620UiPre(value){return `<pre style="white-space:pre-wrap;margin:0;font-size:.75rem;overflow-wrap:anywhere">${escapeHtml(nova620UiJson(value))}</pre>`;}
function nova620UiEmpty(text){return `<div class="empty">${escapeHtml(text)}</div>`;}
function nova620UiRuleTitle(rule){return rule?.directive||rule?.name||rule?.id||"unnamed cognition";}
function nova620UiSourceLabel(item){
  if(item?.sourceDisplayName)return String(item.sourceDisplayName);
  try{if(typeof v534SourceDisplayName==="function")return v534SourceDisplayName(item?.sourceName||"conversation",item?.sourceType||"learned");}catch(_){ }
  const name=String(item?.sourceName||"conversation");if(/^conversation$/i.test(name))return "Conversation";let m=name.replace(/[_-]+/g," ").match(/\btest\s+lesson\s*(\d+)\b/i);if(m)return `Test Lesson ${Number(m[1])}`;m=name.replace(/[_-]+/g," ").match(/\blesson\s*(\d+)\b/i);if(m)return `Lesson ${Number(m[1])}`;return "Document";
}
function nova620UiRuleDetail(rule){return `${rule?.status||"Stored Only"} • ${rule?.domain||"general"}${rule?.cognitiveForm?` • ${rule.cognitiveForm}`:""} • ${nova620UiSourceLabel(rule)}`;}
function nova620UiRuleCard(rule){return nova620UiCard(nova620UiRuleTitle(rule),nova620UiRuleDetail(rule));}
function nova620UiModuleCard(module){return nova620UiCard(`#${module?.moduleNumber||"?"} — ${module?.name||module?.id||"unnamed module"}`,`${(module?.categories||[]).join(", ")||"uncategorized"} • ${module?.enabled===false?"disabled":"enabled"} • ${nova620UiSourceLabel(module)}`);}
function nova620UiSourceSort(a,b){const num=x=>{const m=String(x).match(/(?:Test )?Lesson (\d+)/i);return m?Number(m[1]):Number.MAX_SAFE_INTEGER;};const an=num(a),bn=num(b);if(an!==bn)return an-bn;if(a==="Conversation")return 1;if(b==="Conversation")return -1;return a.localeCompare(b);}
function nova620UiGroupedRuleLists(title,rules,{cardRenderer=nova620UiRuleCard,emptyText="No items."}={}){
  const groups=new Map();for(const rule of rules||[]){const label=nova620UiSourceLabel(rule);if(!groups.has(label))groups.set(label,[]);groups.get(label).push(rule);}
  if(!groups.size)return `<div class="brain-list-group"><div class="brain-list-header"><div class="brain-list-title">${escapeHtml(title)}</div><div class="brain-list-meta">0 total</div></div>${nova620UiEmpty(emptyText)}</div>`;
  const total=(rules||[]).length,rows=[...groups.entries()].sort((a,b)=>nova620UiSourceSort(a[0],b[0])).map(([label,list])=>{const preview=list.slice(0,10),rest=list.slice(10);return `<details class="brain-source-group"><summary class="brain-source-summary"><strong>${escapeHtml(label)}</strong><span>${list.length} item${list.length===1?"":"s"}</span></summary><div class="brain-source-body">${preview.map(cardRenderer).join("")}${rest.length?`<details class="brain-expand"><summary class="brain-expand-summary">Show ${rest.length} more</summary><div class="brain-expand-body">${rest.map(cardRenderer).join("")}</div></details>`:""}</div></details>`;}).join("");
  return `<div class="brain-list-group"><div class="brain-list-header"><div class="brain-list-title">${escapeHtml(title)}</div><div class="brain-list-meta">${total} total • grouped by learning origin</div></div>${rows}</div>`;
}
function nova620UiDiagnosticList(values){const list=(values||[]).filter(Boolean);return list.length?`<ul class="brain-diag-list">${list.map(value=>`<li>${escapeHtml(String(value).replace(/_/g," "))}</li>`).join("")}</ul>`:`<div class="brain-diag-muted">None recorded.</div>`;}
function nova620UiStoredRuleCard(rule){const d=rule?.synthesis||{},attempts=(d.attempts||[]).map(a=>`${a.strategy}: ${a.status}${a.reason?` — ${a.reason}`:""}`),source=nova620UiSourceLabel(rule),missing=nova620UiDiagnosticList([...(d.missingCapabilities||[]),...(d.missingOperations||[])]);return `<details class="brain-card brain-rule-diagnostic"><summary class="brain-rule-summary"><span class="brain-card-title"><strong>${escapeHtml(nova620UiRuleTitle(rule))}</strong></span><span class="brain-card-detail">${escapeHtml(nova620UiRuleDetail(rule))}</span></summary><div class="brain-rule-body"><div class="brain-diag-row"><strong>Why Stored Only</strong><div>${escapeHtml(d.reason||"The latest synthesis pass did not produce a valid executable module.")}</div></div><div class="brain-diag-row"><strong>Missing capability / dependency</strong>${missing}</div><div class="brain-diag-row"><strong>Required operations considered</strong>${nova620UiDiagnosticList(d.requiredOperations)}</div><div class="brain-diag-row"><strong>Synthesis attempts</strong>${nova620UiDiagnosticList(attempts)}</div><div class="brain-diag-row"><strong>Learning origin</strong><div>${escapeHtml(source)}</div></div><div class="brain-diag-row"><strong>Last synthesis status</strong><div>${escapeHtml(d.status||"not attempted")}</div></div></div></details>`;}
function nova620UiExpandableRuleGroup(title,rules,{previewCount=10,emptyText="No items.",summaryDetail="",cardRenderer=nova620UiRuleCard}={}){
  const list=Array.isArray(rules)?rules:[];
  const preview=list.slice(0,previewCount).map(cardRenderer).join("")||nova620UiEmpty(emptyText);
  const remainder=list.slice(previewCount);
  const countLabel=`${list.length} total`;
  const remainderLabel=remainder.length===1?"1 more":`${remainder.length} more`;
  return `<div class="brain-list-group">
    <div class="brain-list-header">
      <div class="brain-list-title">${escapeHtml(title)}</div>
      <div class="brain-list-meta">${escapeHtml(countLabel)}${summaryDetail?` • ${escapeHtml(summaryDetail)}`:""}</div>
    </div>
    <div class="brain-list-preview">${preview}</div>
    ${remainder.length?`<details class="brain-expand"><summary class="brain-expand-summary">Show ${escapeHtml(remainderLabel)}</summary><div class="brain-expand-body">${remainder.map(cardRenderer).join("")}</div></details>`:""}
  </div>`;
}

/* ---------------------------------------------------------
   CHAT
   --------------------------------------------------------- */
function createMessage(role,content,options={}){
  return {
    id:(globalThis.crypto?.randomUUID?crypto.randomUUID():`${Date.now()}-${Math.random()}`),
    role,
    content:String(content??""),
    rawContent:options.rawContent!==undefined?String(options.rawContent):String(content??""),
    semantics:options.semantics||null,
    sourceLabel:options.sourceLabel||null,
    createdAt:Date.now()
  };
}
function addMessage(role,content,options={}){
  const message=createMessage(role,content,options);
  state.messages.push(message);
  if(Number.isFinite(APP_CONFIG.maxMessages)&&state.messages.length>APP_CONFIG.maxMessages)state.messages=state.messages.slice(-APP_CONFIG.maxMessages);
  saveState();renderMessage(message);scrollToBottom();return message;
}
function renderMessage(message){
  if(!elements.chat)return;
  const row=document.createElement("div");row.className=`message-row ${message.role}`;
  const bubble=document.createElement("div");bubble.className=`message ${message.role}`;
  const content=document.createElement("div");content.textContent=String(message.content??"");
  const meta=document.createElement("div");meta.className="message-meta";meta.textContent=`${message.sourceLabel?`${message.sourceLabel} • `:""}${formatTime(message.createdAt)}`;
  bubble.append(content,meta);row.appendChild(bubble);elements.chat.appendChild(row);
}
function renderAllMessages(){if(!elements.chat)return;elements.chat.innerHTML="";for(const message of state.messages||[])renderMessage(message);scrollToBottom();}
function scrollToBottom(){if(!elements.chat)return;requestAnimationFrame(()=>{elements.chat.scrollTop=elements.chat.scrollHeight;});}
function resizeInput(){if(!elements.input)return;elements.input.style.height="auto";elements.input.style.height=`${Math.min(elements.input.scrollHeight||0,160)}px`;}
function showStatus(text){if(!elements.status)return;elements.status.textContent=String(text??"");elements.status.className="status visible";}
function hideStatus(){if(!elements.status)return;elements.status.textContent="";elements.status.className="status";}

/* ---------------------------------------------------------
   CURRENT BRAIN INSPECTOR
   --------------------------------------------------------- */
function nova92Fold(title,meta,body,open=false){
  return `<details class="brain-fold"${open?" open":""}><summary><span>${escapeHtml(title)}</span><span class="brain-list-meta">${escapeHtml(meta||"")}</span></summary><div class="brain-fold-body">${body}</div></details>`;
}
function nova92MeaningTrace(trace){
  const rows=Array.isArray(trace)?trace:[];
  if(!rows.length)return nova620UiEmpty("No human-readable Rust cognitive trace is available for the current turn.");
  const labels={observe:"Notice",interpretation:"Interpret",reference:"Interpret",context:"Connect",relationship:"Connect",goal:"Goal",consideration:"Consider",cognition:"Consider",unresolved:"Unresolved",conclusion:"Conclusion"};
  return rows.map(row=>`<div class="brain-meaning-step"><strong>${escapeHtml(labels[row?.stage]||String(row?.stage||"Thought").replace(/_/g," "))}</strong>${escapeHtml(String(row?.text||""))}</div>`).join("");
}
function nova92KnowledgeById(snapshot,id){
  const knowledge=snapshot?.semanticMemory?.knowledge||{};
  for(const [kind,key] of [["claim","claims"],["guidance","guidance"],["example","examples"],["procedure","procedures"]]){
    const row=(knowledge[key]||[]).find(item=>String(item?.id||"")===String(id||""));
    if(row){
      const teacherFeedback=kind==="example"&&(row?.evidence||[]).some(evidence=>String(evidence?.sourceType||"")==="teacher_feedback");
      return {kind:teacherFeedback?"teacher feedback":kind,row};
    }
  }
  return null;
}
function nova92KnowledgeUsed(snapshot){
  const state=snapshot?.cognitiveState||{};
  const ids=[];
  for(const row of [...(state.propositions||[]),...(state.conclusions||[])])for(const id of row?.evidence||[]){const value=String((typeof id==="object"&&id?(id.id??id.value??""):id)||"");if(value&&!ids.includes(value))ids.push(value);}
  if(!ids.length)return nova620UiEmpty("No specific stored evidence IDs were attached to the current conclusion.");
  return ids.slice(0,24).map(id=>{const found=nova92KnowledgeById(snapshot,id);if(!found)return nova620UiCard(id,"Referenced evidence ID");const row=found.row;const text=row.content||row.directive||row.name||`${row.subject||""} ${row.predicate||""} ${row.object||""}`.trim();return nova620UiCard(`${found.kind}: ${id}`,text||"Stored evidence");}).join("");
}
function nova92AnnotatedHtml(text,annotations,target){
  text=String(text??"");const rows=(annotations||[]).filter(row=>row?.target===target&&Number.isInteger(row?.start)&&Number.isInteger(row?.end)&&row.start<row.end);
  if(!rows.length)return escapeHtml(text);
  const points=new Set([0,text.length]);for(const row of rows){points.add(Math.max(0,Math.min(text.length,row.start)));points.add(Math.max(0,Math.min(text.length,row.end)));}
  const sorted=[...points].sort((a,b)=>a-b);let html="";
  for(let i=0;i<sorted.length-1;i++){
    const a=sorted[i],b=sorted[i+1];if(b<=a)continue;const segment=text.slice(a,b);const styles=rows.filter(row=>row.start<=a&&row.end>=b).map(row=>row.style);const classes=[...new Set(styles)].map(style=>`teacher-mark-${style}`).join(" ");html+=classes?`<span class="${classes}">${escapeHtml(segment)}</span>`:escapeHtml(segment);
  }
  return html;
}
let nova92TeacherSelection=null;
function nova92TeacherAttemptCard(row,index){
  const answerText=String(row?.originalAnswer||"");
  const credit=row?.teacherCreditPercent;
  const awarded=row?.teacherPointsAwarded,possible=row?.teacherPointsPossible,grade=String(row?.teacherGrade||"").trim();
  const effectivePercent=credit!==null&&credit!==undefined?Number(credit):(Number.isFinite(Number(awarded))&&Number.isFinite(Number(possible))&&Number(possible)>0?Number(awarded)/Number(possible)*100:null);
  const effective=effectivePercent!==null?`${effectivePercent.toFixed(1)}% teacher credit`:(row?.automaticStatus?`auto: ${String(row.automaticStatus).toUpperCase()}`:"ungraded");
  const scoreLabel=awarded!==null&&awarded!==undefined&&possible!==null&&possible!==undefined?`${Number(awarded)} / ${Number(possible)}`:null;
  const annotations=Array.isArray(row?.annotations)?row.annotations:[];
  const marks=annotations.length?annotations.map(mark=>{
    const source=mark?.target==="question"?String(row?.question||""):answerText;
    const excerpt=source.slice(Number(mark?.start||0),Number(mark?.end||0));
    const note=String(mark?.note||"").trim();
    return `<span class="teacher-mark-chip"><strong>${escapeHtml(String(mark.target))} • ${escapeHtml(String(mark.style))}</strong> “${escapeHtml(excerpt)}”${note?` — ${escapeHtml(note)}`:""} <button type="button" class="teacher-mark-note" data-teacher-attempt="${escapeHtml(String(row.id))}" data-annotation-id="${escapeHtml(String(mark.id))}" title="Add or edit the note attached to this exact marked span">note</button><button type="button" class="teacher-mark-remove" data-teacher-attempt="${escapeHtml(String(row.id))}" data-annotation-id="${escapeHtml(String(mark.id))}" title="Remove this mark">×</button></span>`;
  }).join(""):"<span class=\"teacher-feedback-status\">No exact spans marked yet.</span>";
  return `<div class="brain-card teacher-feedback-card" data-teacher-card="${escapeHtml(String(row?.id||""))}">
    <div class="brain-card-title"><strong>${index+1}. ${escapeHtml(String(row?.sourceDisplayName||"Test"))}</strong><span class="brain-list-meta">Attempt ${Number(row?.attemptNumber||1)}</span></div>
    ${row?.lessonContext?`<small>Lesson context: ${escapeHtml(String(row.lessonContext))}</small>`:""}
    <div class="curriculum-grade-line"><strong>${escapeHtml(effective)}</strong>${scoreLabel?`<span> • Score ${escapeHtml(scoreLabel)}</span>`:""}${grade?`<span> • Grade ${escapeHtml(grade)}</span>`:""}<span> • ${escapeHtml(String(row?.automaticReason||"No automatic grading explanation."))}</span></div>
    <small>${row?.evidenceReleased?"Available to Nova as non-executable post-test evidence.":"Recorded durably; withheld from Nova's semantic retrieval until this test run finishes."}</small>
    <div><div class="teacher-preserved-label">Preserved question</div><div class="teacher-annotatable" data-teacher-attempt="${escapeHtml(String(row.id))}" data-teacher-target="question">${nova92AnnotatedHtml(row?.question,annotations,"question")}</div></div>
    <div><div class="teacher-preserved-label">Nova's preserved original answer</div>${answerText?`<div class="teacher-annotatable" data-teacher-attempt="${escapeHtml(String(row.id))}" data-teacher-target="answer">${nova92AnnotatedHtml(answerText,annotations,"answer")}</div>`:nova620UiEmpty("No spoken answer was produced for this attempt.")}</div>
    ${(row?.expectedAnswer||row?.expectedBehavior)?`<small>${escapeHtml(row.expectedAnswer?`Expected: ${row.expectedAnswer}`:`Expected behavior: ${row.expectedBehavior}`)}</small>`:""}
    ${(row?.cognitiveTrace||[]).length?`<details><summary>Preserved attempt reasoning</summary>${nova92MeaningTrace(row.cognitiveTrace)}<details><summary>Compact attempt context</summary>${nova620UiPre(row.cognitiveContext||{})}</details><details><summary>Preserved Inspector context</summary>${nova620UiPre(row.inspectorContext||{})}</details></details>`:""}
    <div class="teacher-feedback-controls">
      <div class="teacher-credit-row">
        <label>Score <input class="teacher-credit-input" type="number" min="0" step="0.25" data-teacher-points-awarded="${escapeHtml(String(row.id))}" value="${awarded===null||awarded===undefined?"":escapeHtml(String(awarded))}"> / <input class="teacher-credit-input" type="number" min="0.01" step="0.25" data-teacher-points-possible="${escapeHtml(String(row.id))}" value="${possible===null||possible===undefined?"1":escapeHtml(String(possible))}"></label>
        <label>Credit <input class="teacher-credit-input" type="number" min="0" max="100" step="0.5" data-teacher-credit="${escapeHtml(String(row.id))}" value="${credit===null||credit===undefined?"":escapeHtml(String(credit))}"> %</label>
        <label>Grade <input class="teacher-grade-input" type="text" data-teacher-grade="${escapeHtml(String(row.id))}" value="${escapeHtml(grade)}" placeholder="optional"></label>
        <span class="teacher-feedback-status">Use points, percent, or both. Partial and zero credit are first-class grades.</span>
      </div>
      <textarea class="teacher-notes" data-teacher-notes="${escapeHtml(String(row.id))}" placeholder="Teacher Notes — explain corrections, marked spans, patterns, or what Nova should revisit.">${escapeHtml(String(row?.teacherNotes||""))}</textarea>
      <div class="teacher-annotation-buttons"><button type="button" class="button teacher-annotation-add" data-style="strike" data-teacher-attempt="${escapeHtml(String(row.id))}">Strike selection</button><button type="button" class="button teacher-annotation-add" data-style="highlight" data-teacher-attempt="${escapeHtml(String(row.id))}">Highlight selection</button><button type="button" class="button teacher-annotation-add" data-style="underline" data-teacher-attempt="${escapeHtml(String(row.id))}">Underline selection</button><button type="button" class="button teacher-feedback-save" data-teacher-attempt="${escapeHtml(String(row.id))}">Save Feedback</button></div>
      <div>${marks}</div>
    </div>
    <small>Stored by Rust as non-executable Knowledge with Teacher provenance. Original question and answer are immutable.</small>
  </div>`;
}
function nova92TeacherAttempt(attemptId){return (nova7RustMindBridge?.snapshot?.teacherState?.attempts||[]).find(row=>String(row?.id||"")===String(attemptId||""))||null;}
function nova92ReadTeacherEditor(attemptId){
  const key=CSS.escape(String(attemptId));
  const creditEl=elements.brainContent?.querySelector(`[data-teacher-credit="${key}"]`),awardedEl=elements.brainContent?.querySelector(`[data-teacher-points-awarded="${key}"]`),possibleEl=elements.brainContent?.querySelector(`[data-teacher-points-possible="${key}"]`),gradeEl=elements.brainContent?.querySelector(`[data-teacher-grade="${key}"]`),notesEl=elements.brainContent?.querySelector(`[data-teacher-notes="${key}"]`);
  const numberOrNull=value=>{const raw=String(value??"").trim();if(raw==="")return null;const parsed=Number(raw);return Number.isFinite(parsed)?parsed:null;};
  const awarded=numberOrNull(awardedEl?.value);let possible=numberOrNull(possibleEl?.value);if(awarded===null)possible=null;else if(possible===null)possible=1;
  return {teacherCreditPercent:numberOrNull(creditEl?.value),teacherPointsAwarded:awarded,teacherPointsPossible:possible,teacherGrade:String(gradeEl?.value||"").trim()||null,teacherNotes:String(notesEl?.value||"")};
}
async function nova92CommitTeacherFeedback(attemptId,annotations){
  const row=nova92TeacherAttempt(attemptId);if(!row)throw new Error("Teacher attempt is no longer present in the Rust mind.");const editor=nova92ReadTeacherEditor(attemptId);
  await nova7RustTeacherUpdateFeedback({attemptId:String(attemptId),teacherCreditPercent:editor.teacherCreditPercent,teacherPointsAwarded:editor.teacherPointsAwarded,teacherPointsPossible:editor.teacherPointsPossible,teacherGrade:editor.teacherGrade,teacherNotes:editor.teacherNotes,annotations:nova620UiClone(annotations??row.annotations??[])});
  if(typeof novaSaveDurableCheckpoint==="function")await novaSaveDurableCheckpoint("teacher_feedback");renderBrainInspector();
}
function nova92CaptureTeacherSelection(container){
  const selection=window.getSelection();if(!selection||selection.rangeCount<1||selection.isCollapsed)return;const range=selection.getRangeAt(0);if(!container.contains(range.commonAncestorContainer))return;const prefix=document.createRange();prefix.selectNodeContents(container);prefix.setEnd(range.startContainer,range.startOffset);const selected=document.createRange();selected.selectNodeContents(container);selected.setEnd(range.endContainer,range.endOffset);const start=prefix.toString().length,end=selected.toString().length;if(end<=start)return;nova92TeacherSelection={attemptId:container.dataset.teacherAttempt,target:container.dataset.teacherTarget,start,end};
}
function nova92BindTeacherControls(){
  if(!elements.brainContent)return;
  for(const box of elements.brainContent.querySelectorAll(".teacher-annotatable")){box.addEventListener("mouseup",()=>nova92CaptureTeacherSelection(box));box.addEventListener("keyup",()=>nova92CaptureTeacherSelection(box));}
  for(const button of elements.brainContent.querySelectorAll(".teacher-annotation-add"))button.addEventListener("click",async()=>{try{
    const attemptId=button.dataset.teacherAttempt,style=button.dataset.style;if(!nova92TeacherSelection||nova92TeacherSelection.attemptId!==attemptId){alert("Select exact text in this question or answer first.");return;}
    const row=nova92TeacherAttempt(attemptId),source=nova92TeacherSelection.target==="question"?String(row?.question||""):String(row?.originalAnswer||"");
    const excerpt=source.slice(nova92TeacherSelection.start,nova92TeacherSelection.end);const note=prompt(`Optional Teacher note for this exact ${style} mark:\n\n“${excerpt}”`,"");
    const annotations=nova620UiClone(row?.annotations||[]);annotations.push({id:"",target:nova92TeacherSelection.target,start:nova92TeacherSelection.start,end:nova92TeacherSelection.end,style,note:note===null?null:String(note),createdAt:Date.now()});
    await nova92CommitTeacherFeedback(attemptId,annotations);nova92TeacherSelection=null;
  }catch(error){alert(error?.message||String(error));}});
  for(const button of elements.brainContent.querySelectorAll(".teacher-mark-note"))button.addEventListener("click",async()=>{try{
    const row=nova92TeacherAttempt(button.dataset.teacherAttempt);const annotations=nova620UiClone(row?.annotations||[]);const mark=annotations.find(item=>String(item.id)===String(button.dataset.annotationId));if(!mark)return;
    const source=mark.target==="question"?String(row?.question||""):String(row?.originalAnswer||"");const excerpt=source.slice(Number(mark.start||0),Number(mark.end||0));const note=prompt(`Teacher note attached to this exact marked span:\n\n“${excerpt}”`,String(mark.note||""));if(note===null)return;mark.note=String(note);
    await nova92CommitTeacherFeedback(button.dataset.teacherAttempt,annotations);
  }catch(error){alert(error?.message||String(error));}});
  for(const button of elements.brainContent.querySelectorAll(".teacher-mark-remove"))button.addEventListener("click",async()=>{try{const row=nova92TeacherAttempt(button.dataset.teacherAttempt);const annotations=(row?.annotations||[]).filter(mark=>String(mark.id)!==String(button.dataset.annotationId));await nova92CommitTeacherFeedback(button.dataset.teacherAttempt,annotations);}catch(error){alert(error?.message||String(error));}});
  for(const button of elements.brainContent.querySelectorAll(".teacher-feedback-save"))button.addEventListener("click",async()=>{try{const row=nova92TeacherAttempt(button.dataset.teacherAttempt);await nova92CommitTeacherFeedback(button.dataset.teacherAttempt,row?.annotations||[]);}catch(error){alert(error?.message||String(error));}});
}
function renderBrainInspector(){
  if(!elements.brainContent)return;
  nova92TeacherSelection=null;
  let persistence=null;try{if(typeof nova620PersistenceStatus==="function")persistence=nova620PersistenceStatus();}catch(_){ }
  const rustMind=typeof nova7RustMindStatus==="function"?nova7RustMindStatus():null;const snapshot=rustMind?.snapshot||null;
  if(!snapshot){elements.brainContent.innerHTML=nova620UiEmpty("Rust NovaMind snapshot is unavailable in this runtime. Native Brain Inspector does not reconstruct a JavaScript brain.");return;}
  const canonical=snapshot.cognitiveState||{},semantic=snapshot.semanticMemory?.knowledge||{},learning=snapshot.learningState||{},development=snapshot.developmentState||{},internal=snapshot.internalCognition||{},teacher=snapshot.teacherState||{};
  const lastTurn=snapshot.lastCompletedTurn||snapshot.currentTurn||null,realization=snapshot.lastRealizationReport||null,trace=canonical.cognitiveTrace||[];
  const meaning=`${lastTurn?nova620UiCard("Last input",lastTurn.rawText||"",`<small>${escapeHtml(`${lastTurn.origin||"unknown"} • ${lastTurn.terminalState||realization?.terminalState||"unresolved"} • ${lastTurn.reasonCode||realization?.reasonCode||"no reason code"}`)}</small>`):nova620UiEmpty("No Rust-owned turn has completed yet.")}${realization?nova620UiCard("Rust realization",realization.surface||"[no spoken surface]",`<small>${escapeHtml(`${realization.terminalState||"unknown"} • ${realization.reasonCode||"no reason code"}`)}</small>`):""}${nova92MeaningTrace(trace)}`;
  const english=canonical.englishBootstrap||{};
  const semanticBody=`<div class="brain-explain-grid">${nova620UiCard("Basic English bootstrap",`${english.utteranceKind||"unknown"} • ${(english.clauses||[]).length} structural clause(s)`,`<small>${escapeHtml("Rust preserves wording/structure here without assigning school grammar concepts.")}</small>`)}${nova620UiCard("References",`${(canonical.references||[]).length} represented`)}${nova620UiCard("Entities",`${(canonical.entities||[]).length} represented`)}${nova620UiCard("Events",`${(canonical.events||[]).length} represented`)}${nova620UiCard("Requests",`${(canonical.requests||[]).length} represented`)}${nova620UiCard("Conclusions",`${(canonical.conclusions||[]).length} represented`)}${nova620UiCard("Unresolved",`${(canonical.unresolved||[]).length} remaining`)}</div><details open><summary>Basic English structural observations</summary>${nova620UiPre(english)}</details>${nova620UiPre({references:canonical.references||[],entities:canonical.entities||[],events:canonical.events||[],requests:canonical.requests||[],conclusions:canonical.conclusions||[],unresolved:canonical.unresolved||[]})}`;
  const goalsBody=`${(internal.goals||[]).length?(internal.goals||[]).slice().reverse().slice(0,30).map(goal=>nova620UiCard(goal.description||goal.kind||goal.id,`${goal.status||"unknown"} • ${goal.kind||"goal"} • importance ${Math.round(Number(goal.importance||0)*100)}%`,goal.lastReasonCode?`<small>${escapeHtml(goal.lastReasonCode)}</small>`:"")).join(""):nova620UiEmpty("No internal goals are currently retained.")}`;
  const relationshipsBody=(canonical.relations||[]).length?nova620UiPre(canonical.relations):nova620UiEmpty("No relationships were represented on the current turn.");
  const storedOnly=(learning.rules||[]).filter(rule=>!["module_executable","executable"].includes(String(rule?.status||"")));const storedBody=storedOnly.length?storedOnly.slice(0,60).map(nova620UiStoredRuleCard).join(""):nova620UiEmpty("No Stored Only cognition is currently waiting for capability support.");
  const attempts=(teacher.attempts||[]).slice().reverse();const teacherBody=`${attempts.length?attempts.slice(0,40).map(nova92TeacherAttemptCard).join(""):nova620UiEmpty("No Rust-owned curriculum-test attempts have been recorded yet.")}<div class="brain-card"><strong>Reflection workflow</strong><div class="brain-card-detail">Start Reflection sends an ordinary Teacher utterance through Nova's normal turn path. No reflective questions are hardcoded by the button; preserved test/feedback evidence is available as non-executable Knowledge for Nova to use if her learned cognition supports it.</div></div>`;
  const synthesisBody=`${learning.lastSynthesis?nova620UiPre(learning.lastSynthesis):nova620UiEmpty("No synthesis report yet.")}${nova620UiCard("Experience routes",`${(snapshot.experienceIndex||[]).length} route(s)`)}${nova620UiCard("Development",`${(development.categories||[]).length} categories • ${(development.relations||[]).length} category relationships • ${(development.abstractions||[]).length} provisional abstractions • ${(development.episodes||[]).length} episodes`)}`;
  const traces=typeof nova700DeveloperTraceRows==="function"?nova700DeveloperTraceRows():[];const telemetryTurns=[...traces].reverse().slice(0,12);const telemetryBody=`<div class="brain-card"><strong>Grandpa Rust telemetry</strong><div class="brain-card-detail">Compiler/runtime engineering detail lives here. It is never used to create the human-readable Thinking trace.</div></div>${telemetryTurns.length?telemetryTurns.map((entry,index)=>nova700DeveloperTraceSummary(entry,index,telemetryTurns.length)).join(""):nova620UiEmpty("No completed developer telemetry turns yet.")}<details><summary>Current raw Rust reports</summary>${nova620UiPre({semantic:snapshot.lastSemanticReport||null,cognitive:snapshot.lastCognitiveReport||null,realization:snapshot.lastRealizationReport||null})}</details><details><summary>Current canonical Rust state</summary>${nova620UiPre(canonical)}</details>${persistence?`<details><summary>Persistence diagnostics</summary>${nova620UiPre(persistence)}</details>`:""}`;
  const ownership=snapshot.subsystemOwnership?.owners||{};
  elements.brainContent.innerHTML=`
    ${nova92Fold("Why did Nova do that?",lastTurn?`${lastTurn.terminalState||realization?.terminalState||"unresolved"}`:"no turn yet",meaning,true)}
    ${nova92Fold("Current semantic interpretation",`${(canonical.entities||[]).length+(canonical.events||[]).length+(canonical.relations||[]).length} typed items`,semanticBody,true)}
    ${nova92Fold("Goals & internal cognition",`${(internal.goals||[]).length} retained goal(s)`,goalsBody,false)}
    ${nova92Fold("Relationships",`${(canonical.relations||[]).length} current relationship(s)`,relationshipsBody,false)}
    ${nova92Fold("Knowledge used",`${(semantic.concepts||[]).length} concepts in persistent memory`,nova92KnowledgeUsed(snapshot),true)}
    ${nova92Fold("Stored Only",`${storedOnly.length} item(s)`,storedBody,false)}
    ${nova92Fold("Test history & teacher feedback",`${attempts.length} attempt(s)`,teacherBody,true)}
    ${nova92Fold("Synthesis & development",`revision ${learning.revision||0}`,synthesisBody,false)}
    ${nova92Fold("Grandpa Rust telemetry",`${telemetryTurns.length} recent turn trace(s)`,telemetryBody,false)}
    ${nova92Fold("Rust ownership",`${Object.keys(ownership).length} subsystem declarations`,nova620UiPre({architectureVersion:snapshot.architectureVersion,migrationPhase:snapshot.migrationPhase,reasoningOwner:snapshot.reasoningOwner,subsystemOwnership:ownership}),false)}
  `;
  nova92BindTeacherControls();
}
function openBrain(){renderBrainInspector();elements.appAccessDrawer?.classList.remove("open");elements.brainDrawer?.classList.add("open");elements.drawerBackdrop?.classList.add("open");}
function closeBrain(){elements.brainDrawer?.classList.remove("open");if(!elements.appAccessDrawer?.classList.contains("open"))elements.drawerBackdrop?.classList.remove("open");}

/* ---------------------------------------------------------
   PLAIN-ENGLISH THINKING TRACE PRESENTATION
   --------------------------------------------------------- */
let nova4313ThinkingUi=null;
function nova4313ThinkingLabel(kind){
  const labels={observe:"Notice",interpretation:"Interpret",reference:"Interpret",context:"Connect",relationship:"Connect",goal:"Goal",consideration:"Consider",cognition:"Consider",unresolved:"Unresolved",reasoning:"Connect",learning:"Learn",planning:"Goal",validation:"Check",behavior:"Behavior",memory:"Connect",reflection:"Reflect",conclusion:"Conclusion",limitation:"Unresolved"};
  return labels[kind]||String(kind||"Reason").replace(/_/g," ");
}
function nova4313CreateThinkingUi(){
  if(nova4313ThinkingUi)return nova4313ThinkingUi;
  const row=document.createElement("div");row.className="message-row assistant thinking-row";
  const shell=document.createElement("div");shell.className="thinking-shell";
  const details=document.createElement("details");details.className="thinking-details";
  const summary=document.createElement("summary");summary.className="thinking-summary";
  const spinner=document.createElement("span");spinner.className="thinking-spinner";
  const title=document.createElement("span");title.className="thinking-title";title.textContent="Thinking";
  const status=document.createElement("span");status.className="thinking-status";status.textContent="working";
  summary.append(spinner,title,status);
  const body=document.createElement("div");body.className="thinking-body";
  const steps=document.createElement("div");steps.className="thinking-steps";body.appendChild(steps);
  details.append(summary,body);shell.appendChild(details);row.appendChild(shell);elements.chat?.appendChild(row);
  nova4313ThinkingUi={row,shell,details,summary,spinner,title,status,steps};scrollToBottom();return nova4313ThinkingUi;
}
async function nova4313RenderThinkingTurn(turn){
  const ui=nova4313ThinkingUi||nova4313CreateThinkingUi();
  ui.steps.innerHTML="";
  for(const step of turn?.steps||[]){
    const item=document.createElement("div");item.className=`thinking-step thinking-${step.kind||"reasoning"}`;
    const label=document.createElement("div");label.className="thinking-step-label";label.textContent=nova4313ThinkingLabel(step.kind);
    const text=document.createElement("div");text.className="thinking-step-text";text.textContent=String(step.text||"");
    item.append(label,text);ui.steps.appendChild(item);scrollToBottom();
    const delay=Number(APP_CONFIG.thinkingTrace?.stepDelayMs||0);if(delay)await new Promise(resolve=>setTimeout(resolve,delay));
  }
  ui.spinner.classList.add("complete");ui.status.textContent="finished";ui.summary.classList.add("complete");
  const finalDelay=Number(APP_CONFIG.thinkingTrace?.finalDelayMs||0);if(finalDelay)await new Promise(resolve=>setTimeout(resolve,finalDelay));
}
function nova4313ClearThinkingUiReference(){nova4313ThinkingUi=null;}


/* ---------------------------------------------------------
   V7 ALPHA 9.2 — GRANDPA RUST TELEMETRY HISTORY
   Technical turn history is rendered only inside Brain Inspector.
   --------------------------------------------------------- */
function nova700DeveloperTraceRows(){
  try{return Array.isArray(state?.developerTrace?.history)?state.developerTrace.history:[];}catch(_){return [];}
}
function nova700DeveloperTraceSummary(entry,index,total){
  const sem=entry?.rustSemanticReport||{},cog=entry?.rustCognitiveReport||{},exec=entry?.executive||{};
  const turnNumber=Math.max(1,total-index);
  const prompt=String(entry?.input||"").replace(/\s+/g," ").trim()||"(empty turn)";
  const shortPrompt=prompt.length>92?`${prompt.slice(0,89)}…`:prompt;
  const source=entry?.sourceLabel?`${entry.sourceLabel} • `:"";
  const meta=`${source}${entry?.terminalState||entry?.status||"unknown"} • ${Number(entry?.durationMs||0)} ms`;
  const ids=entry?.canonicalIds||{};
  const stat=(label,value)=>`<div class="developer-trace-stat"><strong>${escapeHtml(label)}</strong>${escapeHtml(String(value??0))}</div>`;
  const technical={
    rustSemanticReport:sem,
    rustCognitiveReport:cog,
    semanticTechnicalTrace:entry?.semanticTechnicalTrace||[],
    executive:exec,
    cognitiveSearch:entry?.cognitiveSearch||null,
    experienceIndexActivity:entry?.experienceIndexActivity||null,
    synthesisDebug:entry?.synthesisDebug||null,
    canonicalIds:ids
  };
  return `<details class="developer-trace-turn"${index===0?" open":""}><summary><span class="developer-trace-prompt">Turn ${turnNumber} — “${escapeHtml(shortPrompt)}”</span><span class="developer-trace-meta">${escapeHtml(meta)}</span></summary><div class="developer-trace-body"><div class="developer-trace-grid">${stat("Rust semantic cycles",sem.semanticCycles||0)}${stat("Concepts active",sem.conceptsActivated||0)}${stat("Assertions",sem.assertionsConstructed||0)}${stat("Requests",`${sem.requestsConstructed||0} / ${sem.requestsResolved||0} resolved`)}${stat("Modules considered",cog.modulesConsidered||0)}${stat("Modules executed",cog.modulesApplied||0)}${stat("Modules changed",cog.modulesChanged||0)}${stat("Search mode",cog.searchMode||entry?.cognitiveSearch?.mode||"n/a")}</div><details><summary>Canonical IDs</summary><pre class="developer-trace-raw">${escapeHtml(nova620UiJson(ids))}</pre></details><details><summary>Exact Rust / executive telemetry</summary><pre class="developer-trace-raw">${escapeHtml(nova620UiJson(technical))}</pre></details></div></details>`;
}
console.info("Nova V7 alpha 9.3 UI loaded — Rust Basic English structure, Teacher feedback, Thinking, and Brain Inspector all read the one Rust-authoritative mind.");
