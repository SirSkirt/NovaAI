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
    candidate:Option<Nova2>,candidate_loss:Option<f32>,candidate_base:u64,
    candidate_focus:String,candidate_epoch:u32,imported_target:Option<Nova2>,
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
                history:Vec::new(),lesson:String::new(),
                candidate:None,candidate_loss:None,candidate_base:0,
                candidate_focus:String::new(),candidate_epoch:0,imported_target:None
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
    candidate_ready:bool,candidate_loss:Option<f32>,candidate_focus:String,candidate_epoch:u32,
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
        candidate_ready:m.candidate.is_some(),candidate_loss:m.candidate_loss,
        candidate_focus:m.candidate_focus.clone(),candidate_epoch:m.candidate_epoch,
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
    max_tokens:Option<usize>,temperature:Option<f32>,sampling:Option<String>,
    context_tokens:Option<usize>)->Result<ChatReply,String>{
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
    let (reply,produced,keys)=generate(&brain,&prompt,max_tokens.unwrap_or(64),
        context_tokens.unwrap_or(256),temperature.unwrap_or(0.6),
        sampling.as_deref().unwrap_or("greedy"))?;
    let mut s=locked(&state)?;
    s.history.push(Turn{user:message,assistant:reply.clone()});
    if s.history.len()>12{s.history.remove(0);}
    let history=serde_json::to_vec(&s.history).map_err(|e|e.to_string())?;
    std::fs::write(file(&app,CHAT)?,history).map_err(|e|e.to_string())?;
    Ok(ChatReply{reply,generated_tokens:produced,
        cache_attention_keys:keys,step:brain.step})
}
#[tauri::command]
pub fn nova2_reset(app:AppHandle,state:State<'_,Nova2State>)->Result<Status,String>{
    if state.inner.training.load(Ordering::Acquire){return Err("Stop training first".into());}
    let mut s=locked(&state)?;
    let p=file(&app,CHECKPOINT)?;
    if p.exists(){std::fs::remove_file(&p).map_err(|e|e.to_string())?;}
    std::fs::write(file(&app,CHAT)?,b"[]").map_err(|e|e.to_string())?;
    s.brain=Nova2::new();s.trained=false;s.history.clear();s.candidate=None;s.candidate_loss=None;s.imported_target=None;
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
    epochs:u32,learning_rate:f32,options:EducationOptions)->Result<Status,String>{
    if epochs==0||epochs>60||!(0.000001..=0.01).contains(&learning_rate){
        return Err("Choose 1–60 epochs and a learning rate between 0.000001 and 0.01".into());
    }
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova-2 is already training".into());
    }
    state.inner.stop.store(false,Ordering::Release);
    {let mut s=locked(&state)?;s.error=None;s.stage="Preparing Nova-2 lessons".into();}
    let status_now={let s=locked(&state)?;status(&state,&s)};
    std::thread::spawn(move||{
        let mut result=Ok(());
        for (index,(name,mode,text)) in lessons.iter().enumerate(){
            if state.inner.stop.load(Ordering::Acquire){break;}
            let scoped=options.foundation_training.unwrap_or(false) &&
                (lessons.len()==1||options.foundation_zip_scope.as_deref()==Some("entireZip")||index<5);
            result=selective_lesson(&app,&state,mode,text,epochs,learning_rate,name,scoped);
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
    mode:String,text:String,epochs:u32,learning_rate:f32,
    options:Option<EducationOptions>)->Result<Status,String>{
    // Validate before scheduling; never mark training active on invalid data.
    trainer::prepare(&mode,&text)?;
    launch(app,state.inner.clone().into(),vec![("Single lesson".into(),mode,text)],epochs,learning_rate,options.unwrap_or_default())
}
#[tauri::command]
pub fn nova2_zip_train(app:AppHandle,state:State<'_,Nova2State>,
    bytes:Vec<u8>,mode:String,epochs:u32,learning_rate:f32,
    options:Option<EducationOptions>)->Result<Status,String>{
    let bundle=crate::lesson_zip::read_bundle(&bytes,&mode)?;
    let lessons=bundle.lessons.into_iter().map(|l|(l.name,l.mode,l.text)).collect();
    launch(app,state.inner.clone().into(),lessons,epochs,learning_rate,options.unwrap_or_default())
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



#[derive(Clone,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct EducationOptions {
    pub foundation_training:Option<bool>,
    pub foundation_zip_scope:Option<String>,
    pub adaptive_learning_rate:Option<bool>,
    pub early_stopping:Option<bool>,
    pub protect_previous:Option<bool>,
    pub rehearsal:Option<bool>,
    pub retention_loss_tolerance:Option<f32>,
}
fn focus_name(f:Focus)->&'static str{
    match f{Focus::Joint=>"joint",Focus::Left=>"transformer",
        Focus::Right=>"dense",Focus::Interconnect=>"interconnect"}
}
fn parse_focus(s:&str)->Result<Focus,String>{
    match s {"joint"=>Ok(Focus::Joint),"transformer"|"left"=>Ok(Focus::Left),
        "dense"|"right"=>Ok(Focus::Right),"interconnect"=>Ok(Focus::Interconnect),
        _=>Err("Choose Dense, Transformer, Interconnect, or Joint".into())}
}
fn validation_with_baseline(brain:&Nova2,valid:&[Unit])->Result<f32,String>{
    heldout_loss(brain,valid)
}
fn selective_lesson(app:&AppHandle,state:&Nova2State,mode:&str,text:&str,
    epochs:u32,peak:f32,label:&str,foundation:bool)->Result<(),String>{
    let prepared=trainer::prepare(mode,text)?;
    let train=units(&prepared.train);let valid=units(&prepared.validation);
    if train.is_empty()||valid.is_empty(){return Err("Lesson requires independent held-out examples".into());}
    let mut start=locked(state)?.brain.clone();
    start.tokenizer.grow_from_corpus(&prepared.training_text);
    let initial_loss=validation_with_baseline(&start,&valid)?;
    let mut best=start.clone();let mut best_loss=initial_loss;
    let foci=if foundation{vec![Focus::Right,Focus::Left,Focus::Interconnect,Focus::Joint]}
        else{vec![Focus::Joint]};
    let total=train.len()*epochs as usize*foci.len();
    {let mut s=locked(state)?;s.lesson=label.into();s.epochs=epochs;
        s.total=total;s.completed=0;s.stage=format!("{label}: initializing candidate training");
        s.heldout=Some(initial_loss);}
    let mut completed=0usize;
    for focus in foci{
        if state.inner.stop.load(Ordering::Acquire){break;}
        let mut brain=start.clone();
        let mut candidate_loss=initial_loss;
        let mut candidate=start.clone();
        for epoch in 0..epochs{
            if state.inner.stop.load(Ordering::Acquire){break;}
            let mut cumulative=0.0f32;let mut count=0usize;
            let mut order=(0..train.len()).collect::<Vec<_>>();
            if epoch>0 && !order.is_empty(){let n=order.len();
                order.rotate_left((epoch as usize*37)%n);}
            for idx in order{
                if state.inner.stop.load(Ordering::Acquire){break;}
                let (ids,targets,mask)=sample(&brain,&train[idx])?;
                let loss=brain.train_sequence(&ids,&targets,&mask,rate(brain.step,peak),focus)?;
                cumulative+=loss;count+=1;completed+=1;
                if count%8==0||count==train.len(){
                    let mut s=locked(state)?;
                    s.epoch=epoch+1;s.completed=completed;
                    s.loss=Some(cumulative/count as f32);
                    s.stage=format!("{label}: {} candidate, epoch {}/{epochs}, sample {}/{}",
                        focus_name(focus),epoch+1,count,train.len());
                    let _=app.emit("nova2-progress",status(state,&s));
                }
            }
            if count==0{break;}
            let held=validation_with_baseline(&brain,&valid)?;
            if held.is_finite()&&held<candidate_loss{
                candidate_loss=held;candidate=brain.clone();
            }
        }
        if candidate_loss<best_loss{
            best_loss=candidate_loss;best=candidate;
        }
    }
    if best_loss<initial_loss{
        best.save(file(app,CHECKPOINT)?)?;
        let mut s=locked(state)?;s.brain=best;s.trained=true;
        s.heldout=Some(best_loss);s.stage=format!(
            "{label}: selected checkpoint (held-out loss {:.4} -> {:.4})",initial_loss,best_loss);
        let _=app.emit("nova2-progress",status(state,&s));
    }else{
        let mut s=locked(state)?;s.heldout=Some(initial_loss);
        s.stage=format!("{label}: no candidate improved held-out loss; original checkpoint retained");
        let _=app.emit("nova2-progress",status(state,&s));
    }
    Ok(())
}

// V2 checkpoints use a separate NNOVA2 format. Never route through HybridBrain::load.
use crate::checkpoint_transfer::TransferState;
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2CheckpointInfo{bytes:usize,step:u64,words:usize,parameters:usize}
#[tauri::command]
pub fn nova2_checkpoint_export_begin(
    app:AppHandle,state:State<'_,Nova2State>,transfer:State<'_,TransferState>
)->Result<Nova2CheckpointInfo,String>{
    if state.is_training(){return Err("Finish Nova-2 training before exporting".into());}
    let s=locked(&state)?;
    if !s.trained{return Err("Train Nova-2 before exporting a checkpoint".into());}
    let scratch=file(&app,"neural-nova-bicameral-export.nnova2")?;
    let result=(||->Result<Vec<u8>,String>{
        s.brain.save(&scratch)?;
        std::fs::read(&scratch).map_err(|e|e.to_string())
    })();
    let _=std::fs::remove_file(&scratch);
    let bytes=transfer.begin_export(result?)?;
    Ok(Nova2CheckpointInfo{bytes,step:s.brain.step,words:s.brain.tokenizer.words.len(),
        parameters:s.brain.parameter_count()})
}
#[tauri::command]
pub fn nova2_checkpoint_export_chunk(
    transfer:State<'_,TransferState>,offset:usize
)->Result<Vec<u8>,String>{transfer.chunk(offset)}
#[tauri::command]
pub fn nova2_checkpoint_export_cancel(transfer:State<'_,TransferState>)->Result<(),String>{
    transfer.export_cancel()
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2AndroidExport{bytes:usize,verified:bool}
#[tauri::command]
pub fn nova2_checkpoint_export_android(
    app:AppHandle,transfer:State<'_,TransferState>,destination_uri:String
)->Result<Nova2AndroidExport,String>{
    #[cfg(target_os="android")]
    {
        if !destination_uri.starts_with("content://"){return Err("Choose a Files document".into());}
        let bytes=transfer.export_read()?;
        let scratch=file(&app,"neural-nova-bicameral-transfer.nnova2")?;
        std::fs::write(&scratch,&bytes).map_err(|e|e.to_string())?;
        let attempted=crate::android_checkpoint_export::write_android_document(
            &scratch,&destination_uri);
        let _=std::fs::remove_file(&scratch);
        let actual=attempted?;
        if actual as usize!=bytes.len(){
            return Err("Incomplete Android Nova-2 checkpoint export".into());
        }
        transfer.export_cancel()?;
        Ok(Nova2AndroidExport{bytes:bytes.len(),verified:true})
    }
    #[cfg(not(target_os="android"))]
    {
        let _=(app,transfer,destination_uri);
        Err("Android document export is only available on Android".into())
    }
}
#[tauri::command]
pub fn nova2_checkpoint_import_begin(
    state:State<'_,Nova2State>,transfer:State<'_,TransferState>,bytes:usize
)->Result<(),String>{
    if state.is_training(){return Err("Finish training before importing".into());}
    transfer.import_begin(bytes)
}
#[tauri::command]
pub fn nova2_checkpoint_import_append(
    transfer:State<'_,TransferState>,chunk:Vec<u8>
)->Result<usize,String>{transfer.import_append(chunk)}
#[tauri::command]
pub fn nova2_checkpoint_import_cancel(transfer:State<'_,TransferState>)->Result<(),String>{
    transfer.cancel()
}
#[tauri::command]
pub fn nova2_checkpoint_import_finish(
    app:AppHandle,state:State<'_,Nova2State>,transfer:State<'_,TransferState>
)->Result<Status,String>{
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Finish training before importing".into());
    }
    let result=(||->Result<Status,String>{
        let bytes=transfer.import_read()?;
        let scratch=file(&app,"neural-nova-bicameral-validate.nnova2")?;
        std::fs::write(&scratch,&bytes).map_err(|e|e.to_string())?;
        let candidate=Nova2::load(&scratch);
        let _=std::fs::remove_file(&scratch);
        let new_model=candidate?;
        let mut s=locked(&state)?;
        let main=file(&app,CHECKPOINT)?;
        if s.trained && main.exists(){
            let stamp=std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e|e.to_string())?.as_secs();
            let backup=file(&app,&format!("nova2-before-import-{stamp}.nnova2"))?;
            std::fs::copy(&main,backup).map_err(|e|e.to_string())?;
        }
        new_model.save(&main)?;
        std::fs::write(file(&app,CHAT)?,b"[]").map_err(|e|e.to_string())?;
        s.brain=new_model;s.trained=true;s.history.clear();s.candidate=None;s.candidate_loss=None;s.imported_target=None;
        s.stage="Imported Nova-2 checkpoint".into();s.error=None;
        s.loss=None;s.heldout=None;s.epoch=0;s.epochs=0;s.completed=0;s.total=0;
        let result=status(&state,&s);
        transfer.cancel()?;
        Ok(result)
    })();
    state.inner.training.store(false,Ordering::Release);
    result.map(|mut s|{s.training=false;s})
}

