//! Native interface for ONE hybrid Nova brain. Both pathways jointly optimize one loss.
use neural_nova::hybrid::{HybridBrain,TrainingFocus};
use neural_nova::dialogue::dialogue_prefix;
use neural_nova::transformer::VOCAB;
use neural_nova::trainer::{prepare,training_order,Example,PreparedLessons,ScheduleConfig,ScheduleState};
use crate::transformer_lab::Lesson;
use crate::test_lab::{TestArchive,TestDefinition,TestReport,TestOptions,TestCase,QuestionResult,Attempt,parse_cases,grade,validate_options};
use crate::diagnostic_log::{self as diag, Diagnostics};
use crate::rehearsal::{self,LessonRecord,RehearsalBank};
use crate::checkpoint_transfer::TransferState;
use crate::training_observer as observer;
use serde::{Deserialize,Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Mutex};
use std::time::{Instant,SystemTime,UNIX_EPOCH};
use tauri::{AppHandle,Emitter,Manager,State};

const CHECKPOINT:&str="neural-nova-hybrid-0.5.nhyb";
const OLD_CHECKPOINT:&str="neural-nova-hybrid-0.4.nhyb";
const FIRST_CHECKPOINT:&str="neural-nova-hybrid-0.3.nhyb";
const CHAT_FILE:&str="neural-nova-conversation-v0.5.json";
const MEMORY_SLOTS:usize=12;
const HISTORY_LIMIT:usize=64;
const BATCH:usize=8;
const DEFAULT_LEARNING_RATE:f32=0.001;
const MIN_IMPROVEMENT:f32=0.0005;
const AUTONOMOUS_TEACHER_TOLERANCE:f32=0.08; // Teacher scoring only; not an inference rule.

struct Session{
    brain:HybridBrain,trained:bool,
    epoch:u32,epochs:u32,completed:usize,total:usize,
    heldout_loss:Option<f32>,heldout_accuracy:Option<f32>,
    train_loss:Option<f32>,error:Option<String>,
    best_heldout_loss:Option<f32>,best_epoch:u32,learning_rate:f32,
    train_lessons:usize,validation_lessons:usize,
    tokens_per_second:f32,stage:String,early_stopped:bool,
    history:Vec<ConversationTurn>,
    retention_baseline:Option<f32>,retention_accuracy:Option<f32>,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ConversationTurn{user:String,assistant:String}
struct Shared{
    session:Mutex<Session>,
    training:AtomicBool,
    stop:AtomicBool,
    test_paused:AtomicBool,
    tests:Mutex<TestArchive>,
}
#[derive(Clone)]
pub struct HybridState{inner:Arc<Shared>}
impl HybridState{
    pub(crate) fn is_training(&self)->bool{
        self.inner.training.load(Ordering::Acquire)
    }
}
/// Teacher-only outcome used by the native ZIP scheduler after a completed
/// lesson. It never changes Nova's predictions or evaluates answers for her.
pub(crate) fn completed_training_outcome(state:&HybridState)
    ->Result<(u32,Option<String>),String>{
    let session=lock(state)?;
    Ok((session.best_epoch,session.error.clone()))
}

#[cfg(target_os="android")]
fn training_notification(app:&AppHandle,body:String,finished:bool){
    use tauri_plugin_notification::NotificationExt;
    let mut note=app.notification().builder().id(90217).silent()
        .title(if finished{"Nova training finished"}else{"Nova training in progress"})
        .body(body);
    if !finished {note=note.ongoing();}
    let _=note.show();
}
impl Default for HybridState{
    fn default()->Self{
        Self{inner:Arc::new(Shared{
            session:Mutex::new(Session{
                brain:HybridBrain::new(),trained:false,epoch:0,epochs:0,completed:0,total:0,
                heldout_loss:None,heldout_accuracy:None,train_loss:None,error:None,
                best_heldout_loss:None,best_epoch:0,learning_rate:DEFAULT_LEARNING_RATE,
                train_lessons:0,validation_lessons:0,tokens_per_second:0.0,
                stage:"Idle".into(),early_stopped:false,history:Vec::new(),
                retention_baseline:None,retention_accuracy:None
            }),
            training:AtomicBool::new(false),stop:AtomicBool::new(false),
            test_paused:AtomicBool::new(false),tests:Mutex::new(TestArchive::default())
        })}
    }
}
fn lock(state:&HybridState)->Result<std::sync::MutexGuard<'_,Session>,String>{
    state.inner.session.lock().map_err(|_|"Hybrid model lock poisoned".into())
}
fn test_archive_path(app:&AppHandle)->Result<PathBuf,String>{
    Ok(path(app)?.with_file_name("neural-nova-tests-v0.6.json"))
}
fn persist_archive(app:&AppHandle,data:&TestArchive)->Result<(),String>{
    let path=test_archive_path(app)?;
    let tmp=path.with_extension("tmp");
    let serialized=serde_json::to_vec_pretty(data).map_err(|e|e.to_string())?;
    {
        use std::io::Write;
        let mut file=fs::File::create(&tmp).map_err(|e|e.to_string())?;
        file.write_all(&serialized).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
    }
    let backup=path.with_extension("bak");
    if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
    if path.exists(){fs::rename(&path,&backup).map_err(|e|e.to_string())?;}
    match fs::rename(&tmp,&path){
        Ok(())=>{
            if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
            Ok(())
        },
        Err(e)=>{
            if backup.exists(){let _=fs::rename(&backup,&path);}
            Err(e.to_string())
        }
    }
}
fn rehearsal_path(app:&AppHandle)->Result<PathBuf,String>{
    Ok(path(app)?.with_file_name("neural-nova-rehearsal-v0.7.json"))
}
fn rehearsal_for_current(app:&AppHandle,brain:&HybridBrain)->Result<RehearsalBank,String>{
    let file=rehearsal_path(app)?;
    let existing=rehearsal::read(&file)?;
    let current=diag::weight_digest(brain);
    if !existing.lessons.is_empty() && existing.model_fingerprint!=current{
        // A migration or restored checkpoint may invalidate the old replay
        // fingerprint. NEVER replace that archive without preserving its bytes.
        let suffix=existing.model_fingerprint.chars()
            .filter(|c|c.is_ascii_hexdigit()).take(16).collect::<String>();
        let suffix=if suffix.is_empty(){"unknown".to_string()}else{suffix};
        let stem=format!("neural-nova-rehearsal-unmatched-{suffix}");
        let data=fs::read(&file).map_err(|e|e.to_string())?;
        let mut backed_up=false;
        for n in 0..100{
            let candidate=file.with_file_name(format!("{stem}-{n}.json"));
            if candidate.exists(){
                if fs::read(&candidate).map_err(|e|e.to_string())?==data{
                    backed_up=true;break;
                }
            }else{
                fs::write(&candidate,&data).map_err(|e|e.to_string())?;
                backed_up=true;break;
            }
        }
        if !backed_up{
            return Err("Cannot back up mismatched rehearsal archive; no training started".into());
        }
    }
    Ok(rehearsal::matched(existing,&current))
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct RehearsalInfo{names:Vec<String>,modes:Vec<String>,matched:bool,
    foundation_lessons_completed:u8,foundation_lessons_remaining:u8}
#[tauri::command]
pub fn hybrid_rehearsal_info(app:AppHandle,state:State<'_,HybridState>)->Result<RehearsalInfo,String>{
    let s=lock(&state)?;
    let file=rehearsal::read(&rehearsal_path(&app)?)?;
    let matched=file.model_fingerprint==diag::weight_digest(&s.brain);
    let bank=rehearsal::matched(file,&diag::weight_digest(&s.brain));
    Ok(RehearsalInfo{names:bank.lessons.iter().map(|l|l.name.clone()).collect(),
        modes:bank.lessons.iter().map(|l|l.mode.clone()).collect(),matched,
        foundation_lessons_completed:bank.foundation_lessons_completed,
        foundation_lessons_remaining:rehearsal::FOUNDATION_LESSON_LIMIT.saturating_sub(bank.foundation_lessons_completed)})
}
#[tauri::command]
pub fn hybrid_rehearsal_add(app:AppHandle,state:State<'_,HybridState>,
    mode:String,text:String,name:String)->Result<RehearsalInfo,String>{
    if state.inner.training.load(Ordering::Acquire){
        return Err("Cannot edit rehearsal lessons during training or testing".into());
    }
    if name.trim().is_empty()||name.len()>80{return Err("Lesson name must be 1–80 characters".into());}
    let mut s=lock(&state)?;
    if !s.trained{return Err("Teach Nova before archiving a previous lesson".into());}
    let mut bank=rehearsal_for_current(&app,&s.brain)?;
    rehearsal::append(&mut bank,LessonRecord{mode,text,name},
        diag::weight_digest(&s.brain))?;
    rehearsal::write(&rehearsal_path(&app)?,&bank)?;
    diag::record(&app.state::<Diagnostics>(),"rehearsal_manual_archive",serde_json::json!({
        "lessonCount":bank.lessons.len(),"step":s.brain.transformer.step,
        "note":"Previous lesson archived for actual neural rehearsal, no update performed"
    }));
    Ok(RehearsalInfo{names:bank.lessons.iter().map(|l|l.name.clone()).collect(),
        modes:bank.lessons.iter().map(|l|l.mode.clone()).collect(),matched:true,
        foundation_lessons_completed:bank.foundation_lessons_completed,
        foundation_lessons_remaining:rehearsal::FOUNDATION_LESSON_LIMIT.saturating_sub(bank.foundation_lessons_completed)})
}
#[tauri::command]
pub fn hybrid_rehearsal_clear(app:AppHandle,state:State<'_,HybridState>)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Cannot clear bank during training".into());}
    let _s=lock(&state)?;
    let target=rehearsal_path(&app)?;
    if target.exists(){fs::remove_file(target).map_err(|e|e.to_string())?;}
    Ok(())
}
fn conversation_path(app:&AppHandle)->Result<PathBuf,String>{
    Ok(path(app)?.with_file_name(CHAT_FILE))
}
fn save_conversation(app:&AppHandle,history:&[ConversationTurn])->Result<(),String>{
    let target=conversation_path(app)?;
    let tmp=target.with_extension("tmp");
    let contents=serde_json::to_vec(history).map_err(|e|e.to_string())?;
    {
        use std::io::Write;
        let mut file=fs::File::create(&tmp).map_err(|e|e.to_string())?;
        file.write_all(&contents).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
    }
    let backup=target.with_extension("bak");
    if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
    if target.exists(){fs::rename(&target,&backup).map_err(|e|e.to_string())?;}
    match fs::rename(&tmp,&target){
        Ok(())=>{
            if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
            Ok(())
        },
        Err(error)=>{
            if backup.exists(){let _=fs::rename(&backup,&target);}
            Err(error.to_string())
        }
    }
}
fn memory_turns(history:&[ConversationTurn])->Vec<String>{
    history.iter().skip(history.len().saturating_sub(MEMORY_SLOTS)).map(|t|
        format!("U: {}\nN: {}",t.user,t.assistant)).collect()
}
fn path(app:&AppHandle)->Result<PathBuf,String>{
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    Ok(dir.join(CHECKPOINT))
}
fn snapshot_active_checkpoint(app:&AppHandle,brain:&HybridBrain,reason:&str)->Result<(),String>{
    let current=path(app)?;
    if !current.exists(){return Ok(());}
    let directory=current.parent().ok_or("Checkpoint has no parent directory")?
        .join("neural-nova-checkpoint-history");
    fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e|e.to_string())?.as_nanos();
    let backup=directory.join(format!(
        "neural-nova-{reason}-step{}-{stamp}.nhyb",brain.transformer.step
    ));
    let size=fs::copy(&current,&backup).map_err(|e|e.to_string())?;
    if size==0 {return Err("Refusing to start learning: checkpoint backup was empty".into());}
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct CheckpointTransferInfo{bytes:usize,step:u64,words:usize,parameters:usize}
/// Take a consistent frozen copy of the whole joint brain including Adam
/// moments and tokenizer. The file on disk remains untouched.
#[tauri::command]
pub fn hybrid_checkpoint_export_begin(app:AppHandle,
    state:State<'_,HybridState>,transfer:State<'_,TransferState>)
    ->Result<CheckpointTransferInfo,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Wait until training completes".into());}
    let s=lock(&state)?;
    if !s.trained{return Err("Teach Nova before exporting neural weights".into());}
    // Always serialize to a REAL .nhyb path. HybridBrain::save stages its
    // output by changing the extension to .tmp; if the requested destination
    // ALREADY ends in .tmp those names collide and its atomic rename fails
    // with "No such file or directory (os error 2)".
    let temp=path(&app)?.with_file_name("neural-nova-checkpoint-export.nhyb");
    let serialized=(||->Result<Vec<u8>,String>{
        s.brain.save(&temp)?;
        fs::read(&temp).map_err(|e|format!("Cannot read serialized checkpoint: {e}"))
    })();
    let _=fs::remove_file(&temp);
    let bytes=serialized?;
    let length=transfer.begin_export(bytes)?;
    Ok(CheckpointTransferInfo{bytes:length,step:s.brain.transformer.step,
        words:s.brain.tokenizer.words.len(),parameters:s.brain.parameter_count()})
}
#[tauri::command]
pub fn hybrid_checkpoint_export_chunk(
    transfer:State<'_,TransferState>,offset:usize)->Result<Vec<u8>,String>{
    transfer.chunk(offset)
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct AndroidCheckpointExportResult{bytes:usize,verified:bool}
/// Save the SAME immutable export bytes via Android's system document picker.
/// Unlike WebView Blob URLs or some Tauri FS content:// writes, this passes
/// the data through a Java ContentResolver stream that remains open until
/// writing and SHA-256 read-back verification both complete.
#[tauri::command]
pub fn hybrid_checkpoint_export_android(
    app:AppHandle,transfer:State<'_,TransferState>,destination_uri:String
)->Result<AndroidCheckpointExportResult,String>{
    #[cfg(target_os="android")]
    {
        if !destination_uri.starts_with("content://"){
            return Err("Choose a .nhyb document destination with Android Files".into());
        }
        let bytes=transfer.export_read()?;
        let scratch=path(&app)?.with_file_name("neural-nova-android-transfer.nhyb");
        let write=fs::write(&scratch,&bytes)
            .map_err(|e|format!("Could not stage the checkpoint in app-private storage: {e}"));
        if let Err(e)=write {return Err(e);}
        let attempted=crate::android_checkpoint_export::write_android_document(
            &scratch,&destination_uri);
        let _=fs::remove_file(&scratch);
        let actual=attempted?;
        if actual as usize!=bytes.len(){
            return Err(format!("Android checkpoint save was incomplete: {actual}/{} bytes; do not rely on this file",bytes.len()));
        }
        transfer.export_cancel()?;
        Ok(AndroidCheckpointExportResult{bytes:bytes.len(),verified:true})
    }
    #[cfg(not(target_os="android"))]
    {
        let _=(app,transfer,destination_uri);
        Err("Android document export is only available in the Android app".into())
    }
}
#[tauri::command]
pub fn hybrid_checkpoint_export_cancel(
    transfer:State<'_,TransferState>
)->Result<(),String>{transfer.export_cancel()}
#[tauri::command]
pub fn hybrid_checkpoint_import_begin(
    state:State<'_,HybridState>,transfer:State<'_,TransferState>,
    bytes:usize)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Wait until training completes".into());}
    transfer.import_begin(bytes)
}
#[tauri::command]
pub fn hybrid_checkpoint_import_append(
    transfer:State<'_,TransferState>,chunk:Vec<u8>)->Result<usize,String>{
    transfer.import_append(chunk)
}
#[tauri::command]
pub fn hybrid_checkpoint_import_cancel(transfer:State<'_,TransferState>)->Result<(),String>{
    transfer.cancel()
}
#[tauri::command]
pub fn hybrid_checkpoint_import_finish(
    app:AppHandle,state:State<'_,HybridState>,
    transfer:State<'_,TransferState>
)->Result<HybridStatus,String>{
    // No training or tests can start during final validation and replacement.
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Wait until Nova finishes teaching or testing".into());
    }
    let attempt=(||->Result<HybridStatus,String>{
        let bytes=transfer.import_read()?;
        let temp=path(&app)?.with_extension("checkpoint-import.tmp");
        fs::write(&temp,&bytes).map_err(|e|e.to_string())?;
        let candidate=HybridBrain::load(&temp);
        let _=fs::remove_file(&temp);
        let brain=candidate?;
        let mut s=lock(&state)?;
        if s.trained{snapshot_active_checkpoint(&app,&s.brain,"before-import")?;}
        // Write the validated imported joint checkpoint atomically first.
        brain.save(path(&app)?)?;
        let marker=path(&app)?.with_extension("no-import");
        if marker.exists(){fs::remove_file(marker).map_err(|e|e.to_string())?;}
        // Old conversation history belongs to the former brain, and should
        // never influence this independently restored one.
        save_conversation(&app,&[])?;
        s.brain=brain;s.trained=true;s.history.clear();
        s.epoch=0;s.epochs=0;s.completed=0;s.total=0;
        s.train_loss=None;s.heldout_loss=None;s.heldout_accuracy=None;
        s.best_epoch=0;s.best_heldout_loss=None;s.early_stopped=false;
        s.error=None;s.stage="Imported joint neural checkpoint".into();
        s.retention_baseline=None;s.retention_accuracy=None;
        diag::record(&app.state::<Diagnostics>(),"checkpoint_imported",serde_json::json!({
            "steps":s.brain.transformer.step,"words":s.brain.tokenizer.words.len(),
            "fingerprint":diag::weight_digest(&s.brain),
            "rehearsalNote":"Archive is matched against imported joint weight fingerprint"
        }));
        let _=transfer.cancel();
        Ok(status(&state,&s))
    })();
    state.inner.training.store(false,Ordering::Release);
    attempt.map(|mut result|{result.training=false;result})
}

