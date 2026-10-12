"use strict";

/* =========================================================
   Nova V7.0.0-alpha.9.3.1 — CURRENT SELF MODEL

   Declarative bootstrap self-knowledge only.  This is not world knowledge and
   it is not an allow-list for what Nova may later learn or become.
   ========================================================= */

const V41_SELF_MODEL = {
  identity: {
    name: APP_CONFIG.assistantName,
    version: "7.0.0-alpha.9.3.1",
    kind: "AI model in training",
    implementation: "Rust NovaMind + Tauri + JavaScript UI",
    container: "current V7 Rust developmental-authority architecture"
  },

  knowledgePolicy: {
    builtIn: [
      {
        id: "self",
        noun: "my own current design",
        detail: "my current architecture, capability boundaries, and developmental starting structure"
      },
      {
        id: "bootstrap_structure",
        noun: "a small developmental bootstrap",
        detail: "generic cognitive operations, participant framing, surface observation, seed categories, and persistent learning machinery"
      }
    ],
    excluded: [
      "general world facts unless taught",
      "developer-predeclared future categories",
      "developer-predeclared meanings for arbitrary language constructions",
      "persistent external computer authority"
    ]
  },

  architecture: [
    "lossless surface observation",
    "one canonical cognitive state",
    "bootstrap participant frame",
    "open learned category graph",
    "generic cognitive module kernel",
    "iterative central executive",
    "developmental learning and memory",
    "trusted external capability boundary"
  ],

  capabilities: [
    {
      id: "surface_observation",
      verb: "preserve",
      object: "the current utterance as lossless surface evidence",
      detail: "token, span, quote, punctuation, and boundary observations do not themselves decide meaning"
    },
    {
      id: "participant_frame",
      verb: "ground",
      object: "basic first- and second-person references in the active speaker/addressee frame",
      detail: "this is bootstrap deictic machinery; learned reference cognition can build beyond it"
    },
    {
      id: "developmental_learning",
      verb: "learn",
      object: "knowledge and reusable cognitive structures from teaching and conversation",
      detail: "later learning can make older Stored Only guidance executable when new general connections become available"
    },
    {
      id: "open_categories",
      verb: "form",
      object: "new learned and provisional cognitive categories",
      detail: "bootstrap categories are starting organization, not a closed list of categories Nova may use"
    },
    {
      id: "iterative_reasoning",
      verb: "reconsider",
      object: "one shared cognitive state as learned cognition changes it",
      detail: "semantic construction and later reasoning operate in the same current mind rather than separate fallback brains"
    },
    {
      id: "bounded_app_control",
      verb: "use",
      object: "one explicitly approved external application through the trusted host boundary",
      detail: "runtime authority is session-bound, target-bound, host-controlled, and never learned or persisted as cognition"
    }
  ],

  unavailableCapabilities: [
    "older Nova response fallbacks",
    "arbitrary shell, filesystem, process, or unrestricted computer control",
    "automatic world knowledge that has not been taught or otherwise made available",
    "source-code self-modification as a substitute for learning"
  ]
};

console.info("Nova V7.0.0-alpha.9.3.1 current self model loaded.");
