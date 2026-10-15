//! Bicameral teacher-only tests: no answer key enters neural inference.
use neural_nova::nova2::{Nova2,Focus};
use crate::nova2_lab::Nova2State;
use crate::test_lab::{TestArchive,TestDefinition,TestOptions,TestReport,TestCase,
    QuestionResult,Attempt,parse_cases,grade,validate_options};
use serde_json;
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use tauri::{AppHandle,State,Emitter,Manager};

#[derive(Default)]
struct Inner{archive:Mutex<TestArchive>,pause:AtomicBool,stop:AtomicBool}
#[derive(Clone,Default)]
pub struct Nova2TestState{inner:Arc<Inner>}
const TEST_FILE:&str="neural-nova-bicameral-teacher-tests.json";
fn file(app:&AppHandle)->Result<std::path::PathBuf,String>{
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    Ok(dir.join(TEST_FILE))
}
fn persist(app:&AppHandle,state:&Nova2TestState)->Result<(),String>{
    let guard=state.inner.archive.lock().map_err(|_|"Teacher archive lock failed")?;
    std::fs::write(file(app)?,serde_json::to_vec_pretty(&*guard)
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())
}
fn now()->u64{
    use std::time::{SystemTime,UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}
fn uid(prefix:&str)->String{
    use std::time::{SystemTime,UNIX_EPOCH};
    let micro=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros();
    format!("{prefix}-{micro}")
}
#[tauri::command]
pub fn nova2_test_boot(app:AppHandle,state:State<'_,Nova2TestState>)->Result<TestArchive,String>{
    let path=file(&app)?;
    if path.exists(){
        let v:TestArchive=serde_json::from_slice(&std::fs::read(&path)
            .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        *state.inner.archive.lock().map_err(|_|"Archive lock failed")?=v;
    }
    nova2_test_archive(state)
}
#[tauri::command]
pub fn nova2_test_archive(state:State<'_,Nova2TestState>)->Result<TestArchive,String>{
    Ok(state.inner.archive.lock().map_err(|_|"Archive lock failed")?.clone())
}
#[tauri::command]
pub fn nova2_test_save(app:AppHandle,state:State<'_,Nova2TestState>,
    name:String,questions_text:String,answers_text:String,
    unseen_questions_text:String,unseen_answers_text:String)->Result<TestDefinition,String>{
    if name.trim().is_empty()||name.len()>80{return Err("Name must be 1-80 characters".into());}
    let cases=parse_cases(&questions_text,&answers_text)?;
    let unseen=if unseen_questions_text.trim().is_empty()&&unseen_answers_text.trim().is_empty(){
        Vec::new()
    }else{parse_cases(&unseen_questions_text,&unseen_answers_text)?};
    let norm=|s:&str|s.split_ascii_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
    let keys=cases.iter().map(|c|norm(&c.question))
        .collect::<std::collections::HashSet<_>>();
    if unseen.iter().any(|c|keys.contains(&norm(&c.question))){
        return Err("Unseen test must not duplicate training/correction questions".into());
    }
    let item=TestDefinition{id:uid("nova2-test"),name,cases,unseen,created_at:now()};
    {
        let mut a=state.inner.archive.lock().map_err(|_|"Archive lock failed")?;
        if a.definitions.len()>=30{return Err("At most 30 test definitions".into());}
        a.definitions.push(item.clone());
    }
    persist(&app,&state)?;Ok(item)
}
#[tauri::command]
pub fn nova2_test_delete(app:AppHandle,state:State<'_,Nova2TestState>,
    test_id:String)->Result<(),String>{
    let mut a=state.inner.archive.lock().map_err(|_|"Archive lock failed")?;
    let before=a.definitions.len();a.definitions.retain(|d|d.id!=test_id);
    if a.definitions.len()==before{return Err("Unknown test".into());}
    drop(a);persist(&app,&state)
}
#[tauri::command]
pub fn nova2_test_pause(state:State<'_,Nova2TestState>)->Result<(),String>{
    state.inner.pause.store(true,Ordering::Release);Ok(())
}
#[tauri::command]
pub fn nova2_test_resume(state:State<'_,Nova2TestState>)->Result<(),String>{
    state.inner.pause.store(false,Ordering::Release);Ok(())
}
#[tauri::command]
pub fn nova2_test_stop(state:State<'_,Nova2TestState>)->Result<(),String>{
    state.inner.stop.store(true,Ordering::Release);
    state.inner.pause.store(false,Ordering::Release);Ok(())
}
fn go(state:&Nova2TestState)->bool{
    while state.inner.pause.load(Ordering::Acquire)&&!state.inner.stop.load(Ordering::Acquire){
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !state.inner.stop.load(Ordering::Acquire)
}
fn answer(brain:&Nova2,c:&TestCase,options:&TestOptions)->Result<Attempt,String>{
    // The answer key is not used in the generation call.
    let prompt=format!("U: {}\nN: ",c.question);
    let output=brain.generate_cached(&prompt,options.max_chars)?
        .trim_end_matches('\n').to_string();
    Ok(Attempt{automatic:grade(&output,&c.expected).into(),response:output,
        teacher_grade:None,correction_loss:None,gradient_steps:0,
        model_step:brain.step,sampling:"greedy / KV cache".into()})
}
fn publish(app:&AppHandle,report:&TestReport){let _=app.emit("nova2-test-progress",report);}
fn update(state:&Nova2TestState,app:&AppHandle,report:&TestReport){
    if let Ok(mut a)=state.inner.archive.lock(){
        if let Some(r)=a.reports.iter_mut().find(|r|r.id==report.id){*r=report.clone();}
    }
    let _=persist(app,state);publish(app,report);
}
#[tauri::command]
pub fn nova2_test_start(app:AppHandle,brain:State<'_,Nova2State>,
    state:State<'_,Nova2TestState>,test_id:String,options:TestOptions)->Result<String,String>{
    validate_options(&options)?;
    let definition=state.inner.archive.lock().map_err(|_|"Archive lock failed")?
        .definitions.iter().find(|d|d.id==test_id).cloned().ok_or("Test not found")?;
    if options.generalization&&definition.unseen.is_empty(){
        return Err("Import untouched generalization questions, or disable that option".into());
    }
    brain.reserve_external()?;
    let original=match brain.snapshot(){
        Ok(b)=>b,Err(e)=>{brain.release_external();return Err(e);}
    };
    state.inner.stop.store(false,Ordering::Release);
    state.inner.pause.store(false,Ordering::Release);
    let id=uid("nova2-run");
    let mut r=TestReport{id:id.clone(),test_id:definition.id.clone(),
        name:definition.name.clone(),started_at:now(),status:"running".into(),
        config:options.clone(),checkpoint:format!("NNOVA2 step {}",original.step),
        questions:definition.cases.iter().map(|c|QuestionResult{
            question:c.question.clone(),expected:c.expected.clone(),
            teacher_answer:None,attempts:vec![],retention:None}).collect(),
        unseen:definition.unseen.iter().map(|c|QuestionResult{
            question:c.question.clone(),expected:c.expected.clone(),
            teacher_answer:None,attempts:vec![],retention:None}).collect(),
        baseline_unseen_correct:None,final_unseen_correct:None,
        initial_correct:0,final_correct:0,corrected:0,still_incorrect:0,
        training_updates:0,start_step:original.step,end_step:original.step,error:None};
    {
        state.inner.archive.lock().map_err(|_|"Archive lock failed")?
            .reports.push(r.clone());
    }
    persist(&app,&state)?;
    let st=state.inner.clone();let live=brain.inner_clone_for_test();
    std::thread::spawn(move||{
        let tests=Nova2TestState{inner:st};let mut model=original;
        let result=(||->Result<(),String>{
            // Baseline all questions and all generalization cases BEFORE any gradient update.
            for (i,c) in definition.cases.iter().enumerate(){
                if !go(&tests){return Ok(());}
                r.questions[i].attempts.push(answer(&model,c,&options)?);
                r.recalculate();update(&tests,&app,&r);
            }
            if options.generalization{
                for (i,c) in definition.unseen.iter().enumerate(){
                    if !go(&tests){return Ok(());}
                    r.unseen[i].attempts.push(answer(&model,c,&options)?);
                }
                r.baseline_unseen_correct=Some(r.unseen.iter().filter(|q|
                    q.attempts.first().is_some_and(|a|a.automatic=="correct")).count());
                update(&tests,&app,&r);
            }
            if options.corrections{
                for (i,c) in definition.cases.iter().enumerate(){
                    if !go(&tests){return Ok(());}
                    if r.questions[i].attempts.first().is_some_and(|a|a.automatic=="correct"){continue;}
                    for _ in 1..options.attempts{
                        if !go(&tests){return Ok(());}
                        let lesson=format!("U: {}\nN: {}",c.question,c.expected);
                        let loss=model.train_text(&lesson,options.learning_rate,Focus::Joint)?;
                        let mut next=answer(&model,c,&options)?;
                        next.correction_loss=Some(loss);next.gradient_steps=1;
                        r.questions[i].attempts.push(next);
                        r.training_updates+=1;r.end_step=model.step;
                        r.recalculate();update(&tests,&app,&r);
                        if r.questions[i].attempts.last().is_some_and(|a|a.automatic=="correct"){break;}
                    }
                }
            }
            if options.retention{
                for (i,c) in definition.cases.iter().enumerate(){
                    if !go(&tests){return Ok(());}
                    r.questions[i].retention=Some(answer(&model,c,&options)?);
                }
            }
            if options.generalization{
                let mut n=0;
                for (i,c) in definition.unseen.iter().enumerate(){
                    if !go(&tests){return Ok(());}
                    let a=answer(&model,c,&options)?;if a.automatic=="correct"{n+=1;}
                    r.unseen[i].attempts.push(a);
                }
                r.final_unseen_correct=Some(n);
            }
            if r.training_updates>0 {
                if !go(&tests){return Ok(());}
                live.install_external(&app,model.clone())?;
            }
            Ok(())
        })();
        r.status=if tests.inner.stop.load(Ordering::Acquire){"stopped".into()}
            else if result.is_ok(){"completed".into()}else{"error".into()};
        if let Err(e)=result{r.error=Some(e);}
        r.end_step=model.step;r.recalculate();update(&tests,&app,&r);
        live.release_external();
        let _=app.emit("nova2-test-done",&r);
        let _=app.emit("nova2-done",());
    });
    Ok(id)
}
#[tauri::command]
pub fn nova2_test_review(app:AppHandle,state:State<'_,Nova2TestState>,
    run_id:String,index:usize,attempt:usize,teacher_grade:String,
    corrected_answer:Option<String>)->Result<(),String>{
    if !["correct","incorrect","needsReview"].contains(&teacher_grade.as_str()){
        return Err("Invalid teacher grade".into());
    }
    {
        let mut a=state.inner.archive.lock().map_err(|_|"Archive lock failed")?;
        let r=a.reports.iter_mut().find(|r|r.id==run_id).ok_or("Report not found")?;
        let q=r.questions.get_mut(index).ok_or("Question not found")?;
        q.attempts.get_mut(attempt).ok_or("Attempt not found")?.teacher_grade=Some(teacher_grade);
        if let Some(ans)=corrected_answer{if !ans.is_empty()&&ans.len()<=256&&ans.is_ascii(){
            q.teacher_answer=Some(ans);
        }}
        r.recalculate();
    }
    persist(&app,&state)
}
