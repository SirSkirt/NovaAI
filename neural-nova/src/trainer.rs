//! Training-only curriculum assembly and deterministic evaluation splits.
//! This module never chooses words or affects inference. One joint hybrid loss trains Nova.
use crate::dialogue::{dialogue_prefix,parse_dialogues,DialoguePair};
use crate::transformer::CONTEXT;
use std::collections::BTreeMap;
use std::ops::Range;

pub type Example = (Vec<u8>,u8);
pub const MAX_SAMPLES:usize=50_000;
pub const MAX_CORPUS_BYTES:usize=50_000;

#[derive(Clone,Debug)]
pub struct PreparedLessons{
    pub train:Vec<Example>,
    pub validation:Vec<Example>,
    pub train_lessons:usize,
    pub validation_lessons:usize,
    pub mode:String,
}

/// Canonicalize prompt *only to avoid train/validation contamination*.
/// Nova receives untouched teacher data and no canonicalizer is used for replies.
fn prompt_group(text:&str)->String {
    text.trim().trim_end_matches(|c:char| c=='?'||c=='!'||c=='.')
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>().join(" ")
}
fn stable_hash(text:&str)->u64{
    let mut hash=0xcbf29ce484222325u64;
    for c in text.bytes(){hash^=c as u64;hash=hash.wrapping_mul(0x100000001b3);}
    hash
}
pub fn dialogue_examples(pairs:&[DialoguePair])->Result<Vec<Example>,String>{
    let mut result=Vec::new();
    for pair in pairs{
        let mut history=dialogue_prefix(&pair.user)?;
        for target in pair.assistant.bytes().chain(std::iter::once(b'\n')){
            result.push((history[history.len().saturating_sub(CONTEXT)..].to_vec(),target));
            history.push(target);
        }
    }
    Ok(result)
}
fn corpus_examples(corpus:&[u8],indices:Range<usize>,lower_bound:usize)->Vec<Example>{
    indices.map(|i|(corpus[i.saturating_sub(CONTEXT).max(lower_bound)..i].to_vec(),corpus[i])).collect()
}
pub fn prepare(mode:&str,text:&str)->Result<PreparedLessons,String>{
    if !text.is_ascii(){return Err("Current tokenizer supports ASCII only".into());}
    match mode{
        "dialogue"=>{
            let pairs=parse_dialogues(text)?;
            let mut groups:BTreeMap<String,Vec<DialoguePair>>=BTreeMap::new();
            for pair in pairs {
                groups.entry(prompt_group(&pair.user)).or_default().push(pair);
            }
            if groups.len()<5{
                return Err("Provide at least five distinct questions after case/punctuation normalization".into());
            }
            let mut by_hash:Vec<_>=groups.into_iter().collect();
            by_hash.sort_by(|a,b|stable_hash(&a.0).cmp(&stable_hash(&b.0)).then(a.0.cmp(&b.0)));
            // Hold out whole question groups, including differently punctuated/cased
            // variants and all their answers. Reordering the lesson file cannot
            // move a question from the train set into validation.
            let held=(by_hash.len()/5).max(1);
            let split=by_hash.len()-held;
            let mut train_pairs=Vec::new();
            let mut valid_pairs=Vec::new();
            for (index,(_,group)) in by_hash.into_iter().enumerate(){
                if index<split{train_pairs.extend(group);}else{valid_pairs.extend(group);}
            }
            let train=dialogue_examples(&train_pairs)?;
            let validation=dialogue_examples(&valid_pairs)?;
            if train.len()>MAX_SAMPLES{
                return Err(format!("Too many dialogue reply characters for one CPU run (max {MAX_SAMPLES} training tokens). Split the lesson set into sessions."));
            }
            Ok(PreparedLessons{train,validation,train_lessons:train_pairs.len(),validation_lessons:valid_pairs.len(),mode:mode.into()})
        },
        "text"=>{
            let corpus=text.as_bytes();
            if corpus.len()<120||corpus.len()>MAX_CORPUS_BYTES{
                return Err(format!("Text lessons require 120–{MAX_CORPUS_BYTES} ASCII bytes per training run"));
            }
            let split=(corpus.len()*9/10).max(2);
            let train=corpus_examples(corpus,1..split,0);
            // Separate the validation context at the boundary. It must not
            // include any bytes from the training portion of this corpus.
            let validation=corpus_examples(corpus,(split+1)..corpus.len(),split);
            if validation.is_empty(){return Err("No held-out text tokens".into());}
            Ok(PreparedLessons{train,validation,train_lessons:1,validation_lessons:1,mode:mode.into()})
        },
        _=>Err("Select dialogue or text lessons".into())
    }
}
/// Epoch one introduces short contexts first, later epochs use unbiased shuffling.
/// No hard-coded responses, target transformations, or alterations to inference.
pub fn training_order(samples:&[Example],epoch:u32,mode:&str)->Vec<usize>{
    let mut order:Vec<usize>=(0..samples.len()).collect();
    if epoch==1 && mode=="dialogue"{
        order.sort_by_key(|&i|(samples[i].0.len(),i));
        return order;
    }
    let mut seed=(epoch as u64).wrapping_mul(0x9e3779b97f4a7c15)^0x4e4f5641;
    for i in (1..order.len()).rev(){
        seed^=seed>>12;seed^=seed<<25;seed^=seed>>27;
        order.swap(i,((seed.wrapping_mul(2685821657736338717))%(i as u64+1)) as usize);
    }
    order
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn split_is_stable_on_reordered_curriculum_and_preserves_question_groups(){
        let original="HELLO?\tHi!\nhello\tHey!\nwhat is rain?\tWater.\nwhat is snow\tIce.\nwhat is a cat\tAnimal.\nwhat is a tree\tPlant.\nbye\tGoodbye.\n";
        let reverse=original.lines().rev().collect::<Vec<_>>().join("\n");
        let a=prepare("dialogue",original).unwrap();
        let b=prepare("dialogue",&reverse).unwrap();
        fn first_targets(items:&[Example])->Vec<Vec<u8>>{
            let mut out=items.iter().filter(|(history,_)|history.ends_with(b"\nN: "))
                .map(|(history,_)|history.clone()).collect::<Vec<_>>();
            out.sort();out
        }
        assert_eq!(first_targets(&a.train),first_targets(&b.train));
        assert_eq!(first_targets(&a.validation),first_targets(&b.validation));
        let train_text=a.train.iter().map(|(x,_)|x.as_slice()).collect::<Vec<_>>();
        let valid_text=a.validation.iter().map(|(x,_)|x.as_slice()).collect::<Vec<_>>();
        assert!(!(train_text.iter().any(|x|x.ends_with(b"U: HELLO?\nN: ")) &&
                  valid_text.iter().any(|x|x.ends_with(b"U: hello\nN: "))));
        assert!(!a.validation.is_empty());
    }
    #[test]
    fn text_validation_cannot_see_training_context(){
        let data="abcdefghij ".repeat(40);
        let prepared=prepare("text",&data).unwrap();
        assert!(prepared.validation[0].0.len()>0);
        assert_eq!(prepared.validation[0].0,vec![data.as_bytes()[data.len()*9/10]]);
        assert!(prepared.validation.iter().all(|(history,_)|!history.is_empty()));
    }
    #[test]
    fn first_dialogue_epoch_is_short_to_long_later_epochs_are_shuffled(){
        let p=prepare("dialogue","a\tX.\nbb\tY.\nccc\tZ.\ndddd\tW.\neeeee\tQ.\n").unwrap();
        let a=training_order(&p.train,1,"dialogue");
        assert!(a.windows(2).all(|w|p.train[w[0]].0.len()<=p.train[w[1]].0.len()));
        assert_eq!(training_order(&p.train,3,"dialogue"),training_order(&p.train,3,"dialogue"));
        assert_ne!(a,training_order(&p.train,3,"dialogue"));
    }
}
