//! Same-checkpoint teacher-curriculum A/B, distinct from character accuracy.
//! Compile/run: cargo run --release --bin curriculum_ab -- 6
//! Teacher-only answer keys are NOT passed to the model at inference.
//! This reports evidence, including failures, not a target to optimize for.
use neural_nova::hybrid::HybridBrain;
use neural_nova::trainer::prepare;
use neural_nova::unified::{
    DIALOGUE,UNDERSTANDING,CONVERSATION,REASONING,EXPLANATION,
};
use std::time::Instant;

const LESSON:&str=concat!(
    "[Dialogue Lesson]\nCommon greetings start conversations.\n[examples]\n",
    "hello  Hello!\nhi  Hi!\nthank you  You're welcome.\ngoodbye  Goodbye!\n",
    "[/examples]\n[/Dialogue Lesson]\n",
    "[Understanding Lesson]\nThe same question may have a different answer when its facts change.\n[examples]\n",
    "Mira has a red ball. What color is Mira's ball?  Red.\n",
    "Tim has a blue ball. What color is Tim's ball?  Blue.\n",
    "Nora has a green ball. What color is Nora's ball?  Green.\n",
    "Ari has a yellow ball. What color is Ari's ball?  Yellow.\n",
    "Jon has a black ball. What color is Jon's ball?  Black.\n",
    "Eve has a white ball. What color is Eve's ball?  White.\n",
    "[/examples]\n[/Understanding Lesson]\n",
    "[Multi-Turn Conversation Lesson]\nEarlier turns can contain relevant facts.\n[examples]\n",
    "[session]\nUser: My name is Mira.\nNova: Hello, Mira.\n",
    "User: What is my name?\nNova: Mira.\n[/session]\n",
    "[session]\nUser: My name is Tim.\nNova: Hello, Tim.\n",
    "User: What is my name?\nNova: Tim.\n[/session]\n",
    "[session]\nUser: My name is Nora.\nNova: Hello, Nora.\n",
    "User: What is my name?\nNova: Nora.\n[/session]\n",
    "[/examples]\n[/Multi-Turn Conversation Lesson]\n",
    "[Reasoning Lesson]\nUse the facts in a question to choose a supported conclusion.\n[examples]\n",
    "Premise: Alice has two apples. Ben has four apples. Question: Who has more apples? Supported: Ben. Contradicted: Alice.\n",
    "Premise: Eve has seven books. Dan has three books. Question: Who has more books? Supported: Eve. Contradicted: Dan.\n",
    "Premise: Jon has one pencil. Ari has five pencils. Question: Who has more pencils? Supported: Ari. Contradicted: Jon.\n",
    "Premise: Tim has six pens. Mira has two pens. Question: Who has more pens? Supported: Tim. Contradicted: Mira.\n",
    "[/examples]\n[/Reasoning Lesson]\n"
);

struct Probe { name:&'static str, prompt:&'static str, correct:&'static str, foil:&'static str }
const HOLDOUT:&[Probe]=&[
    Probe{name:"unseen-property",prompt:"U: Oscar has a purple ball. What color is Oscar's ball?\nN: ",
        correct:"Purple.",foil:"Green."},
    Probe{name:"swapped-quantity",prompt:"U: Theo has two cats. Rosa has six cats. Who has more cats?\nN: ",
        correct:"Rosa.",foil:"Theo."},
    Probe{name:"changed-situation",prompt:"U: Keira gave a red ball to Theo. What color is the ball Theo received?\nN: ",
        correct:"Red.",foil:"Blue."},
];

