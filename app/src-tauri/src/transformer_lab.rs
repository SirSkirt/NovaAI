//! Neural Nova 0.2: independent trainable causal Transformer.
use neural_nova::transformer::{Transformer,Attention,CONTEXT,VOCAB};
use neural_nova::dialogue::{parse_dialogues,dialogue_prefix,DialoguePair};
use serde::{Deserialize,Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Mutex};
use std::time::{SystemTime,UNIX_EPOCH};
use tauri::{AppHandle,Emitter,Manager,State};

const CHECKPOINT:&str="neural-nova-transformer-0.2.ntrf";
const BATCH:usize=8;
const MAX_SAMPLES:usize=12_000;
const MAX_CORPUS_BYTES:usize=14_000;
const LEARNING_RATE:f32=0.003;

struct TrainingStats {
    model:Transformer,
    trained:bool,
    current_epoch:u32,
    total_epochs:u32,
    done:usize,
    total:usize,
    training_loss:Option<f32>,
    heldout_loss:Option<f32>,
    heldout_accuracy:Option<f32>,
    error:Option<String>,
}
struct Shared {
    data:Mutex<TrainingStats>,
    training:AtomicBool,
    stop:AtomicBool,
}
#[derive(Clone)]
pub struct TransformerState{inner:Arc<Shared>}
impl Default for TransformerState {
    fn default()->Self {
        Self{inner:Arc::new(Shared{
            data:Mutex::new(TrainingStats{
                model:Transformer::new(0x4e4f5641_0200),
                trained:false,current_epoch:0,total_epochs:0,
                done:0,total:0,training_loss:None,
                heldout_loss:None,heldout_accuracy:None,error:None
            }),
            training:AtomicBool::new(false),
            stop:AtomicBool::new(false),
        })}
    }
}
fn lock(state:&TransformerState)->Result<std::sync::MutexGuard<'_,TrainingStats>,String>{
    state.inner.data.lock().map_err(|_|"Transformer model lock poisoned".into())
}
fn path(app:&AppHandle)->Result<PathBuf,String>{
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    Ok(dir.join(CHECKPOINT))
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status{
    architecture:String,
    context:usize,
    parameters:usize,
    training:bool,
    trained:bool,
    step:u64,
    epoch:u32,
    epochs:u32,
    completed:usize,
    total:usize,
    training_loss:Option<f32>,
    heldout_loss:Option<f32>,
    heldout_accuracy:Option<f32>,
    error:Option<String>,
}
fn status(state:&TransformerState,brain:&TrainingStats)->Status{
    Status{
        architecture:"ASCII embeddings → causal self-attention → residual feed-forward → next-character probabilities".into(),
        context:CONTEXT, parameters:brain.model.parameter_count(),
        training:state.inner.training.load(Ordering::Acquire),
        trained:brain.trained,step:brain.model.step,
        epoch:brain.current_epoch,epochs:brain.total_epochs,
        completed:brain.done,total:brain.total,
        training_loss:brain.training_loss,heldout_loss:brain.heldout_loss,
        heldout_accuracy:brain.heldout_accuracy,error:brain.error.clone(),
    }
}
#[tauri::command]
pub fn transformer_boot(app:AppHandle,state:State<'_,TransformerState>)->Result<Status,String>{
    if state.inner.training.load(Ordering::Acquire){
        return Err("Stop Transformer training before loading weights".into());
    }
    let mut brain=lock(&state)?;
    let checkpoint=path(&app)?;
    if checkpoint.exists(){
        brain.model=Transformer::load(&checkpoint)?;
        brain.trained=true;
    }
    Ok(status(&state,&brain))
}
#[tauri::command]
pub fn transformer_status(state:State<'_,TransformerState>)->Result<Status,String>{
    let brain=lock(&state)?;
    Ok(status(&state,&brain))
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct AttentionValue {
    index:usize,
    character:String,
    weight:f32,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ChatResult{
    response:String,
    attention:Vec<AttentionValue>,
    status:Status,
}
fn attention_view(raw:Vec<Attention>)->Vec<AttentionValue>{
    raw.into_iter().map(|a|AttentionValue{
        index:a.index,
        character: match a.code {
            b'\n'=>"↵".into(),
            b' '=>"␠".into(),
            32..=126=>char::from(a.code).to_string(),
            other=>format!("{other}"),
        },
        weight:a.probability
    }).collect()
}
fn next_random(seed:&mut u64)->f32 {
    *seed ^= *seed<<13;*seed ^= *seed>>7;*seed ^= *seed<<17;
    (*seed>>40) as f32/16777216.0
}
fn sampled(probs:&[f32],temperature:f32,rand:f32)->u8{
    let mut sum=0.0;
    let mut weights=[0.0f32;VOCAB];
    // Decoder constrains outputs to printable ASCII and trained end-of-reply
    // delimiter; this is not a heuristic answer or fallback.
    for i in 0..VOCAB {
        if i==10 ||(32..127).contains(&i) {
            weights[i]=probs[i].powf(1.0/temperature);
            sum+=weights[i];
        }
    }
    let mut target=rand*sum;
    for (i,w) in weights.iter().enumerate() {
        if *w==0.0 {continue;}
        target-=*w;
        if target<=0.0{return i as u8;}
    }
    b'\n'
}
#[tauri::command]
pub async fn transformer_chat(
    state:State<'_,TransformerState>,message:String,
    max_chars:usize,temperature:f32,
)->Result<ChatResult,String>{
    if !(1..=180).contains(&message.len())||!message.is_ascii() {
        return Err("Use an ASCII question of 1–180 characters".into());
    }
    if message.contains('\n')||message.contains('\r') {
        return Err("Use a single-line question".into());
    }
    if !(1..=160).contains(&max_chars) {return Err("Response length must be 1–160 characters".into());}
    if !temperature.is_finite()||!(0.2..=2.0).contains(&temperature) {
        return Err("Temperature must be 0.2–2.0".into());
    }
    let state=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
        if state.training.load(Ordering::Acquire) {
            return Err("Finish training before generating a reply".into());
        }
        let state=TransformerState{inner:state};
        let brain=lock(&state)?;
        let mut history=dialogue_prefix(&message)?;
        let mut result=Vec::new();
        let mut seed=SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|v|v.as_nanos() as u64).unwrap_or(57).max(1);
        for _ in 0..max_chars {
            let probabilities=brain.model.predict(&history)?;
            let byte=sampled(&probabilities,temperature,next_random(&mut seed));
            if byte==b'\n' {break;}
            history.push(byte);
            result.push(byte);
        }
        let attention=attention_view(brain.model.attention(&history)?);
        Ok(ChatResult{
            response:String::from_utf8(result).map_err(|e|e.to_string())?,
            attention,status:status(&state,&brain),
        })
    }).await.map_err(|e|e.to_string())?
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Lesson{
    mode:String,
    text:String,
    epochs:u32,
}
fn corpus_samples(corpus:&[u8],indices:std::ops::Range<usize>)
    ->Vec<(Vec<u8>,u8)>{
    indices.map(|i|{
        (corpus[i.saturating_sub(CONTEXT)..i].to_vec(),corpus[i])
    }).collect()
}
fn dialogue_samples(pairs:&[DialoguePair])->Result<Vec<(Vec<u8>,u8)>,String>{
    let mut all=Vec::new();
    for pair in pairs {
        let mut history=dialogue_prefix(&pair.user)?;
        for byte in pair.assistant.bytes().chain(std::iter::once(b'\n')){
            all.push((history[history.len().saturating_sub(CONTEXT)..].to_vec(),byte));
            history.push(byte);
        }
    }
    Ok(all)
}
fn datasets(lesson:&Lesson)->Result<(Vec<(Vec<u8>,u8)>,Vec<(Vec<u8>,u8)>,usize),String>{
    if !lesson.text.is_ascii(){return Err("The current 128-token vocabulary is ASCII only".into());}
    match lesson.mode.as_str() {
        "dialogue"=>{
            let pairs=parse_dialogues(&lesson.text)?;
            if pairs.len()<5 {return Err("Provide at least 5 distinct dialogue pairs for meaningful held-out evaluation".into());}
            let valid_count=(pairs.len()/5).max(1);
            let train_count=pairs.len()-valid_count;
            let train=dialogue_samples(&pairs[..train_count])?;
            let valid=dialogue_samples(&pairs[train_count..])?;
            if train.len()>MAX_SAMPLES{return Err("Too many reply characters for this CPU training session".into());}
            Ok((train,valid,valid_count))
        },
        "text"=>{
            let corpus=lesson.text.as_bytes();
            if corpus.len()<120||corpus.len()>MAX_CORPUS_BYTES{
                return Err(format!("Text pretraining requires 120–{MAX_CORPUS_BYTES} ASCII bytes per session"));
            }
            let pivot=(corpus.len()*9/10).max(2);
            let train=corpus_samples(corpus,1..pivot);
            let valid=corpus_samples(corpus,pivot..corpus.len());
            Ok((train,valid,valid.len()))
        },
        _=>Err("Training mode must be text or dialogue".into())
    }
}
#[tauri::command]
pub fn transformer_train(
    app:AppHandle,state:State<'_,TransformerState>,lesson:Lesson
)->Result<(),String>{
    if !(1..=20).contains(&lesson.epochs){
        return Err("Choose 1–20 epochs per Transformer training session".into());
    }
    let (train,valid,heldout)=datasets(&lesson)?;
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Transformer is already training".into());
    }
    let state=state.inner().clone();
    let ready=(||->Result<(),String>{
        let state=TransformerState{inner:state.clone()};
        let mut brain=lock(&state)?;
        brain.current_epoch=1;
        brain.total_epochs=lesson.epochs;
        brain.done=0;
        brain.total=train.len()*lesson.epochs as usize;
        brain.error=None;
        brain.heldout_loss=None;
        brain.heldout_accuracy=None;
        Ok(())
    })();
    if let Err(e)=ready {state.training.store(false,Ordering::Release);return Err(e);}
    state.stop.store(false,Ordering::Release);
    tauri::async_runtime::spawn_blocking(move||{
        let state=TransformerState{inner:state};
        let output=train_inner(&app,&state,&train,&valid,heldout,lesson.epochs);
        if let Err(ref e)=output {
            if let Ok(mut brain)=lock(&state){brain.error=Some(e.clone());}
            eprintln!("Neural Nova Transformer training error: {e}");
        }
        state.inner.training.store(false,Ordering::Release);
        match output {
            Ok(cancelled)=>{let _=app.emit("transformer-done",serde_json::json!({"cancelled":cancelled}));},
            Err(e)=>{let _=app.emit("transformer-error",e);}
        }
    });
    Ok(())
}
fn order(size:usize,epoch:u32)->Vec<usize>{
    let mut indices:Vec<usize>=(0..size).collect();
    let mut seed=0x4e4f5641_2020u64 ^ (epoch as u64).wrapping_mul(0x9e3779b97f4a7c15);
    for i in (1..size).rev(){
        seed^=seed>>12;seed^=seed<<25;seed^=seed>>27;
        indices.swap(i,((seed.wrapping_mul(2685821657736338717))%(i as u64+1)) as usize);
    }
    indices
}
fn train_inner(
    app:&AppHandle,state:&TransformerState,
    training:&[(Vec<u8>,u8)],validation:&[(Vec<u8>,u8)],
    heldout:usize,epochs:u32,
)->Result<bool,String>{
    let mut done=0usize;
    let mut cancelled=false;
    for epoch in 1..=epochs {
        let permutation=order(training.len(),epoch);
        let mut epoch_loss=0.0f64;
        let mut epoch_examples=0usize;
        for chunk in permutation.chunks(BATCH) {
            if state.inner.stop.load(Ordering::Acquire){
                cancelled=true;break;
            }
            let batch:Vec<(Vec<u8>,u8)>=chunk.iter().map(|&i|training[i].clone()).collect();
            let (loss,step)={
                let mut brain=lock(state)?;
                let loss=brain.model.train_batch(&batch,LEARNING_RATE)?;
                brain.training_loss=Some(loss);
                brain.trained=true;
                brain.done=done+batch.len();
                brain.current_epoch=epoch;
                (loss,brain.model.step)
            };
            epoch_examples+=batch.len();
            epoch_loss+=loss as f64*batch.len() as f64;
            done+=batch.len();
            if done%64<batch.len() {
                let _=app.emit("transformer-progress",serde_json::json!({
                    "completed":done,"epoch":epoch,"epochs":epochs,
                    "loss":loss,"step":step
                }));
            }
        }
        let (valid_loss,valid_accuracy)={
            let mut brain=lock(state)?;
            let result=brain.model.evaluate(validation)?;
            brain.heldout_loss=Some(result.0);
            brain.heldout_accuracy=Some(result.1);
            brain.model.save(path(app)?)?;
            result
        };
        let _=app.emit("transformer-epoch",serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "trainLoss":(epoch_loss/epoch_examples.max(1) as f64),
            "validationLoss":valid_loss,
            "validationAccuracy":valid_accuracy,
            "heldoutSamples":heldout,
        }));
        if cancelled{break;}
    }
    Ok(cancelled)
}
#[tauri::command]
pub fn transformer_stop(state:State<'_,TransformerState>)->bool{
    let is_running=state.inner.training.load(Ordering::Acquire);
    if is_running{state.inner.stop.store(true,Ordering::Release);}
    is_running
}
#[tauri::command]
pub fn transformer_reset(app:AppHandle,state:State<'_,TransformerState>)->Result<Status,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Stop training before resetting".into());}
    let mut brain=lock(&state)?;
    brain.model=Transformer::new(0x4e4f5641_0200);
    brain.trained=false;
    brain.current_epoch=0;brain.total_epochs=0;brain.done=0;brain.total=0;
    brain.training_loss=None;brain.heldout_loss=None;brain.heldout_accuracy=None;brain.error=None;
    let checkpoint=path(&app)?;
    if checkpoint.exists(){fs::remove_file(checkpoint).map_err(|e|e.to_string())?;}
    Ok(status(&state,&brain))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn whole_dialogues_are_held_out_from_training(){
        let lesson=Lesson{
            mode:"dialogue".into(),epochs:1,
            text:"hello\tHi!\ngood night\tSleep well.\nwhat is your name\tNova.\nbye\tGoodbye.\nwhat is rain\tFalling water.\n".into()
        };
        let (train,valid,count)=datasets(&lesson).unwrap();
        assert_eq!(count,1);
        assert_eq!(valid.len(),"Falling water.".len()+1);
        assert_eq!(train.len(),"Hi!".len()+"Sleep well.".len()+"Nova.".len()+"Goodbye.".len()+4);
        assert!(train.iter().all(|(history,_)|!history.windows(13).any(|part|part==b"what is rain")));
    }
}
