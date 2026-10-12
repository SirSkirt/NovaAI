//! Native interface for ONE hybrid Nova brain. Both pathways jointly optimize one loss.
use neural_nova::hybrid::HybridBrain;
use neural_nova::dialogue::dialogue_prefix;
use neural_nova::transformer::VOCAB;
use neural_nova::trainer::{prepare,training_order,Example,PreparedLessons,ScheduleConfig,ScheduleState};
use crate::transformer_lab::Lesson;
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
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ConversationTurn{user:String,assistant:String}
struct Shared{
    session:Mutex<Session>,
    training:AtomicBool,
    stop:AtomicBool,
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
                stage:"Idle".into(),early_stopped:false,history:Vec::new()
            }),
            training:AtomicBool::new(false),stop:AtomicBool::new(false)
        })}
    }
}
fn lock(state:&HybridState)->Result<std::sync::MutexGuard<'_,Session>,String>{
    state.inner.session.lock().map_err(|_|"Hybrid model lock poisoned".into())
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
        memory_turns:s.history.len().min(MEMORY_SLOTS),tokenizer_words:s.brain.tokenizer.words.len()
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
#[derive(Clone,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TrainOptions {
    // Accept old 0.3.1 clients as a compatibility fallback only.
    adaptive:Option<bool>,
    adaptive_learning_rate:Option<bool>,
    early_stopping:Option<bool>,
    learning_rate:Option<f32>,
    patience:Option<u32>,
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
        s.early_stopped=false;
        Ok(())
    })();
    if let Err(error)=ready{state.inner.training.store(false,Ordering::Release);return Err(error);}
    state.inner.stop.store(false,Ordering::Release);
    tauri::async_runtime::spawn_blocking(move||{
        let result=train_inner(&app,&state,&plan,lesson.epochs,
            adaptive_learning_rate,early_stopping,starting_lr,patience);
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
    starting_lr:f32,patience:u32,
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
        {
            let mut s=lock(state)?;
            s.epoch=epoch;s.stage=stage.into();s.learning_rate=learning_rate;
        }
        for ids in order.chunks(BATCH){
            if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
            let batch:Vec<Example>=ids.iter().map(|&i|train[i].clone()).collect();
            let (loss,step)={
                let mut s=lock(state)?;
                let loss=if lessons.mode=="conversation"{
                    let extended=ids.iter().map(|&i|(train[i].0.clone(),train[i].1,
                        lessons.memory_train[i].clone())).collect::<Vec<_>>();
                    s.brain.train_memory_batch(&extended,learning_rate)?
                }else{s.brain.train_batch(&batch,learning_rate)?};
                if !loss.is_finite(){return Err("Training loss is non-finite".into());}
                s.trained=true;s.completed=completed+batch.len();s.train_loss=Some(loss);
                s.tokens_per_second=(s.completed as f64/started.elapsed().as_secs_f64().max(0.001)) as f32;
                (loss,s.brain.transformer.step)
            };
            trained_count+=batch.len();
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
        let should_stop=schedule.observe_validation(improved);
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":epoch_loss/trained_count as f64,
            "validationLoss":validation_loss,"validationAccuracy":validation_accuracy,
            "bestLoss":best_loss,"bestEpoch":best_epoch,"improved":improved,
            "learningRate":learning_rate,"tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001),
            "trainLessons":lessons.train_lessons,"validationLessons":lessons.validation_lessons
        }));
        if cancelled{break;}
        if should_stop{
            early_stopped=true;
            break;
        }
    }
    {
        let mut s=lock(state)?;
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
        stage:"Idle".into(),early_stopped:false,history:retained_history
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
