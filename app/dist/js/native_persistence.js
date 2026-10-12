"use strict";

/* =========================================================
   Nova V7 alpha 6.1 — DURABLE PERSISTENCE REPAIR

   One persistence system with runtime-selected durable backends.

   Browser / localhost:
   - IndexedDB is the preferred durable store when available;
   - current + backup records are retained for recovery;
   - localStorage is only a synchronous cache/fallback;
   - a localStorage quota failure never blocks IndexedDB saving.

   Tauri:
   - trusted Rust app-data nova-state.json + backup is authoritative;
   - the stable Tauri identity com.qtech.nova owns the one native state directory;
   - the retired V7 custom QTech\Nova\UserData location is migration input only;
   - localStorage is only a WebView cache/recovery source;
   - a cache quota failure never blocks native saving.

   Startup compares valid cache and durable records by their own savedAt value.
   The newest valid state wins. This prevents a delayed durable write from making
   an older native/IndexedDB snapshot overwrite a newer synchronous cache after a
   quick close/restart.

   Learned cognition never receives direct persistence/filesystem operations.
   Storage remains application infrastructure outside Nova's cognitive VM.
   ========================================================= */

const NOVA620P_PERSISTENCE_SCHEMA=5;
const NOVA620P_BROWSER_DB_NAME="nova-persistent-state";
const NOVA620P_BROWSER_DB_VERSION=1;
const NOVA620P_BROWSER_STORE="state";
const NOVA620P_BROWSER_CURRENT="current";
const NOVA620P_BROWSER_BACKUP="backup";
let nova620pWriteChain=Promise.resolve();
let nova620pBrowserDbPromise=null;

const nova620pPersistence={
  schema:NOVA620P_PERSISTENCE_SCHEMA,
  mode:"uninitialized",
  backend:"none",
  nativeAvailable:false,
  indexedDbAvailable:false,
  localCacheAvailable:true,
  hydrated:false,
  hydrationSource:null,
  migratedLocalToDurable:false,
  recoveredFromBackup:false,
  lastLoadedAt:null,
  lastSavedAt:null,
  storagePath:null,
  stateFile:null,
  backupFile:null,
  nativeStateExists:false,
  nativeBackupExists:false,
  nativeStateBytes:0,
  nativeBackupBytes:0,
  lastNativeReceipt:null,
  lastCacheError:null,
  lastError:null,
  saveTimer:null,
  pendingJson:null,
  latestJson:null,
  lastCheckpointReason:null,
  lastCheckpointAt:null,
  migrationComplete:false,
  writesSuspended:false
};

