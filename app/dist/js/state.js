"use strict";

/*
 * Nova's live runtime state and DOM references.
 * This is short-term process state, not persistent learned memory.
 */



    /* =========================================================
       JS SECTION: APP STATE
       Future file: src/state.js
       ========================================================= */

    const state = {
      messages: [],
      facts: [],
      inferredFacts: [],
      context: {
        lastEntity: "user",
        lastTopic: null,
        lastIntent: null,
        lastQuestionRelation: null,
        recentEntities: [],

        // V4 conversational working memory.
        currentTopic: null,
        topicStack: [],
        salience: {},
        turn: 0,
        lastSemanticFrame: null,
        lastResolvedReferences: [],
        lastParseConfidence: 0
      }
    };



    /* =========================================================
       JS SECTION: DOM REFERENCES
       Future file: src/ui/dom.js
       ========================================================= */

    const elements = {
      chat: document.getElementById("chat"),
      composer: document.getElementById("composer"),
      input: document.getElementById("messageInput"),
      sendButton: document.getElementById("sendButton"),
      appAccessButton: document.getElementById("appAccessButton"),
      appAccessDrawer: document.getElementById("appAccessDrawer"),
      closeAppAccessButton: document.getElementById("closeAppAccessButton"),
      appAccessStatus: document.getElementById("appAccessStatus"),
      appAdapterSelect: document.getElementById("appAdapterSelect"),
      appWindowSelect: document.getElementById("appWindowSelect"),
      refreshAppWindowsButton: document.getElementById("refreshAppWindowsButton"),
      enableAppAccessButton: document.getElementById("enableAppAccessButton"),
      endAppAccessButton: document.getElementById("endAppAccessButton"),
      focusApprovedAppButton: document.getElementById("focusApprovedAppButton"),
      sendLastNovaReplyButton: document.getElementById("sendLastNovaReplyButton"),
      captureAppSelectionButton: document.getElementById("captureAppSelectionButton"),
      autoSendBridgeCheckbox: document.getElementById("autoSendBridgeCheckbox"),
      appBridgeStatus: document.getElementById("appBridgeStatus"),
      learnButton: document.getElementById("learnButton"),
      learnFileInput: document.getElementById("learnFileInput"),
      clearButton: document.getElementById("clearButton"),
      status: document.getElementById("status"),
      brainButton: document.getElementById("brainButton"),
      brainDrawer: document.getElementById("brainDrawer"),
      drawerBackdrop: document.getElementById("drawerBackdrop"),
      closeBrainButton: document.getElementById("closeBrainButton"),
      reflectionButton: document.getElementById("reflectionButton"),
      relearnButton: document.getElementById("relearnButton"),
      testButton: document.getElementById("testButton"),
      brainContent: document.getElementById("brainContent")
    };


    /* =========================================================
       V4.3.13 SECTION: PLAIN-ENGLISH THINKING TRACE STATE

       This is temporary turn instrumentation, not learned memory.
       ========================================================= */

    function nova4313EnsureThinkingState() {
      state.thinking ||= {
        currentTurn:
          null,
        history:
          []
      };

      return state.thinking;
    }

    function nova4313BeginThinking(userMessage) {
      const thinking =
        nova4313EnsureThinkingState();

      const cognition =
        typeof v43EnsureCognition === "function"
          ? v43EnsureCognition()
          : null;

      const humor =
        typeof v437EnsureHumorHistoryMetadata === "function"
          ? v437EnsureHumorHistoryMetadata()
          : null;

      thinking.currentTurn = {
        id:
          `thinking_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
        userMessage:
          String(
            userMessage ||
            ""
          ),
        status:
          "thinking",
        startedAt:
          Date.now(),
        startingContextTurn:
          state.context?.turn ||
          0,
        startingHumorHistoryLength:
          humor?.history?.length ||
          0,
        startingResponsePlanCreatedAt:
          state.context?.lastResponsePlan?.createdAt ||
          0,
        startingValidationCreatedAt:
          cognition?.humorPlanner?.lastValidation?.createdAt ||
          0,
        completedAt:
          null,
        steps:
          []
      };

      return thinking.currentTurn;
    }

    function nova4313Thought(text, kind = "reasoning") {
      const thinking =
        nova4313EnsureThinkingState();

      const turn =
        thinking.currentTurn;

      if (
        !turn ||
        !text
      ) {
        return null;
      }

      const clean =
        normalizeWhitespace(
          String(text)
        );

      if (!clean) {
        return null;
      }

      const previous =
        turn.steps[
          turn.steps.length -
          1
        ];

      if (
        previous &&
        canonical(
          previous.text
        ) ===
          canonical(
            clean
          )
      ) {
        return previous;
      }

      const step = {
        id:
          `thought_${Date.now()}_${turn.steps.length}`,
        kind,
        text:
          clean,
        createdAt:
          Date.now()
      };

      turn.steps.push(
        step
      );

      return step;
    }

    function nova4313CompleteThinking() {
      const thinking =
        nova4313EnsureThinkingState();

      const turn =
        thinking.currentTurn;

      if (!turn) {
        return null;
      }

      turn.status =
        "complete";

      turn.completedAt =
        Date.now();

      const savedTurn =
        typeof structuredClone ===
          "function"
          ? structuredClone(
              turn
            )
          : JSON.parse(
              JSON.stringify(
                turn
              )
            );

      thinking.history.push(
        savedTurn
      );

      thinking.history =
        thinking.history.slice(
          -30
        );

      return turn;
    }


    /* =========================================================
       V7 PASS 4.1: DEVELOPER TRACE STATE

       This is technical telemetry from the same authoritative turn.
       It is not cognition and never feeds Nova's reasoning.
       ========================================================= */

    function nova700EnsureDeveloperTraceState() {
      state.developerTrace ||= { history: [] };
      if (!Array.isArray(state.developerTrace.history)) state.developerTrace.history = [];
      return state.developerTrace;
    }

    function nova700RecordDeveloperTrace(ws, report, response) {
      if (!ws) return null;
      const store = nova700EnsureDeveloperTraceState();
      const mind = ws.canonicalSemanticState || {};
      const now = Date.now();
      const currentThinking = nova4313EnsureThinkingState().currentTurn || null;
      const clone = value => {
        try { return structuredClone(value); }
        catch (_) { try { return JSON.parse(JSON.stringify(value)); } catch (__) { return value; } }
      };
      const ids = list => (Array.isArray(list) ? list : []).map(item => String(item?.id ?? "")).filter(Boolean);
      const lastMessage = Array.isArray(state.messages) ? [...state.messages].reverse().find(message => message?.role === "user") : null;
      const entry = {
        id: `devtrace_${now}_${Math.random().toString(36).slice(2, 8)}`,
        turnId: ws?.rustSemanticSeed && typeof nova7RustMindBridge === "object" ? (nova7RustMindBridge.activeTurnId || null) : null,
        input: String(ws.rawInput || ""),
        sourceLabel: lastMessage?.sourceLabel || null,
        startedAt: Number(currentThinking?.startedAt || ws?.executive?.startedAt || now),
        completedAt: now,
        durationMs: Math.max(0, now - Number(currentThinking?.startedAt || ws?.executive?.startedAt || now)),
        terminalState: ws?.executive?.terminalState || null,
        reasonCode: ws?.executive?.reasonCode || null,
        response: typeof response === "string" ? response : null,
        semanticOwner: mind?.semanticOwner || null,
        status: mind?.status || null,
        canonicalStateId: mind?.id || null,
        canonicalIds: {
          references: ids(mind?.references),
          entities: ids(mind?.entities),
          events: ids(mind?.events),
          relations: ids(mind?.relations),
          propositions: ids(mind?.propositions),
          requests: ids(mind?.requests),
          conclusions: ids(mind?.conclusions),
          unresolved: ids(mind?.unresolved)
        },
        rustSemanticReport: clone(ws?.rustSemanticReport || mind?.rustSemanticReport || null),
        rustCognitiveReport: clone(ws?.rustCognitiveReport || mind?.rustCognitiveReport || null),
        semanticTechnicalTrace: clone(mind?.semanticTrace || []),
        executive: clone(ws?.executive || null),
        cognitiveSearch: clone(ws?.cognitiveSearch || null),
        experienceIndexActivity: clone(ws?.executive?.details?.experienceRoute || null),
        synthesisDebug: clone(ws?.executive?.details?.consolidation || null)
      };
      store.history.push(entry);
      store.history = store.history.slice(-100);
      return entry;
    }

