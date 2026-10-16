//! Nova Messenger: inference-only Rust host for the complete Nova-2 Bicameral brain.
//! The checkpoint, tokenizer, and neural mathematics remain in neural-nova.
//! Teaching, correction, grading, and lesson scheduling are not part of this app.
use neural_nova::nova2::{Nova2,Ablation,Focus,KvCache};
use std::path::PathBuf;
use std::sync::{Mutex,atomic::{AtomicBool,Ordering}};
use tauri::{AppHandle,Manager,State};
use serde::{Serialize,Deserialize};
use crate::checkpoint_transfer::TransferState;

const CHECKPOINT:&str="neural-nova-bicameral-v2.nnova2";
const BUNDLED_NOVA2:&[u8]=include_bytes!(concat!(env!("OUT_DIR"),"/nova2-bundled.nnova2"));
const CHAT:&str="neural-nova-bicameral-conversation.json";

#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Turn{user:String,assistant:String}
struct Session{
    brain:Nova2,trained:bool,stage:String,history:Vec<Turn>,
}
pub struct Nova2State{
    session:Mutex<Session>,
    importing:AtomicBool,
}
impl Default for Nova2State{
    fn default()->Self{
        Self{session:Mutex::new(Session{
            brain:Nova2::new(),trained:false,
            stage:"Starting Nova".into(),history:Vec::new(),
        }),importing:AtomicBool::new(false)}
    }
}
fn locked(s:&Nova2State)->Result<std::sync::MutexGuard<'_,Session>,String>{
    s.session.lock().map_err(|_|"Nova model lock poisoned".into())
}
fn file(app:&AppHandle,name:&str)->Result<PathBuf,String>{
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    Ok(dir.join(name))
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status{
    trained:bool,parameters:usize,transformer_parameters:usize,
    dense_parameters:usize,interconnect_parameters:usize,
    step:u64,words:usize,context:usize,stage:String,
}
fn status(m:&Session)->Status{
    Status{
        trained:m.trained,parameters:m.brain.parameter_count(),
        transformer_parameters:m.brain.group_parameters(Focus::Left),
        dense_parameters:m.brain.group_parameters(Focus::Right),
        interconnect_parameters:m.brain.group_parameters(Focus::Interconnect),
        step:m.brain.step,words:m.brain.tokenizer.words.len(),
        context:m.brain.config.max_tokens,stage:m.stage.clone(),
    }
}
#[tauri::command]
pub fn nova2_boot(app:AppHandle,state:State<'_,Nova2State>)->Result<Status,String>{
    if state.importing.load(Ordering::Acquire){
        return Err("Checkpoint import in progress".into());
    }
    let mut s=locked(&state)?;
    let p=file(&app,CHECKPOINT)?;
    if p.exists(){
        s.brain=Nova2::load(&p)?;
        s.trained=true;
        s.stage=format!("Loaded saved Nova checkpoint · step {}",s.brain.step);
    }else if !BUNDLED_NOVA2.is_empty(){
        let temp=p.with_extension("bundled.tmp");
        std::fs::write(&temp,BUNDLED_NOVA2).map_err(|e|e.to_string())?;
        let model=Nova2::load(&temp);
        let _=std::fs::remove_file(&temp);
        let model=model?;
        if model.step!=19_000{return Err("Bundled Nova checkpoint step mismatch".into());}
        model.save(&p)?;
        s.brain=model;s.trained=true;
        s.stage="Bundled Nova model loaded · 19,000 steps".into();
    }else{
        s.stage="No checkpoint available. Import a .nnova2 model.".into();
    }
    if let Ok(bytes)=std::fs::read(file(&app,CHAT)?){
        // Prior Nova conversation format remains fully compatible.
        if let Ok(turns)=serde_json::from_slice::<Vec<Turn>>(&bytes){
            if turns.len()<=12 && turns.iter().all(|t|
                t.user.is_ascii() && t.assistant.is_ascii() &&
                t.user.len()<=300 && t.assistant.len()<=512
            ){s.history=turns;}
        }
    }
    Ok(status(&s))
}
#[tauri::command]
pub fn nova2_status(state:State<'_,Nova2State>)->Result<Status,String>{
    Ok(status(&locked(&state)?))
}
#[tauri::command]
pub fn nova2_conversation(state:State<'_,Nova2State>)->Result<Vec<Turn>,String>{
    Ok(locked(&state)?.history.clone())
}
#[tauri::command]
pub fn nova2_new_chat(app:AppHandle,state:State<'_,Nova2State>)->Result<(),String>{
    if state.importing.load(Ordering::Acquire){return Err("Checkpoint import in progress".into());}
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
    if message.trim().is_empty() || !message.is_ascii() || message.len()>300{
        return Err("Nova's current tokenizer accepts 1–300 ASCII bytes per message".into());
    }
    if state.importing.load(Ordering::Acquire){return Err("Checkpoint import in progress".into());}
    let (brain,history)={
        let s=locked(&state)?;
        if !s.trained{return Err("Import a trained checkpoint before chatting".into());}
        (s.brain.clone(),s.history.clone())
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
    // Never append a reply from a model which was replaced during inference.
    if s.brain.step!=brain.step{return Err("Model changed during generation; please retry".into());}
    s.history.push(Turn{user:message,assistant:reply.clone()});
    if s.history.len()>12{s.history.remove(0);}
    let bytes=serde_json::to_vec(&s.history).map_err(|e|e.to_string())?;
    std::fs::write(file(&app,CHAT)?,bytes).map_err(|e|e.to_string())?;
    Ok(ChatReply{reply,generated_tokens:produced,
        cache_attention_keys:keys,step:brain.step})
}

// V2 checkpoints use a separate NNOVA2 format. Never route through HybridBrain::load.

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2CheckpointInfo{bytes:usize,step:u64,words:usize,parameters:usize}
#[tauri::command]
pub fn nova2_checkpoint_export_begin(
    app:AppHandle,state:State<'_,Nova2State>,transfer:State<'_,TransferState>
)->Result<Nova2CheckpointInfo,String>{
    if state.importing.load(Ordering::Acquire){return Err("Finish checkpoint import before exporting".into());}
    let s=locked(&state)?;
    if !s.trained{return Err("Load a checkpoint before exporting".into());}
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
    if state.importing.load(Ordering::Acquire){return Err("Checkpoint import already in progress".into());}
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
    if state.importing.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){
        return Err("Checkpoint import already in progress".into());
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
        s.brain=new_model;s.trained=true;s.history.clear();
        s.stage="Imported Nova-2 checkpoint".into();
        let result=status(&s);
        transfer.cancel()?;
        Ok(result)
    })();
    state.importing.store(false,Ordering::Release);
    result
}


