//! Native interface for ONE hybrid Nova brain. Both pathways jointly optimize one loss.
use neural_nova::hybrid::HybridBrain;
use neural_nova::dialogue::dialogue_prefix;
use neural_nova::transformer::VOCAB;
use neural_nova::trainer::{prepare,training_order,Example,PreparedLessons,ScheduleConfig,ScheduleState};
use crate::transformer_lab::Lesson;
use crate::test_lab::{TestArchive,TestDefinition,TestReport,TestOptions,TestCase,QuestionResult,Attempt,parse_cases,grade,validate_options};
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
    if state.inner.training.load(Ordering::Acquire){return Err("Cannot boot while Nova is training".into());}
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
    state:State<'_,HybridState>,message:String,context_chars:Option<usize>,
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
        trace(&s.brain,&context,window,&memory)
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
    let starting_lr=options.as_ref().and_then(|o|o.learning_rate).unwrap_or(DEFAULT_LEARNING_RATE);
    let patience=options.as_ref().and_then(|o|o.patience).unwrap_or(6);
    if !starting_lr.is_finite() || !(0.0003..=0.006).contains(&starting_lr){
        return Err("Learning rate must be between 0.0003 and 0.006".into());
    }
    if !(3..=15).contains(&patience){return Err("Patience must be 3–15 epochs".into());}
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova is already training".into());
    }
    let state=state.inner().clone();
    let ready=(||->Result<(),String>{
        let mut s=lock(&state)?;
        s.brain.learn_token_units(&plan.training_text);
        s.epoch=0;s.epochs=lesson.epochs;s.completed=0;
        s.total=plan.train.len()*lesson.epochs as usize;
        s.heldout_loss=None;s.heldout_accuracy=None;s.train_loss=None;s.error=None;
        s.best_heldout_loss=None;s.best_epoch=0;s.learning_rate=starting_lr;
        s.train_lessons=plan.train_lessons;s.validation_lessons=plan.validation_lessons;
        s.tokens_per_second=0.0;s.stage="Preparing held-out baseline".into();
        s.early_stopped=false;s.retention_baseline=None;s.retention_accuracy=None;
        Ok(())
    })();
    if let Err(error)=ready{state.inner.training.store(false,Ordering::Release);return Err(error);}
    state.inner.stop.store(false,Ordering::Release);
    tauri::async_runtime::spawn_blocking(move||{
        let result=train_inner(&app,&state,&plan,lesson.epochs,
            adaptive_learning_rate,early_stopping,starting_lr,patience,&retention);
        if let Err(ref e)=result{
            if let Ok(mut s)=lock(&state){s.error=Some(e.clone());s.stage="Training failed".into();}
            eprintln!("Neural Nova hybrid trainer error: {e}");
        }
        state.inner.training.store(false,Ordering::Release);
        match result{
            Ok((cancelled,early_stopped,best_epoch))=>{
                let _=app.emit("hybrid-done",serde_json::json!({
                    "cancelled":cancelled,"earlyStopped":early_stopped,"bestEpoch":best_epoch
                }));
            },
            Err(error)=>{let _=app.emit("hybrid-error",error);}
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
fn train_inner(
    app:&AppHandle,state:&HybridState,lessons:&PreparedLessons,
    epochs:u32,adaptive_learning_rate:bool,early_stopping:bool,
    starting_lr:f32,patience:u32,retention:&[TestCase],
)->Result<(bool,bool,u32),String>{
    let train:&[Example]=&lessons.train;
    let valid:&[Example]=&lessons.validation;
    let started=Instant::now();
    let (mut best_brain,mut best_loss,mut best_accuracy,initially_trained)={
        let mut s=lock(state)?;
        let (loss,accuracy)=if lessons.mode=="conversation"{
            evaluate_memory(&s.brain,valid,&lessons.memory_validation)?
        }else{s.brain.evaluate(valid)?};
        if !loss.is_finite(){return Err("Non-finite baseline validation loss".into());}
        s.heldout_loss=Some(loss);s.heldout_accuracy=Some(accuracy);
        s.best_heldout_loss=Some(loss);
        (s.brain.clone(),loss,accuracy,s.trained)
    };
    // Diagnostic baseline is measured after existing vocabulary expansion.
    // The dedicated tokenizer-only A/B command measures the earlier stage.
    if !retention.is_empty(){
        let baseline=retention_accuracy(&lock(state)?.brain,retention)?;
        let mut s=lock(state)?;
        s.retention_baseline=Some(baseline);
        s.retention_accuracy=Some(baseline);
    }
    let mut best_epoch=0u32;
    let mut schedule=ScheduleState::new(starting_lr,ScheduleConfig{
        adaptive_learning_rate,early_stopping,patience,
    });
    let mut completed=0usize;
    let mut cancelled=false;
    let mut early_stopped=false;
    for epoch in 1..=epochs{
        if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
        let stage=if epoch==1&&lessons.mode=="dialogue"{
            "Warm-up: short contexts first"
        }else{"Practice: shuffled examples"};
        let learning_rate=schedule.learning_rate;
        let order=training_order(train,epoch,&lessons.mode);
        let mut epoch_loss=0.0f64;
        let mut trained_count=0usize;
        let mut batches=0usize;
        let mut transformer_grad_norm=0.0f64;
        let mut dense_grad_norm=0.0f64;
        let mut bridge_grad_norm=0.0f64;
        let mut transformer_clip_batches=0usize;
        let mut dense_clip_batches=0usize;
        {
            let mut s=lock(state)?;
            s.epoch=epoch;s.stage=stage.into();s.learning_rate=learning_rate;
        }
        for ids in order.chunks(BATCH){
            if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
            let batch:Vec<Example>=ids.iter().map(|&i|train[i].clone()).collect();
            let (loss,step,diag)={
                let mut s=lock(state)?;
                let loss=if lessons.mode=="conversation"{
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
            trained_count+=batch.len();
            batches+=1;
            transformer_grad_norm+=diag.transformer_norm as f64;
            dense_grad_norm+=diag.dense_norm as f64;
            bridge_grad_norm+=diag.bridge_norm as f64;
            transformer_clip_batches+=usize::from(diag.transformer_clipped);
            dense_clip_batches+=usize::from(diag.dense_clipped);
            completed+=batch.len();
            epoch_loss+=loss as f64*batch.len() as f64;
            if completed%64<batch.len(){
                let _=app.emit("hybrid-progress",serde_json::json!({
                    "completed":completed,"total":train.len()*epochs as usize,
                    "epoch":epoch,"epochs":epochs,"loss":loss,"step":step,
                    "stage":stage,"learningRate":learning_rate,
                    "tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001)
                }));
            }
        }
        if trained_count==0 {break;}
        let (validation_loss,validation_accuracy,improved)={
            let mut s=lock(state)?;
            s.stage="Evaluating held-out lessons".into();
            let (loss,accuracy)=if lessons.mode=="conversation"{
                evaluate_memory(&s.brain,valid,&lessons.memory_validation)?
            }else{s.brain.evaluate(valid)?};
            if !loss.is_finite(){return Err("Non-finite validation loss".into());}
            let improved=loss + MIN_IMPROVEMENT < best_loss;
            if improved{
                best_loss=loss;best_accuracy=accuracy;best_epoch=epoch;
                best_brain=s.brain.clone();
                s.brain.save(path(app)?)?;
            }
            s.best_epoch=best_epoch;
            s.best_heldout_loss=Some(best_loss);
            s.heldout_loss=Some(loss);
            s.heldout_accuracy=Some(accuracy);
            (loss,accuracy,improved)
        };
        let retention_score=if retention.is_empty(){None}else{
            let snapshot=lock(state)?.brain.clone();
            Some(retention_accuracy(&snapshot,retention)?)
        };
        if let Some(value)=retention_score{lock(state)?.retention_accuracy=Some(value);}
        let should_stop=schedule.observe_validation(improved);
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":epoch_loss/trained_count as f64,
            "validationLoss":validation_loss,"validationAccuracy":validation_accuracy,
            "bestLoss":best_loss,"bestEpoch":best_epoch,"improved":improved,
            "learningRate":learning_rate,"tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001),
            "trainLessons":lessons.train_lessons,"validationLessons":lessons.validation_lessons,
            "retentionAccuracy":retention_score,"retentionQuestions":retention.len(),
            "transformerGradientNorm":transformer_grad_norm/batches.max(1) as f64,
            "denseGradientNorm":dense_grad_norm/batches.max(1) as f64,
            "bridgeGradientNorm":bridge_grad_norm/batches.max(1) as f64,
            "transformerClipFraction":transformer_clip_batches as f64/batches.max(1) as f64,
            "denseClipFraction":dense_clip_batches as f64/batches.max(1) as f64
        }));
        if cancelled{break;}
        if should_stop{
            early_stopped=true;
            break;
        }
    }
    let restored_retention=if retention.is_empty(){None}
        else{Some(retention_accuracy(&best_brain,retention)?)};
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
    Ok((cancelled,early_stopped,best_epoch))
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
    // Phase 1: ALL baseline questions must be answered before ANY correction.
    // Otherwise Q2's "initial score" would already have benefited from Q1.
    for index in 0..test.cases.len(){
        if !test_wait(state){report.status="stopped".into();return Ok(());}
        if let Some(answer)=test_attempt(state,&test.cases[index],o,index as u64)?{
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
                    test_record(app,state,report)?;
                }else{report.status="stopped".into();return Ok(());}
            }
            if !test_wait(state){report.status="stopped".into();return Ok(());}
            if let Some(attempt)=test_attempt(state,case,o,
                100_000+(index as u64)*16+attempt_no as u64)?{
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
        Ok(s)=>s.brain.transformer.step,
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
