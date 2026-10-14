//! Untrusted ZIP admission for sequential teaching. This is strictly a teacher
//! input parser: the archive cannot execute code, edit weights or choose replies.
use neural_nova::trainer::prepare;
use serde::Serialize;
use std::collections::HashSet;
use std::io::{Cursor,Read};
use tauri::command;

const MAX_ZIP_BYTES:usize=4*1024*1024;
const MAX_LESSON_BYTES:usize=50_000;
const MAX_LESSONS:usize=40;
const MAX_TOTAL_BYTES:usize=1_600_000;
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ZipLesson{pub name:String,pub mode:String,pub text:String,pub training_samples:usize,pub heldout_samples:usize}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct LessonBundle{pub mode:String,pub lessons:Vec<ZipLesson>,pub total_bytes:usize}
pub fn read_bundle(data:&[u8],mode:&str)->Result<LessonBundle,String>{
    if !matches!(mode,"auto"|"unified"|"text"|"dialogue"|"conversation"|"reasoning"){
        return Err("ZIP format must be auto/unified or an explicit legacy format".into());
    }
    if data.is_empty() || data.len()>MAX_ZIP_BYTES{
        return Err("ZIP must contain data and be smaller than 4 MiB".into());
    }
    let mut archive=zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e|format!("Invalid ZIP: {e}"))?;
    if archive.len()>256{return Err("Archive has too many entries (max 256)".into());}
    let mut lessons=Vec::new();
    let mut names=HashSet::new();
    let mut total=0usize;
    for index in 0..archive.len(){
        let mut file=archive.by_index(index).map_err(|e|e.to_string())?;
        if file.is_dir(){continue;}
        let raw_name=file.name().to_string();
        // Reject traversal, absolute paths, alternate separators or duplicates.
        if raw_name.starts_with('/') || raw_name.contains('\\') ||
            raw_name.split('/').any(|segment|segment.is_empty()||segment=="."||segment=="..")
            || raw_name.chars().any(|c|c.is_control()){
            return Err(format!("Unsafe ZIP filename: {raw_name}"));
        }
        if raw_name.starts_with("__MACOSX/") || raw_name.split('/').last().unwrap_or("").starts_with('.') {
            continue;
        }
        if !raw_name.to_ascii_lowercase().ends_with(".txt"){
            return Err(format!("Only .txt lessons are allowed: {raw_name}"));
        }
        if !names.insert(raw_name.to_ascii_lowercase()){
            return Err(format!("Duplicate lesson name in ZIP: {raw_name}"));
        }
        if lessons.len()>=MAX_LESSONS{return Err("Maximum 40 TXT lessons per batch".into());}
        if file.size()>MAX_LESSON_BYTES as u64{
            return Err(format!("{raw_name}: lesson exceeds 50,000 bytes"));
        }
        let mut bytes=Vec::new();
        file.by_ref().take((MAX_LESSON_BYTES+1) as u64)
            .read_to_end(&mut bytes).map_err(|e|format!("{raw_name}: {e}"))?;
        if bytes.len()>MAX_LESSON_BYTES{return Err(format!("{raw_name}: lesson exceeds size limit"));}
        total+=bytes.len();
        if total>MAX_TOTAL_BYTES{return Err("ZIP's total uncompressed text exceeds 1.6 MiB".into());}
        let text=String::from_utf8(bytes).map_err(|_|format!("{raw_name}: invalid UTF-8"))?;
        if !text.is_ascii(){return Err(format!("{raw_name}: Nova currently requires ASCII lessons"));}
        let text=text.replace("\r\n","\n");
        if text.contains('\r'){return Err(format!("{raw_name}: unsupported line endings"));}
        // The presence of a valid block opener requests strict structured parsing,
        // even if a legacy mode happens to remain selected in the editor.
        // Do not silently reinterpret legacy data as structured examples.
        let structured=text.lines().any(|line|matches!(line.trim(),
            "[Dialogue Lesson]"|"[Understanding Lesson]"|
            "[Multi-Turn Conversation Lesson]"|"[Reasoning Lesson]"));
        let resolved=if structured{"unified"}else if mode=="auto"||mode=="unified"{
            return Err(format!("{raw_name}: not a structured lesson; select its explicit legacy mode to import this older dataset"));
        }else{mode};
        let parsed=prepare(resolved,&text).map_err(|error|format!("{raw_name}: {error}"))?;
        lessons.push(ZipLesson{name:raw_name,mode:resolved.to_string(),text,
            training_samples:parsed.train.len(),heldout_samples:parsed.validation.len()});
    }
    if lessons.is_empty(){return Err("No TXT lessons found inside ZIP".into());}
    lessons.sort_by(|a,b|{
        let ordinal=|name:&str|name.split('/').last().unwrap_or(name)
            .chars().take_while(|c|c.is_ascii_digit())
            .collect::<String>().parse::<u64>().ok();
        match (ordinal(&a.name),ordinal(&b.name)){
            (Some(x),Some(y))=>x.cmp(&y).then(a.name.cmp(&b.name)),
            _=>a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase())
        }
    });
    let bundle_mode=if lessons.iter().all(|x|x.mode=="unified"){
        "unified".to_string()
    }else if lessons.iter().all(|x|x.mode==lessons[0].mode){
        lessons[0].mode.clone()
    }else{"mixed".to_string()};
    Ok(LessonBundle{mode:bundle_mode,lessons,total_bytes:total})
}
#[command]
pub fn hybrid_zip_preview(bytes:Vec<u8>,mode:String)->Result<LessonBundle,String>{
    read_bundle(&bytes,&mode)
}