#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct HybridStatus{
    training:bool,trained:bool,parameters:usize,
    transformer_parameters:usize,dense_parameters:usize,
    transformer_step:u64,dense_step:u32,context:usize,
    epoch:u32,epochs:u32,completed:usize,total:usize,
    heldout_loss:Option<f32>,heldout_accuracy:Option<f32>,
    train_loss:Option<f32>,error:Option<String>,
    best_heldout_loss:Option<f32>,best_epoch:u32,learning_rate:f32,
    train_lessons:usize,validation_lessons:usize,
    tokens_per_second:f32,stage:String,early_stopped:bool,
    memory_turns:usize,tokenizer_words:usize,
    retention_baseline:Option<f32>,retention_accuracy:Option<f32>,
}
fn status(state:&HybridState,s:&Session)->HybridStatus{
    HybridStatus{
        training:state.inner.training.load(Ordering::Acquire),
        trained:s.trained,parameters:s.brain.parameter_count(),
        transformer_parameters:s.brain.transformer.parameter_count(),
        dense_parameters:s.brain.dense.parameter_count(),
        transformer_step:s.brain.transformer.step,dense_step:s.brain.dense_adam_steps(),
        context:256,epoch:s.epoch,epochs:s.epochs,completed:s.completed,total:s.total,
        heldout_loss:s.heldout_loss,heldout_accuracy:s.heldout_accuracy,
        train_loss:s.train_loss,error:s.error.clone(),
        best_heldout_loss:s.best_heldout_loss,best_epoch:s.best_epoch,
        learning_rate:s.learning_rate,
        train_lessons:s.train_lessons,validation_lessons:s.validation_lessons,
        tokens_per_second:s.tokens_per_second,stage:s.stage.clone(),early_stopped:s.early_stopped,
        memory_turns:s.history.len().min(MEMORY_SLOTS),tokenizer_words:s.brain.tokenizer.words.len(),
        retention_baseline:s.retention_baseline,retention_accuracy:s.retention_accuracy
    }
}
#[tauri::command]
pub fn hybrid_boot(app:AppHandle,state:State<'_,HybridState>)->Result<HybridStatus,String>{
    // Android may recreate the WebView while native background ZIP training is
    // still running. UI boot must ATTACH to the live brain, never reload an old
    // checkpoint over the active weights or reject the reconnect.
    if state.inner.training.load(Ordering::Acquire) ||
        app.state::<crate::lesson_zip::ZipQueueState>().is_active(){
        let s=lock(&state)?;
        return Ok(status(&state,&s));
    }
    let mut s=lock(&state)?;
    let file=path(&app)?;
    if file.exists(){
        s.brain=HybridBrain::load(&file)?;
        s.trained=true;
    }else if !file.with_extension("no-import").exists(){
        // Try newest compatible checkpoint first, preserving both originals.
        for legacy in [OLD_CHECKPOINT,FIRST_CHECKPOINT]{
            let candidate=file.with_file_name(legacy);
            if candidate.exists(){
                let migrated=HybridBrain::load(&candidate)?;
                migrated.save(&file)?;
                s.brain=migrated;
                s.trained=true;
                break;
            }
        }
    }
    let conversation=conversation_path(&app)?;
    if conversation.exists(){
        let contents=fs::read(&conversation).map_err(|e|e.to_string())?;
        let turns:Vec<ConversationTurn>=serde_json::from_slice(&contents).map_err(|e|e.to_string())?;
        if turns.len()>HISTORY_LIMIT || turns.iter().any(|t|
            t.user.len()>180||t.assistant.len()>256 || !t.user.is_ascii()||!t.assistant.is_ascii()){
            return Err("Invalid persisted conversation data".into());
        }
        s.history=turns;
    }
    // A terminated native process cannot have a still-running test.
    let archive_path=test_archive_path(&app)?;
    if archive_path.exists(){
        let data=fs::read(&archive_path).map_err(|e|e.to_string())?;
        let mut archive:TestArchive=serde_json::from_slice(&data).map_err(|e|e.to_string())?;
        for report in &mut archive.reports{
            if report.status=="running" || report.status=="paused"{
                report.status="interrupted".into();
            }
        }
        *state.inner.tests.lock().map_err(|_|"Test archive lock poisoned")?=archive;
    }
    Ok(status(&state,&s))
}
#[tauri::command]
pub fn hybrid_conversation(state:State<'_,HybridState>)->Result<Vec<ConversationTurn>,String>{
    Ok(lock(&state)?.history.clone())
}
#[tauri::command]
pub fn hybrid_new_chat(app:AppHandle,state:State<'_,HybridState>)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Training in progress".into());}
    let mut s=lock(&state)?;
    save_conversation(&app,&[])?;
    s.history.clear();
    Ok(())
}
#[tauri::command]
pub fn hybrid_status(state:State<'_,HybridState>)->Result<HybridStatus,String>{
    let s=lock(&state)?;Ok(status(&state,&s))
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct AttentionValue{character:String,weight:f32,index:usize}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct HybridTrace{
    dense_layers:Vec<Vec<f32>>,
    attention:Vec<AttentionValue>,
    bridge_signal:Vec<f32>,
    top_predictions:Vec<TokenProbability>,
}
#[derive(Serialize)]
struct TokenProbability{code:usize,probability:f32}
fn trace(brain:&HybridBrain,history:&[u8],window:usize,memory:&[String])->Result<HybridTrace,String>{
    let (layers,attention,bridge,p)=brain.inspect_with_memory(history,window,memory)?;
    let mut sorted=(0..p.len()).collect::<Vec<usize>>();
    sorted.sort_by(|&a,&b|p[b].total_cmp(&p[a]));
    Ok(HybridTrace{
        dense_layers:layers,bridge_signal:bridge,
        attention:attention.into_iter().map(|a|AttentionValue{
            index:a.index,weight:a.probability,
            character:brain.tokenizer.token_label(a.token_id)
        }).collect(),
        top_predictions:sorted.into_iter().take(8).map(|code|TokenProbability{code,probability:p[code]}).collect(),
    })
}
#[tauri::command]
pub async fn hybrid_inspect(
    app:AppHandle,state:State<'_,HybridState>,message:String,context_chars:Option<usize>,
)->Result<HybridTrace,String>{
    if message.is_empty() || !message.is_ascii() || message.len()>180{
        return Err("Inspect 1–180 ASCII characters".into());
    }
    let window=valid_window(context_chars)?;
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){return Err("Wait for training to finish".into());}
        let s=lock(&state)?;
        let context=dialogue_prefix(&message)?;
        let memory=memory_turns(&s.history);
        let visualization=trace(&s.brain,&context,window,&memory)?;
        if diag::enabled(&app.state::<Diagnostics>()){
            let prediction=diagnostic_prediction(&s.brain,&message,window,&memory,false)
                .unwrap_or_else(|e|serde_json::json!({"error":e}));
            diag::record(&app.state::<Diagnostics>(),"visual_inspection",serde_json::json!({
                "modelStep":s.brain.transformer.step,
                "memorySlots":memory.len(),"probe":prediction
            }));
        }
        Ok(visualization)
    }).await.map_err(|e|e.to_string())?
}
fn valid_window(input:Option<usize>)->Result<usize,String>{
    let window=input.unwrap_or(128);
    if window==128 || window==256{Ok(window)}
    else{Err("Context length must be 128 or 256 characters".into())}
}
fn greedy_sample(p:&[f32])->u8{
    (0..VOCAB).filter(|&i|i==10 || (32..127).contains(&i))
        .max_by(|&i,&j|p[i].total_cmp(&p[j]))
        .unwrap_or(10) as u8
}
fn random(seed:&mut u64)->f32{
    *seed ^= *seed<<13;*seed ^= *seed>>7;*seed ^= *seed<<17;
    (*seed>>40) as f32/16777216.0
}
fn sampled(p:&[f32],temperature:f32,random:f32)->u8{
    let mut weights=[0f32;VOCAB];
    let mut sum=0f32;
    for i in 0..VOCAB{
        if i==10 || (32..127).contains(&i){
            weights[i]=p[i].powf(1.0/temperature);
            sum+=weights[i];
        }
    }
    let mut goal=random*sum;
    for (i,weight) in weights.iter().enumerate(){
        if *weight==0.0{continue;}
        goal-=*weight;
        if goal<=0.0{return i as u8;}
    }
    b'\n'
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ContextReport {
    window:usize,
    prompt_bytes:usize,
    generated_bytes:usize,
    prompt_bytes_retained:usize,
    first_prompt_truncation_char:Option<usize>,
    termination:&'static str,
    deterministic:bool,
    temperature:f32,
    seed_used:Option<u64>,
    prompt_tokens:usize,
    prompt_tokens_retained:usize,
    active_tokens:usize,
    memory_turns:usize,
}
fn context_report(window:usize,prompt_bytes:usize,generated_bytes:usize,
    termination:&'static str,deterministic:bool,temperature:f32,seed_used:Option<u64>
)->ContextReport{
    // The oldest bytes fall out first. Record when the original prompt
    // started disappearing, not just whether the context window is full.
    let overflow=prompt_bytes.saturating_add(generated_bytes).saturating_sub(window);
    let prompt_bytes_retained=prompt_bytes.saturating_sub(overflow);
    let first_prompt_truncation_char=if prompt_bytes>window{Some(0)}
        else{
            let threshold=window-prompt_bytes+1;
            if generated_bytes>=threshold{Some(threshold)}else{None}
        };
    ContextReport{window,prompt_bytes,generated_bytes,prompt_bytes_retained,
        first_prompt_truncation_char,termination,deterministic,temperature,seed_used,
        prompt_tokens:prompt_bytes,prompt_tokens_retained:prompt_bytes_retained,memory_turns:0,
        active_tokens:(prompt_bytes+generated_bytes).min(window)}
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct HybridReply{
    response:String,trace:HybridTrace,status:HybridStatus,context_report:ContextReport,
}
#[tauri::command]
pub async fn hybrid_chat(
    app:AppHandle,state:State<'_,HybridState>,message:String,max_chars:usize,temperature:f32,
    context_chars:Option<usize>,deterministic:Option<bool>,seed:Option<u64>,
    commit:Option<bool>,
)->Result<HybridReply,String>{
    if message.is_empty() || message.len()>180 || !message.is_ascii() || message.contains('\n') || message.contains('\r') {
        return Err("Use a single-line 1–180 character ASCII message".into());
    }
    if !(1..=256).contains(&max_chars) || !temperature.is_finite() || !(0.2..=2.0).contains(&temperature) {
        return Err("Invalid generation length or temperature".into());
    }
    let window=valid_window(context_chars)?;
    let deterministic=deterministic.unwrap_or(false);
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){return Err("Wait for training to finish".into());}
        let mut s=lock(&state)?;
        let memory=memory_turns(&s.history);
        let mut history=dialogue_prefix(&message)?;
        let prompt_bytes=history.len();
        let prompt_tokens=s.brain.token_count(std::str::from_utf8(&history).map_err(|e|e.to_string())?)?;
        let log_prediction=if diag::enabled(&app.state::<Diagnostics>()){
            Some(diagnostic_prediction(&s.brain,&message,window,&memory,false)
                .unwrap_or_else(|e|serde_json::json!({"error":e})))
        }else{None};
        let mut result=Vec::<u8>::new();
        let seed_used=if deterministic{None}else{seed};
        let mut rng=seed.unwrap_or_else(||SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|v|v.as_nanos() as u64).unwrap_or(41)).max(1);
        let mut termination="length-limit";
        for _ in 0..max_chars{
            let p=s.brain.predict_with_memory(&history,window,&memory)?;
            let token=if deterministic{greedy_sample(&p)}else{sampled(&p,temperature,random(&mut rng))};
            if token==b'\n'{termination="learned-end";break;}
            history.push(token);
            result.push(token);
        }
        let response=String::from_utf8(result).map_err(|e|e.to_string())?;
        let full_token_count=s.brain.token_count(
            std::str::from_utf8(&history).map_err(|e|e.to_string())?)?;
        let mut report=context_report(window,prompt_bytes,response.len(),termination,
            deterministic,temperature,seed_used);
        let dropped=full_token_count.saturating_sub(window);
        report.prompt_tokens=prompt_tokens;
        report.prompt_tokens_retained=prompt_tokens.saturating_sub(dropped);
        report.active_tokens=full_token_count.min(window);
        // These fields describe the actual tokenized prompt, not the old
        // character-only estimate. A learned word may occupy one position.
        let full_tokens=s.brain.tokenizer.encode(
            std::str::from_utf8(&history).map_err(|e|e.to_string())?)?;
        let dropped_text=s.brain.tokenizer.decode(&full_tokens[..dropped])?;
        report.prompt_bytes_retained=prompt_bytes.saturating_sub(dropped_text.len());
        report.first_prompt_truncation_char=None; // exact token-level cutoff supplied instead
        report.memory_turns=memory.len();
        if let Some(ref prediction)=log_prediction{
            diag::record(&app.state::<Diagnostics>(),"chat_inference",serde_json::json!({
                "message":message,"response":response,"committed":commit.unwrap_or(true),
                "temperature":temperature,"deterministic":deterministic,
                "samplingSeed":seed_used,"maxChars":max_chars,"window":window,
                "modelStep":s.brain.transformer.step,"denseStep":s.brain.dense_adam_steps(),
                "wordUnits":s.brain.tokenizer.words.len(),"memorySlotsUsed":memory.len(),
                "contextReport":report,
                "firstTokenAnalysis":prediction,
            }));
        }
        let next=ConversationTurn{user:message,assistant:response.clone()};
        let mut future=s.history.clone();
        future.push(next);
        if future.len()>HISTORY_LIMIT{future.remove(0);}
        let neural_trace=trace(&s.brain,&history,window,&memory)?;
        if commit.unwrap_or(true){
            save_conversation(&app,&future)?;
            s.history=future;
        }
        Ok(HybridReply{
            response,trace:neural_trace,status:status(&state,&s),
            context_report:report,
        })
    }).await.map_err(|e|e.to_string())?
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TokenizerProbeSample{
    prompt:String,stage:String,token_ids:Vec<u16>,token_labels:Vec<String>,
    vocabulary:usize,transformer_step:u64,dense_step:u32,
    top_predictions:Vec<TokenProbability>,response:String,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TokenizerProbeReport{
    added_word_units:usize,unchanged_weights:bool,
    samples:Vec<TokenizerProbeSample>,
}
fn tokenizer_probe_sample(brain:&HybridBrain,prompt:&str,stage:&str,
    length:usize,window:usize)->Result<TokenizerProbeSample,String>{
    let mut history=dialogue_prefix(prompt)?;
    let tokens=brain.tokenizer.encode(std::str::from_utf8(&history).map_err(|e|e.to_string())?)?;
    let p=brain.predict_window(&history,window)?;
    let mut ranked=(0..p.len()).collect::<Vec<_>>();
    ranked.sort_by(|&a,&b|p[b].total_cmp(&p[a]));
    let top_predictions=ranked.into_iter().take(8)
        .map(|code|TokenProbability{code,probability:p[code]}).collect();
    let mut response=Vec::new();
    for _ in 0..length{
        let predictions=brain.predict_window(&history,window)?;
        let c=greedy_sample(&predictions);
        if c==b'\n'{break;}
        history.push(c);
        response.push(c);
    }
    Ok(TokenizerProbeSample{
        prompt:prompt.to_owned(),stage:stage.into(),
        token_labels:tokens.iter().map(|&id|brain.tokenizer.token_label(id)).collect(),
        token_ids:tokens,
        vocabulary:brain.tokenizer.words.len(),
        transformer_step:brain.transformer.step,dense_step:brain.dense_adam_steps(),
        top_predictions,response:String::from_utf8(response).map_err(|e|e.to_string())?,
    })
}
/// Non-mutating A/B experiment: stage B grows tokenizer units on a cloned
/// brain, with ZERO optimizer steps or edits to the live neural checkpoint.
#[tauri::command]
pub async fn hybrid_tokenizer_probe(
    state:State<'_,HybridState>,mode:String,lesson_text:String,probes:String,
    max_chars:Option<usize>,context_chars:Option<usize>,
)->Result<TokenizerProbeReport,String>{
    let length=max_chars.unwrap_or(48);
    if !(1..=80).contains(&length){return Err("Probe response length must be 1–80".into());}
    let window=valid_window(context_chars)?;
    let plan=prepare(&mode,&lesson_text)?;
    let prompts=probes.lines().map(str::trim).filter(|s|!s.is_empty()).collect::<Vec<_>>();
    if prompts.is_empty()||prompts.len()>6||
        prompts.iter().any(|s|s.len()>180||!s.is_ascii()||s.contains('\r')){
        return Err("Supply 1–6 ASCII prompts, max 180 characters each".into());
    }
    let prompts=prompts.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){
            return Err("Finish training before a tokenizer-only investigation".into());
        }
        let original=lock(&state)?.brain.clone();
        let mut expanded=original.clone();
        let added=expanded.learn_token_units(&plan.training_text);
        let unchanged_weights=original.transformer.weights==expanded.transformer.weights &&
            original.dense.layers.iter().zip(&expanded.dense.layers).all(|(a,b)|
                a.weights==b.weights&&a.biases==b.biases);
        let mut samples=Vec::new();
        for prompt in &prompts{
            samples.push(tokenizer_probe_sample(&original,prompt,"A: original",length,window)?);
            samples.push(tokenizer_probe_sample(&expanded,prompt,"B: tokenizer only",length,window)?);
        }
        Ok(TokenizerProbeReport{added_word_units:added,unchanged_weights,samples})
    }).await.map_err(|e|e.to_string())?
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct BridgeProbe{
    transformer_norm:f32,dense_norm:f32,projected_bridge_norm:f32,
    combined_norm:f32,bridge_to_transformer_ratio:f32,
    next_character_total_variation:f32,
    transformer_step:u64,dense_step:u32,
}
#[tauri::command]
pub async fn hybrid_bridge_probe(
    state:State<'_,HybridState>,message:String,context_chars:Option<usize>,
)->Result<BridgeProbe,String>{
    if message.is_empty()||message.len()>180||!message.is_ascii(){
        return Err("Use 1–180 ASCII characters for bridge inspection".into());
    }
    let window=valid_window(context_chars)?;
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){
            return Err("Finish neural training before bridge inspection".into());
        }
        let s=lock(&state)?;
        let context=dialogue_prefix(&message)?;
        let a=s.brain.bridge_measurement(&context,window)?;
        Ok(BridgeProbe{
            transformer_norm:a.transformer_norm,dense_norm:a.dense_norm,
            projected_bridge_norm:a.projected_bridge_norm,
            combined_norm:a.combined_norm,
            bridge_to_transformer_ratio:a.bridge_to_transformer_ratio,
            next_character_total_variation:a.next_character_total_variation,
            transformer_step:s.brain.transformer.step,dense_step:s.brain.dense_adam_steps(),
        })
    }).await.map_err(|e|e.to_string())?
}

fn first_character_summary(probabilities:&[f32])->serde_json::Value{
    let mut ranked=(0..probabilities.len()).collect::<Vec<_>>();
    ranked.sort_by(|&a,&b|probabilities[b].total_cmp(&probabilities[a]));
    serde_json::json!({
        "h":probabilities[b'H' as usize],
        "y":probabilities[b'Y' as usize],
        "newline":probabilities[b'\n' as usize],
        "top":ranked.into_iter().take(8).map(|id|serde_json::json!({
            "code":id,"probability":probabilities[id]
        })).collect::<Vec<_>>()
    })
}
fn probability_distance(a:&[f32],b:&[f32])->f32{
    a.iter().zip(b.iter()).map(|(x,y)|(x-y).abs()).sum::<f32>()*0.5
}
/// Every variant uses the SAME current joint brain weights. Memory is never
/// placed in the prompt; it uses the actual trainable episodic reader.
fn diagnostic_prediction(brain:&HybridBrain,prompt:&str,window:usize,
    memory:&[String],include_greedy:bool)->Result<serde_json::Value,String>{
    let context=dialogue_prefix(prompt)?;
    let tokens=brain.tokenizer.encode(std::str::from_utf8(&context).map_err(|e|e.to_string())?)?;
    let without=brain.predict_with_memory(&context,window,&[])?;
    let with=if memory.is_empty(){without.clone()}
        else{brain.predict_with_memory(&context,window,memory)?};
    let first=first_character_summary(&with);
    let bridge=brain.bridge_measurement(&context,window)?;
    let attention=brain.inspect_with_memory(&context,window,memory)?.1;
    let mut output=Vec::new();
    if include_greedy{
        let mut rolling=context.clone();
        for _ in 0..32{
            let p=brain.predict_with_memory(&rolling,window,memory)?;
            let next=greedy_sample(&p);
            if next==b'\n'{break;}
            output.push(next);
            rolling.push(next);
        }
    }
    Ok(serde_json::json!({
        "prompt":prompt,"contextWindowTokens":window,"inputBytes":context.len(),
        "tokenIds":tokens,"tokenLabels":tokens.iter().map(|&id|brain.tokenizer.token_label(id)).collect::<Vec<_>>(),
        "memorySlots":memory.len(),"memoryTurnSizes":memory.iter().map(|m|m.len()).collect::<Vec<_>>(),
        "firstWithMemory":first,
        "firstWithoutMemory":first_character_summary(&without),
        "memoryTotalVariation":probability_distance(&with,&without),
        "attention":attention.iter().map(|a|serde_json::json!({
            "tokenId":a.token_id,"position":a.index,"weight":a.probability
        })).collect::<Vec<_>>(),
        "bridge":{
            "transformerNorm":bridge.transformer_norm,
            "denseNorm":bridge.dense_norm,
            "bridgeNorm":bridge.projected_bridge_norm,
            "combinedNorm":bridge.combined_norm,
            "bridgeRatio":bridge.bridge_to_transformer_ratio,
            "bridgeAblationTotalVariation":bridge.next_character_total_variation,
        },
        "greedy32":String::from_utf8(output).map_err(|e|e.to_string())?,
    }))
}
fn diagnostic_probes(brain:&HybridBrain,prompts:&[String],window:usize,
    memory:&[String],full:bool)->Vec<serde_json::Value>{
    prompts.iter().map(|p|match diagnostic_prediction(brain,p,window,memory,full){
        Ok(v)=>v,Err(e)=>serde_json::json!({"prompt":p,"error":e})
    }).collect()
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct DiagnosticProbeResult{
    transformer_step:u64,dense_step:u32,vocabulary:usize,
    with_conversation_memory:bool,probes:Vec<serde_json::Value>,
    weights_fingerprint:String,
}
#[tauri::command]
pub async fn hybrid_diagnostic_probe(
    app:AppHandle,state:State<'_,HybridState>,prompts:String,context_chars:Option<usize>,
    use_conversation_memory:Option<bool>,
)->Result<DiagnosticProbeResult,String>{
    let window=valid_window(context_chars)?;
    let entries=prompts.lines().map(str::trim).filter(|s|!s.is_empty()).collect::<Vec<_>>();
    if entries.is_empty()||entries.len()>8 ||
        entries.iter().any(|p|p.len()>180||!p.is_ascii()||p.contains('\r')){
        return Err("Supply 1–8 single-line ASCII prompts, up to 180 characters each".into());
    }
    let entries=entries.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){
            return Err("Pause model changes by finishing training before this independent probe".into());
        }
        let (model,memory)={
            let s=lock(&state)?;
            let remembered=if use_conversation_memory.unwrap_or(false){memory_turns(&s.history)}
                else{Vec::new()};
            (s.brain.clone(),remembered)
        };
        let probes=diagnostic_probes(&model,&entries,window,&memory,true);
        let result=DiagnosticProbeResult{
            transformer_step:model.transformer.step,dense_step:model.dense_adam_steps(),
            vocabulary:model.tokenizer.words.len(),
            with_conversation_memory:!memory.is_empty(),
            probes,weights_fingerprint:diag::weight_digest(&model),
        };
        if diag::enabled(&app.state::<Diagnostics>()){
            diag::record(&app.state::<Diagnostics>(),"independent_probe",serde_json::to_value(&result)
                .map_err(|e|e.to_string())?);
        }
        Ok(result)
    }).await.map_err(|e|e.to_string())?
}

