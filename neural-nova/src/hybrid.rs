//! One Neural Nova: dense association pathway -> learned bridge -> Transformer decoder.
//! Both pathways and the bridge receive gradients from the SAME next-token loss.
//! No independent answer selection, no heuristic replies, no ML frameworks.
use crate::{encode_ascii_context,Adam,Gradients,Network,NetworkConfig};
use crate::transformer::{Transformer,Attention};
use crate::tokenizer::Tokenizer;
use std::fs::{self,File};
use std::io::{Read,Write};
use std::path::Path;

const MAGIC:&[u8;8]=b"NHYB0300";

#[derive(Clone)]
pub struct HybridBrain {
    pub transformer:Transformer,
    pub dense:Network,
    pub dense_adam:Adam,
    pub tokenizer:Tokenizer,
}
impl HybridBrain {
    pub fn new()->Self {
        let dense=Network::new(NetworkConfig::neural_nova_v1()).expect("valid hybrid dimensions");
        let dense_adam=Adam::new(&dense,0.001).expect("valid learning rate");
        Self{transformer:Transformer::new(0x4e4f5641_0300),dense,dense_adam,tokenizer:Tokenizer::new()}
    }
    pub fn dense_adam_steps(&self)->u32 {self.dense_adam.step}
    /// Grow the lossless input vocabulary; existing token IDs never change.
    pub fn learn_token_units(&mut self,lesson_text:&str)->usize{
        self.tokenizer.grow_from_corpus(lesson_text)
    }
    pub fn token_count(&self,history:&str)->Result<usize,String>{
        Ok(self.tokenizer.encode(history)?.len())
    }
    pub fn parameter_count(&self)->usize {
        self.transformer.parameter_count()+self.dense.parameter_count()
    }
    fn dense_input(&self,context:&[u8])->Result<(Vec<f32>,Vec<f32>),String>{
        let features=encode_ascii_context(context)?;
        let logits=self.dense.logits(&features)?;
        Ok((features,logits))
    }
    pub fn predict(&self,context:&[u8])->Result<Vec<f32>,String>{
        self.predict_window(context,crate::transformer::CONTEXT)
    }
    pub fn predict_window(&self,context:&[u8],window:usize)->Result<Vec<f32>,String>{
        self.predict_with_memory(context,window,&[])
    }
    pub fn predict_with_memory(&self,context:&[u8],window:usize,memory:&[String])->Result<Vec<f32>,String>{
        let (_,dense)=self.dense_input(context)?;
        let text=std::str::from_utf8(context).map_err(|e|e.to_string())?;
        let tokens=self.tokenizer.encode(text)?;
        let memory_tokens=memory.iter().map(|s|self.tokenizer.encode(s))
            .collect::<Result<Vec<_>,_>>()?;
        self.transformer.predict_tokenized(&tokens,&dense,window,&memory_tokens)
    }
    pub fn inspect(&self,context:&[u8])->Result<(Vec<Vec<f32>>,Vec<Attention>,Vec<f32>,Vec<f32>),String>{
        self.inspect_window(context,crate::transformer::CONTEXT)
    }
    pub fn inspect_window(&self,context:&[u8],window:usize)->Result<(Vec<Vec<f32>>,Vec<Attention>,Vec<f32>,Vec<f32>),String>{
        self.inspect_with_memory(context,window,&[])
    }
    pub fn inspect_with_memory(&self,context:&[u8],window:usize,memory:&[String])->Result<(Vec<Vec<f32>>,Vec<Attention>,Vec<f32>,Vec<f32>),String>{
        let (features,dense)=self.dense_input(context)?;
        let layers=self.dense.activations(&features)?;
        let text=std::str::from_utf8(context).map_err(|e|e.to_string())?;
        let tokens=self.tokenizer.encode(text)?;
        let source=memory.iter().map(|t|self.tokenizer.encode(t)).collect::<Result<Vec<_>,_>>()?;
        let attention=self.transformer.attention_tokenized(&tokens,&dense,window,&source)?;
        let signal=self.transformer.bridge_signal(&dense)?;
        let probabilities=self.transformer.predict_tokenized(&tokens,&dense,window,&source)?;
        Ok((layers,attention,signal,probabilities))
    }
    pub fn train_batch(&mut self,samples:&[(Vec<u8>,u8)],lr:f32)->Result<f32,String>{
        if samples.is_empty(){return Err("Cannot train an empty hybrid batch".into());}
        if !lr.is_finite() || !(1e-6..=0.05).contains(&lr){
            return Err("Hybrid learning rate outside supported range".into());
        }
        let mut tf_grad=vec![0.0f32;self.transformer.parameter_count()];
        let mut dense_grad=Gradients::zeros(&self.dense);
        let mut loss=0.0;
        for (context,target) in samples {
            let (input,logits)=self.dense_input(context)?;
            let tokens=self.tokenizer.encode(std::str::from_utf8(context).map_err(|e|e.to_string())?)?;
            let (item_loss,transformer_gradient,upstream)=
                self.transformer.gradient_tokens_with_memory(&tokens,*target,Some(&logits),&[])?;
            self.dense.accumulate_logits_gradients(&input,&upstream,&mut dense_grad)?;
            loss+=item_loss;
            for (a,b) in tf_grad.iter_mut().zip(transformer_gradient){*a+=b;}
        }
        let inverse=1.0/samples.len() as f32;
        for value in &mut tf_grad{*value*=inverse;}
        dense_grad.scale(inverse);
        dense_grad.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut tf_grad,lr)?;
        self.dense_adam.apply(&mut self.dense,&dense_grad)?;
        Ok(loss*inverse)
    }
    /// Multi-turn supervised teacher examples. Prior turns are compressed into
    /// bounded neural memory slots; gradients update both memory matrices and
    /// the existing dense/Transformer learning pathways together.
    pub fn train_memory_batch(&mut self,samples:&[(Vec<u8>,u8,Vec<String>)],lr:f32)->Result<f32,String>{
        if samples.is_empty(){return Err("Empty memory training batch".into());}
        let mut tf_grad=vec![0f32;self.transformer.parameter_count()];
        let mut dense_grad=Gradients::zeros(&self.dense);
        let mut loss=0.0;
        for (history,target,prior_turns) in samples{
            let (input,logits)=self.dense_input(history)?;
            let tokens=self.tokenizer.encode(std::str::from_utf8(history).map_err(|e|e.to_string())?)?;
            let memory=prior_turns.iter().map(|t|self.tokenizer.encode(t))
                .collect::<Result<Vec<_>,_>>()?;
            let (value,g,upstream)=self.transformer.gradient_tokens_with_memory(
                &tokens,*target,Some(&logits),&memory)?;
            loss+=value;
            self.dense.accumulate_logits_gradients(&input,&upstream,&mut dense_grad)?;
            for (a,b) in tf_grad.iter_mut().zip(g){*a+=b;}
        }
        let inv=1.0/samples.len() as f32;
        for x in &mut tf_grad{*x*=inv;}
        dense_grad.scale(inv);dense_grad.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut tf_grad,lr)?;
        self.dense_adam.apply(&mut self.dense,&dense_grad)?;
        Ok(loss*inv)
    }
    pub fn evaluate(&self,samples:&[(Vec<u8>,u8)])->Result<(f32,f32),String>{
        if samples.is_empty(){return Err("Empty validation set".into());}
        let mut loss=0.0f64;
        let mut correct=0usize;
        for (history,target) in samples{
            let p=self.predict(history)?;
            loss-=(p[*target as usize].max(1e-30) as f64).ln();
            let top=p.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).unwrap().0;
            if top==*target as usize{correct+=1;}
        }
        Ok(((loss/samples.len() as f64) as f32,correct as f32/samples.len() as f32))
    }
    pub fn save(&self,path:impl AsRef<Path>)->Result<(),String>{
        let path=path.as_ref();
        let tmp=path.with_extension("tmp");
        let mut file=File::create(&tmp).map_err(|e|e.to_string())?;
        file.write_all(MAGIC).map_err(|e|e.to_string())?;
        write_u32(&mut file,self.transformer.weights.len() as u32)?;
        file.write_all(&self.transformer.step.to_le_bytes()).map_err(|e|e.to_string())?;
        for array in [&self.transformer.weights,&self.transformer.momentum,&self.transformer.variance]{
            write_floats(&mut file,array)?;
        }
        let dimensions=self.dense.dimensions();
        write_u32(&mut file,dimensions.len() as u32)?;
        for dim in dimensions{write_u32(&mut file,dim as u32)?;}
        write_u32(&mut file,self.dense_adam.step)?;
        for layer in &self.dense.layers{
            write_floats(&mut file,&layer.weights)?;
            write_floats(&mut file,&layer.biases)?;
        }
        for gradients in [&self.dense_adam.m,&self.dense_adam.v]{
            for layer in &gradients.layers{
                write_floats(&mut file,&layer.weights)?;
                write_floats(&mut file,&layer.biases)?;
            }
        }
        // v0.5 trailer: word IDs belong with the weights they condition.
        file.write_all(b"WORDUNIT").map_err(|e|e.to_string())?;
        write_u32(&mut file,self.tokenizer.words.len() as u32)?;
        for word in &self.tokenizer.words {
            write_u32(&mut file,word.len() as u32)?;
            file.write_all(word.as_bytes()).map_err(|e|e.to_string())?;
        }
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file);
        let backup=path.with_extension("bak");
        if backup.exists(){fs::remove_file(&backup).map_err(|e|e.to_string())?;}
        if path.exists(){fs::rename(path,&backup).map_err(|e|e.to_string())?;}
        match fs::rename(&tmp,path){
            Ok(())=>{if backup.exists(){fs::remove_file(backup).map_err(|e|e.to_string())?;}Ok(())},
            Err(e)=>{if backup.exists(){let _=fs::rename(&backup,path);}Err(e.to_string())}
        }
    }
    pub fn load(path:impl AsRef<Path>)->Result<Self,String>{
        let mut file=File::open(path).map_err(|e|e.to_string())?;
        let mut magic=[0u8;8];file.read_exact(&mut magic).map_err(|e|e.to_string())?;
        if &magic!=MAGIC{return Err("Incompatible hybrid checkpoint".into());}
        let mut brain=Self::new();
        let old_count=read_u32(&mut file)? as usize;
        // v0.3 checkpoints had 128 positional embeddings; new checkpoints have
        // 256. Remap weights AND Adam moments without clearing learned state.
        if old_count!=brain.transformer.weights.len() && old_count!=24800 && old_count!=28896{
            return Err("Hybrid Transformer parameter count mismatch".into());
        }
        let mut step=[0u8;8];file.read_exact(&mut step).map_err(|e|e.to_string())?;
        brain.transformer.step=u64::from_le_bytes(step);
        let mut arrays=[vec![0.0;old_count],vec![0.0;old_count],vec![0.0;old_count]];
        for data in &mut arrays{read_floats(&mut file,data)?;}
        brain.transformer.import_compatible_arrays(&arrays[0],&arrays[1],&arrays[2])?;
        let num_dims=read_u32(&mut file)? as usize;
        if num_dims!=brain.dense.dimensions().len(){return Err("Dense topology mismatch".into());}
        let mut dims=Vec::new();
        for _ in 0..num_dims{dims.push(read_u32(&mut file)? as usize);}
        if dims!=brain.dense.dimensions(){return Err("Dense checkpoint dimensions mismatch".into());}
        brain.dense_adam.step=read_u32(&mut file)?;
        for layer in &mut brain.dense.layers{
            read_floats(&mut file,&mut layer.weights)?;
            read_floats(&mut file,&mut layer.biases)?;
        }
        for gradients in [&mut brain.dense_adam.m,&mut brain.dense_adam.v]{
            for layer in &mut gradients.layers{
                read_floats(&mut file,&mut layer.weights)?;
                read_floats(&mut file,&mut layer.biases)?;
            }
        }
        let mut tail=[0u8;8];
        let bytes=file.read(&mut tail).map_err(|e|e.to_string())?;
        if bytes>0{
            if bytes!=8||&tail!=b"WORDUNIT"{return Err("Unknown hybrid checkpoint trailer".into());}
            let count=read_u32(&mut file)? as usize;
            if count>crate::tokenizer::MAX_WORDS{return Err("Excessive tokenizer vocabulary".into());}
            for _ in 0..count{
                let size=read_u32(&mut file)? as usize;
                if !(3..=24).contains(&size){return Err("Bad word token length".into());}
                let mut data=vec![0u8;size];file.read_exact(&mut data).map_err(|e|e.to_string())?;
                brain.tokenizer.words.push(String::from_utf8(data).map_err(|e|e.to_string())?);
            }
            brain.tokenizer.validate()?;
            if file.read(&mut tail).map_err(|e|e.to_string())?!=0{
                return Err("Unexpected bytes after tokenizer".into());
            }
        }
        Ok(brain)
    }
}
impl Default for HybridBrain{fn default()->Self{Self::new()}}
fn write_u32(f:&mut File,x:u32)->Result<(),String>{f.write_all(&x.to_le_bytes()).map_err(|e|e.to_string())}
fn read_u32(f:&mut File)->Result<u32,String>{
    let mut x=[0u8;4];f.read_exact(&mut x).map_err(|e|e.to_string())?;
    Ok(u32::from_le_bytes(x))
}
fn write_floats(f:&mut File,data:&[f32])->Result<(),String>{
    for x in data{f.write_all(&x.to_le_bytes()).map_err(|e|e.to_string())?;}
    Ok(())
}
fn read_floats(f:&mut File,data:&mut [f32])->Result<(),String>{
    for x in data{
        let mut bytes=[0u8;4];f.read_exact(&mut bytes).map_err(|e|e.to_string())?;
        *x=f32::from_le_bytes(bytes);
        if !x.is_finite(){return Err("Non-finite hybrid checkpoint weight".into());}
    }
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn shared_loss_updates_both_hemispheres_and_bridge(){
        let mut brain=HybridBrain::new();
        let before_t=brain.transformer.weights.clone();
        let before_d=brain.dense.layers[0].weights.clone();
        let initial=brain.evaluate(&[(b"U: Hi\nN: ".to_vec(),b'H')]).unwrap().0;
        for _ in 0..4{
            brain.train_batch(&[(b"U: Hi\nN: ".to_vec(),b'H')],0.003).unwrap();
        }
        let after=brain.evaluate(&[(b"U: Hi\nN: ".to_vec(),b'H')]).unwrap().0;
        assert!(after<initial,"combined loss did not improve: {initial} -> {after}");
        assert_ne!(brain.transformer.weights,before_t,"Transformer did not learn");
        assert_ne!(brain.dense.layers[0].weights,before_d,"Dense network did not learn");
    }
    #[test]
    fn old_short_context_predictions_are_identical_in_new_window(){
        let model=HybridBrain::new();
        let q=b"U: hello\nN: ";
        let original=model.predict_window(q,128).unwrap();
        let extended=model.predict_window(q,256).unwrap();
        assert_eq!(original,extended);
    }
    #[test]
    fn old_v03_joint_checkpoint_migrates_without_changing_short_context_results(){
        let mut trained=HybridBrain::new();
        trained.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.001).unwrap();
        let prompt=b"U: hello\nN: ";
        let before=trained.predict_window(prompt,128).unwrap();
        let old_count=24800;
        let path=std::env::temp_dir().join(format!("nova-migrate-{}-{}.nhyb",
            std::process::id(),trained.transformer.step));
        let mut file=File::create(&path).unwrap();
        file.write_all(MAGIC).unwrap();
        write_u32(&mut file,old_count as u32).unwrap();
        file.write_all(&trained.transformer.step.to_le_bytes()).unwrap();
        for src in [&trained.transformer.weights,&trained.transformer.momentum,
                    &trained.transformer.variance]{
            // Legacy Transformer: embeddings [0..4096], position [4096..8192],
            // the remaining matrices immediately follow. New positions
            // occupy [4096..12288], with old 128 positions at the end.
            let mut old=vec![0.0;old_count];
            old[..4096].copy_from_slice(&src[..4096]);
            old[4096..8192].copy_from_slice(&src[8192..12288]);
            old[8192..].copy_from_slice(&src[12288..12288+(old_count-8192)]);
            write_floats(&mut file,&old).unwrap();
        }
        let dimensions=trained.dense.dimensions();
        write_u32(&mut file,dimensions.len() as u32).unwrap();
        for dim in dimensions{write_u32(&mut file,dim as u32).unwrap();}
        write_u32(&mut file,trained.dense_adam.step).unwrap();
        for layer in &trained.dense.layers{
            write_floats(&mut file,&layer.weights).unwrap();
            write_floats(&mut file,&layer.biases).unwrap();
        }
        for gradients in [&trained.dense_adam.m,&trained.dense_adam.v]{
            for layer in &gradients.layers{
                write_floats(&mut file,&layer.weights).unwrap();
                write_floats(&mut file,&layer.biases).unwrap();
            }
        }
        file.sync_all().unwrap();
        drop(file);
        let recovered=HybridBrain::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(recovered.predict_window(prompt,128).unwrap(),before);
        assert_eq!(recovered.transformer.step,trained.transformer.step);
        assert_eq!(recovered.dense_adam.step,trained.dense_adam.step);
        assert_eq!(recovered.dense.layers[0].weights,trained.dense.layers[0].weights);
        assert_eq!(recovered.dense_adam.m.layers[0].weights,trained.dense_adam.m.layers[0].weights);
    }
    #[test]
    fn neural_memory_changes_predictions_and_trains_shared_write_read_matrices(){
        let mut brain=HybridBrain::new();
        let context=b"U: What did I tell you?\nN: ".to_vec();
        let memory=vec!["U: My favorite fruit is apple.\nN: Apple is a fruit.".to_string()];
        let baseline=brain.predict_with_memory(&context,128,&memory).unwrap();
        let without=brain.predict_with_memory(&context,128,&[]).unwrap();
        assert_eq!(baseline,without,"Untrained memory should not perturb migrated dialogue");
        let before=brain.transformer.weights.clone();
        for _ in 0..3{
            brain.train_memory_batch(&[(context.clone(),b'A',memory.clone())],0.001).unwrap();
        }
        let after=&brain.transformer.weights;
        // End-to-end memory lessons update the trainable writer/read matrices.
        assert_ne!(&before[before.len()-2048..],&after[after.len()-2048..]);
        let with_memory=brain.predict_with_memory(&context,128,&memory).unwrap();
        let without_memory=brain.predict_with_memory(&context,128,&[]).unwrap();
        assert_ne!(with_memory,without_memory,"Memory must condition output after learning");
    }
    #[test]
    fn learned_words_and_optimizer_survive_joint_checkpoint_round_trip(){
        let mut brain=HybridBrain::new();
        brain.learn_token_units("Mira opens a box. Mira opens the box.");
        assert!(brain.tokenizer.words.len()>=2);
        let prompt=b"U: Mira opens a box.\nN: ";
        brain.train_batch(&[(prompt.to_vec(),b'H')],0.001).unwrap();
        let expected=brain.predict_with_memory(prompt,128,&[]).unwrap();
        let path=std::env::temp_dir().join(format!("nova-word-memory-{}.nhyb",std::process::id()));
        brain.save(&path).unwrap();
        let restored=HybridBrain::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(restored.tokenizer,brain.tokenizer);
        assert_eq!(restored.transformer.momentum,brain.transformer.momentum);
        assert_eq!(restored.predict_with_memory(prompt,128,&[]).unwrap(),expected);
    }
    #[test]
    fn saves_and_restores_entire_joint_optimizer_state(){
        let mut brain=HybridBrain::new();
        brain.train_batch(&[(b"Hello".to_vec(),b'!')],0.002).unwrap();
        let path=std::env::temp_dir().join(format!("nova-hybrid-{}.nhyb",std::process::id()));
        brain.save(&path).unwrap();
        let loaded=HybridBrain::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.transformer.weights,brain.transformer.weights);
        assert_eq!(loaded.transformer.momentum,brain.transformer.momentum);
        assert_eq!(loaded.transformer.variance,brain.transformer.variance);
        assert_eq!(loaded.dense.layers[0].weights,brain.dense.layers[0].weights);
        assert_eq!(loaded.dense_adam.step,brain.dense_adam.step);
        assert_eq!(loaded.dense_adam.m.layers[0].weights,brain.dense_adam.m.layers[0].weights);
    }
}
