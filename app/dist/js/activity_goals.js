"use strict";

/* =========================================================
   Nova V6.2.16 — ACTIVITY + GOAL STATE

   Fifth staged piece of the developmental-brain architecture.

   This release gives Nova a persistent representation for three different
   cognitive scales:

     operation  — a short cognitive transformation/inspection;
     goal       — a bounded thing Nova is trying to accomplish;
     activity   — an open or bounded session that can contain many goals.

   A completed goal does NOT automatically complete an activity. An active
   activity may generate a new internal goal from a structured unresolved
   need, practice opportunity, question, reflection follow-up, or other
   explicit activity signal.

   V6.2 BOUNDARY:
   - activity sessions are persistent cognitive state;
   - self-generated goals are inspectable intentions, not external actions;
   - this module is NOT connected to Minecraft/app/web control yet;
   - it does not grant permissions or execute external actions;
   - goals/activities are state available to the current central mind;
   - they never grant external authority by themselves.
   ========================================================= */

const NOVA555_VERSION="6.2.16";
const NOVA555_SCHEMA=1;
const NOVA555_MIGRATION_SCHEMA=1;
const NOVA555_ACTIVITY_LIMIT=120;
const NOVA555_GOAL_LIMIT=500;
const NOVA555_HISTORY_LIMIT=240;
const NOVA555_RECENT_FINGERPRINT_LIMIT=120;

const NOVA555_ACTIVITY_STATUS=Object.freeze({
  ACTIVE:"ACTIVE",
  PAUSED:"PAUSED",
  ENDED:"ENDED"
});
const NOVA555_GOAL_STATUS=Object.freeze({
  OPEN:"OPEN",
  ACTIVE:"ACTIVE",
  SUPPORTED:"SUPPORTED",
  BLOCKED:"BLOCKED",
  EXHAUSTED:"EXHAUSTED",
  CANCELLED:"CANCELLED"
});
const NOVA555_ACTIVITY_DECISION=Object.freeze({
  CONTINUE:"CONTINUE",
  PAUSE:"PAUSE",
  END:"END"
});
const NOVA555_GOAL_ORIGIN=Object.freeze({
  USER:"USER",
  SELF:"SELF_GENERATED",
  LEARNING:"LEARNING",
  EXECUTIVE:"EXECUTIVE",
  SYSTEM:"SYSTEM"
});

