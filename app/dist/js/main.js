"use strict";

/*
 * Nova desktop/browser startup and event wiring.
 * This connects the UI to Nova after every subsystem has loaded.
 */



    /* =========================================================
       JS SECTION: CHAT CONTROLLER
       Future file: src/chat/controller.js
       ========================================================= */

    let novaCurriculumTestActive = false;

    async function handleUserMessage(rawMessage, options = {}) {
      const raw = String(rawMessage ?? "");
      const message = normalizeWhitespace(raw);

      if (!raw.trim()) return;

      hideStatus();
      addMessage(
        "user",
        message,
        {
          rawContent: raw,
          sourceLabel: options.sourceLabel || null
        }
      );

      elements.input.value = "";
      resizeInput();
      elements.sendButton.disabled = true;

      if (
        APP_CONFIG.thinkingTrace?.enabled !==
          false
      ) {
        nova4313CreateThinkingUi();
      } else {
        showStatus(`${APP_CONFIG.assistantName} is thinking...`);
      }

      let producedReply = null;
      let rustTurnReceipt = null;

      try {
        if (typeof nova7RustMindBeginTurn === "function") {
          rustTurnReceipt = await nova7RustMindBeginTurn(raw, { sourceLabel: options.sourceLabel || null });
        }
        if (window.__TAURI__?.core?.invoke && !rustTurnReceipt) {
          throw new Error("The authoritative Rust NovaMind did not start this turn. Native Nova will not fall back to a JavaScript brain.");
        }
      } catch (error) {
        if (window.__TAURI__?.core?.invoke) throw error;
      }

      try {
        await wait(randomDelay());

        const reply =
          await generateAssistantReply(
            raw
          );

        producedReply = reply;

        if (
          APP_CONFIG.thinkingTrace?.enabled !==
            false
        ) {
          const thinkingTurn =
            nova4313EnsureThinkingState()
              .currentTurn;

          if (
            thinkingTurn?.steps?.length
          ) {
            await nova4313RenderThinkingTurn(
              thinkingTurn
            );
          }
        } else {
          hideStatus();
        }

        if (typeof reply === "string" && reply.trim()) {
          addMessage(
            "assistant",
            reply
          );
        }

        if (rustTurnReceipt && typeof nova7RustMindFinishTurn === "function") {
          await nova7RustMindFinishTurn({
            responseProduced: Boolean(typeof reply === "string" && reply.trim())
          });
        }

        renderBrainInspector();
        try {
          if (typeof novaSaveDurableCheckpoint === "function") await novaSaveDurableCheckpoint("completed_turn");
        } catch (persistenceError) {
          console.error("Completed-turn durable checkpoint failed:", persistenceError);
        }
      } catch (error) {
        console.error(error);

        if (
          APP_CONFIG.thinkingTrace?.enabled !==
            false
        ) {
          const thinkingTurn =
            nova4313EnsureThinkingState()
              .currentTurn;

          if (
            thinkingTurn?.steps?.length
          ) {
            await nova4313RenderThinkingTurn(
              thinkingTurn
            );
          }
        } else {
          hideStatus();
        }

        if (rustTurnReceipt && typeof nova7RustMindFinishTurn === "function") {
          await nova7RustMindFinishTurn({ responseProduced: false });
        }
        renderBrainInspector();
        try {
          if (typeof novaSaveDurableCheckpoint === "function") await novaSaveDurableCheckpoint("blocked_turn");
        } catch (persistenceError) {
          console.error("Blocked-turn durable checkpoint failed:", persistenceError);
        }
        showStatus("Nova's current reasoning hit an internal error. Open Brain Inspector for the technical details.");
      } finally {
        nova4313ClearThinkingUiReference();
        elements.sendButton.disabled = Boolean(novaCurriculumTestActive);
        if (elements.input) elements.input.disabled = Boolean(novaCurriculumTestActive);
        if (!novaCurriculumTestActive) elements.input?.focus();
      }

      return producedReply;
    }

    async function runVisibleCurriculumTest(suite) {
      if (!suite?.cases?.length || typeof v534RunInteractiveTestSuite !== "function") return null;
      if (novaCurriculumTestActive) throw new Error("A curriculum test is already running.");

      novaCurriculumTestActive = true;
      if (elements.input) elements.input.disabled = true;
      if (elements.sendButton) elements.sendButton.disabled = true;
      if (elements.learnButton) elements.learnButton.disabled = true;

      try {
        const run = await v534RunInteractiveTestSuite(suite, {
          persist: true,
          askQuestion: async (question, meta) => {
            // This is deliberately the exact same visible turn controller used for
            // user/ChatGPT prompts. Only the display source changes to Teacher.
            const response = await handleUserMessage(question, { sourceLabel: meta?.sourceLabel || "Teacher" });
            const executive = state.context?.lastGenericCoreTurn?.executive || null;
            return {
              response: typeof response === "string" ? response : null,
              terminal: executive?.terminalState || null,
              reasonCode: executive?.reasonCode || null
            };
          },
          afterAttempt: async (_attempt, meta) => {
            // Teacher/test history is already in the Rust mind at this point. Persist
            // each completed attempt so grading survives an immediate app update.
            if (typeof novaSaveDurableCheckpoint === "function") await novaSaveDurableCheckpoint("teacher_test_attempt");
            await new Promise(resolve => setTimeout(resolve, 220));
          }
        });

        // The answer key/feedback evidence is released to Nova only after every
        // question in the run has been attempted. Persist that completed release.
        if (typeof novaSaveDurableCheckpoint === "function") await novaSaveDurableCheckpoint("teacher_test_run_complete");
        addMessage("user", v534FormatTestRun(run), { sourceLabel: "Teacher" });
        renderBrainInspector();
        return run;
      } finally {
        novaCurriculumTestActive = false;
        if (elements.input) elements.input.disabled = false;
        if (elements.sendButton) elements.sendButton.disabled = false;
        if (elements.learnButton) elements.learnButton.disabled = false;
        elements.input?.focus();
      }
    }



    /* =========================================================
       JS SECTION: EVENT LISTENERS
       Future file: src/events.js
       ========================================================= */

    elements.composer.addEventListener("submit", event => {
      event.preventDefault();
      handleUserMessage(elements.input.value);
    });

    elements.input.addEventListener("input", resizeInput);

    elements.input.addEventListener("keydown", event => {
      if (
        event.key === "Enter" &&
        !event.shiftKey &&
        !event.isComposing
      ) {
        event.preventDefault();
        handleUserMessage(elements.input.value);
      }
    });

    elements.clearButton.addEventListener("click", () => {
      const confirmed = confirm(
        "Clear conversation, cognitive memory, learned knowledge, and inferred knowledge?"
      );

      if (!confirmed) return;

      resetState();
      renderAllMessages();
      renderBrainInspector();

      addMessage(
        "assistant",
        "Everything is cleared. Tiny existential reset complete."
      );
    });

    if (elements.learnButton && elements.learnFileInput) {
      elements.learnButton.addEventListener("click", () => {
        elements.learnFileInput.value = "";
        elements.learnFileInput.click();
      });

      elements.learnFileInput.addEventListener("change", async event => {
        const file = event.target.files?.[0];
        if (!file) return;
        const oldText = elements.learnButton.textContent;
        elements.learnButton.disabled = true;
        elements.learnButton.textContent = "Reading...";
        showStatus(`${APP_CONFIG.assistantName} is reading ${file.name}...`);
        try {
          const result = await v620HandleDocumentFile(file);
          hideStatus();
          if (typeof result.response === "string" && result.response.trim()) addMessage("assistant", result.response);
          renderBrainInspector();
          if (result.testSuite?.cases?.length) {
            await runVisibleCurriculumTest(result.testSuite);
          } else if (!(typeof result.response === "string" && result.response.trim())) {
            showStatus(`Processed ${file.name}; no labeled test questions were found.`);
          }
        } catch (error) {
          hideStatus();
          showStatus(`Document processing failed: ${error.message || error}`);
        } finally {
          elements.learnButton.disabled = false;
          elements.learnButton.textContent = oldText || "Document";
        }
      });
    }

    elements.brainButton.addEventListener("click", openBrain);

    if (elements.reflectionButton) {
      elements.reflectionButton.addEventListener("click", async () => {
        if (novaCurriculumTestActive) return;
        closeBrain();
        elements.reflectionButton.disabled = true;
        try {
          await handleUserMessage("Time to reflect on your past tests and teacher feedback.", { sourceLabel: "Teacher" });
        } finally {
          elements.reflectionButton.disabled = false;
        }
      });
    }

    if (elements.relearnButton) {
      elements.relearnButton.addEventListener("click", async () => {
        const confirmed = confirm("Relearn rebuilds Nova's derived semantic knowledge and learned cognitive modules from retained lesson sources using only the current V7 migration architecture. No older executable is restored as a fallback. Continue?");
        if (!confirmed) return;
        const oldText=elements.relearnButton.textContent;elements.relearnButton.disabled=true;elements.relearnButton.textContent="Relearning...";
        try {
          const report=await v620RelearnAllKnowledge();
          renderBrainInspector();
          if(report.status==="NO_FULL_LESSON_SOURCES")alert("Relearn found no retained full structured lesson sources. Nothing was changed.");
          else {
            const warningCount=(report.errors||[]).length+(report.warnings||[]).length;
            const regressionCount=Number(report.cognitionComparison?.regressionCount||0);
            const newlyExecutable=Number(report.cognitionComparison?.newlyExecutableCount||0);
            alert(`Relearn ${report.status==="COMPLETED"?"complete":"completed with warnings"}: ${report.recompiledSources} cognitive source(s) rebuilt • ${report.reindexedSemanticSources} semantic source(s) re-indexed • ${newlyExecutable} learned directive(s) became executable • ${warningCount} warning(s)${regressionCount?` • ${regressionCount} previously executable teaching(s) are no longer executable in the current mind and were not restored through legacy code`:""}.`);
          }
        } finally {elements.relearnButton.disabled=false;elements.relearnButton.textContent=oldText;}
      });
    }

    if (elements.testButton) {
      elements.testButton.addEventListener("click", async () => {
        elements.testButton.disabled = true;
        const oldText = elements.testButton.textContent;
        elements.testButton.textContent = "Testing...";

        try {
          const summary = await v620RunRegressionTests();
          alert(`Nova V7 alpha 9.3 regression tests: ${summary.passed}/${summary.total} passed • ${summary.failed} failed.`);
        } finally {
          elements.testButton.disabled = false;
          elements.testButton.textContent = oldText;
        }
      });
    }

    elements.closeBrainButton.addEventListener("click", closeBrain);
    elements.drawerBackdrop.addEventListener("click", () => { closeBrain(); });



    /* =========================================================
       JS SECTION: INITIALIZATION
       Future file: src/main.js
       ========================================================= */

    async function initializeApp() {
      let persistenceBootstrap = null;
      if (typeof nova620BootstrapPersistentState === "function") {
        persistenceBootstrap = await nova620BootstrapPersistentState();
      }
      // During hydration/migrations, preserve the last known-good durable file
      // until Rust semantic/learning authority has been restored. Legacy JS
      // maintenance may call saveState(), but those writes remain pending until
      // the final startup checkpoint commits the fully hydrated current mind.
      if (typeof nova620SetPersistentWritesSuspended === "function") nova620SetPersistentWritesSuspended(true);

      // Load directly from the durable bootstrap result when available. This is
      // important for learned states that are larger than localStorage's cache
      // quota: IndexedDB/Tauri can still restore them without first squeezing the
      // full mind back through localStorage.
      loadState(persistenceBootstrap?.uiState || persistenceBootstrap?.raw || null);
      if (!window.__TAURI__?.core?.invoke) {
        // Browser development mode retains the historical JavaScript maintenance
        // harness. Native Pass 9 never runs JavaScript brain maintenance.
        if (typeof nova620PostLoadStateMaintenance === "function") nova620PostLoadStateMaintenance();
        if (typeof v533PostLoadSynthesisMaintenance === "function") v533PostLoadSynthesisMaintenance();
        if (typeof v530PostLoadEpistemicMaintenance === "function") v530PostLoadEpistemicMaintenance();
        if (typeof v534PostLoadDocumentIngestionMaintenance === "function") v534PostLoadDocumentIngestionMaintenance();
        if (typeof v554PostLoadCognitiveAbstractionMaintenance === "function") v554PostLoadCognitiveAbstractionMaintenance();
        if (typeof v555PostLoadActivityMaintenance === "function") v555PostLoadActivityMaintenance();
        if (typeof v556PostLoadMindStateMaintenance === "function") v556PostLoadMindStateMaintenance();
      }
      let rustInitialization = null;
      if (typeof nova7RustMindInitialize === "function") rustInitialization = await nova7RustMindInitialize();
      if (window.__TAURI__?.core?.invoke && !rustInitialization?.initialized) {
        // Keep durable writes suspended. Never replace a valid saved mind with a
        // partially hydrated frontend snapshot when authoritative Rust startup failed.
        throw new Error("Authoritative Rust NovaMind failed to hydrate; durable state was left untouched.");
      }
      ensureV4Context();
      if (!window.__TAURI__?.core?.invoke) v43EnsureCognition();
      renderAllMessages();
      renderBrainInspector();

      if (state.messages.length === 0) {
        addMessage(
          "assistant",
          [
            `Hey. I'm ${V41_SELF_MODEL.identity.name}, ${V41_SELF_MODEL.identity.kind}.`,
            "",
            "I begin with a small bootstrap cognitive structure and generic operations for forming, relating, transforming, and reusing learned structures. I do not begin with general world facts.",
            "",
            "My live mind uses one canonical cognitive state and one iterative executive. Surface observations do not assign meanings such as hypothetical, actor-question, or emotion from developer-written keyword lists; learned modules can connect language and experience to generic semantic distinctions.",
            "",
            "Teaching can create reusable cognitive modules and categories. Later learning can reuse those modules and can make older stored guidance executable without replacing my mind or falling back to an older Nova.",
            "",
            'Tap "Brain" to inspect my current cognitive state, generic modules, categories, learned rules, memory, and provisional abstractions.'
          ].join("\n")
        );
      }

      if (typeof novaSetPersistenceHydrationReady === "function") novaSetPersistenceHydrationReady(true);
      if (typeof nova620SetPersistentWritesSuspended === "function") nova620SetPersistentWritesSuspended(false);
      try {
        if (typeof novaSaveDurableCheckpoint === "function") await novaSaveDurableCheckpoint("startup_hydrated");
      } catch (persistenceError) {
        console.error("Startup durable checkpoint failed:", persistenceError);
      }

      resizeInput();
      elements.input.focus();
    }

    initializeApp().then(() => {
      if (new URLSearchParams(location.search).get("autotest") === "1") {
        setTimeout(async () => {
          try {
            const summary = await v620RunRegressionTests();
            document.documentElement.dataset.autotest =
              `${summary.passed}/${summary.total}`;
            document.documentElement.dataset.autotestFailed =
              String(summary.failed);
          } catch (error) {
            document.documentElement.dataset.autotest = "ERROR";
            document.documentElement.dataset.autotestError =
              String(error.message || error);
          }
        }, 50);
      }
    }).catch(error => {
      console.error("Nova startup failed:", error);
      try { showStatus("Nova could not safely hydrate the authoritative Rust mind. The last durable state was left untouched."); } catch (_) { }
    });


