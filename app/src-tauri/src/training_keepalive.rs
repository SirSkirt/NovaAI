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
        loop{
            tokio::select!{
                _=ctx.shutdown.cancelled()=>break,
                _=tokio::time::sleep(Duration::from_secs(2))=>{
                    let running=ctx.app.state::<HybridState>().is_training()
                        || ctx.app.state::<ZipQueueState>().is_active();
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
