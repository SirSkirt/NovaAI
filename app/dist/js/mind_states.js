"use strict";

/* =========================================================
   Nova V6.2.16 — MIND STATES + CONTEXTUAL AUTONOMY ENVELOPES

   Current contextual mind-state representation for the V6.2 mind.

   Nova has an explicit representation for HOW MUCH initiative
   is appropriate in a cognitive context, separately from WHAT external
   resources that context could eventually use.

   Core distinction:
     internal autonomy  — how freely Nova may form/continue internal goals;
     external envelope  — what kind of outside action scope the context may
                          request from the trusted host boundary.

   A broad internal mind never implies broad computer authority. A narrow
   task may have a broad read scope while still allowing very little
   self-directed continuation. These are orthogonal dimensions.

   V6.2 BOUNDARY:
   - this is persistent, inspectable cognitive state;
   - a cognitive envelope never manufactures keyboard/mouse/web/file/app authority;
   - runtime host grants may be attached only from the trusted Tauri boundary;
   - no external grant is persisted or restored;
   - internal initiative and external authority remain orthogonal.
   ========================================================= */

const NOVA556_VERSION="6.2.16";
const NOVA556_SCHEMA=1;
const NOVA556_MIGRATION_SCHEMA=1;
const NOVA556_STATE_LIMIT=160;
const NOVA556_HISTORY_LIMIT=260;

const NOVA556_STATE_STATUS=Object.freeze({
  ACTIVE:"ACTIVE",
  PAUSED:"PAUSED",
  ENDED:"ENDED"
});

const NOVA556_PROFILE=Object.freeze({
  IDLE:"IDLE",
  CONVERSATION:"CONVERSATION",
  LEARNING_REFLECTION:"LEARNING_REFLECTION",
  FOCUSED_TASK:"FOCUSED_TASK",
  BOUNDED_LOOKUP:"BOUNDED_LOOKUP",
  DOCUMENT_WORK:"DOCUMENT_WORK",
  OPEN_INTERACTIVE_ACTIVITY:"OPEN_INTERACTIVE_ACTIVITY",
  CONTROLLED_APP_ACTIVITY:"CONTROLLED_APP_ACTIVITY"
});

const NOVA556_INITIATIVE=Object.freeze({
  NONE:"NONE",
  REACTIVE:"REACTIVE",
  BOUNDED:"BOUNDED",
  EXPANSIVE:"EXPANSIVE"
});

const NOVA556_SELF_GOALS=Object.freeze({
  NONE:"NONE",
  FOLLOW_UP_ONLY:"FOLLOW_UP_ONLY",
  WHEN_USEFUL:"WHEN_USEFUL",
  WITHIN_ACTIVITY:"WITHIN_ACTIVITY"
});