function nova620pTauriInvokeAvailable(){return Boolean(window.__TAURI__?.core?.invoke);}
function nova620pIndexedDbAvailable(){return typeof indexedDB!=="undefined"&&indexedDB&&typeof indexedDB.open==="function";}
async function nova620pPersistenceInvoke(command,args={}){if(!nova620pTauriInvokeAvailable())throw new Error("Tauri persistence bridge unavailable");return window.__TAURI__.core.invoke(command,args);}
function nova620pValidStateJson(raw){if(typeof raw!=="string"||!raw.trim())return false;try{const parsed=JSON.parse(raw);return Boolean(parsed&&typeof parsed==="object"&&(Array.isArray(parsed.messages)||parsed.context));}catch(_){return false;}}
function nova620pStateSavedAt(raw){if(!nova620pValidStateJson(raw))return 0;try{const parsed=JSON.parse(raw);const value=Number(parsed?.persistence?.savedAt||0);return Number.isFinite(value)&&value>0?value:0;}catch(_){return 0;}}
function nova620pEnvelope(key,payload){return {key,schema:NOVA620P_PERSISTENCE_SCHEMA,payload,savedAt:nova620pStateSavedAt(payload)||Date.now(),bytes:payload.length};}
function nova620pNewestState(localRaw,durableRaw){
  const local=nova620pValidStateJson(localRaw)?localRaw:null;
  const durable=nova620pValidStateJson(durableRaw)?durableRaw:null;
  if(!local&&!durable)return {raw:null,source:null,localSavedAt:0,durableSavedAt:0};
  if(local&&!durable)return {raw:local,source:"local_cache",localSavedAt:nova620pStateSavedAt(local),durableSavedAt:0};
  if(durable&&!local)return {raw:durable,source:"durable",localSavedAt:0,durableSavedAt:nova620pStateSavedAt(durable)};
  const localSavedAt=nova620pStateSavedAt(local),durableSavedAt=nova620pStateSavedAt(durable);
  // On ties prefer durable storage. A strictly newer cache means the durable
  // write likely had not completed before the previous process closed.
  if(localSavedAt>durableSavedAt)return {raw:local,source:"local_cache",localSavedAt,durableSavedAt};
  return {raw:durable,source:"durable",localSavedAt,durableSavedAt};
}
function nova620pReadLocalCache(){try{const raw=localStorage.getItem(APP_CONFIG.storageKey);nova620pPersistence.localCacheAvailable=true;return nova620pValidStateJson(raw)?raw:null;}catch(error){nova620pPersistence.localCacheAvailable=false;nova620pPersistence.lastCacheError=String(error?.message||error);return null;}}
function nova620pWriteLocalCache(raw){if(!nova620pValidStateJson(raw))return false;try{localStorage.setItem(APP_CONFIG.storageKey,raw);nova620pPersistence.localCacheAvailable=true;nova620pPersistence.lastCacheError=null;return true;}catch(error){nova620pPersistence.localCacheAvailable=false;nova620pPersistence.lastCacheError=String(error?.message||error);return false;}}
function nova620pClearLocalCache(){try{localStorage.removeItem(APP_CONFIG.storageKey);nova620pPersistence.localCacheAvailable=true;nova620pPersistence.lastCacheError=null;return true;}catch(error){nova620pPersistence.localCacheAvailable=false;nova620pPersistence.lastCacheError=String(error?.message||error);return false;}}
function nova620CachePersistentState(raw){if(nova620pPersistence.writesSuspended){if(nova620pValidStateJson(raw)){nova620pPersistence.latestJson=raw;nova620pPersistence.pendingJson=raw;return true;}return false;}return nova620pWriteLocalCache(raw);}

function nova620pOpenBrowserDb(){
  if(!nova620pIndexedDbAvailable())return Promise.resolve(null);
  if(nova620pBrowserDbPromise)return nova620pBrowserDbPromise;
  nova620pBrowserDbPromise=new Promise(resolve=>{
    try{
      const request=indexedDB.open(NOVA620P_BROWSER_DB_NAME,NOVA620P_BROWSER_DB_VERSION);
      request.onupgradeneeded=()=>{const db=request.result;if(!db.objectStoreNames.contains(NOVA620P_BROWSER_STORE))db.createObjectStore(NOVA620P_BROWSER_STORE,{keyPath:"key"});};
      request.onsuccess=()=>{nova620pPersistence.indexedDbAvailable=true;resolve(request.result);};
      request.onerror=()=>{nova620pPersistence.lastError=String(request.error?.message||request.error||"IndexedDB open failed");resolve(null);};
      request.onblocked=()=>{nova620pPersistence.lastError="IndexedDB upgrade was blocked";resolve(null);};
    }catch(error){nova620pPersistence.lastError=String(error?.message||error);resolve(null);}
  });
  return nova620pBrowserDbPromise;
}

async function nova620pBrowserGet(key){
  const db=await nova620pOpenBrowserDb();if(!db)return null;
  return new Promise(resolve=>{
    try{const tx=db.transaction(NOVA620P_BROWSER_STORE,"readonly");const req=tx.objectStore(NOVA620P_BROWSER_STORE).get(key);req.onsuccess=()=>resolve(req.result||null);req.onerror=()=>resolve(null);}catch(_){resolve(null);}
  });
}
async function nova620pBrowserPut(record){
  const db=await nova620pOpenBrowserDb();if(!db)return false;
  return new Promise(resolve=>{
    try{const tx=db.transaction(NOVA620P_BROWSER_STORE,"readwrite");tx.objectStore(NOVA620P_BROWSER_STORE).put(record);tx.oncomplete=()=>resolve(true);tx.onerror=()=>resolve(false);tx.onabort=()=>resolve(false);}catch(_){resolve(false);}
  });
}
async function nova620pBrowserDelete(keys){
  const db=await nova620pOpenBrowserDb();if(!db)return false;
  return new Promise(resolve=>{
    try{const tx=db.transaction(NOVA620P_BROWSER_STORE,"readwrite");const store=tx.objectStore(NOVA620P_BROWSER_STORE);for(const key of keys)store.delete(key);tx.oncomplete=()=>resolve(true);tx.onerror=()=>resolve(false);tx.onabort=()=>resolve(false);}catch(_){resolve(false);}
  });
}

