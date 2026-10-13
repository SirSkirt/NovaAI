//! Android-only foreground keepalive for the *existing* Rust training worker.
//! A foreground service does not do the training; it keeps the process eligible
//! for continued execution while the training worker is running.
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tauri::{Manager, Runtime};
use tauri_plugin_background_service::{BackgroundService,ServiceContext,ServiceError};
use crate::hybrid_lab::HybridState;
use crate::lesson_zip::ZipQueueState;

pub struct TrainingKeepalive;
#[async_trait]
impl<R:Runtime> BackgroundService<R> for TrainingKeepalive {
    async fn init(&mut self,_ctx:&ServiceContext<R>)->Result<(),ServiceError>{Ok(())}
    async fn run(&mut self,ctx:&ServiceContext<R>)->Result<(),ServiceError>{
        let mut observed_training=false;
        let launched=Instant::now();
        // Foreground promotion can briefly race the first ZIP-lesson change.
        // Recheck the actual service notification until Android accepts it.
        let mut last_displayed:Option<(usize,String)>=None;
        let mut last_attempt=Instant::now()-Duration::from_secs(10);
        loop{
            tokio::select!{
                _=ctx.shutdown.cancelled()=>break,
                _=tokio::time::sleep(Duration::from_secs(2))=>{
                    let running=ctx.app.state::<HybridState>().is_training()
                        || ctx.app.state::<ZipQueueState>().is_active();
                    if let Some((name,index,total))=
                        ctx.app.state::<ZipQueueState>().current_lesson(){
                        let current=(index,name.clone());
                        if last_displayed.as_ref()!=Some(&current) &&
                            last_attempt.elapsed()>=Duration::from_secs(8){
                            last_attempt=Instant::now();
                            if crate::android_lesson_notification::show_zip_lesson(
                                &name,index,total){
                                last_displayed=Some(current);
                            }
                        }
                    }
                    if running {observed_training=true;}
                    if observed_training&&!running {break;}
                    // A rejected lesson or cancelled file picker must not leave
                    // a never-ending foreground notification.
                    if !observed_training&&launched.elapsed()>Duration::from_secs(40){break;}
                }
            }
        }
        Ok(())
    }
}
