//! Native interface for ONE hybrid Nova brain. Both pathways jointly optimize one loss.
use neural_nova::hybrid::HybridBrain;
use neural_nova::dialogue::dialogue_prefix;
use neural_nova::transformer::VOCAB;
use crate::transformer_lab::{datasets,Lesson};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Mutex};
use std::time::{SystemTime,UNIX_EPOCH};
use tauri::{AppHandle,Emitter,Manager,State};

const CHECKPOINT:&str="neural-nova-hybrid-0.3.nhyb";
const BATCH:usize=8;
const LEARNING_RATE:f32=0.003;

struct Session{
    brain:HybridBrain,trained:bool,
    epoch:u32,epochs:u32,completed:usize,total:usize,
    heldout_loss:Option<f32>,heldout_accuracy:Option<f32>,
    train_loss:Option<f32>,error:Option<String>,
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
                heldout_loss:None,heldout_accuracy:None,train_loss:None,error:None
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
        train_loss:s.train_loss,error:s.error.clone()
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
#[tauri::command]
pub fn hybrid_train(app:AppHandle,state:State<'_,HybridState>,lesson:Lesson)->Result<(),String>{
    if !(1..=20).contains(&lesson.epochs){return Err("Choose 1–20 epochs".into());}
    let (train,valid,_heldout)=datasets(&lesson)?;
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova is already training".into());
    }
    let state=state.inner().clone();
    let ready=(||->Result<(),String>{
        let mut s=lock(&state)?;
        s.epoch=1;s.epochs=lesson.epochs;s.completed=0;
        s.total=train.len()*lesson.epochs as usize;
        s.heldout_loss=None;s.heldout_accuracy=None;s.train_loss=None;s.error=None;
        Ok(())
    })();
    if let Err(e)=ready{state.inner.training.store(false,Ordering::Release);return Err(e);}
    state.inner.stop.store(false,Ordering::Release);
    tauri::async_runtime::spawn_blocking(move||{
        let result=train_inner(&app,&state,&train,&valid,lesson.epochs);
        if let Err(ref e)=result{
            if let Ok(mut s)=lock(&state){s.error=Some(e.clone());}
            eprintln!("Neural Nova hybrid training error: {e}");
        }
        state.inner.training.store(false,Ordering::Release);
        match result{
            Ok(cancelled)=>{let _=app.emit("hybrid-done",serde_json::json!({"cancelled":cancelled}));},
            Err(error)=>{let _=app.emit("hybrid-error",error);}
        }
    });
    Ok(())
}
fn train_inner(
    app:&AppHandle,state:&HybridState,training:&[(Vec<u8>,u8)],
    validation:&[(Vec<u8>,u8)],epochs:u32,
)->Result<bool,String>{
    let mut completed=0usize;
    let mut cancelled=false;
    for epoch in 1..=epochs{
        let mut epoch_loss=0f64;
        let mut trained_count=0usize;
        // Deterministic shuffle, preserving whole held-out dialogue pairs.
        let mut order:Vec<usize>=(0..training.len()).collect();
        let mut seed=(epoch as u64).wrapping_mul(0x9e3779b97f4a7c15)^0x4e4f5641;
        for i in (1..order.len()).rev(){
            seed^=seed>>12;seed^=seed<<25;seed^=seed>>27;
            order.swap(i,((seed.wrapping_mul(2685821657736338717))%(i as u64+1)) as usize);
        }
        for ids in order.chunks(BATCH){
            if state.inner.stop.load(Ordering::Acquire){cancelled=true;break;}
            let batch=ids.iter().map(|&i|training[i].clone()).collect::<Vec<_>>();
            let (loss,step)={
                let mut s=lock(state)?;
                let loss=s.brain.train_batch(&batch,LEARNING_RATE)?;
                s.trained=true;s.epoch=epoch;s.completed=completed+batch.len();s.train_loss=Some(loss);
                (loss,s.brain.transformer.step)
            };
            completed+=batch.len();trained_count+=batch.len();
            epoch_loss+=loss as f64*batch.len() as f64;
            if completed%64<batch.len(){
                let _=app.emit("hybrid-progress",serde_json::json!({
                    "completed":completed,"total":training.len()*epochs as usize,
                    "epoch":epoch,"epochs":epochs,"loss":loss,"step":step
                }));
            }
        }
        let (validation_loss,validation_accuracy)={
            let mut s=lock(state)?;
            let metrics=s.brain.evaluate(validation)?;
            s.heldout_loss=Some(metrics.0);s.heldout_accuracy=Some(metrics.1);
            s.brain.save(path(app)?)?;
            metrics
        };
        let _=app.emit("hybrid-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":epoch_loss/trained_count.max(1) as f64,
            "validationLoss":validation_loss,"validationAccuracy":validation_accuracy
        }));
        if cancelled{break;}
    }
    Ok(cancelled)
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
        heldout_loss:None,heldout_accuracy:None,train_loss:None,error:None
    };
    let file=path(&app)?;
    if file.exists(){fs::remove_file(file).map_err(|e|e.to_string())?;}
    Ok(status(&state,&s))
}
