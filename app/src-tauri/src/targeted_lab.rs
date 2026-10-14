//! Teacher-only targeted foundation training, separate from ordinary shared
//! curriculum. A candidate is a clone of ONE full hybrid neural mind.
//! Hemisphere/interconnect focus masks optimizer updates, NOT neural inference.
//! Imported checkpoints are staged and validated without replacing live Nova.
use crate::checkpoint_transfer::TransferState;
use crate::hybrid_lab::{self,HybridState};
use crate::training_observer as observer;
use neural_nova::hybrid::{HybridBrain,TrainingFocus};
use neural_nova::trainer::{prepare,training_order,Example,HeldoutReply,PreparedLessons};
use neural_nova::unified::{DIALOGUE,UNDERSTANDING,CONVERSATION,REASONING};
use serde::Serialize;
use std::fs;
use std::sync::{Arc,Mutex};
use std::sync::atomic::{AtomicBool,Ordering};
use tauri::{AppHandle,Emitter,Manager,State};

#[cfg(target_os="android")]
fn target_training_notification(app:&AppHandle,body:String,finished:bool){
    use tauri_plugin_notification::NotificationExt;
    let mut note=app.notification().builder().id(90217).silent()
        .title(if finished{"Nova targeted training finished"}
               else{"Nova targeted training in progress"})
        .body(body);
    if !finished{note=note.ongoing();}
    let _=note.show();
}
#[derive(Default)]
pub struct TargetedState {
    inner:Arc<Mutex<TargetedSession>>,
    import:TransferState,
    stop:Arc<AtomicBool>,
}
#[derive(Default)]
struct TargetedSession {
    imported:Option<HybridBrain>,
    import_label:Option<String>,
    staged:Option<Staged>,
    running:bool,epoch:u32,epochs:u32,completed:usize,total:usize,
    stage:String,error:Option<String>,last_loss:Option<f32>,
}
struct Staged {
    baseline:TargetedScores,candidate_score:TargetedScores,
    brain:HybridBrain,active_fingerprint:String,
    source:String,focus:String,skill:String,best_epoch:u32,
    warnings:Vec<String>,
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct TargetedScores {
    heldout_loss:f32,next_character_accuracy:f32,
    self_generated_prefix_accuracy:f32,
    self_generated_first_accuracy:f32,
    complete_correct:usize,complete_checked:usize,
    contrastive_accuracy:Option<f32>,
    archive_complete_correct:usize,archive_complete_checked:usize,
    archive_prefix_accuracy:f32,
    archive_probe_count:usize,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TargetedStatus {
    running:bool,epoch:u32,epochs:u32,completed:usize,total:usize,
    stage:String,error:Option<String>,train_loss:Option<f32>,
    imported_ready:bool,import_label:Option<String>,
    candidate_ready:bool,source:Option<String>,focus:Option<String>,
    skill:Option<String>,best_epoch:Option<u32>,
    baseline:Option<TargetedScores>,candidate:Option<TargetedScores>,
    warnings:Vec<String>,recommendation:String,
}
fn lock(state:&TargetedState)->Result<std::sync::MutexGuard<'_,TargetedSession>,String>{
    state.inner.lock().map_err(|_|"Targeted training state lock poisoned".into())
}
fn status(s:&TargetedSession)->TargetedStatus{
    let staged=s.staged.as_ref();
    TargetedStatus{
        running:s.running,epoch:s.epoch,epochs:s.epochs,
        completed:s.completed,total:s.total,
        stage:s.stage.clone(),error:s.error.clone(),train_loss:s.last_loss,
        imported_ready:s.imported.is_some(),import_label:s.import_label.clone(),
        candidate_ready:staged.is_some(),
        source:staged.map(|v|v.source.clone()),
        focus:staged.map(|v|v.focus.clone()),
        skill:staged.map(|v|v.skill.clone()),
        best_epoch:staged.map(|v|v.best_epoch),
        baseline:staged.map(|v|v.baseline.clone()),
        candidate:staged.map(|v|v.candidate_score.clone()),
        warnings:staged.map(|v|v.warnings.clone()).unwrap_or_default(),
        recommendation:if let Some(value)=staged{
            if value.warnings.is_empty(){
                "No regression detected on the sampled held-out panels; you may accept or discard this checkpoint."
            }else{
                "Review the reported regressions. Approval requires explicit permission to accept a regression."
            }
        }else{"Run focused training to compare a staged candidate with its unchanged starting checkpoint."}.into(),
    }
}
#[tauri::command]
pub fn targeted_status(state:State<'_,TargetedState>)->Result<TargetedStatus,String>{
    let session=lock(&state)?;
    Ok(status(&session))
}
#[tauri::command]
pub fn targeted_import_begin(state:State<'_,TargetedState>,
    hybrid:State<'_,HybridState>,bytes:usize)->Result<(),String>{
    if hybrid.is_training()||lock(&state)?.running{
        return Err("Stop all training before importing a targeted source".into());
    }
    state.import.import_begin(bytes)
}
#[tauri::command]
pub fn targeted_import_append(state:State<'_,TargetedState>,
    chunk:Vec<u8>)->Result<usize,String>{
    state.import.import_append(chunk)
}
#[tauri::command]
pub fn targeted_import_cancel(state:State<'_,TargetedState>)->Result<(),String>{
    state.import.cancel()
}
#[tauri::command]
pub fn targeted_import_finish(app:AppHandle,state:State<'_,TargetedState>,
    hybrid:State<'_,HybridState>,filename:String)->Result<TargetedStatus,String>{
    if hybrid.is_training(){return Err("Stop neural training before importing".into());}
    if filename.len()>160 || !filename.to_ascii_lowercase().ends_with(".nhyb"){
        return Err("Select a .nhyb neural checkpoint".into());
    }
    let bytes=state.import.import_read()?;
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let path=dir.join("neural-nova-targeted-import.tmp");
    fs::write(&path,bytes).map_err(|e|e.to_string())?;
    let brain=HybridBrain::load(&path);
    let _=fs::remove_file(&path);
    let brain=brain?;
    let mut s=lock(&state)?;
    if s.running{return Err("Targeted training began during import".into());}
    s.imported=Some(brain);s.import_label=Some(filename);
    s.staged=None;s.stage="Imported checkpoint validated and staged; live Nova unchanged".into();
    let result=status(&s);
    drop(s);
    state.import.cancel()?;
    Ok(result)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Skill{Dialogue,Context,Conversation,Reasoning}
impl Skill {
    fn parse(value:&str)->Result<Self,String>{
        match value{
            "dialogue"=>Ok(Self::Dialogue),"context"=>Ok(Self::Context),
            "conversation"=>Ok(Self::Conversation),"reasoning"=>Ok(Self::Reasoning),
            _=>Err("Select dialogue, context, conversation, or reasoning".into())
        }
    }
    fn kind(self)->u8{
        match self{Self::Dialogue=>DIALOGUE,Self::Context=>UNDERSTANDING,
            Self::Conversation=>CONVERSATION,Self::Reasoning=>REASONING}
    }
}
fn focus(value:&str)->Result<TrainingFocus,String>{
    match value{
        "dense"=>Ok(TrainingFocus::Dense),"transformer"=>Ok(TrainingFocus::Transformer),
        "interconnect"=>Ok(TrainingFocus::Interconnect),"joint"=>Ok(TrainingFocus::Joint),
        _=>Err("Select dense, Transformer, interconnect, or joint learning".into())
    }
}
struct FocusedPlan {
    training:Vec<(Example,Vec<String>)>,
    validation:Vec<(Example,Vec<String>)>,
    contrastive_train:Vec<neural_nova::trainer::ReasoningExample>,
    contrastive_test:Vec<neural_nova::trainer::ReasoningExample>,
    heldout:Vec<HeldoutReply>,
    training_text:String,
}
fn select(plan:PreparedLessons,skill:Skill)->Result<FocusedPlan,String>{
    if plan.mode!="unified" || plan.train_kinds.len()!=plan.train.len() ||
        plan.validation_kinds.len()!=plan.validation.len() ||
        plan.memory_train.len()!=plan.train.len() ||
        plan.memory_validation.len()!=plan.validation.len(){
        return Err("Targeted Training requires validated, structured unified lesson blocks".into());
    }
    let k=skill.kind();
    let training=plan.train.iter().zip(&plan.memory_train).zip(&plan.train_kinds)
        .filter(|(_,kind)|**kind==k)
        .map(|((sample,memory),_)|(sample.clone(),memory.clone())).collect::<Vec<_>>();
    let validation=plan.validation.iter().zip(&plan.memory_validation)
        .zip(&plan.validation_kinds).filter(|(_,kind)|**kind==k)
        .map(|((sample,memory),_)|(sample.clone(),memory.clone())).collect::<Vec<_>>();
    if training.is_empty() || validation.is_empty(){
        return Err(format!("This lesson lacks enough distinct {:?} examples for both training and held-out evaluation. Supply at least five independent examples of the selected block type.",skill));
    }
    let heldout=plan.heldout_replies.into_iter().filter(|x|x.3==k).collect();
    let (contrastive_train,contrastive_test)=match skill{
        Skill::Context=>(plan.understanding_train,plan.understanding_validation),
        Skill::Reasoning=>(plan.reasoning_train,plan.reasoning_validation),
        _=>(Vec::new(),Vec::new()),
    };
    // Vocabulary discovery is limited to the SELECTED supervision stream.
    // Otherwise a dense-only focus could silently introduce Transformer word
    // units from unrelated lesson blocks despite freezing its weights.
    let mut focused_text=String::new();
    for (sample,_) in training.iter().step_by(12){
        if focused_text.len()+sample.0.len()+2>50_000{break;}
        focused_text.push_str(std::str::from_utf8(&sample.0).map_err(|e|e.to_string())?);
        focused_text.push(sample.1 as char);
        focused_text.push('\n');
    }
    Ok(FocusedPlan{training,validation,heldout,contrastive_train,contrastive_test,
        training_text:focused_text})
}
fn evaluate(brain:&HybridBrain,plan:&FocusedPlan,archive:&[HeldoutReply])
    ->Result<TargetedScores,String>{
    // Uniform deterministic coverage of the held-out validation characters,
    // rather than the first few (which can systematically bias the score).
    let stride=(plan.validation.len()/256).max(1);
    let mut total_loss=0.0f64;let mut correct=0usize;let mut checked=0usize;
    for (example,memory) in plan.validation.iter().step_by(stride).take(256){
        let probabilities=brain.predict_with_memory(&example.0,256,memory)?;
        total_loss-=(probabilities[example.1 as usize].max(1e-30) as f64).ln();
        let chosen=probabilities.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1))
            .ok_or("Empty neural prediction")?.0;
        correct+=usize::from(chosen==example.1 as usize);
        checked+=1;
    }
    if checked==0{return Err("No held-out examples".into());}
    let mut ranking=0usize;
    for (prompt,good,bad) in plan.contrastive_test.iter().take(8){
        ranking+=usize::from(brain.reasoning_margin(prompt,good,bad)?.1);
    }
    let (prefix,first,complete,possible)=generate_panel(brain,&plan.heldout,4)?;
    let (old_prefix,_,old_complete,old_possible)=generate_panel(brain,archive,8)?;
    let result=TargetedScores{
        heldout_loss:(total_loss/checked as f64) as f32,
        next_character_accuracy:correct as f32/checked as f32,
        self_generated_prefix_accuracy:prefix,
        self_generated_first_accuracy:first,
        complete_correct:complete,complete_checked:possible,
        contrastive_accuracy:if plan.contrastive_test.is_empty(){None}
            else{Some(ranking as f32/plan.contrastive_test.len().min(8) as f32)},
        archive_complete_correct:old_complete,archive_complete_checked:old_possible,
        archive_prefix_accuracy:old_prefix,archive_probe_count:archive.len().min(8),
    };
    if !result.heldout_loss.is_finite(){return Err("Non-finite targeted validation loss".into());}
    Ok(result)
}
fn generate_panel(brain:&HybridBrain,items:&[HeldoutReply],limit:usize)
    ->Result<(f32,f32,usize,usize),String>{
    let mut total=0usize;let mut matches=0usize;let mut initials=0usize;
    let mut complete=0usize;let mut possible=0usize;
    for (prompt,expected,memory,_) in items.iter().take(limit){
        let expected_bytes=expected.iter().copied().chain(std::iter::once(b'\n'))
            .take(32).collect::<Vec<_>>();
        if expected_bytes.is_empty(){continue;}
        let complete_sample=expected.len()+1<=32;
        if complete_sample{possible+=1;}
        let mut seq=prompt.clone();
        let mut generated=Vec::<u8>::new();let mut ended=false;
        for (n,&want) in expected_bytes.iter().enumerate(){
            let p=brain.predict_with_memory(&seq,256,memory)?;
            let got=p.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1))
                .ok_or("Empty neural prediction")?.0 as u8;
            total+=1;matches+=usize::from(got==want);
            if n==0{initials+=usize::from(got==want);}
            if got==b'\n'{ended=true;break;}
            generated.push(got);seq.push(got);
            if seq.len()>512{seq.drain(..seq.len()-512);}
        }
        if complete_sample&&ended&&generated==*expected{complete+=1;}
        // Missing suffix counts as incorrect predictions, not omitted.
        total+=expected_bytes.len().saturating_sub(generated.len()+usize::from(ended));
    }
    Ok((matches as f32/total.max(1) as f32,
        initials as f32/items.len().min(limit).max(1) as f32,complete,possible))
}
fn warnings(baseline:&TargetedScores,candidate:&TargetedScores)->Vec<String>{
    let mut w=Vec::new();
    if candidate.heldout_loss>baseline.heldout_loss+0.08{
        w.push("Selected-skill held-out loss deteriorated by more than 0.08".into());
    }
    if candidate.complete_correct<baseline.complete_correct{
        w.push("Current skill lost previously correct complete self-generated answers".into());
    }
    if candidate.self_generated_first_accuracy+1e-6<baseline.self_generated_first_accuracy{
        w.push("Current skill first-character accuracy declined".into());
    }
    if candidate.self_generated_prefix_accuracy+0.05<baseline.self_generated_prefix_accuracy{
        w.push("Current skill self-generated continuation accuracy declined".into());
    }
    if candidate.archive_complete_correct<baseline.archive_complete_correct{
        w.push("Earlier accepted lessons lost complete correct replies on the sample".into());
    }
    if candidate.archive_prefix_accuracy+0.05<baseline.archive_prefix_accuracy{
        w.push("Earlier accepted lessons' self-generated prefixes regressed".into());
    }
    if let (Some(before),Some(after))=(baseline.contrastive_accuracy,candidate.contrastive_accuracy){
        if after+1e-6<before{
            w.push("Supported-versus-contradicted conclusion preference regressed".into());
        }
    }
    w
}
fn selection_score(s:&TargetedScores)->f32{
    s.heldout_loss+0.2*(1.0-s.self_generated_prefix_accuracy)
        +s.contrastive_accuracy.map(|x|0.3*(1.0-x)).unwrap_or(0.0)
}
#[tauri::command]
pub fn targeted_train(app:AppHandle,state:State<'_,TargetedState>,
    hybrid:State<'_,HybridState>,source:String,focus_name:String,
    skill_name:String,lesson_text:String,epochs:u32,learning_rate:f32
)->Result<(),String>{
    if !(1..=30).contains(&epochs){return Err("Choose 1–30 targeted epochs".into());}
    if !(0.0001..=0.006).contains(&learning_rate)||!learning_rate.is_finite(){
        return Err("Targeted learning rate must be between 0.0001 and 0.006".into());
    }
    let gradient_focus=focus(&focus_name)?;
    let skill=Skill::parse(&skill_name)?;
    if source!="current"&&source!="imported"{return Err("Select current or imported checkpoint".into());}
    let plan=select(prepare("unified",&lesson_text)?,skill)?;
    {
        let s=lock(&state)?;
        if s.running{return Err("Targeted learning is already running".into());}
        if s.staged.is_some(){
            return Err("Accept or discard the previous staged candidate before starting a new targeted run".into());
        }
        if source=="imported" && s.imported.is_none(){
            return Err("Import a .nhyb file into the Targeted Training workspace first".into());
        }
    }
    hybrid_lab::targeted_reserve(&hybrid)?;
    let setup=(||->Result<(HybridBrain,String,Vec<HeldoutReply>),String>{
        let (live,digest)=hybrid_lab::targeted_snapshot(&hybrid)?;
        let archive=if source=="current"{
            hybrid_lab::targeted_archived_validation(&app,&hybrid)?
        }else{Vec::new()};
        let imported=if source=="imported"{lock(&state)?.imported.clone()}else{None};
        Ok((imported.unwrap_or(live),digest,archive))
    })();
    let (base,active_fingerprint,archive)=match setup{
        Ok(data)=>data,Err(e)=>{hybrid_lab::targeted_release(&hybrid);return Err(e);}
    };
    let mut start=base.clone();
    // Tokenizer grows from TRAIN text only; the imported/current full brain
    // remains unchanged until manual approval.
    // Word-unit growth is itself a Transformer representation change.
    // Keep the tokenizer bit-for-bit unchanged when only Dense or Interconnect
    // weights are selected. Both streams still read the same lossless inputs.
    if matches!(gradient_focus,TrainingFocus::Transformer|TrainingFocus::Joint){
        start.learn_token_units(&plan.training_text);
    }
    let base_scores=match evaluate(&start,&plan,&archive){
        Ok(score)=>score,Err(e)=>{hybrid_lab::targeted_release(&hybrid);return Err(e);}
    };
    let observer_id=match observer::start(&app,"targeted",&skill_name){
        Ok(id)=>id,Err(e)=>{hybrid_lab::targeted_release(&hybrid);return Err(e);}
    };
    {
        let mut s=lock(&state)?;
        s.running=true;s.epoch=0;s.epochs=epochs;s.completed=0;
        s.total=plan.training.len()*epochs as usize;
        s.stage="Targeted neural candidate training (live checkpoint untouched)".into();
        s.error=None;s.last_loss=None;s.staged=None;
    }
    state.stop.store(false,Ordering::Release);
    let session=state.inner.clone();
    let cancelled=state.stop.clone();
    let hybrid=hybrid.inner().clone();
    #[cfg(target_os="android")]
    let notice_app=app.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let outcome=(||->Result<Staged,String>{
            let mut brain=start.clone();
            let mut best_brain=start;
            let mut best_scores=base_scores.clone();
            let mut best_epoch=0;
            let practice=observer::training_replies(plan.training.iter()
                .map(|(example,memory)|(example,memory,skill.kind())));
            let baseline=observer::capture(&brain,"targeted",&skill_name,0,
                &practice,&plan.heldout,serde_json::json!({
                    "baseline":true,"scores":base_scores
                })).and_then(|data|observer::append(&app,&observer_id,&data));
            let _=app.emit("nova-observer-epoch",serde_json::json!({
                "sessionId":observer_id,"epoch":0,"error":baseline.err()
            }));
            for epoch in 1..=epochs{
                if cancelled.load(Ordering::Acquire){break;}
                let data=plan.training.iter().map(|x|x.0.clone()).collect::<Vec<_>>();
                let order=training_order(&data,epoch,"unified");
                let mut losses=0.0f64;let mut count=0usize;
                for (batch_index,indices) in order.chunks(8).enumerate(){
                    if cancelled.load(Ordering::Acquire){break;}
                    let with_memory=indices.iter().map(|&i|{
                        (plan.training[i].0.0.clone(),plan.training[i].0.1,
                            plan.training[i].1.clone())
                    }).collect::<Vec<_>>();
                    let value=if skill==Skill::Conversation{
                        brain.train_memory_batch_focused(&with_memory,learning_rate,gradient_focus)?
                    }else{
                        let batch=with_memory.into_iter().map(|(x,y,_)|(x,y)).collect::<Vec<_>>();
                        brain.train_batch_focused(&batch,learning_rate,gradient_focus)?
                    };
                    losses+=value as f64*indices.len() as f64;count+=indices.len();
                    if !plan.contrastive_train.is_empty()&&batch_index%24==0 {
                        let idx=(batch_index+epoch as usize-1)%plan.contrastive_train.len();
                        let pair=plan.contrastive_train[idx].clone();
                        brain.train_preference_batch_focused(&[pair],learning_rate,gradient_focus)?;
                    }
                    if batch_index%16==0 {
                        let mut s=session.lock().map_err(|_|"Targeted progress lock poisoned")?;
                        s.epoch=epoch;s.completed=((epoch-1) as usize*plan.training.len()+count)
                            .min(s.total);s.last_loss=Some((losses/count as f64) as f32);
                    }
                }
                if count==0{break;}
                let score=evaluate(&brain,&plan,&archive)?;
                let epoch_heldout_loss=score.heldout_loss;
                let epoch_scores=serde_json::to_value(&score).map_err(|e|e.to_string())?;
                let improves=selection_score(&score)+0.0005<selection_score(&best_scores);
                if improves {
                    best_brain=brain.clone();
                    best_scores=score;
                    best_epoch=epoch;
                }
                let activity=observer::capture(&brain,"targeted",&skill_name,epoch,
                    &practice,&plan.heldout,serde_json::json!({
                        "scores":epoch_scores,"trainingLoss":losses/count as f64,
                        "selected":improves,"selectedEpoch":best_epoch,
                        "focus":focus_name,"source":source
                    })).and_then(|data|observer::append(&app,&observer_id,&data));
                let activity_error=activity.err();
                let _=app.emit("nova-observer-epoch",serde_json::json!({
                    "sessionId":observer_id,"epoch":epoch,"error":activity_error
                }));
                #[cfg(target_os="android")]
                target_training_notification(&notice_app,format!(
                    "{} focus · {} objective · epoch {}/{} · held-out loss {:.4}",
                    focus_name,skill_name,epoch,epochs,epoch_heldout_loss
                ),false);
                let mut s=session.lock().map_err(|_|"Targeted progress lock poisoned")?;
                s.epoch=epoch;s.completed=epoch as usize*plan.training.len();
                s.last_loss=Some((losses/count as f64) as f32);
                s.stage=format!("Epoch {epoch}/{epochs} evaluated; selected targeted epoch {best_epoch}{}",activity_error.map(|e|format!("; activity log error: {e}")).unwrap_or_default());
            }
            let w=warnings(&base_scores,&best_scores);
            Ok(Staged{
                baseline:base_scores,candidate_score:best_scores,
                brain:best_brain,active_fingerprint,
                source,focus:focus_name,skill:skill_name,best_epoch,
                warnings:w,
            })
        })();
        match session.lock(){
            Ok(mut s)=>{
                s.running=false;
                match outcome{
                    Ok(staged)=>{
                        s.stage=if cancelled.load(Ordering::Acquire){
                            "Targeted training stopped; best completed candidate available"
                        }else{"Targeted candidate ready for comparison and manual approval"}.into();
                        s.staged=Some(staged);
                    },
                    Err(e)=>{s.error=Some(e);s.stage="Targeted candidate failed; live Nova unchanged".into();}
                }
            },Err(_)=>{}
        }
        hybrid_lab::targeted_release(&hybrid);
        #[cfg(target_os="android")]
        target_training_notification(&notice_app,
            "Targeted learning completed. Review the candidate before accepting.".into(),true);
    });
    Ok(())
}
#[tauri::command]
pub fn targeted_stop(state:State<'_,TargetedState>)->Result<(),String>{
    if !lock(&state)?.running{return Err("No targeted training is running".into());}
    state.stop.store(true,Ordering::Release);Ok(())
}
#[tauri::command]
pub fn targeted_discard(state:State<'_,TargetedState>)->Result<TargetedStatus,String>{
    let mut s=lock(&state)?;
    if s.running{return Err("Wait for focused training to stop before discarding".into());}
    s.staged=None;s.stage="Experimental candidate discarded; live Nova unchanged".into();
    Ok(status(&s))
}
#[tauri::command]
pub fn targeted_accept(app:AppHandle,state:State<'_,TargetedState>,
    hybrid:State<'_,HybridState>,allow_regression:bool)->Result<TargetedStatus,String>{
    let mut s=lock(&state)?;
    if s.running{return Err("Finish targeted training before approving weights".into());}
    let staged=s.staged.as_ref().ok_or("No targeted checkpoint to approve")?;
    if staged.best_epoch==0{
        return Err("No targeted epoch improved the selected objective. The starting checkpoint has been retained; nothing to merge".into());
    }
    if !staged.warnings.is_empty()&&!allow_regression{
        return Err("Candidate has sampled skill/retention regressions; review and explicitly confirm override to approve".into());
    }
    hybrid_lab::targeted_commit(&app,&hybrid,&staged.brain,&staged.active_fingerprint)?;
    s.staged=None;s.imported=None;s.import_label=None;
    s.stage="Teacher accepted targeted neural checkpoint; both hemispheres still cooperate".into();
    Ok(status(&s))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn real_unified_objectives_are_not_label_only(){
        let input=concat!(
            "[Dialogue Lesson]\nConversation greetings.\n[examples]\n",
            "hello  Hello!\nhi  Hi!\nhey  Hello.\nbye  Bye.\nthanks  You're welcome.\n",
            "[/examples]\n[/Dialogue Lesson]\n",
            "[Understanding Lesson]\nProperties and objects.\n[examples]\n",
            "Mira has a red box. What color?  Red.\n",
            "Tim has a blue box. What color?  Blue.\n",
            "Nora has a white box. What color?  White.\n",
            "Eve has a green box. What color?  Green.\n",
            "Lee has a gold box. What color?  Gold.\n",
            "[/examples]\n[/Understanding Lesson]"
        );
        let p=prepare("unified",input).unwrap();
        let dialogue=select(p.clone(),Skill::Dialogue).unwrap();
        let context=select(p,Skill::Context).unwrap();
        assert!(dialogue.training.iter().all(|x|x.0.0.starts_with(b"U: ")));
        assert!(!context.training.is_empty());
        assert_ne!(dialogue.training[0].0.0,context.training[0].0.0);
        assert!(context.contrastive_test.len()<=5);
        assert_ne!(dialogue.validation[0].0.0,context.validation[0].0.0);
        assert_ne!(dialogue.training_text,context.training_text);
        assert!(!dialogue.training_text.is_empty());
        assert!(!context.training_text.is_empty());
    }
    #[test]
    fn objective_guard_detects_regression_without_modifying_inference(){
        let old=TargetedScores{
            heldout_loss:1.2,next_character_accuracy:0.5,
            self_generated_prefix_accuracy:0.75,self_generated_first_accuracy:1.0,
            complete_correct:3,complete_checked:4,contrastive_accuracy:Some(0.5),
            archive_complete_correct:3,archive_complete_checked:4,
            archive_prefix_accuracy:0.75,archive_probe_count:4,
        };
        let better=TargetedScores{heldout_loss:1.0,complete_correct:1,
            archive_complete_correct:1,..old.clone()};
        assert!(warnings(&old,&better).iter().any(|x|x.contains("Earlier accepted")));
        assert!(warnings(&old,&better).iter().any(|x|x.contains("Current skill lost")));
        assert!(warnings(&old,&old).is_empty());
    }
}