fn generate(brain:&Nova2,prompt:&str,max_tokens:usize,context_tokens:usize,
    temperature:f32,sampling:&str)->Result<(String,usize,usize),String>{
    let ids=brain.tokenizer.encode(prompt)?;
    let window=context_tokens.clamp(1,brain.config.max_tokens);
    let recent=&ids[ids.len().saturating_sub(window)..];
    let mut context=recent.to_vec();
    let mut cache=KvCache::new(brain,Ablation::default());
    // KvCache::prefill returns raw logits; following append calls return
    // probabilities. Normalize the FIRST distribution as well.
    let normalize=|mut scores:Vec<f32>|{
        let valid=(128+brain.tokenizer.words.len()).min(brain.config.vocab);
        for value in scores.iter_mut().skip(valid){*value=f32::NEG_INFINITY;}
        let max=scores.iter().copied().fold(f32::NEG_INFINITY,f32::max);
        let mut sum=0f32;
        for value in &mut scores{*value=(*value-max).exp();sum+=*value;}
        for value in &mut scores{*value/=sum;}
        scores
    };
    let mut probs=normalize(cache.prefill(brain,recent)?);
    let mut keys_read=cache.stats().attention_keys_read;
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
        output.push(next);
        context.push(next);
        if context.len()>window{context.remove(0);}
        if cache.token_count()>=window{
            // The Python reference re-windows the last N tokens from RoPE
            // position zero. Sliding stale cached keys changes logits.
            // Re-prefill at the window boundary, even though it costs CPU.
            cache=KvCache::new(brain,Ablation::default());
            probs=normalize(cache.prefill(brain,&context)?);
            keys_read+=cache.stats().attention_keys_read;
        }else{
            let before=cache.stats().attention_keys_read;
            probs=brain.predict_cached(&mut cache,next)?;
            keys_read+=cache.stats().attention_keys_read-before;
        }
    }
    let text=brain.tokenizer.decode(&output)?;
    Ok((text,output.len(),keys_read))
}
#[derive(Serialize)]
pub struct ContextComparison{short:String,long:String}
#[tauri::command]
pub fn nova2_compare_context(state:State<'_,Nova2State>,message:String,
    max_tokens:Option<usize>)->Result<ContextComparison,String>{
    if message.trim().is_empty()||!message.is_ascii()||message.len()>300{
        return Err("Provide an ASCII question (up to 300 bytes)".into());
    }
    if state.importing.load(Ordering::Acquire){return Err("Checkpoint import in progress".into());}
    let brain=locked(&state)?.brain.clone();
    let prompt=format!("U: {}\nN: ",message);
    let (short,_,_)=generate(&brain,&prompt,max_tokens.unwrap_or(64),128,0.6,"greedy")?;
    let (long,_,_)=generate(&brain,&prompt,max_tokens.unwrap_or(64),256,0.6,"greedy")?;
    Ok(ContextComparison{short,long})
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2AttentionSensitivity{index:usize,character:String,weight:f32}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2Prediction{code:usize,probability:f32}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Nova2Trace{
    attention:Vec<Nova2AttentionSensitivity>,
    dense_layers:Vec<Vec<f32>>,top_predictions:Vec<Nova2Prediction>,
    bridge_signal:Vec<f32>,interconnect_used:bool,
    method:String
}
#[tauri::command]
pub fn nova2_inspect(state:State<'_,Nova2State>,message:String,
    context_chars:Option<usize>)->Result<Nova2Trace,String>{
    if message.trim().is_empty()||!message.is_ascii()||message.len()>300{
        return Err("Enter a nonempty ASCII question up to 300 bytes".into());
    }
    if state.importing.load(Ordering::Acquire){return Err("Checkpoint import in progress".into());}
    let model=locked(&state)?.brain.clone();
    let prompt=format!("U: {}\nN: ",message);
    let encoded=model.tokenizer.encode(&prompt)?;
    let context=context_chars.unwrap_or(128).clamp(1,model.config.max_tokens);
    let ids=&encoded[encoded.len().saturating_sub(context)..];
    let probs=model.predict(ids,Ablation::default())?;
    let no_right=model.predict(ids,Ablation{no_right:true,..Ablation::default()})?;
    let difference=0.5*probs.iter().zip(no_right.iter())
        .map(|(a,b)|(a-b).abs()).sum::<f32>();
    let length=ids.len();
    let stride=(length/16).max(1);
    let mut sensitivity=Vec::new();
    for i in (0..length).step_by(stride).take(24){
        if length<2{break;}
        let mut removed=ids.to_vec();removed.remove(i);
        let altered=model.predict(&removed,Ablation::default())?;
        let tv=0.5*probs.iter().zip(altered.iter())
            .map(|(a,b)|(a-b).abs()).sum::<f32>();
        let name=if ids[i]<128{
            let c=ids[i] as u8;
            if c==10{"↵".into()}else if c==32{"SPACE".into()}
            else if c<32{format!("0x{:02x}",c)}else{(c as char).to_string()}
        }else{
            model.tokenizer.words.get(ids[i] as usize-128)
                .cloned().unwrap_or_else(||"word".into())
        };
        sensitivity.push(Nova2AttentionSensitivity{
            index:i,character:name,weight:tv
        });
    }
    let mut top=(0..probs.len().min(128)).map(|i|Nova2Prediction{
        code:i,probability:probs[i]
    }).collect::<Vec<_>>();
    top.sort_by(|a,b|b.probability.total_cmp(&a.probability));top.truncate(8);
    let start=model.group_parameters(Focus::Left);
    let right=model.group_parameters(Focus::Right);
    let dense_end=start+right;
    let mut dense_layers=Vec::new();
    for layer in 0..3{
        let from=start+layer*right/3;
        let to=start+(layer+1)*right/3;
        let available=&model.weights[from..to];
        let skip=(available.len()/64).max(1);
        dense_layers.push(available.iter().step_by(skip).take(64).copied().collect());
    }
    let bridge=&model.weights[dense_end..];
    let mut bridge_signal=Vec::new();
    for i in 0..32{
        let a=i*bridge.len()/32;
        let b=(i+1)*bridge.len()/32;
        let mean=bridge[a..b].iter().map(|v|v.abs()).sum::<f32>()/(b-a).max(1) as f32;
        bridge_signal.push(mean);
    }
    Ok(Nova2Trace{attention:sensitivity,dense_layers,top_predictions:top,
        bridge_signal,interconnect_used:difference>1e-9,
        method:"Token-removal output sensitivity; sampled learned Dense and interconnect weights. Not direct attention or activation telemetry.".into()})
}