async function nova620pLoadBrowserState(){
  if(!nova620pIndexedDbAvailable())return null;
  const current=await nova620pBrowserGet(NOVA620P_BROWSER_CURRENT);
  if(current&&nova620pValidStateJson(current.payload)){nova620pPersistence.lastLoadedAt=Date.now();nova620pPersistence.lastError=null;return {raw:current.payload,fromBackup:false};}
  const backup=await nova620pBrowserGet(NOVA620P_BROWSER_BACKUP);
  if(backup&&nova620pValidStateJson(backup.payload)){nova620pPersistence.lastLoadedAt=Date.now();nova620pPersistence.recoveredFromBackup=true;nova620pPersistence.lastError=null;return {raw:backup.payload,fromBackup:true};}
  return null;
}
async function nova620pWriteBrowserState(raw){
  if(!nova620pValidStateJson(raw))return false;
  const db=await nova620pOpenBrowserDb();if(!db)return false;
  try{
    const current=await nova620pBrowserGet(NOVA620P_BROWSER_CURRENT);
    if(current&&nova620pValidStateJson(current.payload)&&current.payload!==raw)await nova620pBrowserPut(nova620pEnvelope(NOVA620P_BROWSER_BACKUP,current.payload));
    const ok=await nova620pBrowserPut(nova620pEnvelope(NOVA620P_BROWSER_CURRENT,raw));
    if(ok){nova620pPersistence.indexedDbAvailable=true;nova620pPersistence.mode="browser_indexeddb";nova620pPersistence.backend="indexeddb";nova620pPersistence.lastSavedAt=Date.now();nova620pPersistence.lastError=null;}
    return ok;
  }catch(error){nova620pPersistence.lastError=String(error?.message||error);return false;}
}

async function nova620pRefreshNativeDiagnostics(){
  if(nova620pTauriInvokeAvailable())throw new Error("Pass 9 retired raw frontend persistence diagnostics. Use Rust NovaMind bootstrap/checkpoint receipts.");
  return null;
}
async function nova620pLoadNativeLocation(){
  if(nova620pTauriInvokeAvailable())throw new Error("Pass 9 retired raw frontend persistence location access. Rust owns the persistent mind path.");
  return null;
}
async function nova620pLoadNativeState(){
  if(nova620pTauriInvokeAvailable())throw new Error("Pass 9 retired raw frontend state loading. Rust bootstraps NovaMind directly.");
  return null;
}
async function nova620pWriteNativeState(){
  if(nova620pTauriInvokeAvailable())throw new Error("Pass 9 retired raw frontend state writes. Use nova_mind_checkpoint.");
  return false;
}

function nova620pWriteDurableState(raw){
  if(nova620pTauriInvokeAvailable())return Promise.reject(new Error("Pass 9 retired JavaScript durable-mind writes on native Nova."));
  if(!nova620pValidStateJson(raw))return Promise.resolve(false);
  nova620pPersistence.latestJson=raw;
  const write=async()=>{
    if(nova620pIndexedDbAvailable()){
      const ok=await nova620pWriteBrowserState(raw);if(ok)return true;
    }
    const cached=nova620pWriteLocalCache(raw);
    if(cached){nova620pPersistence.mode="browser_local_storage_fallback";nova620pPersistence.backend="localStorage";nova620pPersistence.lastSavedAt=Date.now();nova620pPersistence.lastError=null;return true;}
    nova620pPersistence.lastError=nova620pPersistence.lastCacheError||"No durable browser persistence backend accepted Nova state.";
    return false;
  };
  nova620pWriteChain=nova620pWriteChain.catch(()=>false).then(write);return nova620pWriteChain;
}

