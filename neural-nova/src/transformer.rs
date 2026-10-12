//! Small causal, single-head Transformer decoder trained from scratch in Rust.
//! No ML framework, no canned replies, no rule-based answering.
//! Character vocabulary: 128 ASCII tokens. Attention context: last 128 bytes.
//! This single-block CPU reference implementation prioritizes correctness over speed.
use std::fs::File;
use std::io::{Read,Write};
use std::ops::Range;
use std::path::Path;

pub const VOCAB:usize=128;
pub const CONTEXT:usize=128;
pub const D:usize=32;
pub const FF:usize=64;
const MAGIC:&[u8;8]=b"NTRF0200";

#[derive(Clone)]
struct Layout {
    tok:Range<usize>, pos:Range<usize>,
    q:Range<usize>, k:Range<usize>, v:Range<usize>, o:Range<usize>,
    f1:Range<usize>, b1:Range<usize>, f2:Range<usize>, b2:Range<usize>,
    output:Range<usize>, bout:Range<usize>, len:usize,
}
fn allocate(cursor:&mut usize,n:usize)->Range<usize> {
    let start=*cursor;*cursor+=n;start..*cursor
}
fn layout()->Layout {
    let mut n=0;
    let tok=allocate(&mut n,VOCAB*D);
    let pos=allocate(&mut n,CONTEXT*D);
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
    Layout{tok,pos,q,k,v,o,f1,b1,f2,b2,output,bout,len:n}
}
fn random(seed:&mut u64)->f32 {
    *seed^=*seed>>12;*seed^=*seed<<25;*seed^=*seed>>27;
    ((*seed).wrapping_mul(2685821657736338717)>>40) as f32 / 16777216.0
}
fn dot(a:&[f32],b:&[f32])->f32 {a.iter().zip(b).map(|(x,y)|x*y).sum()}
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
#[derive(Clone)]
pub struct Transformer {
    pub weights:Vec<f32>,
    pub momentum:Vec<f32>,
    pub variance:Vec<f32>,
    pub step:u64,
}
struct Cache {
    tokens:Vec<u8>, inputs:Vec<Vec<f32>>,
    query:Vec<f32>, keys:Vec<Vec<f32>>, values:Vec<Vec<f32>>,
    attention:Vec<f32>, context:Vec<f32>,
    mid:Vec<f32>, pre:Vec<f32>, act:Vec<f32>,
    hidden:Vec<f32>, probabilities:Vec<f32>,
}
#[derive(Clone,Debug)]
pub struct Attention {
    pub index:usize,
    pub code:u8,
    pub probability:f32,
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
            (&l.f1,D),(&l.f2,FF),(&l.output,D)
        ] {
            let radius=(2.0/input_size as f32).sqrt()*0.5;
            for w in &mut weights[r.clone()] {
                *w=(random(&mut seed)*2.0-1.0)*radius;
            }
        }
        Self {
            weights, momentum:vec![0.0;l.len],variance:vec![0.0;l.len],step:0
        }
    }
    pub fn parameter_count(&self)->usize { self.weights.len() }
    pub fn context_length(&self)->usize {CONTEXT}
    fn forward(&self,context:&[u8])->Result<Cache,String> {
        if context.is_empty() {return Err("Transformer needs at least one context byte".into());}
        let tokens=context[context.len().saturating_sub(CONTEXT)..].to_vec();
        if !tokens.is_ascii() {return Err("Experimental tokenizer supports ASCII only".into());}
        let l=layout();
        let count=tokens.len();
        // Right-aligned positions: the last token always occupies position 127.
        let inputs:Vec<Vec<f32>>=tokens.iter().enumerate().map(|(i,&token)|{
            let mut x=vec![0.0;D];
            let ti=l.tok.start+token as usize*D;
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
        let mut logits=mv(&self.weights,&l.output,VOCAB,D,&hidden);
        for j in 0..VOCAB {logits[j]+=self.weights[l.bout.start+j];}
        let probabilities=stable_softmax(&logits);
        Ok(Cache{tokens,inputs,query,keys,values,attention,context,mid,pre,act,hidden,probabilities})
    }
    pub fn predict(&self,context:&[u8])->Result<Vec<f32>,String> {
        Ok(self.forward(context)?.probabilities)
    }
    pub fn attention(&self,context:&[u8])->Result<Vec<Attention>,String> {
        let cache=self.forward(context)?;
        Ok(cache.tokens.iter().enumerate().map(|(i,&code)|Attention{
            index:i,code,probability:cache.attention[i]
        }).collect())
    }
    fn gradient(&self,history:&[u8],target:u8)->Result<(f32,Vec<f32>),String> {
        if target as usize>=VOCAB {return Err("Target character must be ASCII".into());}
        let c=self.forward(history)?;
        let l=layout();
        let mut grad=vec![0.0;l.len];
        let loss=-(c.probabilities[target as usize].max(1e-30)).ln();
        let mut delta=c.probabilities.clone();
        delta[target as usize]-=1.0;
        for (j,d) in delta.iter().enumerate() {grad[l.bout.start+j]+=*d;}
        let mut dh=back_mv(&self.weights,&l.output,VOCAB,D,&c.hidden,&delta,&mut grad);
        let dact=back_mv(&self.weights,&l.f2,D,FF,&c.act,&dh,&mut grad);
        for j in 0..D {grad[l.b2.start+j]+=dh[j];}
        let mut dpre=dact;
        for i in 0..FF {
            if c.pre[i]<=0.0 {dpre[i]=0.0;}
            grad[l.b1.start+i]+=dpre[i];
        }
        let dmid_ff=back_mv(&self.weights,&l.f1,FF,D,&c.mid,&dpre,&mut grad);
        for j in 0..D {dh[j]+=dmid_ff[j];}
        let mut dx=vec![vec![0.0;D];c.tokens.len()];
        let last=dx.len()-1;
        for j in 0..D {dx[last][j]+=dh[j];}
        let dcontext=back_mv(&self.weights,&l.o,D,D,&c.context,&dh,&mut grad);
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
            let ti=l.tok.start+token as usize*D;
            let pi=l.pos.start+(CONTEXT-c.tokens.len()+i)*D;
            for j in 0..D {
                grad[ti+j]+=dx[i][j];
                grad[pi+j]+=dx[i][j];
            }
        }
        Ok((loss,grad))
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
        let norm=(sum.iter().map(|g|(*g as f64).powi(2)).sum::<f64>()).sqrt();
        let clip=if norm>1.0 {(1.0/norm) as f32}else{1.0};
        self.step+=1;
        let bias1=(1.0f64-0.9f64.powf(self.step as f64)) as f32;
        let bias2=(1.0f64-0.999f64.powf(self.step as f64)) as f32;
        for i in 0..self.weights.len() {
            let g=sum[i]*clip;
            self.momentum[i]=0.9*self.momentum[i]+0.1*g;
            self.variance[i]=0.999*self.variance[i]+0.001*g*g;
            let m=self.momentum[i]/bias1;
            let v=self.variance[i]/bias2;
            self.weights[i]-=lr*m/(v.sqrt()+1e-8);
            if !self.weights[i].is_finite() {return Err("Non-finite weight during training".into());}
        }
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
        if path.exists(){std::fs::remove_file(path).map_err(|e|e.to_string())?;}
        std::fs::rename(tmp,path).map_err(|e|e.to_string())
    }
    pub fn load(path:impl AsRef<Path>)->Result<Self,String> {
        let mut file=File::open(path).map_err(|e|e.to_string())?;
        let mut magic=[0u8;8];
        file.read_exact(&mut magic).map_err(|e|e.to_string())?;
        if &magic!=MAGIC {return Err("Incompatible Transformer checkpoint".into());}
        let mut count=[0u8;4];
        file.read_exact(&mut count).map_err(|e|e.to_string())?;
        if u32::from_le_bytes(count) as usize!=layout().len {
            return Err("Transformer checkpoint parameter count mismatch".into());
        }
        let mut step=[0u8;8];
        file.read_exact(&mut step).map_err(|e|e.to_string())?;
        let mut out=Self::new(1);
        out.step=u64::from_le_bytes(step);
        for array in [&mut out.weights,&mut out.momentum,&mut out.variance] {
            for value in array {
                let mut bytes=[0u8;4];
                file.read_exact(&mut bytes).map_err(|e|e.to_string())?;
                *value=f32::from_le_bytes(bytes);
                if !value.is_finite(){return Err("Invalid floating-point checkpoint data".into());}
            }
        }
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