#[tauri::command]
pub fn nova2_targeted_train(app:AppHandle,state:State<'_,Nova2State>,
    focus_name:String,skill_name:String,lesson_text:String,epochs:u32,
    learning_rate:f32,source:Option<String>)->Result<Status,String>{
    let focus=parse_focus(&focus_name)?;
    if !(1..=30).contains(&epochs) || !(0.000001..=0.01).contains(&learning_rate){
        return Err("Choose 1-30 epochs and learning rate 0.000001-0.01".into());
    }
    let mode=match skill_name.as_str(){
        "language"=>"text","dialogue"=>"dialogue","conversation"=>"conversation",
        "understanding"|"reasoning"|"unified"=>"unified",
        _=>"unified",
    };
    let prepared=trainer::prepare(mode,&lesson_text)?;
    let train=units(&prepared.train);let valid=units(&prepared.validation);
    if train.is_empty()||valid.is_empty(){return Err("Targeted training needs held-out examples".into());}
    if state.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Nova is already training".into());
    }
    state.inner.stop.store(false,Ordering::Release);
    let (starting,base_step)={
        let mut s=locked(&state)?;
        if s.candidate.is_some(){
            state.inner.training.store(false,Ordering::Release);
            return Err("Accept or discard the existing candidate first".into());
        }
        s.stage=format!("Training {} candidate",focus_name);
        s.epoch=0;s.epochs=epochs;s.completed=0;s.total=train.len()*epochs as usize;
        (if source.as_deref()==Some("imported"){
            s.imported_target.clone().ok_or("Stage an imported .nnova2 first")?
        }else{s.brain.clone()},s.brain.step)
    };
    let now={let s=locked(&state)?;status(&state,&s)};
    let cloned=state.inner.clone();
    std::thread::spawn(move||{
        let state=Nova2State{inner:cloned};
        let result=(||->Result<(),String>{
            let mut brain=starting;
            brain.tokenizer.grow_from_corpus(&prepared.training_text);
            let mut best=brain.clone();
            let mut best_loss=heldout_loss(&brain,&valid)?;
            let mut best_epoch=0;
            for epoch in 0..epochs{
                if state.inner.stop.load(Ordering::Acquire){break;}
                let mut order=(0..train.len()).collect::<Vec<_>>();
                if epoch>0&&!order.is_empty(){let n=order.len();order.rotate_left((37*epoch as usize)%n);}
                let mut count=0;let mut loss_sum=0.;
                for idx in order{
                    if state.inner.stop.load(Ordering::Acquire){break;}
                    let (ids,targets,mask)=sample(&brain,&train[idx])?;
                    loss_sum+=brain.train_sequence(&ids,&targets,&mask,
                        rate(brain.step,learning_rate),focus)?;
                    count+=1;
                    if count%8==0||count==train.len(){
                        let mut s=locked(&state)?;s.epoch=epoch+1;
                        s.completed=epoch as usize*train.len()+count;
                        s.loss=Some(loss_sum/count as f32);
                        let _=app.emit("nova2-progress",status(&state,&s));
                    }
                }
                if count==0{break;}
                let held=heldout_loss(&brain,&valid)?;
                if held.is_finite()&&held<best_loss{
                    best_loss=held;best=brain.clone();best_epoch=epoch+1;
                }
            }
            let mut s=locked(&state)?;
            s.candidate=if best_epoch>0{Some(best)}else{None};
            s.candidate_loss=Some(best_loss);s.candidate_base=base_step;
            s.candidate_focus=focus_name;s.candidate_epoch=best_epoch;
            s.stage=if best_epoch>0{format!("Candidate ready, epoch {}",best_epoch)}
                else{"No held-out improvement; live brain unchanged".into()};
            Ok(())
        })();
        state.inner.training.store(false,Ordering::Release);
        if let Err(e)=result{
            if let Ok(mut s)=locked(&state){s.error=Some(e);s.stage="Targeted training error".into();}
        }
        if let Ok(s)=locked(&state){let _=app.emit("nova2-progress",status(&state,&s));}
        let _=app.emit("nova2-done",());
    });
    Ok(now)
}
#[tauri::command]
pub fn nova2_targeted_discard(state:State<'_,Nova2State>)->Result<Status,String>{
    if state.is_training(){return Err("Stop training before discarding candidate".into());}
    let mut s=locked(&state)?;s.candidate=None;s.candidate_loss=None;
    s.candidate_focus.clear();s.candidate_epoch=0;
    s.stage="Targeted candidate discarded; live brain unchanged".into();
    Ok(status(&state,&s))
}
#[tauri::command]
pub fn nova2_targeted_accept(app:AppHandle,state:State<'_,Nova2State>)
    ->Result<Status,String>{
    if state.is_training(){return Err("Finish training before accepting".into());}
    let mut s=locked(&state)?;
    if s.candidate_base!=s.brain.step{return Err("Live brain changed since candidate training".into());}
    let candidate=s.candidate.as_ref().ok_or("No improved candidate is ready")?.clone();
    let main=file(&app,CHECKPOINT)?;
    if main.exists(){
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_err(|e|e.to_string())?.as_secs();
        std::fs::copy(&main,file(&app,&format!("nova2-before-targeted-{stamp}.nnova2"))?)
            .map_err(|e|e.to_string())?;
    }
    candidate.save(&main)?;
    s.brain=candidate;s.candidate=None;s.candidate_loss=None;s.imported_target=None;s.trained=true;
    s.history.clear();std::fs::write(file(&app,CHAT)?,b"[]").map_err(|e|e.to_string())?;
    s.stage="Accepted complete targeted neural checkpoint".into();
    Ok(status(&state,&s))
}

impl Nova2State{
    pub(crate) fn snapshot(&self)->Result<Nova2,String>{
        Ok(locked(self)?.brain.clone())
    }
    pub(crate) fn reserve_external(&self)->Result<(),String>{
        self.inner.training.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire)
            .map_err(|_|"Nova is already training or testing".to_string())?;
        Ok(())
    }
    pub(crate) fn release_external(&self){
        self.inner.training.store(false,Ordering::Release);
    }
    pub(crate) fn install_external(&self,app:&AppHandle,brain:Nova2)->Result<(),String>{
        let mut s=locked(self)?;
        let path=file(app,CHECKPOINT)?;
        brain.save(&path)?;
        s.brain=brain;s.trained=true;s.candidate=None;s.candidate_loss=None;
        s.stage="Teacher supervised corrections saved to .nnova2".into();
        Ok(())
    }
}

