//! Bounded, chunked checkpoint transfer. Does not parse or change neural weights.
//! Imported brains are validated by HybridBrain::load before committing.
use std::sync::Mutex;
const MAX_CHECKPOINT_BYTES:usize=24*1024*1024;
const CHUNK:usize=256*1024;
#[derive(Default)]
pub struct TransferState(pub Mutex<Transfer>);
#[derive(Default)]
pub struct Transfer{
    pub export:Vec<u8>,
    pub import:Vec<u8>,
    pub expected:usize,
    pub importing:bool,
}
impl TransferState {
    pub fn begin_export(&self,contents:Vec<u8>)->Result<usize,String>{
        if contents.is_empty()||contents.len()>MAX_CHECKPOINT_BYTES{
            return Err("Checkpoint export is empty or exceeds 24 MiB".into());
        }
        let mut state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        state.export=contents;
        Ok(state.export.len())
    }
    pub fn chunk(&self,offset:usize)->Result<Vec<u8>,String>{
        let state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        if state.export.is_empty(){return Err("Start checkpoint export first".into());}
        if offset>state.export.len(){return Err("Checkpoint read offset outside export".into());}
        Ok(state.export[offset..(offset+CHUNK).min(state.export.len())].to_vec())
    }
    pub fn import_begin(&self,expected:usize)->Result<(),String>{
        if expected==0||expected>MAX_CHECKPOINT_BYTES{
            return Err("Checkpoint must be 1 byte–24 MiB".into());
        }
        let mut state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        state.import=Vec::new();state.expected=expected;state.importing=true;
        Ok(())
    }
    pub fn import_append(&self,chunk:Vec<u8>)->Result<usize,String>{
        let mut state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        if !state.importing{return Err("Start checkpoint import first".into());}
        if chunk.is_empty()||chunk.len()>CHUNK || state.import.len()+chunk.len()>state.expected{
            return Err("Invalid checkpoint import chunk or size".into());
        }
        state.import.extend_from_slice(&chunk);
        Ok(state.import.len())
    }
    pub fn import_read(&self)->Result<Vec<u8>,String>{
        let state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        if !state.importing||state.import.len()!=state.expected{
            return Err("Incomplete checkpoint import".into());
        }
        Ok(state.import.clone())
    }
    pub fn cancel(&self)->Result<(),String>{
        let mut state=self.0.lock().map_err(|_|"Checkpoint transfer unavailable")?;
        state.import.clear();state.expected=0;state.importing=false;
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn blocks_oversize_or_partial_imports(){
        let x=TransferState::default();
        assert!(x.import_begin(MAX_CHECKPOINT_BYTES+1).is_err());
        x.import_begin(4).unwrap();
        assert_eq!(x.import_append(vec![1,2]).unwrap(),2);
        assert!(x.import_read().is_err());
        assert_eq!(x.import_append(vec![3,4]).unwrap(),4);
        assert_eq!(x.import_read().unwrap(),vec![1,2,3,4]);
        x.cancel().unwrap();
        assert!(x.import_read().is_err());
    }
    #[test]fn export_is_chunked_and_immutable(){
        let x=TransferState::default();
        assert_eq!(x.begin_export(vec![1,2,3]).unwrap(),3);
        assert_eq!(x.chunk(0).unwrap(),vec![1,2,3]);
        assert_eq!(x.chunk(3).unwrap(),Vec::<u8>::new());
    }
}
