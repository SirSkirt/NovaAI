"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs"),path=require("node:path");
const app=path.resolve(__dirname,"..");
const read=file=>fs.readFileSync(path.join(app,file),"utf8");
const html=read("dist/index.html");
const layout=read("dist/js/nova-messenger-layout.js");
const style=read("dist/nova-messenger.css");
const main=read("dist/js/nova2-main.js");
const native=read("src-tauri/src/lib.rs");
const inspection=read("src-tauri/src/nova2_lab.rs");
new Function(layout);
new Function(main);
for(const pattern of [/nova-messenger-layout\.js/,/nova-messenger\.css/,/id="pane-teach"/,
  /id="pane-targeted"/,/id="pane-chat"/,/id="pane-visualize"/,/id="hemisphereBrain"/,
  /id="observerSession"/,/id="importCheckpoint"/,/id="chatTranscript"/])assert.match(html,pattern);
assert.match(layout,/chat\.hidden=false/);
assert.match(layout,/main\.prepend\(chat\);main\.append\(train\)/);
assert.match(layout,/train\.append\(activity\)/);
assert.match(layout,/train\.append\(pane\)/);
assert.match(layout,/rail\.append\(diagram\)/);
assert.match(layout,/const selected=\["teach","targeted","activity"\]/);
assert.match(main,/window\.NovaMessenger\?\.showTrainSubtab\(name\)/);
assert.match(main,/new CustomEvent\("nova-reply"/);
assert.match(layout,/Thinking tree/);
assert.match(layout,/does not expose a step-by-step reasoning tree/);
assert.match(layout,/trace\.method/);
assert.match(layout,/trace\.denseLayers/);
assert.match(layout,/trace\.attention/);
assert.match(layout,/trace\.interconnectUsed/);
assert.match(layout,/nova2_inspect/);
assert.match(native,/nova2_lab::nova2_inspect/);
assert.match(inspection,/pub fn nova2_inspect/);
assert.match(inspection,/Token-removal output sensitivity/);
assert.match(style,/grid-template-columns:minmax\(0,1fr\) minmax\(300px,365px\)/);
assert.match(style,/@media\(max-width:1120px\)/);
const packageVersion=JSON.parse(read("package.json")).version;
const tauri=JSON.parse(read("src-tauri/tauri.conf.json"));
assert.equal(packageVersion,"0.19.1");
assert.equal(tauri.version,packageVersion);

assert.ok(layout.indexOf("grid.append(conversation);chat.append(grid)") <
          layout.indexOf("const settings=card.querySelector"),
          "Chat must mount before optional inspector reparenting");
assert.match(layout,/notice\.id="modelNotice"/);
assert.match(main,/\$\("sendMessage"\)\.disabled=running\|\|!s\.trained/);
const bundled=read("src-tauri/build.rs");
assert.match(bundled,/resources\/nova2-step19000\.nnova2/);
assert.match(inspection,/BUNDLED_NOVA2/);
assert.match(inspection,/model\.step==19_000/);
assert.match(inspection,/cache\.token_count\(\)>=window/);
assert.match(inspection,/normalize\(cache\.prefill/);
console.log("PASS: Nova messenger, Train subtabs, mobile inspector, honest thinking-tree and native read-only inspection wiring");
