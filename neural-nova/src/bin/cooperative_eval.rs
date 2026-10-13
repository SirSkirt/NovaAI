//! Reproducible held-out evaluation of Neural Nova's cooperative brain.
//! The expected answers exist ONLY in this external evaluator/teacher; inference
//! calls the same neural brain with no symbolic answering or answer lookup.
//! Run: cargo run --release --bin cooperative_eval -- 4
use neural_nova::hybrid::HybridBrain;
use std::time::Instant;

struct Case { label:&'static str, prompt:&'static str, good:&'static str, bad:&'static str }

const TRAIN:[Case;4]=[
    Case{label:"train-a",prompt:"U: Ada has 2 apples. Ben has 5 apples. Who has more apples?\nN: ",good:"Ben",bad:"Ada"},
    Case{label:"train-b",prompt:"U: Cara has 7 pears. Dan has 3 pears. Who has more pears?\nN: ",good:"Cara",bad:"Dan"},
    Case{label:"train-c",prompt:"U: Eli has 4 pens. Fran has 8 pens. Who has more pens?\nN: ",good:"Fran",bad:"Eli"},
    Case{label:"train-d",prompt:"U: Gus has 1 key. Hana has 9 keys. Who has more keys?\nN: ",good:"Hana",bad:"Gus"},
];
const UNSEEN:[Case;5]=[
    Case{label:"new-entities",prompt:"U: Mina has 3 books. Oscar has 6 books. Who has more books?\nN: ",good:"Oscar",bad:"Mina"},
    Case{label:"new-objects",prompt:"U: Tim has 8 cards. Susie has 2 cards. Who has more cards?\nN: ",good:"Tim",bad:"Susie"},
    Case{label:"reversed-relation",prompt:"U: Oscar has 2 books. Mina has 6 books. Who has more books?\nN: ",good:"Mina",bad:"Oscar"},
    Case{label:"irrelevant-fact",prompt:"U: Mina has 3 books. Oscar has 6 books. The sky is blue. Who has more books?\nN: ",good:"Oscar",bad:"Mina"},
    Case{label:"transfer-event",prompt:"U: Nia has 2 stones. Omar has 4 stones. Nia gives Omar 1 stone. Who has more now?\nN: ",good:"Omar",bad:"Nia"},
];

/// Full neural generation: greedy decoding uses only the model's distribution.
fn generate(brain:&HybridBrain,prompt:&str,no_dense:bool,no_exchange:bool)->Result<String,String>{
    let mut history=prompt.as_bytes().to_vec();
    let mut generated=String::new();
    for _ in 0..12 {
        let probs=brain.predict_ablation_with_memory(&history,256,&[],no_dense,no_exchange)?;
        let (next,_)=probs.iter().enumerate().max_by(|a,b|
            a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or("Empty model distribution")?;
        if next==b'\n' as usize {break;}
        generated.push(next as u8 as char);
        history.push(next as u8);
    }
    Ok(generated.trim().to_string())
}
/// Normalized log preference uses neural likelihoods, not a scripted solver.
fn score(brain:&HybridBrain,prompt:&str,answer:&str,no_dense:bool,no_exchange:bool)
    ->Result<f32,String>{
    let mut prefix=prompt.as_bytes().to_vec();
    let mut logp=0.0;
    for b in answer.bytes().chain(std::iter::once(b'\n')){
        let probs=brain.predict_ablation_with_memory(&prefix,256,&[],no_dense,no_exchange)?;
        logp+=probs[b as usize].max(1.0e-30).ln();
        prefix.push(b);
    }
    Ok(logp/(answer.len()+1) as f32)
}
fn report(brain:&HybridBrain,stage:&str,case:&Case,mode:&str,
    no_dense:bool,no_exchange:bool)->Result<(bool,f32),String>{
    let started=Instant::now();
    let generated=generate(brain,case.prompt,no_dense,no_exchange)?;
    let good=score(brain,case.prompt,case.good,no_dense,no_exchange)?;
    let bad=score(brain,case.prompt,case.bad,no_dense,no_exchange)?;
    let success=generated.eq_ignore_ascii_case(case.good);
    println!("{stage}\t{}\t{mode}\t{:?}\t{}\t{:.6}\t{}",
        case.label,generated,success,good-bad,started.elapsed().as_millis());
    Ok((success,good-bad))
}
fn run()->Result<(),String>{
    let steps=std::env::args().nth(1).unwrap_or_else(||"4".into())
        .parse::<usize>().map_err(|e|format!("Invalid step count: {e}"))?;
    if !(1..=128).contains(&steps){return Err("Steps must be 1..=128".into());}
    let mut brain=HybridBrain::new();
    // Keep a tiny older ability and check its retention through reasoning lessons.
    let greeting=(b"U: Hello!\nN: ".to_vec(),b'H');
    for _ in 0..4{brain.train_batch(&[greeting.clone()],0.001)?;}
    let retention_before=brain.evaluate(&[greeting.clone()])?.0;
    println!("stage\tcase\tmode\tgenerated\texact\tpreference_margin\tlatency_ms");
    for c in &UNSEEN{report(&brain,"before",c,"full",false,false)?;}
    let before=brain.clone();  // pure read-only pretraining snapshot.
    let start=Instant::now();
    for i in 0..steps {
        let c=&TRAIN[i%TRAIN.len()];
        brain.train_preference_batch(&[(
            c.prompt.as_bytes().to_vec(),c.good.as_bytes().to_vec(),
            c.bad.as_bytes().to_vec())],0.0003)?;
    }
    eprintln!("Training {} comparison steps took {} ms",
        steps,start.elapsed().as_millis());
    let evaluation_weights=brain.transformer.weights.clone();
    let evaluation_optimizer=brain.transformer.momentum.clone();
    let mut hits=[0usize;3];
    for c in &UNSEEN {
        for (idx,(mode,no_dense,no_exchange)) in [
            ("full",false,false),("no-dense",true,false),("no-exchange",false,true)
        ].iter().copied().enumerate() {
            let (correct,_) = report(&brain,"after",c,mode,no_dense,no_exchange)?;
            if correct{hits[idx]+=1;}
        }
    }
    let retained=brain.evaluate(&[greeting])?.0;
    println!("SUMMARY\t{} steps; heldout exact full={} no-dense={} no-exchange={}/{}; old-greeting-loss {:.4}->{:.4}",
        steps,hits[0],hits[1],hits[2],UNSEEN.len(),retention_before,retained);
    assert_eq!(brain.transformer.weights,evaluation_weights,
        "evaluation or ablation must not change learned weights");
    assert_eq!(brain.transformer.momentum,evaluation_optimizer,
        "evaluation or ablation must not change optimizer history");
    let _=before.predict(b"U: Hello!\\nN: ")?;
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
