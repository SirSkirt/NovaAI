//! Small causal, single-head Transformer decoder trained from scratch in Rust.
//! No ML framework, no canned replies, no rule-based answering.
//! Character vocabulary: 128 ASCII tokens. Configurable 128/256-character context.
//! This single-block CPU reference implementation prioritizes correctness over speed.
use std::fs::File;
use std::io::{Read,Write};
use std::ops::Range;
use std::path::Path;
use crate::tokenizer::{BASE,MAX_WORDS,TOKEN_CAPACITY};
use crate::{Network,Gradients};

pub const VOCAB:usize=128;
pub const CONTEXT:usize=256;
pub const ORIGINAL_CONTEXT:usize=128;
pub const D:usize=32;
pub const FF:usize=64;
/// Repeated trainable processing of the one fused latent representation.
pub const REASONING_STEPS:usize=3;
const REASONING_INPUT_SCALE:f32=1.0/32.0;
/// Two learnable round trips through the EXISTING dense hemisphere; no agent.
pub const EXCHANGE_ROUNDS:usize=2;
const DENSE_INPUT:usize=1024;
const QUERY_SCALE:f32=1.0/32.0;
const MAGIC:&[u8;8]=b"NTRF0200";

#[derive(Clone)]
struct Layout {
    tok:Range<usize>, pos:Range<usize>,
    q:Range<usize>, k:Range<usize>, v:Range<usize>, o:Range<usize>,
    f1:Range<usize>, b1:Range<usize>, f2:Range<usize>, b2:Range<usize>,
    output:Range<usize>, bout:Range<usize>, bridge:Range<usize>,
    words:Range<usize>,memory_write:Range<usize>,memory_read:Range<usize>,
    reason_read:Range<usize>,reason_bias:Range<usize>,reason_update:Range<usize>,
    feedback_query:Range<usize>,context_query:Range<usize>,feedback_response:Range<usize>,
    evidence_query:Range<usize>,evidence_response:Range<usize>,len:usize,
}
fn allocate(cursor:&mut usize,n:usize)->Range<usize> {
    let start=*cursor;*cursor+=n;start..*cursor
}
fn layout()->Layout {layout_for(CONTEXT)}
fn layout_for(window:usize)->Layout {
    let mut n=0;
    let tok=allocate(&mut n,VOCAB*D);
    let pos=allocate(&mut n,window*D);
    let q=allocate(&mut n,D*D);
    let k=allocate(&mut n,D*D);
    let v=allocate(&mut n,D*D);
    let o=allocate(&mut n,D*D);
    let f1=allocate(&mut n,FF*D);
    let b1=allocate(&mut n,FF);
    let f2=allocate(&mut n,D*FF);
    let b2=allocate(&mut n,D);
    let output=allocate(&mut n,VOCAB*D);
    let bout=allocate(&mut n,VOCAB);
    let bridge=allocate(&mut n,D*VOCAB);
    let words=allocate(&mut n,MAX_WORDS*D);
    let memory_write=allocate(&mut n,D*D);
    let memory_read=allocate(&mut n,D*D);
    // Append new matrices so every previously trained index is preserved.
    let reason_read=allocate(&mut n,D*D);
    let reason_bias=allocate(&mut n,D);
    let reason_update=allocate(&mut n,D*D);
    // Appended for byte-for-byte migration of all older learned parameters.
    let feedback_query=allocate(&mut n,DENSE_INPUT*D);
    let context_query=allocate(&mut n,DENSE_INPUT*D);
    let feedback_response=allocate(&mut n,D*VOCAB);
    // Append after ALL v0.13.1 parameters. Imported neurons and Adam moments
    // retain their exact numerical positions. New residual starts neutral.
    let evidence_query=allocate(&mut n,D*D);
    let evidence_response=allocate(&mut n,D*D);
    Layout{tok,pos,q,k,v,o,f1,b1,f2,b2,output,bout,bridge,words,memory_write,memory_read,
        reason_read,reason_bias,reason_update,
        feedback_query,context_query,feedback_response,evidence_query,evidence_response,len:n}
}
fn random(seed:&mut u64)->f32 {
    *seed^=*seed>>12;*seed^=*seed<<25;*seed^=*seed>>27;
    ((*seed).wrapping_mul(2685821657736338717)>>40) as f32 / 16777216.0
}
fn dot(a:&[f32],b:&[f32])->f32 {a.iter().zip(b).map(|(x,y)|x*y).sum()}
fn vector_norm(x:&[f32])->f32{(x.iter().map(|v|(*v as f64)*(*v as f64)).sum::<f64>()).sqrt() as f32}
fn mv(w:&[f32],r:&Range<usize>,rows:usize,cols:usize,x:&[f32])->Vec<f32> {
    (0..rows).map(|i|dot(&w[r.start+i*cols..r.start+(i+1)*cols],x)).collect()
}
fn back_mv(w:&[f32],r:&Range<usize>,rows:usize,cols:usize,
    x:&[f32],dy:&[f32],grad:&mut[f32])->Vec<f32> {
    let mut dx=vec![0.0;cols];
    for i in 0..rows {
        let start=r.start+i*cols;
        for j in 0..cols {
            grad[start+j]+=dy[i]*x[j];
            dx[j]+=w[start+j]*dy[i];
        }
    }
    dx
}
fn stable_softmax(values:&[f32])->Vec<f32> {
    let m=values.iter().copied().fold(f32::NEG_INFINITY,f32::max);
    let mut p:Vec<f32>=values.iter().map(|x|(x-m).exp()).collect();
    let sum:f32=p.iter().sum();
    for value in &mut p {*value/=sum;}
    p
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum TransformerUpdateFocus{All,Language,Interconnect}

#[derive(Clone)]
pub struct Transformer {
    pub weights:Vec<f32>,
    pub momentum:Vec<f32>,
    pub variance:Vec<f32>,
    pub step:u64,
}
struct Cache {
    tokens:Vec<u16>, inputs:Vec<Vec<f32>>,
    query:Vec<f32>, keys:Vec<Vec<f32>>, values:Vec<Vec<f32>>,
    attention:Vec<f32>, context:Vec<f32>,
    mid:Vec<f32>, pre:Vec<f32>, act:Vec<f32>,
    hidden:Vec<f32>, probabilities:Vec<f32>,
    memory_sources:Vec<Vec<u16>>, memory_means:Vec<Vec<f32>>,
    memory_activations:Vec<Vec<f32>>, memory_context:Vec<f32>,
    memory_query:Vec<f32>, memory_attention:Vec<f32>,
    reasoning:Vec<ReasoningCache>,
    hidden_before_bridge_norm:f32,dense_features_norm:f32,bridge_norm:f32,hidden_after_bridge_norm:f32,
}
// Internal states used for exact reverse-mode differentiation through all
// recurrent latent steps. No text templates, rules or alternate decoder.
struct ReasoningCache {
    input:Vec<f32>,candidate:Vec<f32>,exchange:Option<ExchangeCache>,
    evidence:EvidenceCache,
}
// Each recurrent step re-queries source token keys with its UPDATED state.
// Store the distribution for exact softmax BPTT and teacher diagnostics.
struct EvidenceCache {
    query_input:Vec<f32>,query:Vec<f32>,
    attention:Vec<f32>,context:Vec<f32>,
}
/// One queried second computation by the original dense network.
struct ExchangeCache {
    pre_exchange:Vec<f32>,input:Vec<f32>,features:Vec<f32>,gate:Vec<f32>,
}
#[derive(Clone,Debug)]
pub struct BridgeMeasurement{
    pub transformer_norm:f32,pub dense_norm:f32,pub projected_bridge_norm:f32,
    pub combined_norm:f32,pub bridge_to_transformer_ratio:f32,
    pub next_character_total_variation:f32,
}
#[derive(Clone,Debug)]
pub struct Attention {
    pub index:usize,
    pub code:u8,
    pub probability:f32,
    pub token_id:u16,
}
impl Transformer {
    pub fn new(seed:u64)->Self {
        let l=layout();
        let mut weights=vec![0.0;l.len];
        let mut seed=seed.max(1);
        for r in [&l.tok,&l.pos] {
            for w in &mut weights[r.clone()] {
                *w=(random(&mut seed)*2.0-1.0)*0.08;
            }
        }
        for (r,input_size) in [
            (&l.q,D),(&l.k,D),(&l.v,D),(&l.o,D),
            (&l.f1,D),(&l.f2,FF),(&l.output,D),(&l.bridge,VOCAB),
            (&l.words,D),(&l.memory_write,D),(&l.memory_read,D),(&l.reason_read,D),
            (&l.feedback_query,D),(&l.context_query,D),
            (&l.evidence_query,D)
        ] {
            let radius=(2.0/input_size as f32).sqrt()*0.5;
            for w in &mut weights[r.clone()] {
                *w=(random(&mut seed)*2.0-1.0)*radius;
            }
        }
        // New episodic memory is neutral until multi-turn lessons teach the
        // read projection. This protects migrated dialogue weights from
        // random memory output when users reopen an existing conversation.
        weights[l.memory_read.clone()].fill(0.0);
        // Identity at migration/startup: the learned output projection begins
        // at zero, but receives nonzero gradients from the first lesson.
        // This is not an inference fallback or hard-coded reasoning policy.
        weights[l.reason_update.clone()].fill(0.0);
        // No change to old predictions until learning makes response nonzero.
        // Query/context matrices already learnable (random); response learns first.
        weights[l.feedback_response.clone()].fill(0.0);
        weights[l.evidence_response.clone()].fill(0.0);
        Self {
            weights, momentum:vec![0.0;l.len],variance:vec![0.0;l.len],step:0
        }
    }
    pub fn parameter_count(&self)->usize { self.weights.len() }
    /// Parameter count of the last pre-recurrent hybrid checkpoint.
    pub fn previous_parameter_count()->usize {layout().reason_read.start}
    /// Parameter count of complete v0.9/v0.10 checkpoints, pre-cooperation.
    pub fn precooperation_parameter_count()->usize {layout().feedback_query.start}
    /// Size of fully learned v0.13.1 joint checkpoints before recurrent
    /// evidence rereading. This is recognized by BOTH native import paths.
    pub fn preevidence_parameter_count()->usize {layout().evidence_query.start}
    pub(crate) fn exchange_response_start()->usize {layout().feedback_response.start}
    /// Diagnostic-only view of learned latent refinement; no visualizer changes.
    pub fn reasoning_activity(&self,tokens:&[u16],dense:&[f32],memory:&[Vec<u16>])
        ->Result<Vec<f32>,String>{
        let c=self.forward_tokens_with_memory(tokens,Some(dense),CONTEXT,memory)?;
        let l=layout();
        Ok(c.reasoning.iter().map(|step|{
            let z=mv(&self.weights,&l.reason_update,D,D,&step.candidate);
            vector_norm(&z)/REASONING_STEPS as f32
        }).collect())
    }
    /// Read-only per-iteration attention maps of the actual neural evidence
    /// reread. These are not separate answers and never control inference.
    pub fn evidence_attention(&self,context:&[u16],dense:&[f32],
        memory:&[Vec<u16>],network:&Network,base:&[f32])
        ->Result<Vec<Vec<f32>>,String>{
        let cache=self.forward_tokens_internal(
            context,Some(dense),CONTEXT,memory,Some((network,base)))?;
        Ok(cache.reasoning.iter().map(|step|step.evidence.attention.clone()).collect())
    }
    pub fn context_length(&self)->usize {CONTEXT}
    fn forward(&self,context:&[u8])->Result<Cache,String> {self.forward_with_dense(context,None)}
    fn forward_with_dense(&self,context:&[u8],dense:Option<&[f32]>)->Result<Cache,String> {
        self.forward_with_dense_window(context,dense,CONTEXT)
    }
    fn forward_with_dense_window(&self,context:&[u8],dense:Option<&[f32]>,window:usize)->Result<Cache,String> {
        self.forward_tokens_with_memory(&context.iter().map(|&c|c as u16).collect::<Vec<_>>(),dense,window,&[])
    }
    fn forward_tokens_with_memory(&self,context:&[u16],dense:Option<&[f32]>,window:usize,memory:&[Vec<u16>])->Result<Cache,String> {
        self.forward_tokens_internal(context,dense,window,memory,None)
    }
    fn forward_tokens_internal(&self,context:&[u16],dense:Option<&[f32]>,window:usize,
        memory:&[Vec<u16>],cooperation:Option<(&Network,&[f32])>)->Result<Cache,String> {
        if let Some((network,base))=cooperation{
            if network.dimensions()!=[1024,512,512,128] || base.len()!=DENSE_INPUT ||
                base.iter().any(|v|!v.is_finite()){
                return Err("Cooperative dense input or topology mismatch".into());
            }
        }
        if window!=ORIGINAL_CONTEXT && window!=CONTEXT {
            return Err("Select a 128- or 256-token context window".into());
        }
        if let Some(data)=dense {
            if data.len()!=VOCAB || data.iter().any(|v|!v.is_finite()) {
                return Err("Hybrid bridge expects 128 finite dense features".into());
            }
        }
        if context.is_empty() {return Err("Transformer needs at least one context byte".into());}
        let tokens=context[context.len().saturating_sub(window)..].to_vec();
        if tokens.iter().any(|&t|t as usize>=TOKEN_CAPACITY){return Err("Unknown token ID".into());}
        let l=layout();
        let count=tokens.len();
        // Right-aligned positions: the last token always occupies position CONTEXT-1.
        let inputs:Vec<Vec<f32>>=tokens.iter().enumerate().map(|(i,&token)|{
            let mut x=vec![0.0;D];
            let ti=if (token as usize)<BASE{l.tok.start+token as usize*D}
                else{l.words.start+(token as usize-BASE)*D};
            let pi=l.pos.start+(CONTEXT-count+i)*D;
            for j in 0..D {x[j]=self.weights[ti+j]+self.weights[pi+j];}
            x
        }).collect();
        let query=mv(&self.weights,&l.q,D,D,inputs.last().unwrap());
        let keys:Vec<Vec<f32>>=inputs.iter().map(|x|mv(&self.weights,&l.k,D,D,x)).collect();
        let values:Vec<Vec<f32>>=inputs.iter().map(|x|mv(&self.weights,&l.v,D,D,x)).collect();
        let scale=(D as f32).sqrt().recip();
        let attention=stable_softmax(&keys.iter().map(|k|dot(&query,k)*scale).collect::<Vec<_>>());
        let mut context=vec![0.0;D];
        for i in 0..count { for j in 0..D { context[j]+=attention[i]*values[i][j]; } }
        let projected=mv(&self.weights,&l.o,D,D,&context);
        let mut mid=inputs.last().unwrap().clone();
        for j in 0..D { mid[j]+=projected[j]; }
        let mut pre=mv(&self.weights,&l.f1,FF,D,&mid);
        for j in 0..FF {pre[j]+=self.weights[l.b1.start+j];}
        let act:Vec<f32>=pre.iter().map(|x|x.max(0.0)).collect();
        let ff=mv(&self.weights,&l.f2,D,FF,&act);
        let mut hidden=mid.clone();
        for j in 0..D {hidden[j]+=ff[j]+self.weights[l.b2.start+j];}
        let hidden_before_bridge_norm=vector_norm(&hidden);
        let mut dense_features_norm=0f32;
        let mut bridge_norm=0f32;
        if let Some(features)=dense {
            dense_features_norm=vector_norm(features);
            let projected=mv(&self.weights,&l.bridge,D,VOCAB,features);
            bridge_norm=vector_norm(&projected);
            for j in 0..D {hidden[j]+=projected[j];}
        }
        let hidden_after_bridge_norm=vector_norm(&hidden);
        let mut memory_means=Vec::new();
        let mut memory_activations=Vec::new();
        let mut memory_context=vec![0.0;D];
        let memory_query=hidden.clone();
        for source in memory.iter().skip(memory.len().saturating_sub(12)){
            if source.is_empty() {continue;}
            let mut mean=vec![0.0;D];
            for &id in source {
                if id as usize>=TOKEN_CAPACITY{return Err("Unknown memory token".into());}
                let offset=if (id as usize)<BASE {l.tok.start+id as usize*D}
                    else{l.words.start+(id as usize-BASE)*D};
                for j in 0..D{mean[j]+=self.weights[offset+j]/source.len() as f32;}
            }
            let raw=mv(&self.weights,&l.memory_write,D,D,&mean);
            memory_means.push(mean);
            memory_activations.push(raw.iter().map(|v|v.tanh()).collect::<Vec<_>>());
        }
        let memory_attention=if memory_activations.is_empty(){Vec::new()}
            else {
                let scale=(D as f32).sqrt().recip();
                stable_softmax(&memory_activations.iter().map(|slot|
                    dot(&memory_query,slot)*scale).collect::<Vec<_>>())
            };
        for (weight,slot) in memory_attention.iter().zip(&memory_activations){
            for j in 0..D{memory_context[j]+=weight*slot[j];}
        }
        if !memory_attention.is_empty(){
            let added=mv(&self.weights,&l.memory_read,D,D,&memory_context);
            for j in 0..D{hidden[j]+=added[j];}
        }
        // Shared trainable latent reasoning. Recurrent transformations operate
        // after dense/Transformer/memory fusion. No symbolic rules or text
        // generator is used here. The SAME hidden state produces the answer.
        let mut reasoning=Vec::with_capacity(REASONING_STEPS);
        for round in 0..REASONING_STEPS{
            let old=hidden.clone();
            let mut raw=mv(&self.weights,&l.reason_read,D,D,&old);
            for j in 0..D{
                raw[j]= (raw[j]*REASONING_INPUT_SCALE+
                    self.weights[l.reason_bias.start+j]).tanh();
            }
            let update=mv(&self.weights,&l.reason_update,D,D,&raw);
            for j in 0..D{hidden[j]+=update[j]/REASONING_STEPS as f32;}
            let mut exchange=None;
            if round<EXCHANGE_ROUNDS{
                if let Some((network,base))=cooperation{
                    // Transformer hidden state queries the SAME dense network.
                    // Transformer attention context spans the full selected window,
                    // so the dense re-query is not restricted to 64 raw characters.
                    let pre_exchange=hidden.clone();
                    let hq=mv(&self.weights,&l.feedback_query,DENSE_INPUT,D,&pre_exchange);
                    let cq=mv(&self.weights,&l.context_query,DENSE_INPUT,D,&context);
                    let mut input=base.to_vec();
                    let mut gate=vec![0.0;DENSE_INPUT];
                    for j in 0..DENSE_INPUT{
                        gate[j]=((hq[j]+cq[j])*QUERY_SCALE).tanh();
                        input[j]+=gate[j];
                    }
                    let features=network.logits(&input)?;
                    let response=mv(&self.weights,&l.feedback_response,D,VOCAB,&features);
                    for j in 0..D{hidden[j]+=response[j]/EXCHANGE_ROUNDS as f32;}
                    exchange=Some(ExchangeCache{pre_exchange,input,features,gate});
                }
            }
            // NEW: every reasoning round has a fresh learned query over the
            // source evidence. Unlike reusing the initial attention-pooled
            // context, this read can select DIFFERENT facts after an exchange.
            // There is no rule, string matching or teacher-selected answer.
            let query_input=hidden.clone();
            let evidence_query=mv(&self.weights,&l.evidence_query,D,D,&query_input);
            let evidence_attention=stable_softmax(&keys.iter().map(|key|
                dot(&evidence_query,key)*scale).collect::<Vec<_>>());
            let mut evidence_context=vec![0.0;D];
            for (weight,value) in evidence_attention.iter().zip(&values){
                for j in 0..D{evidence_context[j]+=weight*value[j];}
            }
            let evidence_update=mv(&self.weights,&l.evidence_response,D,D,&evidence_context);
            for j in 0..D{hidden[j]+=evidence_update[j]/REASONING_STEPS as f32;}
            reasoning.push(ReasoningCache{input:old,candidate:raw,exchange,
                evidence:EvidenceCache{
                    query_input,query:evidence_query,
                    attention:evidence_attention,context:evidence_context
                }});
        }
        let mut logits=mv(&self.weights,&l.output,VOCAB,D,&hidden);
        for j in 0..VOCAB {logits[j]+=self.weights[l.bout.start+j];}
        let probabilities=stable_softmax(&logits);
        Ok(Cache{tokens,inputs,query,keys,values,attention,context,mid,pre,act,hidden,probabilities,
            memory_sources:memory.iter().skip(memory.len().saturating_sub(12)).cloned().collect(),
            memory_means,memory_activations,memory_context,memory_query,memory_attention,reasoning,
            hidden_before_bridge_norm,dense_features_norm,bridge_norm,hidden_after_bridge_norm})
    }
    pub fn predict(&self,context:&[u8])->Result<Vec<f32>,String> {
        Ok(self.forward(context)?.probabilities)
    }
    pub fn attention(&self,context:&[u8])->Result<Vec<Attention>,String> {
        let cache=self.forward(context)?;
        Ok(cache.tokens.iter().enumerate().map(|(i,&code)|Attention{
            index:i,code:if code<BASE as u16{code as u8}else{b'?'},probability:cache.attention[i],token_id:code
        }).collect())
    }
    fn gradient(&self,history:&[u8],target:u8)->Result<(f32,Vec<f32>),String> {
        let (loss,grad,_)=self.gradient_with_dense(history,target,None)?;
        Ok((loss,grad))
    }
    pub(crate) fn gradient_with_dense(&self,history:&[u8],target:u8,dense:Option<&[f32]>)->Result<(f32,Vec<f32>,Vec<f32>),String> {
        self.gradient_tokens_with_memory(&history.iter().map(|&c|c as u16).collect::<Vec<_>>(),target,dense,&[])
    }
    pub(crate) fn gradient_tokens_with_memory(&self,history:&[u16],target:u8,dense:Option<&[f32]>,memory:&[Vec<u16>])->Result<(f32,Vec<f32>,Vec<f32>),String> {
        self.gradient_tokens_internal(history,target,dense,memory,None,None)
    }
    /// Joint backpropagation through Transformer -> dense -> Transformer
    /// exchanges. Accumulates into the SAME dense-hemisphere gradient buffer.
    pub(crate) fn gradient_tokens_cooperative(&self,history:&[u16],target:u8,
        dense:&[f32],memory:&[Vec<u16>],network:&Network,base:&[f32],
        dense_grad:&mut Gradients)->Result<(f32,Vec<f32>,Vec<f32>),String>{
        self.gradient_tokens_internal(history,target,Some(dense),memory,
            Some((network,base,dense_grad)),None)
    }
    /// Optional ordered teacher evidence spans supervise WHICH source tokens
    /// each recurrent read attends to. Target spans affect GRADIENTS only,
    /// never forward inference, network architecture or generated responses.
    pub(crate) fn gradient_evidence_cooperative(&self,history:&[u16],
        targets:&[Vec<usize>],dense:&[f32],network:&Network,base:&[f32],
        dense_grad:&mut Gradients)->Result<(f32,Vec<f32>,Vec<f32>),String>{
        self.gradient_tokens_internal(history,b'\n',Some(dense),&[],
            Some((network,base,dense_grad)),Some(targets))
    }
    fn gradient_tokens_internal(&self,history:&[u16],target:u8,dense:Option<&[f32]>,
        memory:&[Vec<u16>],mut cooperation:Option<(&Network,&[f32],&mut Gradients)>,
        evidence_targets:Option<&[Vec<usize>]>)
        ->Result<(f32,Vec<f32>,Vec<f32>),String> {
        if target as usize>=VOCAB {return Err("Target character must be ASCII".into());}
        let c=self.forward_tokens_internal(history,dense,CONTEXT,memory,
            cooperation.as_ref().map(|(network,base,_)|(*network,*base)))?;
        let l=layout();
        let mut grad=vec![0.0;l.len];
        let mut dh=vec![0.0;D];
        let loss=if let Some(spans)=evidence_targets{
            if spans.is_empty()||spans.len()>REASONING_STEPS {
                return Err("Provide 1–3 ordered evidence spans".into());
            }
            let mut loss=0.0;
            for (round,positions) in spans.iter().enumerate(){
                if positions.is_empty() || positions.iter().any(|&i|i>=c.tokens.len()){
                    return Err("Teacher evidence target outside retained token context".into());
                }
                let attention=&c.reasoning[round].evidence.attention;
                let mass=positions.iter().map(|&i|attention[i]).sum::<f32>();
                if mass<=0.0{return Err("Evidence attention mass underflow".into());}
                loss-=mass.max(1e-30).ln()/spans.len() as f32;
            }
            loss
        }else{
            let mut delta=c.probabilities.clone();
            delta[target as usize]-=1.0;
            for (j,d) in delta.iter().enumerate(){grad[l.bout.start+j]+=*d;}
            dh=back_mv(&self.weights,&l.output,VOCAB,D,&c.hidden,&delta,&mut grad);
            -(c.probabilities[target as usize].max(1e-30)).ln()
        };
        // Exact BPTT across the recurrent reasoning steps. Each residual
        // update contributes to the same fused state's gradient.
        let mut extra_context_gradient=vec![0.0;D];
        // Attention rereads share Transformer token K/V projections; store
        // their contributions until the original attention backprop below.
        let mut evidence_dx=vec![vec![0.0;D];c.tokens.len()];
        for (round,step) in c.reasoning.iter().enumerate().rev(){
            // Evidence residual is LAST in forward round: differentiate it
            // BEFORE reciprocal dense exchange and before latent refinement.
            let ev=&step.evidence;
            let de=dh.iter().map(|value|value/REASONING_STEPS as f32)
                .collect::<Vec<_>>();
            let dc=back_mv(&self.weights,&l.evidence_response,D,D,
                &ev.context,&de,&mut grad);
            let attention_scores=c.values.iter().map(|value|dot(&dc,value))
                .collect::<Vec<_>>();
            let mean_score=attention_scores.iter().zip(&ev.attention)
                .map(|(value,weight)|value*weight).sum::<f32>();
            let mut dq=vec![0.0;D];
            let scale=(D as f32).sqrt().recip();
            for i in 0..c.tokens.len(){
                let supervised=if let Some(spans)=evidence_targets{
                    if round<spans.len(){
                        let mass=spans[round].iter()
                            .map(|&position|ev.attention[position]).sum::<f32>();
                        let included=spans[round].contains(&i);
                        (ev.attention[i]-if included{ev.attention[i]/mass.max(1e-30)}
                            else{0.0}) / spans.len() as f32
                    }else{0.0}
                }else{0.0};
                let dp=(ev.attention[i]*(attention_scores[i]-mean_score)
                    +supervised)*scale;
                let dv=dc.iter().map(|value|value*ev.attention[i]).collect::<Vec<_>>();
                let dxv=back_mv(&self.weights,&l.v,D,D,&c.inputs[i],&dv,&mut grad);
                let dk=ev.query.iter().map(|value|value*dp).collect::<Vec<_>>();
                let dxk=back_mv(&self.weights,&l.k,D,D,&c.inputs[i],&dk,&mut grad);
                for j in 0..D{
                    evidence_dx[i][j]+=dxv[j]+dxk[j];
                    dq[j]+=dp*c.keys[i][j];
                }
            }
            let dquery=back_mv(&self.weights,&l.evidence_query,D,D,
                &ev.query_input,&dq,&mut grad);
            for j in 0..D{dh[j]+=dquery[j];}
            if let Some(ref exchange)=step.exchange{
                let dresponse=dh.iter().map(|x|x/EXCHANGE_ROUNDS as f32).collect::<Vec<_>>();
                let dfeatures=back_mv(&self.weights,&l.feedback_response,D,VOCAB,
                    &exchange.features,&dresponse,&mut grad);
                if let Some((network,_,dense_grads))=cooperation.as_mut(){
                    let dinput=network.accumulate_logits_gradients_with_input(
                        &exchange.input,&dfeatures,*dense_grads)?;
                    let dgate=dinput.iter().zip(&exchange.gate).map(|(d,g)|
                        d*(1.0-g*g)*QUERY_SCALE).collect::<Vec<_>>();
                    let dquery=back_mv(&self.weights,&l.feedback_query,DENSE_INPUT,D,
                        &exchange.pre_exchange,&dgate,&mut grad);
                    let dcontext=back_mv(&self.weights,&l.context_query,DENSE_INPUT,D,
                        &c.context,&dgate,&mut grad);
                    for j in 0..D{dh[j]+=dquery[j];extra_context_gradient[j]+=dcontext[j];}
                }else{return Err("Cooperative gradient missing dense network".into());}
            }
            let dup=dh.iter().map(|x|x/REASONING_STEPS as f32).collect::<Vec<_>>();
            let dc=back_mv(&self.weights,&l.reason_update,D,D,
                &step.candidate,&dup,&mut grad);
            let dz=dc.iter().zip(&step.candidate)
                .map(|(d,c)|d*(1.0-c*c)).collect::<Vec<_>>();
            for j in 0..D{grad[l.reason_bias.start+j]+=dz[j];}
            let input_scaled=step.input.iter()
                .map(|x|x*REASONING_INPUT_SCALE).collect::<Vec<_>>();
            let upstream=back_mv(&self.weights,&l.reason_read,D,D,
                &input_scaled,&dz,&mut grad);
            for j in 0..D{dh[j]+=upstream[j]*REASONING_INPUT_SCALE;}
        }
        // Memory attention uses the post-bridge hidden state as its query.
        // Include the memory-query derivative in dh BEFORE backpropagating
        // through the dense bridge; otherwise the dense pathway misses it.
        if !c.memory_activations.is_empty(){
            let dcontext=back_mv(&self.weights,&l.memory_read,D,D,&c.memory_context,&dh,&mut grad);
            let scores=c.memory_activations.iter().map(|slot|dot(&dcontext,slot))
                .collect::<Vec<_>>();
            let weighted:f32=scores.iter().zip(&c.memory_attention)
                .map(|(value,weight)|value*weight).sum();
            let mut dquery=vec![0.0;D];
            let scale=(D as f32).sqrt().recip();
            for (i,((ids,mean),activated)) in c.memory_sources.iter()
                .zip(&c.memory_means).zip(&c.memory_activations).enumerate(){
                let dp=c.memory_attention[i]*(scores[i]-weighted)*scale;
                let da=(0..D).map(|j|{
                    dquery[j]+=dp*activated[j];
                    (c.memory_attention[i]*dcontext[j]+dp*c.memory_query[j])
                        *(1.0-activated[j]*activated[j])
                }).collect::<Vec<_>>();
                let dsource=back_mv(&self.weights,&l.memory_write,D,D,mean,&da,&mut grad);
                for &id in ids{
                    let start=if (id as usize)<BASE {l.tok.start+id as usize*D}
                        else{l.words.start+(id as usize-BASE)*D};
                    for j in 0..D{grad[start+j]+=dsource[j]/ids.len() as f32;}
                }
            }
            for j in 0..D{dh[j]+=dquery[j];}
        }
        let dense_gradient=if let Some(features)=dense {
            back_mv(&self.weights,&l.bridge,D,VOCAB,features,&dh,&mut grad)
        }else{vec![0.0;VOCAB]};
        let dact=back_mv(&self.weights,&l.f2,D,FF,&c.act,&dh,&mut grad);
        for j in 0..D {grad[l.b2.start+j]+=dh[j];}
        let mut dpre=dact;
        for i in 0..FF {
            if c.pre[i]<=0.0 {dpre[i]=0.0;}
            grad[l.b1.start+i]+=dpre[i];
        }
        let dmid_ff=back_mv(&self.weights,&l.f1,FF,D,&c.mid,&dpre,&mut grad);
        for j in 0..D {dh[j]+=dmid_ff[j];}
        let mut dx=evidence_dx;
        let last=dx.len()-1;
        for j in 0..D {dx[last][j]+=dh[j];}
        let mut dcontext=back_mv(&self.weights,&l.o,D,D,&c.context,&dh,&mut grad);
        for j in 0..D{dcontext[j]+=extra_context_gradient[j];}
        let mut datt=vec![0.0;c.tokens.len()];
        for i in 0..c.tokens.len() {
            datt[i]=dot(&dcontext,&c.values[i]);
        }
        let weighted:f32=datt.iter().zip(&c.attention).map(|(a,b)|a*b).sum();
        let scale=(D as f32).sqrt().recip();
        let mut dq=vec![0.0;D];
        for i in 0..c.tokens.len() {
            let ds=c.attention[i]*(datt[i]-weighted)*scale;
            let dv:Vec<f32>=dcontext.iter().map(|x|x*c.attention[i]).collect();
            let dvinput=back_mv(&self.weights,&l.v,D,D,&c.inputs[i],&dv,&mut grad);
            let dk:Vec<f32>=c.query.iter().map(|x|x*ds).collect();
            let dkinput=back_mv(&self.weights,&l.k,D,D,&c.inputs[i],&dk,&mut grad);
            for j in 0..D {
                dx[i][j]+=dvinput[j]+dkinput[j];
                dq[j]+=ds*c.keys[i][j];
            }
        }
        let dqinput=back_mv(&self.weights,&l.q,D,D,&c.inputs[last],&dq,&mut grad);
        for j in 0..D {dx[last][j]+=dqinput[j];}
        for (i,&token) in c.tokens.iter().enumerate() {
            let ti=if (token as usize)<BASE{l.tok.start+token as usize*D}
                else{l.words.start+(token as usize-BASE)*D};
            let pi=l.pos.start+(CONTEXT-c.tokens.len()+i)*D;
            for j in 0..D {
                grad[ti+j]+=dx[i][j];
                grad[pi+j]+=dx[i][j];
            }
        }
        Ok((loss,grad,dense_gradient))
    }
    pub fn predict_tokenized(&self,tokens:&[u16],dense:&[f32],window:usize,memory:&[Vec<u16>])->Result<Vec<f32>,String>{
        Ok(self.forward_tokens_with_memory(tokens,Some(dense),window,memory)?.probabilities)
    }
    pub fn predict_cooperative(&self,tokens:&[u16],dense:&[f32],window:usize,
        memory:&[Vec<u16>],network:&Network,base:&[f32])->Result<Vec<f32>,String>{
        Ok(self.forward_tokens_internal(tokens,Some(dense),window,memory,
            Some((network,base)))?.probabilities)
    }
    /// Read-only exchange activity for diagnostics (NOT Visualize).
    pub fn cooperative_activity(&self,tokens:&[u16],dense:&[f32],window:usize,
        network:&Network,base:&[f32])->Result<Vec<f32>,String>{
        let c=self.forward_tokens_internal(tokens,Some(dense),window,&[],
            Some((network,base)))?;
        Ok(c.reasoning.iter().filter_map(|r|r.exchange.as_ref()).map(|e|
            vector_norm(&mv(&self.weights,&layout().feedback_response,D,VOCAB,&e.features))
            /EXCHANGE_ROUNDS as f32).collect())
    }
    pub fn attention_tokenized(&self,tokens:&[u16],dense:&[f32],window:usize,memory:&[Vec<u16>])->Result<Vec<Attention>,String>{
        let c=self.forward_tokens_with_memory(tokens,Some(dense),window,memory)?;
        Ok(c.tokens.iter().enumerate().map(|(i,&id)|Attention{
            index:i,code:if (id as usize)<BASE{id as u8}else{b'?'},
            token_id:id,probability:c.attention[i],
        }).collect())
    }
    pub fn predict_hybrid(&self,context:&[u8],dense:&[f32])->Result<Vec<f32>,String> {
        self.predict_hybrid_window(context,dense,CONTEXT)
    }
    pub fn predict_hybrid_window(&self,context:&[u8],dense:&[f32],window:usize)->Result<Vec<f32>,String> {
        Ok(self.forward_with_dense_window(context,Some(dense),window)?.probabilities)
    }
    pub fn attention_hybrid(&self,context:&[u8],dense:&[f32])->Result<Vec<Attention>,String> {
        self.attention_hybrid_window(context,dense,CONTEXT)
    }
    pub fn attention_hybrid_window(&self,context:&[u8],dense:&[f32],window:usize)->Result<Vec<Attention>,String> {
        let cache=self.forward_with_dense_window(context,Some(dense),window)?;
        Ok(cache.tokens.iter().enumerate().map(|(i,&code)|Attention{
            index:i,code:if (code as usize)<BASE{code as u8}else{b'?'},
            token_id:code,probability:cache.attention[i]
        }).collect())
    }
    /// Evaluation-only bridge ablation. Replaces the dense feature input with
    /// zero while keeping Transformer weights, context, and output head intact.
    /// No trainable parameter changes, no alternate response engine.
    pub fn measure_bridge(&self,tokens:&[u16],features:&[f32],window:usize)
        ->Result<BridgeMeasurement,String>{
        let actual=self.forward_tokens_with_memory(tokens,Some(features),window,&[])?;
        let absent=self.forward_tokens_with_memory(tokens,Some(&[0.0;VOCAB]),window,&[])?;
        let variation=actual.probabilities.iter().zip(&absent.probabilities)
            .map(|(a,b)|(a-b).abs()).sum::<f32>()*0.5;
        Ok(BridgeMeasurement{
            transformer_norm:actual.hidden_before_bridge_norm,
            dense_norm:actual.dense_features_norm,
            projected_bridge_norm:actual.bridge_norm,
            combined_norm:actual.hidden_after_bridge_norm,
            bridge_to_transformer_ratio:actual.bridge_norm/
                actual.hidden_before_bridge_norm.max(1e-8),
            next_character_total_variation:variation,
        })
    }
    pub fn bridge_signal(&self,dense:&[f32])->Result<Vec<f32>,String> {
        if dense.len()!=VOCAB {return Err("Expected 128 dense bridge features".into());}
        Ok(mv(&self.weights,&layout().bridge,D,VOCAB,dense))
    }
    /// Load an older 128-position Transformer, retaining every learned weight
    /// and both Adam moments. The old right-aligned positions are copied into
    /// the *last* 128 positions, so old short-context predictions are preserved.
    /// Positions 0..127 are newly initialized and need training before longer
    /// context is a reliable capability. Works with or without the old bridge.
    pub fn import_compatible_arrays(&mut self,weights:&[f32],momentum:&[f32],variance:&[f32])->Result<(),String>{
        if weights.len()!=momentum.len() || weights.len()!=variance.len(){
            return Err("Transformer checkpoint array sizes disagree".into());
        }
        if weights.iter().chain(momentum).chain(variance).any(|v|!v.is_finite()){
            return Err("Transformer checkpoint contains non-finite values".into());
        }
        let current=layout();
        if weights.len()==current.len {
            self.weights.copy_from_slice(weights);
            self.momentum.copy_from_slice(momentum);
            self.variance.copy_from_slice(variance);
            return Ok(());
        }
        // v0.8 full hybrid Transformer has no recurrent latent matrices.
        // Append neutral, trainable parameters without perturbing predictions.
        if weights.len()==current.evidence_query.start {
            self.weights[..weights.len()].copy_from_slice(weights);
            self.momentum[..weights.len()].copy_from_slice(momentum);
            self.variance[..weights.len()].copy_from_slice(variance);
            return Ok(());
        }
        if weights.len()==current.feedback_query.start{
            self.weights[..weights.len()].copy_from_slice(weights);
            self.momentum[..weights.len()].copy_from_slice(momentum);
            self.variance[..weights.len()].copy_from_slice(variance);
            return Ok(());
        }
        if weights.len()==current.reason_read.start{
            self.weights[..weights.len()].copy_from_slice(weights);
            self.momentum[..weights.len()].copy_from_slice(momentum);
            self.variance[..weights.len()].copy_from_slice(variance);
            return Ok(());
        }
        // The previous 256-character model has no learned word or memory
        // matrices. Append these without disturbing existing parameters.
        if weights.len()==current.words.start{
            self.weights[..weights.len()].copy_from_slice(weights);
            self.momentum[..weights.len()].copy_from_slice(momentum);
            self.variance[..weights.len()].copy_from_slice(variance);
            return Ok(());
        }
        let legacy=layout_for(ORIGINAL_CONTEXT);
        if weights.len()!=legacy.words.start && weights.len()!=legacy.bridge.start {
            return Err("Unsupported Transformer checkpoint parameter count".into());
        }
        for (dst,src) in [(&mut self.weights,weights),
                          (&mut self.momentum,momentum),
                          (&mut self.variance,variance)] {
            for (old,new) in [
                (&legacy.tok,&current.tok),(&legacy.pos,&current.pos),
                (&legacy.q,&current.q),(&legacy.k,&current.k),
                (&legacy.v,&current.v),(&legacy.o,&current.o),
                (&legacy.f1,&current.f1),(&legacy.b1,&current.b1),
                (&legacy.f2,&current.f2),(&legacy.b2,&current.b2),
                (&legacy.output,&current.output),(&legacy.bout,&current.bout)
            ] {
                let start=if old.start==legacy.pos.start && old.end==legacy.pos.end{
                    new.end-old.len()
                }else{new.start};
                dst[start..start+old.len()].copy_from_slice(&src[old.clone()]);
            }
            if src.len()==legacy.words.start {
                dst[current.bridge.clone()].copy_from_slice(&src[legacy.bridge.clone()]);
            }
        }
        Ok(())
    }
    /// Norm only of the learned dense-to-Transformer bridge gradient.
    pub(crate) fn bridge_gradient_norm(&self,gradient:&[f32])->Result<f32,String>{
        let range=layout().bridge;
        if gradient.len()!=self.weights.len(){return Err("Bridge gradient shape mismatch".into());}
        Ok(vector_norm(&gradient[range]))
    }
    /// Preclip norm over exactly those Transformer parameters whose Adam
    /// moments and weights will be updated. Frozen groups do not count as
    /// clipped, even though their gradients participate in backpropagation.
    pub(crate) fn active_gradient_norm(&self,gradient:&[f32],
        focus:TransformerUpdateFocus)->Result<f32,String>{
        let l=layout();
        if gradient.len()!=l.len{return Err("Transformer gradient length mismatch".into());}
        let norm=gradient.iter().enumerate().filter(|(i,_)|{
            let linked=l.bridge.contains(i)||l.feedback_query.contains(i)
                ||l.context_query.contains(i)||l.feedback_response.contains(i)
                ||l.evidence_query.contains(i)||l.evidence_response.contains(i);
            match focus{
                TransformerUpdateFocus::All=>true,
                TransformerUpdateFocus::Language=>!linked,
                TransformerUpdateFocus::Interconnect=>linked
            }
        }).map(|(_,g)|(*g as f64)*(*g as f64)).sum::<f64>().sqrt();
        Ok(norm as f32)
    }
    /// Which Transformer parameter groups may receive a learned update.
    /// All groups still participate in forward and reverse-mode computation.
    /// Frozen groups do not change their weights OR Adam moments.
    pub(crate) fn apply_gradients_focused(&mut self,sum:&mut [f32],lr:f32,
        focus:TransformerUpdateFocus)->Result<(),String>{
        if sum.len()!=self.weights.len(){return Err("Transformer gradient length mismatch".into());}
        if !lr.is_finite() || !(1e-6..=0.05).contains(&lr){
            return Err("Invalid learning rate".into());
        }
        let l=layout();
        let is_link=|i:usize| l.bridge.contains(&i)
            ||l.feedback_query.contains(&i)
            ||l.context_query.contains(&i)
            ||l.feedback_response.contains(&i)
            ||l.evidence_query.contains(&i)||l.evidence_response.contains(&i);
        let active=|i:usize|match focus {
            TransformerUpdateFocus::All=>true,
            TransformerUpdateFocus::Language=>!is_link(i),
            TransformerUpdateFocus::Interconnect=>is_link(i),
        };
        // Preserve the original f64 optimizer calculation exactly.
        // The separate f32 probe exists only for diagnostics.
        let norm=(sum.iter().enumerate().filter(|(i,_)|active(*i))
            .map(|(_,g)|(*g as f64)*(*g as f64)).sum::<f64>()).sqrt();
        let clip=if norm>1.0{(1.0/norm) as f32}else{1.0};
        self.step+=1;
        let bias1=(1.0f64-0.9f64.powf(self.step as f64)) as f32;
        let bias2=(1.0f64-0.999f64.powf(self.step as f64)) as f32;
        for i in 0..self.weights.len(){
            if !active(i){continue;}
            let g=sum[i]*clip;
            self.momentum[i]=0.9*self.momentum[i]+0.1*g;
            self.variance[i]=0.999*self.variance[i]+0.001*g*g;
            let m=self.momentum[i]/bias1;
            let v=self.variance[i]/bias2;
            self.weights[i]-=lr*m/(v.sqrt()+1e-8);
            if !self.weights[i].is_finite(){
                return Err("Non-finite weight during focused training".into());
            }
        }
        Ok(())
    }
    pub(crate) fn apply_gradients(&mut self,sum:&mut [f32],lr:f32)->Result<(),String>{
        self.apply_gradients_focused(sum,lr,TransformerUpdateFocus::All)
    }
    pub fn train_batch(&mut self,samples:&[(Vec<u8>,u8)],lr:f32)->Result<f32,String> {
        if samples.is_empty() {return Err("Cannot train on empty batch".into());}
        if !lr.is_finite()||!(1e-6..=0.05).contains(&lr) {
            return Err("Learning rate must be between 0.000001 and 0.05".into());
        }
        let mut sum=vec![0.0;self.weights.len()];
        let mut loss=0.0;
        for (history,target) in samples {
            let (item_loss,gradient)=self.gradient(history,*target)?;
            loss+=item_loss;
            for (g,v) in sum.iter_mut().zip(gradient){*g+=v;}
        }
        let inv=1.0/samples.len() as f32;
        for g in &mut sum {*g*=inv;}
        self.apply_gradients(&mut sum,lr)?;
        Ok(loss*inv)
    }
    pub fn evaluate(&self,samples:&[(Vec<u8>,u8)])->Result<(f32,f32),String> {
        if samples.is_empty(){return Err("Evaluation dataset is empty".into());}
        let mut loss=0.0f64;
        let mut correct=0;
        for (history,target) in samples {
            let p=self.predict(history)?;
            loss-=(p[*target as usize].max(1e-30) as f64).ln();
            let predicted=p.iter().enumerate()
                .max_by(|a,b|a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap().0;
            correct+=usize::from(predicted==*target as usize);
        }
        Ok(((loss/samples.len() as f64) as f32,correct as f32/samples.len() as f32))
    }
    pub fn save(&self,path:impl AsRef<Path>)->Result<(),String> {
        let path=path.as_ref();
        let tmp=path.with_extension("tmp");
        let mut file=File::create(&tmp).map_err(|e|e.to_string())?;
        file.write_all(MAGIC).map_err(|e|e.to_string())?;
        file.write_all(&(self.weights.len() as u32).to_le_bytes()).map_err(|e|e.to_string())?;
        file.write_all(&self.step.to_le_bytes()).map_err(|e|e.to_string())?;
        for array in [&self.weights,&self.momentum,&self.variance] {
            for &w in array {file.write_all(&w.to_le_bytes()).map_err(|e|e.to_string())?;}
        }
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file); // Windows cannot reliably rename a still-open checkpoint.
        let backup=path.with_extension("bak");
        if backup.exists(){std::fs::remove_file(&backup).map_err(|e|e.to_string())?;}
        if path.exists(){std::fs::rename(path,&backup).map_err(|e|e.to_string())?;}
        match std::fs::rename(&tmp,path){
            Ok(())=>{
                if backup.exists(){std::fs::remove_file(&backup).map_err(|e|e.to_string())?;}
                Ok(())
            },
            Err(e)=>{
                if backup.exists(){let _=std::fs::rename(&backup,path);}
                Err(format!("Unable to install Transformer checkpoint: {e}"))
            }
        }
    }
    pub fn load(path:impl AsRef<Path>)->Result<Self,String> {
        let mut file=File::open(path).map_err(|e|e.to_string())?;
        let mut magic=[0u8;8];
        file.read_exact(&mut magic).map_err(|e|e.to_string())?;
        if &magic!=MAGIC {return Err("Incompatible Transformer checkpoint".into());}
        let mut count=[0u8;4];
        file.read_exact(&mut count).map_err(|e|e.to_string())?;
        let stored_count=u32::from_le_bytes(count) as usize;
        // Backward-compatible load for the independent v0.2 Transformer: its
        // original parameters/moments are retained; the appended trainable
        // bridge receives fresh initialization and zero optimizer moments.
        let old=layout_for(ORIGINAL_CONTEXT);
        if stored_count!=layout().len && stored_count!=layout().evidence_query.start && stored_count!=layout().feedback_query.start && stored_count!=layout().reason_read.start && stored_count!=layout().words.start && stored_count!=old.words.start && stored_count!=old.bridge.start {
            return Err("Transformer checkpoint parameter count mismatch".into());
        }
        let mut step=[0u8;8];
        file.read_exact(&mut step).map_err(|e|e.to_string())?;
        let mut out=Self::new(1);
        out.step=u64::from_le_bytes(step);
        let mut arrays=[vec![0.0;stored_count],vec![0.0;stored_count],vec![0.0;stored_count]];
        for array in &mut arrays{
            for value in array{
                let mut bytes=[0u8;4];
                file.read_exact(&mut bytes).map_err(|e|e.to_string())?;
                *value=f32::from_le_bytes(bytes);
            }
        }
        out.import_compatible_arrays(&arrays[0],&arrays[1],&arrays[2])?;
        let mut trailing=[0u8;1];
        if file.read(&mut trailing).map_err(|e|e.to_string())?!=0 {
            return Err("Unexpected bytes at end of checkpoint".into());
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selecting_128_or_256_matches_on_short_context(){
        let model=Transformer::new(7);
        let dense=vec![0.2;VOCAB];
        let a=model.predict_hybrid_window(b"U: hello\nN: ",&dense,128).unwrap();
        let b=model.predict_hybrid_window(b"U: hello\nN: ",&dense,256).unwrap();
        assert_eq!(a,b);
        let long=vec![b'a';175];
        assert_eq!(model.attention_hybrid_window(&long,&dense,128).unwrap().len(),128);
        assert_eq!(model.attention_hybrid_window(&long,&dense,256).unwrap().len(),175);
        assert!(model.predict_hybrid_window(b"hello",&dense,129).is_err());
    }
    #[test]
    fn legacy_128_position_import_preserves_short_context_weights_and_moments(){
        let mut learned=Transformer::new(89);
        let previous=layout_for(ORIGINAL_CONTEXT);
        let current=layout();
        let mut old=[vec![0.0;previous.words.start],vec![0.0;previous.words.start],vec![0.0;previous.words.start]];
        learned.momentum.fill(0.017);
        learned.variance.fill(0.021);
        for (dest,source) in old.iter_mut().zip([&learned.weights,&learned.momentum,&learned.variance]){
            for (old_range,new_range) in [
                (&previous.tok,&current.tok),(&previous.pos,&current.pos),
                (&previous.q,&current.q),(&previous.k,&current.k),
                (&previous.v,&current.v),(&previous.o,&current.o),
                (&previous.f1,&current.f1),(&previous.b1,&current.b1),
                (&previous.f2,&current.f2),(&previous.b2,&current.b2),
                (&previous.output,&current.output),(&previous.bout,&current.bout),
                (&previous.bridge,&current.bridge)]{
                let offset=if old_range.start==previous.pos.start && old_range.end==previous.pos.end{
                    new_range.end-old_range.len()
                }else{new_range.start};
                dest[old_range.clone()].copy_from_slice(&source[offset..offset+old_range.len()]);
            }
        }
        let mut migrated=Transformer::new(123);
        migrated.import_compatible_arrays(&old[0],&old[1],&old[2]).unwrap();
        let dense=vec![0.3;VOCAB];
        assert_eq!(learned.predict_hybrid_window(b"U: hello\nN: ",&dense,128).unwrap(),
                   migrated.predict_hybrid_window(b"U: hello\nN: ",&dense,128).unwrap());
        assert_eq!(&migrated.momentum[current.pos.end-D*128..current.pos.end],
                   &learned.momentum[current.pos.end-D*128..current.pos.end]);
        assert_eq!(&migrated.variance[current.bridge.clone()],&learned.variance[current.bridge]);
    }
    #[test]
    fn causal_predictions_ignore_data_after_the_context() {
        let model=Transformer::new(3);
        assert_eq!(model.parameter_count(),layout().len);
        let a=model.predict(b"Hello, world").unwrap();
        let b=model.predict(b"unused preface Hello, world").unwrap();
        assert_ne!(a,b);
        let att=model.attention(b"Hello").unwrap();
        assert_eq!(att.len(),5);
        assert!((att.iter().map(|v|v.probability).sum::<f32>()-1.0).abs()<1e-5);
        let p=model.predict(b"Hello").unwrap();
        assert!((p.iter().sum::<f32>()-1.0).abs()<1e-5);
    }
    #[test]
    fn recurrent_path_is_neutral_then_learns_from_prediction(){
        let mut model=Transformer::new(19);
        let l=layout();
        let before=model.predict(b"U: hello\nN: ").unwrap();
        assert_eq!(model.reasoning_activity(&[b'H' as u16],&[0.0;VOCAB],&[]).unwrap(),
            vec![0.0;REASONING_STEPS]);
        let before_reader=model.weights[l.reason_read.clone()].to_vec();
        model.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.003).unwrap();
        assert_ne!(&model.weights[l.reason_update.clone()],&vec![0.0;l.reason_update.len()]);
        assert_eq!(model.weights[l.reason_read.clone()],before_reader,
            "first gradient updates the neutral output projection; reader gets gradients afterward");
        model.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.003).unwrap();
        assert_ne!(model.weights[l.reason_read.clone()],before_reader);
        let after=model.predict(b"U: hello\nN: ").unwrap();
        assert_ne!(before,after);
        assert!(model.reasoning_activity(&[b'H' as u16],&[0.0;VOCAB],&[])
            .unwrap().iter().any(|x|*x>0.0));
    }
    #[test]
    fn recurrent_bptt_matches_finite_difference(){
        let mut model=Transformer::new(55);
        let l=layout();
        // Mimic a few learned recurrent steps to make reader derivatives nonzero.
        for i in 0..D{
            model.weights[l.reason_update.start+i*D+i]=0.3;
            model.weights[l.reason_bias.start+i]=0.02;
        }
        let context=b"U: red box\nN: ".iter().map(|c|*c as u16).collect::<Vec<_>>();
        let dense=(0..VOCAB).map(|i|if i%2==0{0.08}else{-0.04}).collect::<Vec<_>>();
        let (_,g,dinput)=model.gradient_tokens_with_memory(&context,b'R',Some(&dense),&[]).unwrap();
        let delta=0.002;
        for i in [l.reason_update.start+D+1,l.reason_read.start+D*3+4,l.reason_bias.start+2,
            l.bridge.start+7*VOCAB+11]{
            let mut positive=model.clone();positive.weights[i]+=delta;
            let mut negative=model.clone();negative.weights[i]-=delta;
            let objective=|m:&Transformer|{
                -m.forward_tokens_with_memory(&context,Some(&dense),CONTEXT,&[])
                    .unwrap().probabilities[b'R' as usize].ln()
            };
            let numeric=(objective(&positive)-objective(&negative))/(2.0*delta);
            assert!((g[i]-numeric).abs()<0.025,
                "recurrent weight {i}: analytic {} numerical {numeric}",g[i]);
        }
        let mut positive=dense.clone();positive[7]+=delta;
        let mut negative=dense.clone();negative[7]-=delta;
        let objective=|x:&[f32]|-model.forward_tokens_with_memory(&context,Some(x),CONTEXT,&[])
            .unwrap().probabilities[b'R' as usize].ln();
        let numeric=(objective(&positive)-objective(&negative))/(2.0*delta);
        assert!((dinput[7]-numeric).abs()<0.025,
            "recurrent dense input derivative: {} numerical {numeric}",dinput[7]);
    }
    #[test]
    fn previous_full_transformer_import_is_bitwise_prediction_compatible(){
        let trained=Transformer::new(73);
        let legacy=Transformer::previous_parameter_count();
        assert!(trained.parameter_count()>legacy);
        let mut migrated=Transformer::new(999);
        migrated.import_compatible_arrays(&trained.weights[..legacy],
            &trained.momentum[..legacy],&trained.variance[..legacy]).unwrap();
        assert_eq!(trained.predict(b"U: Hi\nN: ").unwrap(),
            migrated.predict(b"U: Hi\nN: ").unwrap());
        assert_eq!(&migrated.momentum[..legacy],&trained.momentum[..legacy]);
        assert_eq!(&migrated.variance[..legacy],&trained.variance[..legacy]);
    }
    #[test]
    fn gradient_matches_finite_difference() {
        let model=Transformer::new(9);
        let history=b"Hello!";
        let (_,grad)=model.gradient(history,b'a').unwrap();
        let offsets=layout();
        for index in [offsets.tok.start+(b'o' as usize)*D+1,
             offsets.q.start+1,offsets.k.start+3,offsets.v.start+5,
             offsets.o.start+4,offsets.f1.start+6,
             offsets.f2.start+8,offsets.output.start+9] {
            let eps=0.001;
            let mut plus=model.clone();plus.weights[index]+=eps;
            let mut minus=model.clone();minus.weights[index]-=eps;
            let lp=-plus.predict(history).unwrap()[b'a' as usize].ln();
            let lm=-minus.predict(history).unwrap()[b'a' as usize].ln();
            let numeric=(lp-lm)/(2.0*eps);
            assert!((grad[index]-numeric).abs()<0.01,
                "index={index}: analytic={} numeric={numeric}",grad[index]);
        }
    }
    #[test]
    fn learned_bridge_and_memory_gradients_match_finite_difference(){
        let mut model=Transformer::new(71);
        let l=layout();
        // Use two distinct learned memories and a nonzero trained-like reader
        // to make the attention-query derivative measurable.
        for j in 0..D{
            model.weights[l.memory_write.start+j*D+j]=8.0;
            model.weights[l.memory_read.start+j*D+j]=4.0;
            model.weights[l.tok.start+(b'A' as usize)*D+j]=0.35;
            model.weights[l.tok.start+(b'Z' as usize)*D+j]=-0.35;
        }
        let history=b"U: ask\nN: ".iter().map(|&v|v as u16).collect::<Vec<_>>();
        let memory=vec![vec![b'A' as u16;5],vec![b'Z' as u16;5]];
        let features=(0..VOCAB).map(|i|if i%2==0{0.07}else{-0.03}).collect::<Vec<_>>();
        let (_,grad,input_gradient)=model.gradient_tokens_with_memory(
            &history,b'H',Some(&features),&memory).unwrap();
        let eps=0.002f32;
        for index in [l.bridge.start+3*VOCAB+10,l.memory_read.start+3*D+3,
                      l.memory_write.start+2*D+2]{
            let mut plus=model.clone();plus.weights[index]+=eps;
            let mut minus=model.clone();minus.weights[index]-=eps;
            let forward=|m:&Transformer|m.forward_tokens_with_memory(
                &history,Some(&features),CONTEXT,&memory).unwrap().probabilities[b'H' as usize];
            let numeric=(-forward(&plus).ln()+forward(&minus).ln())/(2.0*eps);
            assert!((numeric-grad[index]).abs()<0.02,
                "weight {index}: finite difference {numeric} vs gradient {}",grad[index]);
        }
        let mut plus=features.clone();plus[10]+=eps;
        let mut minus=features.clone();minus[10]-=eps;
        let pplus=model.forward_tokens_with_memory(&history,Some(&plus),CONTEXT,&memory)
            .unwrap().probabilities[b'H' as usize];
        let pminus=model.forward_tokens_with_memory(&history,Some(&minus),CONTEXT,&memory)
            .unwrap().probabilities[b'H' as usize];
        let numeric=(-pplus.ln()+pminus.ln())/(2.0*eps);
        assert!((numeric-input_gradient[10]).abs()<0.02,
            "dense input finite difference {numeric} vs {}",input_gradient[10]);
    }
    #[test]
    fn bridge_measurement_is_a_read_only_ablation(){
        let model=Transformer::new(11);
        let before=model.weights.clone();
        let inputs=vec![b'U' as u16,b':' as u16,b' ' as u16,b'h' as u16];
        let features=vec![0.1;VOCAB];
        let measure=model.measure_bridge(&inputs,&features,128).unwrap();
        assert!(measure.projected_bridge_norm.is_finite());
        assert!(measure.bridge_to_transformer_ratio.is_finite());
        assert!((0.0..=1.0).contains(&measure.next_character_total_variation));
        assert_eq!(model.weights,before);
    }
    #[test]
    fn reciprocal_exchange_updates_dense_twice_and_gradients_are_exact(){
        let mut model=Transformer::new(51);
        let l=layout();
        let dense_network=Network::new(crate::NetworkConfig::neural_nova_v1()).unwrap();
        let history=b"U: Susie has two apples. Tim has four. Who has more?\nN: ";
        let tokens=history.iter().map(|&v|v as u16).collect::<Vec<_>>();
        let input=crate::encode_ascii_context(history).unwrap();
        let baseline=dense_network.logits(&input).unwrap();
        // Pretend the response projection has learned nonzero values.
        // The first response affects the hidden state that queries dense again.
        model.weights[l.feedback_response.start]=1.5;
        model.weights[l.feedback_response.start+VOCAB]=0.7;
        model.weights[l.feedback_query.start]=4.0;
        let cache=model.forward_tokens_internal(&tokens,Some(&baseline),CONTEXT,&[],
            Some((&dense_network,&input))).unwrap();
        assert_eq!(cache.reasoning.iter().filter(|s|s.exchange.is_some()).count(),
            EXCHANGE_ROUNDS);
        let first=cache.reasoning[0].exchange.as_ref().unwrap();
        let second=cache.reasoning[1].exchange.as_ref().unwrap();
        assert_ne!(first.pre_exchange,second.pre_exchange,
            "Transformer must evolve between two dense queries");
        assert_ne!(first.input,second.input,
            "later dense computation must receive the evolving Transformer state");
        let mut dense_grad=Gradients::zeros(&dense_network);
        let (_,tf_grad,_)=model.gradient_tokens_cooperative(
            &tokens,b'T',&baseline,&[],&dense_network,&input,&mut dense_grad).unwrap();
        let eps=0.005;
        for index in [l.feedback_response.start,l.feedback_query.start,l.context_query.start]{
            let mut plus=model.clone();plus.weights[index]+=eps;
            let mut minus=model.clone();minus.weights[index]-=eps;
            let objective=|m:&Transformer|{
                -m.predict_cooperative(&tokens,&baseline,CONTEXT,&[],&dense_network,&input)
                    .unwrap()[b'T' as usize].ln()
            };
            let numerical=(objective(&plus)-objective(&minus))/(2.0*eps);
            assert!((tf_grad[index]-numerical).abs()<0.04,
                "index {index}: analytic {} numerical {numerical}",tf_grad[index]);
        }
        let mut plus=dense_network.clone();plus.layers[0].weights[0]+=eps;
        let mut minus=dense_network.clone();minus.layers[0].weights[0]-=eps;
        let objective=|net:&Network|{
            -model.predict_cooperative(&tokens,&baseline,CONTEXT,&[],net,&input)
                .unwrap()[b'T' as usize].ln()
        };
        let numeric=(objective(&plus)-objective(&minus))/(2.0*eps);
        assert!((dense_grad.layers[0].weights[0]-numeric).abs()<0.04,
            "dense-backward analytic {} numerical {numeric}",
            dense_grad.layers[0].weights[0]);
    }
    #[test]
    fn focused_optimizer_freezes_untargeted_weights_and_adam_moments(){
        let original=Transformer::new(2026);
        let groups=[TransformerUpdateFocus::Language,TransformerUpdateFocus::Interconnect];
        let l=layout();
        for focus in groups {
            let mut net=original.clone();
            let mut gradient=vec![0.25;net.weights.len()];
            net.apply_gradients_focused(&mut gradient,0.001,focus).unwrap();
            let link=|i:usize|l.bridge.contains(&i)||
                l.feedback_query.contains(&i)||l.context_query.contains(&i)||
                l.feedback_response.contains(&i)||
                l.evidence_query.contains(&i)||l.evidence_response.contains(&i);
            let mut changed=0usize;
            for i in 0..net.weights.len(){
                let targeted=if focus==TransformerUpdateFocus::Language{!link(i)}else{link(i)};
                if targeted {
                    if net.weights[i]!=original.weights[i]{changed+=1;}
                }else{
                    assert_eq!(net.weights[i],original.weights[i]);
                    assert_eq!(net.momentum[i],original.momentum[i]);
                    assert_eq!(net.variance[i],original.variance[i]);
                }
            }
            assert!(changed>0);
        }
    }

    #[test]
    fn evidence_reread_is_neutral_on_v0131_and_preserves_optimizer_moments(){
        let mut old=Transformer::new(99);
        let boundary=Transformer::preevidence_parameter_count();
        old.train_batch(&[(b"U: Mira owns the blue key. What is blue?\nN: ".to_vec(),
            b'T')],0.0003).unwrap();
        let history=b"U: Mira owns the blue key. What is blue?\nN: ";
        // Emulate pre-v0.14 inference: old checkpoints had no trained
        // evidence readout, even when modern code did the test training.
        old.weights[layout().evidence_response.clone()].fill(0.0);
        let before=old.predict(history).unwrap();
        let mut imported=Transformer::new(99);
        imported.import_compatible_arrays(&old.weights[..boundary],
            &old.momentum[..boundary],&old.variance[..boundary]).unwrap();
        imported.step=old.step;
        assert_eq!(imported.predict(history).unwrap(),before);
        assert_eq!(&imported.weights[..boundary],&old.weights[..boundary]);
        assert_eq!(&imported.momentum[..boundary],&old.momentum[..boundary]);
        assert_eq!(&imported.variance[..boundary],&old.variance[..boundary]);
        assert!(imported.weights[layout().evidence_response].iter().all(|v|*v==0.0));
    }
    #[test]
    fn reasoning_rereads_token_evidence_then_trains_the_query(){
        let mut model=Transformer::new(76);
        let l=layout();
        let tokens=b"U: Red box holds a key. Blue box holds a coin. What is in the blue box?\nN: "
            .iter().map(|c|*c as u16).collect::<Vec<_>>();
        let dense=vec![0.07;VOCAB];
        let evidence_net=Network::new(crate::NetworkConfig::neural_nova_v1()).unwrap();
        let evidence_input=crate::encode_ascii_context(
            b"U: Red box holds a key. Blue box holds a coin. What is in the blue box?\nN: ").unwrap();
        let maps=model.evidence_attention(&tokens,&dense,&[],&evidence_net,&evidence_input).unwrap();
        assert_eq!(maps.len(),REASONING_STEPS);
        assert!(maps.iter().all(|att|(att.iter().sum::<f32>()-1.0).abs()<1e-5));
        assert_eq!(model.weights[l.evidence_response.clone()],
            vec![0.0;l.evidence_response.len()]);
        let (_,first,_)=model.gradient_tokens_with_memory(&tokens,b'A',Some(&dense),&[]).unwrap();
        assert!(first[l.evidence_response.clone()].iter().any(|v|v.abs()>1e-8),
            "evidence readout must get supervised output gradients on first step");
        // Make residual nonzero; ensure exact derivatives reach evidence
        // query, reread keys, and upstream token values.
        for j in 0..D{
            model.weights[l.evidence_response.start+j*D+j]=0.28;
        }
        let (_,gradient,_)=model.gradient_tokens_with_memory(
            &tokens,b'A',Some(&dense),&[]).unwrap();
        let eps=0.003;
        for index in [l.evidence_response.start+33,l.evidence_query.start+35,
                      l.v.start+25,l.k.start+11]{
            let mut plus=model.clone();plus.weights[index]+=eps;
            let mut minus=model.clone();minus.weights[index]-=eps;
            let loss=|m:&Transformer|{
                -m.forward_tokens_with_memory(&tokens,Some(&dense),CONTEXT,&[])
                    .unwrap().probabilities[b'A' as usize].max(1e-30).ln()
            };
            let finite_difference=(loss(&plus)-loss(&minus))/(2.0*eps);
            assert!((gradient[index]-finite_difference).abs()<0.045,
                "evidence BPTT index {index}: analytic {} numerical {finite_difference}",
                gradient[index]);
        }
    }
    #[test]
    fn teacher_evidence_alignment_bptt_matches_finite_difference(){
        let model=Transformer::new(915);
        let dense_network=Network::new(crate::NetworkConfig::neural_nova_v1()).unwrap();
        let prompt=b"U: A red bag contains a blue key. The key is under a shelf. Where is the key?\nN: ";
        let token_ids=prompt.iter().map(|&c|c as u16).collect::<Vec<_>>();
        let base=crate::encode_ascii_context(prompt).unwrap();
        let dense=dense_network.logits(&base).unwrap();
        let evidence=vec![(6..13).collect::<Vec<_>(),(37..49).collect::<Vec<_>>()];
        let mut dense_grads=Gradients::zeros(&dense_network);
        let (_,grad,_)=model.gradient_evidence_cooperative(
            &token_ids,&evidence,&dense,&dense_network,&base,&mut dense_grads).unwrap();
        let objective=|m:&Transformer|{
            let attentions=m.evidence_attention(
                &token_ids,&dense,&[],&dense_network,&base).unwrap();
            let loss=evidence.iter().enumerate().map(|(r,positions)|{
                let mass=positions.iter().map(|&i|attentions[r][i]).sum::<f32>();
                -mass.max(1e-30).ln()
            }).sum::<f32>()/evidence.len() as f32;
            loss
        };
        let l=layout();
        let eps=0.003;
        for index in [l.evidence_query.start+12,l.k.start+19,l.reason_read.start+17,
            l.feedback_response.start+3]{
            let mut plus=model.clone();plus.weights[index]+=eps;
            let mut minus=model.clone();minus.weights[index]-=eps;
            let numeric=(objective(&plus)-objective(&minus))/(2.0*eps);
            assert!((grad[index]-numeric).abs()<0.05,
                "evidence alignment index {index} analytic {} numeric {numeric}",grad[index]);
        }
    }
    #[test]
    fn can_train_reply_character_and_restore_optimizer() {
        let mut model=Transformer::new(41);
        let example=vec![(b"U: Hello\nN: ".to_vec(),b'H')];
        let initial=-model.predict(&example[0].0).unwrap()[b'H' as usize].ln();
        for _ in 0..60 {model.train_batch(&example,0.005).unwrap();}
        let final_loss=-model.predict(&example[0].0).unwrap()[b'H' as usize].ln();
        assert!(final_loss<initial*0.20,"{initial} -> {final_loss}");
        let p=std::env::temp_dir().join(format!("neural-transformer-{}.ntrf",std::process::id()));
        model.save(&p).unwrap();
        let restored=Transformer::load(&p).unwrap();
        std::fs::remove_file(&p).unwrap();
        assert_eq!(restored.step,model.step);
        assert_eq!(restored.weights,model.weights);
        assert_eq!(restored.momentum,model.momentum);
    }
}
