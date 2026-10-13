//! Teacher-controlled, persistent rehearsal curricula. Never imported by
//! neural inference. A model only learns these samples via normal joint loss.
use neural_nova::trainer::{prepare,PreparedLessons};
use serde::{Serialize,Deserialize};
use std::{fs,path::Path};

pub const MAX_ARCHIVED_LESSONS:usize=128;
pub const RETENTION_VALIDATION_LIMIT:usize=256;
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LessonRecord{
    pub mode:String,pub text:String,pub name:String,
}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RehearsalBank{
    pub model_fingerprint:String,pub lessons:Vec<LessonRecord>,
}
pub fn read(path:&Path)->Result<RehearsalBank,String>{
    if !path.exists(){return Ok(RehearsalBank::default());}
    let raw=fs::read(path).map_err(|e|e.to_string())?;
    if raw.len()>8_000_000{return Err("Rehearsal bank exceeds 8MB; export or archive it before adding more lessons".into());}
    let bank:RehearsalBank=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
    if bank.lessons.len()>MAX_ARCHIVED_LESSONS ||
      bank.lessons.iter().any(|x|x.text.len()>50_000 || x.name.len()>80){
        return Err("Rehearsal bank contains invalid lesson records".into());
    }
    Ok(bank)
}
pub fn write(path:&Path,bank:&RehearsalBank)->Result<(),String>{
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
    else {RehearsalBank{model_fingerprint:fingerprint.to_string(),lessons:Vec::new()}}
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
/// The replay set contains only previous TRAIN splits. Held-out material is
/// reserved exclusively for checkpoint evaluation.
pub fn previous_plans(bank:&RehearsalBank)->Result<Vec<PreparedLessons>,String>{
    bank.lessons.iter().map(|v|prepare(&v.mode,&v.text)).collect()
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