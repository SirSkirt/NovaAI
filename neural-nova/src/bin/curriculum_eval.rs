//! Matched-start, equal-optimizer-step pilot of unified versus flat supervision.
//! All expected answers are external teacher evaluation data, never inference.
//! Run: cargo run --release --bin curriculum_eval -- 1
use neural_nova::hybrid::HybridBrain;
use neural_nova::trainer::{prepare,PreparedLessons,Example};
use std::time::Instant;

const BATCH:usize=8;

fn curriculum()->String{
    let mut s=String::new();
    s.push_str("[Dialogue Lesson]\nGreetings introduce conversations.\n[examples]\n");
    for (p,a) in [("hello","Hello!"),("hi","Hi!"),("good morning","Good morning!"),
        ("good evening","Good evening!"),("goodbye","Goodbye!"),("thank you","You are welcome.")]{
        s.push_str(&format!("{p}  {a}\n"));
    }
    s.push_str("[/examples]\n[/Dialogue Lesson]\n");
    s.push_str("[Understanding Lesson]\nA sentence describes an entity and one attribute.\n[examples]\n");
    for (p,a) in [
        ("Mira has a red ball. What color is the ball?","Red."),
        ("Tim has four apples. How many apples?","Four."),
        ("Nora opened the door. Who opened it?","Nora."),
        ("Eli has a blue bag. What color is the bag?","Blue."),
        ("Lena moved the chair. Who moved it?","Lena."),
        ("Omar has seven books. How many books?","Seven."),
    ]{s.push_str(&format!("{p}  {a}\n"));}
    s.push_str("[/examples]\n[/Understanding Lesson]\n");
    s.push_str("[Multi-Turn Conversation Lesson]\nEarlier turns can change what is relevant.\n[examples]\n");
    for (name,thing) in [("Mira","apples"),("Jon","pearls"),("Nora","flowers")]{
        s.push_str(&format!("[session]\nUser: Hello, Nova.\nNova: Hello!\nUser: My name is {name}.\nNova: Nice to meet you.\nUser: What is my name?\nNova: {name}.\nUser: I collect {thing}.\nNova: Interesting.\n[/session]\n"));
    }
    s.push_str("[/examples]\n[/Multi-Turn Conversation Lesson]\n");
    s.push_str("[Reasoning Lesson]\nA supported conclusion follows the evidence.\n[examples]\n");
    for (a,na,b,nb,object) in [("Ada",2,"Ben",4,"apples"),("Cara",7,"Dan",3,"keys"),
        ("Eli",1,"Fran",8,"books"),("Gus",6,"Hana",2,"pens")]{
        let winner=if na>nb{a}else{b};
        let loser=if na<nb{a}else{b};
        s.push_str(&format!("Premise: {a} has {na} {object}. {b} has {nb} {object}. Question: Who has more? Supported: {winner}. Contradicted: {loser}.\n"));
    }
    s.push_str("[/examples]\n[/Reasoning Lesson]\n");
    s
}
fn generate(brain:&HybridBrain,prefix:&[u8],memory:&[String],limit:usize)->Result<Vec<u8>,String>{
    let mut history=prefix.to_vec();let mut result=Vec::new();
    for _ in 0..limit{
        let p=brain.predict_with_memory(&history,256,memory)?;
        let token=p.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1))
            .ok_or("Missing probability distribution")?.0 as u8;
        if token==b'\n'{break;}
        result.push(token);history.push(token);
    }
    Ok(result)
}
fn report(brain:&HybridBrain,plan:&PreparedLessons,name:&str)->Result<(),String>{
    let mut right=[0usize;4];let mut count=[0usize;4];
    for (prefix,expected,memory,kind) in &plan.heldout_replies{
        let kind=*kind as usize;
        if count[kind]>=2{continue;}
        let result=generate(brain,prefix,memory,expected.len().min(48)+1)?;
        right[kind]+=usize::from(result==*expected);
        count[kind]+=1;
        println!("case\t{name}\t{kind}\texpected={:?}\tgenerated={:?}",
            String::from_utf8_lossy(expected),String::from_utf8_lossy(&result));
    }
    let p=brain.predict(b"U: hello\nN: ")?;
    let exact=(0..4).map(|k|if count[k]==0{"n/a".into()}
        else{format!("{}/{}",right[k],count[k])}).collect::<Vec<String>>().join(",");
    println!("summary\t{name}\tper-category-exact={exact}\tpY={:.6}\tpH={:.6}\tsteps={}",
        p[b'Y' as usize],p[b'H' as usize],brain.transformer.step);
    Ok(())
}
fn run()->Result<(),String>{
    let epochs=std::env::args().nth(1).unwrap_or_else(||"1".into())
        .parse::<usize>().map_err(|e|e.to_string())?;
    if !(1..=5).contains(&epochs){return Err("Choose 1–5 pilot epochs".into());}
    let plan=prepare("unified",&curriculum())?;
    let start=HybridBrain::new();
    let mut flat=start.clone();
    let mut multi=start;
    report(&multi,&plan,"starting")?;
    let started=Instant::now();
    for _ in 0..epochs{
        for kind in 0..=4{
            let ids=(0..plan.train.len()).filter(|&i|plan.train_kinds[i]==kind)
                .collect::<Vec<_>>();
            for chunk in ids.chunks(BATCH){
                let samples=chunk.iter().map(|&i|plan.train[i].clone())
                    .collect::<Vec<Example>>();
                flat.train_batch(&samples,0.0003)?;
                if kind==2 {
                    let with_memory=chunk.iter().map(|&i|(
                        plan.train[i].0.clone(),plan.train[i].1,
                        plan.memory_train[i].clone()
                    )).collect::<Vec<_>>();
                    multi.train_memory_batch(&with_memory,0.0003)?;
                }else{multi.train_batch(&samples,0.0003)?;}
            }
        }
        let contrast=plan.understanding_train.iter().take(1)
            .chain(plan.reasoning_train.iter().take(1));
        for objective in contrast{
            multi.train_preference_batch(&[objective.clone()],0.0003)?;
            // Equal optimizer-step control, using only flat supervision.
            flat.train_batch(&[plan.train[0].clone()],0.0003)?;
        }
    }
    println!("Training took {}ms; equal optimizer counts={} and {}",
        started.elapsed().as_millis(),flat.transformer.step,multi.transformer.step);
    if flat.transformer.step!=multi.transformer.step{
        return Err("A/B optimizer steps were not matched".into());
    }
    report(&flat,&plan,"flat-supervision")?;
    report(&multi,&plan,"unified-objectives")?;
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("curriculum pilot error: {e}");std::process::exit(1);}}
