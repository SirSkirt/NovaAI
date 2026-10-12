//! Teacher-controlled, persistent rehearsal curricula. Never imported by
//! neural inference. A model only learns these samples via normal joint loss.
use neural_nova::trainer::{prepare,PreparedLessons};
use serde::{Serialize,Deserialize};
use std::{fs,path::Path};

pub const MAX_ARCHIVED_LESSONS:usize=3;
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
    if raw.len()>250_000{return Err("Rehearsal bank exceeds size limit".into());}
    let bank:RehearsalBank=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
    if bank.lessons.len()>MAX_ARCHIVED_LESSONS ||
      bank.lessons.iter().any(|x|x.text.len()>50_000 || x.name.len()>80){
        return Err("Rehearsal bank contains invalid lesson records".into());
    }
    Ok(bank)
}
pub fn write(path:&Path,bank:&RehearsalBank)->Result<(),String>{
    let raw=serde_json::to_vec(bank).map_err(|e|e.to_string())?;
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
    bank.lessons.push(record);
    if bank.lessons.len()>MAX_ARCHIVED_LESSONS{
        bank.lessons.drain(..bank.lessons.len()-MAX_ARCHIVED_LESSONS);
    }
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
#[cfg(test)]
mod tests{
    use super::*;
    fn lesson(s:&str)->LessonRecord{LessonRecord{
        mode:"dialogue".into(),name:s.into(),
        text:format!("hello\tHi {s}\nbye\tBye {s}\ncat\tAnimal {s}\ndog\tPet {s}\nbird\tBird {s}\n")
    }}
    #[test]fn bounded_archive_keeps_last_three_and_must_match_weights(){
        let mut bank=RehearsalBank::default();
        for i in 0..5{append(&mut bank,lesson(&i.to_string()),format!("fp-{i}")).unwrap();}
        assert_eq!(bank.lessons.len(),3);
        assert_eq!(matched(bank.clone(),"fp-4").lessons.len(),3);
        assert!(matched(bank,"other-checkpoint").lessons.is_empty());
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
    #[test]fn selection_is_bounded_and_deterministic(){
        let plan=prepare("text",&"Hello there ".repeat(250)).unwrap();
        let ids=prior_validation_selection(&plan);
        assert!(ids.len()<=RETENTION_VALIDATION_LIMIT);
        assert!(ids.windows(2).all(|p|p[0]<p[1]));
    }
}