#[derive(Clone,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TrainOptions {
    // Accept old 0.3.1 clients as a compatibility fallback only.
    adaptive:Option<bool>,
    adaptive_learning_rate:Option<bool>,
    early_stopping:Option<bool>,
    learning_rate:Option<f32>,
    patience:Option<u32>,
    retention_test_id:Option<String>,
    rehearsal:Option<bool>,
    protect_previous:Option<bool>,
    retention_loss_tolerance:Option<f32>,
    foundation_training:Option<bool>,
}
fn resolved_switches(options:Option<&TrainOptions>)->(bool,bool){
    let legacy=options.and_then(|o|o.adaptive);
    let adaptive_learning_rate=options.and_then(|o|o.adaptive_learning_rate)
        .or(legacy).unwrap_or(false);
    let early_stopping=options.and_then(|o|o.early_stopping)
        .or(legacy).unwrap_or(true);
    (adaptive_learning_rate,early_stopping)
}
#[tauri::command]
pub fn hybrid_train(
    app:AppHandle,state:State<'_,HybridState>,lesson:Lesson,
    options:Option<TrainOptions>,
)->Result<(),String>{
    if !(1..=60).contains(&lesson.epochs){return Err("Choose 1–60 epochs".into());}
    let plan=prepare(&lesson.mode,&lesson.text)?;
    // Saved teacher evaluation bank is READ ONLY. It never enters train samples.
    let retention=if let Some(id)=options.as_ref().and_then(|o|o.retention_test_id.as_ref())
        .filter(|id|!id.is_empty()){
        let archive=test_archive(&state)?;
        archive.definitions.iter().find(|test|test.id==*id)
            .ok_or("Selected previous-skill benchmark was not found")?
            .cases.iter().take(4).cloned().collect::<Vec<_>>()
    }else{Vec::new()};
    let (adaptive_learning_rate,early_stopping)=resolved_switches(options.as_ref());
    let rehearsal_enabled=options.as_ref().and_then(|x|x.rehearsal).unwrap_or(true);
    let guard_enabled=options.as_ref().and_then(|x|x.protect_previous).unwrap_or(true);
    let tolerance=options.as_ref().and_then(|x|x.retention_loss_tolerance).unwrap_or(0.02);
    if !tolerance.is_finite() || !(0.0..=0.5).contains(&tolerance){
        return Err("Previous-lesson loss tolerance must be between 0 and 0.5".into());
    }
    let starting_lr=options.as_ref().and_then(|o|o.learning_rate).unwrap_or(DEFAULT_LEARNING_RATE);
    let patience=options.as_ref().and_then(|o|o.patience).unwrap_or(6);
    if !starting_lr.is_finite() || !(0.0001..=0.006).contains(&starting_lr){
        return Err("Learning rate must be between 0.0001 and 0.006".into());
    }
    if !(3..=15).contains(&patience){return Err("Patience must be 3–15 epochs".into());}
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova is already training".into());
    }
    let state=state.inner().clone();
    // Only curricula associated with EXACTLY these joint weights are eligible;
    // restoring an unrelated checkpoint cannot inject another run's lessons.
    let starting_bank=match lock(&state){
        Ok(s)=>rehearsal_for_current(&app,&s.brain),
        Err(e)=>Err(e),
    };
    let starting_bank=match starting_bank{
        Ok(bank)=>bank,
        Err(e)=>{state.inner.training.store(false,Ordering::Release);return Err(e);}
    };
    // An overflowing replay bank must fail BEFORE training; otherwise
    // a newly accepted neural checkpoint could be saved without its lesson.
    if starting_bank.lessons.len()>=rehearsal::MAX_ARCHIVED_LESSONS{
        state.inner.training.store(false,Ordering::Release);
        return Err("Rehearsal archive is full (128). Export the lesson archive before continuing; older entries were not deleted".into());
    }
    // Foundation is deliberately opt-in, but automatically ends after five
    // COMPLETED lesson sessions, regardless of whether a candidate is accepted.
    // The counter belongs to the current weights' teacher archive, not UI state.
    let foundation_requested=options.as_ref().and_then(|o|o.foundation_training).unwrap_or(false);
    let foundation=foundation_requested &&
        starting_bank.foundation_lessons_completed<rehearsal::FOUNDATION_LESSON_LIMIT;
    if foundation && (!rehearsal_enabled || !guard_enabled){
        state.inner.training.store(false,Ordering::Release);
        return Err("Foundation Training requires rehearsal and prior-skill protection enabled".into());
    }
    let previous=if rehearsal_enabled || guard_enabled{
        match rehearsal::previous_plans(&starting_bank){
            Ok(v)=>v,
            Err(e)=>{state.inner.training.store(false,Ordering::Release);return Err(e);}
        }
    }else{Vec::new()};
    // Compute exact replay row totals from the same scheduler as training.
    // A short curriculum still rehearses older categories by default.
    let total_training_samples=(1..=lesson.epochs).map(|epoch|{
        let batches=objective_batches(&plan,epoch).len();
        let extra=if rehearsal_enabled{
            balanced_replay_schedule(&previous,epoch,batches)
                .iter().map(|(_,ids,_)|ids.len()).sum::<usize>()
        }else{0};
        let boundary=reply_boundary_batches(&plan,epoch,batches)
            .iter().map(Vec::len).sum::<usize>();
        plan.train.len()+extra+boundary
    }).sum::<usize>() * if foundation{4}else{1};
    let ready=(||->Result<(),String>{
        let mut s=lock(&state)?;
        if s.trained{snapshot_active_checkpoint(&app,&s.brain,"before-lesson")?;}
        let observing=diag::enabled(&app.state::<Diagnostics>());
        let prompts=["hello","Hi","good morning","hey there"]
            .iter().map(|p|p.to_string()).collect::<Vec<_>>();
        if observing {
            diag::record(&app.state::<Diagnostics>(),"lesson_before_vocabulary",serde_json::json!({
                "lessonMode":lesson.mode,"requestedEpochs":lesson.epochs,
                "adaptiveRate":adaptive_learning_rate,"earlyStopping":early_stopping,
                "learningRate":starting_lr,"patience":patience,
                "lessonLengthBytes":lesson.text.len(),"lessonFingerprint":diag::fingerprint(&lesson.text),
                "trainingExamples":plan.train.len(),"validationExamples":plan.validation.len(),
                "supervisedResponseInitials":supervised_initials(&plan),
                "trainingSections":plan.train_lessons,"heldoutSections":plan.validation_lessons,
                "categoryTrainingUnits":plan.category_train,
                "categoryHeldoutUnits":plan.category_validation,
                "validationStrategy":if lesson.mode=="text"{
                    "stable-hash whole paragraph/sentence groups; cross-boundary contexts forbidden"
                }else{"mode-specific independent examples"},
                "trainingSplitFingerprint":diag::fingerprint(&plan.training_text),
                "tokenizerWords":s.brain.tokenizer.words.len(),
                "stepTransformer":s.brain.transformer.step,"stepDense":s.brain.dense_adam_steps(),
                "weightsFingerprint":diag::weight_digest(&s.brain),
                "isolatedGreetingProbes":diagnostic_probes(&s.brain,&prompts,128,&[],false),
                "chatMemoryTurnsUnusedByDialogueTraining":s.history.len().min(MEMORY_SLOTS),
            }));
        }
        let previous_word_count=s.brain.tokenizer.words.len();
        let introduced=s.brain.learn_token_units(&plan.training_text);
        if observing {
            diag::record(&app.state::<Diagnostics>(),"lesson_after_vocabulary",serde_json::json!({
                "addedWords":introduced,"totalWords":s.brain.tokenizer.words.len(),
                "newWordUnits":&s.brain.tokenizer.words[previous_word_count..],
                "stepTransformer":s.brain.transformer.step,"stepDense":s.brain.dense_adam_steps(),
                "weightsFingerprint":diag::weight_digest(&s.brain),
                "isolatedGreetingProbes":diagnostic_probes(&s.brain,&prompts,128,&[],false),
            }));
        }
        s.epoch=0;s.epochs=lesson.epochs;s.completed=0;
        s.total=total_training_samples;
        s.heldout_loss=None;s.heldout_accuracy=None;s.train_loss=None;s.error=None;
        s.best_heldout_loss=None;s.best_epoch=0;s.learning_rate=starting_lr;
        s.train_lessons=plan.train_lessons;s.validation_lessons=plan.validation_lessons;
        s.tokens_per_second=0.0;s.stage="Preparing held-out baseline".into();
        s.early_stopped=false;s.retention_baseline=None;s.retention_accuracy=None;
        Ok(())
    })();
    if let Err(error)=ready{state.inner.training.store(false,Ordering::Release);return Err(error);}
    state.inner.stop.store(false,Ordering::Release);
    let observer_id=match observer::start(&app,"unified",&lesson.mode){
        Ok(id)=>id,Err(e)=>{
            state.inner.training.store(false,Ordering::Release);return Err(e);
        }
    };
    tauri::async_runtime::spawn_blocking(move||{
        let result=if foundation {
            foundation_train_inner(&app,&state,&plan,lesson.epochs,
                adaptive_learning_rate,early_stopping,starting_lr,patience,&retention,
                &previous,rehearsal_enabled,tolerance,&observer_id,
                starting_bank.foundation_lessons_completed)
        }else{
            train_inner(&app,&state,&plan,lesson.epochs,
                adaptive_learning_rate,early_stopping,starting_lr,patience,&retention,
                &previous,rehearsal_enabled,guard_enabled,tolerance,&observer_id)
        };
        if let Err(ref e)=result{
            if let Ok(mut s)=lock(&state){s.error=Some(e.clone());s.stage="Training failed".into();}
            eprintln!("Neural Nova hybrid trainer error: {e}");
            #[cfg(target_os="android")]
            training_notification(&app,format!("Training failed: {}",e),true);
        }
        // Keep training exclusive until teacher archive has been saved.
        match result{
            Ok((cancelled,early_stopped,best_epoch))=>{
                if best_epoch>0 || (foundation && !cancelled) {
                    // Advance the teacher archive only after the selected joint
                    // neural checkpoint is saved. No held-out examples enter replay.
                    let saved=(||->Result<(),String>{
                        let end=diag::weight_digest(&lock(&state)?.brain);
                        let mut bank=starting_bank;
                        if best_epoch>0{
                            rehearsal::append(&mut bank,LessonRecord{
                                mode:lesson.mode.clone(),text:lesson.text.clone(),
                                name:format!("{} lesson · step {}",lesson.mode,
                                    lock(&state)?.brain.transformer.step)
                            },end.clone())?;
                        }
                        // Rejections also consume a completed foundation
                        // lesson: the five-slot schedule never silently extends.
                        if foundation && !cancelled{
                            rehearsal::advance_foundation(&mut bank,&end)?;
                        }
                        rehearsal::write(&rehearsal_path(&app)?,&bank)
                    })();
                    if let Err(e)=saved{
                        // Do not let ZIP training treat this lesson as safely
                        // accepted: future replay would omit a learned lesson.
                        // The checkpoint is already saved, so surface a clear
                        // repair-needed error instead of silently progressing.
                        let issue=format!(
                            "Neural checkpoint saved, but teacher rehearsal archive failed: {e}"
                        );
                        diag::record(&app.state::<Diagnostics>(),"rehearsal_archive_error",
                            serde_json::json!({"message":issue}));
                        if let Ok(mut session)=lock(&state){
                            session.error=Some(issue.clone());
                            session.stage="Lesson archive failed after checkpoint save".into();
                        }
                        let _=app.emit("hybrid-error",issue);
                    }
                }
                state.inner.training.store(false,Ordering::Release);
                let _=app.emit("hybrid-done",serde_json::json!({
                    "cancelled":cancelled,"earlyStopped":early_stopped,"bestEpoch":best_epoch
                }));
            },
            Err(error)=>{
                diag::record(&app.state::<Diagnostics>(),"training_error",serde_json::json!({
                    "message":error,"mayHaveUnrestoredInMemoryWeights":true
                }));
                state.inner.training.store(false,Ordering::Release);
                let _=app.emit("hybrid-error",error);
            }
        }
    });
    Ok(())
}
/// Diagnostic only: complete greedy generation from exactly the same brain,
/// with previous questions/answers excluded from its input and memory.
fn retention_accuracy(brain:&HybridBrain,bank:&[TestCase])->Result<f32,String>{
    if bank.is_empty(){return Err("No prior-skill examples to benchmark".into());}
    let o=TestOptions{attempts:1,corrections:false,learning_rate:0.001,
        max_chars:64,context_chars:128,deterministic:true,
        seed:None,retention:false,generalization:false};
    let stop=AtomicBool::new(false);
    let mut correct=0usize;
    for case in bank{
        let response=test_inference(brain,&case.question,&o,0,&stop)?
            .ok_or("Benchmark was interrupted")?;
        if grade(&response,&case.expected)=="correct"{correct+=1;}
    }
    Ok(correct as f32/bank.len() as f32)
}
/// Curriculum, validation and optimizer scheduling ONLY. The hybrid brain
/// still computes all cognition, predictions and shared gradients itself.
fn evaluate_memory(brain:&HybridBrain,valid:&[Example],memory:&[Vec<String>])->Result<(f32,f32),String>{
    if valid.is_empty()||valid.len()!=memory.len(){return Err("Misaligned memory validation".into());}
    let mut loss=0.0f64;let mut correct=0usize;
    for ((history,target),prior) in valid.iter().zip(memory){
        let p=brain.predict_with_memory(history,256,prior)?;
        loss-=(p[*target as usize].max(1e-30) as f64).ln();
        let top=p.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).unwrap().0;
        if top==*target as usize{correct+=1;}
    }
    Ok(((loss/valid.len() as f64) as f32,correct as f32/valid.len() as f32))
}
/// Category-balanced representative heldout retention panel. The entire
/// source archive remains saved; this sampled check is not exhaustive.
fn previous_validation_loss(brain:&HybridBrain,previous:&[PreparedLessons])
    ->Result<Option<f32>,String>{
    if previous.is_empty(){return Ok(None);}
    let mut totals=[0.0f64;5];
    let mut counts=[0usize;5];
    for index in rehearsal::retention_plan_indices(previous){
        let lesson=&previous[index];
        let indices=rehearsal::prior_validation_selection(lesson);
        if indices.is_empty(){continue;}
        if lesson.mode=="unified"{
            let (_,_,per)=mixed_validation(brain,lesson,&indices)?;
            for (kind,loss) in per.iter().enumerate(){
                if let Some(value)=loss{
                    if !value.is_finite(){return Err("Non-finite category retention loss".into());}
                    totals[kind]+=*value as f64;counts[kind]+=1;
                }
            }
            if !lesson.reasoning_validation.is_empty(){
                totals[3]+=(0.4*reasoning_validation_limited(brain,lesson,2)?.0) as f64;
            }
            if !lesson.understanding_validation.is_empty(){
                totals[1]+=(0.2*understanding_validation_limited(brain,lesson,2)?.0) as f64;
            }
        }else{
            let selected=indices.iter().map(|&i|lesson.validation[i].clone()).collect::<Vec<_>>();
            let (loss,_) = if lesson.mode=="conversation"{
                let histories=indices.iter().map(|&i|lesson.memory_validation[i].clone())
                    .collect::<Vec<_>>();
                evaluate_memory(brain,&selected,&histories)?
            }else{brain.evaluate(&selected)?};
            if !loss.is_finite(){return Err("Non-finite previous validation loss".into());}
            let kind=match lesson.mode.as_str(){
                "dialogue"=>0,"conversation"=>2,"reasoning"=>3,"text"=>4,
                _=>return Err("Unknown archive lesson mode".into())
            };
            totals[kind]+=loss as f64;
            counts[kind]+=1;
            if kind==3{
                totals[3]+=(0.4*reasoning_validation_limited(brain,lesson,2)?.0) as f64;
            }
        }
    }
    let present=counts.iter().filter(|&&count|count>0).count();
    if present==0{return Ok(None);}
    let mut total=0.0f64;
    for kind in 0..5{
        if counts[kind]>0 {total+=totals[kind]/counts[kind] as f64;}
    }
    Ok(Some((total/present as f64) as f32))
}
fn reasoning_validation(brain:&HybridBrain,lessons:&PreparedLessons)
    ->Result<(f32,f32),String>{
    reasoning_validation_limited(brain,lessons,16)
}
fn reasoning_validation_limited(brain:&HybridBrain,lessons:&PreparedLessons,limit:usize)
    ->Result<(f32,f32),String>{
    if lessons.reasoning_validation.is_empty(){return Ok((0.0,0.0));}
    let mut sum=0.0f32;
    let mut wins=0usize;
    for (context,good,bad) in lessons.reasoning_validation.iter().take(limit){
        let (loss,win)=brain.reasoning_margin(context,good,bad)?;
        if !loss.is_finite(){return Err("Non-finite reasoning validation loss".into());}
        sum+=loss;if win{wins+=1;}
    }
    let n=lessons.reasoning_validation.len().min(limit);
    Ok((sum/n as f32,wins as f32/n as f32))
}
/// Category-aware unit scheduling. Keep mini-batches homogeneous, and rotate
/// objectives so a long Dialogue block cannot drown out one Reasoning block.
fn objective_batches(lesson:&PreparedLessons,epoch:u32)->Vec<Vec<usize>>{
    let order=training_order(&lesson.train,epoch,&lesson.mode);
    if lesson.mode!="unified"{return order.chunks(BATCH).map(|c|c.to_vec()).collect();}
    let mut queues=vec![Vec::<Vec<usize>>::new();5];
    for id in order{
        let kind=lesson.train_kinds[id] as usize;
        queues[kind].push(vec![id]);
    }
    for queue in &mut queues{
        let flattened=queue.iter().map(|x|x[0]).collect::<Vec<_>>();
        *queue=flattened.chunks(BATCH).map(|c|c.to_vec()).collect();
    }
    let mut result=Vec::new();
    let mut cursor=[0usize;5];
    loop{
        let mut progress=false;
        for category in 0..5{
            if cursor[category]<queues[category].len(){
                result.push(queues[category][cursor[category]].clone());
                cursor[category]+=1;progress=true;
            }
        }
        if !progress{break;}
    }
    result
}
/// The beginning and learned end-of-turn are rare relative to middle reply
/// characters. Give these *real teacher response targets* an auxiliary
/// optimizer batch every eighth regular batch, balanced by objective.
/// This is training only: no greeting or completion is hardcoded at inference.
fn reply_boundary_batches(lesson:&PreparedLessons,epoch:u32,
    new_batches:usize)->Vec<Vec<usize>>{
    if lesson.mode!="unified" || new_batches==0 {return Vec::new();}
    let mut first=vec![Vec::<usize>::new();4];
    let mut ends=vec![Vec::<usize>::new();4];
    for (i,((context,target),&kind)) in lesson.train.iter()
        .zip(&lesson.train_kinds).enumerate(){
        let k=kind as usize;
        if k>=4 {continue;} // Explanatory teacher text is never a Nova reply.
        if context.ends_with(b"\nN: "){first[k].push(i);}
        if *target==b'\n' && context.windows(4).any(|w|w==b"\nN: "){
            ends[k].push(i);
        }
    }
    let available=(0..4).filter(|&k|!first[k].is_empty()&&!ends[k].is_empty())
        .collect::<Vec<_>>();
    if available.is_empty(){return Vec::new();}
    let slots=(new_batches+7)/8;
    let mut result=Vec::with_capacity(slots);
    for slot in 0..slots{
        let k=available[(slot+epoch as usize-1)%available.len()];
        let first_id=first[k][(slot+epoch as usize-1)%first[k].len()];
        let end_id=ends[k][(slot+epoch as usize-1)%ends[k].len()];
        result.push(vec![first_id,end_id]);
    }
    result
}
/// Deterministic balanced replay, with round-robin categories AND archived
/// lesson sources. One giant/old lesson cannot monopolize a category simply
/// because it contributes more character batches. Never uses held-out rows.
/// Even a one-batch new lesson receives one prior-skill rehearsal batch.
fn balanced_replay_schedule(previous:&[PreparedLessons],epoch:u32,
    new_batches:usize)->Vec<(usize,Vec<usize>,u8)>{
    if previous.is_empty()||new_batches==0{return Vec::new();}
    let mut sources=vec![Vec::<(usize,Vec<Vec<usize>>)>::new();5];
    for (source,old) in previous.iter().enumerate(){
        let mut by_category=vec![Vec::<Vec<usize>>::new();5];
        for ids in objective_batches(old,epoch){
            if ids.is_empty(){continue;}
            let kind=if old.mode=="unified"{old.train_kinds[ids[0]] as usize}
                else if old.mode=="conversation"{2}
                else if old.mode=="reasoning"{3}
                else if old.mode=="text"{4}else{0};
            by_category[kind].push(ids);
        }
        for (kind,batches) in by_category.into_iter().enumerate(){
            if !batches.is_empty(){sources[kind].push((source,batches));}
        }
    }
    let categories=(0..5).filter(|&k|!sources[k].is_empty()).collect::<Vec<_>>();
    if categories.is_empty(){return Vec::new();}
    let slots=(new_batches+2)/3; // ceil; last partial chunk gets rehearsal.
    let mut per_category=[0usize;5];
    let mut result=Vec::with_capacity(slots);
    for slot in 0..slots {
        let category=categories[(slot+epoch as usize-1)%categories.len()];
        let curs=per_category[category];
        let available=&sources[category];
        let (source,batches)=&available[(curs+epoch as usize-1)%available.len()];
        let batch=batches[(curs+epoch as usize-1)%batches.len()].clone();
        result.push((*source,batch,category as u8));
        per_category[category]+=1;
    }
    result
}
/// Score precisely the requested mixed held-out rows, including memory inputs.
fn mixed_validation(brain:&HybridBrain,lesson:&PreparedLessons,indices:&[usize])
    ->Result<(f32,f32,[Option<f32>;5]),String>{
    let mut loss=[0f64;5];let mut count=[0usize;5];let mut correct=0usize;
    for &i in indices{
        let (context,target)=&lesson.validation[i];
        let kind=lesson.validation_kinds[i] as usize;
        let memory=if kind==neural_nova::unified::CONVERSATION as usize{
            &lesson.memory_validation[i][..]
        }else{&[][..]};
        let p=brain.predict_with_memory(context,256,memory)?;
        loss[kind]-=p[*target as usize].max(1e-30).ln() as f64;
        count[kind]+=1;
        if p.iter().enumerate().max_by(|a,b|a.1.partial_cmp(b.1)
            .unwrap_or(std::cmp::Ordering::Equal)).map(|x|x.0)==Some(*target as usize){
            correct+=1;
        }
    }
    let per=std::array::from_fn(|i|if count[i]>0{
        Some((loss[i]/count[i] as f64) as f32)
    }else{None});
    let average=per.iter().flatten().sum::<f32>()/
        per.iter().flatten().count().max(1) as f32;
    Ok((average,correct as f32/indices.len().max(1) as f32,per))
}
/// Read-only exact *generated* response evaluation, including prior turns.
/// Separates whole-answer competency from next-character teacher forcing.
fn generated_heldout(brain:&HybridBrain,lesson:&PreparedLessons)
    ->Result<[Option<f32>;4],String>{
    let mut counts=[0usize;4];let mut correct=[0usize;4];
    for (prefix,expected,memory,kind) in &lesson.heldout_replies{
        let category=*kind as usize;
        if category>=4 || counts[category]>=2{continue;}
        counts[category]+=1;
        let mut prompt=prefix.clone();
        let mut output=Vec::new();
        let mut finished=false;
        for _ in 0..expected.len().min(96)+1{
            let p=brain.predict_with_memory(&prompt,256,memory)?;
            let next=p.iter().enumerate().max_by(|a,b|a.1.partial_cmp(b.1)
                .unwrap_or(std::cmp::Ordering::Equal))
                .ok_or("No neural prediction")?.0 as u8;
            if next==b'\n'{finished=true;break;}
            output.push(next);prompt.push(next);
        }
        if finished&&output==*expected{correct[category]+=1;}
    }
    Ok(std::array::from_fn(|i|if counts[i]==0{None}
        else{Some(correct[i] as f32/counts[i] as f32)}))
}
/// Teacher-side *autonomous* validation: Nova generates its own preceding
/// characters, instead of receiving the correct prefix at every position.
/// Only held-out examples are scored. Targets NEVER update neural weights.
/// One held-out reply per category, max 12 generated characters each, bounded
/// for local CPU training. A fully correct response is counted only if Nova
/// generates its learned newline at the correct place.
#[derive(Clone,Copy,Debug,Serialize)]
#[serde(rename_all="camelCase")]
struct AutonomousValidation {
    self_conditioned_loss:f32,
    first_character_accuracy:f32,
    generated_prefix_accuracy:f32,
    complete_answers_correct:usize,
    complete_answers_checked:usize,
    represented_categories:usize,
    max_characters_per_probe:usize,
}
fn autonomous_validation(brain:&HybridBrain,lessons:&PreparedLessons)
    ->Result<Option<AutonomousValidation>,String>{
    autonomous_validation_replies(brain,&lessons.heldout_replies)
}
/// Exact same neural rollout for the current lesson and older archived heldout
/// lessons. Baseline and candidate MUST use identical fixed prompts.
fn autonomous_validation_replies(brain:&HybridBrain,replies:&[neural_nova::trainer::HeldoutReply])
    ->Result<Option<AutonomousValidation>,String>{
    const MAX_ROLLOUT:usize=12;
    const EARLY_END_MISSING_PENALTY:f32=6.0;
    let mut categories=[false;4];
    let mut total_loss=0.0f32;
    let mut total_chars=0usize;
    let mut correct_chars=0usize;
    let mut correct_first=0usize;
    let mut complete=0usize;
    let mut full_correct=0usize;
    for (prompt,reply,memory,kind) in replies {
        let k=*kind as usize;
        if k>=4 || categories[k] {continue;}
        categories[k]=true;
        let target=reply.iter().copied().chain(std::iter::once(b'\n'))
            .take(MAX_ROLLOUT).collect::<Vec<_>>();
        if target.is_empty(){continue;}
        let is_complete=reply.len()+1<=MAX_ROLLOUT;
        if is_complete{complete+=1;}
        total_chars+=target.len();
        let mut history=prompt.clone();
        let mut generated=Vec::new();
        let mut ended=false;
        for (i,&expected) in target.iter().enumerate(){
            let probabilities=brain.predict_with_memory(&history,256,memory)?;
            let guess=probabilities.iter().enumerate().max_by(|a,b|
                a.1.total_cmp(b.1)).ok_or("Empty neural output distribution")?.0 as u8;
            total_loss-=probabilities[expected as usize].max(1e-30).ln();
            if guess==expected{correct_chars+=1;}
            if i==0 && guess==expected{correct_first+=1;}
            if guess==b'\n' {
                ended=true;
                // Chat really terminates on newline; missing target positions
                // are counted as missed, not silently removed from averages.
                total_loss+=EARLY_END_MISSING_PENALTY*(target.len()-i-1) as f32;
                break;
            }
            generated.push(guess);
            history.push(guess);
        }
        if is_complete && ended && generated==*reply{full_correct+=1;}
    }
    let tested=categories.iter().filter(|&&x|x).count();
    if tested==0{return Ok(None);}
    let score=AutonomousValidation{
        self_conditioned_loss:total_loss/total_chars.max(1) as f32,
        first_character_accuracy:correct_first as f32/tested as f32,
        generated_prefix_accuracy:correct_chars as f32/total_chars.max(1) as f32,
        complete_answers_correct:full_correct,
        complete_answers_checked:complete,
        represented_categories:tested,
        max_characters_per_probe:MAX_ROLLOUT,
    };
    if !score.self_conditioned_loss.is_finite(){
        return Err("Non-finite autonomous held-out loss".into());
    }
    Ok(Some(score))
}
/// Fixed, bounded PREVIOUS-LESSON heldout panels for genuine conversation
/// retention. Keep newest and oldest sampled answers by category so an
/// improved current-lesson loss cannot hide loss of earlier complete replies.
/// Each panel holds at most four independent withheld replies; never trained.
/// Stable identity for the held-out questions in this lesson; it allows a
/// human reviewer to tell whether a previous screenshot used the SAME test
/// set. The fingerprint is teacher diagnostics, never an inference feature.
fn heldout_panel_id(lessons:&PreparedLessons)->String{
    let mut hash=0xcbf29ce484222325u64;
    for (prompt,expected,memory,kind) in &lessons.heldout_replies{
        for b in prompt.iter().copied().chain(expected.iter().copied())
            .chain(std::iter::once(*kind)){
            hash^=b as u64;
            hash=hash.wrapping_mul(0x100000001b3);
        }
        for turn in memory {
            for &b in turn.as_bytes(){
                hash^=b as u64;
                hash=hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("{hash:016x}")
}
fn archived_autonomous_panels(previous:&[PreparedLessons])
    ->Vec<Vec<neural_nova::trainer::HeldoutReply>>{
    if previous.is_empty(){return Vec::new();}
    let collect=|recent_first:bool|{
        let mut represented=[false;4];
        let mut panel=Vec::new();
        let ordered=if recent_first{
            previous.iter().rev().collect::<Vec<_>>()
        }else{previous.iter().collect::<Vec<_>>()};
        for plan in ordered{
            for entry in &plan.heldout_replies{
                let kind=entry.3 as usize;
                if kind<4&&!represented[kind]{
                    panel.push(entry.clone());
                    represented[kind]=true;
                }
            }
            if represented.iter().all(|&x|x){break;}
        }
        panel
    };
    let newest=collect(true);
    if newest.is_empty(){return Vec::new();}
    let oldest=collect(false);
    if oldest!=newest{vec![newest,oldest]}else{vec![newest]}
}
fn archived_autonomous_scores(
    brain:&HybridBrain,panels:&[Vec<neural_nova::trainer::HeldoutReply>],
)->Result<Vec<AutonomousValidation>,String>{
    panels.iter().map(|panel|autonomous_validation_replies(brain,panel)?
        .ok_or("Empty archived independent-response retention panel".into())).collect()
}
fn archived_autonomous_retained(before:&[AutonomousValidation],
    candidate:&[AutonomousValidation])->bool{
    before.len()==candidate.len()&&before.iter().zip(candidate).all(|(old,new)|{
        old.represented_categories==new.represented_categories &&
        old.complete_answers_checked==new.complete_answers_checked &&
        new.complete_answers_correct>=old.complete_answers_correct &&
        new.first_character_accuracy+1e-6>=old.first_character_accuracy &&
        new.generated_prefix_accuracy+0.05>=old.generated_prefix_accuracy &&
        new.self_conditioned_loss<=old.self_conditioned_loss+0.05
    })
}
/// The numeric lesson baseline is fixed for the ENTIRE training session.
/// Without this, several selected epochs can each spend a fresh +0.08
/// tolerance and ratchet the "best" heldout loss arbitrarily far above
/// the actual starting checkpoint.
fn stays_near_original_baseline(
    lesson_baseline_loss:f32,selected_loss:f32,candidate_loss:f32
)->bool{
    candidate_loss.is_finite() &&
    candidate_loss<=lesson_baseline_loss+AUTONOMOUS_TEACHER_TOLERANCE &&
    candidate_loss<=selected_loss+AUTONOMOUS_TEACHER_TOLERANCE
}
/// A new checkpoint must improve the already-established held-out loss,
/// avoid regressing observed autonomous replies, AND improve the combined
/// held-out criterion. When neither model can produce a complete reply yet,
/// smooth self-conditioned likelihood can still show learning progress.
fn autonomous_selection_ok(
    previous:Option<AutonomousValidation>,
    candidate:Option<AutonomousValidation>,
    previous_heldout:f32,candidate_heldout:f32,
)->bool {
    match (previous,candidate) {
        (Some(old),Some(new))=>{
            // A real improvement in independent generation can justify a
            // *small* increase in teacher-forced loss. Otherwise this would
            // remain a next-character-only selector with a rollout veto.
            // The joint score and existing retention check still must win.
            let old_combined=previous_heldout+0.35*old.self_conditioned_loss;
            let new_combined=candidate_heldout+0.35*new.self_conditioned_loss;
            candidate_heldout<=previous_heldout+AUTONOMOUS_TEACHER_TOLERANCE &&
            new.self_conditioned_loss<=old.self_conditioned_loss+0.05 &&
            new.first_character_accuracy+1e-6>=old.first_character_accuracy &&
            new.complete_answers_correct>=old.complete_answers_correct &&
            new_combined+MIN_IMPROVEMENT<old_combined
        },
        (None,None)=>candidate_heldout+MIN_IMPROVEMENT<previous_heldout,
        _=>false,
    }
}
/// Genuine neural first-character distribution, not a guessed cause of Yello.
fn greeting_first_character_bias(brain:&HybridBrain)->Option<serde_json::Value>{
    let p=brain.predict(b"U: hello\nN: ").ok()?;
    let top=p.iter().enumerate().max_by(|a,b|a.1.partial_cmp(b.1)
        .unwrap_or(std::cmp::Ordering::Equal))?.0;
    Some(serde_json::json!({
        "topAscii":top,"pY":p[b'Y' as usize],"pH":p[b'H' as usize],
        "pI":p[b'I' as usize],
        "entropy":-p.iter().filter(|&&x|x>0.0).map(|&x|x*x.ln()).sum::<f32>(),
    }))
}
/// Read-only teacher metric: answer likelihoods for the SAME held-out
/// contextual question, comparing full neural cooperation against suppressing
/// dense features or iterative exchange. No correct conclusion is calculated
/// here and no answer key is ever supplied to model inference.
fn heldout_context_ablation(
    brain:&HybridBrain,lessons:&PreparedLessons,
)->Result<Option<serde_json::Value>,String>{
    let Some((prompt,good,bad))=lessons.understanding_validation.first() else {
        return Ok(None);
    };
    let score=|answer:&[u8],dense_off:bool,exchange_off:bool|
        ->Result<f32,String>{
        let mut history=prompt.clone();
        let mut logp=0.0f32;
        for target in answer.iter().copied().chain(std::iter::once(b'\n')){
            let probabilities=brain.predict_ablation_with_memory(
                &history,256,&[],dense_off,exchange_off)?;
            logp+=probabilities[target as usize].max(1.0e-30).ln();
            history.push(target);
            if history.len()>512{history.drain(..history.len()-512);}
        }
        Ok(logp/(answer.len()+1) as f32)
    };
    let mut margins=[0.0f32;3];
    for (i,(dense_off,exchange_off)) in
        [(false,false),(true,false),(false,true)].iter().copied().enumerate(){
        margins[i]=score(good,dense_off,exchange_off)?-
            score(bad,dense_off,exchange_off)?;
    }
    Ok(Some(serde_json::json!({
        "samples":1,"fullAnswerMargin":margins[0],
        "denseSuppressedMargin":margins[1],
        "exchangeSuppressedMargin":margins[2],
        "denseContribution":margins[0]-margins[1],
        "iterativeContribution":margins[0]-margins[2],
        "teacherOnlyHeldoutAnswers":true,
        "note":"One read-only heldout comparison; differences are not evidence of general reasoning by themselves"
    })))
}
/// Teacher-only response initial distribution. This diagnoses a bias such as
/// 'Yello!' from actual supervised targets, not invented input/answer rules.
fn supervised_initials(lesson:&PreparedLessons)->serde_json::Value{
    let mut target_totals=[0usize;5];
    let mut starts_y=[0usize;5];
    let mut starts_h=[0usize;5];
    for ((history,target),kind) in lesson.train.iter().zip(&lesson.train_kinds){
        let k=*kind as usize;
        if k>=5 || !history.ends_with(b"\nN: "){continue;}
        target_totals[k]+=1;
        if *target==b'Y'{starts_y[k]+=1;}
        if *target==b'H'{starts_h[k]+=1;}
    }
    serde_json::json!({"firstResponseTargetsByObjective":target_totals,
        "startsWithYByObjective":starts_y,
        "startsWithHByObjective":starts_h,
        "explanationsAreNotReplyTargets":true})
}
fn understanding_validation(brain:&HybridBrain,lessons:&PreparedLessons)
    ->Result<(f32,f32),String>{
    understanding_validation_limited(brain,lessons,16)
}
fn understanding_validation_limited(brain:&HybridBrain,lessons:&PreparedLessons,limit:usize)
    ->Result<(f32,f32),String>{
    if lessons.understanding_validation.is_empty(){return Ok((0.0,0.0));}
    let mut sum=0.0f32;let mut wins=0usize;
    for (context,correct,wrong) in lessons.understanding_validation.iter().take(limit){
        let (loss,win)=brain.reasoning_margin(context,correct,wrong)?;
        if !loss.is_finite(){return Err("Non-finite contextual understanding loss".into());}
        sum+=loss;wins+=usize::from(win);
    }
    let n=lessons.understanding_validation.len().min(limit);
    Ok((sum/n as f32,wins as f32/n as f32))
}
fn current_validation(brain:&HybridBrain,lessons:&PreparedLessons)
    ->Result<(f32,f32,f32,f32),String>{
    let (next_loss,accuracy)=if lessons.mode=="unified"{
        let ids=(0..lessons.validation.len()).collect::<Vec<_>>();
        let (loss,correct,_)=mixed_validation(brain,lessons,&ids)?;
        (loss,correct)
    }else if lessons.mode=="conversation"{
        evaluate_memory(brain,&lessons.validation,&lessons.memory_validation)?
    }else{brain.evaluate(&lessons.validation)?};
    let (ranking,ranking_acc)=reasoning_validation(brain,lessons)?;
    let context=if lessons.mode=="unified"{
        0.2*understanding_validation(brain,lessons)?.0
    }else{0.0};
    let selected_loss=if lessons.mode=="reasoning"||lessons.mode=="unified"{
        next_loss+0.4*ranking+context
    }else{next_loss};
    Ok((selected_loss,accuracy,ranking,ranking_acc))
}
/// Candidate-only training. Four complete brain CLONES begin each epoch from
/// the same accepted checkpoint and receive the same ordered samples. Only the
/// allowed optimizer masks change. Every candidate retains its own Adam state.
/// Rejected branches NEVER modify live Nova or on-disk neural checkpoints.
fn foundation_train_inner(
    app:&AppHandle,state:&HybridState,lessons:&PreparedLessons,
    epochs:u32,adaptive_learning_rate:bool,early_stopping:bool,
    starting_lr:f32,patience:u32,retention:&[TestCase],
    previous:&[PreparedLessons],rehearsal_enabled:bool,
    loss_tolerance:f32,observer_id:&str,foundation_done:u8,
)->Result<(bool,bool,u32),String>{
    let started=Instant::now();
    let (mut best_brain,mut best_loss,mut best_accuracy,initially_trained)={
        let s=lock(state)?;
        let (loss,accuracy,_,_)=current_validation(&s.brain,lessons)?;
        if !loss.is_finite(){return Err("Non-finite foundation starting loss".into());}
        (s.brain.clone(),loss,accuracy,s.trained)
    };
    let baseline_loss=best_loss;
    let mut best_autonomous=autonomous_validation(&best_brain,lessons)?;
    let prior_baseline=previous_validation_loss(&best_brain,previous)?;
    let teacher_baseline=if retention.is_empty(){None}
        else{Some(retention_accuracy(&best_brain,retention)?)};
    let archived_panels=archived_autonomous_panels(previous);
    let archived_before=archived_autonomous_scores(&best_brain,&archived_panels)?;
    let practice={
        let empty=Vec::<String>::new();
        observer::training_replies(lessons.train.iter().enumerate().map(|(i,ex)|(
            ex,lessons.memory_train.get(i).unwrap_or(&empty),
            lessons.train_kinds.get(i).copied().unwrap_or(0)
        )))
    };
    let contexts=lessons.train.iter().filter(|(p,_)|p.ends_with(b"\nN: "))
        .map(|(p,_)|p.clone()).collect::<Vec<_>>();
    let base_observation=observer::capture(&best_brain,"foundation",&lessons.mode,0,
        &practice,&lessons.heldout_replies,&contexts,serde_json::json!({
            "baseline":true,"heldoutLoss":best_loss,
            "foundationLesson":foundation_done+1,
            "candidates":["Dense","Transformer","Interconnect","Joint"]
        }));
    let obs_err=base_observation.and_then(|v|observer::append(app,observer_id,&v)).err();
    let _=app.emit("nova-observer-epoch",serde_json::json!({
        "sessionId":observer_id,"epoch":0,"error":obs_err
    }));
    let mut best_epoch=0u32;
    let mut done_samples=0usize;
    let mut cancelled=false;
    let mut stopped=false;
    let mut schedule=ScheduleState::new(starting_lr,ScheduleConfig{
        adaptive_learning_rate,early_stopping,patience
    });
    let focus_options=[
        ("Dense",TrainingFocus::Dense),
        ("Transformer",TrainingFocus::Transformer),
        ("Interconnect",TrainingFocus::Interconnect),
        ("Joint",TrainingFocus::Joint)
    ];
    for epoch in 1..=epochs{
        if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
        let lr=schedule.learning_rate;
        let batch_ids=objective_batches(lessons,epoch);
        let prior_batches=if rehearsal_enabled{
            balanced_replay_schedule(previous,epoch,batch_ids.len())
        }else{Vec::new()};
        let boundary=reply_boundary_batches(lessons,epoch,batch_ids.len());
        let base=best_brain.clone();
        let mut proposals=Vec::new();
        let mut winner:Option<(HybridBrain,f32,f32,Option<AutonomousValidation>,
            String,f32,f32,Option<f32>)>=None;
        let base_score=best_loss+best_autonomous.map(|x|0.35*x.self_conditioned_loss)
            .unwrap_or(0.0);
        let mut previous_score=base_score;
        for (focus_name,mask) in focus_options{
            if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
            let mut brain=base.clone();
            let mut training_loss=0.0f64;
            let mut learned=0usize;
            let mut boundary_index=0usize;
            let mut replay_index=0usize;
            {
                let mut s=lock(state)?;
                s.epoch=epoch;s.stage=format!(
                    "Foundation {}/5 · epoch {}/{} · evaluating {} candidate",
                    foundation_done+1,epoch,epochs,focus_name);
                s.learning_rate=lr;
            }
            for (batch_index,ids) in batch_ids.iter().enumerate(){
                if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
                let kind=if lessons.mode=="unified"{lessons.train_kinds[ids[0]]}else{
                    match lessons.mode.as_str(){"conversation"=>2,"reasoning"=>3,
                        "text"=>4,_=>0}
                };
                let batch=ids.iter().map(|&i|lessons.train[i].clone())
                    .collect::<Vec<_>>();
                let loss=if kind==2{
                    let memory=ids.iter().map(|&i|(
                        lessons.train[i].0.clone(),lessons.train[i].1,
                        lessons.memory_train[i].clone()
                    )).collect::<Vec<_>>();
                    brain.train_memory_batch_focused(&memory,lr,mask)?
                }else{brain.train_batch_focused(&batch,lr,mask)?};
                if !loss.is_finite(){return Err("Non-finite foundation batch loss".into());}
                training_loss+=loss as f64*ids.len() as f64;learned+=ids.len();
                if !lessons.reasoning_train.is_empty()&&kind==3&&
                    ((batch_index+1)%4==0||batch_index+1==batch_ids.len()){
                    let i=(batch_index/4+epoch as usize-1)%lessons.reasoning_train.len();
                    brain.train_preference_batch_focused(
                        &[lessons.reasoning_train[i].clone()],lr,mask)?;
                }
                if !lessons.understanding_train.is_empty()&&kind==1&&
                    ((batch_index+1)%4==0||batch_index+1==batch_ids.len()){
                    let i=(batch_index/4+epoch as usize-1)%lessons.understanding_train.len();
                    brain.train_preference_batch_focused(
                        &[lessons.understanding_train[i].clone()],
                        (lr*0.5).max(0.0001),mask)?;
                }
                if !lessons.evidence_train.is_empty()&&kind==3&&
                    ((batch_index+1)%4==0||batch_index+1==batch_ids.len()){
                    let i=(batch_index/4+epoch as usize-1)%lessons.evidence_train.len();
                    brain.train_evidence_batch_focused(
                        &[lessons.evidence_train[i].clone()],
                        (lr*0.5).max(0.0001),mask)?;
                }
                if (batch_index+1)%3==0||batch_index+1==batch_ids.len(){
                    if replay_index<prior_batches.len(){
                        let (source,old_ids,objective)=&prior_batches[replay_index];
                        replay_index+=1;
                        let old=&previous[*source];
                        let examples=old_ids.iter().map(|&i|old.train[i].clone())
                            .collect::<Vec<_>>();
                        let replay_loss=if old.mode=="conversation"||*objective==2{
                            let memory=old_ids.iter().map(|&i|(
                                old.train[i].0.clone(),old.train[i].1,
                                old.memory_train[i].clone())).collect::<Vec<_>>();
                            brain.train_memory_batch_focused(&memory,lr,mask)?
                        }else{brain.train_batch_focused(&examples,lr,mask)?};
                        if !replay_loss.is_finite(){
                            return Err("Non-finite foundation rehearsal loss".into());
                        }
                        training_loss+=replay_loss as f64*old_ids.len() as f64;
                        learned+=old_ids.len();
                        done_samples+=old_ids.len();
                    }
                }
                if lessons.mode=="unified"&&
                    ((batch_index+1)%8==0||batch_index+1==batch_ids.len())&&
                    boundary_index<boundary.len(){
                    let ids=&boundary[boundary_index];boundary_index+=1;
                    let kind=lessons.train_kinds[ids[0]];
                    let focused=ids.iter().map(|&i|lessons.train[i].clone())
                        .collect::<Vec<_>>();
                    let output=if kind==2{
                        let m=ids.iter().map(|&i|(
                            lessons.train[i].0.clone(),lessons.train[i].1,
                            lessons.memory_train[i].clone())).collect::<Vec<_>>();
                        brain.train_memory_batch_focused(&m,lr,mask)?
                    }else{brain.train_batch_focused(&focused,lr,mask)?};
                    training_loss+=output as f64*ids.len() as f64;
                    learned+=ids.len();
                    done_samples+=ids.len();
                }
                done_samples+=ids.len();
                if done_samples%64<ids.len(){
                    let mut s=lock(state)?;
                    s.completed=done_samples;s.total=s.total.max(done_samples);
                    s.tokens_per_second=done_samples as f32/
                        started.elapsed().as_secs_f32().max(0.001);
                    s.train_loss=Some(loss);
                    let _=app.emit("hybrid-progress",serde_json::json!({
                        "completed":done_samples,"total":s.total,
                        "epoch":epoch,"epochs":epochs,
                        "loss":loss,"stage":s.stage,"learningRate":lr,
                        "tokensPerSecond":s.tokens_per_second
                    }));
                }
            }
            if cancelled{break;}
            if learned==0{return Err("Foundation lesson produced no training samples".into());}
            // IMPORTANT: All candidates are compared using the SAME held-out
            // and earlier-skill panels. Teacher answers are never trained on.
            let (loss,acc,_,reason_accuracy)=current_validation(&brain,lessons)?;
            let autonomous=autonomous_validation(&brain,lessons)?;
            let previous_loss=previous_validation_loss(&brain,previous)?;
            let teacher=if retention.is_empty(){None}
                else{Some(retention_accuracy(&brain,retention)?)};
            let retained=rehearsal::eligible_checkpoint(previous_loss,prior_baseline,
                loss_tolerance,teacher,teacher_baseline);
            let archived=archived_autonomous_scores(&brain,&archived_panels)?;
            let archived_ok=archived_autonomous_retained(&archived_before,&archived);
            let valid=retained&&archived_ok&&
                stays_near_original_baseline(baseline_loss,best_loss,loss)&&
                autonomous_selection_ok(best_autonomous,autonomous,best_loss,loss);
            let score=loss+autonomous.map(|v|0.35*v.self_conditioned_loss)
                .unwrap_or(0.0);
            proposals.push(serde_json::json!({
                "pathway":focus_name,"eligible":valid,"heldoutLoss":loss,
                "nextCharacterAccuracy":acc,"autonomous":autonomous,
                "reasoningPreferenceAccuracy":reason_accuracy,
                "previousHeldoutLoss":previous_loss,"teacherAccuracy":teacher,
                "retentionAccepted":retained,"archivedRepliesRetained":archived_ok,
                "jointScore":score,
                "trainingLoss":training_loss/learned as f64,
                "modelStep":brain.transformer.step,
                "fingerprint":diag::weight_digest(&brain)
            }));
            // No blind neuron splicing or independently selected optimizer
            // moments. Accept the WHOLE candidate, including all Adam states.
            if valid&&score+MIN_IMPROVEMENT<previous_score{
                previous_score=score;
                winner=Some((brain,loss,acc,autonomous,focus_name.into(),
                    (training_loss/learned as f64) as f32,reason_accuracy,previous_loss));
            }
        }
        if cancelled{break;}
        let selected=winner.is_some();
        let (selected_name,epoch_train,reason_accuracy,prior_loss)=
            if let Some((brain,loss,acc,auto,name,train_score,reason,prior))=winner{
                best_brain=brain;best_loss=loss;best_accuracy=acc;
                best_autonomous=auto;best_epoch=epoch;
                (name,train_score,reason,prior)
            }else{
                ("None — retained previous complete checkpoint".to_string(),
                 f32::NAN,0.0,prior_baseline)
            };
        {
            let mut s=lock(state)?;
            s.best_epoch=best_epoch;s.best_heldout_loss=Some(best_loss);
            s.heldout_loss=Some(best_loss);s.heldout_accuracy=Some(best_accuracy);
            s.stage=format!("Foundation · epoch {} · {}",epoch,selected_name);
            s.train_loss=if epoch_train.is_finite(){Some(epoch_train)}else{None};
        }
        if diag::enabled(&app.state::<Diagnostics>()){
            diag::record(&app.state::<Diagnostics>(),"foundation_candidates",
                serde_json::json!({"foundationLesson":foundation_done+1,
                    "epoch":epoch,"startingScore":base_score,
                    "selected":selected_name,"candidates":proposals}));
        }
        let report=observer::capture(&best_brain,"foundation",&lessons.mode,epoch,
            &practice,&lessons.heldout_replies,&contexts,serde_json::json!({
                "foundationLesson":foundation_done+1,
                "selected":selected,"selectedEpoch":best_epoch,
                "selectionReason":format!("Foundation candidate: {}",selected_name),
                "candidateScores":proposals,
                "heldoutLoss":best_loss,"heldoutAccuracy":best_accuracy,
                "priorHeldoutLoss":prior_loss,
                "reasoningPreferenceAccuracy":reason_accuracy
            }));
        let err=report.and_then(|v|observer::append(app,observer_id,&v)).err();
        let _=app.emit("nova-observer-epoch",serde_json::json!({
            "sessionId":observer_id,"epoch":epoch,"error":err
        }));
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "foundation":true,"foundationLesson":foundation_done+1,
            "epoch":epoch,"epochs":epochs,
            "selectionReason":format!("Foundation candidate: {}",selected_name),
            "candidateScores":proposals,
            "validationLoss":best_loss,"validationAccuracy":best_accuracy,
            "bestLoss":best_loss,"bestEpoch":best_epoch,
            "improved":selected,"trainLoss":epoch_train,
            "autonomousHeldout":best_autonomous,
            "selectedAutonomousHeldout":best_autonomous,
            "reasoningRankingAccuracy":reason_accuracy,
            "previousHeldoutLoss":prior_loss,
            "tokensPerSecond":done_samples as f64/started.elapsed().as_secs_f64().max(0.001)
        }));
        if schedule.observe_validation(selected){stopped=true;break;}
    }
    // Live Nova was not exposed to ANY candidate before final acceptance.
    // A cancellation can keep an earlier accepted epoch, matching normal trainer.
    {
        let mut s=lock(state)?;
        s.brain=best_brain;s.trained=initially_trained||best_epoch>0;
        s.best_epoch=best_epoch;s.best_heldout_loss=Some(best_loss);
        s.heldout_loss=Some(best_loss);s.heldout_accuracy=Some(best_accuracy);
        s.completed=done_samples;
        s.early_stopped=stopped;s.learning_rate=schedule.learning_rate;
        s.tokens_per_second=done_samples as f32/
            started.elapsed().as_secs_f32().max(0.001);
        s.stage=if cancelled{"Foundation stopped; best checkpoint restored"}
            else if stopped{"Foundation early stopped; best checkpoint restored"}
            else{"Foundation complete; best checkpoint restored"}.into();
        if best_epoch>0{s.brain.save(path(app)?)?;}
    }
    #[cfg(target_os="android")]
    training_notification(app,format!(
        "Foundation lesson {}/5 • selected epoch {} of {}",
        foundation_done+1,best_epoch,epochs),true);
    Ok((cancelled,stopped,best_epoch))
}
fn train_inner(
    app:&AppHandle,state:&HybridState,lessons:&PreparedLessons,
    epochs:u32,adaptive_learning_rate:bool,early_stopping:bool,
    starting_lr:f32,patience:u32,retention:&[TestCase],
    previous:&[PreparedLessons],rehearsal_enabled:bool,
    protect_previous:bool,loss_tolerance:f32,observer_id:&str,
)->Result<(bool,bool,u32),String>{
    let train:&[Example]=&lessons.train;
    let started=Instant::now();
    let (mut best_brain,mut best_loss,mut best_accuracy,initially_trained,
        mut best_autonomous)={
        let mut s=lock(state)?;
        let (loss,accuracy,ranking_loss,ranking_accuracy)=current_validation(&s.brain,lessons)?;
        if !loss.is_finite(){return Err("Non-finite baseline validation loss".into());}
        let autonomous=if lessons.mode=="unified"{
            autonomous_validation(&s.brain,lessons)?
        }else{None};
        s.heldout_loss=Some(loss);s.heldout_accuracy=Some(accuracy);
        s.best_heldout_loss=Some(loss);
        if diag::enabled(&app.state::<Diagnostics>()){
            diag::record(&app.state::<Diagnostics>(),"lesson_validation_baseline",serde_json::json!({
                "loss":loss,"nextCharacterAccuracy":accuracy,
                "autonomousHeldout":autonomous,
                "reasoningRankingLoss":ranking_loss,"reasoningRankingAccuracy":ranking_accuracy,
                "generatedHeldoutExactByCategory":if lessons.mode=="unified"{
                    generated_heldout(&s.brain,lessons).ok()
                }else{None},
                "greetingFirstCharacterBias":greeting_first_character_bias(&s.brain),
                "understandingContrast":if lessons.mode=="unified"{
                    understanding_validation(&s.brain,lessons).ok()
                }else{None},
                "heldoutContextAblation":if lessons.mode=="unified"{
                    heldout_context_ablation(&s.brain,lessons).ok().flatten()
                }else{None},
                "categoryLoss":if lessons.mode=="unified"{
                    mixed_validation(&s.brain,lessons,
                        &(0..lessons.validation.len()).collect::<Vec<_>>())
                        .ok().map(|x|x.2)
                }else{None},
                "step":s.brain.transformer.step,"wordUnits":s.brain.tokenizer.words.len(),
                "weightsFingerprint":diag::weight_digest(&s.brain),
                "selectionUsesOnlyCurrentLessonValidation":true,
                "checkpointSelectionIncludesIndependentGeneration":lessons.mode=="unified",
            }));
        }
        (s.brain.clone(),loss,accuracy,s.trained,autonomous)
    };
    // Preserve an immutable benchmark for THIS lesson across every epoch.
    // "Selected" cannot drift > tolerance from the original lesson baseline.
    let lesson_baseline_loss=best_loss;
    let lesson_baseline_autonomous=best_autonomous;
    let current_heldout_panel_id=heldout_panel_id(lessons);
    // Independently generate prior archived responses under a fixed set of
    // held-out prompts. The archived panels are read-only: the learner never
    // receives their answers during the new lesson.
    let prior_autonomous_panels=if protect_previous{
        archived_autonomous_panels(previous)
    }else{Vec::new()};
    let prior_autonomous_baseline=archived_autonomous_scores(
        &best_brain,&prior_autonomous_panels)?;
    // Diagnostic baseline is measured after existing vocabulary expansion.
    // The dedicated tokenizer-only A/B command measures the earlier stage.
    if !retention.is_empty(){
        let baseline=retention_accuracy(&lock(state)?.brain,retention)?;
        let mut s=lock(state)?;
        s.retention_baseline=Some(baseline);
        s.retention_accuracy=Some(baseline);
    }
    let prior_baseline=if protect_previous{
        previous_validation_loss(&lock(state)?.brain,previous)?
    }else{None};
    let teacher_baseline=if protect_previous&&!retention.is_empty(){
        Some(retention_accuracy(&lock(state)?.brain,retention)?)
    }else{None};
    if diag::enabled(&app.state::<Diagnostics>()){
        diag::record(&app.state::<Diagnostics>(),"rehearsal_baseline",serde_json::json!({
            "previousLessons":previous.len(),"rehearsalEnabled":rehearsal_enabled,
            "retentionGuardEnabled":protect_previous,"priorHeldoutLoss":prior_baseline,
            "teacherGreedyAccuracy":teacher_baseline,"lossTolerance":loss_tolerance,
            "criterion":"new validation loss improves AND previous loss within baseline+tolerance AND optional teacher accuracy does not fall"
        }));
    }
    // Snapshot original weights BEFORE epoch one, without applying gradients.
    let practice={
        let empty=Vec::<String>::new();
        observer::training_replies(lessons.train.iter().enumerate().map(|(i,ex)|(
            ex,lessons.memory_train.get(i).unwrap_or(&empty),
            lessons.train_kinds.get(i).copied().unwrap_or(0)
        )))
    };
    let training_contexts=lessons.train.iter()
        .filter(|(prompt,_)|prompt.ends_with(b"\nN: "))
        .map(|(prompt,_)|prompt.clone()).collect::<Vec<_>>();
    let baseline_snapshot=(||->Result<(),String>{
        let guard=lock(state)?;
        let record=observer::capture(&guard.brain,"unified",&lessons.mode,0,
            &practice,&lessons.heldout_replies,&training_contexts,serde_json::json!({
                "baseline":true,"heldoutLoss":lesson_baseline_loss
            }))?;
        observer::append(app,observer_id,&record)
    })();
    let _=app.emit("nova-observer-epoch",serde_json::json!({
        "sessionId":observer_id,"epoch":0,"error":baseline_snapshot.err()
    }));
    let mut best_epoch=0u32;
    let mut schedule=ScheduleState::new(starting_lr,ScheduleConfig{
        adaptive_learning_rate,early_stopping,patience,
    });
    let mut completed=0usize;
    let mut cancelled=false;
    let mut early_stopped=false;
    #[cfg(target_os="android")]
    let mut last_android_notice=Instant::now()-std::time::Duration::from_secs(20);
    for epoch in 1..=epochs{
        if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
        let stage=if epoch==1&&lessons.mode=="dialogue"{
            "Warm-up: short contexts first"
        }else{"Practice: shuffled examples"};
        let learning_rate=schedule.learning_rate;
        let batches_new=objective_batches(lessons,epoch);
        let focus_batches=reply_boundary_batches(lessons,epoch,batches_new.len());
        let mut focus_count=0usize;
        // Sample old categories fairly and rotate *archived lesson sources*,
        // rather than selecting arbitrary first batches from large lessons.
        let prior_batches=if rehearsal_enabled {
            balanced_replay_schedule(previous,epoch,batches_new.len())
        }else{Vec::new()};
        let mut replay_batches=0usize;
        let mut contextual_batches=0usize;
        let mut evidence_batches=0usize;
        let mut evidence_loss_sum=0.0f64;
        let mut epoch_loss=0.0f64;
        let mut trained_count=0usize;
        let mut batches=0usize;
        let mut transformer_grad_norm=0.0f64;
        let mut dense_grad_norm=0.0f64;
        let mut bridge_grad_norm=0.0f64;
        let mut transformer_clip_batches=0usize;
        let mut dense_clip_batches=0usize;
        let mut transformer_clip_scale_total=0.0f64;
        let mut dense_clip_scale_total=0.0f64;
        {
            let mut s=lock(state)?;
            s.epoch=epoch;s.stage=stage.into();s.learning_rate=learning_rate;
        }
        for (new_batch_index,ids) in batches_new.iter().enumerate(){
            if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
            let batch:Vec<Example>=ids.iter().map(|&i|train[i].clone()).collect();
            let (loss,step,diag)={
                let mut s=lock(state)?;
                let memory_batch=lessons.mode=="conversation" ||
                    (lessons.mode=="unified"&&lessons.train_kinds[ids[0]]
                        ==neural_nova::unified::CONVERSATION);
                let loss=if memory_batch{
                    let extended=ids.iter().map(|&i|(train[i].0.clone(),train[i].1,
                        lessons.memory_train[i].clone())).collect::<Vec<_>>();
                    s.brain.train_memory_batch(&extended,learning_rate)?
                }else{s.brain.train_batch(&batch,learning_rate)?};
                if !loss.is_finite(){return Err("Training loss is non-finite".into());}
                s.trained=true;s.completed=completed+batch.len();s.train_loss=Some(loss);
                s.tokens_per_second=(s.completed as f64/started.elapsed().as_secs_f64().max(0.001)) as f32;
                (loss,s.brain.transformer.step,
                    s.brain.last_batch_diagnostics.clone().ok_or("Gradient diagnostics missing")?)
            };
            if diag::enabled(&app.state::<Diagnostics>()){
                diag::record(&app.state::<Diagnostics>(),"train_batch",serde_json::json!({
                    "epoch":epoch,"optimizerStep":step,"examples":batch.len(),
                    "learningRate":learning_rate,"batchLoss":loss,
                    "transformerGradientNorm":diag.transformer_norm,
                    "denseGradientNorm":diag.dense_norm,
                    "bridgeGradientNorm":diag.bridge_norm,
                    "transformerClipped":diag.transformer_clipped,
                    "denseClipped":diag.dense_clipped,
                    "clipThreshold":diag.clip_threshold,
                    "transformerClipScale":diag.transformer_clip_scale,
                    "denseClipScale":diag.dense_clip_scale,
                    "usesEpisodicMemory":lessons.mode=="conversation"||
                        (lessons.mode=="unified" && lessons.train_kinds[ids[0]]==2),
                    "learningObjective":if lessons.mode=="unified"{
                        Some(lessons.train_kinds[ids[0]])}else{None},
                }));
            }
            // Every fourth new batch additionally optimizes a DIFFERENT
            // sequence-level loss: prefer supported conclusions to explicitly
            // contradicted conclusions. There is still only one shared brain.
            if (lessons.mode=="reasoning"||lessons.mode=="unified") &&
                !lessons.reasoning_train.is_empty() &&
                ((new_batch_index+1)%4==0 || new_batch_index+1==batches_new.len()){
                let index=(new_batch_index/4+epoch as usize-1)%lessons.reasoning_train.len();
                let item=lessons.reasoning_train[index].clone();
                let (rank_loss,reasoning_step)={
                    let mut s=lock(state)?;
                    let loss=s.brain.train_preference_batch(&[item],learning_rate)?;
                    if !loss.is_finite(){return Err("Non-finite reasoning preference loss".into());}
                    (loss,s.brain.transformer.step)
                };
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"reasoning_preference_batch",
                        serde_json::json!({"epoch":epoch,"optimizerStep":reasoning_step,
                            "rankLoss":rank_loss,"curriculumIndex":index,
                            "bothNetworksUpdated":true}));
                }
            }
            // Optional explicit, ordered evidence step supervision.
            // This teaches recurrent attention on source facts; the teacher's
            // phrases NEVER enter inference or determine Nova's reply.
            if !lessons.evidence_train.is_empty() &&
                ((new_batch_index+1)%4==0 || new_batch_index+1==batches_new.len()){
                let index=(new_batch_index/4+epoch as usize-1)%lessons.evidence_train.len();
                let item=lessons.evidence_train[index].clone();
                let mut s=lock(state)?;
                let evidence_lr=(learning_rate*0.5).max(0.0001);
                let evidence_loss=s.brain.train_evidence_batch(&[item],evidence_lr)?;
                if !evidence_loss.is_finite(){return Err("Nonfinite evidence alignment loss".into());}
                evidence_batches+=1;evidence_loss_sum+=evidence_loss as f64;
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"reasoning_evidence_alignment",
                        serde_json::json!({"epoch":epoch,"loss":evidence_loss,
                            "trainingExampleIndex":index,"modelStep":s.brain.transformer.step,
                            "learningRate":evidence_lr,"sourceOnly":true}));
                }
            }
            // A separate teacher-provided contextual contrast objective
            // compares complete answers to other independent training examples
            // with different facts. It optimizes the SAME cooperative brain.
            if lessons.mode=="unified" && lessons.train_kinds[ids[0]]==1 &&
                !lessons.understanding_train.is_empty() &&
                (contextual_batches==0 || (new_batch_index+1)%4==0){
                let index=(contextual_batches+epoch as usize-1)%
                    lessons.understanding_train.len();
                let mut s=lock(state)?;
                let contextual_lr=(learning_rate*0.5).max(0.0001);
                let comparison_loss=s.brain.train_preference_batch(
                    &[lessons.understanding_train[index].clone()],contextual_lr)?;
                if !comparison_loss.is_finite(){
                    return Err("Non-finite understanding preference objective".into());
                }
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"understanding_contrast",serde_json::json!({
                        "epoch":epoch,"optimizerStep":s.brain.transformer.step,
                        "loss":comparison_loss,"learningRate":contextual_lr,
                        "bothHemisphereGradients":true,"teacherOnlyComparison":true
                    }));
                }
                contextual_batches+=1;
            }
            trained_count+=batch.len();
            batches+=1;
            transformer_grad_norm+=diag.transformer_norm as f64;
            dense_grad_norm+=diag.dense_norm as f64;
            bridge_grad_norm+=diag.bridge_norm as f64;
            transformer_clip_batches+=usize::from(diag.transformer_clipped);
            dense_clip_batches+=usize::from(diag.dense_clipped);
            transformer_clip_scale_total+=diag.transformer_clip_scale as f64;
            dense_clip_scale_total+=diag.dense_clip_scale as f64;
            completed+=batch.len();
            epoch_loss+=loss as f64*batch.len() as f64;
            // Replay ONE real previous-lesson batch after every three new
            // batches, round-robin across at most three older lesson archives.
            // Both pathways receive the same actual gradients. The held-out
            // portions of archived lessons are NEVER sampled here.
            if !prior_batches.is_empty() &&
                ((new_batch_index+1)%3==0 || new_batch_index+1==batches_new.len()) {
                let (source,ids,objective)=prior_batches[replay_batches%prior_batches.len()].clone();
                let old=&previous[source];
                replay_batches+=1;
                let replay:Vec<Example>=ids.iter().map(|&i|old.train[i].clone()).collect();
                let (replay_loss,step,grad)={
                    let mut s=lock(state)?;
                    let replay_loss=if old.mode=="conversation" ||
                        (old.mode=="unified"&&objective==2){
                        let with_memory=ids.iter().map(|&i|(
                            old.train[i].0.clone(),old.train[i].1,old.memory_train[i].clone()
                        )).collect::<Vec<_>>();
                        s.brain.train_memory_batch(&with_memory,learning_rate)?
                    }else{s.brain.train_batch(&replay,learning_rate)?};
                    if !replay_loss.is_finite(){return Err("Non-finite rehearsal loss".into());}
                    if old.mode=="unified" && objective==1 &&
                        !old.understanding_train.is_empty(){
                        let id=(replay_batches+epoch as usize-1)%old.understanding_train.len();
                        let extra=s.brain.train_preference_batch(
                            &[old.understanding_train[id].clone()],
                            (learning_rate*0.5).max(0.0001))?;
                        if !extra.is_finite(){return Err("Non-finite contextual rehearsal".into());}
                    }
                    if (old.mode=="reasoning" || old.mode=="unified") && objective==3 &&
                        !old.reasoning_train.is_empty(){
                        // Preserve conclusion comparison as well as the wording
                        // when an older reasoning lesson gets replayed.
                        let id=(replay_batches+epoch as usize-1)%old.reasoning_train.len();
                        let extra=s.brain.train_preference_batch(
                            &[old.reasoning_train[id].clone()],learning_rate)?;
                        if !extra.is_finite(){return Err("Non-finite reasoning rehearsal loss".into());}
                    }
                    s.completed=completed+replay.len();
                    s.train_loss=Some(replay_loss);
                    (replay_loss,s.brain.transformer.step,
                        s.brain.last_batch_diagnostics.clone().ok_or("Missing rehearsal gradients")?)
                };
                completed+=replay.len();trained_count+=replay.len();batches+=1;
                epoch_loss+=replay_loss as f64*replay.len() as f64;
                transformer_grad_norm+=grad.transformer_norm as f64;
                dense_grad_norm+=grad.dense_norm as f64;
                bridge_grad_norm+=grad.bridge_norm as f64;
                transformer_clip_batches+=usize::from(grad.transformer_clipped);
                dense_clip_batches+=usize::from(grad.dense_clipped);
                transformer_clip_scale_total+=grad.transformer_clip_scale as f64;
                dense_clip_scale_total+=grad.dense_clip_scale as f64;
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"rehearsal_batch",serde_json::json!({
                        "epoch":epoch,"sourceIndex":source,"sourceMode":old.mode,
                        "optimizerStep":step,"examples":replay.len(),"loss":replay_loss,
                        "learningRate":learning_rate,"usesEpisodicMemory":old.mode=="conversation"||objective==2,
                        "learningObjective":objective,
                        "gradientTransformer":grad.transformer_norm,"gradientDense":grad.dense_norm
                    }));
                }
            }
            // The same trainable two-hemisphere network receives a balanced
            // dose of actual first-character and learned end-of-reply targets.
            // There is no inference-time answer routing or canned dialogue.
            if !focus_batches.is_empty() &&
                ((new_batch_index+1)%8==0 || new_batch_index+1==batches_new.len()){
                let ids=&focus_batches[focus_count];
                focus_count+=1;
                let kind=lessons.train_kinds[ids[0]];
                let (focus_loss,grad,step)={
                    let mut s=lock(state)?;
                    let focus_loss=if kind==neural_nova::unified::CONVERSATION{
                        let examples=ids.iter().map(|&id|(
                            train[id].0.clone(),train[id].1,
                            lessons.memory_train[id].clone()
                        )).collect::<Vec<_>>();
                        s.brain.train_memory_batch(&examples,learning_rate)?
                    }else{
                        let examples=ids.iter().map(|&id|train[id].clone())
                            .collect::<Vec<_>>();
                        s.brain.train_batch(&examples,learning_rate)?
                    };
                    if !focus_loss.is_finite(){
                        return Err("Non-finite response-boundary training loss".into());
                    }
                    let grad=s.brain.last_batch_diagnostics.clone()
                        .ok_or("Response-boundary gradient diagnostics missing")?;
                    let step=s.brain.transformer.step;
                    s.train_loss=Some(focus_loss);
                    s.completed=completed+ids.len();
                    (focus_loss,grad,step)
                };
                completed+=ids.len();
                trained_count+=ids.len();
                batches+=1;
                epoch_loss+=focus_loss as f64*ids.len() as f64;
                transformer_grad_norm+=grad.transformer_norm as f64;
                dense_grad_norm+=grad.dense_norm as f64;
                bridge_grad_norm+=grad.bridge_norm as f64;
                transformer_clip_batches+=usize::from(grad.transformer_clipped);
                dense_clip_batches+=usize::from(grad.dense_clipped);
                transformer_clip_scale_total+=grad.transformer_clip_scale as f64;
                dense_clip_scale_total+=grad.dense_clip_scale as f64;
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"reply_boundary_batch",
                        serde_json::json!({
                            "epoch":epoch,"objective":kind,
                            "samples":ids.len(),"loss":focus_loss,
                            "optimizerStep":step,"firstCharacterAndLearnedReplyEnd":true,
                            "teacherProvidedTargetsOnly":true,
                            "bothPathwaysUpdated":true
                        }));
                }
            }
            if completed%64<batch.len(){
                #[cfg(target_os="android")]
                if last_android_notice.elapsed()>=std::time::Duration::from_secs(8){
                    let total=lock(state)?.total;
                    let pct=if total==0{0}else{(100*completed/total).min(100)};
                    training_notification(app,format!(
                        "Epoch {}/{} • {}% • batch loss {:.4}",epoch,epochs,pct,loss
                    ),false);
                    last_android_notice=Instant::now();
                }
                let _=app.emit("hybrid-progress",serde_json::json!({
                    "completed":completed,"total":lock(state)?.total,
                    "epoch":epoch,"epochs":epochs,"loss":loss,"step":step,
                    "stage":stage,"learningRate":learning_rate,
                    "tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001)
                }));
            }
        }
        if trained_count==0 {break;}
        let (validation_loss,validation_accuracy,prior_loss,retention_score,
            eligible,improved,reasoning_ranking_loss,reasoning_ranking_accuracy,
            current_autonomous,autonomous_guard,archived_autonomous_guard,
            prior_autonomous_candidate,selection_reason)={
            let mut s=lock(state)?;
            s.stage="Evaluating current and earlier held-out lessons".into();
            let (loss,accuracy,ranking_loss,ranking_accuracy)=current_validation(&s.brain,lessons)?;
            if !loss.is_finite(){return Err("Non-finite validation loss".into());}
            let old_loss=if protect_previous{
                previous_validation_loss(&s.brain,previous)?
            }else{None};
            let teacher=if retention.is_empty(){None}else{
                Some(retention_accuracy(&s.brain,retention)?)
            };
            let eligible=!protect_previous ||
                rehearsal::eligible_checkpoint(
                    old_loss,prior_baseline,loss_tolerance,teacher,teacher_baseline);
            // Screening via teacher-forced validation avoids expensive
            // rollouts when an epoch obviously regressed. A candidate that
            // qualifies must ALSO preserve autonomous first-character and
            // full-answer performance on independent held-out examples.
            let candidate=eligible && if lessons.mode=="unified"{
                stays_near_original_baseline(lesson_baseline_loss,best_loss,loss)
            }else{loss+MIN_IMPROVEMENT<best_loss};
            let autonomous=if candidate && lessons.mode=="unified"{
                autonomous_validation(&s.brain,lessons)?
            }else{None};
            let autonomous_guard=if candidate{
                autonomous_selection_ok(best_autonomous,autonomous,best_loss,loss)
            }else{false};
            let archived_candidate=if candidate && autonomous_guard{
                archived_autonomous_scores(&s.brain,&prior_autonomous_panels)?
            }else{Vec::new()};
            let archived_autonomous_guard=if candidate && autonomous_guard{
                archived_autonomous_retained(&prior_autonomous_baseline,&archived_candidate)
            }else{false};
            let improved=candidate&&autonomous_guard&&archived_autonomous_guard;
            // Explain the selected checkpoint rather than calling it the
            // numerically lowest held-out loss when it may be a JOINT score.
            let selection_reason=if !eligible{
                "rejected: earlier-skill heldout/teacher guard"
            }else if !candidate{
                "rejected: current lesson exceeds fixed baseline tolerance"
            }else if !autonomous_guard{
                "rejected: autonomous current-lesson regression or joint score"
            }else if !archived_autonomous_guard{
                "rejected: earlier learned replies regressed"
            }else{if prior_baseline.is_none() && prior_autonomous_panels.is_empty() &&
                    retention.is_empty(){
                    "selected: joint lesson score; no earlier-skill baseline available"
                }else{"selected: joint lesson score; sampled earlier abilities retained"}};
            if improved{
                best_loss=loss;best_accuracy=accuracy;best_epoch=epoch;
                best_autonomous=autonomous;
                best_brain=s.brain.clone();
                s.brain.save(path(app)?)?;
            }
            s.best_epoch=best_epoch;
            s.best_heldout_loss=Some(best_loss);
            s.heldout_loss=Some(loss);
            s.heldout_accuracy=Some(accuracy);
            (loss,accuracy,old_loss,teacher,eligible,improved,ranking_loss,
                ranking_accuracy,autonomous,autonomous_guard,
                archived_autonomous_guard,archived_candidate,selection_reason)
        };
        if let Some(value)=retention_score{lock(state)?.retention_accuracy=Some(value);}
        let should_stop=schedule.observe_validation(improved);
        if diag::enabled(&app.state::<Diagnostics>()){
            let brain=lock(state)?.brain.clone();
            let probes=["hello","Hi","good morning","hey there"].iter()
                .map(|s|s.to_string()).collect::<Vec<_>>();
            diag::record(&app.state::<Diagnostics>(),"train_epoch",serde_json::json!({
                "epoch":epoch,"learningRate":learning_rate,
                "trainingLoss":epoch_loss/trained_count as f64,
                "evidenceAlignmentUpdates":evidence_batches,
                "evidenceAlignmentLoss":if evidence_batches==0{None}
                    else{Some(evidence_loss_sum/evidence_batches as f64)},
                "autonomousHeldout":current_autonomous,
                "selectedAutonomousHeldout":best_autonomous,
                "lessonBaselineAutonomous":lesson_baseline_autonomous,
                "lessonBaselineHeldoutLoss":lesson_baseline_loss,
                "heldoutPanelId":current_heldout_panel_id,
                "selectionReason":selection_reason,
                "archivedAutonomousBaseline":prior_autonomous_baseline,
                "archivedAutonomousCandidate":prior_autonomous_candidate,
                "archivedAutonomousGuardAccepted":archived_autonomous_guard,
                "autonomousGuardAccepted":autonomous_guard,
                "autonomousSelectionUsesTeacherHeldoutOnly":true,
                "validationLoss":validation_loss,"validationAccuracy":validation_accuracy,
                "reasoningRankingLoss":reasoning_ranking_loss,"reasoningRankingAccuracy":reasoning_ranking_accuracy,
                "generatedHeldoutExactByCategory":if lessons.mode=="unified"{
                    generated_heldout(&brain,lessons).ok()
                }else{None},
                "greetingFirstCharacterBias":greeting_first_character_bias(&brain),
                "understandingContrast":if lessons.mode=="unified"{
                    understanding_validation(&brain,lessons).ok()
                }else{None},
                "heldoutContextAblation":if lessons.mode=="unified" &&
                    (epoch==1 || epoch%3==0 || should_stop){
                    heldout_context_ablation(&brain,lessons).ok().flatten()
                }else{None},
                "categoryHeldoutLoss":if lessons.mode=="unified"{
                    mixed_validation(&brain,lessons,
                        &(0..lessons.validation.len()).collect::<Vec<_>>())
                        .ok().map(|x|x.2)
                }else{None},
                "bestValidationLoss":best_loss,"bestEpoch":best_epoch,"improved":improved,
                "priorHeldoutLoss":prior_loss,"priorBaselineLoss":prior_baseline,
                "retentionGuardAccepted":eligible,"rehearsalBatches":replay_batches,
                "earlyStopNow":should_stop,"retentionAccuracy":retention_score,
                "transformerGradientMean":transformer_grad_norm/batches.max(1) as f64,
                "denseGradientMean":dense_grad_norm/batches.max(1) as f64,
                "bridgeGradientMean":bridge_grad_norm/batches.max(1) as f64,
                "transformerClipFraction":transformer_clip_batches as f64/batches.max(1) as f64,
                "denseClipFraction":dense_clip_batches as f64/batches.max(1) as f64,
                "clipThreshold":1.0,
                "transformerMeanClipScale":transformer_clip_scale_total/batches.max(1) as f64,
                "denseMeanClipScale":dense_clip_scale_total/batches.max(1) as f64,
                "modelStep":brain.transformer.step,"wordUnits":brain.tokenizer.words.len(),
                "weightsFingerprint":diag::weight_digest(&brain),
                "recurrentLatentSteps":neural_nova::transformer::REASONING_STEPS,
                "recurrentUpdateNorms":brain.reasoning_activity(b"U: hello\nN: ").ok(),
                "isolatedGreetingProbes":diagnostic_probes(&brain,&probes,128,&[],false),
            }));
        }
        // Measure every epoch, including rejected checkpoints; all generation
        // is on the actual current weights with no answer-fed inference.
        let observation=(||->Result<(),String>{
            let guard=lock(state)?;
            let record=observer::capture(&guard.brain,"unified",&lessons.mode,epoch,
                &practice,&lessons.heldout_replies,&training_contexts,serde_json::json!({
                    "trainingLoss":epoch_loss/trained_count as f64,
                "evidenceAlignmentUpdates":evidence_batches,
                "evidenceAlignmentLoss":if evidence_batches==0{None}
                    else{Some(evidence_loss_sum/evidence_batches as f64)},
                    "evidenceAlignmentUpdates":evidence_batches,
                    "evidenceAlignmentLoss":if evidence_batches==0{None}
                        else{Some(evidence_loss_sum/evidence_batches as f64)},
                    "heldoutLoss":validation_loss,
                    "heldoutAccuracy":validation_accuracy,
                    "selected":improved,"selectedEpoch":best_epoch,
                    "selectionReason":selection_reason,
                    "reasoningPreferenceAccuracy":reasoning_ranking_accuracy,
                    "priorHeldoutLoss":prior_loss,"retentionAccuracy":retention_score,
                    "transformerGradientNorm":transformer_grad_norm/batches.max(1) as f64,
                    "denseGradientNorm":dense_grad_norm/batches.max(1) as f64,
                    "bridgeGradientNorm":bridge_grad_norm/batches.max(1) as f64,
                    "transformerClipFraction":transformer_clip_batches as f64/batches.max(1) as f64,
                    "denseClipFraction":dense_clip_batches as f64/batches.max(1) as f64,
                "clipThreshold":1.0,
                "transformerMeanClipScale":transformer_clip_scale_total/batches.max(1) as f64,
                "denseMeanClipScale":dense_clip_scale_total/batches.max(1) as f64
                }))?;
            observer::append(app,observer_id,&record)
        })();
        let _=app.emit("nova-observer-epoch",serde_json::json!({
            "sessionId":observer_id,"epoch":epoch,"error":observation.err()
        }));
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":epoch_loss/trained_count as f64,
            "autonomousHeldout":current_autonomous,
            "selectedAutonomousHeldout":best_autonomous,
            "lessonBaselineAutonomous":lesson_baseline_autonomous,
            "lessonBaselineLoss":lesson_baseline_loss,
            "heldoutPanelId":current_heldout_panel_id,
            "selectionReason":selection_reason,
            "archivedAutonomousBaseline":prior_autonomous_baseline,
            "archivedAutonomousCandidate":prior_autonomous_candidate,
            "archivedAutonomousGuardAccepted":archived_autonomous_guard,
            "autonomousGuardAccepted":autonomous_guard,
            "validationLoss":validation_loss,"validationAccuracy":validation_accuracy,
            "bestLoss":best_loss,"bestEpoch":best_epoch,"improved":improved,
            "previousHeldoutLoss":prior_loss,"previousBaselineLoss":prior_baseline,
            "guardAccepted":eligible,"rehearsalBatches":replay_batches,
            "learningRate":learning_rate,"tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001),
            "trainLessons":lessons.train_lessons,"validationLessons":lessons.validation_lessons,
            "retentionAccuracy":retention_score,"retentionQuestions":retention.len(),
            "transformerGradientNorm":transformer_grad_norm/batches.max(1) as f64,
            "denseGradientNorm":dense_grad_norm/batches.max(1) as f64,
            "bridgeGradientNorm":bridge_grad_norm/batches.max(1) as f64,
            "transformerClipFraction":transformer_clip_batches as f64/batches.max(1) as f64,
            "denseClipFraction":dense_clip_batches as f64/batches.max(1) as f64,
                "clipThreshold":1.0,
                "transformerMeanClipScale":transformer_clip_scale_total/batches.max(1) as f64,
                "denseMeanClipScale":dense_clip_scale_total/batches.max(1) as f64
        }));
        if cancelled{break;}
        if should_stop{
            early_stopped=true;
            break;
        }
    }
    // If every candidate regressed earlier skills, restore the entire
    // unmodified pre-lesson joint checkpoint (including tokenizer/optimizer).
    if best_epoch==0 && initially_trained {
        best_brain=HybridBrain::load(path(app)?)?;
    }
    let restored_retention=if retention.is_empty(){None}
        else{Some(retention_accuracy(&best_brain,retention)?)};
    if diag::enabled(&app.state::<Diagnostics>()){
        let prompts=["hello","Hi","good morning","hey there"].iter()
            .map(|p|p.to_string()).collect::<Vec<_>>();
        diag::record(&app.state::<Diagnostics>(),"training_selected_checkpoint",serde_json::json!({
            "requestedEpochs":epochs,"bestEpoch":best_epoch,"bestValidationLoss":best_loss,
            "lessonBaselineLoss":lesson_baseline_loss,
            "heldoutPanelId":current_heldout_panel_id,
            "lessonBaselineAutonomous":lesson_baseline_autonomous,
            "archivedAutonomousBaseline":prior_autonomous_baseline,
            "bestAutonomousHeldout":best_autonomous,
            "earlyStopped":early_stopped,"cancelled":cancelled,
            "selectedStep":best_brain.transformer.step,
            "weightsFingerprint":diag::weight_digest(&best_brain),
            "selectedGreetingProbes":diagnostic_probes(&best_brain,&prompts,128,&[],true),
            "recurrentLatentSteps":neural_nova::transformer::REASONING_STEPS,
            "recurrentUpdateNorms":best_brain.reasoning_activity(b"U: hello\nN: ").ok(),
            "retentionAccuracy":restored_retention,
            "selectionPolicy":if protect_previous{
                "joint current-lesson score subject to fixed lesson baseline, archived prior full-answer retention, prior heldout and optional teacher accuracy safeguards"
            }else{"joint current-lesson score subject to fixed lesson baseline (earlier-skill guard disabled by teacher)"},
            "rehearsalEnabled":rehearsal_enabled,"priorLessonCount":previous.len(),
            "priorHeldoutBaseline":prior_baseline,"lossTolerance":loss_tolerance,
        }));
    }
    {
        let mut s=lock(state)?;
        s.retention_accuracy=restored_retention;
        // Last training steps may overfit. Only the best held-out joint
        // checkpoint is kept, never selected between two independently
        // answering models.
        s.brain=best_brain;
        s.trained=initially_trained || best_epoch>0;
        s.best_epoch=best_epoch;
        s.best_heldout_loss=Some(best_loss);
        s.heldout_loss=Some(best_loss);
        s.heldout_accuracy=Some(best_accuracy);
        s.learning_rate=schedule.learning_rate;
        s.early_stopped=early_stopped;
        s.tokens_per_second=(completed as f64/started.elapsed().as_secs_f64().max(0.001)) as f32;
        s.stage=if cancelled{"Stopped; best checkpoint restored"}
                else if early_stopped{"Early stop; best checkpoint restored"}
                else{"Complete; best checkpoint restored"}.into();
        if s.trained{s.brain.save(path(app)?)?;}
    }
    #[cfg(target_os="android")]
    training_notification(app,format!(
        "{} • selected epoch {} of {}",
        if cancelled{"Stopped"}else if early_stopped{"Early stopping"}else{"Completed"},
        best_epoch,epochs
    ),true);
    Ok((cancelled,early_stopped,best_epoch))
}
/// Experimental targeted teaching can hold the shared training lock while
/// training a separate cloned candidate, preventing ordinary ZIP teaching,
/// checkpoint imports, reset, or chat from racing the eventual commit.
pub(crate) fn targeted_reserve(state:&HybridState)->Result<(),String>{
    state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire)
        .map_err(|_|"Finish the existing Nova training session first".to_string())?;
    Ok(())
}
pub(crate) fn targeted_release(state:&HybridState){
    state.inner.training.store(false,Ordering::Release);
}
pub(crate) fn targeted_snapshot(state:&HybridState)
    ->Result<(HybridBrain,String),String>{
    let s=lock(state)?;
    Ok((s.brain.clone(),diag::weight_digest(&s.brain)))
}
/// Return only PREVIOUS, withheld teacher benchmarks. Targets are never
/// included in a focused gradient update.
pub(crate) fn targeted_archived_validation(
    app:&AppHandle,state:&HybridState
)->Result<Vec<neural_nova::trainer::HeldoutReply>,String>{
    let s=lock(state)?;
    let bank=rehearsal_for_current(app,&s.brain)?;
    let plans=rehearsal::previous_plans(&bank)?;
    Ok(archived_autonomous_panels(&plans).into_iter().flatten().collect())
}
/// Apply a manually approved whole-brain candidate using the same validated
/// checkpoint backup/atomic save path as normal teaching. Never splice
/// individual neuron arrays between different checkpoint lineages.
pub(crate) fn targeted_commit(app:&AppHandle,state:&HybridState,
    candidate:&HybridBrain,expected_active_fingerprint:&str
)->Result<HybridStatus,String>{
    targeted_reserve(state)?;
    let attempt=(||->Result<HybridStatus,String>{
        let mut s=lock(state)?;
        if diag::weight_digest(&s.brain)!=expected_active_fingerprint{
            return Err("Active Nova weights changed since targeted training began. Discard this stale candidate or begin again; no checkpoint was overwritten".into());
        }
        if s.trained{snapshot_active_checkpoint(app,&s.brain,"before-targeted-accept")?;}
        candidate.save(path(app)?)?;
        save_conversation(app,&[])?;
        s.brain=candidate.clone();
        s.trained=true;
        s.history.clear();
        s.epoch=0;s.epochs=0;s.completed=0;s.total=0;
        s.heldout_loss=None;s.heldout_accuracy=None;s.train_loss=None;
        s.best_heldout_loss=None;s.best_epoch=0;
        s.early_stopped=false;s.retention_baseline=None;s.retention_accuracy=None;
        s.error=None;s.stage="Teacher accepted targeted neural candidate".into();
        diag::record(&app.state::<Diagnostics>(),"targeted_checkpoint_accepted",serde_json::json!({
            "optimizerStep":s.brain.transformer.step,
            "denseStep":s.brain.dense_adam_steps(),
            "fingerprint":diag::weight_digest(&s.brain),
            "wholeBrain":true,"originalCheckpointBackedUp":true,
        }));
        Ok(status(state,&s))
    })();
    targeted_release(state);
    attempt
}
#[tauri::command]
pub fn hybrid_stop(state:State<'_,HybridState>)->bool{
    let training=state.inner.training.load(Ordering::Acquire);
    if training{state.inner.stop.store(true,Ordering::Release);}
    training
}
#[tauri::command]
pub fn hybrid_reset(app:AppHandle,state:State<'_,HybridState>)->Result<HybridStatus,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Stop training before resetting weights".into());}
    let mut s=lock(&state)?;
    let retained_history=s.history.clone();
    *s=Session{
        brain:HybridBrain::new(),trained:false,epoch:0,epochs:0,completed:0,total:0,
        heldout_loss:None,heldout_accuracy:None,train_loss:None,error:None,
        best_heldout_loss:None,best_epoch:0,learning_rate:DEFAULT_LEARNING_RATE,
        train_lessons:0,validation_lessons:0,tokens_per_second:0.0,
        stage:"Idle".into(),early_stopped:false,history:retained_history,
        retention_baseline:None,retention_accuracy:None
    };
    let file=path(&app)?;
    if file.exists(){fs::remove_file(&file).map_err(|e|e.to_string())?;}
    let teacher_bank=rehearsal_path(&app)?;
    if teacher_bank.exists(){fs::remove_file(&teacher_bank).map_err(|e|e.to_string())?;}
    // Preserve the old checkpoint, but prevent reset weights from
    // automatically re-importing them on subsequent launches.
    fs::write(file.with_extension("no-import"),b"explicit reset; do not auto-import older weights") 
        .map_err(|e|e.to_string())?;
    Ok(status(&state,&s))
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    #[test]
    fn context_report_tracks_original_prompt_cutoff_exactly(){
        let a=context_report(128,12,116,"length-limit",false,0.6,Some(1));
        assert_eq!(a.prompt_bytes_retained,12);
        assert_eq!(a.first_prompt_truncation_char,None);
        let b=context_report(128,12,117,"length-limit",false,0.6,Some(1));
        assert_eq!(b.prompt_bytes_retained,11);
        assert_eq!(b.first_prompt_truncation_char,Some(117));
        let c=context_report(256,12,128,"length-limit",true,0.6,None);
        assert_eq!(c.prompt_bytes_retained,12);
        assert_eq!(c.first_prompt_truncation_char,None);
        let d=context_report(128,150,2,"learned-end",false,0.6,None);
        assert_eq!(d.first_prompt_truncation_char,Some(0));
        assert_eq!(d.prompt_bytes_retained,126);
    }
    #[test]
    fn extended_context_and_greedy_sampling_are_deterministic(){
        assert_eq!(valid_window(Some(128)).unwrap(),128);
        assert_eq!(valid_window(Some(256)).unwrap(),256);
        assert!(valid_window(Some(384)).is_err());
        let mut p=vec![0.0;VOCAB];p[b'Y' as usize]=0.9;p[b'A' as usize]=0.8;
        assert_eq!(greedy_sample(&p),b'Y');
    }
    #[test]
    fn native_options_decode_two_independent_toggles(){
        for &(adaptive,stop) in &[(false,false),(false,true),(true,false),(true,true)]{
            let json=serde_json::json!({
                "adaptiveLearningRate":adaptive,"earlyStopping":stop,
                "learningRate":0.001,"patience":6
            });
            let options:TrainOptions=serde_json::from_value(json).unwrap();
            assert_eq!(resolved_switches(Some(&options)),(adaptive,stop));
        }
    }
    #[test]
    fn default_gentle_fixed_schedule_and_backward_compatible_clients(){
        assert_eq!(resolved_switches(None),(false,true));
        assert_eq!(DEFAULT_LEARNING_RATE,0.001);
        for enabled in [false,true]{
            let option:TrainOptions=serde_json::from_value(serde_json::json!({
                "adaptive":enabled
            })).unwrap();
            assert_eq!(resolved_switches(Some(&option)),(enabled,enabled));
        }
    }
}


