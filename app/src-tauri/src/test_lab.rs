//! Teacher-side testing and assessment. Never imported by neural inference.
//! No answer-key value is passed to the model's generation function.
use serde::{Serialize,Deserialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TestCase{pub question:String,pub expected:String}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TestDefinition{
    pub id:String,pub name:String,pub cases:Vec<TestCase>,pub unseen:Vec<TestCase>,
    pub created_at:u64
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TestOptions{
    pub attempts:usize,pub corrections:bool,pub learning_rate:f32,
    pub max_chars:usize,pub context_chars:usize,pub deterministic:bool,
    pub seed:Option<u64>,pub retention:bool,pub generalization:bool,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Attempt{
    pub response:String,pub automatic:String,pub teacher_grade:Option<String>,
    pub correction_loss:Option<f32>,pub gradient_steps:u64,
    pub model_step:u64,pub sampling:String,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct QuestionResult{
    pub question:String,pub expected:String,pub teacher_answer:Option<String>,
    pub attempts:Vec<Attempt>,pub retention:Option<Attempt>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TestReport{
    pub id:String,pub test_id:String,pub name:String,pub started_at:u64,
    pub status:String,pub config:TestOptions,
    #[serde(default)] pub checkpoint:String,
    pub questions:Vec<QuestionResult>,pub unseen:Vec<QuestionResult>,
    pub baseline_unseen_correct:Option<usize>,pub final_unseen_correct:Option<usize>,
    pub initial_correct:usize,pub final_correct:usize,pub corrected:usize,
    pub still_incorrect:usize,pub training_updates:u64,pub start_step:u64,pub end_step:u64,
    pub error:Option<String>,
}
#[derive(Clone,Debug,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct TestArchive{
    pub definitions:Vec<TestDefinition>,
    pub reports:Vec<TestReport>,
}
/// Strictly conservative: only surrounding whitespace may be ignored.
/// Case/punctuation-only differences need a human, never assumed correct.
pub fn grade(generated:&str,expected:&str)->&'static str{
    if generated.trim()==expected.trim(){"correct"}
    else if generated.trim().eq_ignore_ascii_case(expected.trim()) ||
        generated.trim().trim_end_matches(&['.','!','?'][..]) ==
        expected.trim().trim_end_matches(&['.','!','?'][..]){"needsReview"}
    else{"incorrect"}
}
fn valid_line(value:&str,limit:usize)->bool{
    !value.is_empty()&&value.len()<=limit &&
    value.bytes().all(|b|(32..=126).contains(&b))
}
fn entries(text:&str)->Vec<String>{
    text.lines().map(|s|s.trim_end_matches('\r').trim().to_string())
        .filter(|s|!s.is_empty()&&!s.starts_with('#')).collect()
}
pub fn parse_cases(questions:&str,answer_key:&str)->Result<Vec<TestCase>,String>{
    if questions.len()>60_000||answer_key.len()>60_000{
        return Err("Test files are limited to 60 KB each".into());
    }
    let question_lines=entries(questions);
    let answer_lines=entries(answer_key);
    if question_lines.is_empty()||question_lines.len()>100{
        return Err("Provide between 1 and 100 test questions".into());
    }
    let combined=answer_lines.is_empty()&&question_lines.iter().all(|s|s.contains('\t'));
    if !combined&&answer_lines.len()!=question_lines.len(){
        return Err(format!("Question count ({}) and answer-key count ({}) do not match",
            question_lines.len(),answer_lines.len()));
    }
    let mut cases=Vec::new();
    for (i,line) in question_lines.iter().enumerate(){
        let (q,a)=if combined{line.split_once('\t').unwrap()}
            else{(line.as_str(),answer_lines[i].as_str())};
        if !valid_line(q,180)||!valid_line(a,256){
            return Err(format!("Line {}: question (1–180) and answer (1–256) must be printable ASCII",i+1));
        }
        if q.contains('\t')||a.contains('\t'){
            return Err(format!("Line {}: extra TAB or invalid separator",i+1));
        }
        cases.push(TestCase{question:q.into(),expected:a.into()});
    }
    Ok(cases)
}
pub fn validate_options(o:&TestOptions)->Result<(),String>{
    if !(1..=5).contains(&o.attempts){return Err("Choose 1–5 attempts".into());}
    if !o.learning_rate.is_finite()||!(0.0003..=0.006).contains(&o.learning_rate){
        return Err("Corrective learning rate must be 0.0003–0.006".into());
    }
    if !(1..=256).contains(&o.max_chars){return Err("Reply limit must be 1–256".into());}
    if o.context_chars!=128&&o.context_chars!=256{
        return Err("Context must be 128 or 256 tokens".into());
    }
    Ok(())
}
impl TestReport {
    pub fn recalculate(&mut self){
        self.initial_correct=self.questions.iter().filter(|q|
            q.attempts.first().is_some_and(|a|a.automatic=="correct")).count();
        self.final_correct=self.questions.iter().filter(|q|
            q.attempts.last().is_some_and(|a|a.teacher_grade.as_deref().unwrap_or(&a.automatic)=="correct")).count();
        self.corrected=self.questions.iter().filter(|q|
            q.attempts.first().is_some_and(|a|a.automatic!="correct")&&
            q.attempts.iter().any(|a|a.gradient_steps>0)&&
            q.attempts.last().is_some_and(|a|a.teacher_grade.as_deref().unwrap_or(&a.automatic)=="correct")).count();
        self.still_incorrect=self.questions.iter().filter(|q|
            q.attempts.last().is_some_and(|a|a.teacher_grade.as_deref().unwrap_or(&a.automatic)=="incorrect")).count();
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn independent_files_and_combined_tab_import(){
        let a=parse_cases("hello\nwhat is your name?","Hi!\nMy name is Nova.").unwrap();
        let b=parse_cases("hello\tHi!\nwhat is your name?\tMy name is Nova.","").unwrap();
        assert_eq!(a[0].question,b[0].question);
        assert_eq!(a[1].expected,b[1].expected);
        assert!(parse_cases("hi\nbye","Hello!").is_err());
    }
    #[test]fn no_semantic_assumptions_or_silent_case_equivalence(){
        assert_eq!(grade(" My name is Nova. ","My name is Nova."),"correct");
        assert_eq!(grade("my name is nova.","My name is Nova."),"needsReview");
        assert_eq!(grade("My name is Nova","My name is Nova."),"needsReview");
        assert_eq!(grade("I am a talking tree.","My name is Nova."),"incorrect");
    }
    #[test]fn validate_training_budget_and_settings(){
        let mut o=TestOptions{attempts:3,corrections:true,learning_rate:0.001,max_chars:128,
            context_chars:128,deterministic:true,seed:None,retention:true,generalization:false};
        assert!(validate_options(&o).is_ok());o.attempts=100;
        assert!(validate_options(&o).is_err());
    }
}