function v555Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v555Canon(v){try{return canonical(String(v??""));}catch(_){return String(v??"").trim().toLowerCase();}}
function v555Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v555Key(v){return v555Canon(v).replace(/[^a-z0-9]+/g,"_").replace(/^_+|_+$/g,"");}
function v555Unique(xs,keyFn=x=>typeof x==="object"?JSON.stringify(x):String(x)){const out=[],seen=new Set();for(const x of xs||[]){if(x==null)continue;const k=keyFn(x);if(!k||seen.has(k))continue;seen.add(k);out.push(x);}return out;}
function v555Now(){return Date.now();}
function v555Id(prefix){return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2,8)}`;}

function v555ActivityStore(){
  const memory=v43Memory();
  if(!memory.activityCognition||memory.activityCognition.schema!==NOVA555_SCHEMA){
    const old=memory.activityCognition||{};
    memory.activityCognition={
      schema:NOVA555_SCHEMA,
      version:NOVA555_VERSION,
      activities:Array.isArray(old.activities)?old.activities:[],
      goals:Array.isArray(old.goals)?old.goals:[],
      history:Array.isArray(old.history)?old.history.slice(-NOVA555_HISTORY_LIMIT):[],
      recentGoalFingerprints:Array.isArray(old.recentGoalFingerprints)?old.recentGoalFingerprints.slice(-NOVA555_RECENT_FINGERPRINT_LIMIT):[],
      lastActivityId:old.lastActivityId||null,
      lastGoalId:old.lastGoalId||null,
      migration:old.migration||null,
      createdAt:old.createdAt||v555Now(),
      updatedAt:v555Now()
    };
  }
  memory.activityCognition.version=NOVA555_VERSION;
  memory.activityCognition.activities ||= [];
  memory.activityCognition.goals ||= [];
  memory.activityCognition.history ||= [];
  memory.activityCognition.recentGoalFingerprints ||= [];
  return memory.activityCognition;
}

function v555RecordHistory(type,detail={}){
  const store=v555ActivityStore();
  store.history.push({id:v555Id("activity_event"),type,...v555Clone(detail),at:v555Now()});
  store.history=store.history.slice(-NOVA555_HISTORY_LIMIT);store.updatedAt=v555Now();
}

function v555ActivityById(id){return v555ActivityStore().activities.find(a=>a.id===id)||null;}
function v555GoalById(id){return v555ActivityStore().goals.find(g=>g.id===id)||null;}
function v555GoalsForActivity(activityId){return v555ActivityStore().goals.filter(g=>g.activityId===activityId);}
function v555ActiveActivities(){return v555ActivityStore().activities.filter(a=>a.status===NOVA555_ACTIVITY_STATUS.ACTIVE);}
function v555MostRecentActivity(){const store=v555ActivityStore();return v555ActivityById(store.lastActivityId)||store.activities.at(-1)||null;}

function v555CreateActivity(spec={}){
  const store=v555ActivityStore();
  const title=v555Clean(spec.title||spec.label||spec.purpose||spec.kind||"activity");
  const kind=v555Key(spec.kind||"general_activity")||"general_activity";
  const continuationMode=spec.continuationMode==="TASK_BOUNDED"?"TASK_BOUNDED":"OPEN_ENDED";
  const activity={
    id:v555Id("activity"),
    type:"activity",
    kind,
    title,
    purpose:v555Clean(spec.purpose||title),
    scope:v555Clone(spec.scope||null),
    status:NOVA555_ACTIVITY_STATUS.ACTIVE,
    continuationMode,
    origin:spec.origin||NOVA555_GOAL_ORIGIN.USER,
    parentActivityId:spec.parentActivityId||null,
    rootGoalId:null,
    activeGoalIds:[],
    completedGoalIds:[],
    blockedGoalIds:[],
    exhaustedGoalIds:[],
    generatedGoalIds:[],
    observations:[],
    continuation:{decision:NOVA555_ACTIVITY_DECISION.CONTINUE,reason:"ACTIVITY_CREATED",evaluatedAt:v555Now()},
    externalAuthority:{granted:false,note:"Activity state never grants external-action authority; the trusted runtime host does."},
    createdAt:v555Now(),
    updatedAt:v555Now(),
    pausedAt:null,
    endedAt:null,
    endReason:null
  };
  store.activities.push(activity);store.activities=store.activities.slice(-NOVA555_ACTIVITY_LIMIT);store.lastActivityId=activity.id;store.updatedAt=v555Now();
  if(spec.goal){
    const root=v555CreateGoal(activity.id,{...spec.goal,origin:spec.goal.origin||activity.origin,parentGoalId:null});
    if(root)activity.rootGoalId=root.id;
  }
  v555RecordHistory("ACTIVITY_CREATED",{activityId:activity.id,kind,title,continuationMode});
  try{saveState();}catch(_){ }
  return activity;
}

function v555GoalFingerprint(activityId,spec={}){
  return [activityId||"no_activity",v555Key(spec.goalType||spec.type||"goal"),v555Canon(spec.description||spec.target||""),v555Canon(spec.target||"")].join("|");
}

function v555CreateGoal(activityId,spec={}){
  const activity=v555ActivityById(activityId);if(!activity||activity.status===NOVA555_ACTIVITY_STATUS.ENDED)return null;
  const description=v555Clean(spec.description||spec.target||spec.goal||"");if(!description)return null;
  const fingerprint=v555GoalFingerprint(activityId,spec);
  const existing=v555GoalsForActivity(activityId).find(g=>[NOVA555_GOAL_STATUS.OPEN,NOVA555_GOAL_STATUS.ACTIVE].includes(g.status)&&g.fingerprint===fingerprint);
  if(existing)return existing;
  const goal={
    id:v555Id("goal"),
    type:"goal",
    activityId,
    parentGoalId:spec.parentGoalId||null,
    goalType:v555Key(spec.goalType||spec.type||"activity_goal")||"activity_goal",
    description,
    target:v555Clone(spec.target??null),
    origin:spec.origin||NOVA555_GOAL_ORIGIN.USER,
    status:spec.status===NOVA555_GOAL_STATUS.OPEN?NOVA555_GOAL_STATUS.OPEN:NOVA555_GOAL_STATUS.ACTIVE,
    priority:Number.isFinite(Number(spec.priority))?Math.max(0,Math.min(1,Number(spec.priority))):0.7,
    generationBasis:v555Clone(spec.generationBasis||null),
    evidence:v555Clone(spec.evidence||[]),
    unresolved:v555Clone(spec.unresolved||[]),
    result:null,
    reasonCode:null,
    fingerprint,
    externallyExecutable:false,
    createdAt:v555Now(),
    startedAt:v555Now(),
    completedAt:null,
    updatedAt:v555Now()
  };
  const store=v555ActivityStore();store.goals.push(goal);store.goals=store.goals.slice(-NOVA555_GOAL_LIMIT);store.lastGoalId=goal.id;store.updatedAt=v555Now();
  if(goal.status===NOVA555_GOAL_STATUS.ACTIVE&&!activity.activeGoalIds.includes(goal.id))activity.activeGoalIds.push(goal.id);
  if(goal.origin===NOVA555_GOAL_ORIGIN.SELF&&!activity.generatedGoalIds.includes(goal.id))activity.generatedGoalIds.push(goal.id);
  activity.updatedAt=v555Now();
  v555RecordHistory("GOAL_CREATED",{activityId,goalId:goal.id,origin:goal.origin,goalType:goal.goalType,description:goal.description});
  try{saveState();}catch(_){ }
  return goal;
}

function v555SetGoalTerminal(goalId,status,detail={}){
  const allowed=[NOVA555_GOAL_STATUS.SUPPORTED,NOVA555_GOAL_STATUS.BLOCKED,NOVA555_GOAL_STATUS.EXHAUSTED,NOVA555_GOAL_STATUS.CANCELLED];if(!allowed.includes(status))return null;
  const goal=v555GoalById(goalId);if(!goal)return null;const activity=v555ActivityById(goal.activityId);
  goal.status=status;goal.result=v555Clone(detail.result??null);goal.reasonCode=detail.reasonCode||status;goal.completedAt=v555Now();goal.updatedAt=v555Now();
  if(activity){
    activity.activeGoalIds=activity.activeGoalIds.filter(id=>id!==goal.id);
    if(status===NOVA555_GOAL_STATUS.SUPPORTED&&!activity.completedGoalIds.includes(goal.id))activity.completedGoalIds.push(goal.id);
    if(status===NOVA555_GOAL_STATUS.BLOCKED&&!activity.blockedGoalIds.includes(goal.id))activity.blockedGoalIds.push(goal.id);
    if(status===NOVA555_GOAL_STATUS.EXHAUSTED&&!activity.exhaustedGoalIds.includes(goal.id))activity.exhaustedGoalIds.push(goal.id);
    activity.updatedAt=v555Now();
  }
  v555RecordHistory("GOAL_TERMINAL",{activityId:goal.activityId,goalId:goal.id,status,reasonCode:goal.reasonCode});
  try{saveState();}catch(_){ }
  return goal;
}
function v555CompleteGoal(goalId,detail={}){return v555SetGoalTerminal(goalId,NOVA555_GOAL_STATUS.SUPPORTED,detail);}
function v555BlockGoal(goalId,reasonCode="BLOCKED",detail={}){return v555SetGoalTerminal(goalId,NOVA555_GOAL_STATUS.BLOCKED,{...detail,reasonCode});}
function v555ExhaustGoal(goalId,reasonCode="EXHAUSTED",detail={}){return v555SetGoalTerminal(goalId,NOVA555_GOAL_STATUS.EXHAUSTED,{...detail,reasonCode});}

function v555NormalizeSignal(signal,index=0){
  if(!signal)return null;
  if(typeof signal==="string")return {id:`signal_${index}`,kind:"opportunity",description:v555Clean(signal),importance:.55,source:"structured_activity_signal"};
  const description=v555Clean(signal.description||signal.question||signal.label||signal.target||"");if(!description)return null;
  const kind=v555Key(signal.kind||signal.type||"opportunity")||"opportunity";
  const importance=Math.max(0,Math.min(1,Number(signal.importance??signal.priority??.6)));
  return {id:signal.id||`signal_${index}`,kind,description,target:v555Clone(signal.target??null),importance,source:signal.source||"structured_activity_signal",detail:v555Clone(signal.detail||null)};
}

function v555CandidateFromSignal(activity,signal){
  const s=v555NormalizeSignal(signal);if(!s)return null;
  const map={
    unresolved:{goalType:"resolve_uncertainty",verb:"Resolve"},
    uncertainty:{goalType:"investigate_uncertainty",verb:"Investigate"},
    question:{goalType:"clarify_question",verb:"Clarify"},
    practice:{goalType:"practice_skill",verb:"Practice"},
    learning_gap:{goalType:"resolve_learning_gap",verb:"Resolve"},
    reflection:{goalType:"reflection_followup",verb:"Revisit"},
    cooperation:{goalType:"cooperate",verb:"Help with"},
    opportunity:{goalType:"pursue_opportunity",verb:"Pursue"}
  };
  const chosen=map[s.kind]||map.opportunity;
  const description=/^(?:resolve|investigate|clarify|practice|revisit|help|pursue)\b/i.test(s.description)?s.description:`${chosen.verb} ${s.description}`;
  return {goalType:chosen.goalType,description,target:s.target,priority:s.importance,origin:NOVA555_GOAL_ORIGIN.SELF,generationBasis:{signalId:s.id,signalKind:s.kind,source:s.source,description:s.description},evidence:[s]};
}

function v555ProposeSelfGeneratedGoals(activityId,signals=[]){
  const activity=v555ActivityById(activityId);if(!activity||activity.status!==NOVA555_ACTIVITY_STATUS.ACTIVE)return [];
  const store=v555ActivityStore();const candidates=[];
  for(let i=0;i<(signals||[]).length;i++){
    const candidate=v555CandidateFromSignal(activity,v555NormalizeSignal(signals[i],i));if(!candidate)continue;
    const fingerprint=v555GoalFingerprint(activityId,candidate);
    const activeDuplicate=v555GoalsForActivity(activityId).some(g=>[NOVA555_GOAL_STATUS.OPEN,NOVA555_GOAL_STATUS.ACTIVE].includes(g.status)&&g.fingerprint===fingerprint);
    const recentDuplicate=store.recentGoalFingerprints.some(x=>x.fingerprint===fingerprint&&v555Now()-Number(x.at||0)<10*60*1000);
    if(activeDuplicate||recentDuplicate)continue;
    candidates.push({...candidate,fingerprint});
  }
  return candidates.sort((a,b)=>Number(b.priority||0)-Number(a.priority||0));
}

function v555GenerateNextGoal(activityId,signals=[]){
  const activity=v555ActivityById(activityId);if(!activity||activity.status!==NOVA555_ACTIVITY_STATUS.ACTIVE)return null;
  const candidates=v555ProposeSelfGeneratedGoals(activityId,signals);const candidate=candidates[0];if(!candidate)return null;
  const goal=v555CreateGoal(activityId,candidate);if(!goal)return null;
  const store=v555ActivityStore();store.recentGoalFingerprints.push({fingerprint:goal.fingerprint,goalId:goal.id,activityId,at:v555Now()});store.recentGoalFingerprints=store.recentGoalFingerprints.slice(-NOVA555_RECENT_FINGERPRINT_LIMIT);store.updatedAt=v555Now();
  v555RecordHistory("SELF_GOAL_GENERATED",{activityId,goalId:goal.id,basis:v555Clone(goal.generationBasis),priority:goal.priority});
  try{saveState();}catch(_){ }
  return goal;
}

function v555EvaluateActivityContinuation(activityId,options={}){
  const activity=v555ActivityById(activityId);if(!activity)return null;
  if(activity.status===NOVA555_ACTIVITY_STATUS.ENDED)return {decision:NOVA555_ACTIVITY_DECISION.END,reason:activity.endReason||"ALREADY_ENDED",activityId,evaluatedAt:v555Now()};
  if(options.explicitEnd===true||options.sessionComplete===true)return {decision:NOVA555_ACTIVITY_DECISION.END,reason:options.reason||"ACTIVITY_END_REQUESTED",activityId,evaluatedAt:v555Now()};
  if(options.explicitPause===true)return {decision:NOVA555_ACTIVITY_DECISION.PAUSE,reason:options.reason||"ACTIVITY_PAUSE_REQUESTED",activityId,evaluatedAt:v555Now()};
  const active=v555GoalsForActivity(activityId).filter(g=>g.status===NOVA555_GOAL_STATUS.ACTIVE);
  if(active.length)return {decision:NOVA555_ACTIVITY_DECISION.CONTINUE,reason:"ACTIVE_GOAL_REMAINS",activityId,activeGoalIds:active.map(g=>g.id),evaluatedAt:v555Now()};
  const candidates=v555ProposeSelfGeneratedGoals(activityId,options.signals||[]);
  if(candidates.length)return {decision:NOVA555_ACTIVITY_DECISION.CONTINUE,reason:"MEANINGFUL_NEXT_GOAL_AVAILABLE",activityId,candidates:v555Clone(candidates.slice(0,5)),evaluatedAt:v555Now()};
  const blocked=v555GoalsForActivity(activityId).filter(g=>g.status===NOVA555_GOAL_STATUS.BLOCKED);
  if(options.waitingForExternal===true||blocked.some(g=>/RESOURCE|AUTH|WAIT|INPUT/i.test(String(g.reasonCode||""))))return {decision:NOVA555_ACTIVITY_DECISION.PAUSE,reason:"WAITING_ON_EXTERNAL_CONDITION",activityId,evaluatedAt:v555Now()};
  if(activity.continuationMode==="TASK_BOUNDED")return {decision:NOVA555_ACTIVITY_DECISION.END,reason:"BOUNDED_ACTIVITY_HAS_NO_OPEN_GOAL",activityId,evaluatedAt:v555Now()};
  return {decision:NOVA555_ACTIVITY_DECISION.PAUSE,reason:"OPEN_ACTIVITY_HAS_NO_CURRENT_MEANINGFUL_GOAL",activityId,evaluatedAt:v555Now()};
}

function v555ApplyActivityDecision(activityId,decision){
  const activity=v555ActivityById(activityId);if(!activity||!decision)return null;
  activity.continuation=v555Clone(decision);activity.updatedAt=v555Now();
  if(decision.decision===NOVA555_ACTIVITY_DECISION.PAUSE){activity.status=NOVA555_ACTIVITY_STATUS.PAUSED;activity.pausedAt=v555Now();}
  if(decision.decision===NOVA555_ACTIVITY_DECISION.END){
    activity.status=NOVA555_ACTIVITY_STATUS.ENDED;activity.endedAt=v555Now();activity.endReason=decision.reason||"ENDED";
    for(const goal of v555GoalsForActivity(activityId)){
      if([NOVA555_GOAL_STATUS.OPEN,NOVA555_GOAL_STATUS.ACTIVE].includes(goal.status)){goal.status=NOVA555_GOAL_STATUS.CANCELLED;goal.reasonCode="ACTIVITY_ENDED";goal.completedAt=v555Now();goal.updatedAt=v555Now();}
    }
    activity.activeGoalIds=[];
  }
  v555RecordHistory("ACTIVITY_DECISION",{activityId,decision:decision.decision,reason:decision.reason});
  try{saveState();}catch(_){ }
  return activity;
}

function v555ResumeActivity(activityId,reason="RESUMED"){
  const activity=v555ActivityById(activityId);if(!activity||activity.status===NOVA555_ACTIVITY_STATUS.ENDED)return null;
  activity.status=NOVA555_ACTIVITY_STATUS.ACTIVE;activity.pausedAt=null;activity.continuation={decision:NOVA555_ACTIVITY_DECISION.CONTINUE,reason,evaluatedAt:v555Now()};activity.updatedAt=v555Now();
  v555RecordHistory("ACTIVITY_RESUMED",{activityId,reason});try{saveState();}catch(_){ }return activity;
}

function v555EndActivity(activityId,reason="ACTIVITY_ENDED"){
  return v555ApplyActivityDecision(activityId,{decision:NOVA555_ACTIVITY_DECISION.END,reason,activityId,evaluatedAt:v555Now()});
}

function v555AdvanceActivity(activityId,signals=[],options={}){
  const activity=v555ActivityById(activityId);if(!activity)return null;
  if(activity.status===NOVA555_ACTIVITY_STATUS.PAUSED&&options.resume===true)v555ResumeActivity(activityId,"ADVANCE_RESUME");
  if(v555ActivityById(activityId)?.status!==NOVA555_ACTIVITY_STATUS.ACTIVE)return {activity:v555Clone(v555ActivityById(activityId)),generatedGoal:null,decision:v555EvaluateActivityContinuation(activityId,options)};
  for(const signal of signals||[]){const normalized=v555NormalizeSignal(signal);if(normalized)activity.observations.push({...normalized,at:v555Now()});}
  activity.observations=activity.observations.slice(-80);
  let generatedGoal=null;
  const active=v555GoalsForActivity(activityId).some(g=>g.status===NOVA555_GOAL_STATUS.ACTIVE);
  if(!active)generatedGoal=v555GenerateNextGoal(activityId,signals);
  const decision=v555EvaluateActivityContinuation(activityId,{...options,signals});
  activity.continuation=v555Clone(decision);activity.updatedAt=v555Now();
  v555RecordHistory("ACTIVITY_ADVANCED",{activityId,generatedGoalId:generatedGoal?.id||null,decision:decision?.decision||null,reason:decision?.reason||null});
  try{saveState();}catch(_){ }
  return {activity:v555Clone(activity),generatedGoal:v555Clone(generatedGoal),decision:v555Clone(decision)};
}

function v555CognitiveScaleModel(){
  return {
    version:NOVA555_VERSION,
    levels:[
      {id:"operation",duration:"short",description:"One cognitive transformation, inspection, retrieval, comparison, or update inside a goal."},
      {id:"goal",duration:"bounded",description:"A resolvable intention that can terminate supported, blocked, exhausted, or cancelled without automatically ending its containing activity."},
      {id:"activity",duration:"session",description:"A continuing context that may contain many user-provided and self-generated goals and can continue after one subgoal completes."}
    ],
    boundary:"Self-generated goals are internal cognitive intentions only. External actions remain separate from internal goals and require the current mind-state envelope plus the trusted application capability gate."
  };
}

function v555ActivitySummary(){
  const store=v555ActivityStore();const activities=store.activities||[],goals=store.goals||[];const last=v555MostRecentActivity();
  return {
    schema:store.schema,version:NOVA555_VERSION,
    activities:activities.length,
    activeActivities:activities.filter(a=>a.status===NOVA555_ACTIVITY_STATUS.ACTIVE).length,
    pausedActivities:activities.filter(a=>a.status===NOVA555_ACTIVITY_STATUS.PAUSED).length,
    endedActivities:activities.filter(a=>a.status===NOVA555_ACTIVITY_STATUS.ENDED).length,
    goals:goals.length,
    activeGoals:goals.filter(g=>g.status===NOVA555_GOAL_STATUS.ACTIVE).length,
    selfGeneratedGoals:goals.filter(g=>g.origin===NOVA555_GOAL_ORIGIN.SELF).length,
    supportedGoals:goals.filter(g=>g.status===NOVA555_GOAL_STATUS.SUPPORTED).length,
    blockedGoals:goals.filter(g=>g.status===NOVA555_GOAL_STATUS.BLOCKED).length,
    exhaustedGoals:goals.filter(g=>g.status===NOVA555_GOAL_STATUS.EXHAUSTED).length,
    lastActivity:last?v555Clone({id:last.id,title:last.title,kind:last.kind,status:last.status,continuation:last.continuation,activeGoalIds:last.activeGoalIds,generatedGoalIds:last.generatedGoalIds}):null
  };
}

function v555PostLoadActivityMaintenance(){
  const store=v555ActivityStore();
  const migration={schema:NOVA555_MIGRATION_SCHEMA,version:NOVA555_VERSION,ranAt:v555Now(),reason:"current_activity_store_verified"};
  store.migration=v555Clone(migration);store.updatedAt=v555Now();try{saveState();}catch(_){ }return migration;
}

/* ---------------------------------------------------------
   CURRENT ACTIVITY / GOAL COGNITIVE REGISTRY
   --------------------------------------------------------- */
function v555ActivityCognitiveRegistry(){
  return {
    version:NOVA555_VERSION,
    staged:false,
    liveExecutiveWiring:true,
    operations:[
      {id:"create_activity_session",level:"activity",description:"Create a persistent activity context that may contain multiple goals."},
      {id:"register_activity_goal",level:"goal",description:"Register a bounded goal inside an activity without ending the activity when the goal finishes."},
      {id:"generate_self_goal_from_activity_signal",level:"goal",description:"Create a novel internal goal from a structured unresolved need or opportunity in the active activity."},
      {id:"evaluate_activity_continuation",level:"activity",description:"Decide whether an activity should continue, pause, or end independently of any one goal."}
    ]
  };
}

/* ---------------------------------------------------------
   VERSION / SELF-KNOWLEDGE / INSPECTOR INTEGRATION
   --------------------------------------------------------- */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA555_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const upsert=(id,entry)=>{const old=V41_SELF_MODEL.capabilities.find(c=>c.id===id);if(old)Object.assign(old,entry,{id});else V41_SELF_MODEL.capabilities.push({id,...entry});};
    upsert("hierarchical_goal_activity_model",{verb:"distinguish",object:"cognitive operations, bounded goals, and open-ended activities",detail:"Completing one goal does not automatically terminate the containing activity; an activity can retain context and continue through additional goals."});
    upsert("self_generated_activity_goals",{verb:"form",object:"bounded internal goals from meaningful unresolved activity signals",detail:"Self-generated goals retain the structured reason they were formed, reject recent duplicates, and remain internal intentions unless the current mind-state envelope and trusted host authorize an external action."});
    upsert("activity_continuation_state",{verb:"represent",object:"whether an activity should continue, pause, or end independently of a subgoal",detail:"Open activities can continue after a subgoal succeeds, pause when no meaningful next goal exists or an external condition is required, and end explicitly without implying system-wide shutdown."});
  }
}catch(_){ }


try{v555ActivityStore();}catch(_){ }
console.info("Nova V6.2.16 activity and goal state loaded.");