fn normalized_answer_score(brain:&HybridBrain,prompt:&str,answer:&str,
    no_dense:bool,no_exchange:bool)->Result<f32,String>{
    let mut history=prompt.as_bytes().to_vec();
    let mut logp=0.0f32;
    for next in answer.bytes().chain(std::iter::once(b'\n')){
        let probabilities=brain.predict_ablation_with_memory(
            &history,256,&[],no_dense,no_exchange)?;
        logp+=probabilities[next as usize].max(1.0e-30).ln();
        history.push(next);
        if history.len()>512{history.drain(..history.len()-512);}
    }
    Ok(logp/(answer.len()+1) as f32)
}
fn generate(brain:&HybridBrain,prompt:&str)->Result<String,String>{
    let mut context=prompt.as_bytes().to_vec();
    let mut result=String::new();
    for _ in 0..14{
        let p=brain.predict_window(&context,256)?;
        let next=p.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1))
            .ok_or("Neural model returned empty probabilities")?.0 as u8;
        if next==b'\n'{break;}
        result.push(next as char);context.push(next);
    }
    Ok(result)
}
fn evaluate(brain:&HybridBrain,label:&str,mode:&str,
    dense_off:bool,exchange_off:bool)->Result<(),String>{
    for probe in HOLDOUT{
        let output=if dense_off||exchange_off{"(ablation: not generated)".into()}
            else{generate(brain,probe.prompt)?};
        let score_correct=normalized_answer_score(
            brain,probe.prompt,probe.correct,dense_off,exchange_off)?;
        let score_foil=normalized_answer_score(
            brain,probe.prompt,probe.foil,dense_off,exchange_off)?;
        println!("{label}\t{mode}\t{}\t{:?}\t{}\t{:.5}",
            probe.name,output,output==probe.correct,score_correct-score_foil);
    }
    Ok(())
}
fn run()->Result<(),String>{
    let steps=std::env::args().nth(1).unwrap_or_else(||"6".into())
        .parse::<usize>().map_err(|e|e.to_string())?;
    if !(1..=64).contains(&steps){return Err("Choose 1..=64 update steps".into());}
    let lesson=prepare("unified",LESSON)?;
    // All four objective categories exist and heldout answers never enter
    // either training arm or token vocabulary expansion.
    for kind in 0..=REASONING{
        if lesson.category_train[kind as usize]==0{
            return Err(format!("Curriculum has no training for objective {kind}"));
        }
    }
    if !lesson.validation.is_empty() && HOLDOUT.iter().any(|probe|
        lesson.training_text.contains(probe.prompt)){
        return Err("Unseen evaluation case leaked into training data".into());
    }

    let mut initial=HybridBrain::new();
    initial.learn_token_units(&lesson.training_text);
    let starting_step=initial.transformer.step;
    let mut legacy=initial.clone();
    let mut unified=initial;
    let mut candidates=vec![Vec::new();5];
    for (i,&kind) in lesson.train_kinds.iter().enumerate(){
        candidates[kind as usize].push(i);
    }
    let learning_rate=0.0003;
    let started=Instant::now();
    for i in 0..steps {
        // Comparable budget: one joint neural optimizer step in each arm.
        let k=if i%2==0{DIALOGUE as usize}else{EXPLANATION as usize};
        let ids=if candidates[k].is_empty(){&candidates[DIALOGUE as usize]}else{&candidates[k]};
        let index=ids[i%ids.len()];
        legacy.train_batch(&[lesson.train[index].clone()],learning_rate)?;

        match i%6 {
            3 if !lesson.reasoning_train.is_empty()=>{
                unified.train_preference_batch(&[
                    lesson.reasoning_train[i%lesson.reasoning_train.len()].clone()
                ],learning_rate)?;
            },
            5 if !lesson.understanding_train.is_empty()=>{
                unified.train_preference_batch(&[
                    lesson.understanding_train[i%lesson.understanding_train.len()].clone()
                ],learning_rate*0.5)?;
            },
            _=>{
                let category=match i%6 {
                    0=>DIALOGUE,1=>UNDERSTANDING,2=>CONVERSATION,
                    4=>REASONING,_=>DIALOGUE
                } as usize;
                let indices=if candidates[category].is_empty(){
                    &candidates[DIALOGUE as usize]
                }else{&candidates[category]};
                let index=indices[i%indices.len()];
                let (history,next)=&lesson.train[index];
                if category==CONVERSATION as usize {
                    unified.train_memory_batch(&[(
                        history.clone(),*next,lesson.memory_train[index].clone()
                    )],learning_rate)?;
                } else {
                    unified.train_batch(&[(history.clone(),*next)],learning_rate)?;
                }
            }
        }
    }
    if legacy.transformer.step!=unified.transformer.step ||
        unified.transformer.step!=starting_step+steps as u64 {
        return Err("A/B arms did not receive the same optimizer-step budget".into());
    }
    println!("teacher_arms\t{}\tsteps\t{} ms",steps,started.elapsed().as_millis());
    println!("arm\tpath\tprobe\tgenerated\texact\tanswer_margin");
    let before=unified.transformer.weights.clone();
    let optimizer=unified.transformer.momentum.clone();
    evaluate(&legacy,"dialogue-language-only","full",false,false)?;
    evaluate(&unified,"multiobjective","full",false,false)?;
    evaluate(&unified,"multiobjective","dense-off",true,false)?;
    evaluate(&unified,"multiobjective","exchange-off",false,true)?;
    assert_eq!(before,unified.transformer.weights,
        "Heldout evaluation must never train or alter neural weights");
    assert_eq!(optimizer,unified.transformer.momentum,
        "Heldout evaluation must never alter optimizer history");
    println!("NOTE: a few steps on a new tiny brain cannot demonstrate general reasoning; compare repeated heldout tests on real retained checkpoints.");
    Ok(())
}
fn main(){
    if let Err(error)=run(){
        eprintln!("Curriculum A/B failed: {error}");
        std::process::exit(1);
    }
}