async function nova9WriteRustMind(uiState={},reason="explicit_checkpoint"){
  try{
    const receipt=await nova620pPersistenceInvoke("nova_mind_checkpoint",{uiState:uiState&&typeof uiState==="object"?uiState:{},reason:String(reason||"explicit_checkpoint")});
    nova620pPersistence.nativeAvailable=true;nova620pPersistence.mode="tauri_rust_mind";nova620pPersistence.backend="rust";
    nova620pPersistence.lastNativeReceipt=receipt||null;nova620pPersistence.lastSavedAt=Date.now();nova620pPersistence.lastCheckpointReason=String(reason||"explicit_checkpoint");nova620pPersistence.lastCheckpointAt=Date.now();nova620pPersistence.lastError=null;
    if(receipt?.diagnostics&&typeof receipt.diagnostics==="object"){
      const info=receipt.diagnostics;nova620pPersistence.storagePath=info.directory||nova620pPersistence.storagePath;nova620pPersistence.stateFile=info.stateFile||nova620pPersistence.stateFile;nova620pPersistence.backupFile=info.backupFile||nova620pPersistence.backupFile;nova620pPersistence.nativeStateExists=Boolean(info.stateExists);nova620pPersistence.nativeBackupExists=Boolean(info.backupExists);nova620pPersistence.nativeStateBytes=Number(info.stateBytes||0);nova620pPersistence.nativeBackupBytes=Number(info.backupBytes||0);nova620pPersistence.migrationComplete=Boolean(info.migrationComplete);
    }
    if(typeof nova7RustMindBridge==="object"&&receipt?.snapshot)nova7RustMindBridge.snapshot=nova7RustMindClone(receipt.snapshot);
    return true;
  }catch(error){nova620pPersistence.lastError=String(error?.message||error);return false;}
}
async function nova9CommitRustMind(uiState={},reason="explicit_checkpoint"){
  if(!nova620pTauriInvokeAvailable()||nova620pPersistence.writesSuspended)return false;
  nova620pWriteChain=nova620pWriteChain.catch(()=>false).then(()=>nova9WriteRustMind(uiState,reason));
  return Boolean(await nova620pWriteChain);
}
function nova9QueueRustMindCheckpoint(uiState={}){
  if(!nova620pTauriInvokeAvailable())return false;
  if(nova620pPersistence.writesSuspended)return true;
  nova620pWriteChain=nova620pWriteChain.catch(()=>false).then(()=>nova9WriteRustMind(uiState,"queued_ui_checkpoint"));
  return true;
}

async function nova620CommitPersistentState(raw,reason="explicit_checkpoint"){
  if(nova620pPersistence.writesSuspended)return false;
  if(!nova620pValidStateJson(raw))return false;
  nova620pPersistence.latestJson=raw;
  nova620pPersistence.pendingJson=null;
  if(nova620pPersistence.saveTimer){clearTimeout(nova620pPersistence.saveTimer);nova620pPersistence.saveTimer=null;}
  const ok=await nova620pWriteDurableState(raw);
  if(ok){nova620pPersistence.lastCheckpointReason=String(reason||"explicit_checkpoint");nova620pPersistence.lastCheckpointAt=Date.now();}
  return ok;
}

function nova620SetPersistentWritesSuspended(value){
  nova620pPersistence.writesSuspended=Boolean(value);
  return nova620pPersistence.writesSuspended;
}

