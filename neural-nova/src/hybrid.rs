//! One Neural Nova: trainable reciprocal Transformer <-> dense hemisphere exchange.
//! Both pathways and the bridge receive gradients from the SAME next-token loss.
//! No independent answer selection, no heuristic replies, no ML frameworks.
use crate::{encode_ascii_context,Adam,Gradients,Network,NetworkConfig};
use crate::transformer::{Transformer,Attention,BridgeMeasurement};
use crate::tokenizer::Tokenizer;
use std::fs::{self,File};
use std::io::{Read,Write};
use std::path::Path;

const MAGIC:&[u8;8]=b"NHYB0300";

#[derive(Clone,Debug)]
pub struct BatchGradientDiagnostics{
    pub transformer_norm:f32,pub dense_norm:f32,pub bridge_norm:f32,
    pub transformer_clipped:bool,pub dense_clipped:bool,
}
#[derive(Clone)]
pub struct HybridBrain {
    pub transformer:Transformer,
    pub dense:Network,
    pub dense_adam:Adam,
    pub tokenizer:Tokenizer,
    pub last_batch_diagnostics:Option<BatchGradientDiagnostics>,
}
impl HybridBrain {
    pub fn new()->Self {
        let dense=Network::new(NetworkConfig::neural_nova_v1()).expect("valid hybrid dimensions");
        let dense_adam=Adam::new(&dense,0.001).expect("valid learning rate");
        Self{transformer:Transformer::new(0x4e4f5641_0300),dense,dense_adam,
            tokenizer:Tokenizer::new(),last_batch_diagnostics:None}
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
        self.predict_ablation_with_memory(context,window,memory,false,false)
    }
    /// Evaluation-only ablations; neither changes weights, moments or checkpoint.
    /// With dense_off, both original bridge and reciprocal exchanges are absent.
    /// With exchange_off, the original fused v0.10 pathway remains active.
    pub fn predict_ablation_with_memory(&self,context:&[u8],window:usize,
        memory:&[String],dense_off:bool,exchange_off:bool)->Result<Vec<f32>,String>{
        let (base,dense)=self.dense_input(context)?;
        let tokens=self.tokenizer.encode(
            std::str::from_utf8(context).map_err(|e|e.to_string())?)?;
        let memory_tokens=memory.iter().map(|s|self.tokenizer.encode(s))
            .collect::<Result<Vec<_>,_>>()?;
        if dense_off{
            self.transformer.predict_tokenized(&tokens,&[0.0;128],window,&memory_tokens)
        }else if exchange_off{
            self.transformer.predict_tokenized(&tokens,&dense,window,&memory_tokens)
        }else{
            self.transformer.predict_cooperative(&tokens,&dense,window,
                &memory_tokens,&self.dense,&base)
        }
    }
    /// Internal exchange diagnostics; do not alter the existing brain visualizer.
    pub fn exchange_activity(&self,context:&[u8])->Result<Vec<f32>,String>{
        let (base,dense)=self.dense_input(context)?;
        let tokens=self.tokenizer.encode(
            std::str::from_utf8(context).map_err(|e|e.to_string())?)?;
        self.transformer.cooperative_activity(&tokens,&dense,256,&self.dense,&base)
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
        let probabilities=self.transformer.predict_cooperative(&tokens,&dense,window,&source,&self.dense,&features)?;
        Ok((layers,attention,signal,probabilities))
    }
    /// Read-only recurrent state measurements. No extra inference branch.
    pub fn reasoning_activity(&self,context:&[u8])->Result<Vec<f32>,String>{
        let (_,dense)=self.dense_input(context)?;
        let text=std::str::from_utf8(context).map_err(|e|e.to_string())?;
        let tokens=self.tokenizer.encode(text)?;
        self.transformer.reasoning_activity(&tokens,&dense,&[])
    }
    pub fn bridge_measurement(&self,context:&[u8],window:usize)->Result<BridgeMeasurement,String>{
        let (_,features)=self.dense_input(context)?;
        let tokens=self.tokenizer.encode(
            std::str::from_utf8(context).map_err(|e|e.to_string())?)?;
        self.transformer.measure_bridge(&tokens,&features,window)
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
                self.transformer.gradient_tokens_cooperative(&tokens,*target,&logits,&[],
                    &self.dense,&input,&mut dense_grad)?;
            self.dense.accumulate_logits_gradients(&input,&upstream,&mut dense_grad)?;
            loss+=item_loss;
            for (a,b) in tf_grad.iter_mut().zip(transformer_gradient){*a+=b;}
        }
        let inverse=1.0/samples.len() as f32;
        for value in &mut tf_grad{*value*=inverse;}
        dense_grad.scale(inverse);
        let transformer_norm=(tf_grad.iter().map(|&g|(g as f64)*(g as f64)).sum::<f64>()).sqrt() as f32;
        let dense_norm=dense_grad.global_norm();
        let bridge_norm=self.transformer.bridge_gradient_norm(&tf_grad)?;
        self.last_batch_diagnostics=Some(BatchGradientDiagnostics{
            transformer_norm,dense_norm,bridge_norm,
            transformer_clipped:transformer_norm>1.0,dense_clipped:dense_norm>1.0,
        });
        dense_grad.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut tf_grad,lr)?;
        self.dense_adam.set_learning_rate(lr)?;
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
            let (value,g,upstream)=self.transformer.gradient_tokens_cooperative(
                &tokens,*target,&logits,&memory,&self.dense,&input,&mut dense_grad)?;
            loss+=value;
            self.dense.accumulate_logits_gradients(&input,&upstream,&mut dense_grad)?;
            for (a,b) in tf_grad.iter_mut().zip(g){*a+=b;}
        }
        let inv=1.0/samples.len() as f32;
        for x in &mut tf_grad{*x*=inv;}
        dense_grad.scale(inv);
        let transformer_norm=(tf_grad.iter().map(|&g|(g as f64)*(g as f64)).sum::<f64>()).sqrt() as f32;
        let dense_norm=dense_grad.global_norm();
        let bridge_norm=self.transformer.bridge_gradient_norm(&tf_grad)?;
        self.last_batch_diagnostics=Some(BatchGradientDiagnostics{
            transformer_norm,dense_norm,bridge_norm,
            transformer_clipped:transformer_norm>1.0,dense_clipped:dense_norm>1.0,
        });
        dense_grad.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut tf_grad,lr)?;
        self.dense_adam.set_learning_rate(lr)?;
        self.dense_adam.apply(&mut self.dense,&dense_grad)?;
        Ok(loss*inv)
    }

    /// A sequence-level preference objective, not ordinary one-character
    /// teacher forcing. It compares the joint model's likelihood of complete
    /// competing conclusions. Gradients reach BOTH networks and their bridge.
    /// Chat inference continues to be generated by the same one brain.
    fn answer_score_gradient(&self,context:&[u8],answer:&[u8])
        ->Result<(f32,Vec<f32>,Gradients),String>{
        if answer.is_empty()||answer.len()>96 || !answer.is_ascii(){
            return Err("Reasoning candidates must contain 1–96 ASCII characters".into());
        }
        let mut sequence=context.to_vec();
        let mut gradients=vec![0.0;self.transformer.parameter_count()];
        let mut dense_grad=Gradients::zeros(&self.dense);
        let mut total_loss=0.0f32;
        let mut count=0usize;
        for target in answer.iter().copied().chain(std::iter::once(b'\n')){
            let (input,features)=self.dense_input(&sequence)?;
            let tokens=self.tokenizer.encode(std::str::from_utf8(&sequence).map_err(|e|e.to_string())?)?;
            let (loss,tf,upstream)=self.transformer.gradient_tokens_cooperative(
                &tokens,target,&features,&[],&self.dense,&input,&mut dense_grad)?;
            self.dense.accumulate_logits_gradients(&input,&upstream,&mut dense_grad)?;
            for (sum,g) in gradients.iter_mut().zip(tf){*sum+=g;}
            total_loss+=loss;count+=1;
            sequence.push(target);
            if sequence.len()>512{sequence.drain(..sequence.len()-512);}
        }
        let scale=1.0/count as f32;
        for v in &mut gradients{*v*=scale;}
        dense_grad.scale(scale);
        Ok((-total_loss*scale,gradients,dense_grad))
    }
    fn answer_score(&self,prompt:&[u8],answer:&[u8])->Result<f32,String>{
        if answer.is_empty()||answer.len()>96||!answer.is_ascii(){
            return Err("Reasoning candidate must have 1–96 ASCII characters".into());
        }
        let mut prefix=prompt.to_vec();
        let mut total=0.0f32;
        for c in answer.iter().copied().chain(std::iter::once(b'\n')){
            let probs=self.predict_window(&prefix,256)?;
            total+=probs[c as usize].max(1e-30).ln();
            prefix.push(c);
            if prefix.len()>512{prefix.drain(..prefix.len()-512);}
        }
        Ok(total/(answer.len()+1) as f32)
    }
    pub fn reasoning_margin(&self,prompt:&[u8],preferred:&[u8],rejected:&[u8])
        ->Result<(f32,bool),String>{
        let good=self.answer_score(prompt,preferred)?;
        let bad=self.answer_score(prompt,rejected)?;
        let delta=bad-good;
        Ok((delta.max(0.0)+(1.0+(-delta.abs()).exp()).ln(),good>bad))
    }
    pub fn train_preference_batch(&mut self,
        samples:&[(Vec<u8>,Vec<u8>,Vec<u8>)],learning_rate:f32)->Result<f32,String>{
        if samples.is_empty() || samples.len()>4 {
            return Err("Reasoning batch requires 1–4 comparisons".into());
        }
        if !learning_rate.is_finite() || !(1e-6..=0.05).contains(&learning_rate){
            return Err("Reasoning learning rate outside supported range".into());
        }
        let mut combined_tf=vec![0.0;self.transformer.parameter_count()];
        let mut combined_dense=Gradients::zeros(&self.dense);
        let mut total=0.0f32;
        for (prompt,preferred,rejected) in samples{
            let (good,gp,dp)=self.answer_score_gradient(prompt,preferred)?;
            let (bad,gn,dn)=self.answer_score_gradient(prompt,rejected)?;
            let gap=bad-good;
            // Stable softplus and sigmoid. Ranking loss = softplus(bad-good).
            let weight=if gap>=0.0{1.0/(1.0+(-gap).exp())}
                else{gap.exp()/(1.0+gap.exp())};
            total+=gap.max(0.0)+(1.0+(-gap.abs()).exp()).ln();
            // ∂loss = sigmoid(bad-good) * (∂negLogGood - ∂negLogBad).
            for ((sum,pos),neg) in combined_tf.iter_mut().zip(gp).zip(gn){
                *sum+=weight*(pos-neg);
            }
            let mut delta=dp;
            delta.add_scaled(&dn,-1.0);
            combined_dense.add_scaled(&delta,weight);
        }
        let scale=1.0/samples.len() as f32;
        for val in &mut combined_tf{*val*=scale;}
        combined_dense.scale(scale);
        let transformer_norm=(combined_tf.iter().map(|x|(*x as f64).powi(2)).sum::<f64>()).sqrt() as f32;
        let dense_norm=combined_dense.global_norm();
        let bridge_norm=self.transformer.bridge_gradient_norm(&combined_tf)?;
        self.last_batch_diagnostics=Some(BatchGradientDiagnostics{
            transformer_norm,dense_norm,bridge_norm,
            transformer_clipped:transformer_norm>1.0,dense_clipped:dense_norm>1.0,
        });
        combined_dense.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut combined_tf,learning_rate)?;
        self.dense_adam.set_learning_rate(learning_rate)?;
        self.dense_adam.apply(&mut self.dense,&combined_dense)?;
        Ok(total*scale)
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
        if old_count!=brain.transformer.weights.len() &&
            old_count!=Transformer::previous_parameter_count() &&
            old_count!=Transformer::precooperation_parameter_count() &&
            old_count!=24800 && old_count!=28896{
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
    /// Simulate the actual pre-recurrent checkpoint: all old weights and
    /// optimizer moments are preserved, new reasoning projection is neutral.
    fn emulate_pre_reasoning_checkpoint(brain:&mut HybridBrain){
        let old=Transformer::previous_parameter_count();
        let mut previous=Transformer::new(11);
        previous.step=brain.transformer.step;
        previous.import_compatible_arrays(&brain.transformer.weights[..old],
            &brain.transformer.momentum[..old],
            &brain.transformer.variance[..old]).unwrap();
        brain.transformer=previous;
    }
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
    fn whole_answer_preference_updates_both_existing_networks(){
        let mut brain=HybridBrain::new();
        let prefix=b"U: The box is red. What color is it?\nN: ".to_vec();
        let good=b"Red.".to_vec();let bad=b"Blue.".to_vec();
        let prior_tf=brain.transformer.weights.clone();
        let prior_dense=brain.dense.layers[0].weights.clone();
        let before_tf=brain.transformer.step;
        let result=brain.train_preference_batch(&[(prefix.clone(),good.clone(),bad.clone())],0.0001).unwrap();
        assert!(result.is_finite());
        assert_eq!(brain.transformer.step,before_tf+1);
        assert_eq!(brain.dense_adam_steps(),1);
        assert_ne!(brain.transformer.weights,prior_tf);
        assert_ne!(brain.dense.layers[0].weights,prior_dense);
        let (loss,_)=brain.reasoning_margin(&prefix,&good,&bad).unwrap();
        assert!(loss.is_finite());
    }
    #[test]
    fn preference_training_updates_the_shared_recurrent_state_not_a_second_brain(){
        let mut brain=HybridBrain::new();
        let prompt=b"U: The copper box contains a stone. What is in the copper box?\nN: ".to_vec();
        let good=b"A stone.".to_vec();
        let bad=b"An apple.".to_vec();
        let initially=brain.reasoning_activity(&prompt).unwrap();
        let tf_start=brain.transformer.weights.clone();
        let dense_start=brain.dense.layers[0].weights.clone();
        let step=brain.transformer.step;
        for _ in 0..2{
            brain.train_preference_batch(&[(prompt.clone(),good.clone(),bad.clone())],
                0.0003).unwrap();
        }
        assert_eq!(brain.transformer.step,step+2);
        assert_ne!(brain.transformer.weights,tf_start);
        assert_ne!(brain.dense.layers[0].weights,dense_start);
        let activity=brain.reasoning_activity(&prompt).unwrap();
        assert_eq!(activity.len(),crate::transformer::REASONING_STEPS);
        assert!(initially.iter().all(|value|*value==0.0));
        assert!(activity.iter().any(|value|*value>0.0));
        assert!(brain.reasoning_margin(&prompt,&good,&bad).unwrap().0.is_finite());
    }
    #[test]
    fn pre_reasoning_joint_checkpoint_import_preserves_predictions_optimizer_and_tokenizer(){
        let mut brain=HybridBrain::new();
        brain.learn_token_units("hello hello morning morning");
        for _ in 0..2{
            brain.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.001).unwrap();
        }
        emulate_pre_reasoning_checkpoint(&mut brain);
        let prompt=b"U: hello\nN: ";
        let before=brain.predict_window(prompt,256).unwrap();
        let old_count=Transformer::previous_parameter_count();
        let path=std::env::temp_dir().join(format!("nova-v08-reason-migration-{}-{}.nhyb",
            std::process::id(),brain.transformer.step));
        let mut file=File::create(&path).unwrap();
        file.write_all(MAGIC).unwrap();
        write_u32(&mut file,old_count as u32).unwrap();
        file.write_all(&brain.transformer.step.to_le_bytes()).unwrap();
        for array in [&brain.transformer.weights,&brain.transformer.momentum,&brain.transformer.variance]{
            write_floats(&mut file,&array[..old_count]).unwrap();
        }
        let dimensions=brain.dense.dimensions();
        write_u32(&mut file,dimensions.len() as u32).unwrap();
        for d in dimensions{write_u32(&mut file,d as u32).unwrap();}
        write_u32(&mut file,brain.dense_adam.step).unwrap();
        for layer in &brain.dense.layers{
            write_floats(&mut file,&layer.weights).unwrap();
            write_floats(&mut file,&layer.biases).unwrap();
        }
        for moments in [&brain.dense_adam.m,&brain.dense_adam.v]{
            for layer in &moments.layers{
                write_floats(&mut file,&layer.weights).unwrap();
                write_floats(&mut file,&layer.biases).unwrap();
            }
        }
        file.write_all(b"WORDUNIT").unwrap();
        write_u32(&mut file,brain.tokenizer.words.len() as u32).unwrap();
        for word in &brain.tokenizer.words{
            write_u32(&mut file,word.len() as u32).unwrap();
            file.write_all(word.as_bytes()).unwrap();
        }
        file.sync_all().unwrap();drop(file);
        let restored=HybridBrain::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(restored.predict_window(prompt,256).unwrap(),before);
        assert_eq!(restored.transformer.step,brain.transformer.step);
        assert_eq!(restored.tokenizer.words,brain.tokenizer.words);
        assert_eq!(restored.dense_adam.step,brain.dense_adam.step);
        assert_eq!(restored.transformer.momentum[..old_count],
            brain.transformer.momentum[..old_count]);
        assert_eq!(restored.transformer.variance[..old_count],
            brain.transformer.variance[..old_count]);
        assert_eq!(restored.dense.layers[0].weights,brain.dense.layers[0].weights);
        let before_training=restored.reasoning_activity(prompt).unwrap();
        assert_eq!(before_training,vec![0.0;crate::transformer::REASONING_STEPS]);
    }
    #[test]
    fn old_v03_joint_checkpoint_migrates_without_changing_short_context_results(){
        let mut trained=HybridBrain::new();
        trained.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.001).unwrap();
        emulate_pre_reasoning_checkpoint(&mut trained);
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
    fn tokenizer_growth_is_a_no_weight_update_representation_change(){
        let brain=HybridBrain::new();
        let mut other=brain.clone();
        let prompt=b"U: hello\nN: ";
        let tokens_before=brain.tokenizer.encode(std::str::from_utf8(prompt).unwrap()).unwrap();
        let original=brain.predict_window(prompt,128).unwrap();
        let added=other.learn_token_units("hello hello planet planet");
        let tokens_after=other.tokenizer.encode(std::str::from_utf8(prompt).unwrap()).unwrap();
        let changed=other.predict_window(prompt,128).unwrap();
        assert_eq!(added,2);
        assert_ne!(tokens_before,tokens_after,
            "Introducing a learned whole word replaces previously used character inputs");
        assert_eq!(brain.transformer.weights,other.transformer.weights,
            "Vocabulary growth alone must not secretly train or alter neural weights");
        assert_eq!(brain.transformer.step,other.transformer.step);
        assert_ne!(original,changed,
            "New tokenization must be tracked because it can perturb next-character probabilities");
    }
    #[test]
    fn both_hemispheres_honor_the_selected_learning_rate(){
        let mut brain=HybridBrain::new();
        brain.train_batch(&[(b"U: hello\nN: ".to_vec(),b'H')],0.003).unwrap();
        assert_eq!(brain.dense_adam.learning_rate(),0.003);
        brain.train_memory_batch(&[(
            b"U: hello\nN: ".to_vec(),b'H',vec!["U: hi\nN: Hey".to_string()]
        )],0.001).unwrap();
        assert_eq!(brain.dense_adam.learning_rate(),0.001);
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
