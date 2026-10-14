"use strict";
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path");
const read=p=>fs.readFileSync(path.join(__dirname,"..",p),"utf8");
const html=read("dist/index.html"),js=read("dist/js/nova-ui.js");
const bulk=read("dist/js/mass-learning-ui.js"),native=read("src-tauri/src/hybrid_lab.rs");
const rehearsal=read("src-tauri/src/rehearsal.rs");
const core=fs.readFileSync(path.join(__dirname,"..","..","neural-nova","src","hybrid.rs"),"utf8");
assert.match(html,/id="foundationTrainingEnabled"/);
assert.match(html,/id="foundationTrainingStatus"/);
assert.match(js,/foundationTraining:\$\("foundationTrainingEnabled"\)\.checked/);
assert.match(bulk,/foundationTraining:\$\("foundationTrainingEnabled"\)\.checked/);
assert.match(js,/foundationLessonsRemaining/);
assert.match(js,/candidateScores/);
assert.match(native,/foundation_train_inner/);
assert.match(native,/foundation_training:Option<bool>/);
assert.match(native,/foundation_lessons_completed/);
for(const focus of ["Dense","Transformer","Interconnect","Joint"])
  assert.match(native,new RegExp("TrainingFocus::"+focus));
for(const pattern of [/train_memory_batch_focused/,/train_preference_batch_focused/,
  /train_evidence_batch_focused/,/autonomous_selection_ok/,/archived_autonomous_retained/,
  /previous_validation_loss/,/retention_accuracy/,/s\.brain=best_brain/])
  assert.match(native,pattern);
assert.match(rehearsal,/FOUNDATION_LESSON_LIMIT:u8=5/);
assert.match(rehearsal,/foundation_lessons_completed:u8/);
assert.match(rehearsal,/foundation_progress_survives_rejected_lessons_and_resets_on_new_brain/);
assert.match(core,/saves_and_restores_entire_joint_optimizer_state/);
console.log("Foundation candidate selection, optimizer masks, retention, persisted counter and UI/ZIP wiring OK.");
