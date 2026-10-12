"use strict";

/* =========================================================
   Nova V6.2.16 — FREE THOUGHT / DEVELOPMENTAL GOAL INTEGRATION

   This is a current subsystem of the one live mind, not an older executive and
   not a wrapper around turn generation.

   Responsibilities:
   - turn meaningful internal signals into inspectable self-generated goals;
   - run those goals through the same V6.2 central executive as user cognition;
   - preserve blocked internal goals so later learning can make them worth
     reconsidering;
   - record learning/conversation experiences as developmental events;
   - never grant external authority and never surface autonomous text directly.

   The subsystem does NOT contain lesson-shaped reasoning such as "if the lesson
   is about emotions, do X".  It only deals with generic developmental signals:
   changed cognition, unresolved cognition, newly executable cognition, blocked
   goals, reflection opportunities, and activity continuation.
   ========================================================= */

const NOVA621_FREE_THOUGHT_VERSION="6.2.16";
const NOVA621_FREE_THOUGHT_SCHEMA=1;
const NOVA621_SIGNAL_LIMIT=240;
const NOVA621_EXPERIENCE_LIMIT=180;
const NOVA621_HISTORY_LIMIT=300;

function v621NativeRetired(){return Boolean(window.__TAURI__?.core?.invoke);}
function v621NativeRetiredError(){throw new Error("JavaScript free thought is retired on native Nova: Rust Pass 8 owns internal goals and free thought.");}