/* Experimental teacher-side test loop. This code is NOT part of Nova's
 * inference or neural architecture. Expected answers are never passed into
 * a generation call; they are only used after the response is captured. */
fn test_time()->u64{
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs()).unwrap_or(0)
}
fn test_unique_id(prefix:&str)->String{
    let nanos=SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d|d.as_nanos()).unwrap_or(0);
    format!("{prefix}-{nanos}")
}
fn test_archive(state:&HybridState)->Result<std::sync::MutexGuard<'_,TestArchive>,String>{
    state.inner.tests.lock().map_err(|_|"Teacher archive lock poisoned".into())
}
fn test_record(app:&AppHandle,state:&HybridState,report:&TestReport)->Result<(),String>{
    let mut data=test_archive(state)?;
    let index=data.reports.iter().position(|r|r.id==report.id)
        .ok_or("Test record was removed")?;
    data.reports[index]=report.clone();
    persist_archive(app,&data)
}
#[tauri::command]
pub fn hybrid_test_archive(state:State<'_,HybridState>)->Result<TestArchive,String>{
    Ok(test_archive(&state)?.clone())
}
#[tauri::command]
pub fn hybrid_test_save(app:AppHandle,state:State<'_,HybridState>,
    name:String,questions_text:String,answers_text:String,
    unseen_questions_text:String,unseen_answers_text:String,
)->Result<TestDefinition,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Wait until the current training or test finishes".into());}
    let name=name.trim();
    if name.is_empty()||name.len()>80{return Err("Test name must be 1–80 characters".into());}
    let cases=parse_cases(&questions_text,&answers_text)?;
    let unseen=if unseen_questions_text.trim().is_empty()&&unseen_answers_text.trim().is_empty(){
        Vec::new()
    }else{parse_cases(&unseen_questions_text,&unseen_answers_text)?};
    // Leakage guard: held-out prompts may not duplicate correction questions,
    // including capitalization and harmless whitespace differences.
    let key=|s:&str|s.split_ascii_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
    let existing=cases.iter().map(|c|key(&c.question)).collect::<std::collections::BTreeSet<_>>();
    if unseen.iter().any(|c|existing.contains(&key(&c.question))){
        return Err("Unseen evaluation contains a question also present in the corrective test".into());
    }
    let id=test_unique_id("test");
    let result=TestDefinition{id,name:name.to_string(),cases,unseen,created_at:test_time()};
    let mut archive=test_archive(&state)?;
    if archive.definitions.len()>=30{return Err("30 saved tests maximum; delete an older test first".into());}
    if archive.definitions.iter().any(|x|x.id==result.id){
        return Err("Save was requested twice in the same second; retry".into());
    }
    archive.definitions.push(result.clone());
    persist_archive(&app,&archive)?;
    Ok(result)
}
#[tauri::command]
pub fn hybrid_test_delete(app:AppHandle,state:State<'_,HybridState>,test_id:String)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Cannot delete a test during training".into());}
    let mut archive=test_archive(&state)?;
    let old=archive.definitions.len();
    archive.definitions.retain(|x|x.id!=test_id);
    if old==archive.definitions.len(){return Err("Test not found".into());}
    // Reports remain immutable evidence of previous results.
    persist_archive(&app,&archive)
}
#[tauri::command]
pub fn hybrid_test_pause(state:State<'_,HybridState>)->Result<(),String>{
    if !state.inner.training.load(Ordering::Acquire){return Err("No active training/test".into());}
    state.inner.test_paused.store(true,Ordering::Release);
    Ok(())
}
#[tauri::command]
pub fn hybrid_test_resume(state:State<'_,HybridState>)->Result<(),String>{
    state.inner.test_paused.store(false,Ordering::Release);Ok(())
}
#[tauri::command]
pub fn hybrid_test_stop(state:State<'_,HybridState>)->Result<(),String>{
    state.inner.stop.store(true,Ordering::Release);
    state.inner.test_paused.store(false,Ordering::Release);
    Ok(())
}
fn test_wait(state:&HybridState)->bool{
    while state.inner.test_paused.load(Ordering::Acquire)
        && !state.inner.stop.load(Ordering::Acquire){
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !state.inner.stop.load(Ordering::Acquire)
}
/// Only accepts the question, a frozen brain, and generation settings.
/// No answer key, grade, or teacher content is accessible in this function.
fn test_inference(brain:&HybridBrain,question:&str,o:&TestOptions,
    salt:u64,stop:&AtomicBool)->Result<Option<String>,String>{
    let mut prefix=dialogue_prefix(question)?;
    let mut result=Vec::new();
    let mut rng=o.seed.unwrap_or(42).wrapping_add(salt).max(1);
    for _ in 0..o.max_chars{
        if stop.load(Ordering::Acquire){return Ok(None);}
        // No chat-history or expected-answer memory: tests are isolated.
        let prob=brain.predict_with_memory(&prefix,o.context_chars,&[])?;
        let code=if o.deterministic {greedy_sample(&prob)}
            else{sampled(&prob,0.6,random(&mut rng))};
        if code==b'\n'{break;}
        prefix.push(code);result.push(code);
    }
    Ok(Some(String::from_utf8(result).map_err(|e|e.to_string())?))
}
fn test_attempt(state:&HybridState,case:&TestCase,o:&TestOptions,
    salt:u64)->Result<Option<Attempt>,String>{
    if !test_wait(state){return Ok(None);}
    // Use actual joint HybridBrain weights, copied once for each independent
    // inference call. No answer-key data is included in the input or memory.
    let (brain,step)={
        let model=lock(state)?;
        (model.brain.clone(),model.brain.transformer.step)
    };
    let response=match test_inference(&brain,&case.question,o,salt,&state.inner.stop)?{
        Some(v)=>v,None=>return Ok(None),
    };
    Ok(Some(Attempt{
        automatic:grade(&response,&case.expected).into(),response,
        teacher_grade:None,correction_loss:None,gradient_steps:0,model_step:step,
        sampling:if o.deterministic{"greedy".into()}
                 else{format!("seeded:{}",o.seed.unwrap_or(42).wrapping_add(salt))},
    }))
}
/// One supervised pass per incorrect attempt. The only entry point for
/// expected answers into the model is the existing shared-loss train_batch.
/// Roll back an interrupted/failed correction rather than saving half a batch.
fn test_correction(app:&AppHandle,state:&HybridState,question:&str,
    expected:&str,rate:f32)->Result<Option<(f32,u64)>,String>{
    let before={
        let model=lock(state)?;
        model.brain.clone()
    };
    let first=dialogue_prefix(question)?;
    let mut history=first;
    let mut pairs=Vec::<(Vec<u8>,u8)>::new();
    for target in expected.bytes().chain(std::iter::once(b'\n')){
        pairs.push((history[history.len().saturating_sub(512)..].to_vec(),target));
        history.push(target);
    }
    let mut total_loss=0f64;
    let mut completed=0usize;
    let mut updates=0u64;
    let result=(||->Result<Option<(f32,u64)>,String>{
        for batch in pairs.chunks(8){
            if !test_wait(state){return Ok(None);}
            let loss={
                let mut s=lock(state)?;
                let value=s.brain.train_batch(batch,rate)?;
                if !value.is_finite(){return Err("Nonfinite corrective training loss".into());}
                value
            };
            total_loss+=loss as f64*batch.len() as f64;
            completed+=batch.len();
            updates+=1;
        }
        let mut model=lock(state)?;
        if state.inner.stop.load(Ordering::Acquire){return Ok(None);}
        model.brain.save(path(app)?)?;
        model.trained=true;
        Ok(Some(((total_loss/completed as f64) as f32,updates)))
    })();
    if !matches!(result,Ok(Some(_))){
        let mut s=lock(state)?;
        s.brain=before;
    }
    result
}
fn test_emit(app:&AppHandle,report:&TestReport)->Result<(),String>{
    app.emit("hybrid-test-progress",report.clone()).map_err(|e|e.to_string())
}
fn test_runner(app:&AppHandle,state:&HybridState,test:&TestDefinition,
    o:&TestOptions,report:&mut TestReport)->Result<(),String>{
    if diag::enabled(&app.state::<Diagnostics>()){
        diag::record(&app.state::<Diagnostics>(),"test_started",serde_json::json!({
            "testId":test.id,"runId":report.id,"name":test.name,
            "questionCount":test.cases.len(),"unseenCount":test.unseen.len(),
            "settings":o,"baselineFirst":true,"modelStep":report.start_step,
        }));
    }
    // Phase 1: ALL baseline questions must be answered before ANY correction.
    // Otherwise Q2's "initial score" would already have benefited from Q1.
    for index in 0..test.cases.len(){
        if !test_wait(state){report.status="stopped".into();return Ok(());}
        if let Some(answer)=test_attempt(state,&test.cases[index],o,index as u64)?{
            if diag::enabled(&app.state::<Diagnostics>()){
                diag::record(&app.state::<Diagnostics>(),"test_baseline",serde_json::json!({
                    "index":index,"question":test.cases[index].question,
                    "expected":test.cases[index].expected,"response":answer.response,
                    "grade":answer.automatic,"modelStep":answer.model_step,
                }));
            }
            report.questions[index].attempts.push(answer);
            report.recalculate();
            test_record(app,state,report)?;
            test_emit(app,report)?;
        }else{report.status="stopped".into();return Ok(());}
    }
    // Baseline unseen questions are never used for corrections or vocabulary growth.
    if o.generalization {
        for (index,case) in test.unseen.iter().enumerate(){
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            if let Some(answer)=test_attempt(state,case,o,10_000+index as u64)?{
                report.unseen[index].attempts.push(answer);
                test_record(app,state,report)?;
            }else{report.status="stopped".into();return Ok(());}
        }
        report.baseline_unseen_correct=Some(report.unseen.iter().filter(|r|
            r.attempts.first().is_some_and(|a|a.automatic=="correct")).count());
        test_emit(app,report)?;
    }
    // Phase 2: correct AFTER baseline, then make independent new inferences.
    for (index,case) in test.cases.iter().enumerate(){
        for attempt_no in 1..o.attempts {
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            let previous=report.questions[index].attempts.last().unwrap().automatic.clone();
            if o.corrections && previous=="incorrect" {
                let correction=test_correction(app,state,&case.question,&case.expected,o.learning_rate)?;
                if let Some((loss,steps))=correction{
                    if let Some(prev)=report.questions[index].attempts.last_mut(){
                        prev.correction_loss=Some(loss);prev.gradient_steps=steps;
                    }
                    report.training_updates+=steps;
                    report.end_step=lock(state)?.brain.transformer.step;
                    if diag::enabled(&app.state::<Diagnostics>()){
                        let s=lock(state)?;
                        diag::record(&app.state::<Diagnostics>(),"test_correction",serde_json::json!({
                            "index":index,"question":case.question,"expected":case.expected,
                            "attempt":attempt_no,"correctionLoss":loss,"gradientSteps":steps,
                            "modelStep":s.brain.transformer.step,
                            "weightFingerprint":diag::weight_digest(&s.brain),
                            "gradients":s.brain.last_batch_diagnostics.as_ref().map(|v|
                                serde_json::json!({
                                    "transformerNorm":v.transformer_norm,
                                    "denseNorm":v.dense_norm,"bridgeNorm":v.bridge_norm,
                                    "transformerClipped":v.transformer_clipped,
                                    "denseClipped":v.dense_clipped,
                                })),
                        }));
                    }
                    test_record(app,state,report)?;
                }else{report.status="stopped".into();return Ok(());}
            }
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            if let Some(attempt)=test_attempt(state,case,o,
                100_000+(index as u64)*16+attempt_no as u64)?{
                if diag::enabled(&app.state::<Diagnostics>()){
                    diag::record(&app.state::<Diagnostics>(),"test_retry",serde_json::json!({
                        "index":index,"attempt":attempt_no+1,
                        "question":case.question,"response":attempt.response,
                        "expected":case.expected,"grade":attempt.automatic,
                        "modelStep":attempt.model_step,
                    }));
                }
                report.questions[index].attempts.push(attempt);
                report.recalculate();
                test_record(app,state,report)?;
                test_emit(app,report)?;
            }else{report.status="stopped".into();return Ok(());}
        }
    }
    // Phase 3: retention (independent inference; NEVER update weights).
    if o.retention {
        for (index,case) in test.cases.iter().enumerate(){
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            if report.questions[index].attempts.iter().any(|a|a.gradient_steps>0){
                if let Some(attempt)=test_attempt(state,case,o,200_000+index as u64)?{
                    report.questions[index].retention=Some(attempt);
                    test_record(app,state,report)?;
                    test_emit(app,report)?;
                }else{report.status="stopped".into();return Ok(());}
            }
        }
    }
    if o.generalization {
        for (index,case) in test.unseen.iter().enumerate(){
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            if let Some(a)=test_attempt(state,case,o,10_000+index as u64)?{
                report.unseen[index].attempts.push(a);
                test_record(app,state,report)?;
            }else{report.status="stopped".into();return Ok(());}
        }
        report.final_unseen_correct=Some(report.unseen.iter().filter(|r|
            r.attempts.last().is_some_and(|a|a.automatic=="correct")).count());
    }
    report.recalculate();
    report.status="complete".into();
    if diag::enabled(&app.state::<Diagnostics>()){
        diag::record(&app.state::<Diagnostics>(),"test_complete",serde_json::json!({
            "runId":report.id,"initialCorrect":report.initial_correct,
            "finalCorrect":report.final_correct,"corrected":report.corrected,
            "trainingUpdates":report.training_updates,
            "startStep":report.start_step,"endStep":report.end_step,
            "unseenBefore":report.baseline_unseen_correct,
            "unseenAfter":report.final_unseen_correct,
        }));
    }
    Ok(())
}
#[tauri::command]
pub fn hybrid_test_start(app:AppHandle,state:State<'_,HybridState>,
    test_id:String,options:TestOptions)->Result<String,String>{
    validate_options(&options)?;
    let test={
        let data=test_archive(&state)?;
        data.definitions.iter().find(|t|t.id==test_id).cloned()
            .ok_or("Test not found")?
    };
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova is already learning or taking a test".into());
    }
    state.inner.stop.store(false,Ordering::Release);
    state.inner.test_paused.store(false,Ordering::Release);
    let steps=match lock(&state){
        Ok(s)=>{
            if options.corrections && s.trained{
                if let Err(error)=snapshot_active_checkpoint(&app,&s.brain,"before-test"){
                    state.inner.training.store(false,Ordering::Release);
                    return Err(error);
                }
            }
            s.brain.transformer.step
        },
        Err(e)=>{state.inner.training.store(false,Ordering::Release);return Err(e);}
    };
    let id=test_unique_id("run");
    let mut report=TestReport{
        id:id.clone(),test_id:test.id.clone(),name:test.name.clone(),started_at:test_time(),
        status:"running".into(),config:options.clone(),checkpoint:CHECKPOINT.into(),
        questions:test.cases.iter().map(|c|QuestionResult{question:c.question.clone(),
            expected:c.expected.clone(),teacher_answer:None,attempts:Vec::new(),retention:None}).collect(),
        unseen:test.unseen.iter().map(|c|QuestionResult{question:c.question.clone(),
            expected:c.expected.clone(),teacher_answer:None,attempts:Vec::new(),retention:None}).collect(),
        baseline_unseen_correct:None,final_unseen_correct:None,
        initial_correct:0,final_correct:0,corrected:0,still_incorrect:0,
        training_updates:0,start_step:steps,end_step:steps,error:None
    };
    let registered=(||->Result<(),String>{
        let mut data=test_archive(&state)?;
        data.reports.push(report.clone());
        if data.reports.len()>25{data.reports.remove(0);}
        persist_archive(&app,&data)
    })();
    if let Err(e)=registered{
        state.inner.training.store(false,Ordering::Release);
        return Err(e);
    }
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        let result=test_runner(&app,&state,&test,&options,&mut report);
        if let Err(e)=result{
            report.status="failed".into();
            report.error=Some(e);
        }
        if report.status=="running"{report.status="complete".into();}
        if let Ok(s)=lock(&state){report.end_step=s.brain.transformer.step;}
        if let Err(e)=test_record(&app,&state,&report){eprintln!("Failed saving test overview: {e}");}
        let _=test_emit(&app,&report);
        state.inner.training.store(false,Ordering::Release);
        state.inner.test_paused.store(false,Ordering::Release);
        let _=app.emit("hybrid-test-done",report);
    });
    Ok(id)
}
#[tauri::command]
pub fn hybrid_test_review(app:AppHandle,state:State<'_,HybridState>,
    run_id:String,index:usize,attempt:usize,teacher_grade:String,
    corrected_answer:Option<String>,
)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Wait for the test to finish before reviewing".into());}
    if !["correct","incorrect","needsReview"].contains(&teacher_grade.as_str()){
        return Err("Unsupported teacher assessment".into());
    }
    let mut data=test_archive(&state)?;
    let report=data.reports.iter_mut().find(|r|r.id==run_id).ok_or("Run not found")?;
    let question=report.questions.get_mut(index).ok_or("Question not found")?;
    if let Some(target)=corrected_answer{
        if !target.is_empty(){
            if target.len()>256||!target.bytes().all(|c|(32..=126).contains(&c)){
                return Err("Teacher's corrected answer must be 1–256 printable ASCII characters".into());
            }
            // The original key is never overwritten. This is a review note,
            // not an automatic or hidden inference override or weight update.
            question.teacher_answer=Some(target);
        }
    }
    let answer=question.attempts.get_mut(attempt).ok_or("Attempt not found")?;
    answer.teacher_grade=Some(teacher_grade);
    report.recalculate();
    persist_archive(&app,&data)
}
#[cfg(test)]
mod test_runner_tests{
    use super::*;
    #[test]
    fn autonomous_checkpoint_selection_rejects_character_only_progress(){
        let original=AutonomousValidation{
            self_conditioned_loss:3.0,first_character_accuracy:0.50,
            generated_prefix_accuracy:0.25,complete_answers_correct:1,
            complete_answers_checked:2,represented_categories:2,
            max_characters_per_probe:12,
        };
        let worse=AutonomousValidation{
            self_conditioned_loss:3.5,first_character_accuracy:0.0,
            complete_answers_correct:0,..original
        };
        // Next-character validation improved a lot, but Nova would no
        // longer generate the earlier complete answer or its first letter.
        assert!(!autonomous_selection_ok(Some(original),Some(worse),4.0,3.5));
        let recovered=AutonomousValidation{
            self_conditioned_loss:2.8,first_character_accuracy:0.50,
            generated_prefix_accuracy:0.40,
            complete_answers_correct:1,..original
        };
        assert!(autonomous_selection_ok(Some(original),Some(recovered),4.0,3.9));
        // Unlike a simple veto, genuinely better autonomous generation may
        // be selected when teacher-forced heldout loss rises only slightly.
        let much_better=AutonomousValidation{
            self_conditioned_loss:2.2,generated_prefix_accuracy:0.55,..recovered
        };
        assert!(autonomous_selection_ok(Some(original),Some(much_better),4.0,4.04));
        assert!(!autonomous_selection_ok(Some(original),Some(much_better),4.0,4.11));
        // No hidden answer solver, no hardcoded requirement for already-fluent
        // output: 0/2 complete answers can still make measurable progress.
        let novice=AutonomousValidation{
            self_conditioned_loss:4.8,first_character_accuracy:0.0,
            complete_answers_correct:0,..original
        };
        let improving=AutonomousValidation{
            self_conditioned_loss:4.5,generated_prefix_accuracy:0.12,..novice
        };
        assert!(autonomous_selection_ok(Some(novice),Some(improving),5.0,4.8));
    }
    #[test]
    fn joint_selection_cannot_ratchet_heldout_loss_above_original_baseline(){
        let initial=1.4332;
        assert!(stays_near_original_baseline(initial,initial,1.48));
        // Multiple accepted small regressions may not compound into a
        // "NEW BEST" at a much worse heldout loss than session start.
        assert!(!stays_near_original_baseline(initial,1.50,1.57));
        assert!(!stays_near_original_baseline(initial,1.49,2.1174));
        assert!(!stays_near_original_baseline(initial,1.49,f32::NAN));
    }
    #[test]
    fn archived_full_reply_regressions_block_checkpoint_selection(){
        let baseline=AutonomousValidation{
            self_conditioned_loss:1.2,first_character_accuracy:0.75,
            generated_prefix_accuracy:0.708,complete_answers_correct:3,
            complete_answers_checked:4,represented_categories:4,
            max_characters_per_probe:12,
        };
        let deteriorated=AutonomousValidation{
            self_conditioned_loss:1.0,first_character_accuracy:1.0,
            generated_prefix_accuracy:0.76,complete_answers_correct:1,
            ..baseline
        };
        // Even with BETTER prediction loss, first-character performance
        // and prefix accuracy, losing two correct complete replies is a
        // measurable earlier-skill regression.
        assert!(!archived_autonomous_retained(&[baseline],&[deteriorated]));
        assert!(archived_autonomous_retained(&[baseline],&[baseline]));
        let lost_first=AutonomousValidation{
            first_character_accuracy:0.50,..baseline
        };
        assert!(!archived_autonomous_retained(&[baseline],&[lost_first]));
        let lost_prefix=AutonomousValidation{
            generated_prefix_accuracy:0.60,..baseline
        };
        assert!(!archived_autonomous_retained(&[baseline],&[lost_prefix]));
        assert!(!archived_autonomous_retained(&[baseline],&[]));
    }
    #[test]
    fn archived_retention_panels_prefer_newest_and_cover_oldest(){
        let old=prepare("unified",concat!(
            "[Dialogue Lesson]\nGreeting practice.\n[examples]\n",
            "hi  Hello.\nhey  Hey.\nbye  Goodbye.\nthanks  Thanks.\n",
            "good morning  Morning.\n[/examples]\n[/Dialogue Lesson]"
        )).unwrap();
        let new=prepare("unified",concat!(
            "[Dialogue Lesson]\nMore greetings.\n[examples]\n",
            "hello  Hi!\nfarewell  Bye!\nappreciate it  Welcome!\n",
            "goodnight  Night!\ngreetings  Hello!\n",
            "[/examples]\n[/Dialogue Lesson]"
        )).unwrap();
        let p=archived_autonomous_panels(&[old.clone(),new.clone()]);
        assert_eq!(p.len(),2,"older and newer lessons need distinct retention");
        assert_eq!(p[0].len(),1);
        assert_eq!(p[1].len(),1);
        assert_eq!(p[0][0].0,new.heldout_replies[0].0);
        assert_eq!(p[1][0].0,old.heldout_replies[0].0);
        let src=std::slice::from_ref(&new);
        let same=archived_autonomous_panels(src);
        assert_eq!(same.len(),1,"identical oldest and latest must not duplicate");
    }
    #[test]
    fn autonomous_retention_checks_never_mutate_the_neural_checkpoint(){
        let old=prepare("unified",concat!(
            "[Dialogue Lesson]\nGreeting examples.\n[examples]\n",
            "hello  Hello!\nhi  Hi!\nbye  Goodbye!\n",
            "thanks  Thanks!\ngood night  Good night!\n",
            "[/examples]\n[/Dialogue Lesson]"
        )).unwrap();
        let panels=archived_autonomous_panels(&[old]);
        let brain=HybridBrain::new();
        let weights=brain.transformer.weights.clone();
        let momentum=brain.transformer.momentum.clone();
        let step=brain.transformer.step;
        let a=archived_autonomous_scores(&brain,&panels).unwrap();
        let b=archived_autonomous_scores(&brain,&panels).unwrap();
        assert_eq!(a.len(),b.len());
        assert_eq!(a[0].self_conditioned_loss,b[0].self_conditioned_loss);
        assert_eq!(brain.transformer.weights,weights);
        assert_eq!(brain.transformer.momentum,momentum);
        assert_eq!(brain.transformer.step,step);
    }
    #[test]
    fn autonomous_heldout_is_readonly_and_reproducible(){
        let raw=concat!(
            "[Dialogue Lesson]\nGreetings teach reply beginnings and endings.\n",
            "[examples]\nhello  Hello!\nhi  Hi!\nbye  Goodbye!\n",
            "thanks  Thank you!\ngood night  Good night!\n",
            "[/examples]\n[/Dialogue Lesson]"
        );
        let lessons=prepare("unified",raw).unwrap();
        let brain=HybridBrain::new();
        let weights=brain.transformer.weights.clone();
        let momentum=brain.transformer.momentum.clone();
        let step=brain.transformer.step;
        let a=autonomous_validation(&brain,&lessons).unwrap().unwrap();
        let b=autonomous_validation(&brain,&lessons).unwrap().unwrap();
        assert_eq!(a.self_conditioned_loss,b.self_conditioned_loss);
        assert_eq!(a.first_character_accuracy,b.first_character_accuracy);
        assert_eq!(a.generated_prefix_accuracy,b.generated_prefix_accuracy);
        assert_eq!(a.represented_categories,1);
        assert!(a.complete_answers_checked>=1);
        assert!(a.complete_answers_correct<=a.complete_answers_checked);
        assert_eq!(brain.transformer.weights,weights);
        assert_eq!(brain.transformer.momentum,momentum);
        assert_eq!(brain.transformer.step,step);
    }
    #[test]
    fn unified_boundary_training_uses_only_supervised_answer_tokens(){
        let raw=concat!(
            "[Dialogue Lesson]\nGreetings are used to begin conversations.\n",
            "[examples]\nhello  Hello!\nhi  Hi!\nbye  Goodbye!\n",
            "thanks  You're welcome.\ngood morning  Good morning!\n",
            "[/examples]\n[/Dialogue Lesson]"
        );
        let plan=prepare("unified",raw).unwrap();
        let scheduled=reply_boundary_batches(&plan,1,17);
        assert_eq!(scheduled.len(),3);
        for ids in &scheduled {
            assert_eq!(ids.len(),2);
            assert!(ids.iter().all(|&id|id<plan.train.len() &&
                plan.train_kinds[id]==neural_nova::unified::DIALOGUE));
            assert!(plan.train[ids[0]].0.ends_with(b"\nN: "));
            assert_eq!(plan.train[ids[1]].1,b'\n');
            assert!(plan.train[ids[1]].0.windows(4).any(|w|w==b"\nN: "));
        }
        assert!(plan.train.iter().zip(&plan.train_kinds)
            .any(|(_,kind)|*kind==neural_nova::unified::EXPLANATION));
    }
    #[test]
    fn replay_even_tiny_lessons_and_rotate_distinct_archive_sources(){
        let archive=vec![
            prepare("dialogue","hi\tHello!\nbye\tGoodbye!\ncat\tCat.\ndog\tDog.\nfish\tFish.\n").unwrap(),
            prepare("dialogue","hello\tHey!\nthanks\tWelcome.\nsun\tWarm.\nsnow\tCold.\nrain\tWet.\n").unwrap(),
            prepare("reasoning",&(0..6).map(|n|format!(
                "Person {n} has red shoes.\tWhat color are the shoes?\tRed.\tBlue."
            )).collect::<Vec<_>>().join("\n")).unwrap(),
        ];
        let single=balanced_replay_schedule(&archive,1,1);
        assert_eq!(single.len(),1,"one new batch must still rehearse");
        let many=balanced_replay_schedule(&archive,1,10);
        assert_eq!(many.len(),4,"one replay per three or fewer new batches");
        assert!(many.iter().any(|(_,_,kind)|*kind==0));
        assert!(many.iter().any(|(_,_,kind)|*kind==3));
        assert!(many.iter().all(|(source,ids,_)|!ids.is_empty()&&
            ids.iter().all(|&i|i<archive[*source].train.len())));
        let follow=balanced_replay_schedule(&archive,2,10);
        assert_ne!(many,follow,"category/source order must vary across epochs");
    }
    #[test]
    fn contextual_ablation_is_readonly_and_uses_teacher_heldout_rows(){
        let lesson=neural_nova::trainer::prepare("unified",concat!(
            "[Understanding Lesson]\nProperties are represented in sentences.\n[examples]\n",
            "Mira has red shoes. What color are Mira's shoes?  Red.\n",
            "Tim has blue shoes. What color are Tim's shoes?  Blue.\n",
            "Nora has green shoes. What color are Nora's shoes?  Green.\n",
            "Al has black shoes. What color are Al's shoes?  Black.\n",
            "Dee has orange shoes. What color are Dee's shoes?  Orange.\n",
            "[/examples]\n[/Understanding Lesson]"
        )).unwrap();
        let brain=HybridBrain::new();
        let before=brain.transformer.weights.clone();
        let before_step=brain.transformer.step;
        let result=heldout_context_ablation(&brain,&lesson).unwrap();
        if let Some(report)=result{
            assert_eq!(report["teacherOnlyHeldoutAnswers"],true);
            assert!(report["fullAnswerMargin"].as_f64().unwrap().is_finite());
        }
        assert_eq!(brain.transformer.weights,before);
        assert_eq!(brain.transformer.step,before_step);
        let counts=supervised_initials(&lesson);
        assert_eq!(counts["explanationsAreNotReplyTargets"],true);
    }
    #[test]
    fn small_corrective_example_improves_target_probability_in_one_joint_brain(){
        let mut brain=HybridBrain::new();
        let prompt=dialogue_prefix("hello").unwrap();
        let initial=brain.predict_window(&prompt,128).unwrap()[b'H' as usize];
        for _ in 0..6{
            let loss=brain.train_batch(&[(prompt.clone(),b'H')],0.001).unwrap();
            assert!(loss.is_finite());
        }
        let after=brain.predict_window(&prompt,128).unwrap()[b'H' as usize];
        assert!(after>initial,
            "Six real joint gradient updates should improve this controlled target");
        assert_eq!(brain.transformer.step,6);
        assert_eq!(brain.dense_adam_steps(),6);
    }
    #[test]
    fn pure_generator_receives_no_expected_answer_and_is_repeatable(){
        let brain=HybridBrain::new();
        let options=TestOptions{attempts:3,corrections:false,learning_rate:0.001,
            max_chars:12,context_chars:128,deterministic:true,seed:None,
            retention:false,generalization:false};
        let stop=AtomicBool::new(false);
        let a=test_inference(&brain,"hello",&options,1,&stop).unwrap();
        let b=test_inference(&brain,"hello",&options,2,&stop).unwrap();
        assert_eq!(a,b);
    }
}
