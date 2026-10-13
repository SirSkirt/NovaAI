"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const script=fs.readFileSync(path.join(__dirname,"../dist/js/mass-learning-ui.js"),"utf8");
function harness(android){
  const nodes=new Map();
  const get=id=>{
    if(!nodes.has(id))nodes.set(id,{
      id,hidden:false,style:{},disabled:false,checked:false,value:"",
      files:[],textContent:"",scrollHeight:0,scrollTop:0,
      callbacks:{},addEventListener(event,handler){this.callbacks[event]=handler;}
    });
    return nodes.get(id);
  };
  get("lessonMode").value="text";
  get("trainingEpochs").value="3";
  get("massLessonsLog").textContent="No bulk lessons loaded.";
  let webReads=0,nativeReads=0,pickerCalls=0,validation=null;
  const selected={name:"01_lessons.zip",size:4,async arrayBuffer(){
    webReads++;return Uint8Array.from([80,75,3,4]).buffer;
  }};
  get("massLessonZip").files=android?[]:[selected];
  const window={
    __TAURI__:{
      core:{async invoke(command,args){
        if(command!=="hybrid_zip_preview")throw Error("Unexpected "+command);
        validation=args;
        return {lessons:[{name:"01_test.txt",trainingSamples:24,heldoutSamples:6}],
          mode:"text",totalBytes:192};
      }},
      dialog:{async open(){pickerCalls++;return "content://provider/document/zip";}},
      fs:{async readFile(){nativeReads++;return Uint8Array.from([80,75,3,4]);}}
    }
  };
  const context={window,document:{getElementById:get},
    navigator:{userAgent:android?"Mozilla/5.0 (Linux; Android 16; SM-S938W) AppleWebKit/537.36":"Windows Chrome"},
    console,confirm:()=>true};
  vm.runInNewContext(script,context,{filename:"mass-learning-ui.js"});
  return {get,window,selected,stats:()=>({webReads,nativeReads,pickerCalls,validation})};
}
(async()=>{
  const mobile=harness(true);
  assert.equal(mobile.get("browserMassLessonPicker").style.display,"none");
  assert.equal(mobile.get("nativeMassLessonZip").hidden,false);
  await mobile.get("nativeMassLessonZip").callbacks.click();
  assert.equal(mobile.stats().nativeReads,1);
  assert.equal(mobile.stats().webReads,0);
  assert.match(mobile.get("massLessonsStatus").textContent,/read successfully/);
  await mobile.get("previewMassLessons").callbacks.click();
  assert.deepEqual(Array.from(mobile.stats().validation.bytes),[80,75,3,4]);
  assert.equal(mobile.get("startMassLessons").disabled,false);
  assert.equal(mobile.stats().webReads,0,"native Android picker must bypass File.arrayBuffer");
  const desktop=harness(false);
  await desktop.get("previewMassLessons").callbacks.click();
  assert.equal(desktop.stats().webReads,1);
  assert.equal(desktop.stats().pickerCalls,0);
  assert.equal(desktop.stats().nativeReads,0);
  console.log("PASS: Android ZIP reads via native content URI; desktop stays with browser file input.");
})().catch(e=>{console.error(e);process.exitCode=1;});