function nova620QueuePersistentStateSave(raw){
  if(!nova620pValidStateJson(raw))return false;
  nova620pPersistence.latestJson=raw;
  if(nova620pPersistence.writesSuspended){
    nova620pPersistence.pendingJson=raw;
    return true;
  }

  // Native Nova writes are dispatched immediately into the serialized durable
  // write chain. Do not leave the only current mind snapshot sitting behind a
  // browser timer that can disappear when the desktop process closes.
  if(nova620pTauriInvokeAvailable()){
    nova620pPersistence.pendingJson=null;
    nova620pWriteDurableState(raw).catch(()=>false);
    return true;
  }

  // Browser mode keeps a short debounce to avoid excessive IndexedDB writes.
  nova620pPersistence.pendingJson=raw;
  if(nova620pPersistence.saveTimer)return true;
  nova620pPersistence.saveTimer=setTimeout(async()=>{
    const payload=nova620pPersistence.pendingJson;nova620pPersistence.pendingJson=null;nova620pPersistence.saveTimer=null;
    if(payload)await nova620pWriteDurableState(payload);
    if(nova620pPersistence.pendingJson)nova620QueuePersistentStateSave(nova620pPersistence.pendingJson);
  },80);
  return true;
}
async function nova620FlushPersistentState(){
  if(nova620pPersistence.writesSuspended)return false;
  if(nova620pPersistence.saveTimer){clearTimeout(nova620pPersistence.saveTimer);nova620pPersistence.saveTimer=null;}
  const payload=nova620pPersistence.pendingJson||nova620pPersistence.latestJson||nova620pReadLocalCache();
  nova620pPersistence.pendingJson=null;
  return payload?nova620pWriteDurableState(payload):false;
}
async function nova620ClearPersistentState(){
  let ok=nova620pClearLocalCache();
  nova620pPersistence.pendingJson=null;nova620pPersistence.latestJson=null;
  if(nova620pTauriInvokeAvailable()){
    try{await nova620pWriteChain.catch(()=>false);const receipt=await nova620pPersistenceInvoke("nova_mind_reset",{uiState:{}});if(typeof nova7RustMindBridge==="object"&&receipt?.snapshot)nova7RustMindBridge.snapshot=nova7RustMindClone(receipt.snapshot);}catch(error){nova620pPersistence.lastError=String(error?.message||error);ok=false;}
  }
  if(nova620pIndexedDbAvailable()){
    const cleared=await nova620pBrowserDelete([NOVA620P_BROWSER_CURRENT,NOVA620P_BROWSER_BACKUP]);if(!cleared)ok=false;
  }
  nova620pPersistence.lastSavedAt=Date.now();return ok;
}

async function nova620BootstrapPersistentState(){
  nova620pPersistence.hydrated=false;nova620pPersistence.recoveredFromBackup=false;nova620pPersistence.hydrationSource=null;

  if(nova620pTauriInvokeAvailable()){
    nova620pPersistence.nativeAvailable=true;nova620pPersistence.mode="tauri_rust_mind";nova620pPersistence.backend="rust";
    try{
      const receipt=await nova620pPersistenceInvoke("nova_mind_bootstrap");
      nova620pPersistence.hydrationSource=receipt?.source||"rust_pass9";nova620pPersistence.migratedLocalToDurable=Boolean(receipt?.migratedLegacy);nova620pPersistence.lastLoadedAt=Date.now();nova620pPersistence.lastError=null;
      if(receipt?.diagnostics&&typeof receipt.diagnostics==="object"){
        const info=receipt.diagnostics;nova620pPersistence.storagePath=info.directory||null;nova620pPersistence.stateFile=info.stateFile||null;nova620pPersistence.backupFile=info.backupFile||null;nova620pPersistence.nativeStateExists=Boolean(info.stateExists);nova620pPersistence.nativeBackupExists=Boolean(info.backupExists);nova620pPersistence.nativeStateBytes=Number(info.stateBytes||0);nova620pPersistence.nativeBackupBytes=Number(info.backupBytes||0);nova620pPersistence.migrationComplete=Boolean(info.migrationComplete);
      }
      if(typeof nova7RustMindBridge==="object"&&receipt?.snapshot)nova7RustMindBridge.snapshot=nova7RustMindClone(receipt.snapshot);
      nova620pPersistence.hydrated=true;
      return {mode:nova620pPersistence.mode,backend:nova620pPersistence.backend,uiState:receipt?.uiState||{},snapshot:receipt?.snapshot||null,loaded:Boolean(receipt?.restored),source:nova620pPersistence.hydrationSource,migrated:Boolean(receipt?.migratedLegacy),recoveredFromBackup:false};
    }catch(error){nova620pPersistence.lastError=String(error?.message||error);throw error;}
  }

  const localRaw=nova620pReadLocalCache();
  if(nova620pIndexedDbAvailable()){
    nova620pPersistence.mode="browser_indexeddb";nova620pPersistence.backend="indexeddb";
    const durable=await nova620pLoadBrowserState();
    const durableRaw=durable?.raw||null;
    const chosen=nova620pNewestState(localRaw,durableRaw);
    let migrated=false;
    if(chosen.raw){
      nova620pPersistence.latestJson=chosen.raw;
      nova620pPersistence.hydrationSource=chosen.source==="local_cache"?"local_cache_recovery":(durable?.fromBackup?"indexeddb_backup":"indexeddb");
      if(chosen.source==="local_cache"){migrated=await nova620pWriteBrowserState(chosen.raw);nova620pPersistence.migratedLocalToDurable=migrated;}else{nova620pWriteLocalCache(chosen.raw);}
      nova620pPersistence.lastLoadedAt=Date.now();
    }
    nova620pPersistence.hydrated=true;
    return {mode:nova620pPersistence.mode,backend:nova620pPersistence.backend,raw:chosen.raw,loaded:Boolean(chosen.raw),source:nova620pPersistence.hydrationSource,migrated,recoveredFromBackup:Boolean(durable?.fromBackup),localSavedAt:chosen.localSavedAt,durableSavedAt:chosen.durableSavedAt};
  }
  nova620pPersistence.mode="browser_local_storage_fallback";nova620pPersistence.backend="localStorage";nova620pPersistence.hydrationSource=localRaw?"localStorage":null;nova620pPersistence.latestJson=localRaw;nova620pPersistence.hydrated=true;
  return {mode:nova620pPersistence.mode,backend:nova620pPersistence.backend,raw:localRaw,loaded:Boolean(localRaw),source:nova620pPersistence.hydrationSource,migrated:false,recoveredFromBackup:false,localSavedAt:nova620pStateSavedAt(localRaw),durableSavedAt:0};
}

