//! Training-only curriculum assembly and deterministic evaluation splits.
//! This module never chooses words or affects inference. One joint hybrid loss trains Nova.
use crate::dialogue::{dialogue_prefix,parse_dialogues,DialoguePair};
// Longer raw contexts permit the input tokenizer to compress multiple words
// before the Transformer selects its final 256 token positions.
const RAW_WINDOW:usize=512;
use std::collections::BTreeMap;
use std::ops::Range;

pub type Example = (Vec<u8>,u8);
pub type ReasoningExample=(Vec<u8>,Vec<u8>,Vec<u8>);
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
    pub reasoning_train:Vec<ReasoningExample>,
    pub reasoning_validation:Vec<ReasoningExample>,
    pub understanding_train:Vec<ReasoningExample>,
    pub understanding_validation:Vec<ReasoningExample>,
    /// Alignment with train/validation examples; empty for explicit legacy modes.
    pub train_kinds:Vec<u8>,pub validation_kinds:Vec<u8>,
    /// Counts of complete independent units by objective (0 dialogue,
    /// 1 understanding, 2 multi-turn, 3 reasoning, 4 explanatory language).
    pub category_train:[usize;5],pub category_validation:[usize;5],
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
            result.push((history[history.len().saturating_sub(RAW_WINDOW)..].to_vec(),target));
            history.push(target);
        }
    }
    Ok(result)
}
fn corpus_examples(corpus:&[u8],indices:Range<usize>,lower_bound:usize)->Vec<Example>{
    indices.map(|i|(corpus[i.saturating_sub(RAW_WINDOW).max(lower_bound)..i].to_vec(),corpus[i])).collect()
}
/// Make *independent* text sections before splitting. Prefer paragraphs
/// separated by a blank line. With fewer than five, fall back to complete
/// sentences/individual lines (including single-line prose paragraphs).
/// A window must never cross an independently held-out section boundary.
fn text_sections(text:&str)->Vec<String>{
    let mut paragraphs=Vec::new();
    let mut paragraph=String::new();
    for line in text.lines(){
        if line.trim().is_empty(){
            if !paragraph.trim().is_empty(){
                paragraphs.push(paragraph.trim().to_string());paragraph.clear();
            }
        }else{
            if !paragraph.is_empty(){paragraph.push('\n');}
            paragraph.push_str(line.trim());
        }
    }
    if !paragraph.trim().is_empty(){paragraphs.push(paragraph.trim().to_string());}
    if paragraphs.len()>=5{return paragraphs;}
    // A sentence ends on terminal punctuation followed by whitespace, or
    // at a line ending. This splitter has no learned language assumptions.
    let mut sentences=Vec::new();
    let bytes=text.as_bytes();
    let mut start=0usize;
    for i in 0..bytes.len(){
        let b=bytes[i];
        let followed_by_space=i+1==bytes.len()||bytes[i+1].is_ascii_whitespace();
        if b==b'\n'||((b==b'.'||b==b'!'||b==b'?')&&followed_by_space){
            let stop=if b==b'\n'{i}else{i+1};
            let section=text[start..stop].trim();
            if section.len()>=2{sentences.push(section.to_string());}
            start=i+1;
        }
    }
    if start<text.len(){
        let section=text[start..].trim();
        if section.len()>=2{sentences.push(section.to_string());}
    }
    sentences
}
/// Split *whole* sections by stable hash, keeping case/whitespace variants
/// and verbatim duplicates together. File order cannot decide membership.
fn prepare_text_sections(text:&str)->Result<PreparedLessons,String>{
    let sections=text_sections(text);
    let mut groups:BTreeMap<String,Vec<String>>=BTreeMap::new();
    for section in sections{
        let canonical=section.split_ascii_whitespace()
            .map(str::to_ascii_lowercase).collect::<Vec<_>>().join(" ");
        groups.entry(canonical).or_default().push(section);
    }
    if groups.len()<5{
        return Err("Text lessons need at least five distinct paragraphs (blank-line separated), sentences, or lines for a meaningful held-out split. Add varied sections instead of repeating one passage.".into());
    }
    let mut ordered=groups.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|a,b|stable_hash(&a.0).cmp(&stable_hash(&b.0)).then(a.0.cmp(&b.0)));
    let held=(ordered.len()/5).max(1);
    let boundary=ordered.len()-held;
    let mut train_units=Vec::new();
    let mut valid_units=Vec::new();
    for (index,(_,variants)) in ordered.into_iter().enumerate(){
        if index<boundary{train_units.extend(variants);}
        else{valid_units.extend(variants);}
    }
    let examples=|units:&[String]|units.iter().flat_map(|unit|{
        let bytes=unit.as_bytes();
        corpus_examples(bytes,1..bytes.len(),0)
    }).collect::<Vec<_>>();
    let train=examples(&train_units);
    let validation=examples(&valid_units);
    if train.is_empty()||validation.is_empty(){
        return Err("Text lesson needs longer distinct sentences or paragraphs in both training and held-out sets".into());
    }
    if train.len()>MAX_SAMPLES{
        return Err(format!("Too many Text training targets (max {MAX_SAMPLES}); split into smaller lessons"));
    }
    Ok(PreparedLessons{
        train,validation,train_lessons:train_units.len(),
        validation_lessons:valid_units.len(),mode:"text".into(),
        training_text:train_units.join("\n\n"),
        memory_train:Vec::new(),memory_validation:Vec::new(),
        reasoning_train:Vec::new(),reasoning_validation:Vec::new(),
        understanding_train:Vec::new(),understanding_validation:Vec::new(),train_kinds:Vec::new(),validation_kinds:Vec::new(),
        category_train:[0;5],category_validation:[0;5],
    })
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
                data.push((history[history.len().saturating_sub(RAW_WINDOW)..].to_vec(),target));
                histories.push(prior.iter().skip(prior.len().saturating_sub(12)).cloned().collect());
                history.push(target);
            }
            prior.push(format!("U: {}\nN: {}",turn.user,turn.assistant));
            if prior.len()>12{prior.remove(0);}
        }
    }
    Ok((data,histories))
}
/// Teacher data for reasoning: premise TAB question TAB supported conclusion
/// TAB contradicted conclusion. Only the inference-free curriculum builder
/// uses the rejected answer. It never enters the user's chat prompt.
fn prepare_reasoning(text:&str)->Result<PreparedLessons,String>{
    let mut groups:BTreeMap<String,Vec<(DialoguePair,ReasoningExample)>>=BTreeMap::new();
    for (index,line) in text.lines().filter(|s|!s.trim().is_empty()).enumerate(){
        let fields=line.split('\t').collect::<Vec<_>>();
        if fields.len()!=4 {return Err(format!(
            "Reasoning line {} must have PREMISE TAB QUESTION TAB CORRECT TAB INCORRECT",index+1));}
        let [fact,question,correct,wrong]=[fields[0].trim(),fields[1].trim(),
            fields[2].trim(),fields[3].trim()];
        if correct.is_empty()||wrong.is_empty()||correct==wrong||
            correct.len()>96||wrong.len()>96{
            return Err(format!("Reasoning line {} needs different, nonempty conclusions ≤96 characters",index+1));
        }
        let prompt=format!("{fact} {question}");
        if prompt.len()>180||fact.is_empty()||question.is_empty(){
            return Err(format!("Reasoning premise and question on line {} must fit in 180 characters",index+1));
        }
        let prefix=dialogue_prefix(&prompt)?;
        let pair=DialoguePair{user:prompt.clone(),assistant:correct.to_string()};
        groups.entry(prompt_group(&prompt)).or_default().push((pair,
            (prefix,correct.as_bytes().to_vec(),wrong.as_bytes().to_vec())));
    }
    if groups.len()<5{return Err("Reasoning requires at least five distinct premise/question groups for held-out testing".into());}
    let mut sorted=groups.into_iter().collect::<Vec<_>>();
    sorted.sort_by(|a,b|stable_hash(&a.0).cmp(&stable_hash(&b.0)).then(a.0.cmp(&b.0)));
    let cutoff=sorted.len()-(sorted.len()/5).max(1);
    let mut train_pairs=Vec::new();let mut validation_pairs=Vec::new();
    let mut reasoning_train=Vec::new();let mut reasoning_validation=Vec::new();
    for (i,(_,group)) in sorted.into_iter().enumerate(){
        for (pair,compare) in group{
            if i<cutoff{train_pairs.push(pair);reasoning_train.push(compare);}
            else{validation_pairs.push(pair);reasoning_validation.push(compare);}
        }
    }
    let train=dialogue_examples(&train_pairs)?;
    let validation=dialogue_examples(&validation_pairs)?;
    if train.len()>MAX_SAMPLES||reasoning_train.len()>1000{
        return Err("Reasoning lesson too large; split into smaller groups".into());
    }
    let training_text=train_pairs.iter().map(|p|format!("{} {}\n",p.user,p.assistant))
        .collect::<String>();
    Ok(PreparedLessons{train,validation,mode:"reasoning".into(),
        train_lessons:train_pairs.len(),validation_lessons:validation_pairs.len(),
        training_text,memory_train:Vec::new(),memory_validation:Vec::new(),
        reasoning_train,reasoning_validation,
        understanding_train:Vec::new(),understanding_validation:Vec::new(),
        train_kinds:Vec::new(),validation_kinds:Vec::new(),
        category_train:[0;5],category_validation:[0;5]})
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
                mode:mode.into(),training_text,memory_train:Vec::new(),memory_validation:Vec::new(),
                reasoning_train:Vec::new(),reasoning_validation:Vec::new(),
        understanding_train:Vec::new(),understanding_validation:Vec::new(),train_kinds:Vec::new(),validation_kinds:Vec::new(),
        category_train:[0;5],category_validation:[0;5],})
        },
        "reasoning"=>prepare_reasoning(text),
        "unified"=>crate::unified::prepare(text),
        "text"=>{
            if text.len()<120||text.len()>MAX_CORPUS_BYTES{
                return Err(format!("Text lessons require 120–{MAX_CORPUS_BYTES} ASCII bytes per training run"));
            }
            prepare_text_sections(text)
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
                training_text,memory_train,memory_validation,
                reasoning_train:Vec::new(),reasoning_validation:Vec::new(),
        understanding_train:Vec::new(),understanding_validation:Vec::new(),train_kinds:Vec::new(),validation_kinds:Vec::new(),
        category_train:[0;5],category_validation:[0;5],})
        },
        _=>Err("Select dialogue, text, conversation, or reasoning lessons".into())
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
    adaptive_floor:f32,
    config:ScheduleConfig,
}
impl ScheduleState {
    pub fn new(learning_rate:f32,config:ScheduleConfig)->Self{
        // Older rates retain the historical 0.0003 floor. Lower starting
        // rates must NEVER be pushed upward by the adaptive schedule.
        let adaptive_floor=if learning_rate>0.0003{0.0003}
            else{(learning_rate*0.1).max(0.00001)};
        Self{learning_rate,stale_epochs:0,adaptive_floor,config}
    }
    /// Early stopping and rate adaptation are independent toggles.
    /// When enabled, the adaptive schedule may only REDUCE the learning rate.
    pub fn observe_validation(&mut self,improved:bool)->bool{
        if improved{self.stale_epochs=0;}else{self.stale_epochs=self.stale_epochs.saturating_add(1);}
        if self.config.early_stopping && self.stale_epochs>=self.config.patience{
            return true;
        }
        if self.config.adaptive_learning_rate && self.stale_epochs>0 && self.stale_epochs%2==0{
            self.learning_rate=(self.learning_rate*0.5).max(self.adaptive_floor);
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
    fn low_learning_rate_adaptive_schedule_never_increases_it(){
        for rate in [0.0001,0.0003,0.001,0.003]{
            let mut schedule=ScheduleState::new(rate,ScheduleConfig{
                adaptive_learning_rate:true,early_stopping:false,patience:6
            });
            let mut last=rate;
            for _ in 0..20{
                assert!(!schedule.observe_validation(false));
                assert!(schedule.learning_rate<=last+1e-9,
                    "adaptive rate increased from {last} to {}",schedule.learning_rate);
                assert!(schedule.learning_rate>0.0);
                last=schedule.learning_rate;
            }
            if rate==0.0001{assert!(last<rate);}
        }
    }
    #[test]
    fn reasoning_pair_split_does_not_train_on_rejected_answers(){
        let data=(0..9).map(|i|format!(
            "The box {i} is red.\tWhat color is box {i}?\tRed.\tBlue."
        )).collect::<Vec<_>>().join("\n");
        let p=prepare("reasoning",&data).unwrap();
        assert_eq!(p.reasoning_train.len()+p.reasoning_validation.len(),9);
        assert!(p.reasoning_train.iter().all(|(_,good,bad)|good==b"Red."&&bad==b"Blue."));
        assert_eq!(p.train_lessons+p.validation_lessons,9);
        assert!(!p.training_text.contains("Blue."));
    }
    #[test]
    fn text_validation_uses_whole_paragraphs_and_no_cross_split_context(){
        let data=(0..12).map(|i|format!(
            "Distinct section {i}: Mira opens the blue box and counts the candles.\nThe reader checks the order."
        )).collect::<Vec<_>>().join("\n\n");
        let p=prepare("text",&data).unwrap();
        assert_eq!(p.train_lessons+p.validation_lessons,12);
        assert!(p.train_lessons>=9 && p.validation_lessons>=2);
        assert!(p.validation.iter().all(|(context,_)|!context.is_empty()));
        assert!(p.validation.iter().all(|(context,_)|context.len()<RAW_WINDOW));
        // Whole paragraph boundaries mean each target's context starts inside
        // the same held-out paragraph. Nothing can extend from train sections.
        let training=String::from_utf8(p.training_text.clone().into_bytes()).unwrap();
        for i in 0..12{
            let unique=format!("Distinct section {i}:");
            let in_train=training.contains(&unique);
            let in_valid=p.validation.iter().any(|(history,_)|
                String::from_utf8_lossy(history).contains(&unique));
            assert!(!(in_train && in_valid));
        }
    }
    #[test]
    fn text_section_split_is_stable_under_reordering(){
        let lines=(0..14).map(|i|format!(
            "Line {i} describes a different object and its place in the room."
        )).collect::<Vec<_>>();
        let a=prepare("text",&lines.join("\n")).unwrap();
        let b=prepare("text",&lines.iter().rev().cloned().collect::<Vec<_>>().join("\n")).unwrap();
        assert_eq!(a.train,a.train.clone());
        let mut av=a.validation.clone();let mut bv=b.validation.clone();
        av.sort();bv.sort();assert_eq!(av,bv);
        let mut at=a.train.clone();let mut bt=b.train.clone();
        at.sort();bt.sort();assert_eq!(at,bt);
    }
    #[test]
    fn text_duplicate_sections_do_not_cross_holdout(){
        let parts=(0..10).map(|i|format!(
            "Repeated sentence {i} is different enough to remain independent."
        )).collect::<Vec<_>>();
        let mut all=parts.clone();
        all.extend(parts.iter().map(|x|x.to_uppercase()));
        let p=prepare("text",&all.join("\n")).unwrap();
        let canonical_in_train=p.training_text.to_ascii_lowercase();
        for row in &parts{
            let prefix=row.to_ascii_lowercase();
            if canonical_in_train.contains(&prefix){
                assert!(!p.validation.iter().any(|(history,_)|
                    String::from_utf8_lossy(history).to_ascii_lowercase().contains(&prefix)));
            }
        }
    }
    #[test]
    fn text_rejects_single_repeated_block_instead_of_fake_validation(){
        assert!(prepare("text",&"abcdefghij ".repeat(40)).is_err());
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
