"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs"),path=require("node:path");
const root=path.resolve(__dirname,"..");
const read=p=>fs.readFileSync(path.join(root,p),"utf8");
const html=read("dist/index.html"),js=read("dist/js/nova-messenger.js");
const rust=read("src-tauri/src/lib.rs"),runtime=read("src-tauri/src/nova2_lab.rs");
const signing=read("scripts/configure_android_ci.py");
const provider=read("android/NovaTrainingContextProvider.java");
const cap=JSON.parse(read("src-tauri/capabilities/main.json"));
const pkg=JSON.parse(read("package.json"));
const conf=JSON.parse(read("src-tauri/tauri.conf.json"));
assert.equal(pkg.version,"0.20.0");
assert.equal(conf.version,pkg.version);
assert.equal(conf.identifier,"com.qtech.neuralnova.experimental");
assert.equal(conf.productName,"Neural Nova Experimental");
assert.doesNotMatch(JSON.stringify(conf),/background-service/);
assert.doesNotMatch(cap.permissions.join(" "),/background-service/);
for(const id of ["novaMessenger","chatTranscript","chatMessage","sendMessage","brainStateText",
  "novaThinkingTree","novaTraceNodes","novaBrainRail","hemisphereBrain","bridgeFibers",
  "leftActivity","rightActivity","inspectBrain","attentionTiles","predictions",
  "checkpointTools","importCheckpointNative","importCheckpoint","exportCheckpoint"]){
  assert.ok(html.includes('id="'+id+'"'),"Static messenger is missing DOM id "+id);
}
assert.match(html,/<script defer src="\.\/js\/nova-messenger\.js"><\/script>/);
assert.doesNotMatch(html,/nova2-main\.js|nova-messenger-layout\.js|nova2-test-ui\.js/);
assert.doesNotMatch(html,/Train Nova|id="pane-train"|id="pane-teach"|id="pane-targeted"|id="testLab"|id="trainingStage"/);
assert.ok(html.indexOf('id="chatTranscript"')<html.indexOf("js/nova-messenger.js")||html.includes('id="chatTranscript"'));
new Function(js);
assert.match(js,/invoke\("nova2_boot"\)/);
assert.match(js,/invoke\("nova2_chat"/);
assert.match(js,/invoke\("nova2_inspect"/);
assert.match(js,/invoke\("nova2_checkpoint_import_finish"/);
assert.match(js,/invoke\("nova2_checkpoint_export_android"/);
assert.match(js,/showBrain/);
assert.match(js,/renderHistory/);
assert.match(js,/trace\(result,message\)/);
assert.doesNotMatch(js,/nova2_train|nova2_stop|nova2_zip_train|nova2_targeted_train|applyCorrections|teacherQuestion/);
assert.match(runtime,/BUNDLED_NOVA2/);
assert.match(runtime,/model.step!=19_000/);
assert.match(runtime,/Nova2::load\(&p\)/);
assert.match(runtime,/cache\.token_count\(\)>=window/);
assert.match(runtime,/pub fn nova2_checkpoint_import_finish/);
assert.match(runtime,/pub fn nova2_chat/);
assert.doesNotMatch(runtime,/pub fn nova2_train|pub fn nova2_zip_train|pub fn nova2_targeted_train|pub fn nova2_reset|pub fn nova2_test/);
const commands=[...rust.matchAll(/nova2_lab::(nova2_\w+)/g)].map(m=>m[1]);
for(const needed of ["nova2_boot","nova2_chat","nova2_inspect","nova2_checkpoint_import_finish",
  "nova2_checkpoint_export_android","nova2_new_chat"])assert.ok(commands.includes(needed));
assert.ok(commands.every(c=>!/train|test|stop|reset|lesson|rehearsal|observer|targeted|zip/.test(c)),
  "Training command still registered in app: "+commands.join(", "));
assert.doesNotMatch(rust,/mod (neural|hybrid_lab|nova2_tests|targeted_lab|lesson_zip|training_keepalive)/);
assert.match(provider,/endsWith\("\.nnova2"\)/);
assert.match(signing,/EXPECTED_SHA256="070b1e684da8087d6c30874c16c9be831b7cbce49264d61040ee73ecb5652ade"/);
const cargo=read("src-tauri/Cargo.toml");
assert.doesNotMatch(cargo,/tauri-plugin-background-service|tauri-plugin-notification/);
console.log("PASS: static Nova messenger, brain inspector, preserved checkpoint format, Android identity & signing; no training API");