#[tauri::command]
pub fn nova2_targeted_stage(app:AppHandle,state:State<'_,Nova2State>,bytes:Vec<u8>)
    ->Result<Status,String>{
    if state.is_training(){return Err("Finish training before staging a candidate".into());}
    if bytes.len()<8 || bytes.len()>24*1024*1024{
        return Err("Imported .nnova2 must be between 8 bytes and 24 MiB".into());
    }
    let scratch=file(&app,"neural-nova-bicameral-targeted-stage.nnova2")?;
    std::fs::write(&scratch,&bytes).map_err(|e|e.to_string())?;
    let loaded=Nova2::load(&scratch);
    let _=std::fs::remove_file(&scratch);
    let mut s=locked(&state)?;
    s.imported_target=Some(loaded?);
    s.stage="Imported .nnova2 staged as isolated targeted-training source".into();
    Ok(status(&state,&s))
}
impl Nova2State{
    pub(crate) fn inner_clone_for_test(&self)->Self{self.clone()}
}

fn generate(brain:&Nova2,prompt:&str,max_tokens:usize,context_tokens:usize,
    temperature:f32,sampling:&str)->Result<(String,usize,usize),String>{
    let ids=brain.tokenizer.encode(prompt)?;
    let window=context_tokens.clamp(1,brain.config.max_tokens);
    let recent=&ids[ids.len().saturating_sub(window)..];
    let mut cache=KvCache::new(brain,Ablation::default());
    let mut probs=cache.prefill(brain,recent)?;
    let mut output=Vec::new();
    let mut seed=if sampling=="fixed"||sampling=="seeded"{42u64}
        else{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_nanos() as u64};
    let random=sampling=="random"||sampling=="fixed"||sampling=="seeded";
    for _ in 0..max_tokens.clamp(1,256){
        let next=if random{
            let temp=temperature.clamp(0.2,2.);
            let weights=probs.iter().map(|&p|p.max(0.).powf(1./temp))
                .collect::<Vec<f32>>();
            let sum=weights.iter().sum::<f32>();
            seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;
            let mut choice=(seed as f64/u64::MAX as f64) as f32*sum;
            let mut chosen=0usize;
            for (idx,value) in weights.iter().enumerate(){
                chosen=idx;choice-=*value;if choice<=0.{break;}
            }
            chosen as u16
        }else{
            probs.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1))
                .ok_or("Empty Nova-2 output")?.0 as u16
        };
        if next==b'\n' as u16{break;}
        output.push(next);probs=brain.predict_cached(&mut cache,next)?;
    }
    let text=brain.tokenizer.decode(&output)?;
    Ok((text,output.len(),cache.stats().attention_keys_read))
}
#[derive(Serialize)]
pub struct ContextComparison{short:String,long:String}
#[tauri::command]
pub fn nova2_compare_context(state:State<'_,Nova2State>,message:String,
    max_tokens:Option<usize>)->Result<ContextComparison,String>{
    if message.trim().is_empty()||!message.is_ascii()||message.len()>300{
        return Err("Provide an ASCII question (up to 300 bytes)".into());
    }
    if state.is_training(){return Err("Finish training first".into());}
    let brain=locked(&state)?.brain.clone();
    let prompt=format!("U: {}\nN: ",message);
    let (short,_,_)=generate(&brain,&prompt,max_tokens.unwrap_or(64),128,0.6,"greedy")?;
    let (long,_,_)=generate(&brain,&prompt,max_tokens.unwrap_or(64),256,0.6,"greedy")?;
    Ok(ContextComparison{short,long})
}
