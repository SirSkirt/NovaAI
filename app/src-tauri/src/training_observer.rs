//! Read-only teacher epoch observations. These measurements NEVER affect
//! inference, gradients, checkpoint acceptance or the existing brain drawing.
use neural_nova::hybrid::HybridBrain;
use neural_nova::trainer::{Example,HeldoutReply};
use serde_json::{json,Value};
use std::fs::{self,File,OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64,Ordering};
use tauri::{AppHandle,Manager};

const MAX_LOG:u64=24*1024*1024;
const MAX_GENERATED:usize=24;
const MAX_ABLATION_CHARS:usize=12;
static NEXT_RUN:AtomicU64=AtomicU64::new(0);

fn folder(app:&AppHandle)->Result<PathBuf,String>{
    let folder=app.path().app_data_dir().map_err(|e|e.to_string())?
        .join("neural-nova-epoch-observations");
    fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    Ok(folder)
}
fn valid_id(id:&str)->bool{
    !id.is_empty()&&id.len()<96&&id.bytes()
        .all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c==b'-')
}
fn path(app:&AppHandle,id:&str)->Result<PathBuf,String>{
    if !valid_id(id){return Err("Invalid activity history name".into());}
    Ok(folder(app)?.join(format!("{id}.jsonl")))
}
pub fn start(app:&AppHandle,source:&str,objective:&str)->Result<String,String>{
    let id=format!("{}-{}-{}",crate::diagnostic_log::now_ms(),
        std::process::id(),NEXT_RUN.fetch_add(1,Ordering::Relaxed));
    let _=File::create(path(app,&id)?).map_err(|e|e.to_string())?;
    append(app,&id,&json!({"type":"start","source":source,"objective":objective,
        "note":"Read-only predictions and neural activity, NOT weights or a thoughts transcript"}))?;
    Ok(id)
}
pub fn append(app:&AppHandle,id:&str,value:&Value)->Result<(),String>{
    let bytes=serde_json::to_vec(value).map_err(|e|e.to_string())?;
    if bytes.len()>256*1024{return Err("Epoch observation exceeded 256 KiB".into());}
    let mut f=OpenOptions::new().append(true).open(path(app,id)?).map_err(|e|e.to_string())?;
    if f.metadata().map_err(|e|e.to_string())?.len()+bytes.len() as u64+1>MAX_LOG{
        return Err("Epoch history exceeded 24 MiB; existing records preserved".into());
    }
    f.write_all(&bytes).map_err(|e|e.to_string())?;
    f.write_all(b"\n").map_err(|e|e.to_string())?;
    f.sync_data().map_err(|e|e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn nova_observer_sessions(app:AppHandle)->Result<Vec<String>,String>{
    let mut names=Vec::new();
    for item in fs::read_dir(folder(&app)?).map_err(|e|e.to_string())?{
        let file=item.map_err(|e|e.to_string())?;
        if !file.file_type().map_err(|e|e.to_string())?.is_file(){continue;}
        let name=file.file_name().to_string_lossy().to_string();
        if let Some(id)=name.strip_suffix(".jsonl"){
            if valid_id(id){names.push(id.to_string());}
        }
    }
    names.sort_unstable_by(|a,b|b.cmp(a));
    names.truncate(40);
    Ok(names)
}
#[tauri::command]
pub fn nova_observer_read(app:AppHandle,session_id:String)->Result<Vec<Value>,String>{
    let file=path(&app,&session_id)?;
    if fs::metadata(&file).map_err(|e|e.to_string())?.len()>MAX_LOG{
        return Err("Activity log exceeds safe read size".into());
    }
    fs::read_to_string(file).map_err(|e|e.to_string())?.lines()
        .map(|line|serde_json::from_str(line).map_err(|e|e.to_string())).collect()
}
/// Recover full supervised practice answers from adjacent teacher-forced
/// training examples, without referencing or copying held-out examples.
pub fn training_replies<'a>(
    samples:impl Iterator<Item=(&'a Example,&'a Vec<String>,u8)>
)->Vec<HeldoutReply>{
    let samples=samples.collect::<Vec<_>>();
    let mut seen=[false;4];let mut out=Vec::new();
    for (i,(sample,memory,kind)) in samples.iter().enumerate(){
        let k=*kind as usize;
        if k>=4||seen[k]||!sample.0.ends_with(b"\nN: "){continue;}
        let mut context=sample.0.clone();let mut expected=Vec::new();
        for (next,_,next_kind) in samples.iter().skip(i).take(98){
            if *next_kind!=*kind||
                next.0!=context[context.len().saturating_sub(512)..]{break;}
            if next.1==b'\n'{break;}
            expected.push(next.1);
            context.push(next.1);
        }
        if expected.is_empty(){continue;}
        out.push((sample.0.clone(),expected,(*memory).clone(),*kind));
        seen[k]=true;
        if seen.iter().all(|v|*v){break;}
    }
    out
}

fn prompt_key(raw:&[u8])->String{
    let text=String::from_utf8_lossy(raw);
    let text=text.strip_prefix("U: ").unwrap_or(&text);
    let text=text.split("\nN: ").next().unwrap_or(text);
    text.trim().trim_end_matches(['?','!','.']).split_ascii_whitespace()
        .map(str::to_ascii_lowercase).collect::<Vec<_>>().join(" ")
}
fn select_heldout(heldout:&[HeldoutReply],
    training_keys:&std::collections::BTreeSet<String>)->Vec<HeldoutReply>{
    (0..4u8).filter_map(|kind|{
        heldout.iter().filter(|x|x.3==kind)
            .max_by_key(|(prompt,_,memory,_)|{
                let novel=!training_keys.contains(&prompt_key(prompt));
                usize::from(novel)*100+
                    (if kind==2{memory.len().min(12)*10}else{0})+
                    usize::from(prompt.len()>24)
            }).cloned()
    }).collect()
}
fn greedy(p:&[f32])->u8{
    (0..p.len()).filter(|i|*i==10||(32..127).contains(i))
        .max_by(|a,b|p[*a].total_cmp(&p[*b])).unwrap_or(10) as u8
}
fn variation(a:&[f32],b:&[f32])->f32{
    a.iter().zip(b).map(|(x,y)|(x-y).abs()).sum::<f32>()/2.0
}
fn responses(brain:&HybridBrain,split:&str,samples:&[HeldoutReply],
    training_keys:&std::collections::BTreeSet<String>)->Result<Vec<Value>,String>{
    let mut seen=[false;4];let mut out=Vec::new();
    for (prompt,answer,memory,kind) in samples{
        let k=*kind as usize;
        if k>=4||seen[k]{continue;}
        seen[k]=true;
        let mut context=prompt.clone();let mut generated=Vec::new();let mut ended=false;
        for _ in 0..MAX_GENERATED{
            let p=brain.predict_with_memory(&context,256,memory)?;
            let next=greedy(&p);
            if next==b'\n'{ended=true;break;}
            generated.push(next);context.push(next);
            if context.len()>512{context.drain(..context.len()-512);}
        }
        let prefix=generated.iter().zip(answer).take_while(|(x,y)|x==y).count();
        let category=["Dialogue","Understanding","Conversation","Reasoning"][k];
        out.push(json!({
            "split":split,"category":category,
            "promptSeenInTraining":split=="held-out validation"&&
                training_keys.contains(&prompt_key(prompt)),
            "independentPrompt":split=="held-out validation"&&
                !training_keys.contains(&prompt_key(prompt)),
            "prompt":String::from_utf8_lossy(&prompt[prompt.len().saturating_sub(180)..]),
            "expected":String::from_utf8_lossy(&answer[..answer.len().min(MAX_GENERATED)]),
            "expectedTruncated":answer.len()>MAX_GENERATED,
            "generated":String::from_utf8_lossy(&generated),
            "endedWithNewline":ended,
            "completeCorrect":answer.len()<MAX_GENERATED&&ended&&generated==*answer,
            "correctPrefixCharacters":prefix,
            "matchedFirstCharacter":!generated.is_empty()&&!answer.is_empty()&&
                generated[0]==answer[0],"memoryTurns":memory.len(),
            "sampleLimit":MAX_GENERATED
        }));
    }
    Ok(out)
}

fn expected_nll(brain:&HybridBrain,prompt:&[u8],answer:&[u8],
    memory:&[String],dense_off:bool,exchange_off:bool)->Result<f32,String>{
    let mut context=prompt.to_vec();let mut loss=0.0f32;let mut n=0usize;
    for want in answer.iter().copied().chain(std::iter::once(b'\n'))
        .take(MAX_ABLATION_CHARS){
        let p=brain.predict_ablation_with_memory(
            &context,256,memory,dense_off,exchange_off)?;
        loss-=p[want as usize].max(1e-30).ln();
        context.push(want);
        if context.len()>512{context.drain(..context.len()-512);}
        n+=1;
    }
    Ok(loss/n.max(1) as f32)
}
fn ablation_reply(brain:&HybridBrain,prompt:&[u8],memory:&[String],
    dense_off:bool,exchange_off:bool)->Result<(String,bool),String>{
    let mut context=prompt.to_vec();let mut reply=Vec::new();
    for _ in 0..MAX_ABLATION_CHARS{
        let p=brain.predict_ablation_with_memory(
            &context,256,memory,dense_off,exchange_off)?;
        let next=greedy(&p);
        if next==b'\n'{
            return Ok((String::from_utf8_lossy(&reply).to_string(),true));
        }
        reply.push(next);context.push(next);
        if context.len()>512{context.drain(..context.len()-512);}
    }
    Ok((String::from_utf8_lossy(&reply).to_string(),false))
}
fn ablation_comparisons(brain:&HybridBrain,heldout:&[HeldoutReply],
    training_keys:&std::collections::BTreeSet<String>)->Result<Vec<Value>,String>{
    let mut result=Vec::new();
    for (prompt,answer,memory,kind) in heldout{
        let mut variants=Vec::new();let mut nll=[0f32;3];
        for (i,(label,no_dense,no_exchange)) in [
            ("Full brain",false,false),("No dense",true,false),
            ("No reciprocal exchange",false,true)
        ].into_iter().enumerate(){
            nll[i]=expected_nll(brain,prompt,answer,memory,no_dense,no_exchange)?;
            let (generated,ended)=ablation_reply(
                brain,prompt,memory,no_dense,no_exchange)?;
            let correct_prefix=generated.as_bytes().iter().zip(answer)
                .take_while(|(a,b)|a==b).count();
            variants.push(json!({
                "mode":label,"generated":generated,"ended":ended,
                "teacherForcedMeanNll":nll[i],"correctPrefixCharacters":correct_prefix,
                "completeCorrect":answer.len()+1<=MAX_ABLATION_CHARS&&ended&&
                    generated.as_bytes()==answer.as_slice()
            }));
        }
        let category=["Dialogue","Understanding","Conversation","Reasoning"][*kind as usize];
        result.push(json!({
            "category":category,
            "prompt":String::from_utf8_lossy(&prompt[prompt.len().saturating_sub(180)..]),
            "expected":String::from_utf8_lossy(&answer[..answer.len().min(MAX_ABLATION_CHARS)]),
            "truncatedAnswer":answer.len()+1>MAX_ABLATION_CHARS,
            "independentPrompt":!training_keys.contains(&prompt_key(prompt)),
            "memoryTurns":memory.len(),
            "benefitOfFullVsNoDenseNll":nll[1]-nll[0],
            "benefitOfFullVsNoExchangeNll":nll[2]-nll[0],
            "variants":variants
        }));
    }
    Ok(result)
}
/// Full dense/attention/bridge trace can be replayed on the original SVG.
pub fn capture(brain:&HybridBrain,source:&str,objective:&str,epoch:u32,
    training:&[HeldoutReply],heldout:&[HeldoutReply],
    training_contexts:&[Vec<u8>],metrics:Value
)->Result<Value,String>{
    let keys=training_contexts.iter().map(|v|prompt_key(v))
        .collect::<std::collections::BTreeSet<_>>();
    let selected=select_heldout(heldout,&keys);
    let mut replies=responses(brain,"training practice",training,&keys)?;
    replies.extend(responses(brain,"held-out validation",&selected,&keys)?);
    let ablations=ablation_comparisons(brain,&selected,&keys)?;
    let overlap=heldout.iter().filter(|(prompt,_,_,_)|
        keys.contains(&prompt_key(prompt))).count();
    let (prompt,_,memory,_)=selected.first().or_else(||training.first())
        .cloned().unwrap_or((b"U: hello\nN: ".to_vec(),Vec::new(),Vec::new(),0));
    let (layers,attention,bridge,p)=brain.inspect_with_memory(&prompt,256,&memory)?;
    let no_dense=brain.predict_ablation_with_memory(&prompt,256,&memory,true,false)?;
    let no_exchange=brain.predict_ablation_with_memory(&prompt,256,&memory,false,true)?;
    let dense_effect=variation(&p,&no_dense);
    let exchange_effect=variation(&p,&no_exchange);
    let mut rank=(0..p.len()).collect::<Vec<_>>();
    rank.sort_by(|a,b|p[*b].total_cmp(&p[*a]));
    let rms=|v:&[f32]|(v.iter().map(|x|x*x).sum::<f32>()/v.len().max(1) as f32).sqrt();
    let active=dense_effect>1e-5;
    Ok(json!({
        "type":"epoch","timestampMs":crate::diagnostic_log::now_ms().to_string(),
        "source":source,"objective":objective,"epoch":epoch,
        "step":brain.transformer.step,
        "weightFingerprint":crate::diagnostic_log::weight_digest(brain),
        "metrics":metrics,"replies":replies,"ablationComparisons":ablations,
        "heldoutIntegrity":{
            "totalReplies":heldout.len(),"samePromptAsTraining":overlap,
            "independentPromptCount":heldout.len()-overlap,
            "selectedCount":selected.len(),
            "warning":if overlap>0{
                "Some held-out questions repeat training questions; do not treat those as unseen prompt-generalization evidence."
            }else{
                "Selected prompts do not repeat training questions; paraphrase and topic overlap remain possible."
            }
        },
        "pathways":{
            "denseRmsByLayer":layers.iter().map(|layer|rms(layer)).collect::<Vec<_>>(),
            "bridgeRms":rms(&bridge),
            "fullVsNoDenseTotalVariation":dense_effect,
            "fullVsNoReciprocalExchangeTotalVariation":exchange_effect,
            "recurrentUpdateNorms":brain.reasoning_activity(&prompt).ok(),
            "recurrentEvidenceTopTokens":brain.evidence_activity(&prompt,&memory).ok(),
            "exchangeRoundNorms":brain.exchange_activity(&prompt).ok(),
            "interconnectUsed":active,
            "reciprocalExchangeChangedOutput":exchange_effect>1e-5
        },
        "brainSnapshot":{
            "denseLayers":layers,
            "attention":attention.iter().map(|a|json!({
                "index":a.index,"weight":a.probability,
                "character":brain.tokenizer.token_label(a.token_id)
            })).collect::<Vec<_>>(),
            "bridgeSignal":bridge,
            "topPredictions":rank.into_iter().take(8).map(|code|json!({
                "code":code,"probability":p[code]
            })).collect::<Vec<_>>(),
            "interconnectUsed":active
        },
        "limitations":"24-character practice outputs, 12-character ablation outputs and teacher-only answer likelihoods; not independent proof of reasoning or a saved weight checkpoint."
    }))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn training_examples_reconstruct_only_supervised_replies(){
        let memory=Vec::new();let start=b"U: hello\nN: ".to_vec();
        let samples=vec![(start.clone(),b'H'),
            ([start.clone(),vec![b'H']].concat(),b'i'),
            ([start.clone(),b"Hi".to_vec()].concat(),b'\n')];
        let out=training_replies(samples.iter().map(|item|(item,&memory,0)));
        assert_eq!(out.len(),1);assert_eq!(out[0].1,b"Hi");
    }
    #[test]
    fn heldout_conversation_probe_prefers_unseen_followup_with_memory(){
        let training=[b"U: hello nova\nN: ".to_vec()].into_iter()
            .map(|v|prompt_key(&v)).collect();
        let input=vec![
            (b"U: hello nova\nN: ".to_vec(),b"Hello!".to_vec(),vec![],2),
            (b"U: What is my name?\nN: ".to_vec(),b"Mira.".to_vec(),
                vec!["U: My name is Mira.\nN: Hello Mira.".into()],2)
        ];
        let picked=select_heldout(&input,&training);
        assert_eq!(picked.len(),1);
        assert_eq!(picked[0].1,b"Mira.");
        assert_eq!(picked[0].2.len(),1);
        assert!(!training.contains(&prompt_key(&picked[0].0)));
    }
    #[test]
    fn heldout_repeated_only_sample_is_not_mislabeled_independent(){
        let training=[b"U: hello nova\nN: ".to_vec()].into_iter()
            .map(|v|prompt_key(&v)).collect();
        let heldout=vec![(b"U: hello nova\nN: ".to_vec(),
            b"Hello!".to_vec(),vec![],2)];
        let picked=select_heldout(&heldout,&training);
        assert!(training.contains(&prompt_key(&picked[0].0)));
    }
    #[test]
    fn observation_paths_do_not_escape_directory(){
        assert!(valid_id("123456-12-0"));
        assert!(!valid_id("../hello"));assert!(!valid_id("a/b"));
    }
}