function v556Clone(v){try{return structuredClone(v);}catch(_){try{return JSON.parse(JSON.stringify(v));}catch(__){return v;}}}
function v556Clean(v){try{return normalizeWhitespace(String(v??""));}catch(_){return String(v??"").replace(/\s+/g," ").trim();}}
function v556Key(v){return v556Clean(v).toUpperCase().replace(/[^A-Z0-9]+/g,"_").replace(/^_+|_+$/g,"");}
function v556Now(){return Date.now();}
function v556Id(prefix){return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2,8)}`;}

const NOVA556_PROFILES=Object.freeze({
  [NOVA556_PROFILE.IDLE]:Object.freeze({
    id:NOVA556_PROFILE.IDLE,
    label:"Idle",
    description:"No externally directed task or open activity is currently being pursued.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.NONE,selfGeneratedGoals:NOVA556_SELF_GOALS.NONE,continuation:"NONE",reflection:"LOW",mayRevisitInternalState:false}),
    external:Object.freeze({scopeBreadth:"NONE",targetBinding:"NONE",capabilities:[],activation:"NONE",requiresExplicitGrant:true,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.CONVERSATION]:Object.freeze({
    id:NOVA556_PROFILE.CONVERSATION,
    label:"Conversation",
    description:"Reactive conversation with modest internal follow-up and no implied external-action scope.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.BOUNDED,selfGeneratedGoals:NOVA556_SELF_GOALS.FOLLOW_UP_ONLY,continuation:"TURN",reflection:"MEDIUM",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"NONE",targetBinding:"NONE",capabilities:[],activation:"NONE",requiresExplicitGrant:true,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.LEARNING_REFLECTION]:Object.freeze({
    id:NOVA556_PROFILE.LEARNING_REFLECTION,
    label:"Learning / reflection",
    description:"High internal autonomy for integrating, self-testing, connecting, and questioning newly learned material without external-action authority.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.EXPANSIVE,selfGeneratedGoals:NOVA556_SELF_GOALS.WHEN_USEFUL,continuation:"BOUNDED_COGNITIVE_SESSION",reflection:"HIGH",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"NONE",targetBinding:"NONE",capabilities:[],activation:"NONE",requiresExplicitGrant:true,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.FOCUSED_TASK]:Object.freeze({
    id:NOVA556_PROFILE.FOCUSED_TASK,
    label:"Focused task",
    description:"A bounded task where Nova may reason and create only directly useful follow-up goals before returning a result.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.BOUNDED,selfGeneratedGoals:NOVA556_SELF_GOALS.FOLLOW_UP_ONLY,continuation:"TASK",reflection:"MEDIUM",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"TASK_DEFINED",targetBinding:"TASK_DEFINED",capabilities:[],activation:"TASK_REQUEST",requiresExplicitGrant:true,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.BOUNDED_LOOKUP]:Object.freeze({
    id:NOVA556_PROFILE.BOUNDED_LOOKUP,
    label:"Bounded lookup",
    description:"Low-autonomy research: the resource domain may be broad, but initiative remains tightly bound to the requested lookup.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.REACTIVE,selfGeneratedGoals:NOVA556_SELF_GOALS.FOLLOW_UP_ONLY,continuation:"TASK",reflection:"LOW",mayRevisitInternalState:false}),
    external:Object.freeze({scopeBreadth:"BROAD_READ_DOMAIN",targetBinding:"RESOURCE_DOMAIN",capabilities:["web.search","web.read"],activation:"USER_TASK",requiresExplicitGrant:false,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.DOCUMENT_WORK]:Object.freeze({
    id:NOVA556_PROFILE.DOCUMENT_WORK,
    label:"Document work",
    description:"Work centered on explicitly supplied documents with moderate internal autonomy and no authority outside the document set.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.BOUNDED,selfGeneratedGoals:NOVA556_SELF_GOALS.WHEN_USEFUL,continuation:"TASK_OR_LEARNING_EPISODE",reflection:"HIGH",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"SUPPLIED_RESOURCES",targetBinding:"DOCUMENT_SET",capabilities:["document.read"],activation:"DOCUMENT_SELECTION",requiresExplicitGrant:false,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY]:Object.freeze({
    id:NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY,
    label:"Open interactive activity",
    description:"High internal autonomy inside a continuing interactive session, paired with a deliberately narrow single-target external scope.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.EXPANSIVE,selfGeneratedGoals:NOVA556_SELF_GOALS.WITHIN_ACTIVITY,continuation:"SESSION",reflection:"HIGH",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"SINGLE_TARGET",targetBinding:"ONE_APPROVED_TARGET",capabilities:["approved_target.observe","approved_target.input"],activation:"SESSION_GRANT",requiresExplicitGrant:true,releaseOnExit:true})
  }),
  [NOVA556_PROFILE.CONTROLLED_APP_ACTIVITY]:Object.freeze({
    id:NOVA556_PROFILE.CONTROLLED_APP_ACTIVITY,
    label:"Controlled app activity",
    description:"Moderate internal autonomy for work in one explicitly approved application; authority never expands to other applications or host capabilities.",
    internal:Object.freeze({initiative:NOVA556_INITIATIVE.BOUNDED,selfGeneratedGoals:NOVA556_SELF_GOALS.WHEN_USEFUL,continuation:"TASK_OR_SESSION",reflection:"MEDIUM",mayRevisitInternalState:true}),
    external:Object.freeze({scopeBreadth:"SINGLE_TARGET",targetBinding:"ONE_APPROVED_TARGET",capabilities:["approved_target.observe","approved_target.input"],activation:"SESSION_GRANT",requiresExplicitGrant:true,releaseOnExit:true})
  })
});

function v556Profile(profileId){return NOVA556_PROFILES[profileId]||NOVA556_PROFILES[NOVA556_PROFILE.FOCUSED_TASK];}

function v556MindStore(){
  const memory=v43Memory();
  if(!memory.mindStateCognition||memory.mindStateCognition.schema!==NOVA556_SCHEMA){
    const old=memory.mindStateCognition||{};
    memory.mindStateCognition={
      schema:NOVA556_SCHEMA,
      version:NOVA556_VERSION,
      currentStateId:old.currentStateId||null,
      states:Array.isArray(old.states)?old.states:[],
      history:Array.isArray(old.history)?old.history.slice(-NOVA556_HISTORY_LIMIT):[],
      activityBindings:Array.isArray(old.activityBindings)?old.activityBindings:[],
      migration:old.migration||null,
      createdAt:old.createdAt||v556Now(),
      updatedAt:v556Now()
    };
  }
  const store=memory.mindStateCognition;
  store.version=NOVA556_VERSION;store.states ||= [];store.history ||= [];store.activityBindings ||= [];
  return store;
}

function v556RecordHistory(type,detail={}){
  const store=v556MindStore();store.history.push({id:v556Id("mind_event"),type,...v556Clone(detail),at:v556Now()});store.history=store.history.slice(-NOVA556_HISTORY_LIMIT);store.updatedAt=v556Now();
}

function v556MindStateById(id){return v556MindStore().states.find(s=>s.id===id)||null;}
function v556CurrentMindState(){const store=v556MindStore();return v556MindStateById(store.currentStateId)||null;}

function v556BuildEnvelope(profileId,context={}){
  const profile=v556Profile(profileId);const target=v556Clone(context.target||context.targetBinding||null);
  return {
    profileId:profile.id,
    internal:v556Clone(profile.internal),
    external:{
      ...v556Clone(profile.external),
      target,
      authority:{granted:false,status:"RUNTIME_AUTHORITY_REQUIRED",grantId:null,validatedAt:null},
      invariants:[
        "NO_EXTERNAL_AUTHORITY_BY_DEFAULT",
        "EXTERNAL_SCOPE_DOES_NOT_EXPAND_FROM_INTERNAL_AUTONOMY",
        "TARGET_SCOPE_MUST_REMAIN_BOUND",
        "AUTHORITY_IS_NOT_PERSISTED_ACROSS_RUNTIME_RESTART",
        "USER_OR_TRUSTED_HOST_CAN_REVOKE_AT_ANY_TIME"
      ]
    }
  };
}

function v556CreateMindState(profileId,spec={}){
  const profile=v556Profile(profileId);const store=v556MindStore();
  const existing=v556CurrentMindState();
  if(existing&&existing.status===NOVA556_STATE_STATUS.ACTIVE&&spec.replaceCurrent!==false){
    existing.status=NOVA556_STATE_STATUS.ENDED;existing.endedAt=v556Now();existing.endReason=spec.transitionReason||"REPLACED_BY_NEW_MIND_STATE";existing.updatedAt=v556Now();
    v556RecordHistory("MIND_STATE_ENDED",{mindStateId:existing.id,profileId:existing.profileId,reason:existing.endReason,releaseExternalAuthority:true});
  }
  const state={
    id:v556Id("mind_state"),type:"mind_state",profileId:profile.id,label:profile.label,
    description:v556Clean(spec.description||profile.description),status:NOVA556_STATE_STATUS.ACTIVE,
    activityId:spec.activityId||null,goalId:spec.goalId||null,reason:v556Clean(spec.reason||spec.transitionReason||"context_selected"),
    envelope:v556BuildEnvelope(profile.id,{target:spec.target||null}),
    source:v556Clone(spec.source||null),createdAt:v556Now(),updatedAt:v556Now(),pausedAt:null,endedAt:null,endReason:null
  };
  store.states.push(state);store.states=store.states.slice(-NOVA556_STATE_LIMIT);store.currentStateId=state.id;store.updatedAt=v556Now();
  if(state.activityId){
    store.activityBindings=store.activityBindings.filter(x=>x.activityId!==state.activityId);
    store.activityBindings.push({activityId:state.activityId,mindStateId:state.id,profileId:state.profileId,at:v556Now()});
  }
  v556RecordHistory("MIND_STATE_CREATED",{mindStateId:state.id,profileId:state.profileId,activityId:state.activityId,reason:state.reason});
  try{saveState();}catch(_){ }
  return state;
}

function v556PauseMindState(id,reason="MIND_STATE_PAUSED"){
  const state=v556MindStateById(id);if(!state||state.status===NOVA556_STATE_STATUS.ENDED)return null;
  state.status=NOVA556_STATE_STATUS.PAUSED;state.pausedAt=v556Now();state.updatedAt=v556Now();
  v556RecordHistory("MIND_STATE_PAUSED",{mindStateId:id,profileId:state.profileId,reason,releaseExternalAuthority:true});try{saveState();}catch(_){ }return state;
}
function v556ResumeMindState(id,reason="MIND_STATE_RESUMED"){
  const state=v556MindStateById(id);if(!state||state.status===NOVA556_STATE_STATUS.ENDED)return null;
  // External authority is always scrubbed on resume. V6 must revalidate it.
  state.status=NOVA556_STATE_STATUS.ACTIVE;state.pausedAt=null;state.envelope.external.authority={granted:false,status:"RUNTIME_AUTHORITY_REQUIRED",grantId:null,validatedAt:null};state.updatedAt=v556Now();
  v556MindStore().currentStateId=id;v556RecordHistory("MIND_STATE_RESUMED",{mindStateId:id,profileId:state.profileId,reason});try{saveState();}catch(_){ }return state;
}
function v556EndMindState(id,reason="MIND_STATE_ENDED"){
  const state=v556MindStateById(id);if(!state)return null;
  state.status=NOVA556_STATE_STATUS.ENDED;state.endedAt=v556Now();state.endReason=reason;state.updatedAt=v556Now();state.envelope.external.authority={granted:false,status:"RELEASED",grantId:null,validatedAt:v556Now()};
  const store=v556MindStore();if(store.currentStateId===id)store.currentStateId=null;v556RecordHistory("MIND_STATE_ENDED",{mindStateId:id,profileId:state.profileId,reason,releaseExternalAuthority:true});try{saveState();}catch(_){ }return state;
}

function v556TransitionMindState(profileId,detail={}){
  const before=v556CurrentMindState();const next=v556CreateMindState(profileId,{...detail,replaceCurrent:true,transitionReason:detail.reason||`TRANSITION_TO_${profileId}`});
  return {from:before?v556Clone({id:before.id,profileId:before.profileId,status:before.status}):null,to:v556Clone(next),releasePreviousExternalAuthority:Boolean(before)};
}

function v556SuggestProfileForActivity(activityId,context={}){
  const activity=typeof v555ActivityById==="function"?v555ActivityById(activityId):null;if(!activity)return null;
  if(context.profileId&&NOVA556_PROFILES[context.profileId])return {activityId,profileId:context.profileId,reason:"EXPLICIT_CONTEXT_PROFILE"};
  const interaction=v556Key(context.interactionClass||activity.scope?.interactionClass||"");
  if(["PLAY","GAME","SANDBOX","INTERACTIVE","OPEN_INTERACTIVE"].includes(interaction))return {activityId,profileId:NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY,reason:"OPEN_INTERACTIVE_CONTEXT"};
  if(["APP","CONTROLLED_APP","SINGLE_APP"].includes(interaction))return {activityId,profileId:NOVA556_PROFILE.CONTROLLED_APP_ACTIVITY,reason:"CONTROLLED_SINGLE_TARGET_CONTEXT"};
  if(activity.continuationMode==="OPEN_ENDED")return {activityId,profileId:NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY,reason:"OPEN_ENDED_ACTIVITY_DEFAULT"};
  return {activityId,profileId:NOVA556_PROFILE.FOCUSED_TASK,reason:"TASK_BOUNDED_ACTIVITY_DEFAULT"};
}

function v556BindMindStateToActivity(activityId,context={}){
  const suggestion=v556SuggestProfileForActivity(activityId,context);if(!suggestion)return null;
  return v556CreateMindState(suggestion.profileId,{activityId,reason:suggestion.reason,target:context.target||null,source:{kind:"activity_binding",activityId}});
}

function v556CheckDeclaredExternalScope(mindStateId,request={}){
  const state=v556MindStateById(mindStateId)||v556CurrentMindState();if(!state)return {allowed:false,reason:"NO_MIND_STATE",staged:false};
  const capability=v556Clean(request.capability||"");const allowedCapabilities=state.envelope?.external?.capabilities||[];
  if(!capability)return {allowed:false,reason:"NO_CAPABILITY_REQUESTED",staged:false,mindStateId:state.id};
  if(!allowedCapabilities.includes(capability))return {allowed:false,reason:"CAPABILITY_OUTSIDE_AUTONOMY_ENVELOPE",staged:false,mindStateId:state.id,capability};
  const boundTarget=state.envelope?.external?.target||null;const requestedTarget=request.target||null;
  if(boundTarget&&requestedTarget&&JSON.stringify(boundTarget)!==JSON.stringify(requestedTarget))return {allowed:false,reason:"TARGET_OUTSIDE_AUTONOMY_ENVELOPE",staged:false,mindStateId:state.id,capability};
  return {allowed:false,reason:"HOST_AUTHORIZATION_REQUIRED",staged:false,mindStateId:state.id,capability,withinDeclaredEnvelope:true,requiresExplicitGrant:Boolean(state.envelope?.external?.requiresExplicitGrant)};
}


/* ---------------------------------------------------------
   RUNTIME HOST AUTHORITY — CURRENT MIND, NOT PERSISTED

   The mind state describes what scope is cognitively appropriate. The
   trusted Tauri host remains the only source of an actual grant. Attaching
   a grant never calls saveState(), so runtime authority is not made into
   learned/persistent cognition.
   --------------------------------------------------------- */
function v556RuntimeGrantTarget(grant){
  return grant?{kind:"window",windowId:String(grant.windowId||""),pid:Number(grant.pid||0),processName:grant.processName||"",adapter:grant.adapter||""}:null;
}
function v556EnsureMindStateForRuntimeGrant(grant){
  if(!grant)return null;
  const target=v556RuntimeGrantTarget(grant);
  let current=v556CurrentMindState();
  const canUseApprovedTarget=Boolean(current?.envelope?.external?.capabilities?.some?.(x=>String(x).startsWith("approved_target.")));
  if(!canUseApprovedTarget){
    const activity=(typeof v555ActiveActivities==="function"?v555ActiveActivities():[]).at(-1)||null;
    const profileId=activity?NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY:NOVA556_PROFILE.CONTROLLED_APP_ACTIVITY;
    current=v556TransitionMindState(profileId,{activityId:activity?.id||null,target,reason:"USER_GRANTED_APPROVED_APP",source:{kind:"trusted_host_grant",grantId:grant.grantId||null}}).to;
  }else{
    current.envelope.external.target=target;
    current.updatedAt=v556Now();
  }
  return current;
}
function v556AttachRuntimeAuthority(grant,mindStateId=null){
  if(!grant)return null;
  let current=(mindStateId&&v556MindStateById(mindStateId))||v556EnsureMindStateForRuntimeGrant(grant);
  if(!current)return null;
  const declared=current.envelope?.external?.capabilities||[];
  if(!declared.some(x=>String(x).startsWith("approved_target.")))return null;
  const target=v556RuntimeGrantTarget(grant);
  current.envelope.external.target=target;
  current.envelope.external.authority={granted:true,runtimeValidated:true,status:"HOST_GRANTED_RUNTIME",grantId:grant.grantId||null,validatedAt:v556Now(),hostCapabilities:v556Clone(grant.capabilities||[])};
  current.updatedAt=v556Now();
  v556RecordHistory("HOST_AUTHORITY_ATTACHED",{mindStateId:current.id,grantId:grant.grantId||null,target});
  return current;
}
function v556ClearRuntimeAuthority(reason="HOST_AUTHORITY_RELEASED"){
  const current=v556CurrentMindState();
  if(current?.envelope?.external){
    current.envelope.external.authority={granted:false,runtimeValidated:false,status:reason,grantId:null,validatedAt:v556Now()};
    current.updatedAt=v556Now();
  }
  v556RecordHistory("HOST_AUTHORITY_CLEARED",{mindStateId:current?.id||null,reason});
  return current;
}
function v556CheckRuntimeExternalAction(request={},mindStateId=null){
  const current=(mindStateId&&v556MindStateById(mindStateId))||v556CurrentMindState();
  if(!current)return {allowed:false,reason:"NO_MIND_STATE"};
  const capability=v556Clean(request.capability||"");
  const declared=current.envelope?.external?.capabilities||[];
  if(!capability)return {allowed:false,reason:"NO_CAPABILITY_REQUESTED",mindStateId:current.id};
  if(!declared.includes(capability))return {allowed:false,reason:"CAPABILITY_OUTSIDE_AUTONOMY_ENVELOPE",capability,mindStateId:current.id};
  const bound=current.envelope?.external?.target||null,requested=request.target||null;
  if(bound&&requested&&JSON.stringify(bound)!==JSON.stringify(requested))return {allowed:false,reason:"TARGET_OUTSIDE_AUTONOMY_ENVELOPE",capability,mindStateId:current.id};
  const authority=current.envelope?.external?.authority;
  if(!authority?.granted||!authority?.runtimeValidated)return {allowed:false,reason:"HOST_AUTHORIZATION_REQUIRED",capability,mindStateId:current.id,withinDeclaredEnvelope:true};
  return {allowed:true,reason:"HOST_AUTHORIZED_AND_IN_SCOPE",capability,mindStateId:current.id,grantId:authority.grantId,target:v556Clone(bound),hostMustRevalidateEachAction:true};
}

function v556AutonomyComparison(){
  const lookup=v556Profile(NOVA556_PROFILE.BOUNDED_LOOKUP),interactive=v556Profile(NOVA556_PROFILE.OPEN_INTERACTIVE_ACTIVITY);
  return {
    boundedLookup:{internalInitiative:lookup.internal.initiative,externalScope:lookup.external.scopeBreadth,continuation:lookup.internal.continuation},
    openInteractive:{internalInitiative:interactive.internal.initiative,externalScope:interactive.external.scopeBreadth,continuation:interactive.internal.continuation},
    principle:"Internal autonomy and external scope are independent dimensions: a lookup may read broadly while remaining task-bounded, while an interactive session may think broadly while remaining locked to one approved target."
  };
}

function v556MindStateSummary(){
  const store=v556MindStore(),current=v556CurrentMindState();
  return {
    schema:store.schema,version:NOVA556_VERSION,states:store.states.length,
    activeStates:store.states.filter(s=>s.status===NOVA556_STATE_STATUS.ACTIVE).length,
    pausedStates:store.states.filter(s=>s.status===NOVA556_STATE_STATUS.PAUSED).length,
    endedStates:store.states.filter(s=>s.status===NOVA556_STATE_STATUS.ENDED).length,
    activityBindings:store.activityBindings.length,
    current:current?v556Clone({id:current.id,profileId:current.profileId,label:current.label,status:current.status,activityId:current.activityId,internal:current.envelope?.internal,external:{scopeBreadth:current.envelope?.external?.scopeBreadth,targetBinding:current.envelope?.external?.targetBinding,capabilities:current.envelope?.external?.capabilities,authority:current.envelope?.external?.authority}}):null,
    staged:false,liveExecutiveWiring:true,liveExternalAuthority:true
  };
}

function v556PostLoadMindStateMaintenance(){
  const store=v556MindStore();let scrubbed=0;
  for(const state of store.states||[]){
    if(state?.envelope?.external){
      if(state.envelope.external.authority?.granted) scrubbed++;
      state.envelope.external.authority={granted:false,status:"RESTORED_WITHOUT_AUTHORITY",grantId:null,validatedAt:v556Now()};
    }
  }
  const migration={schema:NOVA556_MIGRATION_SCHEMA,version:NOVA556_VERSION,ranAt:v556Now(),externalAuthorityRecordsScrubbed:scrubbed,standaloneGoalUnification:"DEFERRED_TO_V6",reason:"mind state persists; external authorization never persists"};
  store.migration=v556Clone(migration);store.updatedAt=v556Now();try{saveState();}catch(_){ }return migration;
}

function v556MindStateCognitiveRegistry(){
  return {
    version:NOVA556_VERSION,staged:false,liveExecutiveWiring:true,liveCapabilityWiring:true,
    operations:[
      {id:"select_contextual_mind_state",level:"mind_state",description:"Represent the cognitive mode appropriate to the current context without granting external authority."},
      {id:"construct_contextual_autonomy_envelope",level:"mind_state",description:"Represent internal initiative separately from the external capability scope that may later be authorized."},
      {id:"transition_mind_state",level:"mind_state",description:"Move between contextual cognitive modes while releasing any previous external authority instead of carrying it forward."},
      {id:"evaluate_envelope_scope",level:"authorization_boundary",description:"Check whether a proposed external action even belongs to the declared contextual envelope before trusted authorization is considered."}
    ]
  };
}

/* ---------------------------------------------------------
   VERSION / SELF-KNOWLEDGE / WORKSPACE INTEGRATION
   --------------------------------------------------------- */
try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.identity.version=NOVA556_VERSION;V41_SELF_MODEL.capabilities ||= [];
    const upsert=(id,entry)=>{const old=V41_SELF_MODEL.capabilities.find(c=>c.id===id);if(old)Object.assign(old,entry,{id});else V41_SELF_MODEL.capabilities.push({id,...entry});};
    upsert("contextual_mind_states",{verb:"represent",object:"different cognitive modes with different appropriate degrees of internal initiative",detail:"Conversation, learning/reflection, bounded lookup, document work, focused tasks, open interactive activities, and controlled-app activities can carry different continuation and self-generated-goal policies."});
    upsert("orthogonal_autonomy_envelopes",{verb:"separate",object:"internal cognitive autonomy from external action scope",detail:"High internal autonomy never implies broad computer authority; a broad read domain can still have low initiative, while an open interactive session can have high initiative but remain limited to one approved target."});
    upsert("nonpersistent_external_authority",{verb:"preserve",object:"the rule that external authorization is host-controlled and never restored merely because a mind state persisted",detail:"The intended envelope may persist, but runtime external grants do not. The trusted host must revalidate authority for the current session and target."});
  }
}catch(_){ }


try{v556MindStore();}catch(_){ }
console.info("Nova V6.2.16 mind states and contextual autonomy envelopes loaded.");