function nova620PersistenceStatus(){return {...nova620pPersistence,saveTimer:Boolean(nova620pPersistence.saveTimer),pendingJson:Boolean(nova620pPersistence.pendingJson),latestJson:Boolean(nova620pPersistence.latestJson)};}

window.addEventListener("pagehide",()=>{if(nova620pTauriInvokeAvailable()&&typeof novaBuildUiStateSnapshot==="function")nova9CommitRustMind(novaBuildUiStateSnapshot(),"pagehide").catch(()=>{});else nova620FlushPersistentState().catch(()=>{});});
document.addEventListener("visibilitychange",()=>{if(document.visibilityState==="hidden"){if(nova620pTauriInvokeAvailable()&&typeof novaBuildUiStateSnapshot==="function")nova9CommitRustMind(novaBuildUiStateSnapshot(),"visibility_hidden").catch(()=>{});else nova620FlushPersistentState().catch(()=>{});}});

function nova620PostLoadStateMaintenance(){
  const report={cleanup:null,synthesis:null,ranAt:Date.now()};
  try{if(typeof v521RemoveTeachingEvidenceCognition==="function")report.cleanup=v521RemoveTeachingEvidenceCognition();}catch(error){report.cleanup={error:String(error?.message||error)};}
  try{
    if(window.__TAURI__?.core?.invoke)report.synthesis={pendingRustStartupSync:true,authority:"rust_v7_pass_9",reason:"browser-only maintenance; native hydration is Rust-owned"};
    else if(typeof v520GeneralSynthesisPass==="function")report.synthesis=v520GeneralSynthesisPass({reason:"browser_post_persistent_hydration",force:true});
  }catch(error){report.synthesis={error:String(error?.message||error)};}
  // Do not persist here. Rust semantic/learning authority has not been restored
  // yet. initializeApp performs one awaited checkpoint after Rust hydration.
  return report;
}

try{
  if(typeof V41_SELF_MODEL==="object"&&V41_SELF_MODEL){
    V41_SELF_MODEL.capabilities ||= [];
    const add=c=>{if(!V41_SELF_MODEL.capabilities.some(x=>x.id===c.id))V41_SELF_MODEL.capabilities.push(c);};
    add({id:"durable_native_memory",verb:"persist",object:"learned state across native application updates",detail:"In Tauri, the trusted application layer stores my serialized state in Tauri's stable application-data directory for com.qtech.nova, with a verified backup. Alpha 6.1 restored the proven V5 app-data authority; newer valid snapshots from the retired V7 custom folder are migrated forward once instead of remaining a second persistence system."});
    add({id:"durable_browser_memory",verb:"persist",object:"learned state across ordinary browser/local-server sessions",detail:"Outside Tauri, the application stores my complete serialized state in IndexedDB with a backup when the browser permits it. localStorage is only a synchronous cache/fallback and may be smaller than my learned state."});
  }
}catch(_){ }

console.info("Nova V7 alpha 6.1 durable persistence repair loaded.");