function v621Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v621Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v621Canon(v){try{return canonical(String(v??""));}catch(_){return v621Clean(v).toLowerCase();}}
function v621Now(){return Date.now();}
function v621Id(prefix){return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2,8)}`;}

function v621FreeThoughtStore(){
  const memory=typeof v43Memory==="function"?v43Memory():null;
  if(!memory)return {schema:NOVA621_FREE_THOUGHT_SCHEMA,version:NOVA621_FREE_THOUGHT_VERSION,signals:[],experiences:[],history:[]};
  const old=memory.freeThought||{};
  if(old.schema!==NOVA621_FREE_THOUGHT_SCHEMA){
    memory.freeThought={
      schema:NOVA621_FREE_THOUGHT_SCHEMA,
      version:NOVA621_FREE_THOUGHT_VERSION,
      signals:Array.isArray(old.signals)?old.signals:[],
      experiences:Array.isArray(old.experiences)?old.experiences:[],
      history:Array.isArray(old.history)?old.history:[],
      lastTickAt:old.lastTickAt||null,
      lastExperienceId:old.lastExperienceId||null,
      createdAt:old.createdAt||v621Now(),
      updatedAt:v621Now()
    };
  }
  memory.freeThought.version=NOVA621_FREE_THOUGHT_VERSION;
  memory.freeThought.signals ||= [];
  memory.freeThought.experiences ||= [];
  memory.freeThought.history ||= [];
  return memory.freeThought;
}

function v621Record(type,detail={}){
  const store=v621FreeThoughtStore();
  store.history.push({id:v621Id("free_event"),type,...v621Clone(detail),at:v621Now()});
  store.history=store.history.slice(-NOVA621_HISTORY_LIMIT);store.updatedAt=v621Now();
}

function v621RuleSnapshot(){
  let rules=[];try{rules=typeof v500CognitiveRuleInventory==="function"?(v500CognitiveRuleInventory()?.rules||[]):[];}catch(_){ }
  return rules.map(r=>({id:r.id||null,directive:v621Clean(r.directive||""),status:r.status||null,moduleId:r.moduleId||null,sourceId:r.sourceId||null,sourceName:r.sourceName||null}));
}

function v621SnapshotDiff(before=[],after=[]){
  const B=new Map((before||[]).filter(x=>x.id).map(x=>[x.id,x])),A=new Map((after||[]).filter(x=>x.id).map(x=>[x.id,x]));
  const added=[],newlyExecutable=[],changed=[];
  for(const [id,row] of A){
    const old=B.get(id);if(!old){added.push(row);if(row.status==="module_executable")newlyExecutable.push(row);continue;}
    if(old.status!==row.status||old.moduleId!==row.moduleId){changed.push({before:old,after:row});if(old.status!=="module_executable"&&row.status==="module_executable")newlyExecutable.push(row);}
  }
  return {added,newlyExecutable,changed};
}

function v621RecordExperience(spec={}){
  if(v621NativeRetired())return v621NativeRetiredError();
  const store=v621FreeThoughtStore(),experience={
    id:v621Id("experience"),
    kind:spec.kind||"cognitive_event",
    source:spec.source||null,
    description:v621Clean(spec.description||""),
    before:v621Clone(spec.before||null),
    after:v621Clone(spec.after||null),
    changes:v621Clone(spec.changes||null),
    detail:v621Clone(spec.detail||null),
    createdAt:v621Now()
  };
  store.experiences.push(experience);store.experiences=store.experiences.slice(-NOVA621_EXPERIENCE_LIMIT);store.lastExperienceId=experience.id;store.updatedAt=v621Now();
  v621Record("EXPERIENCE_RECORDED",{experienceId:experience.id,kind:experience.kind});try{saveState();}catch(_){ }
  return experience;
}

function v621QueueSignal(activityId,signal,experienceId=null){
  if(v621NativeRetired())return v621NativeRetiredError();
  if(!activityId||!signal)return null;let normalized=null;try{normalized=typeof v555NormalizeSignal==="function"?v555NormalizeSignal(signal):signal;}catch(_){normalized=signal;}
  if(!normalized?.description)return null;
  const store=v621FreeThoughtStore(),signature=[activityId,v621Canon(normalized.kind),v621Canon(normalized.description),experienceId||""].join("|");
  if(store.signals.some(x=>x.status==="QUEUED"&&x.signature===signature))return null;
  const item={id:v621Id("free_signal"),activityId,experienceId:experienceId||null,signal:v621Clone(normalized),signature,status:"QUEUED",queuedAt:v621Now(),consumedAt:null};
  store.signals.push(item);store.signals=store.signals.slice(-NOVA621_SIGNAL_LIMIT);store.updatedAt=v621Now();v621Record("FREE_THOUGHT_SIGNAL_QUEUED",{signalId:item.id,activityId,kind:normalized.kind,experienceId});try{saveState();}catch(_){ }return item;
}

function v621QueuedSignals(activityId){return v621FreeThoughtStore().signals.filter(x=>x.activityId===activityId&&x.status==="QUEUED");}
function v621ConsumeNextSignal(activityId){
  const rows=v621QueuedSignals(activityId).sort((a,b)=>Number(b.signal?.importance||0)-Number(a.signal?.importance||0)||a.queuedAt-b.queuedAt),item=rows[0];if(!item)return null;
  item.status="CONSUMED";item.consumedAt=v621Now();v621FreeThoughtStore().updatedAt=v621Now();return v621Clone(item);
}

function v621EnsureReflectionActivity(spec={}){
  if(v621NativeRetired())return v621NativeRetiredError();
  const store=typeof v555ActivityStore==="function"?v555ActivityStore():null;
  if(!store)return null;
  const experienceId=spec.experienceId||null;
  let activity=(store.activities||[]).find(a=>a.status===NOVA555_ACTIVITY_STATUS.ACTIVE&&a.kind==="learning_reflection"&&(!experienceId||a.scope?.experienceId===experienceId));
  if(!activity){
    activity=v555CreateActivity({kind:"learning_reflection",title:spec.title||"Integrate recent experience",purpose:spec.purpose||"Integrate recent learning with current cognition",continuationMode:"TASK_BOUNDED",origin:NOVA555_GOAL_ORIGIN.LEARNING,scope:{interactionClass:"reflection",experienceId}});
  }
  try{if(typeof v556BindMindStateToActivity==="function")v556BindMindStateToActivity(activity.id,{profileId:NOVA556_PROFILE.LEARNING_REFLECTION});}catch(_){ }
  return activity;
}

function v621LearningSignals(experience,synthesis,diff){
  const signals=[];const remaining=Number(synthesis?.remaining||0),upgraded=Number(synthesis?.upgraded||diff?.newlyExecutable?.length||0);
  if(remaining>0)signals.push({kind:"learning_gap",description:"Reconsider Stored Only cognition affected by the recent learning experience",importance:.92,source:"developmental_change",detail:{experienceId:experience.id,remaining}});
  if(upgraded>0)signals.push({kind:"reflection",description:"Revisit older knowledge that may now connect to newly executable cognition",importance:.88,source:"developmental_change",detail:{experienceId:experience.id,upgraded}});
  signals.push({kind:"reflection",description:"Integrate the recent learning experience with related existing cognition",importance:.72,source:"learning_experience",detail:{experienceId:experience.id}});
  let blocked=[];try{blocked=(v555ActivityStore()?.goals||[]).filter(g=>g.origin===NOVA555_GOAL_ORIGIN.SELF&&g.status===NOVA555_GOAL_STATUS.BLOCKED).slice(-6);}catch(_){ }
  if((upgraded>0||diff?.changed?.length)&&blocked.length)signals.push({kind:"reflection",description:"Reconsider previously blocked internal goals after the change in available cognition",importance:.84,source:"developmental_change",detail:{experienceId:experience.id,blockedGoalIds:blocked.map(g=>g.id)}});
  return signals;
}

function v621GoalResultStatus(report){
  const terminal=report?.workspace?.executive?.terminalState||report?.executive?.terminalState||null;
  if(terminal==="SUPPORTED")return NOVA555_GOAL_STATUS.SUPPORTED;
  if(terminal==="BLOCKED")return NOVA555_GOAL_STATUS.BLOCKED;
  return NOVA555_GOAL_STATUS.EXHAUSTED;
}

async function v621ExecuteGoal(goal,options={}){
  if(v621NativeRetired())return v621NativeRetiredError();
  if(!goal)return null;
  if(typeof v620RunInternalGoal!=="function")return {goal:v621Clone(goal),status:"CURRENT_EXECUTIVE_UNAVAILABLE"};
  const result=await v620RunInternalGoal(goal,{origin:"free_thought",experienceId:options.experienceId||null});
  const stored=typeof v555GoalById==="function"?v555GoalById(goal.id):null;
  v621Record("FREE_THOUGHT_GOAL_EXECUTED",{goalId:goal.id,activityId:goal.activityId,status:stored?.status||v621GoalResultStatus(result),reasonCode:stored?.reasonCode||result?.workspace?.executive?.reasonCode||null,experienceId:options.experienceId||null});
  return result;
}

async function v621RunFreeThoughtTick(activityId=null,options={}){
  if(v621NativeRetired())return v621NativeRetiredError();
  let activity=activityId&&typeof v555ActivityById==="function"?v555ActivityById(activityId):null;
  if(!activity){try{activity=(typeof v555ActiveActivities==="function"?v555ActiveActivities():[]).filter(a=>a.kind==="learning_reflection").at(-1)||null;}catch(_){ }}
  if(!activity||activity.status!==NOVA555_ACTIVITY_STATUS.ACTIVE)return {status:"NO_ACTIVE_FREE_THOUGHT_ACTIVITY",activity:null};

  let mindState=null;try{mindState=(typeof v556MindStore==="function"?v556MindStore().activityBindings:[]).map(b=>b.activityId===activity.id?typeof v556MindStateById==="function"?v556MindStateById(b.mindStateId):null:null).find(Boolean)||null;if(!mindState&&typeof v556BindMindStateToActivity==="function")mindState=v556BindMindStateToActivity(activity.id,{profileId:NOVA556_PROFILE.LEARNING_REFLECTION});}catch(_){ }
  const policy=mindState?.envelope?.internal?.selfGeneratedGoals;
  if(policy===NOVA556_SELF_GOALS.NONE)return {status:"SELF_GOALS_DISABLED",activity:v621Clone(activity),mindState:v621Clone(mindState)};

  const maxGoals=Math.max(1,Math.min(6,Number(options.maxGoals||3))),executed=[];let generated=0;
  for(let i=0;i<maxGoals;i++){
    let activeGoal=null;try{activeGoal=v555GoalsForActivity(activity.id).find(g=>g.status===NOVA555_GOAL_STATUS.ACTIVE)||null;}catch(_){ }
    let signalItem=null;
    if(!activeGoal){
      signalItem=v621ConsumeNextSignal(activity.id);if(!signalItem)break;
      const advanced=v555AdvanceActivity(activity.id,[signalItem.signal]);activeGoal=advanced?.generatedGoal||null;if(activeGoal)generated++;
    }
    if(!activeGoal)continue;
    const result=await v621ExecuteGoal(activeGoal,{experienceId:signalItem?.experienceId||options.experienceId||null});executed.push({goalId:activeGoal.id,result:v621Clone(result)});
  }

  const queued=v621QueuedSignals(activity.id).length,activeGoals=v555GoalsForActivity(activity.id).filter(g=>g.status===NOVA555_GOAL_STATUS.ACTIVE).length;
  if(!queued&&!activeGoals){const decision=v555EvaluateActivityContinuation(activity.id,{signals:[]});if(decision&&decision.decision!==NOVA555_ACTIVITY_DECISION.CONTINUE)v555ApplyActivityDecision(activity.id,decision);}
  const store=v621FreeThoughtStore();store.lastTickAt=v621Now();store.updatedAt=v621Now();v621Record("FREE_THOUGHT_TICK",{activityId:activity.id,generated,executed:executed.length,queuedRemaining:queued});try{saveState();}catch(_){ }
  return {status:"TICK_COMPLETE",activity:v621Clone(v555ActivityById(activity.id)),mindState:v621Clone(mindState),generated,executed,queuedRemaining:queued};
}

async function v621AfterLearning(spec={}){
  if(v621NativeRetired())return v621NativeRetiredError();
  const before=spec.beforeRules||[],after=spec.afterRules||v621RuleSnapshot(),diff=v621SnapshotDiff(before,after);
  const experience=v621RecordExperience({kind:"learning",source:spec.source||"conversation",description:"Learning changed Nova's stored cognitive state",before:{rules:before.length},after:{rules:after.length},changes:{rulesAdded:diff.added.length,newlyExecutable:diff.newlyExecutable.length,rulesChanged:diff.changed.length},detail:{synthesis:v621Clone(spec.synthesis||null),learning:v621Clone(spec.learning||null),cognitiveLearning:v621Clone(spec.cognitiveLearning||null)}});
  const activity=v621EnsureReflectionActivity({experienceId:experience.id,title:"Reflect on recent learning"});if(!activity)return {experience,activity:null,tick:null};
  for(const signal of v621LearningSignals(experience,spec.synthesis,diff))v621QueueSignal(activity.id,signal,experience.id);
  const tick=await v621RunFreeThoughtTick(activity.id,{experienceId:experience.id,maxGoals:spec.maxGoals||3});
  return {experience:v621Clone(experience),activity:v621Clone(v555ActivityById(activity.id)),diff:v621Clone(diff),tick:v621Clone(tick)};
}

async function v621AfterBlockedTurn(workspace){
  if(v621NativeRetired())return v621NativeRetiredError();
  if(!workspace||workspace.executive?.terminalState!=="BLOCKED")return null;
  const descriptions=(workspace.canonicalSemanticState?.unresolved||[]).map(x=>v621Clean(x.description||x.label||x.type||"")).filter(Boolean);if(!descriptions.length)return null;
  const experience=v621RecordExperience({kind:"conversation_block",source:"conversation",description:"A conversation exposed unresolved cognition",detail:{reasonCode:workspace.executive?.reasonCode||null,unresolved:descriptions}});
  const activity=v621EnsureReflectionActivity({experienceId:experience.id,title:"Reconsider unresolved conversation"});if(!activity)return null;
  v621QueueSignal(activity.id,{kind:"unresolved",description:descriptions[0],importance:.76,source:"conversation_experience",detail:{experienceId:experience.id,reasonCode:workspace.executive?.reasonCode||null}},experience.id);
  return v621RunFreeThoughtTick(activity.id,{experienceId:experience.id,maxGoals:1});
}

function v621FreeThoughtSummary(){
  if(v621NativeRetired()){const rust=nova7RustMindBridge?.snapshot?.internalCognition||null,goals=rust?.goals||[];return {schema:rust?.schemaVersion||1,version:"7.0.0-alpha.8",authority:"rust_v7_pass_8",selfGeneratedGoals:goals.length,queuedSignals:goals.filter(g=>g.status==="QUEUED").length,blockedSelfGoals:goals.filter(g=>g.status==="BLOCKED").length,historyEvents:(rust?.history||[]).length,lastGoal:goals.at(-1)||null};}
  const store=v621FreeThoughtStore();let goals=[];try{goals=v555ActivityStore()?.goals||[];}catch(_){ }
  return {schema:store.schema,version:store.version,experiences:store.experiences.length,queuedSignals:store.signals.filter(x=>x.status==="QUEUED").length,historyEvents:store.history.length,lastTickAt:store.lastTickAt,selfGeneratedGoals:goals.filter(g=>g.origin===NOVA555_GOAL_ORIGIN.SELF).length,blockedSelfGoals:goals.filter(g=>g.origin===NOVA555_GOAL_ORIGIN.SELF&&g.status===NOVA555_GOAL_STATUS.BLOCKED).length,lastExperience:store.experiences.at(-1)||null};
}

try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA621_FREE_THOUGHT_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const entries=[
      {id:"free_thought_current_executive",verb:"advance",object:"self-generated internal goals through the same current executive used for other cognition",detail:"Free thought is not an older responder or a second executive. Internal goals use the live Generic Modules mind and may remain blocked until later learning changes what cognition is available."},
      {id:"developmental_experience_memory",verb:"retain",object:"learning and unresolved-conversation events as developmental experiences",detail:"Experiences can generate internal reflection opportunities and later changes in cognition can make earlier blocked goals worth reconsidering."}
    ];
    for(const entry of entries){const old=V41_SELF_MODEL.capabilities.find(c=>c.id===entry.id);if(old)Object.assign(old,entry);else V41_SELF_MODEL.capabilities.push(entry);}
  }
}catch(_){ }

try{if(!v621NativeRetired())v621FreeThoughtStore();}catch(_){ }
console.info("Nova legacy browser-only free thought loaded; native Pass 8 internal goals are Rust-owned.");
