//! Small causal, single-head Transformer decoder trained from scratch in Rust.
//! No ML framework, no canned replies, no rule-based answering.
//! Character vocabulary: 128 ASCII tokens. Configurable 128/256-character context.
//! This single-block CPU reference implementation prioritizes correctness over speed.
use std::fs::File;
use std::io::{Read,Write};
use std::ops::Range;
use std::path::Path;

pub const VOCAB:usize=128;
pub const CONTEXT:usize=256;
pub const ORIGINAL_CONTEXT:usize=128;
pub const D:usize=32;
pub const FF:usize=64;
const MAGIC:&[u8;8]=b"NTRF0200";

#[derive(Clone)]
struct Layout {
    tok:Range<usize>, pos:Range<usize>,
    q:Range<usize>, k:Range<usize>, v:Range<usize>, o:Range<usize>,
    f1:Range<usize>, b1:Range<usize>, f2:Range<usize>, b2:Range<usize>,
    output:Range<usize>, bout:Range<usize>, bridge:Range<usize>, len:usize,
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
    Layout{tok,pos,q,k,v,o,f1,b1,f2,b2,output,bout,bridge,len:n}
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
            (&l.f1,D),(&l.f2,FF),(&l.output,D),(&l.bridge,VOCAB)
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
    fn forward(&self,context:&[u8])->Result<Cache,String> {self.forward_with_dense(context,None)}
    fn forward_with_dense(&self,context:&[u8],dense:Option<&[f32]>)->Result<Cache,String> {
        self.forward_with_dense_window(context,dense,CONTEXT)
    }
    fn forward_with_dense_window(&self,context:&[u8],dense:Option<&[f32]>,window:usize)->Result<Cache,String> {
        if window!=ORIGINAL_CONTEXT && window!=CONTEXT {
            return Err("Select a 128- or 256-character context window".into());
        }
        if let Some(data)=dense {
            if data.len()!=VOCAB || data.iter().any(|v|!v.is_finite()) {
                return Err("Hybrid bridge expects 128 finite dense features".into());
            }
        }
        if context.is_empty() {return Err("Transformer needs at least one context byte".into());}
        let tokens=context[context.len().saturating_sub(window)..].to_vec();
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
        if let Some(features)=dense {
            let projected=mv(&self.weights,&l.bridge,D,VOCAB,features);
            for j in 0..D {hidden[j]+=projected[j];}
        }
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
        let (loss,grad,_)=self.gradient_with_dense(history,target,None)?;
        Ok((loss,grad))
    }
    pub(crate) fn gradient_with_dense(&self,history:&[u8],target:u8,dense:Option<&[f32]>)->Result<(f32,Vec<f32>,Vec<f32>),String> {
        if target as usize>=VOCAB {return Err("Target character must be ASCII".into());}
        let c=self.forward_with_dense(history,dense)?;
        let l=layout();
        let mut grad=vec![0.0;l.len];
        let loss=-(c.probabilities[target as usize].max(1e-30)).ln();
        let mut delta=c.probabilities.clone();
        delta[target as usize]-=1.0;
        for (j,d) in delta.iter().enumerate() {grad[l.bout.start+j]+=*d;}
        let mut dh=back_mv(&self.weights,&l.output,VOCAB,D,&c.hidden,&delta,&mut grad);
        let dense_gradient=if let Some(features)=dense {
            back_mv(&self.weights,&l.bridge,D,VOCAB,features,&dh,&mut grad)
        } else {vec![0.0;VOCAB]};
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
        Ok((loss,grad,dense_gradient))
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
            index:i,code,probability:cache.attention[i]
        }).collect())
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
        let legacy=layout_for(ORIGINAL_CONTEXT);
        if weights.len()!=legacy.len && weights.len()!=legacy.bridge.start {
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
            if src.len()==legacy.len {
                dst[current.bridge.clone()].copy_from_slice(&src[legacy.bridge.clone()]);
            }
        }
        Ok(())
    }
    pub(crate) fn apply_gradients(&mut self,sum:&mut [f32],lr:f32)->Result<(),String> {
        if sum.len()!=self.weights.len() {return Err("Transformer gradient length mismatch".into());}
        if !lr.is_finite()||!(1e-6..=0.05).contains(&lr) {return Err("Invalid learning rate".into());}
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
        Ok(())
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
        if stored_count!=layout().len && stored_count!=old.len && stored_count!=old.bridge.start {
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
        let a=model.predict_hybrid_window(b"U: hello\\nN: ",&dense,128).unwrap();
        let b=model.predict_hybrid_window(b"U: hello\\nN: ",&dense,256).unwrap();
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
        let mut old=[vec![0.0;previous.len],vec![0.0;previous.len],vec![0.0;previous.len]];
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
        assert_eq!(learned.predict_hybrid_window(b"U: hello\\nN: ",&dense,128).unwrap(),
                   migrated.predict_hybrid_window(b"U: hello\\nN: ",&dense,128).unwrap());
        assert_eq!(&migrated.momentum[current.pos.end-D*128..current.pos.end],
                   &learned.momentum[current.pos.end-D*128..current.pos.end]);
        assert_eq!(&migrated.variance[current.bridge],&learned.variance[current.bridge]);
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
