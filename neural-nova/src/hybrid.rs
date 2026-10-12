//! One Neural Nova: dense association pathway -> learned bridge -> Transformer decoder.
//! Both pathways and the bridge receive gradients from the SAME next-token loss.
//! No independent answer selection, no heuristic replies, no ML frameworks.
use crate::{encode_ascii_context,Adam,Gradients,Network,NetworkConfig};
use crate::transformer::{Transformer,Attention};
use std::fs::{self,File};
use std::io::{Read,Write};
use std::path::Path;

const MAGIC:&[u8;8]=b"NHYB0300";

pub struct HybridBrain {
    pub transformer:Transformer,
    pub dense:Network,
    pub dense_adam:Adam,
}
impl HybridBrain {
    pub fn new()->Self {
        let dense=Network::new(NetworkConfig::neural_nova_v1()).expect("valid hybrid dimensions");
        let dense_adam=Adam::new(&dense,0.001).expect("valid learning rate");
        Self{transformer:Transformer::new(0x4e4f5641_0300),dense,dense_adam}
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
        let (_,dense)=self.dense_input(context)?;
        self.transformer.predict_hybrid(context,&dense)
    }
    pub fn inspect(&self,context:&[u8])->Result<(Vec<Vec<f32>>,Vec<Attention>,Vec<f32>,Vec<f32>),String>{
        let (features,dense)=self.dense_input(context)?;
        let layers=self.dense.activations(&features)?;
        let attention=self.transformer.attention_hybrid(context,&dense)?;
        let signal=self.transformer.bridge_signal(&dense)?;
        let probabilities=self.transformer.predict_hybrid(context,&dense)?;
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
            let (item_loss,transformer_gradient,upstream)=
                self.transformer.gradient_with_dense(context,*target,Some(&logits))?;
            let dense_gradient=self.dense.gradients_from_logits(&input,&upstream)?;
            loss+=item_loss;
            for (a,b) in tf_grad.iter_mut().zip(transformer_gradient){*a+=b;}
            for (a,b) in dense_grad.layers.iter_mut().zip(dense_gradient.layers) {
                for (x,y) in a.weights.iter_mut().zip(b.weights){*x+=y;}
                for (x,y) in a.biases.iter_mut().zip(b.biases){*x+=y;}
            }
        }
        let inverse=1.0/samples.len() as f32;
        for value in &mut tf_grad{*value*=inverse;}
        dense_grad.scale(inverse);
        dense_grad.clip_global_norm(1.0);
        self.transformer.apply_gradients(&mut tf_grad,lr)?;
        self.dense_adam.apply(&mut self.dense,&dense_grad)?;
        Ok(loss*inverse)
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
        if read_u32(&mut file)? as usize!=brain.transformer.weights.len(){
            return Err("Hybrid Transformer parameter count mismatch".into());
        }
        let mut step=[0u8;8];file.read_exact(&mut step).map_err(|e|e.to_string())?;
        brain.transformer.step=u64::from_le_bytes(step);
        for data in [&mut brain.transformer.weights,&mut brain.transformer.momentum,&mut brain.transformer.variance]{
            read_floats(&mut file,data)?;
        }
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
        let mut tail=[0u8;1];
        if file.read(&mut tail).map_err(|e|e.to_string())?!=0{
            return Err("Unexpected data in hybrid checkpoint".into());
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
