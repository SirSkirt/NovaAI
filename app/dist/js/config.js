"use strict";

/*
 * Nova application configuration.
 * Keep product/runtime configuration here rather than in the brain.
 */


    /* =========================================================
       JS SECTION: CONFIG
       Future file: src/config.js
       ========================================================= */

    const APP_CONFIG = {
      assistantName: "Nova",
      storageKey: "local-assistant-v4.8.0-controlled-app-access",
      thinkingTrace: {
        enabled: true,
        stepDelayMs: 260,
        finalDelayMs: 180
      },
      // Do not impose a cognitive history truncation. Physical storage limits are
      // a sandbox/resource boundary, not a reasoning rule.
      maxMessages: Infinity,
      maxRecentContext: 24,
      typingDelayMin: 120,
      typingDelayMax: 320
    };
