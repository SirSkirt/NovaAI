//! Nova-2 native app runtime. Separate v2 weights, checkpoints and training session;
//! existing v1 .nhyb checkpoints are never imported, overwritten or deleted.
use neural_nova::nova2::{Nova2,Ablation,Focus,KvCache};
use neural_nova::trainer::{self,Example};
use std::path::PathBuf;
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use tauri::{AppHandle,Manager,Emitter,State};
use serde::{Serialize,Deserialize};

const CHECKPOINT:&str="neural-nova-bicameral-v2.nnova2";
const CHAT:&str="neural-nova-bicameral-conversation.json";
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Turn{user:String,assistant:String}
struct Session{
    brain:Nova2,trained:bool,epoch:u32,epochs:u32,
    completed:usize,total:usize,loss:Option<f32>,heldout:Option<f32>,
    stage:String,error:Option<String>,history:Vec<Turn>,lesson:String,
}
struct Inner{
    session:Mutex<Session>,training:AtomicBool,stop:AtomicBool,
}
#[derive(Clone)]
pub struct Nova2State{inner:Arc<Inner>}
impl Nova2State{
    pub(crate) fn is_training(&self)->bool{
        self.inner.training.load(Ordering::Acquire)
    }
}
impl Default for Nova2State{
    fn default()->Self{
        Self{inner:Arc::new(Inner{
            session:Mutex::new(Session{
                brain:Nova2::new(),trained:false,epoch:0,epochs:0,
                completed:0,total:0,loss:None,heldout:None,
                stage:"Nova-2 ready for teaching".into(),error:None,
                history:Vec::new(),lesson:String::new()
            }),
            training:AtomicBool::new(false),stop:AtomicBool::new(false),
        })}
    }
}
fn locked(s:&Nova2State)->Result<std::sync::MutexGuard<'_,Session>,String>{
    s.inner.session.lock().map_err(|_|"Nova-2 model lock poisoned".into())
}
fn file(app:&AppHandle,name:&str)->Result<PathBuf,String>{
    let directory=app.path().app_data_dir().map_err(|e|e.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
    Ok(directory.join(name))
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status{
    trained:bool,training:bool,parameters:usize,
    transformer_parameters:usize,dense_parameters:usize,interconnect_parameters:usize,
    step:u64,epoch:u32,epochs:u32,completed:usize,total:usize,
    loss:Option<f32>,heldout_loss:Option<f32>,stage:String,error:Option<String>,
    words:usize,context:usize,lesson:String,cache_enabled:bool,
}
fn status(s:&Nova2State,m:&Session)->Status{
    Status{
        trained:m.trained,training:s.inner.training.load(Ordering::Acquire),
        parameters:m.brain.parameter_count(),
        transformer_parameters:m.brain.group_parameters(Focus::Left),
        dense_parameters:m.brain.group_parameters(Focus::Right),
        interconnect_parameters:m.brain.group_parameters(Focus::Interconnect),
        step:m.brain.step,epoch:m.epoch,epochs:m.epochs,
        completed:m.completed,total:m.total,loss:m.loss,heldout_loss:m.heldout,
        stage:m.stage.clone(),error:m.error.clone(),
        words:m.brain.tokenizer.words.len(),context:m.brain.config.max_tokens,
        lesson:m.lesson.clone(),cache_enabled:true,
    }
}
#[tauri::command]
pub fn nova2_boot(app:AppHandle,state:State<'_,Nova2State>)->Result<Status,String>{
    if state.inner.training.load(Ordering::Acquire){
        let s=locked(&state)?;return Ok(status(&state,&s));
    }
    let mut s=locked(&state)?;
    let p=file(&app,CHECKPOINT)?;
    if p.exists(){s.brain=Nova2::load(&p)?;s.trained=true;}
    let chat=file(&app,CHAT)?;
    if chat.exists(){
        let bytes=std::fs::read(chat).map_err(|e|e.to_string())?;
        let turns:Vec<Turn>=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
        if turns.len()<=12 && turns.iter().all(|t|
            t.user.is_ascii()&&t.assistant.is_ascii()&&t.user.len()<=300&&t.assistant.len()<=512){
            s.history=turns;
        }
    }
    Ok(status(&state,&s))
}
#[tauri::command]
pub fn nova2_status(state:State<'_,Nova2State>)->Result<Status,String>{
    let s=locked(&state)?;Ok(status(&state,&s))
}
#[tauri::command]
pub fn nova2_conversation(state:State<'_,Nova2State>)->Result<Vec<Turn>,String>{
    Ok(locked(&state)?.history.clone())
}
#[tauri::command]
pub fn nova2_new_chat(app:AppHandle,state:State<'_,Nova2State>)->Result<(),String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Training in progress".into());}
    let mut s=locked(&state)?;
    std::fs::write(file(&app,CHAT)?,b"[]").map_err(|e|e.to_string())?;
    s.history.clear();Ok(())
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct ChatReply{reply:String,generated_tokens:usize,cache_attention_keys:usize,step:u64}
#[tauri::command]
pub fn nova2_chat(app:AppHandle,state:State<'_,Nova2State>,message:String,
    max_tokens:Option<usize>)->Result<ChatReply,String>{
    if message.trim().is_empty()||!message.is_ascii()||message.len()>300{
        return Err("Write a 1–300 byte ASCII message".into());
    }
    if state.inner.training.load(Ordering::Acquire){return Err("Finish teaching before chatting".into());}
    let (brain,history)={
        let s=locked(&state)?;(s.brain.clone(),s.history.clone())
    };
    let mut prompt=String::new();
    for turn in history.iter().rev().take(4).rev(){
        prompt.push_str("U: ");prompt.push_str(&turn.user);prompt.push_str("\nN: ");
        prompt.push_str(&turn.assistant);prompt.push('\n');
    }
    prompt.push_str("U: ");prompt.push_str(&message);prompt.push_str("\nN: ");
    let ids=brain.tokenizer.encode(&prompt)?;
    let mut cache=KvCache::new(&brain,Ablation::default());
    let mut distribution=Vec::new();
    for &id in &ids{distribution=brain.predict_cached(&mut cache,id)?;}
    if distribution.is_empty(){return Err("Unable to encode the prompt".into());}
    let mut output=Vec::new();
    for _ in 0..max_tokens.unwrap_or(64).clamp(1,256){
        let next=distribution.iter().enumerate()
            .max_by(|a,b|a.1.total_cmp(b.1)).ok_or("Empty Nova-2 prediction")?.0 as u16;
        if next==b'\n' as u16{break;}
        output.push(next);distribution=brain.predict_cached(&mut cache,next)?;
    }
    let reply=brain.tokenizer.decode(&output)?;
    let mut s=locked(&state)?;
    s.history.push(Turn{user:message,assistant:reply.clone()});
    if s.history.len()>12{s.history.remove(0);}
    let history=serde_json::to_vec(&s.history).map_err(|e|e.to_string())?;
    std::fs::write(file(&app,CHAT)?,history).map_err(|e|e.to_string())?;
    Ok(ChatReply{reply,generated_tokens:output.len(),
        cache_attention_keys:cache.stats().attention_keys_read,step:brain.step})
}
#[tauri::command]
pub fn nova2_reset(app:AppHandle,state:State<'_,Nova2State>)->Result<Status,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Stop training first".into());}
    let mut s=locked(&state)?;
    let p=file(&app,CHECKPOINT)?;
    if p.exists(){std::fs::remove_file(&p).map_err(|e|e.to_string())?;}
    std::fs::write(file(&app,CHAT)?,b"[]").map_err(|e|e.to_string())?;
    s.brain=Nova2::new();s.trained=false;s.history.clear();
    s.epoch=0;s.epochs=0;s.total=0;s.completed=0;
    s.error=None;s.loss=None;s.heldout=None;s.stage="Fresh Nova-2 brain".into();
    Ok(status(&state,&s))
}
#[derive(Clone)]
struct Unit{prompt:Vec<u8>,answer:Vec<u8>}
fn units(examples:&[Example])->Vec<Unit>{
    let mut out:Vec<Unit>=Vec::new();
    for (context,target) in examples{
        let should_append=out.last().is_some_and(|last|{
            let mut expected=last.prompt.clone();
            expected.extend_from_slice(&last.answer);
            let suffix=&expected[expected.len().saturating_sub(512)..];
            suffix==context.as_slice()
        });
        if should_append{
            out.last_mut().unwrap().answer.push(*target);
        }else{
            out.push(Unit{prompt:context.clone(),answer:vec![*target]});
        }
    }
    out
}
fn sample(brain:&Nova2,u:&Unit)->Result<(Vec<u16>,Vec<u16>,Vec<f32>),String>{
    let mut seq=u.prompt.clone();seq.extend_from_slice(&u.answer);
    let text=std::str::from_utf8(&seq).map_err(|e|e.to_string())?;
    let all=brain.tokenizer.encode(text)?;
    let start=all.len().saturating_sub(brain.config.max_tokens+1);
    let segment=&all[start..];
    if segment.len()<2{return Err("Empty training token span".into());}
    let prompt=std::str::from_utf8(&u.prompt).map_err(|e|e.to_string())?;
    let prompt_len=brain.tokenizer.encode(prompt)?.len();
    let mut mask=Vec::new();
    for idx in start+1..all.len(){
        mask.push(if idx>=prompt_len{1.}else{0.2});
    }
    Ok((segment[..segment.len()-1].to_vec(),segment[1..].to_vec(),mask))
}
fn heldout_loss(brain:&Nova2,held:&[Unit])->Result<f32,String>{
    if held.is_empty(){return Err("No heldout lesson examples".into());}
    let mut total=0.;let mut n=0;
    for unit in held.iter().step_by((held.len()/12).max(1)).take(12){
        let (input,targets,mask)=sample(brain,unit)?;
        let logits=brain.logits(&input,Ablation::default())?;
        let v=brain.config.vocab;
        for (i,&goal) in targets.iter().enumerate(){
            if mask[i]<0.9{continue;}
            let row=&logits[i*v..(i+1)*v];
            let top=row.iter().copied().fold(f32::NEG_INFINITY,f32::max);
            let sum=row.iter().map(|x|(x-top).exp()).sum::<f32>();
            total+=top+sum.ln()-row[goal as usize];n+=1;
        }
    }
    if n==0{return Err("Heldout probes contain no answer tokens".into());}
    Ok(total/n as f32)
}
fn rate(step:u64,peak:f32)->f32{
    let st=step as f32;
    if st<50.{return peak*(st+1.)/50.;}
    let phase=((st-50.)/1950.).clamp(0.,1.);
    peak*(0.1+0.9*0.5*(1.+(std::f32::consts::PI*phase).cos()))
}
fn train_one(app:&AppHandle,state:&Nova2State,mode:&str,text:&str,epochs:u32,
    peak:f32,label:&str)->Result<(),String>{
    let prepared=trainer::prepare(mode,text)?;
    let train=units(&prepared.train);let valid=units(&prepared.validation);
    if train.is_empty()||valid.is_empty(){return Err("Lesson has no train/heldout examples".into());}
    let mut brain={locked(state)?.brain.clone()};
    brain.tokenizer.grow_from_corpus(&prepared.training_text);
    {
        let mut s=locked(state)?;s.lesson=label.into();s.epochs=epochs;
        s.total=train.len()*epochs as usize;s.completed=0;s.stage="Training every token position".into();
    }
    for epoch in 0..epochs{
        if state.inner.stop.load(Ordering::Acquire){break;}
        let mut cumulative=0.;let mut count=0;
        let mut order=(0..train.len()).collect::<Vec<_>>();
        if epoch>0{
            let len=order.len().max(1);
            // Deterministic epoch rotation; no teacher-based answer decisions.
            order.rotate_left((epoch as usize*37)%len);
        }
        for idx in order {
            if state.inner.stop.load(Ordering::Acquire){break;}
            let (input,targets,mask)=sample(&brain,&train[idx])?;
            let learning_rate=rate(brain.step,peak);
            let loss=brain.train_sequence(&input,&targets,&mask,learning_rate,Focus::Joint)?;
            cumulative+=loss;count+=1;
            if count%8==0||count==train.len(){
                let mut s=locked(state)?;
                s.epoch=epoch+1;s.completed=epoch as usize*train.len()+count;
                s.loss=Some(cumulative/count as f32);
                s.stage=format!("{label}: epoch {}/{epochs}, sample {}/{}",
                    epoch+1,count,train.len());
                let _=app.emit("nova2-progress",status(state,&s));
            }
        }
        if count==0{break;}
        let held=heldout_loss(&brain,&valid)?;
        brain.save(file(app,CHECKPOINT)?)?;
        {
            let mut s=locked(state)?;
            s.brain=brain.clone();s.trained=true;
            s.loss=Some(cumulative/count as f32);s.heldout=Some(held);
            s.epoch=epoch+1;s.stage=format!("{label}: epoch {} saved",epoch+1);
            let _=app.emit("nova2-progress",status(state,&s));
        }
    }
    Ok(())
}
fn launch(app:AppHandle,state:Nova2State,lessons:Vec<(String,String,String)>,
    epochs:u32,learning_rate:f32)->Result<Status,String>{
    if epochs==0||epochs>25||!(0.000001..=0.01).contains(&learning_rate){
        return Err("Choose 1–25 epochs and a learning rate between 0.000001 and 0.01".into());
    }
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova-2 is already training".into());
    }
    state.inner.stop.store(false,Ordering::Release);
    {let mut s=locked(&state)?;s.error=None;s.stage="Preparing Nova-2 lessons".into();}
    let status_now={let s=locked(&state)?;status(&state,&s)};
    std::thread::spawn(move||{
        let mut result=Ok(());
        for (name,mode,text) in &lessons{
            if state.inner.stop.load(Ordering::Acquire){break;}
            result=train_one(&app,&state,mode,text,epochs,learning_rate,name);
            if result.is_err(){break;}
        }
        state.inner.training.store(false,Ordering::Release);
        {
            if let Ok(mut s)=locked(&state){
                match &result{
                    Ok(_)=>s.stage=if state.inner.stop.load(Ordering::Acquire){
                        "Nova-2 training stopped; last completed epoch saved".into()
                    }else{"Nova-2 training finished".into()},
                    Err(e)=>{s.error=Some(e.clone());s.stage="Nova-2 training error".into();}
                };
                let _=app.emit("nova2-progress",status(&state,&s));
            }
        }
        match result{
            Ok(())=>{let _=app.emit("nova2-done",());},
            Err(e)=>{let _=app.emit("nova2-error",e);},
        }
    });
    Ok(status_now)
}
#[tauri::command]
pub fn nova2_train(app:AppHandle,state:State<'_,Nova2State>,
    mode:String,text:String,epochs:u32,learning_rate:f32)->Result<Status,String>{
    // Validate before scheduling; never mark training active on invalid data.
    trainer::prepare(&mode,&text)?;
    launch(app,state.inner.clone().into(),vec![("Single lesson".into(),mode,text)],epochs,learning_rate)
}
#[tauri::command]
pub fn nova2_zip_train(app:AppHandle,state:State<'_,Nova2State>,
    bytes:Vec<u8>,mode:String,epochs:u32,learning_rate:f32)->Result<Status,String>{
    let bundle=crate::lesson_zip::read_bundle(&bytes,&mode)?;
    let lessons=bundle.lessons.into_iter().map(|l|(l.name,l.mode,l.text)).collect();
    launch(app,state.inner.clone().into(),lessons,epochs,learning_rate)
}
#[tauri::command]
pub fn nova2_stop(state:State<'_,Nova2State>)->bool{
    let active=state.inner.training.load(Ordering::Acquire);
    if active{state.inner.stop.store(true,Ordering::Release);}
    active
}
impl From<Arc<Inner>> for Nova2State{
    fn from(inner:Arc<Inner>)->Self{Self{inner}}
}
