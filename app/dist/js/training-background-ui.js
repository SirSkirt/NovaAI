"use strict";
(()=>{
const native=/Android/i.test(navigator.userAgent);
const invoke=(cmd,args)=>window.__TAURI__?.core?.invoke(cmd,args)
  ?? Promise.reject(Error("Tauri native service unavailable"));
let requested=false;
async function start(){
  if(!native)return;
  // Android may stop the native keepalive automatically between lessons.
  // Always inspect native state rather than trusting an old JS boolean.
  try{
    if(await invoke("plugin:background-service|is_running")){
      requested=true;return;
    }
    await invoke("plugin:background-service|start",{
      config:{
        serviceLabel:"Neural Nova: teaching in progress",
        foregroundServiceType:"specialUse"
      }
    });
    requested=true;
  }catch(e){
    const message=String(e);
    // If started concurrently from a previous lesson, keep it.
    if(/already running|alreadyRunning/i.test(message)){requested=true;return;}
    requested=false;
    throw new Error("Android foreground training service failed: "+message+
      ". Training has NOT been started. Open Android app settings and allow notifications.");
  }
}
async function stop(){
  if(!native)return;
  requested=false;
  try{await invoke("plugin:background-service|stop");}
  catch(e){
    if(!/not running|notRunning|not found/i.test(String(e)))
      console.warn("Neural Nova: failed to close foreground service",e);
  }
}
window.NovaTrainingBackground={start,stop,get native(){return native;}};
})();
