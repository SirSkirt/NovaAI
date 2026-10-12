# Nova V7 Architecture — Rust-Authoritative Mind + Alpha 9.3

## One Nova

Native Tauri Nova has one authoritative mind: `NovaMind` in Rust.

```text
User / Teacher input
        ↓
thin JS UI/IPC
        ↓
Rust NovaMind
  Basic English structural bootstrap
  semantic construction + participant grounding
  semantic memory / learned cognition
  central executive
  experience / development
  internal goals
  teacher/test evidence
  realization
        ↓
Rust realization + read-only snapshot projections
        ↓
JS renders Chat / Thinking / Brain Inspector / Teacher controls
```

There is no second English engine, Teacher brain, Reflection brain, or JavaScript fallback Nova.

## Basic English bootstrap

`semantics/bootstrap.rs` is part of the existing Rust semantic pipeline. Its job is substrate-level structure, not curriculum knowledge.

It may preserve:

- whole utterance and sentence/quotation boundaries;
- simple phrase spans;
- structural left / link / right groupings;
- simple `X is Y` and `X ... Y` shapes;
- auxiliary/operator surface such as `can`;
- negation and coordination cues;
- statement/question/request surface distinction;
- basic deictic/reference and previous-utterance candidates;
- small English glue vocabulary needed to keep simple teaching instructions intact.

It must **not** install classroom concepts or a hidden ontology. Capitalization is not treated as proof that something is a proper name. Structural arguments are not automatically labeled with learned semantic roles such as agent/patient. Content words do not gain hidden definitions from the bootstrap.

Learned English and reasoning continue through the ordinary semantic-memory/cognitive-learning machinery.

## Thinking and Brain Inspector

Thinking is a meaning-level rendering of the authoritative Rust `cognitiveTrace`. Alpha 9.3 may show Basic English interpretation such as how wording was grouped, but it does not create a second interpretation.

Brain Inspector shows the same Rust state in technical form, including `englishBootstrap`, canonical semantic structures, knowledge used, goals, Stored Only, teacher/test history, synthesis/development, persistence and Grandpa Rust telemetry.

## Teacher evidence

`NovaMind.teacher_state` owns teacher/test history. Original questions and Nova answers are immutable.

Feedback may add:

- points awarded / possible;
- optional percentage;
- optional grade label;
- Teacher Notes;
- exact strike/highlight/underline spans;
- optional notes attached to each span.

Rust validates spans against the preserved UTF-16 text. If both points and percentage are supplied, they must describe the same grade.

Completed attempts are durable immediately, but expected-answer/grading evidence is not released into semantic memory until the full test run finalizes. Released feedback is `teacher_feedback` evidence with Teacher provenance and `nonExecutable=true`.

## Reflection

Start Reflection sends an ordinary Teacher utterance through Nova's normal Rust turn path. It does not provide reflective questions or invoke a hidden reflection algorithm. Preserved educational history is evidence; Nova must use her current learned cognition to inspect it.

## Persistence

Rust still owns the complete persistent `NovaMind`. Alpha 9.3 extends the serialized Rust schemas through serde-defaulted fields and normalizes prior alpha 9.2 Teacher history. The low-level alpha 6.1/V5-style atomic app-data writer remains infrastructure and is not replaced.

## External tools

Tool/app authorization remains separate from cognition and from Teacher tooling.
