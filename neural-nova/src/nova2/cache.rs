//! Incremental KV cache for faithful Nova-2 generation.
//! Each execution of a shared upper transformer block has its OWN cache:
//! loop 2 reads different input activations than loop 1.
//! Training never uses the cache; no parameters or gradients are changed.
use super::{Nova2,Ablation};

#[derive(Clone,Debug,Default)]
struct Stage{keys:Vec<f32>,values:Vec<f32>}
#[derive(Clone,Copy,Debug,Default)]
pub struct CacheStats{pub cached_tokens:usize,pub attention_keys_read:usize,pub transformer_evaluations:usize}
pub struct KvCache{
    stages:Vec<Stage>,tokens:Vec<u16>,chars:Vec<u8>,
    sequence:usize,model_step:u64,model_signature:u64,abl:Ablation,
    stats:CacheStats,
}
impl KvCache{
    pub fn new(model:&Nova2,abl:Ablation)->Self{
        let c=&model.config;
        let stages=(0..c.split+(c.blocks-c.split)*c.loops)
            .map(|_|Stage::default()).collect();
        Self{stages,tokens:Vec::new(),chars:Vec::new(),sequence:0,
            model_step:model.step,model_signature:signature(model),
            abl,stats:CacheStats::default()}
    }
    pub fn stats(&self)->CacheStats{self.stats}
    pub fn token_count(&self)->usize{self.sequence}
    pub fn reset(&mut self,model:&Nova2){
        *self=Self::new(model,self.abl);
    }
    pub fn prefill(&mut self,model:&Nova2,ids:&[u16])->Result<Vec<f32>,String>{
        let mut last=Vec::new();
        for &id in ids{last=self.append(model,id)?;}
        if last.is_empty(){return Err("Empty prompt".into());}Ok(last)
    }
    pub fn append(&mut self,model:&Nova2,token:u16)->Result<Vec<f32>,String>{
        if model.step!=self.model_step||signature(model)!=self.model_signature{
            return Err("KV cache is stale; create a new cache after training or loading".into());
        }
        let c=&model.config;let id=token as usize;
        if id>=c.vocab||id>=128+model.tokenizer.words.len(){
            return Err("Invalid generated or input token ID".into());
        }
        let value=if id<128{vec![id as u8]}else{
            model.tokenizer.words[id-128].as_bytes().to_vec()
        };
        self.chars.extend_from_slice(&value);
        if self.chars.len()>c.window_chars{
            self.chars.drain(..self.chars.len()-c.window_chars);
        }
        let f=right_features(&self.chars,c.window_chars);
        let mut h=model.val("embed")[id*c.d..(id+1)*c.d].to_vec();
        let mut stage=0;
        for i in 0..c.split{
            h=block(model,&mut self.stages[stage],&h,i,self.sequence,self.abl,
                &mut self.stats,c.max_tokens);
            stage+=1;
        }
        // Bias before SiLU, matching the reference's r.w1 + r.b1.
        let mut z1raw=mat(model.val("r.w1"),c.r1,c.window_chars*16,&f);
        for (x,b) in z1raw.iter_mut().zip(model.val("r.b1")){*x+=*b;}
        let mut z1=silu(&z1raw);
        if self.abl.no_right{z1.fill(0.);}
        if !self.abl.no_afferent{
            h=inject(model,&h,&z1,"c.n1","c.g1w","c.g1b","c.a1");
        }
        let mut z2=efferent(model,&z1,&h,self.abl);
        for round in 0..c.loops{
            if !self.abl.no_afferent{
                h=inject(model,&h,&z2,"c.n2","c.g2w","c.g2b","c.a2");
            }
            for i in c.split..c.blocks{
                h=block(model,&mut self.stages[stage],&h,i,self.sequence,self.abl,
                    &mut self.stats,c.max_tokens);
                stage+=1;
            }
            if round+1<c.loops{z2=efferent(model,&z1,&h,self.abl);}
        }
        let final_hidden=norm(&h,model.val("ln_f"));
        let emb=model.val("embed");
        let mut logits=mat(emb,c.vocab,c.d,&final_hidden);
        for (x,b) in logits.iter_mut().zip(model.val("out_bias")){*x+=*b;}
        self.tokens.push(token);
        if self.tokens.len()>c.max_tokens{self.tokens.remove(0);}
        self.sequence+=1;
        self.stats.cached_tokens=self.tokens.len();
        Ok(logits)
    }
}
fn signature(model:&Nova2)->u64{
    // Deliberately lightweight guard; a cache is always freshly instantiated
    // for a generation request, so this is a defensive stale-state check.
    let len=model.weights.len();let mut sig=model.step^(len as u64);
    for i in [0,len/4,len/2,len*3/4,len.saturating_sub(1)]{
        sig=sig.rotate_left(9)^model.weights[i].to_bits() as u64;
    }
    sig
}
fn mat(w:&[f32],rows:usize,cols:usize,x:&[f32])->Vec<f32>{
    assert_eq!(x.len(),cols);let mut out=vec![0.;rows];
    for i in 0..rows {
        let mut s=0f32;for j in 0..cols{s+=w[i*cols+j]*x[j];}
        out[i]=s;
    }
    out
}
fn norm(x:&[f32],gain:&[f32])->Vec<f32>{
    let div=(x.iter().map(|x|x*x).sum::<f32>()/x.len() as f32+1e-6).sqrt();
    x.iter().zip(gain).map(|(x,g)|x*g/div).collect()
}
fn silu(x:&[f32])->Vec<f32>{x.iter().map(|x|x/(1.+(-x).exp())).collect()}
fn sigmoid(x:&[f32])->Vec<f32>{x.iter().map(|x|1./(1.+(-x).exp())).collect()}
fn add(x:&[f32],y:&[f32])->Vec<f32>{
    x.iter().zip(y).map(|(x,y)|x+y).collect()
}
fn inject(model:&Nova2,h:&[f32],z:&[f32],ln:&str,gate_w:&str,gate_b:&str,a:&str)->Vec<f32>{
    let d=model.config.d;
    let hn=norm(h,model.val(ln));
    let gate=mat(model.val(gate_w),d,d,&hn);
    let gate=gate.iter().zip(model.val(gate_b)).map(|(x,b)|x+b).collect::<Vec<f32>>();
    let gate=sigmoid(&gate);
    let injected=mat(model.val(a),d,z.len(),z);
    h.iter().zip(gate.iter().zip(&injected))
        .map(|(h,(g,x))|h+g*x).collect()
}
fn efferent(model:&Nova2,z1:&[f32],h:&[f32],abl:Ablation)->Vec<f32>{
    let c=&model.config;
    let mut pre=mat(model.val("r.w2"),c.r2,c.r1,z1);
    for (v,b) in pre.iter_mut().zip(model.val("r.b2")){*v+=*b;}
    if !abl.no_efferent{
        let hn=norm(h,model.val("c.n1"));
        let back=mat(model.val("r.wl"),c.r2,c.d,&hn);
        for (v,b) in pre.iter_mut().zip(&back){*v+=*b;}
    }
    let mut z=silu(&pre);
    if abl.no_right{z.fill(0.);}
    z
}
fn right_features(chars:&[u8],window:usize)->Vec<f32>{
    let mut f=vec![0f32;window*16];
    let start=window-chars.len();
    for (i,&ch) in chars.iter().enumerate(){
        let idx=(start+i)*16;
        for bit in 0..7{f[idx+bit]=if (ch>>bit)&1==1{1.}else{-1.};}
        f[idx+7]=f32::from(ch.is_ascii_uppercase());
        f[idx+8]=f32::from(ch.is_ascii_lowercase());
        f[idx+9]=f32::from(ch.is_ascii_digit());
        f[idx+10]=f32::from(matches!(ch,b' '|b'\t'|b'\r'));
        f[idx+11]=f32::from(ch.is_ascii_punctuation());
        f[idx+12]=f32::from(b"aeiouAEIOU".contains(&ch));
        f[idx+13]=f32::from(matches!(ch,b'.'|b'!'|b'?'|b'\n'));
        f[idx+14]=f32::from(ch==b'\n');f[idx+15]=1.;
    }
    f
}
fn block(model:&Nova2,stage:&mut Stage,h:&[f32],i:usize,pos:usize,_abl:Ablation,
        stats:&mut CacheStats,max_context:usize)->Vec<f32>{
    let c=&model.config;let d=c.d;let hd=d/c.heads;
    let p=|name:&str|format!("b{i}.{name}");
    let x=norm(h,model.val(&p("ln1")));
    let mut q=mat(model.val(&p("wq")),d,d,&x);
    let mut k=mat(model.val(&p("wk")),d,d,&x);
    let v=mat(model.val(&p("wv")),d,d,&x);
    let half=hd/2;
    for head in 0..c.heads{for j in 0..half{
        let angle=pos as f32*c.rope_base.powf(-2.*j as f32/hd as f32);
        let (sn,cs)=angle.sin_cos();let a=head*hd+j;let b=a+half;
        let qa=q[a];let qb=q[b];q[a]=qa*cs-qb*sn;q[b]=qb*cs+qa*sn;
        let ka=k[a];let kb=k[b];k[a]=ka*cs-kb*sn;k[b]=kb*cs+ka*sn;
    }}
    stage.keys.extend(&k);stage.values.extend(&v);
    let n=stage.keys.len()/d;
    if n>max_context{
        stage.keys.drain(..d);stage.values.drain(..d);
    }
    let n=stage.keys.len()/d;
    let mut att=vec![0.;d];let scale=1./(hd as f32).sqrt();
    for head in 0..c.heads{
        let mut scores=vec![0f32;n];let mut top=f32::NEG_INFINITY;
        for t in 0..n{
            let mut score=0.;
            for j in 0..hd{
                score+=q[head*hd+j]*stage.keys[t*d+head*hd+j];
            }
            scores[t]=score*scale;top=top.max(scores[t]);
        }
        let mut total=0f32;
        for s in &mut scores{*s=(*s-top).exp();total+=*s;}
        for t in 0..n{
            let weight=scores[t]/total;
            for j in 0..hd{
                att[head*hd+j]+=weight*stage.values[t*d+head*hd+j];
            }
        }
    }
    stats.attention_keys_read+=n*c.heads;
    stats.transformer_evaluations+=1;
    let a=mat(model.val(&p("wo")),d,d,&att);
    let h=add(h,&a);
    let x=norm(&h,model.val(&p("ln2")));
    let gate=silu(&mat(model.val(&p("w_gate")),c.ff,d,&x));
    let up=mat(model.val(&p("w_up")),c.ff,d,&x);
    let ff=gate.iter().zip(up).map(|(x,y)|x*y).collect::<Vec<f32>>();
    let down=mat(model.val(&p("w_down")),d,c.ff,&ff);
    add(&h,&down)
}
impl Nova2{
    pub fn predict_cached(&self,cache:&mut KvCache,token:u16)->Result<Vec<f32>,String>{
        let logits=cache.append(self,token)?;
        let mut p=logits;
        let valid=(128+self.tokenizer.words.len()).min(self.config.vocab);
        for x in p.iter_mut().skip(valid){*x=f32::NEG_INFINITY;}
        let max=p.iter().copied().fold(f32::NEG_INFINITY,f32::max);
        let mut total=0.;
        for x in &mut p{*x=(*x-max).exp();total+=*x;}
        for x in &mut p{*x/=total;}
        Ok(p)
    }
    pub fn generate_cached(&self,prompt:&str,max_tokens:usize)->Result<String,String>{
        let ids=self.tokenizer.encode(prompt)?;
        let mut cache=KvCache::new(self,Ablation::default());
        let mut probs=Vec::new();
        for &id in &ids{probs=self.predict_cached(&mut cache,id)?;}
        if probs.is_empty(){return Err("Generation needs nonempty prompt".into());}
        let mut output=Vec::new();
        for _ in 0..max_tokens{
            let next=probs.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).unwrap().0 as u16;
            output.push(next);
            if next==b'\n' as u16{break;}
            probs=self.predict_cached(&mut cache,next)?;
        }
        self.tokenizer.decode(&output)
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    use crate::nova2::{Config,Focus};
    #[test]fn cached_matches_full_logits_across_both_thinking_rounds(){
        let mut cfg=Config::default();
        cfg.d=16;cfg.heads=2;cfg.ff=24;cfg.blocks=3;cfg.split=1;cfg.loops=2;
        cfg.r1=12;cfg.r2=10;cfg.window_chars=4;cfg.max_tokens=16;
        let m=Nova2::with_config(cfg,23);
        for abl in [Ablation::default(),Ablation{no_efferent:true,..Ablation::default()},
            Ablation{no_afferent:true,..Ablation::default()},
            Ablation{no_right:true,..Ablation::default()}]{
            let mut cache=KvCache::new(&m,abl);
            let mut prefix=Vec::new();
            for id in [3,10,17,0,7,14,21,4]{
                prefix.push(id);
                let got=cache.append(&m,id).unwrap();
                let full=m.logits(&prefix,abl).unwrap();
                let target=&full[full.len()-m.config.vocab..];
                for (x,y) in got.iter().zip(target){
                    assert!((x-y).abs()<2e-4,"KV mismatch: {x} vs {y}, len {}",prefix.len());
                }
            }
            assert_eq!(cache.stats().cached_tokens,prefix.len());
        }
    }
    #[test]fn cache_invalidated_after_gradient_update(){
        let mut m=Nova2::new();
        let mut cache=KvCache::new(&m,Ablation::default());
        m.adam_step(&vec![0.;m.parameter_count()],0.0001,Focus::Joint).unwrap();
        assert!(cache.append(&m,b'a' as u16).is_err());
    }
}
