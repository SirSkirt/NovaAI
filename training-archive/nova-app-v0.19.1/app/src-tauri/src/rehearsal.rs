//! Teacher-controlled, persistent rehearsal curricula. Never imported by
//! neural inference. A model only learns these samples via normal joint loss.
use neural_nova::trainer::{prepare,PreparedLessons};
use serde::{Serialize,Deserialize};
use std::{fs,path::Path};

pub const MAX_ARCHIVED_LESSONS:usize=128;
pub const FOUNDATION_LESSON_LIMIT:u8=5;
pub const RETENTION_VALIDATION_LIMIT:usize=64;
pub const RETENTION_PLANS_PER_CHECK:usize=16;
pub const MAX_ACTIVE_REPLAY_LESSONS:usize=12;
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LessonRecord{
    pub mode:String,pub text:String,pub name:String,
}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RehearsalBank{
    pub model_fingerprint:String,pub lessons:Vec<LessonRecord>,
    #[serde(default)] pub foundation_lessons_completed:u8,
}
pub fn read(path:&Path)->Result<RehearsalBank,String>{
    if !path.exists(){return Ok(RehearsalBank::default());}
    let raw=fs::read(path).map_err(|e|e.to_string())?;
    if raw.len()>8_000_000{return Err("Rehearsal bank exceeds 8MB; export or archive it before adding more lessons".into());}
    let bank:RehearsalBank=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
    if bank.foundation_lessons_completed>FOUNDATION_LESSON_LIMIT{return Err("Invalid foundation lesson count".into());}
    if bank.lessons.len()>MAX_ARCHIVED_LESSONS ||
      bank.lessons.iter().any(|x|x.text.len()>50_000 || x.name.len()>80){
        return Err("Rehearsal bank contains invalid lesson records".into());
    }
    Ok(bank)
}
pub fn write(path:&Path,bank:&RehearsalBank)->Result<(),String>{
    if bank.foundation_lessons_completed>FOUNDATION_LESSON_LIMIT {return Err("Invalid foundation lesson count".into());}
    if bank.lessons.len()>MAX_ARCHIVED_LESSONS {
        return Err("Rehearsal archive full; export old lessons rather than deleting them".into());
    }
    let raw=serde_json::to_vec(bank).map_err(|e|e.to_string())?;
    if raw.len()>8_000_000 {return Err("Rehearsal bank exceeds 8MB".into());}
    let temp=path.with_extension("tmp");
    {
        use std::io::Write;
        let mut output=fs::File::create(&temp).map_err(|e|e.to_string())?;
        output.write_all(&raw).map_err(|e|e.to_string())?;
        output.sync_all().map_err(|e|e.to_string())?;
    }
    // Keep the previous known-good bank until replacement is in place.
    let backup=path.with_extension("bak");
    if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
    if path.exists(){fs::rename(path,&backup).map_err(|e|e.to_string())?;}
    match fs::rename(&temp,path){
        Ok(())=>{if backup.exists(){let _=fs::remove_file(&backup);}Ok(())},
        Err(err)=>{if backup.exists(){let _=fs::rename(&backup,path);}Err(err.to_string())}
    }
}
pub fn matched(bank:RehearsalBank,fingerprint:&str)->RehearsalBank{
    if bank.model_fingerprint==fingerprint {bank}
    else {RehearsalBank{model_fingerprint:fingerprint.to_string(),lessons:Vec::new(),foundation_lessons_completed:0}}
}
pub fn append(bank:&mut RehearsalBank,record:LessonRecord,ending_fingerprint:String)->Result<(),String>{
    if record.text.len()>50_000||record.name.len()>80 {
        return Err("Lesson is too large for rehearsal archive".into());
    }
    // Recompute deterministic split to ensure the archived lesson is valid.
    prepare(&record.mode,&record.text)?;
    if let Some(i)=bank.lessons.iter().position(|v|v.mode==record.mode&&v.text==record.text){
        bank.lessons.remove(i);
    }
    if bank.lessons.len()==MAX_ARCHIVED_LESSONS {
        return Err("Rehearsal archive is full (128 lessons). No older material has been deleted.".into());
    }
    bank.lessons.push(record);
    bank.model_fingerprint=ending_fingerprint;
    Ok(())
}
/// Count successfully completed FOUNDATION lesson sessions even when no
/// candidate was eligible. Restores/foreign models cannot inherit the counter.
pub fn advance_foundation(bank:&mut RehearsalBank,ending_fingerprint:&str)
    ->Result<(),String>{
    if bank.model_fingerprint!=ending_fingerprint{
        return Err("Foundation counter fingerprint does not match selected checkpoint".into());
    }
    if bank.foundation_lessons_completed>=FOUNDATION_LESSON_LIMIT{
        return Err("Foundation's five lessons are already complete".into());
    }
    bank.foundation_lessons_completed+=1;
    Ok(())
}
/// The replay set contains only previous TRAIN splits. Held-out material is
/// reserved exclusively for checkpoint evaluation.
/// Keep all 128 original TXT sources on disk, but only materialize a
/// bounded category-balanced historical panel in RAM for this lesson.
pub fn active_replay_indices(bank:&RehearsalBank)->Vec<usize>{
    let mut selected=std::collections::BTreeSet::new();
    let tagged=|record:&LessonRecord,category:usize|{
        if record.mode=="unified"{
            let tag=match category{
                0=>"[Dialogue Lesson]",1=>"[Understanding Lesson]",
                2=>"[Multi-Turn Conversation Lesson]",3=>"[Reasoning Lesson]",
                _=>return true,
            };
            record.text.contains(tag)
        }else{
            matches!((category,record.mode.as_str()),
                (0,"dialogue")|(2,"conversation")|(3,"reasoning")|(4,"text"))
        }
    };
    for kind in 0..5{
        if let Some(i)=bank.lessons.iter().position(|v|tagged(v,kind)){
            selected.insert(i);
        }
        if let Some(i)=bank.lessons.iter().rposition(|v|tagged(v,kind)){
            selected.insert(i);
        }
    }
    if bank.lessons.is_empty(){return Vec::new();}
    let n=bank.lessons.len();
    for step in 0..MAX_ACTIVE_REPLAY_LESSONS{
        if selected.len()>=MAX_ACTIVE_REPLAY_LESSONS{break;}
        let i=if MAX_ACTIVE_REPLAY_LESSONS==1{0}
            else{step*(n-1)/(MAX_ACTIVE_REPLAY_LESSONS-1)};
        selected.insert(i);
    }
    if selected.len()<MAX_ACTIVE_REPLAY_LESSONS{
        for i in 0..n{
            if selected.len()>=MAX_ACTIVE_REPLAY_LESSONS{break;}
            selected.insert(i);
        }
    }
    selected.into_iter().take(MAX_ACTIVE_REPLAY_LESSONS).collect()
}
pub fn previous_plans(bank:&RehearsalBank)->Result<Vec<PreparedLessons>,String>{
    active_replay_indices(bank).into_iter()
        .map(|i|prepare(&bank.lessons[i].mode,&bank.lessons[i].text)).collect()
}
/// Bounded, deterministic long-term retention panel. Each observed category
/// contributes oldest + newest examples first, then evenly spaced archive
/// positions. The full archive remains intact and available for rehearsal.
/// Results are explicitly a sample, NOT an exhaustive memory guarantee.
pub fn retention_plan_indices(plans:&[PreparedLessons])->Vec<usize>{
    let mut chosen=std::collections::BTreeSet::new();
    for kind in 0..5usize{
        let represented=|plan:&PreparedLessons|{
            if plan.mode=="unified"{
                plan.category_train[kind]>0||plan.category_validation[kind]>0
            }else{
                match (kind,plan.mode.as_str()){
                    (0,"dialogue")|(2,"conversation")|(3,"reasoning")|(4,"text")=>true,
                    _=>false,
                }
            }
        };
        if let Some(i)=plans.iter().position(represented){chosen.insert(i);}
        if let Some(i)=plans.iter().rposition(represented){chosen.insert(i);}
    }
    if plans.is_empty(){return Vec::new();}
    let steps=RETENTION_PLANS_PER_CHECK.min(plans.len());
    for i in 0..steps{
        if chosen.len()>=RETENTION_PLANS_PER_CHECK{break;}
        let index=if steps==1{0}else{i*(plans.len()-1)/(steps-1)};
        chosen.insert(index);
    }
    if chosen.len()<RETENTION_PLANS_PER_CHECK{
        for i in 0..plans.len(){
            if chosen.len()>=RETENTION_PLANS_PER_CHECK{break;}
            chosen.insert(i);
        }
    }
    chosen.into_iter().collect()
}
/// Deterministic bounded per-prior-lesson validation score. Not a response
/// fallback and not put into train samples.
pub fn prior_validation_selection(plan:&PreparedLessons)->Vec<usize>{
    let n=plan.validation.len();
    if n<=RETENTION_VALIDATION_LIMIT{return (0..n).collect();}
    (0..RETENTION_VALIDATION_LIMIT)
        .map(|i|i*n/RETENTION_VALIDATION_LIMIT).collect()
}
/// Guard the new lesson against measurable deterioration on independently
/// held-out examples from earlier lessons. It cannot introduce canned answers.
pub fn eligible_checkpoint(
    previous_loss:Option<f32>,baseline_loss:Option<f32>,max_loss_increase:f32,
    teacher_accuracy:Option<f32>,teacher_baseline:Option<f32>,
)->bool{
    let prior_ok=match (previous_loss,baseline_loss){
        (Some(now),Some(before))=>now.is_finite() && before.is_finite() &&
            now<=before+max_loss_increase,
        (None,None)=>true,
        _=>false,
    };
    // A saved user-selected set of completed answers must not regress at all
    // in exact greedy accuracy. The teacher still controls this benchmark.
    let teacher_ok=match (teacher_accuracy,teacher_baseline){
        (Some(now),Some(before))=>now+0.000001>=before,
        (None,None)=>true,
        _=>false,
    };
    prior_ok&&teacher_ok
}
#[cfg(test)]
mod tests{
    use super::*;
    fn lesson(s:&str)->LessonRecord{LessonRecord{
        mode:"dialogue".into(),name:s.into(),
        text:format!("hello\tHi {s}\nbye\tBye {s}\ncat\tAnimal {s}\ndog\tPet {s}\nbird\tBird {s}\n")
    }}
    #[test]fn foundation_progress_survives_rejected_lessons_and_resets_on_new_brain(){
        let mut bank=matched(RehearsalBank::default(),"brain-a");
        for _ in 0..FOUNDATION_LESSON_LIMIT{
            advance_foundation(&mut bank,"brain-a").unwrap();
        }
        assert_eq!(bank.foundation_lessons_completed,5);
        assert!(advance_foundation(&mut bank,"brain-a").is_err());
        assert_eq!(matched(bank.clone(),"brain-a").foundation_lessons_completed,5);
        assert_eq!(matched(bank,"new-checkpoint").foundation_lessons_completed,0);
    }
    #[test]fn archive_never_silently_discards_older_lessons(){
        let mut bank=RehearsalBank::default();
        for i in 0..5{append(&mut bank,lesson(&i.to_string()),format!("fp-{i}")).unwrap();}
        assert_eq!(bank.lessons.len(),5);
        assert_eq!(matched(bank.clone(),"fp-4").lessons.len(),5);
        assert!(matched(bank,"other-checkpoint").lessons.is_empty());
    }
    #[test]fn archive_rejects_capacity_without_deleting_old_sources(){
        let mut bank=RehearsalBank::default();
        for i in 0..MAX_ARCHIVED_LESSONS{
            append(&mut bank,lesson(&i.to_string()),format!("fp-{i}")).unwrap();
        }
        let old=bank.lessons.clone();
        assert!(append(&mut bank,lesson("overflow"),"bad".into()).is_err());
        assert_eq!(bank.lessons.len(),old.len());
        assert_eq!(bank.lessons[0].text,old[0].text);
    }
    #[test]fn active_replay_is_bounded_but_archive_is_not_truncated(){
        let mut bank=RehearsalBank::default();
        for i in 0..70{append(&mut bank,lesson(&i.to_string()),format!("fp-{i}")).unwrap();}
        let ids=active_replay_indices(&bank);
        assert!(ids.len()<=MAX_ACTIVE_REPLAY_LESSONS);
        assert_eq!(bank.lessons.len(),70);
        assert_eq!(ids.first().copied(),Some(0));
        assert_eq!(ids.last().copied(),Some(69));
    }
    #[test]fn replay_never_trains_on_its_holdout(){
        let mut bank=RehearsalBank::default();
        append(&mut bank,lesson("one"),"fingerprint".into()).unwrap();
        let p=previous_plans(&bank).unwrap().pop().unwrap();
        // Held-out prompt groups are absent from training examples.
        let contexts=p.train.iter().map(|x|x.0.clone()).collect::<Vec<_>>();
        for pair in &p.validation{
            if pair.0.ends_with(b"\nN: "){assert!(!contexts.contains(&pair.0));}
        }
        assert!(!p.validation.is_empty());
    }
    #[test]fn protection_rejects_forgetting_even_if_current_lesson_improves(){
        assert!(eligible_checkpoint(Some(1.08),Some(1.0),0.10,Some(1.0),Some(1.0)));
        assert!(!eligible_checkpoint(Some(1.3),Some(1.0),0.10,None,None));
        assert!(!eligible_checkpoint(Some(0.9),Some(1.0),0.10,Some(0.5),Some(1.0)));
        assert!(eligible_checkpoint(None,None,0.10,None,None));
    }
    #[test]fn retention_panel_covers_older_and_newer_categories(){
        let mut plans=Vec::new();
        let dialogue=prepare("dialogue",&lesson("one").text).unwrap();
        let example=(0..7).map(|n|format!(
            "Room {n} stores a copper object and its associated location."
        )).collect::<Vec<_>>().join("\n");
        let text=prepare("text",&example).unwrap();
        for _ in 0..60{plans.push(dialogue.clone());}
        plans.insert(3,text.clone());
        plans.push(text);
        let indices=retention_plan_indices(&plans);
        assert!(indices.len()<=RETENTION_PLANS_PER_CHECK);
        assert!(indices.contains(&0));
        assert!(indices.contains(&(plans.len()-1)));
        assert!(indices.contains(&3));
    }
    #[test]fn selection_is_bounded_and_deterministic(){
        let corpus=(0..300).map(|i|format!(
            "Section {i} describes a distinct object and where it sits inside the classroom."
        )).collect::<Vec<_>>().join("\n");
        let plan=prepare("text",&corpus).unwrap();
        let ids=prior_validation_selection(&plan);
        assert!(ids.len()<=RETENTION_VALIDATION_LIMIT);
        assert!(ids.windows(2).all(|p|p[0]<p[1]));
    }
}