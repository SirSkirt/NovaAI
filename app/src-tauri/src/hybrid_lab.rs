//! Native interface for ONE hybrid Nova brain. Both pathways jointly optimize one loss.
use neural_nova::hybrid::HybridBrain;
use neural_nova::dialogue::dialogue_prefix;
use neural_nova::transformer::VOCAB;
use neural_nova::trainer::{prepare,training_order,Example,PreparedLessons};
use crate::transformer_lab::Lesson;
use serde::{Deserialize,Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Mutex};
use std::time::{Instant,SystemTime,UNIX_EPOCH};
use tauri::{AppHandle,Emitter,Manager,State};

const CHECKPOINT:&str="neural-nova-hybrid-0.3.nhyb";
const BATCH:usize=8;
const DEFAULT_LEARNING_RATE:f32=0.003;
const MIN_IMPROVEMENT:f32=0.0005;
const MIN_LEARNING_RATE:f32=0.0003;

struct Session{
    brain:HybridBrain,trained:bool,
    epoch:u32,epochs:u32,completed:usize,total:usize,
    heldout_loss:Option<f32>,heldout_accuracy:Option<f32>,
    train_loss:Option<f32>,error:Option<String>,
    best_heldout_loss:Option<f32>,best_epoch:u32,learning_rate:f32,
    train_lessons:usize,validation_lessons:usize,
    tokens_per_second:f32,stage:String,early_stopped:bool,
}
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
                stage:"Idle".into(),early_stopped:false
            }),
            training:AtomicBool::new(false),stop:AtomicBool::new(false)
        })}
    }
}
fn lock(state:&HybridState)->Result<std::sync::MutexGuard<'_,Session>,String>{
    state.inner.session.lock().map_err(|_|"Hybrid model lock poisoned".into())
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
}
fn status(state:&HybridState,s:&Session)->HybridStatus{
    HybridStatus{
        training:state.inner.training.load(Ordering::Acquire),
        trained:s.trained,parameters:s.brain.parameter_count(),
        transformer_parameters:s.brain.transformer.parameter_count(),
        dense_parameters:s.brain.dense.parameter_count(),
        transformer_step:s.brain.transformer.step,dense_step:s.brain.dense_adam_steps(),
        context:128,epoch:s.epoch,epochs:s.epochs,completed:s.completed,total:s.total,
        heldout_loss:s.heldout_loss,heldout_accuracy:s.heldout_accuracy,
        train_loss:s.train_loss,error:s.error.clone(),
        best_heldout_loss:s.best_heldout_loss,best_epoch:s.best_epoch,
        learning_rate:s.learning_rate,
        train_lessons:s.train_lessons,validation_lessons:s.validation_lessons,
        tokens_per_second:s.tokens_per_second,stage:s.stage.clone(),early_stopped:s.early_stopped
    }
}
#[tauri::command]
pub fn hybrid_boot(app:AppHandle,state:State<'_,HybridState>)->Result<HybridStatus,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Cannot boot while Nova is training".into());}
    let mut s=lock(&state)?;
    let file=path(&app)?;
    if file.exists(){s.brain=HybridBrain::load(file)?;s.trained=true;}
    Ok(status(&state,&s))
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
fn trace(brain:&HybridBrain,history:&[u8])->Result<HybridTrace,String>{
    let (layers,attention,bridge,p)=brain.inspect(history)?;
    let mut sorted=(0..p.len()).collect::<Vec<usize>>();
    sorted.sort_by(|&a,&b|p[b].total_cmp(&p[a]));
    Ok(HybridTrace{
        dense_layers:layers,bridge_signal:bridge,
        attention:attention.into_iter().map(|a|AttentionValue{
            index:a.index,weight:a.probability,
            character:match a.code{b' '=>"␠".into(),b'\n'=>"↵".into(),32..=126=>char::from(a.code).to_string(),other=>format!("{other}")}
        }).collect(),
        top_predictions:sorted.into_iter().take(8).map(|code|TokenProbability{code,probability:p[code]}).collect(),
    })
}
#[tauri::command]
pub async fn hybrid_inspect(state:State<'_,HybridState>,message:String)->Result<HybridTrace,String>{
    if message.is_empty() || !message.is_ascii() || message.len()>180{
        return Err("Inspect 1–180 ASCII characters".into());
    }
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){return Err("Wait for training to finish".into());}
        let s=lock(&state)?;
        let context=dialogue_prefix(&message)?;
        trace(&s.brain,&context)
    }).await.map_err(|e|e.to_string())?
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
pub struct HybridReply{response:String,trace:HybridTrace,status:HybridStatus}
#[tauri::command]
pub async fn hybrid_chat(state:State<'_,HybridState>,message:String,max_chars:usize,temperature:f32)->Result<HybridReply,String>{
    if message.is_empty() || message.len()>180 || !message.is_ascii() || message.contains('\n') || message.contains('\r') {
        return Err("Use a single-line 1–180 character ASCII message".into());
    }
    if !(1..=160).contains(&max_chars) || !temperature.is_finite() || !(0.2..=2.0).contains(&temperature) {
        return Err("Invalid generation length or temperature".into());
    }
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.inner.training.load(Ordering::Acquire){return Err("Wait for training to finish".into());}
        let s=lock(&state)?;
        let mut history=dialogue_prefix(&message)?;
        let mut result=Vec::<u8>::new();
        let mut seed=SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|v|v.as_nanos() as u64).unwrap_or(41).max(1);
        for _ in 0..max_chars{
            let p=s.brain.predict(&history)?;
            let token=sampled(&p,temperature,random(&mut seed));
            if token==b'\n'{break;}
            history.push(token);
            result.push(token);
        }
        Ok(HybridReply{
            response:String::from_utf8(result).map_err(|e|e.to_string())?,
            trace:trace(&s.brain,&history)?,status:status(&state,&s)
        })
    }).await.map_err(|e|e.to_string())?
}
#[derive(Clone,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TrainOptions {
    adaptive:Option<bool>,
    learning_rate:Option<f32>,
    patience:Option<u32>,
}
#[tauri::command]
pub fn hybrid_train(
    app:AppHandle,state:State<'_,HybridState>,lesson:Lesson,
    options:Option<TrainOptions>,
)->Result<(),String>{
    if !(1..=60).contains(&lesson.epochs){return Err("Choose 1–60 epochs".into());}
    let plan=prepare(&lesson.mode,&lesson.text)?;
    let adaptive=options.as_ref().and_then(|o|o.adaptive).unwrap_or(true);
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
        let result=train_inner(&app,&state,&plan,lesson.epochs,adaptive,starting_lr,patience);
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
fn train_inner(
    app:&AppHandle,state:&HybridState,lessons:&PreparedLessons,
    epochs:u32,adaptive:bool,starting_lr:f32,patience:u32,
)->Result<(bool,bool,u32),String>{
    let train:&[Example]=&lessons.train;
    let valid:&[Example]=&lessons.validation;
    let started=Instant::now();
    let (mut best_brain,mut best_loss,initially_trained)={
        let mut s=lock(state)?;
        let (loss,accuracy)=s.brain.evaluate(valid)?;
        if !loss.is_finite(){return Err("Non-finite baseline validation loss".into());}
        s.heldout_loss=Some(loss);s.heldout_accuracy=Some(accuracy);
        s.best_heldout_loss=Some(loss);
        (s.brain.clone(),loss,s.trained)
    };
    let mut best_epoch=0u32;
    let mut learning_rate=starting_lr;
    let mut stale_epochs=0u32;
    let mut completed=0usize;
    let mut cancelled=false;
    let mut early_stopped=false;
    for epoch in 1..=epochs{
        if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
        let stage=if epoch==1&&lessons.mode=="dialogue"{
            "Warm-up: short contexts first"
        }else{"Practice: shuffled examples"};
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
                let loss=s.brain.train_batch(&batch,learning_rate)?;
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
            let (loss,accuracy)=s.brain.evaluate(valid)?;
            if !loss.is_finite(){return Err("Non-finite validation loss".into());}
            let improved=loss + MIN_IMPROVEMENT < best_loss;
            if improved{
                best_loss=loss;best_epoch=epoch;
                best_brain=s.brain.clone();
                s.brain.save(path(app)?)?;
            }
            s.best_epoch=best_epoch;
            s.best_heldout_loss=Some(best_loss);
            s.heldout_loss=Some(loss);
            s.heldout_accuracy=Some(accuracy);
            (loss,accuracy,improved)
        };
        if improved {stale_epochs=0;}
        else{stale_epochs+=1;}
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":epoch_loss/trained_count as f64,
            "validationLoss":validation_loss,"validationAccuracy":validation_accuracy,
            "bestLoss":best_loss,"bestEpoch":best_epoch,"improved":improved,
            "learningRate":learning_rate,"tokensPerSecond":completed as f64/started.elapsed().as_secs_f64().max(0.001),
            "trainLessons":lessons.train_lessons,"validationLessons":lessons.validation_lessons
        }));
        if cancelled{break;}
        if adaptive{
            if stale_epochs>=patience {
                early_stopped=true;
                break;
            }
            if stale_epochs>0 && stale_epochs%2==0{
                learning_rate=(learning_rate*0.5).max(MIN_LEARNING_RATE);
            }
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
        s.learning_rate=learning_rate;
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
    *s=Session{
        brain:HybridBrain::new(),trained:false,epoch:0,epochs:0,completed:0,total:0,
        heldout_loss:None,heldout_accuracy:None,train_loss:None,error:None,
        best_heldout_loss:None,best_epoch:0,learning_rate:DEFAULT_LEARNING_RATE,
        train_lessons:0,validation_lessons:0,tokens_per_second:0.0,
        stage:"Idle".into(),early_stopped:false
    };
    let file=path(&app)?;
    if file.exists(){fs::remove_file(file).map_err(|e|e.to_string())?;}
    Ok(status(&state,&s))
}
