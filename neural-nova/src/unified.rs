//! Strict, teacher-only unified lesson reader. Blocks set learning objectives;
//! they NEVER become hardcoded responses or run during Nova's inference.
use crate::dialogue::{dialogue_prefix,validate_pair,DialoguePair};
use crate::trainer::{PreparedLessons,ReasoningExample,MAX_SAMPLES};
use std::collections::{BTreeMap,BTreeSet};

pub const DIALOGUE:u8=0;
pub const UNDERSTANDING:u8=1;
pub const CONVERSATION:u8=2;
pub const REASONING:u8=3;
pub const EXPLANATION:u8=4;
const RAW_WINDOW:usize=512;

#[derive(Clone,Debug)]
struct Block {kind:u8, explanation:String, examples:Vec<String>}
#[derive(Clone,Debug)]
struct Unit {kind:u8, group:String, block:usize, turns:Vec<DialoguePair>,
    comparison:Option<ReasoningExample>}

fn canonical(text:&str)->String {
    text.trim().trim_end_matches(['?','!','.'])
        .split_ascii_whitespace().map(str::to_ascii_lowercase)
        .collect::<Vec<_>>().join(" ")
}
fn hash(text:&str)->u64 {
    let mut h=0xcbf29ce484222325u64;
    for b in text.bytes(){h^=b as u64;h=h.wrapping_mul(0x100000001b3);}
    h
}
fn kind_name(kind:u8)->&'static str {
    match kind { DIALOGUE=>"Dialogue Lesson",UNDERSTANDING=>"Understanding Lesson",
        CONVERSATION=>"Multi-Turn Conversation Lesson",REASONING=>"Reasoning Lesson",
        _=>"unknown" }
}
fn marker_kind(text:&str)->Option<u8>{
    (0..=3).find(|&kind|text==format!("[{}]",kind_name(kind)))
}
fn parse_blocks(text:&str)->Result<Vec<Block>,String>{
    if !text.is_ascii(){return Err("Unified lessons require ASCII text".into());}
    if text.len()>50_000 {return Err("Unified TXT exceeds the 50,000-byte lesson limit".into());}
    let mut blocks=Vec::<Block>::new();
    let mut active:Option<Block>=None;
    let mut examples=false;
    let mut seen_examples=false;
    let mut ended_examples=false;
    for (i,raw) in text.lines().enumerate() {
        let line=raw.trim_end_matches('\r');
        let trimmed=line.trim();
        let line_no=i+1;
        if let Some(kind)=marker_kind(trimmed){
            if active.is_some(){return Err(format!("Line {line_no}: nested lesson block"));}
            active=Some(Block{kind,explanation:String::new(),examples:Vec::new()});
            examples=false;seen_examples=false;ended_examples=false;
            continue;
        }
        if let Some(block)=active.as_mut(){
            if trimmed==format!("[/{}]",kind_name(block.kind)){
                if examples || !ended_examples {
                    return Err(format!("Line {line_no}: close [examples] before closing {}",kind_name(block.kind)));
                }
                if block.explanation.trim().is_empty(){
                    return Err(format!("Line {line_no}: {} needs instructional text",kind_name(block.kind)));
                }
                if block.examples.iter().all(|s|s.trim().is_empty()){
                    return Err(format!("Line {line_no}: {} has no examples",kind_name(block.kind)));
                }
                blocks.push(active.take().unwrap());
                continue;
            }
            if trimmed=="[examples]"{
                if examples || seen_examples || ended_examples{
                    return Err(format!("Line {line_no}: duplicate or nested [examples]"));
                }
                seen_examples=true;examples=true;continue;
            }
            if trimmed=="[/examples]"{
                if !examples{return Err(format!("Line {line_no}: unexpected [/examples]"));}
                examples=false;ended_examples=true;continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') &&
                trimmed!="[session]" && trimmed!="[/session]" {
                return Err(format!("Line {line_no}: unknown or mismatched lesson marker {trimmed}"));
            }
            if examples {
                block.examples.push(trimmed.to_string());
            } else if ended_examples {
                if !trimmed.is_empty(){return Err(format!("Line {line_no}: content after [/examples]"));}
            } else {
                if trimmed=="[session]" || trimmed=="[/session]" {
                    return Err(format!("Line {line_no}: sessions belong inside [examples]"));
                }
                if !trimmed.is_empty(){
                    if !block.explanation.is_empty(){block.explanation.push('\n');}
                    block.explanation.push_str(trimmed);
                }
            }
        }else if !trimmed.is_empty(){
            return Err(format!("Line {line_no}: content outside a lesson block: {trimmed}"));
        }
    }
    if let Some(block)=active{return Err(format!("Unclosed [{}]",kind_name(block.kind)));}
    if blocks.is_empty(){return Err("No [Dialogue Lesson], [Understanding Lesson], [Multi-Turn Conversation Lesson] or [Reasoning Lesson] blocks found".into());}
    Ok(blocks)
}
fn parse_two_spaces(line:&str,kind:u8)->Result<DialoguePair,String>{
    if line.contains('\t'){return Err(format!("{} examples use TWO ASCII spaces, never TAB",kind_name(kind)));}
    let (question,answer)=line.split_once("  ").ok_or_else(||format!(
        "{} example requires exactly two ASCII spaces between input and answer: {line}",kind_name(kind)))?;
    if question.ends_with(' ')||answer.starts_with(' ') ||
        question.is_empty()||answer.is_empty() {
        return Err("Expected one exactly-two-space separator and nonempty input/answer".into());
    }
    let pair=DialoguePair{user:question.to_string(),assistant:answer.to_string()};
    validate_pair(&pair)?;
    Ok(pair)
}
fn parse_sessions(lines:&[String])->Result<Vec<Vec<DialoguePair>>,String>{
    let mut sessions=Vec::new();
    let mut current:Option<Vec<DialoguePair>>=None;
    let mut pending:Option<String>=None;
    for line in lines {
        if line.is_empty(){continue;}
        if line=="[session]"{
            if current.is_some(){return Err("Nested [session] is not allowed".into());}
            current=Some(Vec::new());continue;
        }
        if line=="[/session]"{
            let completed=current.take().ok_or("[/session] without [session]")?;
            if pending.is_some(){return Err("Session ends before Nova replies".into());}
            if completed.len()<2{return Err("Each [session] needs at least two User/Nova exchanges".into());}
            sessions.push(completed);continue;
        }
        let session=current.as_mut().ok_or("Conversation content outside [session]")?;
        if let Some(user)=line.strip_prefix("User: "){
            if pending.is_some(){return Err("Two User turns without Nova reply".into());}
            pending=Some(user.to_string());
        }else if let Some(reply)=line.strip_prefix("Nova: "){
            let user=pending.take().ok_or("Nova reply without preceding User turn")?;
            let pair=DialoguePair{user,assistant:reply.to_string()};
            validate_pair(&pair)?;
            session.push(pair);
        }else{return Err(format!("Expected User: or Nova: inside session; got {line}"));}
    }
    if current.is_some(){return Err("Unclosed [session]".into());}
    if sessions.is_empty(){return Err("Multi-turn [examples] needs a [session]".into());}
    Ok(sessions)
}
/// Teacher-only contrast selection. Similar input wording with DIFFERENT
/// teacher-provided answers makes context use relevant: the model must learn
/// which situation supports which answer, not just prefer a familiar answer.
fn lexical_features(text:&str)->BTreeSet<String>{
    text.split(|c:char| !c.is_ascii_alphanumeric())
        .filter(|w|!w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
fn context_pair_similarity(a:&BTreeSet<String>,b:&BTreeSet<String>)->i64{
    let shared=a.intersection(b).count() as i64;
    let unmatched=a.symmetric_difference(b).count() as i64;
    8*shared-unmatched
}
/// All candidates are from ONE split (training OR independent validation).
/// Never synthesize a correct answer, infer a fact, or use the other split.
fn select_context_negative(
    source:usize,pairs:&[&DialoguePair],features:&[BTreeSet<String>],
)->Option<usize>{
    const MAX_CANDIDATES:usize=192; // Bounded teacher preprocessing on Android.
    let n=pairs.len();
    if n<=1{return None;}
    let step=((n-1)+MAX_CANDIDATES-1)/MAX_CANDIDATES;
    (1..n).step_by(step.max(1))
        .filter_map(|offset|{
            let i=(source+offset)%n;
            if pairs[i].assistant==pairs[source].assistant{return None;}
            let overlap=context_pair_similarity(&features[source],&features[i]);
            let len_diff=(pairs[source].assistant.len() as i64-
                pairs[i].assistant.len() as i64).abs();
            Some((i,overlap*4-len_diff))
        })
        .max_by(|(a,score_a),(b,score_b)|
            score_a.cmp(score_b).then_with(||b.cmp(a)))
        .map(|(index,_)|index)
}
fn parse_comparison(line:&str)->Result<(DialoguePair,ReasoningExample),String>{
    let a=line.strip_prefix("Premise: ").ok_or("Reasoning example must start with Premise: ")?;
    let (fact,rest)=a.split_once(" Question: ").ok_or("Reasoning example requires Question: ")?;
    let (question,rest)=rest.split_once(" Supported: ").ok_or("Reasoning example requires Supported: ")?;
    let (right,wrong)=rest.split_once(" Contradicted: ").ok_or("Reasoning example requires Contradicted: ")?;
    for value in [fact,question,right,wrong]{
        if value.trim().is_empty(){return Err("Reasoning fields must not be empty".into());}
    }
    if right==wrong || right.len()>96 || wrong.len()>96 || !right.is_ascii() || !wrong.is_ascii(){
        return Err("Supported and contradicted conclusions must differ, be ASCII and <=96 bytes".into());
    }
    let prompt=format!("{fact} {question}");
    let pair=DialoguePair{user:prompt,assistant:right.to_string()};
    validate_pair(&pair)?;
    let prefix=dialogue_prefix(&pair.user)?;
    Ok((pair,(prefix,right.as_bytes().to_vec(),wrong.as_bytes().to_vec())))
}
fn units(blocks:&[Block])->Result<Vec<Unit>,String>{
    let mut result=Vec::new();
    let mut previously_seen:BTreeMap<(u8,String),String>=BTreeMap::new();
    for (idx,block) in blocks.iter().enumerate(){
        let mut add_pair=|pair:DialoguePair,comparison:Option<ReasoningExample>|->Result<(),String>{
            let key=canonical(&pair.user);
            let prior=(block.kind,key.clone());
            if let Some(other)=previously_seen.get(&prior){
                if other!=&pair.assistant{return Err(format!(
                    "Conflicting expected answers for one {} input: {}",kind_name(block.kind),pair.user));}
                return Ok(());
            }
            previously_seen.insert(prior,pair.assistant.clone());
            result.push(Unit{kind:block.kind,group:key,block:idx,turns:vec![pair],comparison});
            Ok(())
        };
        match block.kind{
            DIALOGUE|UNDERSTANDING=>{
                for line in block.examples.iter().filter(|s|!s.is_empty()){
                    add_pair(parse_two_spaces(line,block.kind)?,None)?;
                }
            },
            REASONING=>{
                for line in block.examples.iter().filter(|s|!s.is_empty()){
                    let (pair,comparison)=parse_comparison(line)?;
                    add_pair(pair,Some(comparison))?;
                }
            },
            CONVERSATION=>{
                let sessions=parse_sessions(&block.examples)?;
                for turns in sessions{
                    let group=format!("session:{}",canonical(
                        &turns.iter().map(|p|format!("{} => {}",p.user,p.assistant))
                            .collect::<Vec<_>>().join(" | ")));
                    result.push(Unit{kind:block.kind,group,block:idx,turns,comparison:None});
                }
            },
            _=>unreachable!()
        }
    }
    Ok(result)
}
fn supervised(pair:&DialoguePair,prior:&[String],
    prepared:&mut PreparedLessons,validation:bool,kind:u8)->Result<(),String>{
    let mut history=dialogue_prefix(&pair.user)?;
    if validation{
        prepared.heldout_replies.push((
            history.clone(),pair.assistant.as_bytes().to_vec(),prior.to_vec(),kind));
    }
    for target in pair.assistant.bytes().chain(std::iter::once(b'\n')){
        let example=(history[history.len().saturating_sub(RAW_WINDOW)..].to_vec(),target);
        if validation{
            prepared.validation.push(example);
            prepared.memory_validation.push(prior.to_vec());
            prepared.validation_kinds.push(kind);
        }else{
            prepared.train.push(example);
            prepared.memory_train.push(prior.to_vec());
            prepared.train_kinds.push(kind);
        }
        history.push(target);
    }
    Ok(())
}
pub fn prepare(text:&str)->Result<PreparedLessons,String>{
    let blocks=parse_blocks(text)?;
    let units=units(&blocks)?;
    if units.len()<2{return Err("Unified lessons need at least two independent examples/sessions for a held-out evaluation".into());}
    let mut grouped:BTreeMap<String,Vec<usize>>=BTreeMap::new();
    for (i,unit) in units.iter().enumerate(){grouped.entry(unit.group.clone()).or_default().push(i);}
    if grouped.len()<2{return Err("Unified lesson examples all describe one repeated question; add independent examples".into());}
    let mut names:Vec<String>=grouped.keys().cloned().collect();
    names.sort_by(|a,b|hash(a).cmp(&hash(b)).then(a.cmp(b)));
    // A category represented by just one independent unit cannot have both
    // training and validation. Keep its only unit in training; report zero
    // held-out for that category rather than claiming to have tested it.
    let mut protected=BTreeSet::<String>::new();
    for kind in 0..=3{
        let kinds=names.iter().filter(|name|grouped[*name].iter()
            .any(|&i|units[i].kind==kind)).collect::<Vec<_>>();
        if kinds.len()==1{protected.insert((*kinds[0]).clone());}
    }
    let candidates=names.iter().filter(|key|!protected.contains(*key))
        .cloned().collect::<Vec<_>>();
    if candidates.is_empty(){
        return Err("Cannot reserve independent held-out groups without withholding every example in a single-example category".into());
    }
    let mut held:BTreeSet<String>=candidates.iter().rev()
        .take((names.len()/5).max(1).min(candidates.len())).cloned().collect();
    // Aim for a validation sample from each category with >=2 independent
    // prompt groups. Entire repeated groups move together across categories.
    for kind in 0..=3{
        let candidates=names.iter().filter(|name|grouped[*name].iter()
            .any(|&i|units[i].kind==kind)).collect::<Vec<_>>();
        if candidates.len()>=2 && !candidates.iter().any(|x|held.contains(*x)){
            if let Some(candidate)=candidates.iter().rev().find(|key|!protected.contains(**key)){
                held.insert((**candidate).clone());
            }
        }
    }
    // Never allocate every unique unit to validation.
    if held.len()==names.len(){held.remove(&names[0]);}
    // Do not strand an objective with validation only. Each represented
    // category must have at least one independent training unit.
    for kind in 0..=3{
        let belonging=names.iter().filter(|name|grouped[*name].iter()
            .any(|&i|units[i].kind==kind)).collect::<Vec<_>>();
        if !belonging.is_empty() && belonging.iter().all(|name|held.contains(*name)){
            held.remove(belonging[0]);
        }
    }
    let mut prepared=PreparedLessons{
        train:Vec::new(),validation:Vec::new(),train_lessons:0,
        validation_lessons:0,mode:"unified".into(),training_text:String::new(),
        memory_train:Vec::new(),memory_validation:Vec::new(),
        reasoning_train:Vec::new(),reasoning_validation:Vec::new(),
        understanding_train:Vec::new(),understanding_validation:Vec::new(),
        heldout_replies:Vec::new(),
        train_kinds:Vec::new(),validation_kinds:Vec::new(),
        category_train:[0;5],category_validation:[0;5],
    };
    let mut has_train=vec![false;blocks.len()];
    // Preserve entire group boundaries. No expected answer from a held-out
    // question/session may enter the train set, preference loss, or replay.
    for unit in &units{
        let validation=held.contains(&unit.group);
        if validation{
            prepared.validation_lessons+=1;
            prepared.category_validation[unit.kind as usize]+=1;
        }else{
            has_train[unit.block]=true;
            prepared.train_lessons+=1;
            prepared.category_train[unit.kind as usize]+=1;
        }
        let mut prior=Vec::<String>::new();
        for pair in &unit.turns{
            supervised(pair,&prior,&mut prepared,validation,unit.kind)?;
            if !validation {
                prepared.training_text.push_str(&format!("{}\n{}\n",pair.user,pair.assistant));
            }
            if unit.kind==CONVERSATION{
                prior.push(format!("U: {}\nN: {}",pair.user,pair.assistant));
                if prior.len()>12{prior.remove(0);}
            }
        }
        if let Some(item)=unit.comparison.as_ref(){
            if validation{prepared.reasoning_validation.push(item.clone());}
            else{prepared.reasoning_train.push(item.clone());}
        }
    }
    // Contrast context-conditioned answers from DIFFERENT independent
    // understanding examples. All targets originate from teacher examples;
    // no scripted inference, semantic rule or synthetic correct answer.
    for validation in [false,true]{
        let candidates=units.iter().filter(|unit|unit.kind==UNDERSTANDING &&
            held.contains(&unit.group)==validation).collect::<Vec<_>>();
        let pairs=candidates.iter().map(|item|&item.turns[0]).collect::<Vec<_>>();
        let features=pairs.iter().map(|pair|lexical_features(&pair.user))
            .collect::<Vec<_>>();
        let mut comparisons=Vec::<ReasoningExample>::new();
        for (i,pair) in pairs.iter().enumerate(){
            if let Some(other)=select_context_negative(i,&pairs,&features){
                comparisons.push((dialogue_prefix(&pair.user)?,
                    pair.assistant.as_bytes().to_vec(),
                    pairs[other].assistant.as_bytes().to_vec()));
            }
        }
        if validation{prepared.understanding_validation=comparisons;}
        else{prepared.understanding_train=comparisons;}
    }
    // Explanations contribute lightly through *language* prediction only.
    // They are NOT labeled as Nova's replies, nor converted to prewritten
    // interpretations, facts, rules, or inferred answer targets.
    // Restrict to 32 targets per train-bearing block to avoid dominating
    // supervised understanding/conversation objectives.
    for (idx,block) in blocks.iter().enumerate(){
        if !has_train[idx]{continue;}
        let bytes=block.explanation.as_bytes();
        // The trainer's instructional prose belongs to a DIFFERENT
        // conditioning context than Nova's conversation replies. Both use
        // the same neural output head and both receive real gradients; the
        // teacher marker prevents treating bare explanation continuations as
        // answer-like prompts. It is a training-input scaffold, not a model
        // response rule or symbolic interpreter.
        let mut prefix=b"T: ".to_vec();
        for &target in bytes.iter().take(32){
            prepared.train.push((prefix.clone(),target));
            prepared.memory_train.push(Vec::new());
            prepared.train_kinds.push(EXPLANATION);
            prefix.push(target);
        }
        prepared.category_train[EXPLANATION as usize]+=1;
        prepared.training_text.push_str(&block.explanation);
        prepared.training_text.push('\n');
    }
    if prepared.train.is_empty()||prepared.validation.is_empty(){
        return Err("Unified lesson needs independent train and held-out examples".into());
    }
    if prepared.train.len()>MAX_SAMPLES{
        return Err(format!("Too many training characters (max {MAX_SAMPLES})"));
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn basic()->String{
        concat!(
            "[Dialogue Lesson]\nGreetings are used to begin conversations.\n[examples]\n",
            "hello  Hello!\nhi nova  Hi!\nhow are you  Fine, thanks.\n",
            "[/examples]\n[/Dialogue Lesson]\n",
            "[Understanding Lesson]\nSentences describe properties and quantities.\n[examples]\n",
            "Mira has a red ball. What color is Mira's ball?  Red.\n",
            "Tim has four apples. How many apples?  Four.\n",
            "Nora opened the door. Who opened it?  Nora.\n",
            "[/examples]\n[/Understanding Lesson]\n",
            "[Multi-Turn Conversation Lesson]\nEarlier conversation matters.\n[examples]\n",
            "[session]\nUser: Hello.\nNova: Hello!\nUser: My name is Mira.\nNova: Nice to meet you.\n",
            "User: What is my name?\nNova: Mira.\n[/session]\n",
            "[/examples]\n[/Multi-Turn Conversation Lesson]\n",
            "[Reasoning Lesson]\nConclusions must follow evidence.\n[examples]\n",
            "Premise: Susie has two apples. Tim has four apples. Question: Who has more? Supported: Tim. Contradicted: Susie.\n",
            "Premise: Mira has a red ball. Question: What color is the ball? Supported: Red. Contradicted: Blue.\n",
            "[/examples]\n[/Reasoning Lesson]\n"
        ).to_string()
    }
    #[test]fn contrast_prefers_similar_questions_over_unrelated_answers(){
        let data=[
            DialoguePair{user:"Mira has a red ball. What color is Mira's ball?".into(),
                         assistant:"Red.".into()},
            DialoguePair{user:"Tim has a blue ball. What color is Tim's ball?".into(),
                         assistant:"Blue.".into()},
            DialoguePair{user:"Nora has four apples. How many apples does Nora have?".into(),
                         assistant:"Four.".into()},
        ];
        let pairs=data.iter().collect::<Vec<_>>();
        let features=pairs.iter().map(|x|lexical_features(&x.user)).collect::<Vec<_>>();
        assert_eq!(select_context_negative(0,&pairs,&features),Some(1));
        assert_eq!(select_context_negative(1,&pairs,&features),Some(0));
        assert_ne!(select_context_negative(2,&pairs,&features),Some(2));
    }
    #[test]fn contextual_targets_never_cross_into_other_split(){
        let mut examples=String::from(
            "[Understanding Lesson]\nEntities and their properties.\n[examples]\n");
        for i in 0..18{
            examples.push_str(&format!(
                "Person{i} has a {} ball. What color is Person{i}'s ball?  {}.\n",
                if i%2==0{"red"}else{"blue"},
                if i%2==0{"Red"}else{"Blue"}
            ));
        }
        examples.push_str("[/examples]\n[/Understanding Lesson]");
        let p=prepare(&examples).unwrap();
        for (prefix,good,bad) in &p.understanding_train{
            assert_ne!(good,bad);
            assert!(!p.understanding_validation.iter().any(|(held,_,_)|held==prefix));
        }
        for (prefix,good,bad) in &p.understanding_validation{
            assert_ne!(good,bad);
            assert!(!p.understanding_train.iter().any(|(train,_,_)|train==prefix));
        }
    }
    #[test]fn combined_blocks_create_joint_objectives(){
        let p=prepare(&basic()).unwrap();
        assert_eq!(p.mode,"unified");
        assert!(p.train_kinds.contains(&DIALOGUE));
        assert!(p.train_kinds.contains(&UNDERSTANDING));
        assert!(p.train_kinds.contains(&CONVERSATION));
        assert!(p.train_kinds.contains(&REASONING));
        assert!(p.train_kinds.contains(&EXPLANATION));
        assert!(!p.reasoning_train.is_empty() || !p.reasoning_validation.is_empty());
        assert!(!p.understanding_train.is_empty());
        assert!(p.understanding_train.iter().all(|(_,good,bad)|good!=bad));
        assert_eq!(p.train.len(),p.memory_train.len());
        assert_eq!(p.train.len(),p.train_kinds.len());
        assert_eq!(p.validation.len(),p.validation_kinds.len());
        assert!(!p.heldout_replies.is_empty());
        assert!(p.heldout_replies.iter().all(|(_,expected,_,kind)|
            !expected.is_empty() && *kind<=REASONING));

    }
    #[test]fn mismatched_or_unsafe_blocks_reject_before_training(){
        assert!(prepare("[Dialogue Lesson]\n[examples]\nhi  Hey\n[/examples]\n[/Understanding Lesson]").is_err());
        assert!(prepare("[Dialogue Lesson]\nGreeting.\n[examples]\nhi\tHey\nbye  Bye\n[/examples]\n[/Dialogue Lesson]").is_err());
        assert!(prepare(&basic().replace("[/session]","")).is_err());
        assert!(prepare(&basic().replace(" Supported: Tim."," Answer: Tim.")).is_err());
        assert!(prepare(&basic().replace("[/examples]","[/unknown]")).is_err());
    }
    #[test]fn every_objective_has_training_examples_even_with_overlapping_groups(){
        let p=prepare(&basic()).unwrap();
        for kind in 0..4{
            assert!(p.category_train[kind]>0,
                "Objective {kind} was assigned exclusively to validation");
        }
        for (prefix,_,_,_) in &p.heldout_replies{
            assert!(!p.reasoning_train.iter().any(|(prompt,_,_)|prompt==prefix));
            assert!(!p.understanding_train.iter().any(|(prompt,_,_)|prompt==prefix));
        }
    }
    #[test]fn instructional_prose_is_conditioned_as_teacher_not_as_nova_reply(){
        let plan=prepare(&basic()).unwrap();
        let examples=plan.train.iter().zip(&plan.train_kinds)
            .filter(|(_,kind)|**kind==EXPLANATION).collect::<Vec<_>>();
        assert!(!examples.is_empty());
        assert!(examples.iter().all(|((context,_),_)|
            context.starts_with(b"T: ") && !context.ends_with(b"\nN: ")));
        assert!(plan.heldout_replies.iter().all(|(_,_,_,kind)|*kind!=EXPLANATION));
        assert!(plan.train.iter().zip(&plan.train_kinds)
            .any(|((context,_),kind)|*kind==DIALOGUE && context.ends_with(b"\nN: ")));
    }
    #[test]fn stable_holdouts_never_create_reply_or_preference_targets(){
        let raw=basic();
        let p=prepare(&raw).unwrap();
        for ((context,target),kind) in p.validation.iter().zip(&p.validation_kinds){
            assert_ne!(*kind,EXPLANATION);
            if *kind==REASONING && context.ends_with(b"\nN: "){
                assert!(p.reasoning_train.iter().all(|(prompt,_,_)|prompt!=context));
            }
            assert!(*target<128);
        }
        let p2=prepare(&raw).unwrap();
        assert_eq!(p.train,p2.train);
        assert_eq!(p.validation,p2.validation);
    }
}
