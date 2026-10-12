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
    pub training_text:String,
    pub memory_train:Vec<Vec<String>>,
    pub memory_validation:Vec<Vec<String>>,
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
fn conversation_sessions(text:&str)->Result<Vec<Vec<DialoguePair>>,String>{
    let mut sessions=Vec::new();
    let mut session=Vec::new();
    let mut pending:Option<String>=None;
    for (line_no,line) in text.lines().enumerate(){
        let line=line.trim_end_matches('\r').trim();
        if line.is_empty() || line=="---" {
            if pending.is_some(){return Err(format!("Missing N: answer before line {}",line_no+1));}
            if !session.is_empty(){
                if session.len()<2{return Err("Each conversation lesson needs at least two complete turns".into());}
                sessions.push(std::mem::take(&mut session));
            }
            continue;
        }
        if let Some(value)=line.strip_prefix("U: "){
            if pending.is_some(){return Err(format!("Missing N: response before line {}",line_no+1));}
            pending=Some(value.to_string());
        }else if let Some(value)=line.strip_prefix("N: "){
            let user=pending.take().ok_or_else(||format!("N: without U: on line {}",line_no+1))?;
            let pair=DialoguePair{user,assistant:value.to_string()};
            crate::dialogue::validate_pair(&pair)?;
            session.push(pair);
        }else{return Err(format!("Expected U: or N: on line {}",line_no+1));}
    }
    if pending.is_some(){return Err("Last U: has no N: reply".into());}
    if !session.is_empty(){
        if session.len()<2{return Err("Each conversation lesson needs at least two complete turns".into());}
        sessions.push(session);
    }
    if sessions.len()<5{return Err("Use at least five independent multi-turn sessions for held-out evaluation".into());}
    Ok(sessions)
}
fn memory_samples(sessions:&[Vec<DialoguePair>])->Result<(Vec<Example>,Vec<Vec<String>>),String>{
    let mut data=Vec::new();let mut histories=Vec::new();
    for session in sessions{
        let mut prior=Vec::<String>::new();
        for turn in session{
            let mut history=dialogue_prefix(&turn.user)?;
            for target in turn.assistant.bytes().chain(std::iter::once(b'\n')){
                data.push((history[history.len().saturating_sub(CONTEXT)..].to_vec(),target));
                histories.push(prior.iter().rev().take(12).rev().cloned().collect());
                history.push(target);
            }
            prior.push(format!("U: {}\nN: {}",turn.user,turn.assistant));
            if prior.len()>12{prior.remove(0);}
        }
    }
    Ok((data,histories))
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
            let training_text=train_pairs.iter()
                .map(|p|format!("{} {}\n",p.user,p.assistant)).collect::<String>();
            Ok(PreparedLessons{train,validation,train_lessons:train_pairs.len(),validation_lessons:valid_pairs.len(),
                mode:mode.into(),training_text,memory_train:Vec::new(),memory_validation:Vec::new()})
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
            Ok(PreparedLessons{train,validation,train_lessons:1,validation_lessons:1,
                mode:mode.into(),training_text:text[..split].to_string(),
                memory_train:Vec::new(),memory_validation:Vec::new()})
        },
        "conversation"=>{
            let mut sessions=conversation_sessions(text)?;
            // Whole conversations, not individual turns, are the validation unit.
            sessions.sort_by_key(|s|stable_hash(&s.iter().map(|p|p.user.as_str()).collect::<String>()));
            let n=(sessions.len()/5).max(1);
            let held=sessions.split_off(sessions.len()-n);
            let training_text=sessions.iter().flat_map(|session|session.iter())
                .map(|p|format!("{} {}\n",p.user,p.assistant)).collect::<String>();
            let (train,memory_train)=memory_samples(&sessions)?;
            let (validation,memory_validation)=memory_samples(&held)?;
            if train.len()>MAX_SAMPLES{return Err("Too many conversation training characters".into());}
            Ok(PreparedLessons{train,validation,
                train_lessons:sessions.len(),validation_lessons:held.len(),mode:mode.into(),
                training_text,memory_train,memory_validation})
        },
        _=>Err("Select dialogue, text, or conversation lessons".into())
    }
}
/// Training schedule; does not choose any response or change neural inference.
#[derive(Clone,Copy,Debug)]
pub struct ScheduleConfig {
    pub adaptive_learning_rate:bool,
    pub early_stopping:bool,
    pub patience:u32,
}
#[derive(Clone,Copy,Debug)]
pub struct ScheduleState {
    pub learning_rate:f32,
    pub stale_epochs:u32,
    config:ScheduleConfig,
}
impl ScheduleState {
    pub fn new(learning_rate:f32,config:ScheduleConfig)->Self{
        Self{learning_rate,stale_epochs:0,config}
    }
    /// Early stopping and rate adaptation are independent toggles.
    /// When enabled, the adaptive schedule may only REDUCE the learning rate.
    pub fn observe_validation(&mut self,improved:bool)->bool{
        if improved{self.stale_epochs=0;}else{self.stale_epochs=self.stale_epochs.saturating_add(1);}
        if self.config.early_stopping && self.stale_epochs>=self.config.patience{
            return true;
        }
        if self.config.adaptive_learning_rate && self.stale_epochs>0 && self.stale_epochs%2==0{
            self.learning_rate=(self.learning_rate*0.5).max(0.0003);
        }
        false
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
    fn conversation_memory_targets_are_aligned_and_split_by_session(){
        let mut text=String::new();
        for i in 0..6{
            text.push_str(&format!("U: hello {i}\nN: Hi.\nU: What did I say?\nN: You said hello {i}.\n\n"));
        }
        let p=prepare("conversation",&text).unwrap();
        assert_eq!(p.memory_train.len(),p.train.len());
        assert_eq!(p.memory_validation.len(),p.validation.len());
        assert!(p.memory_train.iter().any(|turns|!turns.is_empty()));
        assert!(p.memory_validation.iter().any(|turns|!turns.is_empty()));
        assert_eq!(p.train_lessons+p.validation_lessons,6);
    }
    #[test]
    fn early_stop_is_independent_of_adaptive_rate(){
        let mut state=ScheduleState::new(0.001,ScheduleConfig{adaptive_learning_rate:false,early_stopping:true,patience:3});
        assert!(!state.observe_validation(false));
        assert!(!state.observe_validation(false));
        assert_eq!(state.learning_rate,0.001);
        assert!(state.observe_validation(false));
        assert_eq!(state.learning_rate,0.001);
    }
    #[test]
    fn adaptive_rate_does_not_force_early_stop(){
        let mut state=ScheduleState::new(0.003,ScheduleConfig{adaptive_learning_rate:true,early_stopping:false,patience:3});
        for _ in 0..8{assert!(!state.observe_validation(false));}
        assert!(state.learning_rate<0.003);
        assert!(state.learning_rate>=0.0003);
    }
    #[test]
    fn both_disabled_keep_constant_rate_for_entire_training(){
        let mut state=ScheduleState::new(0.001,ScheduleConfig{adaptive_learning_rate:false,early_stopping:false,patience:3});
        for _ in 0..12{assert!(!state.observe_validation(false));}
        assert_eq!(state.learning_rate,0.001);
    }
    #[test]
    fn adaptive_never_increases_rate_even_after_improvement(){
        let mut state=ScheduleState::new(0.001,ScheduleConfig{adaptive_learning_rate:true,early_stopping:true,patience:6});
        state.observe_validation(false);state.observe_validation(false);
        let reduced=state.learning_rate;
        assert!(reduced<0.001);
        assert!(!state.observe_validation(true));
        assert_eq!(state.learning_rate,reduced);
        assert_eq!(state.stale_epochs,0);
    }
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
