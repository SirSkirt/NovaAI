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

assert.match(native,/"step":brain\.transformer\.step/);
assert.match(native,/hybrid_request_stop/);
assert.match(js,/Number\.isFinite\(p\.step\)/);
assert.match(js,/hybrid-stop-requested/);
assert.match(js,/Stop Nova's current training session/);
console.log("Foundation progress event schema and native cancellation provenance guards OK.");

const zip=read("src-tauri/src/lesson_zip.rs");
assert.match(html,/id="foundationZipScope"/);
assert.match(html,/Foundation Training v2/);
assert.match(bulk,/foundationZipScope:foundationScope/);
assert.match(bulk,/learningEvidence/);
assert.match(native,/foundation_zip_scope:Option<String>/);
assert.match(native,/foundation_retry_attempt:Option<u8>/);
assert.match(native,/foundation_alternate_schedule:Option<bool>/);
assert.match(native,/autonomous_validation_replies\(&best_brain,&practice\)/);
assert.match(native,/foundation_fit_vs_generalization/);
assert.match(native,/generalization-gap-suspected/);
assert.match(zip,/fn recovery_plan/);
assert.match(zip,/foundation_retries_use_distinct_approaches/);
assert.match(zip,/max_attempts:usize=if selective\{4\}else\{1\}/);
assert.match(zip,/let \(best_epoch,error,evidence\)=hybrid_lab::completed_training_outcome/);
assert.match(zip,/s\.learning_evidence=evidence/);
assert.match(zip,/"firstFive"\|"entireZip"/);
console.log("Foundation v2 ZIP duration, bounded retries, checkpoint guards and separate fit/generalization evidence OK.");

assert.match(native,/best_rejected/);
assert.match(native,/rejected .* candidate/);
assert.match(native,/retention_blocked/);
assert.match(bulk,/evidence\.evaluatedFrom/);
console.log("Rejected-candidate diagnostics are separated from retained checkpoint evidence.");