// Native ZIP sequencing: the WebView is NOT responsible for starting lessons
// after each completion. Android may suspend JS with the display off.
use crate::hybrid_lab::{self,HybridState,TrainOptions};
use crate::transformer_lab::Lesson;
use std::sync::{Arc,Mutex};
use std::sync::atomic::{AtomicBool,Ordering};
use std::time::Duration;
use tauri::{AppHandle,Emitter,Manager,State};

#[derive(Clone,Default,Serialize)]
#[serde(rename_all="camelCase")]
pub struct ZipTrainingStatus {
    active:bool,
    stopping:bool,
    total:usize,
    current:usize,
    completed:usize,
    skipped:usize,
    failed:usize,
    attempt:u8,
    retry_strategy:String,
    learning_evidence:Option<hybrid_lab::LessonLearningEvidence>,
    lesson:String,
    stage:String,
    error:Option<String>,
}
/// Teacher scheduler outcome, not a neural answer or cognition rule.
#[derive(Debug,PartialEq,Eq)]
enum LessonDisposition {
    Accepted(u32),
    BaselineRetained,
}
/// A protected baseline is a VALID training outcome: it means no candidate
/// beat the held-out/retention criteria, not that the training worker failed.
/// Never count it as learned or add it to the rehearsal archive.
fn classify_lesson_outcome(best_epoch:u32,error:Option<String>)
    ->Result<LessonDisposition,String>{
    if let Some(problem)=error{return Err(problem);}
    if best_epoch==0{Ok(LessonDisposition::BaselineRetained)}
    else{Ok(LessonDisposition::Accepted(best_epoch))}
}
/// Recovery plans are teacher-only scheduling, never changes to inference.
fn recovery_plan(attempt:usize,base_rate:f32,epochs:u32)->(f32,u32,bool,&'static str){
    let lower=(base_rate*0.1).min(0.000015).max(0.00001).min(base_rate);
    match attempt {
        0=>(base_rate,epochs,false,"standard"),
        1=>(lower,epochs,false,"lower learning rate"),
        2=>(base_rate,epochs.saturating_add(6).min(60),true,
            "alternative example schedule + six extra epochs"),
        _=>(lower,epochs.saturating_add(6).min(60),true,
            "combined alternative schedule + lower learning rate"),
    }
}
/// Record a lesson's actual education outcome without inventing learning.
/// Zero eligible epochs means the baseline was retained, so it is NOT counted
/// as accepted and MUST NOT be written to the rehearsal archive.
fn apply_lesson_disposition(status:&mut ZipTrainingStatus,
    name:&str,disposition:LessonDisposition) {
    match disposition {
        LessonDisposition::Accepted(epoch)=>{
            status.completed+=1;
            status.stage=format!(
                "Accepted {} (epoch {}); preparing next lesson",name,epoch);
        },
        LessonDisposition::BaselineRetained=>{
            status.skipped+=1;
            status.stage=format!(
                "No eligible improvement in {}; best baseline retained. Continuing to next lesson",
                name);
        }
    }
}
#[derive(Default)]
struct ZipQueueShared {
    active:AtomicBool,
    stopping:AtomicBool,
    progress:Mutex<ZipTrainingStatus>,
}
#[derive(Clone,Default)]
pub struct ZipQueueState { inner:Arc<ZipQueueShared> }
impl ZipQueueState {
    pub(crate) fn is_active(&self)->bool{
        self.inner.active.load(Ordering::Acquire)
    }
    /// Read-only progress for Android's service notification retry. The
    /// lesson comes from the native ZIP queue, never JavaScript polling.
    #[cfg(target_os="android")]
    pub(crate) fn current_lesson(&self)->Option<(String,usize,usize)>{
        let status=self.inner.progress.lock().ok()?;
        if !status.active || status.current==0 || status.lesson.is_empty(){return None;}
        Some((status.lesson.clone(),status.current,status.total))
    }
    fn snapshot(&self)->Result<ZipTrainingStatus,String>{
        self.inner.progress.lock().map(|x|x.clone())
            .map_err(|_|"ZIP scheduler state lock poisoned".into())
    }
    fn update(&self,app:&AppHandle,update:impl FnOnce(&mut ZipTrainingStatus)){
        if let Ok(mut status)=self.inner.progress.lock(){
            let old_lesson=status.lesson.clone();
            let old_position=status.current;
            update(&mut status);
            status.active=self.is_active();
            status.stopping=self.inner.stopping.load(Ordering::Acquire);
            // A lesson transition, not JS or a timer, drives the SECOND
            // notification. The first (epoch/loss) notification is untouched.
            let lesson_changed=status.active && status.current>0 &&
                !status.lesson.is_empty() &&
                (status.current!=old_position || status.lesson!=old_lesson);
            #[cfg(target_os="android")]
            let label=if lesson_changed {
                Some((status.lesson.clone(),status.current,status.total))
            }else{None};
            #[cfg(not(target_os="android"))]
            let _=lesson_changed;
            let snapshot=status.clone();
            drop(status);
            let _=app.emit("hybrid-zip-progress",snapshot);
            #[cfg(target_os="android")]
            if let Some((name,index,total))=label {
                crate::android_lesson_notification::show_zip_lesson(
                    &name,index,total);
            }
        }
    }
}
#[tauri::command]
pub fn hybrid_zip_status(queue:State<'_,ZipQueueState>)->Result<ZipTrainingStatus,String>{
    queue.snapshot()
}
#[tauri::command]
pub fn hybrid_zip_stop(app:AppHandle,queue:State<'_,ZipQueueState>,
    brain:State<'_,HybridState>)->bool{
    if !queue.is_active(){return false;}
    queue.inner.stopping.store(true,Ordering::Release);
    queue.update(&app,|s|s.stage="Stopping after current lesson".into());
    let _=hybrid_lab::hybrid_request_stop(&app,&brain,"native-zip-stop");
    true
}
/// Re-validates the original ZIP on the Rust side. No JS events are required
/// to advance the queue, preserve checkpoint selection, or archive lessons.
#[tauri::command]
pub fn hybrid_zip_train(app:AppHandle,brain:State<'_,HybridState>,
    queue:State<'_,ZipQueueState>,bytes:Vec<u8>,mode:String,epochs:u32,
    options:Option<TrainOptions>)->Result<(),String>{
    if !(1..=60).contains(&epochs){return Err("Choose 1–60 epochs".into());}
    if brain.is_training(){return Err("A neural training session is already active".into());}
    let prepared=read_bundle(&bytes,&mode)?;
    let scope=options.as_ref().and_then(|o|o.foundation_zip_scope.as_deref())
        .unwrap_or("firstFive").to_string();
    if !matches!(scope.as_str(),"firstFive"|"entireZip"){
        return Err("ZIP foundation duration must be firstFive or entireZip".into());
    }
    if queue.inner.active.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire)
        .is_err(){return Err("ZIP training queue is already running".into());}
    queue.inner.stopping.store(false,Ordering::Release);
    let queue=queue.inner().clone();
    queue.update(&app,|s|{
        *s=ZipTrainingStatus{active:true,total:prepared.lessons.len(),
            stage:"Starting native ZIP training".into(),..Default::default()};
    });
    tauri::async_runtime::spawn_blocking(move||{
        let result=(||->Result<(),String>{
            'lessons: for (i,lesson) in prepared.lessons.iter().enumerate(){
                if queue.inner.stopping.load(Ordering::Acquire){break;}
                let selective=options.as_ref().is_some_and(|o|o.zip_foundation_enabled()) &&
                    (scope=="entireZip" || i<5);
                let max_attempts:usize=if selective{4}else{1};
                let starting_lr=options.as_ref()
                    .map(|o|o.zip_learning_rate()).unwrap_or(0.001);
                // Rates below 0.000020 are used only by recovery attempts.
                // No failed candidate changes the complete accepted checkpoint.
                queue.update(&app,|s|{
                    s.current=i+1;s.lesson=lesson.name.clone();
                    s.attempt=0;s.retry_strategy.clear();s.learning_evidence=None;
                    s.stage="Preparing lesson in native Rust".into();
                });
                let mut final_outcome=LessonDisposition::BaselineRetained;
                for attempt in 0..max_attempts {
                    if queue.inner.stopping.load(Ordering::Acquire){break 'lessons;}
                    let (rate,attempt_epochs,alt,strategy)=
                        recovery_plan(attempt,starting_lr,epochs);
                    queue.update(&app,|s|{
                        s.attempt=(attempt+1) as u8;
                        s.retry_strategy=strategy.into();
                        s.stage=format!("Lesson {} of {} · attempt {}/{} · {} · LR {:.6} · {} epochs",
                            i+1,prepared.lessons.len(),attempt+1,max_attempts,
                            strategy,rate,attempt_epochs);
                    });
                    // The ZIP defines scope. Every attempt restarts from the
                    // last accepted complete brain and optimizer state.
                    let attempt_options=options.clone().map(|o|o.for_zip_attempt(
                        &scope,(i+1) as u32,prepared.lessons.len() as u32,
                        attempt as u8,rate,alt));
                    hybrid_lab::hybrid_train(
                        app.clone(),app.state::<HybridState>(),
                        Lesson{mode:lesson.mode.clone(),text:lesson.text.clone(),
                            epochs:attempt_epochs},
                        attempt_options)?;
                    // Android WebView lifecycle never controls this queue.
                    loop {
                        if queue.inner.stopping.load(Ordering::Acquire){
                            let _=hybrid_lab::hybrid_request_stop(
                                &app,&app.state::<HybridState>(),"native-zip-stop");
                        }
                        if !app.state::<HybridState>().is_training(){break;}
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    let (best_epoch,error,evidence)=hybrid_lab::completed_training_outcome(
                        &app.state::<HybridState>())?;
                    if queue.inner.stopping.load(Ordering::Acquire){break 'lessons;}
                    let disposition=classify_lesson_outcome(best_epoch,error)
                        .map_err(|e|format!("{}: {}",lesson.name,e))?;
                    queue.update(&app,|s|{
                        s.learning_evidence=evidence;
                    });
                    if matches!(disposition,LessonDisposition::Accepted(_)){
                        final_outcome=disposition;
                        break;
                    }
                    if attempt+1<max_attempts {
                        queue.update(&app,|s|{
                            let finding=s.learning_evidence.as_ref()
                                .map(|e|e.finding.as_str())
                                .unwrap_or("insufficient-evidence");
                            s.stage=format!(
                                "No eligible checkpoint for {} · {} · retry {}/{} next",
                                lesson.name,finding,attempt+1,max_attempts-1);
                        });
                    }
                }
                queue.update(&app,|s|
                    apply_lesson_disposition(s,&lesson.name,final_outcome));
            }
            Ok(())
        })();
        if let Err(ref error)=result{
            queue.update(&app,|s|{
                s.failed+=1;s.error=Some(error.clone());s.stage="ZIP training failed".into();
            });
        }
        let stopping=queue.inner.stopping.load(Ordering::Acquire);
        queue.inner.active.store(false,Ordering::Release);
        queue.update(&app,|s|{
            s.active=false;
            s.stage=if result.is_err(){"Stopped after error".into()}
                else if stopping{"Stopped by teacher".into()}
                else{format!("Finished ZIP education: {} accepted, {} skipped (baseline retained)",
                    s.completed,s.skipped)};
        });
        if let Ok(status)=queue.snapshot(){
            let _=app.emit("hybrid-zip-done",status);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::io::Write;
    #[test]fn foundation_retries_use_distinct_approaches(){
        let baseline=recovery_plan(0,0.0001,12);
        let precision=recovery_plan(1,0.0001,12);
        let alternate=recovery_plan(2,0.0001,12);
        let combined=recovery_plan(3,0.0001,12);
        assert_eq!((baseline.0,baseline.1,baseline.2),(0.0001,12,false));
        assert!(precision.0<0.00002 && precision.0<baseline.0);
        assert_eq!((precision.1,precision.2),(12,false));
        assert_eq!((alternate.0,alternate.1,alternate.2),
            (baseline.0,18,true));
        assert_eq!((combined.0,combined.1,combined.2),
            (precision.0,18,true));
        assert_eq!(recovery_plan(3,0.0001,60).1,60);
    }
    fn make_zip(entries:&[(&str,&str)])->Vec<u8>{
        let mut out=Cursor::new(Vec::new());
        {
            let mut zip=zip::ZipWriter::new(&mut out);
            for (name,text) in entries{
                zip.start_file(*name,zip::write::SimpleFileOptions::default()).unwrap();
                zip.write_all(text.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }
    fn curriculum()->String{
        (0..8).map(|n|format!("Unique text section {n} describes a new event with different details."))
            .collect::<Vec<_>>().join("\n\n")
    }
    #[test]fn protected_baseline_allows_following_zip_lessons(){
        let data=curriculum();
        let bundle=read_bundle(&make_zip(&[
            ("01_baseline.txt",&data),("02_second.txt",&data)]),"text").unwrap();
        let mut status=ZipTrainingStatus{
            total:bundle.lessons.len(),..Default::default()
        };
        // First lesson early-stops with no eligible improvement. The shared
        // neural trainer has already restored its protected checkpoint.
        status.current=1;
        let first=classify_lesson_outcome(0,None).unwrap();
        assert_eq!(first,LessonDisposition::BaselineRetained);
        apply_lesson_disposition(&mut status,&bundle.lessons[0].name,first);
        assert_eq!(status.skipped,1);
        assert_eq!(status.completed,0);
        assert_eq!(status.failed,0);
        assert!(status.stage.contains("Continuing to next lesson"));
        // Next ZIP lesson is still scheduled and can actually be accepted.
        status.current=2;
        let second=classify_lesson_outcome(3,None).unwrap();
        assert_eq!(second,LessonDisposition::Accepted(3));
        apply_lesson_disposition(&mut status,&bundle.lessons[1].name,second);
        assert_eq!(status.completed,1);
        assert_eq!(status.skipped,1);
        assert_eq!(status.current,2);
        let failure=classify_lesson_outcome(0,Some("checkpoint save failed".into()));
        assert_eq!(failure.unwrap_err(),"checkpoint save failed");
        // A genuine write/training failure is still fatal.
    }
    #[test]fn skipped_lessons_are_not_mislabeled_as_accepted(){
        let mut progress=ZipTrainingStatus::default();
        apply_lesson_disposition(&mut progress,"01.txt",
            LessonDisposition::BaselineRetained);
        assert_eq!(progress.completed,0);
        assert_eq!(progress.skipped,1);
        assert!(progress.stage.contains("No eligible improvement"));
        apply_lesson_disposition(&mut progress,"02.txt",
            LessonDisposition::Accepted(4));
        assert_eq!(progress.completed,1);
        assert_eq!(progress.skipped,1);
    }
    #[test]fn sorted_and_validated_lessons(){
        let c=curriculum();
        let data=make_zip(&[("02_second.txt",&c),("01_first.txt",&c)]);
        let bundle=read_bundle(&data,"text").unwrap();
        assert_eq!(bundle.lessons[0].name,"01_first.txt");
        assert_eq!(bundle.lessons.len(),2);
        assert!(bundle.lessons.iter().all(|x|x.training_samples>0&&x.heldout_samples>0));
    }
    #[test]fn refuse_traversal_and_unsupported_files(){
        let c=curriculum();
        assert!(read_bundle(&make_zip(&[("../evil.txt",&c)]),"text").is_err());
        assert!(read_bundle(&make_zip(&[("script.js","alert('hi')")]),"text").is_err());
        assert!(read_bundle(&make_zip(&[("good.txt",&c)]),"unknown").is_err());
    }
    #[test]fn reasoning_zip_uses_existing_teacher_mode_and_splits_holdouts(){
        let data=(0..8).map(|n|format!(
            "The box {n} is red.\tWhat color is box {n}?\tRed.\tBlue."
        )).collect::<Vec<_>>().join("\n");
        let zipped=make_zip(&[("01_reasoning.txt",&data)]);
        let bundle=read_bundle(&zipped,"reasoning").unwrap();
        assert_eq!(bundle.mode,"reasoning");
        assert_eq!(bundle.lessons.len(),1);
        assert!(bundle.lessons[0].training_samples>0);
        assert!(bundle.lessons[0].heldout_samples>0);
    }
    #[test]fn auto_detects_mixed_block_lessons_and_numeric_sort(){
        let template=concat!(
            "[Dialogue Lesson]\nGreetings help start conversation.\n[examples]\n",
            "hello  Hi!\nbye  Goodbye!\n",
            "[/examples]\n[/Dialogue Lesson]\n"
        );
        let bytes=make_zip(&[("10_final.txt",template),("02_second.txt",template),
            ("01_first.txt",template)]);
        let parsed=read_bundle(&bytes,"auto").unwrap();
        assert_eq!(parsed.mode,"unified");
        assert_eq!(parsed.lessons.iter().map(|x|x.name.as_str()).collect::<Vec<_>>(),
            vec!["01_first.txt","02_second.txt","10_final.txt"]);
        assert!(parsed.lessons.iter().all(|x|x.mode=="unified"));
        assert!(read_bundle(&make_zip(&[("01_old.txt","hello\\tHi!")]),"auto").is_err());
    }
    #[test]fn require_real_distinct_text_sections(){
        let data=make_zip(&[("weak.txt",&"abcde ".repeat(100))]);
        assert!(read_bundle(&data,"text").is_err());
    }
}