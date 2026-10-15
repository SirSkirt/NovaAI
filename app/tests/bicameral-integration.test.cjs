"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path"),vm=require("node:vm");
const root=path.resolve(__dirname,"..");const read=p=>fs.readFileSync(path.join(root,p),"utf8");
const ui=read("dist/index.html");
const main=read("dist/js/nova2-main.js"),tests=read("dist/js/nova2-test-ui.js");
const native=read("src-tauri/src/lib.rs"),runtime=read("src-tauri/src/nova2_lab.rs");
const education=read("src-tauri/src/nova2_tests.rs"),keepalive=read("src-tauri/src/training_keepalive.rs");
const neural=read("../neural-nova/src/nova2/mod.rs"),cache=read("../neural-nova/src/nova2/cache.rs");
const assertIncludes=(source,pattern)=>assert.match(source,pattern);
assertIncludes(ui,/id="pane-teach"/);
assert.doesNotMatch(ui,/id="pane-nova2"/);
assert.doesNotMatch(ui,/Legacy Teach/);
assertIncludes(ui,/v0\.19\.1 · Bicameral/);
assertIncludes(ui,/src="\.\/js\/nova2-main\.js"/);
assertIncludes(ui,/src="\.\/js\/nova2-test-ui\.js"/);
assert.doesNotMatch(ui,/src="\.\/js\/nova-ui\.js"/);
assert.doesNotMatch(ui,/src="\.\/js\/nova2-ui\.js"/);
assert.doesNotMatch(ui,/\.nhyb/);
assertIncludes(main,/nova2_boot/);
assertIncludes(main,/nova2_train/);
assertIncludes(main,/nova2_zip_train/);
assertIncludes(main,/nova2_targeted_train/);
assertIncludes(main,/nova2_targeted_accept/);
assertIncludes(main,/nova2_chat/);
assertIncludes(main,/nova2_checkpoint_export_begin/);
assertIncludes(main,/nova2_checkpoint_import_finish/);
assertIncludes(main,/nova2_targeted_stage/);
assertIncludes(main,/foundationTraining:\$\("foundationTrainingEnabled"\)\.checked/);
assertIncludes(tests,/nova2_test_start/);
assertIncludes(tests,/nova2_test_review/);
assertIncludes(education,/Baseline all questions and all generalization cases BEFORE any gradient update/);
assertIncludes(education,/brain\.reserve_external\(\)/);
assertIncludes(native,/manage\(Nova2State::default\(\)\)/);
assertIncludes(native,/manage\(Nova2TestState::default\(\)\)/);
assert.doesNotMatch(native,/manage\(HybridState::default\(\)\)/);
assert.doesNotMatch(native,/manage\(TargetedState::default\(\)\)/);
assert.doesNotMatch(native,/manage\(NeuralState::default\(\)\)/);
assertIncludes(native,/nova2_tests::nova2_test_start/);
assertIncludes(native,/nova2_lab::nova2_targeted_train/);
assertIncludes(keepalive,/state::<Nova2State>\(\)\.is_training\(\)/);
assert.doesNotMatch(keepalive,/state::<HybridState>/);
assertIncludes(runtime,/const CHECKPOINT:&str="neural-nova-bicameral-v2\.nnova2"/);
assertIncludes(runtime,/Nova2::load\(&p\)/);
assertIncludes(runtime,/fn focus_name\(f:Focus\)/);
assertIncludes(runtime,/Focus::Right,Focus::Left,Focus::Interconnect,Focus::Joint/);
assertIncludes(runtime,/KvCache::new/);
assertIncludes(neural,/pub const MAX_PARAMETERS:usize=1_200_144/);
assertIncludes(neural,/vocab:384,d:128,heads:4,ff:288,blocks:4/);
assertIncludes(cache,/cached_matches_full_logits_across_both_thinking_rounds/);
const ids=[...main.matchAll(/\$\("([^"]+)"\)/g)].map(x=>x[1]);
for(const id of new Set(ids))assert.ok(ui.includes('id="'+id+'"'),"Missing main-workflow DOM id: "+id);
const testIds=[...tests.matchAll(/\$\("([^"]+)"\)/g)].map(x=>x[1]);
for(const id of new Set(testIds))assert.ok(ui.includes('id="'+id+'"'),"Missing teacher-test DOM id: "+id);
// Fake browser boot proves no hidden invocation of the legacy brain.
const dom=new Map();const commands=[];const started=[];
function element(id){if(!dom.has(id))dom.set(id,{
  id,hidden:false,disabled:false,style:{},textContent:"",value:"",checked:false,
  files:[],scrollHeight:0,scrollTop:0,replaceChildren(){},
  listeners:{},addEventListener(name,fn){this.listeners[name]=fn;},
  click(){return this.listeners.click?.();},focus(){},
  get closest(){return ()=>({open:false});},
});return dom.get(id);}
const browser={
  console,Uint8Array,Blob,TextDecoder,Number,String,Math,Array,Promise,Error,
  localStorage:{getItem(){return null;},setItem(){}},
  confirm(){return true;},navigator:{userAgent:"Windows"},
  setInterval(){},setTimeout(){},
  document:{getElementById:element,querySelectorAll(){return [];},
    dispatchEvent(){},createElement(){return {append(){},style:{}};}},
  window:{NovaTrainingBackground:{async start(){},async stop(){}},
    __TAURI__:{event:{async listen(){}},core:{async invoke(name,args){
      commands.push(name);
      if(name==="nova2_boot")return{
        parameters:1182144,transformerParameters:755200,denseParameters:336320,
        interconnectParameters:90624,step:15_000,words:80,stage:"Ready",
        training:false,trained:false,heldoutLoss:null,total:0,candidateReady:false
      };
      if(name==="nova2_conversation")return [];
      if(name==="nova2_zip_preview")return {lessons:[{name:"01_foundations.txt",mode:"unified"}],mode:"unified"};
      if(name==="nova2_zip_train"){started.push(args);return {parameters:1182144,transformerParameters:755200,denseParameters:336320,step:0,training:true,total:10,completed:0,stage:"Teaching ZIP"};}
      return null;
    }}}}
};

element("trainingEpochs").value="12";element("trainingRate").value="0.0001";
element("foundationTrainingEnabled").checked=true;
element("foundationZipScope").value="entireZip";
element("massLessonZip").files=[{size:4,async arrayBuffer(){return new Uint8Array([80,75,3,4]).buffer;}}];
vm.runInNewContext(main,browser,{filename:"nova2-main.js"});
Promise.resolve().then(()=>Promise.resolve()).then(async()=>{
 assert.deepEqual(commands.slice(0,2),["nova2_boot","nova2_conversation"]);
 assert.ok(commands.every(x=>!x.startsWith("hybrid_")), "Boot must not use legacy backend");
 // Regression: a validated ZIP can begin with an EMPTY single-lesson editor.
 assert.equal(element("lessonData").value,"");
 await element("previewMassLessons").click();
 await element("startMassLessons").click();
 assert.equal(started.length,1,"ZIP worker should begin without a pasted single lesson");
 assert.equal(started[0].mode,"auto");
 assert.equal(started[0].options.foundationTraining,true);
 assert.equal(started[0].options.foundationZipScope,"entireZip");
 console.log("PASS: one Bicameral Nova-2 brain, .nnova2 persistence, teacher/targeted/test wiring, KV cache");
}).catch(e=>{console.error(e);process.exitCode=1;});
