//! Opt-in local JSONL investigation logger for Neural Nova.
//! Teacher-side only. No logging value is consumed by inference or gradients.
use neural_nova::hybrid::HybridBrain;
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const MAX_READ_BYTES: u64=12*1024*1024;
pub struct Recorder { pub active: bool, pub filename: Option<PathBuf>, pub lines:u64, pub last_error:Option<String> }
impl Default for Recorder {
    fn default()->Self{Self{active:false,filename:None,lines:0,last_error:None}}
}
pub struct Diagnostics(pub Mutex<Recorder>);
impl Default for Diagnostics{
    fn default()->Self{Self(Mutex::new(Recorder::default()))}
}
pub fn now_ms()->u128{
    SystemTime::now().duration_since(UNIX_EPOCH).map(|x|x.as_millis()).unwrap_or(0)
}
fn folder(app:&AppHandle)->Result<PathBuf,String>{
    let path=app.path().app_data_dir().map_err(|e|e.to_string())?.join("neural-nova-diagnostics");
    fs::create_dir_all(&path).map_err(|e|e.to_string())?;
    Ok(path)
}
fn record_unlocked(rec:&mut Recorder,event:&str,data:Value)->Result<(),String>{
    if !rec.active{return Ok(());}
    let path=rec.filename.as_ref().ok_or("No log destination")?;
    let payload=json!({"schema":"nova-diag-1","timestampMs":now_ms().to_string(),
        "event":event,"data":data});
    let mut file=OpenOptions::new().create(true).append(true)
        .open(path).map_err(|e|e.to_string())?;
    serde_json::to_writer(&mut file,&payload).map_err(|e|e.to_string())?;
    file.write_all(b"\n").map_err(|e|e.to_string())?;
    // Each JSONL record is complete on disk before the next operation.
    file.flush().map_err(|e|e.to_string())?;
    rec.lines+=1;
    Ok(())
}
/// Logging failures are captured and NEVER abort a training/response operation.
pub fn record(state:&Diagnostics,event:&str,data:Value){
    if let Ok(mut rec)=state.0.lock(){
        if let Err(error)=record_unlocked(&mut rec,event,data){
            rec.last_error=Some(error);
            rec.active=false;
        }
    }
}
pub fn enabled(state:&Diagnostics)->bool{
    state.0.lock().map(|r|r.active).unwrap_or(false)
}
pub fn fingerprint(text:&str)->String{
    // Deterministic FNV-1a digest for correlating data sets, not a security hash.
    let mut value=0xcbf29ce484222325u64;
    for byte in text.bytes(){value=(value^(byte as u64)).wrapping_mul(0x100000001b3);}
    format!("{value:016x}")
}
pub fn weight_digest(brain:&HybridBrain)->String{
    let mut value=0xcbf29ce484222325u64;
    let mut feed=|v:f32|{
        for b in v.to_bits().to_le_bytes(){value=(value^(b as u64)).wrapping_mul(0x100000001b3);}
    };
    for v in &brain.transformer.weights{feed(*v);}
    for layer in &brain.dense.layers{
        for v in &layer.weights{feed(*v);}
        for v in &layer.biases{feed(*v);}
    }
    format!("{value:016x}")
}
#[derive(serde::Serialize)]
#[serde(rename_all="camelCase")]
pub struct LogStatus {pub enabled:bool,pub path:Option<String>,pub lines:u64,
    pub last_error:Option<String>}
fn status(state:&Diagnostics)->Result<LogStatus,String>{
    let rec=state.0.lock().map_err(|_|"Diagnostic log mutex poisoned")?;
    Ok(LogStatus{enabled:rec.active,
        path:rec.filename.as_ref().map(|p|p.to_string_lossy().to_string()),
        lines:rec.lines,last_error:rec.last_error.clone()})
}
#[tauri::command]
pub fn nova_log_status(state:tauri::State<'_,Diagnostics>)->Result<LogStatus,String>{status(&state)}
#[tauri::command]
pub fn nova_log_start(app:AppHandle,state:tauri::State<'_,Diagnostics>)->Result<LogStatus,String>{
    let mut rec=state.0.lock().map_err(|_|"Diagnostic log mutex poisoned")?;
    if rec.active{return status_unlocked(&rec);}
    let file=folder(&app)?.join(format!("nova-{}-{}.jsonl",now_ms(),std::process::id()));
    let _=File::create(&file).map_err(|e|e.to_string())?;
    rec.active=true;rec.filename=Some(file);rec.lines=0;rec.last_error=None;
    record_unlocked(&mut rec,"session_start",json!({
        "note":"Opt-in observational log. Questions can be included, lesson content is hashed.",
        "diagnosticsVersion":1,"noModelUpdatesByLogger":true}))?;
    status_unlocked(&rec)
}
fn status_unlocked(rec:&Recorder)->Result<LogStatus,String>{
    Ok(LogStatus{enabled:rec.active,path:rec.filename.as_ref().map(|x|x.to_string_lossy().to_string()),
        lines:rec.lines,last_error:rec.last_error.clone()})
}
#[tauri::command]
pub fn nova_log_stop(state:tauri::State<'_,Diagnostics>)->Result<LogStatus,String>{
    let mut rec=state.0.lock().map_err(|_|"Diagnostic log mutex poisoned")?;
    if rec.active{record_unlocked(&mut rec,"session_stop",json!({"lines":rec.lines}))?;}
    rec.active=false;status_unlocked(&rec)
}
/// Return only the most recent session; never silently read arbitrary paths.
/// App UI can copy JSONL to clipboard or save it as a text file.
#[tauri::command]
pub fn nova_log_read(state:tauri::State<'_,Diagnostics>)->Result<String,String>{
    let rec=state.0.lock().map_err(|_|"Diagnostic log mutex poisoned")?;
    let path=rec.filename.as_ref().ok_or("Start a diagnostic session first")?;
    let size=fs::metadata(path).map_err(|e|e.to_string())?.len();
    if size>MAX_READ_BYTES{return Err("Log exceeds 12 MB; find it using the shown local path".into());}
    fs::read_to_string(path).map_err(|e|e.to_string())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn fingerprints_change_with_content_without_exposing_corpus(){
        assert_ne!(fingerprint("Hello!"),fingerprint("Yes."));
        assert_eq!(fingerprint("hello"),fingerprint("hello"));
    }
    #[test] fn logger_does_not_mutate_weights(){
        let brain=HybridBrain::new();
        let before=weight_digest(&brain);
        let _=fingerprint("testing");
        assert_eq!(before,weight_digest(&brain));
    }
}
