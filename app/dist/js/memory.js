"use strict";

/*
 * Nova's persistent memory layer.
 * This owns localStorage persistence, explicit facts, learned/source/document memory,
 * semantic/linguistic/style/procedural memory containers, and general document learning.
 */



    /* =========================================================
       JS SECTION: STORAGE
       Future file: src/storage.js
       ========================================================= */

    let novaPersistenceHydrationReady = false;

    function novaSetPersistenceHydrationReady(value) {
      novaPersistenceHydrationReady = Boolean(value);
      return novaPersistenceHydrationReady;
    }

    function novaPersistenceIsHydrationReady() {
      return novaPersistenceHydrationReady;
    }

    function novaBuildUiStateSnapshot() {
      return {
        messages: state.messages.slice(-APP_CONFIG.maxMessages),
        thinking: state.thinking ? { history: (state.thinking.history || []).slice(-30), currentTurn: null } : null,
        developerTrace: state.developerTrace ? { history: (state.developerTrace.history || []).slice(-100) } : null
      };
    }

    function novaBuildPersistentStateSnapshot() {
      if (window.__TAURI__?.core?.invoke) return novaBuildUiStateSnapshot();
      return {
        messages: state.messages.slice(-APP_CONFIG.maxMessages),
        thinking: state.thinking ? { history: (state.thinking.history || []).slice(-30), currentTurn: null } : null,
        developerTrace: state.developerTrace ? { history: (state.developerTrace.history || []).slice(-100) } : null,
        // Current persistent knowledge lives under context. The old flat fact
        // arrays remain session-only compatibility state and are never a second
        // durable knowledge authority.
        context: state.context,
        persistence: {
          schemaVersion: 3,
          appVersion: (typeof V41_SELF_MODEL === "object" && V41_SELF_MODEL?.identity?.version) || null,
          savedAt: Date.now()
        }
      };
    }

    function novaSerializePersistentStateSnapshot() {
      try {
        return JSON.stringify(novaBuildPersistentStateSnapshot());
      } catch (error) {
        console.error("Save serialization failed:", error);
        return null;
      }
    }

    function novaCacheSerializedPersistentState(serialized) {
      if (!serialized) return false;
      // localStorage is only a synchronous cache/fallback. A large learned mind
      // can exceed the browser's localStorage quota, so a cache failure must NOT
      // prevent IndexedDB or the Tauri app-data backend from receiving the save.
      if (typeof nova620CachePersistentState === "function") {
        const cached = nova620CachePersistentState(serialized);
        if (!cached) console.warn("Nova local cache could not store the current state; durable persistence will still be attempted.");
        return cached;
      }
      try {
        localStorage.setItem(APP_CONFIG.storageKey, serialized);
        return true;
      } catch (error) {
        console.warn("Nova local cache save failed; durable persistence will still be attempted:", error);
        return false;
      }
    }

    function saveState() {
      // Script-load migrations run before native/bootstrap hydration. Never let
      // a default frontend state acquire a fresh savedAt and outrank the real
      // durable mind before it has even been loaded.
      if (!novaPersistenceHydrationReady) return true;
      if (window.__TAURI__?.core?.invoke && typeof nova9QueueRustMindCheckpoint === "function") {
        return nova9QueueRustMindCheckpoint(novaBuildUiStateSnapshot());
      }
      const serialized = novaSerializePersistentStateSnapshot();
      if (!serialized) return false;
      const cached = novaCacheSerializedPersistentState(serialized);

      if (typeof nova620QueuePersistentStateSave === "function") {
        nova620QueuePersistentStateSave(serialized);
        return true;
      }

      // During very early script startup the durable facade may not be loaded
      // yet. In that narrow case, a successful cache write is still a valid save
      // and will be migrated by persistence bootstrap.
      return cached;
    }

    async function novaSaveDurableCheckpoint(reason = "explicit_checkpoint") {
      if (!novaPersistenceHydrationReady) return false;
      if (window.__TAURI__?.core?.invoke && typeof nova9CommitRustMind === "function") {
        const ok = await nova9CommitRustMind(novaBuildUiStateSnapshot(), reason);
        if (!ok) console.error(`Nova Rust-mind checkpoint failed (${reason}).`);
        return Boolean(ok);
      }
      const serialized = novaSerializePersistentStateSnapshot();
      if (!serialized) return false;
      novaCacheSerializedPersistentState(serialized);

      if (typeof nova620CommitPersistentState === "function") {
        const ok = await nova620CommitPersistentState(serialized, reason);
        if (!ok) console.error(`Nova durable checkpoint failed (${reason}).`);
        return Boolean(ok);
      }

      if (typeof nova620QueuePersistentStateSave === "function") {
        nova620QueuePersistentStateSave(serialized);
        if (typeof nova620FlushPersistentState === "function") {
          const ok = await nova620FlushPersistentState();
          return Boolean(ok);
        }
        return true;
      }

      return true;
    }

    function loadState(rawOverride = null) {
      try {
        let saved = null;
        if (rawOverride && typeof rawOverride === "object") saved = rawOverride;
        else {
          let raw = (typeof rawOverride === "string" && rawOverride.trim()) ? rawOverride : null;
          if (!raw) { try { raw = localStorage.getItem(APP_CONFIG.storageKey); } catch (_) { raw = null; } }
          if (!raw) return false;
          saved = JSON.parse(raw);
        }
        if (Array.isArray(saved.messages)) state.messages = saved.messages;
        if (saved.thinking && typeof saved.thinking === "object") state.thinking = { currentTurn: null, history: Array.isArray(saved.thinking.history) ? saved.thinking.history : [] };
        if (saved.developerTrace && typeof saved.developerTrace === "object") state.developerTrace = { history: Array.isArray(saved.developerTrace.history) ? saved.developerTrace.history : [] };

        // Current persistent semantic/cognitive state lives under context.
        // Session-only flat fact arrays are not a second knowledge authority.
        state.facts = [];
        state.inferredFacts = [];

        if (!window.__TAURI__?.core?.invoke && saved.context && typeof saved.context === "object") {
          state.context = { ...state.context, ...saved.context };
        }

        if (!window.__TAURI__?.core?.invoke && typeof v460EnforceSemanticMemoryAuthority === "function") {
          v460EnforceSemanticMemoryAuthority();
        }
        return true;
      } catch (error) {
        console.error("Load failed:", error);
        return false;
      }
    }


    function resetState() {
      state.messages = [];
      state.facts = [];
      state.inferredFacts = [];
      state.thinking = { currentTurn: null, history: [] };
      state.developerTrace = { history: [] };
      state.context = {
        lastEntity: "user",
        lastTopic: null,
        lastIntent: null,
        lastQuestionRelation: null,
        recentEntities: [],

        // Conversational working memory.
        currentTopic: null,
        topicStack: [],
        salience: {},
        turn: 0,
        lastSemanticFrame: null,
        lastResolvedReferences: [],
        lastParseConfidence: 0
      };

      try { localStorage.removeItem(APP_CONFIG.storageKey); } catch (_) { }
      if (typeof nova620ClearPersistentState === "function") {
        nova620ClearPersistentState().catch(() => {});
      }
      // On native Pass 9, nova620ClearPersistentState resets the complete Rust
      // NovaMind and commits the fresh authoritative mind. No second JS reset path.
      if (!window.__TAURI__?.core?.invoke && typeof nova7RustMindResetDevelopmentalState === "function") {
        nova7RustMindResetDevelopmentalState().catch(() => {});
      }
    }



    /* =========================================================
       JS SECTION: FACT MODEL
       Future file: src/brain/facts.js

       A fact has:
       subject  - who/what
       relation - how it relates
       object   - value or another entity
       source   - explicit | inferred
       because  - explanation for inferred facts
       ========================================================= */

    function makeFact(subject, relation, object, options = {}) {
      return {
        id: options.id || (
          crypto.randomUUID
            ? crypto.randomUUID()
            : `${Date.now()}-${Math.random()}`
        ),
        subject: cleanEntity(subject),
        relation: canonical(relation).replace(/\s+/g, "_"),
        object: cleanEntity(object),
        source: options.source || "explicit",
        because: options.because || null,
        createdAt: options.createdAt || Date.now()
      };
    }

    function factKey(fact) {
      return [
        canonical(fact.subject),
        canonical(fact.relation),
        canonical(fact.object)
      ].join("|");
    }

    function factsEqual(a, b) {
      return factKey(a) === factKey(b);
    }

    function getAllFacts() {
      return [...state.facts, ...state.inferredFacts];
    }

    function hasFact(subject, relation, object = null) {
      return getAllFacts().some(fact => {
        const subjectMatches =
          canonical(fact.subject) === canonical(subject);

        const relationMatches =
          canonical(fact.relation) === canonical(relation);

        const objectMatches =
          object === null ||
          canonical(fact.object) === canonical(object);

        return subjectMatches && relationMatches && objectMatches;
      });
    }

    function findFacts({ subject = null, relation = null, object = null } = {}) {
      return getAllFacts().filter(fact => {
        if (
          subject !== null &&
          canonical(fact.subject) !== canonical(subject)
        ) return false;

        if (
          relation !== null &&
          canonical(fact.relation) !== canonical(relation)
        ) return false;

        if (
          object !== null &&
          canonical(fact.object) !== canonical(object)
        ) return false;

        return true;
      });
    }

    function addExplicitFact(subject, relation, object) {
      const fact = makeFact(subject, relation, object);

      // Handle mutually exclusive user preferences.
      if (relation === "likes") {
        state.facts = state.facts.filter(existing =>
          !(
            canonical(existing.subject) === canonical(subject) &&
            canonical(existing.relation) === "dislikes" &&
            canonical(existing.object) === canonical(object)
          )
        );
      }

      if (relation === "dislikes") {
        state.facts = state.facts.filter(existing =>
          !(
            canonical(existing.subject) === canonical(subject) &&
            canonical(existing.relation) === "likes" &&
            canonical(existing.object) === canonical(object)
          )
        );
      }

      // Replace single-value relations instead of stacking contradictions.
      const singleValueRelations = new Set([
        "name",
        "lives_in",
        "works_at",
        "age",
        "job"
      ]);

      if (singleValueRelations.has(relation)) {
        state.facts = state.facts.filter(existing =>
          !(
            canonical(existing.subject) === canonical(subject) &&
            canonical(existing.relation) === canonical(relation)
          )
        );
      }

      if (!state.facts.some(existing => factsEqual(existing, fact))) {
        state.facts.push(fact);
      }

      runInference();
      saveState();
      return fact;
    }



    /* =========================================================
       V4.2 SECTION: MINI DOCUMENT LEARNING
       Future files:
         src/learning/document-reader.js
         src/learning/document-learner.js

       Files are read locally in the browser.
       Supported now:
         .txt
         .md

       The learner:
         1. splits the document into sentence-like chunks
         2. runs each chunk through the normal semantic analyzer
         3. commits only trusted asserted statements
         4. skips questions / hypotheticals / uncertain statements
         5. records what it could not learn instead of guessing
       ========================================================= */

    function v42SplitLearningDocument(rawText) {
      return String(rawText)
        .replace(/\r\n?/g, "\n")
        .replace(/^#{1,6}\s+/gm, "")
        .split(/(?<=[.!?])\s+|\n+/)
        .map(part => normalizeWhitespace(part))
        .filter(part => part.length >= 2);
    }

    function v42LearnSentence(sentence, fileName) {
      const semantic = v4SemanticAnalyze(sentence);

      const report = {
        sentence,
        type: semantic.type,
        modality: semantic.modality?.type || null,
        confidence: semantic.confidence ?? 0,
        learnedFacts: 0,
        reason: null
      };

      if (semantic.type !== "statement") {
        report.reason = `not an asserted statement (${semantic.type})`;
        return report;
      }

      if (
        semantic.modality &&
        !["asserted", "ability"].includes(semantic.modality.type)
      ) {
        report.reason = `not committed because modality is ${semantic.modality.type}`;
        return report;
      }

      const before = new Set(
        state.facts.map(factKey)
      );

      const extracted = v4CommitAssertion(
        sentence,
        semantic
      );

      if (!extracted.length) {
        report.reason =
          "the parser could not turn it into a trusted fact";
        return report;
      }

      const extractedKeys = new Set(
        extracted.map(fact => factKey(fact))
      );

      for (const stored of state.facts) {
        if (
          extractedKeys.has(factKey(stored)) &&
          !before.has(factKey(stored))
        ) {
          stored.source = `document:${fileName}`;
          stored.learnedAt = Date.now();
          report.learnedFacts++;
        }
      }

      if (report.learnedFacts === 0) {
        report.reason = "already known or duplicated";
      }

      return report;
    }

    async function v42LearnFromFile(file) {
      const rawText = await file.text();

      const maxCharacters = 250000;

      if (rawText.length > maxCharacters) {
        throw new Error(
          `The mini learner currently accepts up to ${maxCharacters.toLocaleString()} characters per file.`
        );
      }

      const sentences =
        v42SplitLearningDocument(rawText);

      const results = [];

      for (const sentence of sentences) {
        results.push(
          v42LearnSentence(sentence, file.name)
        );
      }

      runInference();

      const summary = {
        fileName: file.name,
        characters: rawText.length,
        sentenceCount: sentences.length,
        learnedFacts: results.reduce(
          (sum, result) => sum + result.learnedFacts,
          0
        ),
        learnedSentences: results.filter(
          result => result.learnedFacts > 0
        ).length,
        skippedSentences: results.filter(
          result => result.learnedFacts === 0
        ).length,
        results: results.slice(-100)
      };

      ensureV4Context();
      state.context.lastLearningReport = summary;

      v41RememberResponsePlan(
        v41Plan("document_learning_complete", {
          fileName: summary.fileName,
          learnedFacts: summary.learnedFacts,
          learnedSentences: summary.learnedSentences,
          skippedSentences: summary.skippedSentences
        })
      );

      saveState();
      return summary;
    }

    function v42DescribeLearningSummary(summary) {
      const learned =
        summary.learnedFacts > 0
          ? `I learned ${summary.learnedFacts} new fact${summary.learnedFacts === 1 ? "" : "s"} from ${summary.learnedSentences} sentence${summary.learnedSentences === 1 ? "" : "s"}`
          : `I couldn't safely turn any of it into new factual knowledge`;

      const skipped =
        summary.skippedSentences > 0
          ? ` I skipped ${summary.skippedSentences} sentence${summary.skippedSentences === 1 ? "" : "s"} that were unsupported, non-factual, hypothetical, uncertain, or already known.`
          : "";

      return `I finished reading ${summary.fileName}. ${learned}.${skipped}`;
    }




    /* =========================================================
       V4.3 SECTION: COGNITIVE STATE
       Future files:
         src/cognition/state.js
         src/cognition/working-memory.js
         src/cognition/goals.js
         src/cognition/open-loops.js
         src/memory/index.js

       V4.3 introduces a small executive/cognitive layer.

       The important change:
         "I do not know the answer"
       is no longer equivalent to:
         "I cannot understand the situation."

       Nova can keep:
         - a current problem model
         - active goals
         - pending/open loops
         - working memory
         - source/document memory
         - semantic memory
         - linguistic memory
         - style memory
         - abstracted/summarized memory
         - episodic memory
         - procedural memory
       ========================================================= */

    function v43NewId(prefix = "item") {
      return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
    }

    function v43EnsureCognition() {
      ensureV4Context();

      if (!state.context.cognition || typeof state.context.cognition !== "object") {
        state.context.cognition = {};
      }

      const c = state.context.cognition;

      c.workingMemory ||= {
        currentProblem: null,
        currentTask: null,
        pendingDocument: null,
        threadSummary: null,
        recentObservations: []
      };

      c.goals ||= [];
      c.openLoops ||= [];

      c.memory ||= {
        episodic: [],
        semantic: [],
        linguistic: [],
        style: [],
        procedural: [],
        abstracted: [],
        sources: []
      };

      c.lastAttention ||= null;
      c.lastRetrieval ||= null;
      c.lastPlan ||= null;
      c.lastEvaluation ||= null;
      c.lastDocumentDecision ||= null;

      return c;
    }

    function v43Memory() {
      return v43EnsureCognition().memory;
    }

    function v43WorkingMemory() {
      return v43EnsureCognition().workingMemory;
    }

    function v43ActiveGoals() {
      return v43EnsureCognition().goals.filter(goal => goal.status === "active");
    }

    function v43ActiveGoal(type = null) {
      const active = v43ActiveGoals();
      if (!type) return active.at(-1) || null;
      return [...active].reverse().find(goal => goal.type === type) || null;
    }

    function v43OpenLoops(status = "waiting") {
      return v43EnsureCognition().openLoops.filter(loop =>
        !status || loop.status === status
      );
    }

    function v43BeginHandledTurn() {
      ensureV4Context();
      state.context.turn += 1;

      if (typeof v4DecaySalience === "function") {
        v4DecaySalience();
      }

      return state.context.turn;
    }

    function v43RememberEpisode(kind, description, extra = {}) {
      const memory = v43Memory();

      memory.episodic.push({
        id: v43NewId("episode"),
        kind,
        description,
        turn: state.context.turn,
        createdAt: Date.now(),
        ...extra
      });

      memory.episodic = memory.episodic.slice(-160);
    }

    function v43CreateGoal(type, description, options = {}) {
      const c = v43EnsureCognition();

      const existing = [...c.goals].reverse().find(goal =>
        goal.status === "active" &&
        goal.type === type &&
        canonical(goal.subject || "") === canonical(options.subject || "")
      );

      if (existing) {
        existing.description = description || existing.description;
        existing.updatedTurn = state.context.turn;
        existing.confidence = Math.max(
          existing.confidence || 0,
          options.confidence || 0
        );
        return existing;
      }

      const goal = {
        id: v43NewId("goal"),
        type,
        description,
        subject: options.subject || null,
        status: "active",
        confidence: options.confidence ?? 0.8,
        createdTurn: state.context.turn,
        updatedTurn: state.context.turn,
        source: options.source || "conversation"
      };

      c.goals.push(goal);
      c.goals = c.goals.slice(-60);
      return goal;
    }

    function v43CreateOpenLoop(type, description, options = {}) {
      const c = v43EnsureCognition();

      const existing = [...c.openLoops].reverse().find(loop =>
        loop.status === "waiting" &&
        loop.type === type &&
        canonical(loop.subject || "") === canonical(options.subject || "")
      );

      if (existing) {
        existing.description = description || existing.description;
        existing.updatedTurn = state.context.turn;
        return existing;
      }

      const loop = {
        id: v43NewId("loop"),
        type,
        description,
        subject: options.subject || null,
        expectedKind: options.expectedKind || null,
        purpose: options.purpose || null,
        goalId: options.goalId || null,
        status: "waiting",
        createdTurn: state.context.turn,
        updatedTurn: state.context.turn
      };

      c.openLoops.push(loop);
      c.openLoops = c.openLoops.slice(-80);
      return loop;
    }

    function v43ResolveOpenLoop(loopId, resolution = "completed") {
      const loop = v43EnsureCognition().openLoops.find(item => item.id === loopId);
      if (!loop) return null;

      loop.status = resolution;
      loop.resolvedTurn = state.context.turn;
      loop.resolvedAt = Date.now();
      return loop;
    }



    /* =========================================================
       V4.3 SECTION: DOCUMENT MODEL / SOURCE MEMORY
       Future files:
         src/documents/intake.js
         src/documents/classifier.js
         src/memory/source-memory.js
       ========================================================= */

    function v43TextLikeFile(file) {
      const name = file.name || "";

      return (
        /\.(?:txt|md|markdown|log|csv|json)$/i.test(name) ||
        /^text\//i.test(file.type || "") ||
        ["application/json"].includes(file.type)
      );
    }

    function v43ClassifyDocument(name, rawText) {
      const title = canonical(name);
      const sample = canonical(rawText.slice(0, 30000));

      const scores = {
        manual: 0,
        language: 0,
        style: 0,
        factual: 0,
        notes: 0
      };

      const add = (type, amount, pattern, haystack = sample) => {
        if (pattern.test(haystack)) scores[type] += amount;
      };

      add("manual", 4, /\bmanual\b/, title);
      add("manual", 2, /\bguide\b|\binstructions?\b/, title);
      add("manual", 3, /\bwarning\b|\bcaution\b|\btroubleshoot|\bmaintenance\b|\boperation\b/);
      add("manual", 2, /\bpress\b|\bhold\b|\binstall\b|\breset\b/);

      add("language", 4, /\bslang\b|\bdefinition\b|\bvocabulary\b/, title);
      add("language", 3, /\bmeans?\b|\bis used to mean\b|\bmeaning:/);
      add("language", 2, /\b(?:example|examples):/);

      add("style", 4, /\baccent\b|\bspeech\b|\bstyle\b/, title);
      add("style", 3, /\bpronounc|\bphrasing\b|\bwording\b|\btends to say\b/);
      if ((rawText.match(/^[A-Za-z][A-Za-z0-9 _'-]{1,30}:\s+.+$/gm) || []).length >= 3) {
        scores.style += 4;
      }

      if ((rawText.match(/\b(?:is|are|was|were|began|ended|means|includes)\b/gi) || []).length >= 4) {
        scores.factual += 2;
      }

      if (/^\s*[-*]\s+/m.test(rawText) || /^#{1,6}\s+/m.test(rawText)) {
        scores.notes += 1;
      }

      const ranked = Object.entries(scores)
        .sort((a, b) => b[1] - a[1]);

      return {
        type: ranked[0][1] > 0 ? ranked[0][0] : "unknown",
        scores,
        confidence:
          ranked[0][1] <= 0
            ? 0.2
            : Math.min(
                0.98,
                0.55 + (ranked[0][1] - (ranked[1]?.[1] || 0)) * 0.06
              )
      };
    }

    function v43StoreSource(name, rawText, classification, options = {}) {
      const memory = v43Memory();

      const existing = [...memory.sources].reverse().find(source =>
        canonical(source.name) === canonical(name) &&
        source.characterCount === rawText.length
      );

      if (existing) return existing;

      const maxStoredCharacters = 120000;

      const source = {
        id: v43NewId("source"),
        name,
        type: classification.type,
        classification,
        characterCount: rawText.length,
        text: rawText.slice(0, maxStoredCharacters),
        sourceTextTruncated: rawText.length > maxStoredCharacters,
        createdAt: Date.now(),
        receivedTurn: state.context.turn,
        purpose: options.purpose || null,
        goalId: options.goalId || null
      };

      memory.sources.push(source);
      memory.sources = memory.sources.slice(-24);

      return source;
    }

    function v43InferDocumentPurpose(source) {
      const c = v43EnsureCognition();
      const waiting = v43OpenLoops("waiting").filter(loop =>
        loop.type === "expected_document"
      );

      if (waiting.length === 1) {
        return {
          action: "analyze_for_goal",
          confidence: 0.97,
          reason: "A single document was explicitly expected earlier in the conversation.",
          goalId: waiting[0].goalId,
          openLoopId: waiting[0].id,
          purpose: waiting[0].purpose
        };
      }

      if (waiting.length > 1) {
        const filenameWords = v43ContentWords(source.name);

        const scored = waiting
          .map(loop => ({
            loop,
            score: v43ContentWords(
              `${loop.subject || ""} ${loop.description || ""}`
            ).filter(word => filenameWords.includes(word)).length
          }))
          .sort((a, b) => b.score - a.score);

        if (
          scored[0]?.score > 0 &&
          scored[0].score > (scored[1]?.score || 0)
        ) {
          return {
            action: "analyze_for_goal",
            confidence: 0.9,
            reason: "The filename matches one pending document expectation better than the others.",
            goalId: scored[0].loop.goalId,
            openLoopId: scored[0].loop.id,
            purpose: scored[0].loop.purpose
          };
        }

        return {
          action: "ask",
          confidence: 0.35,
          reason: "More than one document is currently expected."
        };
      }

      const problem = c.workingMemory.currentProblem;
      const goal = v43ActiveGoal("understand_problem");

      if (
        problem &&
        goal &&
        (
          source.classification.type === "manual" ||
          /\bmanual\b|\bguide\b|\binstruction/i.test(source.name)
        )
      ) {
        return {
          action: "analyze_for_goal",
          confidence: 0.78,
          reason: "A manual-like document arrived while a problem-understanding goal is active.",
          goalId: goal.id,
          purpose: `Use the document to improve understanding of the ${problem.subject} problem`
        };
      }

      return {
        action: "ask",
        confidence: 0.45,
        reason:
          source.classification.type === "unknown"
            ? "The document's purpose is not clear from context or content."
            : `The document appears to be ${source.classification.type}-oriented, but the user's intended task is not clear.`
      };
    }



    /* =========================================================
       V4.3 SECTION: GENERAL LEARNING UNITS
       Future file: src/learning/general-learner.js

       A document can teach more than facts.

       Learning units:
         fact
         definition
         language_definition
         usage_example
         style_rule
         style_example
         procedure
         warning
         observation
       ========================================================= */

    function v43PushMemory(bucket, item, max = 500) {
      const memory = v43Memory();
      memory[bucket] ||= [];

      const key = canonical(
        [
          item.type,
          item.term,
          item.text,
          item.summary,
          item.sourceId
        ].filter(Boolean).join("|")
      );

      const exists = memory[bucket].some(existing =>
        canonical(
          [
            existing.type,
            existing.term,
            existing.text,
            existing.summary,
            existing.sourceId
          ].filter(Boolean).join("|")
        ) === key
      );

      if (!exists) {
        memory[bucket].push(item);
        memory[bucket] = memory[bucket].slice(-max);
      }

      return !exists;
    }

    function v43ExtractQuotedPair(text) {
      const quoted = [...String(text).matchAll(/["“](.+?)["”]/g)]
        .map(match => match[1]);

      return quoted;
    }

    function v43ClassifyLearningUnit(sentence, learningContext = {}) {
      const clean = normalizeWhitespace(sentence);

      let match = clean.match(
        /^["“]?([^"”]{1,80})["”]?\s+(?:means|can mean|refers to|is used to mean)\s+(.+)$/i
      );

      if (match) {
        return {
          type: "language_definition",
          term: normalizeWhitespace(match[1]),
          meaning: normalizeWhitespace(match[2]),
          text: clean,
          confidence: 0.96
        };
      }

      match = clean.match(/^meaning:\s*(.+)$/i);

      if (match && learningContext.lastTerm) {
        return {
          type: "language_definition",
          term: learningContext.lastTerm,
          meaning: normalizeWhitespace(match[1]),
          text: clean,
          confidence: 0.92
        };
      }

      if (
        /^(?:nova|the assistant)\s+(?:should|should not|shouldn't|may|can|must|must not)\b/i.test(clean) ||
        /^before\s+(?:answering|responding|saying)\b/i.test(clean) ||
        /^only after\b/i.test(clean) ||
        /^a useful (?:answer|response|question|strategy)\b/i.test(clean) ||
        /^a good default\b/i.test(clean) ||
        /^the goal is\b/i.test(clean) ||
        /\bshould usually be a last resort\b/i.test(clean) ||
        /\bshould not automatically\b/i.test(clean)
      ) {
        return {
          type: "guidance_rule",
          text: clean,
          confidence: 0.93
        };
      }

      if (
        /\b(?:pronounc|accent|speech|phrasing|wording|tends to say|usually says|speaks?)\b/i.test(clean)
      ) {
        return {
          type: "style_rule",
          text: clean,
          confidence: 0.82
        };
      }

      const dialogue = clean.match(
        /^([A-Za-z][A-Za-z0-9 _'-]{1,40}):\s*(.+)$/
      );

      if (dialogue) {
        return {
          type: "style_example",
          speaker: normalizeWhitespace(dialogue[1]),
          example: normalizeWhitespace(dialogue[2]),
          text: clean,
          confidence: 0.9
        };
      }

      if (/^(?:example|examples):?/i.test(clean) || v43ExtractQuotedPair(clean).length) {
        return {
          type: "usage_example",
          term: learningContext.lastTerm || null,
          text: clean,
          confidence: 0.72
        };
      }

      if (/\b(?:warning|caution|danger)\b/i.test(clean) || /\bnever\b|\bdo not\b|\bmust not\b/i.test(clean)) {
        return {
          type: "warning",
          text: clean,
          confidence: 0.9
        };
      }

      if (
        /\b(?:press|hold|select|choose|install|remove|clean|reset|restart|turn off|turn on|connect|disconnect)\b/i.test(clean) ||
        /\b(?:first|next|then|finally)\b/i.test(clean)
      ) {
        return {
          type: "procedure",
          text: clean,
          confidence: 0.76
        };
      }

      const semantic = v4SemanticAnalyze(clean);

      if (
        semantic.type === "statement" &&
        ["asserted", "ability"].includes(semantic.modality.type)
      ) {
        return {
          type: "fact",
          text: clean,
          confidence: semantic.confidence ?? 0.7
        };
      }

      return {
        type: "observation",
        text: clean,
        confidence: 0.45
      };
    }

    function v43LearnGeneralDocument(source, options = {}) {
      const chunks = v42SplitLearningDocument(source.text);
      const learningContext = {
        lastTerm: null,
        styleSpeakers: {}
      };

      let structuredFacts = 0;
      let memoryUnits = 0;

      for (const chunk of chunks) {
        const unit = v43ClassifyLearningUnit(
          chunk,
          learningContext
        );

        const base = {
          id: v43NewId("memory"),
          sourceId: source.id,
          sourceName: source.name,
          learnedAt: Date.now(),
          confidence: unit.confidence,
          text: unit.text,
          type: unit.type
        };

        if (unit.type === "language_definition") {
          learningContext.lastTerm = unit.term;

          memoryUnits += v43PushMemory(
            "linguistic",
            {
              ...base,
              term: unit.term,
              meaning: unit.meaning,
              register:
                /\bslang\b|\binformal\b/i.test(unit.text)
                  ? "informal"
                  : "unspecified",
              examples: []
            }
          ) ? 1 : 0;

          continue;
        }

        if (unit.type === "usage_example") {
          const added = v43PushMemory(
            "linguistic",
            {
              ...base,
              term: unit.term,
              role: "example"
            }
          );

          if (added) memoryUnits++;
          continue;
        }

        if (unit.type === "style_rule") {
          memoryUnits += v43PushMemory(
            "style",
            {
              ...base,
              profile:
                options.styleProfile ||
                source.name.replace(/\.[^.]+$/, "")
            }
          ) ? 1 : 0;

          continue;
        }

        if (unit.type === "style_example") {
          learningContext.styleSpeakers[unit.speaker] =
            (learningContext.styleSpeakers[unit.speaker] || 0) + 1;

          memoryUnits += v43PushMemory(
            "style",
            {
              ...base,
              profile: unit.speaker,
              speaker: unit.speaker,
              example: unit.example
            }
          ) ? 1 : 0;

          continue;
        }

        if (
          unit.type === "procedure" ||
          unit.type === "warning" ||
          unit.type === "guidance_rule"
        ) {
          memoryUnits += v43PushMemory(
            "procedural",
            {
              ...base,
              guidance:
                unit.type === "guidance_rule"
                  ? v431CompileGuidanceRule(unit.text)
                  : null
            }
          ) ? 1 : 0;
        } else {
          memoryUnits += v43PushMemory(
            "semantic",
            base
          ) ? 1 : 0;
        }

        if (unit.type === "fact") {
          const semantic = v4SemanticAnalyze(chunk);
          const before = state.facts.length;
          const extracted = v4CommitAssertion(chunk, semantic);

          if (extracted.length) {
            const newFacts = state.facts.slice(before);

            for (const fact of newFacts) {
              fact.source = `document:${source.name}`;
              fact.sourceId = source.id;
              fact.learnedAt = Date.now();
            }

            structuredFacts += extracted.length;
          }
        }
      }

      runInference();

      const styleEntries = v43Memory().style.filter(
        item => item.sourceId === source.id
      );

      if (styleEntries.length) {
        v43BuildStyleProfilesFromSource(source.id);
      }

      return {
        sourceId: source.id,
        chunks: chunks.length,
        memoryUnits,
        structuredFacts
      };
    }

    function v43BuildStyleProfilesFromSource(sourceId) {
      const memory = v43Memory();
      const examples = memory.style.filter(item =>
        item.sourceId === sourceId &&
        item.type === "style_example" &&
        item.example
      );

      const bySpeaker = new Map();

      for (const item of examples) {
        const key = item.profile || item.speaker || "Unknown speaker";
        if (!bySpeaker.has(key)) bySpeaker.set(key, []);
        bySpeaker.get(key).push(item.example);
      }

      for (const [profile, lines] of bySpeaker.entries()) {
        const words = lines
          .flatMap(line => v43ContentWords(line));

        const contractions = lines.reduce(
          (count, line) =>
            count +
            ((line.match(/\b\w+'\w+\b/g) || []).length),
          0
        );

        const questions = lines.filter(line => /\?$/.test(line.trim())).length;
        const exclamations = lines.filter(line => /!$/.test(line.trim())).length;

        const frequent = Object.entries(
          words.reduce((map, word) => {
            map[word] = (map[word] || 0) + 1;
            return map;
          }, {})
        )
          .filter(([, count]) => count >= 2)
          .sort((a, b) => b[1] - a[1])
          .slice(0, 10)
          .map(([word]) => word);

        v43PushMemory(
          "style",
          {
            id: v43NewId("style_profile"),
            type: "style_profile",
            profile,
            sourceId,
            learnedAt: Date.now(),
            text: `Learned style profile for ${profile}`,
            averageWordsPerExample:
              lines.length
                ? Math.round(
                    lines.reduce(
                      (sum, line) =>
                        sum + line.trim().split(/\s+/).length,
                      0
                    ) / lines.length
                  )
                : 0,
            contractionCount: contractions,
            questionRate: lines.length ? questions / lines.length : 0,
            exclamationRate: lines.length ? exclamations / lines.length : 0,
            frequentWords: frequent,
            exampleCount: lines.length,
            confidence: Math.min(0.95, 0.45 + lines.length * 0.04)
          }
        );
      }
    }


    /* =========================================================
       V4.3.12 SECTION: PERSISTENT KNOWLEDGE ACCESS
       Persistent Memory owns what Nova has learned.
       ========================================================= */

    const NOVA4312_STOP = new Set([
      "the","a","an","and","or","but","to","of","for","in","on","at","with",
      "from","by","is","are","was","were","be","been","being","it","this",
      "that","these","those","i","you","he","she","they","we","my","your",
      "his","her","their","our","as","if","then","than","so","do","does",
      "did","have","has","had","can","could","will","would","should","may",
      "might","about","into","what","when","where","why","how","tell","learn",
      "learned","knowledge","document","documents","lesson","lessons","notes",
      "information","thing","things","first","second","third"
    ]);

    function nova4312Stem(word) {
      let w =
        canonical(word || "")
          .replace(/[^a-z0-9'-]/g, "");

      if (
        w.length > 5 &&
        /ies$/.test(w)
      ) {
        w =
          w.slice(0, -3) +
          "y";
      } else if (
        w.length > 4 &&
        /s$/.test(w) &&
        !/(?:ss|us|is)$/.test(w)
      ) {
        w =
          w.slice(0, -1);
      }

      return w;
    }

    function nova4312Words(text) {
      return canonical(text || "")
        .replace(/[^a-z0-9'-]+/g, " ")
        .split(/\s+/)
        .map(
          nova4312Stem
        )
        .filter(word =>
          word.length >= 3 &&
          !NOVA4312_STOP.has(word)
        );
    }

    function nova4312ItemText(item) {
      return normalizeWhitespace(
        [
          item?.term,
          item?.meaning,
          item?.text,
          item?.summary,
          ...(item?.points || []).map(point =>
            typeof point === "string"
              ? point
              : (
                  point?.sourceText ||
                  point?.text ||
                  ""
                )
          )
        ]
          .filter(Boolean)
          .join(" ")
      );
    }

    function nova4312SourceTopic(source) {
      const title =
        String(
          source?.name ||
          ""
        )
          .replace(/\.[^.]+$/, "")
          .replace(/[_-]+/g, " ")
          .replace(/\s+/g, " ")
          .trim();

      if (!title) {
        return null;
      }

      return title
        .split(
          " "
        )
        .map((word, index) => {
          if (
            /^[A-Z0-9]{2,}$/.test(
              word
            )
          ) {
            return word;
          }

          if (
            /^[A-Z][a-z]+$/.test(
              word
            )
          ) {
            return word.toLowerCase();
          }

          if (
            index === 0
          ) {
            return (
              word.charAt(0).toLowerCase() +
              word.slice(1)
            );
          }

          return word;
        })
        .join(
          " "
        );
    }

    function nova4312SourceItems(sourceId) {
      const memory =
        v43Memory();

      const items = [];

      for (const bucket of [
        "semantic",
        "abstracted",
        "linguistic"
      ]) {
        for (const item of memory[bucket] || []) {
          if (
            item.sourceId !==
              sourceId
          ) {
            continue;
          }

          const role =
            typeof v436MemoryRole ===
              "function"
              ? v436MemoryRole(
                  item,
                  bucket
                )
              : "content";

          if (
            [
              "control",
              "style",
              "example"
            ].includes(
              role
            )
          ) {
            continue;
          }

          items.push({
            bucket,
            item
          });
        }
      }

      return items;
    }

    function nova4312Overlap(a, b) {
      const aa =
        new Set(
          nova4312Words(a)
        );

      const bb =
        new Set(
          nova4312Words(b)
        );

      let score = 0;

      for (const word of aa) {
        if (
          bb.has(
            word
          )
        ) {
          score += 1;
        }
      }

      return score;
    }

    function nova4312LearnedTopics() {
      const memory =
        v43Memory();

      const topics = [];

      for (const source of memory.sources || []) {
        const items =
          nova4312SourceItems(
            source.id
          );

        if (!items.length) {
          continue;
        }

        topics.push({
          topic:
            nova4312SourceTopic(
              source
            ),
          sourceId:
            source.id,
          source,
          items,
          learnedAt:
            source.createdAt ||
            0
        });
      }

      // Facts taught in conversation can also become learned topics.
      const groups =
        new Map();

      for (const fact of state.facts || []) {
        if (
          fact.sourceId ||
          !fact.subject ||
          /^(?:user|you|nova|assistant|i)$/i.test(
            fact.subject
          )
        ) {
          continue;
        }

        const key =
          canonical(
            fact.subject
          );

        if (
          !groups.has(
            key
          )
        ) {
          groups.set(
            key,
            []
          );
        }

        groups.get(
          key
        ).push(
          fact
        );
      }

      for (const facts of groups.values()) {
        topics.push({
          topic:
            facts[0].subject,
          sourceId:
            null,
          source:
            null,
          items:
            [],
          facts,
          learnedAt:
            Math.max(
              ...facts.map(fact =>
                fact.learnedAt ||
                fact.createdAt ||
                0
              )
            )
        });
      }

      return topics
        .filter(item =>
          item.topic
        )
        .sort(
          (a, b) =>
            b.learnedAt -
            a.learnedAt
        );
    }

    function nova4312FindLearnedTopic(query) {
      return nova4312LearnedTopics()
        .map(item => ({
          ...item,
          score:
            nova4312Overlap(
              query,
              item.topic
            ) *
              4 +
            (
              canonical(
                query
              ).includes(
                canonical(
                  item.topic
                )
              ) ||
              canonical(
                item.topic
              ).includes(
                canonical(
                  query
                )
              )
                ? 6
                : 0
            )
        }))
        .filter(item =>
          item.score > 0
        )
        .sort(
          (a, b) =>
            b.score -
              a.score ||
            b.learnedAt -
              a.learnedAt
        )[0] ||
        null;
    }

    function nova4312Concepts(items, topic = "", limit = 7) {
      const stop =
        new Set([
          ...NOVA4312_STOP,
          ...nova4312Words(
            topic
          ),
          "completed",
          "appeared",
          "developed",
          "created",
          "built",
          "introduced",
          "became",
          "began",
          "started",
          "used",
          "using",
          "made",
          "many",
          "later",
          "commercial"
        ]);

      const counts =
        new Map();

      const display =
        new Map();

      for (const entry of items) {
        const raw =
          nova4312ItemText(
            entry.item ||
            entry
          );

        for (const word of raw.match(/[A-Za-z][A-Za-z0-9'-]{2,}/g) || []) {
          const stem =
            nova4312Stem(
              word
            );

          if (
            !stem ||
            stop.has(
              stem
            )
          ) {
            continue;
          }

          counts.set(
            stem,
            (
              counts.get(
                stem
              ) ||
              0
            ) +
              1
          );

          const old =
            display.get(
              stem
            );

          if (
            !old ||
            (
              /^[A-Z]/.test(
                word
              ) &&
              !/^[A-Z]/.test(
                old
              )
            )
          ) {
            display.set(
              stem,
              word
            );
          }
        }
      }

      return [
        ...counts.entries()
      ]
        .sort(
          (a, b) =>
            b[1] -
            a[1]
        )
        .slice(
          0,
          limit
        )
        .map(
          ([stem]) =>
            display.get(
              stem
            ) ||
            stem
        );
    }

    function nova4312Years(items) {
      const years = [];

      for (const entry of items) {
        const raw =
          nova4312ItemText(
            entry.item ||
            entry
          );

        for (const match of raw.matchAll(/\b(1[5-9]\d{2}|20\d{2}|2100)\b/g)) {
          if (
            !years.includes(
              match[1]
            )
          ) {
            years.push(
              match[1]
            );
          }
        }
      }

      return years.sort(
        (a, b) =>
          Number(a) -
          Number(b)
      );
    }

    function nova4312TopicBundle(query) {
      const match =
        nova4312FindLearnedTopic(
          query
        );

      if (!match) {
        return null;
      }

      const facts =
        match.sourceId
          ? (
              state.facts ||
              []
            ).filter(fact =>
              fact.sourceId ===
                match.sourceId
            )
          : (
              match.facts ||
              []
            );

      return {
        topic:
          match.topic,
        items:
          match.items ||
          [],
        facts,
        years:
          nova4312Years(
            match.items ||
            []
          ),
        concepts:
          nova4312Concepts(
            match.items ||
            [],
            match.topic,
            7
          ),
        learnedAt:
          match.learnedAt
      };
    }



    /* =========================================================
       V4.4.0 SECTION: SEMANTIC MEMORY CORE / MEANING UNITS

       Source text is evidence. Nova reasons over typed meaning units.
       The old memory buckets remain for compatibility while V4.4 migrates
       retrieval and execution toward this representation.
       ========================================================= */

    const NOVA440_SCHEMA_VERSION = 1;

    const NOVA440_MEANING_STOP = new Set([
      "the","a","an","and","or","but","to","of","for","in","on","at",
      "with","from","by","is","are","was","were","be","been","being",
      "it","this","that","these","those","i","you","he","she","they",
      "we","my","your","his","her","their","our","as","if","then",
      "than","so","do","does","did","have","has","had","can","could",
      "will","would","should","may","might","about","into","what","when",
      "where","why","how","tell","learn","learned","learning","lesson",
      "document","file","example","examples","something","anything","thing",
      "things","also","usually","often","sometimes","supposed","meant"
    ]);

    function v440EnsureMeaningCore() {
      const memory = v43Memory();

      memory.meaningUnits ||= [];
      memory.meaningCore ||= {
        version: NOVA440_SCHEMA_VERSION,
        migratedSourceIds: [],
        lastLearningReport: null,
        lastRetrieval: null
      };

      if (memory.meaningCore.version !== NOVA440_SCHEMA_VERSION) {
        memory.meaningCore.version = NOVA440_SCHEMA_VERSION;
      }

      return memory.meaningCore;
    }

    function v440MeaningUnits({ migrate = true } = {}) {
      v440EnsureMeaningCore();

      if (migrate) {
        v440MigrateStoredSources();
      }

      return v43Memory().meaningUnits;
    }

    function v440CleanTopic(value) {
      let text = normalizeWhitespace(String(value || ""))
        .replace(/\.[A-Za-z0-9]{1,8}$/i, "")
        .replace(/[_-]+/g, " ")
        .replace(/^nova\s+/i, "")
        .replace(/\b(?:lesson|notes|document|guide)\s*\d*$/i, "")
        .replace(/\s+/g, " ")
        .trim();

      text = text
        .replace(/^(?:a|an|the)\s+/i, "")
        .replace(/\s+(?:are|is)\s+(?:supposed|meant)\s+to\s+work\s+like\s+this.*$/i, "")
        .replace(/\s+works?\s+like\s+this.*$/i, "")
        .trim();

      // Very small English singularization for topic labels, not world facts.
      if (/\bjokes$/i.test(text)) text = text.replace(/jokes$/i, "joke");

      return text || null;
    }

    function v440MeaningTokens(text) {
      return canonical(text || "")
        .replace(/[^a-z0-9'-]+/g, " ")
        .split(/\s+/)
        .map(word => {
          if (word.length > 5 && /ies$/.test(word)) return word.slice(0, -3) + "y";
          if (word.length > 4 && /s$/.test(word) && !/(?:ss|us|is)$/.test(word)) return word.slice(0, -1);
          return word;
        })
        .filter(word => word.length >= 3 && !NOVA440_MEANING_STOP.has(word));
    }

    function v440UnitTopic(unit) {
      return v440CleanTopic(
        unit.topic ||
        unit.name ||
        unit.concept ||
        unit.subject ||
        unit.demonstrates ||
        unit.sourceName ||
        ""
      );
    }

    function v440UnitSignature(unit) {
      const parts = [unit.type];

      if (unit.type === "procedure") {
        parts.push(v440CleanTopic(unit.name));
      } else if (unit.type === "proposition") {
        parts.push(unit.subject, unit.relation, unit.object);
      } else if (unit.type === "property") {
        parts.push(unit.subject, unit.property, unit.value || unit.meaning);
      } else if (unit.type === "event") {
        parts.push(unit.subject, unit.action, unit.object, unit.time);
      } else if (unit.type === "change") {
        parts.push(unit.subject, unit.from, unit.to, unit.time);
      } else if (unit.type === "definition") {
        parts.push(unit.concept, unit.category);
      } else if (unit.type === "guidance") {
        parts.push(unit.target, unit.strength, unit.directive);
      } else if (unit.type === "example") {
        parts.push(unit.demonstrates, JSON.stringify(unit.sequence || unit.content || ""));
      } else {
        parts.push(v440UnitTopic(unit), unit.meaning || unit.description || "");
      }

      return canonical(parts.filter(Boolean).join("|"));
    }

    function v440EvidenceRecord(meta = {}) {
      return {
        sourceId: meta.sourceId || null,
        sourceName: meta.sourceName || null,
        sourceType: meta.sourceType || (meta.sourceId ? "document" : "conversation"),
        text: normalizeWhitespace(meta.text || ""),
        section: meta.section || null,
        paragraphIndex: Number.isInteger(meta.paragraphIndex) ? meta.paragraphIndex : null,
        sentenceIndex: Number.isInteger(meta.sentenceIndex) ? meta.sentenceIndex : null,
        learnedAt: meta.learnedAt || Date.now()
      };
    }

    function v440MergeProcedure(existing, incoming) {
      const oldSequence = existing.sequence || [];
      const newSequence = incoming.sequence || [];

      if (newSequence.length > oldSequence.length || incoming.confidence > (existing.confidence || 0)) {
        existing.sequence = JSON.parse(JSON.stringify(newSequence));
        existing.interactionMode = incoming.interactionMode || existing.interactionMode;
        existing.roles = [...new Set([...(existing.roles || []), ...(incoming.roles || [])])];
        existing.purpose = incoming.purpose || existing.purpose || null;
      }

      existing.requirements = [...new Set([...(existing.requirements || []), ...(incoming.requirements || [])])];
      existing.optionalTraits = [...new Set([...(existing.optionalTraits || []), ...(incoming.optionalTraits || [])])];
      existing.examples ||= [];

      for (const example of incoming.examples || []) {
        const key = canonical(JSON.stringify(example));
        if (!existing.examples.some(item => canonical(JSON.stringify(item)) === key)) {
          existing.examples.push(JSON.parse(JSON.stringify(example)));
        }
      }

      existing.examples = existing.examples.slice(-8);
      existing.confidence = Math.max(existing.confidence || 0, incoming.confidence || 0);
      existing.updatedAt = Date.now();
      existing.revision = (existing.revision || 1) + 1;
    }

    function v440StoreMeaningUnit(unit, evidenceMeta = {}) {
      if (!unit || !unit.type) return null;

      const memory = v43Memory();
      v440EnsureMeaningCore();

      const prepared = {
        id: unit.id || v43NewId("meaning"),
        schemaVersion: NOVA440_SCHEMA_VERSION,
        confidence: unit.confidence ?? 0.76,
        learnedAt: unit.learnedAt || Date.now(),
        updatedAt: Date.now(),
        revision: unit.revision || 1,
        ...unit
      };

      prepared.topic = v440CleanTopic(prepared.topic || v440UnitTopic(prepared));
      prepared.entities = [...new Set((prepared.entities || []).map(value => normalizeWhitespace(value)).filter(Boolean))];
      prepared.tags = [...new Set((prepared.tags || []).filter(Boolean))];
      prepared.evidence ||= [];

      const evidence = v440EvidenceRecord({
        ...evidenceMeta,
        sourceId: evidenceMeta.sourceId ?? prepared.sourceId ?? null,
        sourceName: evidenceMeta.sourceName ?? prepared.sourceName ?? null,
        text: evidenceMeta.text || prepared.evidenceText || prepared.text || ""
      });

      const signature = v440UnitSignature(prepared);
      const existing = memory.meaningUnits.find(item => v440UnitSignature(item) === signature);

      if (existing) {
        if (existing.type === "procedure") {
          v440MergeProcedure(existing, prepared);
        } else {
          existing.confidence = Math.max(existing.confidence || 0, prepared.confidence || 0);
          existing.updatedAt = Date.now();
          existing.revision = (existing.revision || 1) + 1;
          existing.entities = [...new Set([...(existing.entities || []), ...(prepared.entities || [])])];
          existing.tags = [...new Set([...(existing.tags || []), ...(prepared.tags || [])])];
        }

        existing.evidence ||= [];
        const evidenceKey = canonical(`${evidence.sourceId}|${evidence.section}|${evidence.text}`);
        if (evidence.text && !existing.evidence.some(item => canonical(`${item.sourceId}|${item.section}|${item.text}`) === evidenceKey)) {
          existing.evidence.push(evidence);
          existing.evidence = existing.evidence.slice(-12);
        }

        return existing;
      }

      if (evidence.text) prepared.evidence.push(evidence);
      memory.meaningUnits.push(prepared);
      memory.meaningUnits = memory.meaningUnits.slice(-1200);
      return prepared;
    }

    function v440DocumentSegments(rawText) {
      const lines = String(rawText || "").replace(/\r\n?/g, "\n").split("\n");
      const segments = [];
      let section = null;
      let paragraph = [];
      let paragraphIndex = 0;

      function flush() {
        const text = normalizeWhitespace(paragraph.join(" "));
        if (text) {
          segments.push({ section, text, paragraphIndex: paragraphIndex++ });
        }
        paragraph = [];
      }

      for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line) {
          flush();
          continue;
        }

        const heading = line.match(/^#{1,6}\s+(.+)$/) ||
          (line.length <= 90 && /^[A-Z][A-Za-z0-9 '&()\/-]{2,}:?$/.test(line) && !/[.!?]$/.test(line)
            ? [line, line.replace(/:$/, "")]
            : null);

        if (heading) {
          flush();
          section = v440CleanTopic(heading[1]);
          continue;
        }

        paragraph.push(line.replace(/^[-*•]\s+/, ""));
      }

      flush();
      return segments;
    }

    function v440QuotedSpans(text) {
      const spans = [];
      const regex = /["“]([^"”]+)["”]/g;
      let match;
      while ((match = regex.exec(String(text)))) {
        spans.push({ text: normalizeWhitespace(match[1]), index: match.index, end: regex.lastIndex });
      }
      return spans;
    }

    function v440QuoteActor(text, quoteIndex) {
      const before = String(text).slice(Math.max(0, quoteIndex - 130), quoteIndex);
      const assistantMatches = [...before.matchAll(/\b(?:I|Nova|the assistant)\s+(?:would\s+)?(?:say|respond|reply|ask)\b/gi)];
      const userMatches = [...before.matchAll(/\b(?:you|the user|the listener|listener)\s+(?:would\s+)?(?:say|respond|reply|ask)\b/gi)];
      const a = assistantMatches.at(-1)?.index ?? -1;
      const u = userMatches.at(-1)?.index ?? -1;
      if (a < 0 && u < 0) return null;
      return a > u ? "assistant" : "user";
    }

    function v440ProcedureTopic(text, context = {}) {
      const clean = normalizeWhitespace(text);
      const patterns = [
        /^(.{2,100}?)\s+(?:are|is)\s+(?:supposed|meant)\s+to\s+work\s+like\s+this\b/i,
        /^(.{2,100}?)\s+works?\s+like\s+this\b/i,
        /^(?:here(?:'s| is) how|this is how)\s+(.{2,100}?)\s+works?\b/i,
        /^(?:the )?(?:steps|procedure|process)\s+for\s+(.{2,100}?)\s+(?:are|is)\b/i
      ];

      for (const pattern of patterns) {
        const match = clean.match(pattern);
        if (match) return v440CleanTopic(match[1]);
      }

      return v440CleanTopic(context.section || context.topic || context.sourceTopic || null);
    }

    function v440ExtractDialogueProcedure(text, context = {}) {
      const clean = normalizeWhitespace(text);
      const quoted = v440QuotedSpans(clean);
      const topic = v440ProcedureTopic(clean, context);

      const dialogueSignal =
        quoted.length >= 2 &&
        (/\b(?:I|you|listener|user|assistant|Nova)\s+(?:would\s+)?(?:say|respond|reply|ask)\b/i.test(clean) ||
         /\bwho(?:'s| is) there\b/i.test(clean) ||
         /\bfull\s+(?:joke\s+)?example\b/i.test(clean));

      const procedureSignal =
        /\b(?:works? like this|supposed to work like this|steps? are|procedure|process|then|next|finally)\b/i.test(clean);

      if (!topic || (!dialogueSignal && !procedureSignal)) return null;

      const fullExampleIndex = clean.search(/\bfull\s+(?:joke\s+)?example\s*:/i);
      const coreText = fullExampleIndex >= 0 ? clean.slice(0, fullExampleIndex) : clean;
      const coreQuotes = v440QuotedSpans(coreText);
      const sequence = [];
      const roles = new Set();

      for (let index = 0; index < coreQuotes.length; index++) {
        const quote = coreQuotes[index];
        let actor = v440QuoteActor(coreText, quote.index);

        if (!actor && sequence.length) {
          actor = sequence.at(-1).actor === "assistant" ? "user" : "assistant";
        }

        if (!actor && index === 0) actor = "assistant";
        if (!actor) continue;

        roles.add(actor);

        const previous = sequence.at(-1);
        const step = {
          order: sequence.length + 1,
          actor,
          kind: actor === "user" ? "expected_response" : "utterance",
          content: quote.text
        };

        // Generalize a demonstrated setup binding:
        // "Interrupting Cow" -> "Interrupting Cow Who"
        if (
          actor === "user" &&
          previous?.actor === "assistant" &&
          canonical(quote.text).startsWith(canonical(previous.content)) &&
          /\bwho\??$/i.test(quote.text)
        ) {
          previous.kind = "provide_value";
          previous.bind = "setup";
          previous.example = previous.content;
          delete previous.content;
          step.kind = "expected_pattern";
          step.pattern = "[setup] who?";
          step.example = quote.text;
          delete step.content;
        }

        sequence.push(step);
      }

      // Phrases can describe a variable setup even before an example shows it.
      if (/\b(?:something|a word|a phrase|a name)\s+(?:for\s+)?(?:setting up|to set up)\s+the\s+punchline\b/i.test(coreText)) {
        const assistantStep = [...sequence].reverse().find(step => step.actor === "assistant" && step.order >= 3);
        if (assistantStep) {
          assistantStep.kind = "provide_value";
          assistantStep.bind = assistantStep.bind || "setup";
          assistantStep.example ||= assistantStep.content || null;
          delete assistantStep.content;
        }
      }

      const examples = [];
      if (fullExampleIndex >= 0) {
        const exampleQuotes = v440QuotedSpans(clean.slice(fullExampleIndex));
        if (exampleQuotes.length) {
          examples.push({
            type: "dialogue_example",
            sequence: exampleQuotes.map((quote, index) => ({
              actor: index % 2 === 0 ? "assistant" : "user",
              content: quote.text
            }))
          });
        }
      }

      const mentionsPayoff = /\b(?:punchline|payoff|interrupt|joke ends?|finish(?:es)?)\b/i.test(clean);
      const hasAssistantPayoff = sequence.some(step => step.kind === "deliver_payoff");

      if (mentionsPayoff && !hasAssistantPayoff && sequence.length >= 2) {
        const examplePayoff = examples[0]?.sequence?.at(-1)?.content || null;
        sequence.push({
          order: sequence.length + 1,
          actor: "assistant",
          kind: "deliver_payoff",
          relationTo: "setup",
          example: examplePayoff
        });
        roles.add("assistant");
      }

      if (sequence.length < 2) return null;

      const multiTurn = roles.has("assistant") && roles.has("user");
      const requirements = ["ordered_steps"];
      if (multiTurn) requirements.push("wait_for_response");
      if (sequence.some(step => step.bind === "setup")) requirements.push("preserve_setup_binding");
      if (sequence.some(step => step.kind === "deliver_payoff")) requirements.push("payoff_after_setup");

      return {
        type: "procedure",
        name: topic,
        topic,
        purpose: /\bjoke\b/i.test(topic) ? "perform the learned joke format" : null,
        interactionMode: multiTurn ? "multi_turn" : "single_turn",
        roles: [...roles],
        sequence,
        requirements,
        optionalTraits: [],
        examples,
        entities: [topic, ...sequence.flatMap(step => [step.example, step.content].filter(Boolean))],
        tags: ["procedure", multiTurn ? "interactive" : "single_turn"],
        confidence: multiTurn ? 0.97 : 0.84
      };
    }

    function v440ExtractGuidanceUnit(sentence, context = {}) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence));
      let match = clean.match(/^(.{1,120}?)\s+(must|must not|should|should not|shouldn't|can also|may also|can|may|might|could|sometimes)\s+(.+)$/i);
      if (!match) return null;

      const modal = canonical(match[2]);
      const optional = /^(?:can also|may also|can|may|might|could|sometimes)$/.test(modal);
      const forbidden = /not|shouldn't/.test(modal);
      const strength = optional ? "optional" : (/must/.test(modal) ? "required" : "recommended");
      const target = v440CleanTopic(match[1]) || context.topic || context.section || null;

      return {
        type: "guidance",
        topic: target,
        target,
        strength,
        polarity: forbidden ? "negative" : "positive",
        directive: normalizeWhitespace(match[3]),
        entities: [target].filter(Boolean),
        tags: ["guidance", strength],
        confidence: optional ? 0.82 : 0.9
      };
    }

    function v440ExtractDefinitionUnit(sentence, context = {}) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence));
      const match = clean.match(/^(?:a|an|the)?\s*(.{1,100}?)\s+(?:is|are)\s+(?:a|an)\s+(.+)$/i);
      if (!match) return null;

      const concept = v440CleanTopic(match[1]);
      const rhs = normalizeWhitespace(match[2]);
      const words = rhs.split(/\s+/).filter(Boolean);
      const category = words.length ? words.at(-1).replace(/[,.]$/, "") : rhs;
      const properties = words.slice(0, -1).filter(word => !/^(?:very|really|quite|extremely)$/i.test(word));

      return {
        type: "definition",
        topic: concept || context.topic || context.section || null,
        concept,
        category,
        properties,
        entities: [concept, category, ...properties].filter(Boolean),
        tags: ["definition"],
        confidence: 0.86
      };
    }

    function v440ExtractEventUnits(sentence, context = {}) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence));
      const units = [];
      let match;

      match = clean.match(/^In\s+(\d{4}),\s+(.+?)\s+was\s+([A-Za-z][A-Za-z-]*(?:ed|en))\s+by\s+(.+)$/i);
      if (match) {
        units.push({
          type: "event", topic: context.topic || v440CleanTopic(match[2]),
          subject: normalizeWhitespace(match[2]), action: canonical(match[3]), object: normalizeWhitespace(match[4]), time: match[1],
          entities: [match[2], match[4]], tags: ["event","dated"], confidence: 0.92
        });
      }

      match = clean.match(/^(?:In|By)\s+(\d{4}),\s+(.+?)\s+(appeared|arrived|began|started|ended|finished|was completed|was introduced|was developed|was created|was built)(?:\s+(.+))?$/i);
      if (match) {
        units.push({
          type: "event", topic: context.topic || v440CleanTopic(match[2]),
          subject: normalizeWhitespace(match[2]), action: canonical(match[3]).replace(/\s+/g,"_"), object: normalizeWhitespace(match[4] || "") || null, time: match[1],
          entities: [match[2], match[4]].filter(Boolean), tags: ["event","dated"], confidence: 0.9
        });
      }

      match = clean.match(/^(.+?)\s+changed\s+from\s+(.+?)\s+to\s+(.+)$/i);
      if (match) {
        units.push({
          type: "change", topic: context.topic || v440CleanTopic(match[1]),
          subject: normalizeWhitespace(match[1]), from: normalizeWhitespace(match[2]), to: normalizeWhitespace(match[3]), time: (clean.match(/\b(1[5-9]\d{2}|20\d{2})\b/) || [])[1] || null,
          entities: [match[1],match[2],match[3]], tags: ["change","contrast"], confidence: 0.92
        });
      }

      return units;
    }

    function v440ExtractPropositionUnits(sentence, context = {}) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence));
      const semantic = v4SemanticAnalyze(clean);
      if (semantic.type !== "statement" || !["asserted","ability"].includes(semantic.modality?.type)) return [];

      let facts = [];
      try {
        facts = extractFactsFromText(clean) || [];
      } catch (_) {
        facts = [];
      }

      return facts.map(fact => ({
        type: "proposition",
        topic: context.topic || context.section || v440CleanTopic(fact.subject),
        subject: fact.subject,
        relation: fact.relation,
        object: fact.object,
        polarity: semantic.negated ? "negative" : "positive",
        tense: semantic.tense,
        qualifiers: [],
        entities: [fact.subject, fact.object].filter(Boolean),
        tags: ["proposition"],
        confidence: semantic.confidence ?? 0.76
      }));
    }

    function v440ExtractExampleUnit(sentence, context = {}) {
      const clean = normalizeWhitespace(sentence);
      if (!/\b(?:for example|example|full .* example)\b\s*:?/i.test(clean)) return null;
      const quotes = v440QuotedSpans(clean);
      return {
        type: "example",
        topic: context.topic || context.section || null,
        demonstrates: context.topic || context.section || null,
        content: clean,
        sequence: quotes.length >= 2 ? quotes.map((quote, index) => ({ actor: index % 2 === 0 ? "assistant" : "user", content: quote.text })) : [],
        entities: [context.topic || context.section].filter(Boolean),
        tags: ["example"],
        confidence: 0.78
      };
    }

    function v440ExtractMeaningUnitsFromText(text, context = {}) {
      const units = [];
      const sourceTopic = v440CleanTopic(context.sourceTopic || context.sourceName || "");
      const localContext = { ...context, sourceTopic };

      const procedure = v440ExtractDialogueProcedure(text, localContext);
      if (procedure) {
        units.push(procedure);
        localContext.topic = procedure.name;
      }

      const sentences = v42SplitLearningDocument(text);
      for (let sentenceIndex = 0; sentenceIndex < sentences.length; sentenceIndex++) {
        const sentence = sentences[sentenceIndex];
        const sentenceContext = { ...localContext, sentenceIndex };

        const example = v440ExtractExampleUnit(sentence, sentenceContext);
        if (example) units.push(example);

        const guidance = v440ExtractGuidanceUnit(sentence, sentenceContext);
        if (guidance) units.push(guidance);

        const definition = v440ExtractDefinitionUnit(sentence, sentenceContext);
        if (definition) units.push(definition);

        units.push(...v440ExtractEventUnits(sentence, sentenceContext));

        const looksLikeProcedureNarration =
          Boolean(procedure) &&
          /\b(?:I|you|listener|user|assistant|Nova)\s+(?:would\s+)?(?:say|respond|reply|ask)\b/i.test(sentence);

        if (!looksLikeProcedureNarration && !guidance && !example) {
          units.push(...v440ExtractPropositionUnits(sentence, sentenceContext));
        }
      }

      return units;
    }

    function v440LearnDocumentMeanings(source, options = {}) {
      const core = v440EnsureMeaningCore();
      const sourceTopic = v440CleanTopic(source.name);
      const segments = v440DocumentSegments(source.text);
      const counts = {};
      const storedIds = [];

      for (const segment of segments) {
        const context = {
          sourceId: source.id,
          sourceName: source.name,
          sourceTopic,
          section: segment.section,
          topic: segment.section || sourceTopic,
          paragraphIndex: segment.paragraphIndex
        };

        const units = v440ExtractMeaningUnitsFromText(segment.text, context);

        for (const unit of units) {
          const stored = v440StoreMeaningUnit(unit, {
            sourceId: source.id,
            sourceName: source.name,
            sourceType: "document",
            text: segment.text,
            section: segment.section,
            paragraphIndex: segment.paragraphIndex
          });

          if (stored) {
            counts[stored.type] = (counts[stored.type] || 0) + 1;
            storedIds.push(stored.id);
          }
        }
      }

      if (!core.migratedSourceIds.includes(source.id)) core.migratedSourceIds.push(source.id);
      core.migratedSourceIds = core.migratedSourceIds.slice(-80);
      core.lastLearningReport = {
        sourceId: source.id,
        sourceName: source.name,
        meaningUnits: storedIds.length,
        counts,
        migrated: Boolean(options.migration),
        createdAt: Date.now()
      };

      return core.lastLearningReport;
    }

    function v440MigrateStoredSources() {
      const core = v440EnsureMeaningCore();
      const sources = v43Memory().sources || [];

      for (const source of sources) {
        if (!source?.id || core.migratedSourceIds.includes(source.id)) continue;
        v440LearnDocumentMeanings(source, { migration: true });
      }
    }

    function v440DetectTeachingAct(text) {
      const clean = normalizeWhitespace(text);
      if (/\?$/.test(clean)) return null;

      const procedureTeaching = /\b(?:works? like this|supposed to work like this|meant to work like this|here(?:'s| is) how|the steps are|I would say|you would say|setting up the punchline|full .* example)\b/i.test(clean);
      const explicitTeaching = /^(?:let me teach you|here is how|here's how|remember this|learn this|this is how)\b/i.test(clean);

      if (!procedureTeaching && !explicitTeaching) return null;

      const procedure = v440ExtractDialogueProcedure(clean, { sourceTopic: null });

      return {
        type: procedure ? "procedure" : "teaching",
        topic: procedure?.name || v440ProcedureTopic(clean, {}) || null,
        procedure,
        confidence: procedure ? 0.97 : 0.78
      };
    }

    function v440LearnConversationMeaning(text, options = {}) {
      const teaching = options.teaching || v440DetectTeachingAct(text);
      const topic = teaching?.topic || state.context.currentTopic || null;
      const context = {
        sourceId: null,
        sourceName: "conversation",
        sourceTopic: topic,
        topic,
        section: null,
        paragraphIndex: null
      };

      const units = v440ExtractMeaningUnitsFromText(text, context);
      const stored = [];

      for (const unit of units) {
        const result = v440StoreMeaningUnit(unit, {
          sourceType: "conversation",
          sourceName: "conversation",
          text
        });
        if (result) stored.push(result);
      }

      const core = v440EnsureMeaningCore();
      core.lastLearningReport = {
        sourceId: null,
        sourceName: "conversation",
        meaningUnits: stored.length,
        counts: Object.fromEntries([...new Set(stored.map(unit => unit.type))].map(type => [type, stored.filter(unit => unit.type === type).length])),
        topic,
        createdAt: Date.now()
      };

      return { teaching, units: stored, report: core.lastLearningReport };
    }

    function v440MeaningUnitSearchText(unit) {
      const values = [
        v440UnitTopic(unit), unit.name, unit.concept, unit.subject, unit.relation,
        unit.object, unit.property, unit.value, unit.category, unit.action,
        unit.from, unit.to, unit.time, unit.target, unit.directive,
        ...(unit.entities || []), ...(unit.tags || [])
      ];

      if (unit.type === "procedure") {
        for (const step of unit.sequence || []) {
          values.push(step.content, step.pattern, step.bind, step.kind, step.example);
        }
      }

      return normalizeWhitespace(values.filter(Boolean).join(" "));
    }

    function v440DetectSemanticQuery(text) {
      const clean = stripEndingPunctuation(normalizeNaturalLanguage(text));
      const selfIntent = typeof v41DetectSelfIntent === "function" ? v41DetectSelfIntent(text) : null;
      if (selfIntent) return null;

      if (/^(?:what have you learned|what did you learn|what have i taught you|tell me what you have learned)$/i.test(clean)) {
        return { intent: "inventory", topic: null, raw: text };
      }

      let match = clean.match(/^(?:can|could|would) you (?:please )?explain how (.+?) works?$/i) ||
        clean.match(/^explain how (.+?) works?$/i) ||
        clean.match(/^how does (.+?) work$/i) ||
        clean.match(/^how do (.+?) work$/i) ||
        clean.match(/^how is (.+?) supposed to work$/i);

      if (match) return { intent: "explain_procedure", topic: v440CleanTopic(match[1]), raw: text };

      match = clean.match(/^(?:what have you learned about|what did you learn about|what do you know about|tell me about|can you tell me about|could you tell me about)\s+(.+)$/i);
      if (match) return { intent: "explain_topic", topic: v440CleanTopic(match[1]), raw: text };

      match = clean.match(/^(?:what is|what are)\s+(.+)$/i);
      if (match) return { intent: "define", topic: v440CleanTopic(match[1]), raw: text };

      return null;
    }

    function v440ScoreMeaningUnit(unit, query) {
      const queryTokens = new Set(v440MeaningTokens(query.topic || query.raw || ""));
      const unitTokens = new Set(v440MeaningTokens(v440MeaningUnitSearchText(unit)));
      let score = (unit.confidence || 0.5) * 2;
      const reasons = [];

      for (const token of queryTokens) {
        if (unitTokens.has(token)) score += 2.4;
      }

      const topic = canonical(v440UnitTopic(unit) || "");
      const requested = canonical(query.topic || "");
      if (topic && requested && (topic.includes(requested) || requested.includes(topic))) {
        score += 10;
        reasons.push("topic match");
      }

      const desiredTypes = {
        explain_procedure: ["procedure"],
        define: ["definition","proposition","property"],
        explain_topic: ["definition","event","change","proposition","property","procedure","guidance"]
      }[query.intent] || [];

      if (desiredTypes.includes(unit.type)) {
        score += unit.type === desiredTypes[0] ? 7 : 4;
        reasons.push(`useful ${unit.type}`);
      }

      if (unit.type === "example" && query.intent !== "example") score -= 3;
      if (unit.type === "guidance" && query.intent === "explain_topic") score -= 0.5;

      return { unit, score, reasons };
    }

    function v440RetrieveMeaning(query, options = {}) {
      const units = v440MeaningUnits();
      const scored = units
        .map(unit => v440ScoreMeaningUnit(unit, query))
        .filter(entry => entry.score >= (options.minScore ?? 3))
        .sort((a,b) => b.score - a.score)
        .slice(0, options.limit || 12);

      const core = v440EnsureMeaningCore();
      core.lastRetrieval = {
        query: JSON.parse(JSON.stringify(query)),
        results: scored.map(entry => ({
          unitId: entry.unit.id,
          type: entry.unit.type,
          topic: v440UnitTopic(entry.unit),
          score: Number(entry.score.toFixed(2)),
          reasons: entry.reasons
        })),
        createdAt: Date.now()
      };

      v43EnsureCognition().lastRetrieval = {
        query: query.raw || query.topic || "semantic query",
        results: scored.map(entry => ({
          role: `meaning:${entry.unit.type}`,
          type: entry.unit.type,
          score: entry.score,
          sourceName: entry.unit.evidence?.[0]?.sourceName || null,
          topic: v440UnitTopic(entry.unit)
        }))
      };

      return scored;
    }

    function v440FindProcedure(topic) {
      if (!topic) return null;
      const results = v440RetrieveMeaning({ intent: "explain_procedure", topic, raw: topic }, { limit: 6, minScore: 4 });
      return results.find(entry => entry.unit.type === "procedure")?.unit || null;
    }

    function v440ProcedureStepEnglish(step, procedure) {
      const actor = step.actor === "user" ? "you" : "I";

      if (step.kind === "utterance") {
        const spoken = /[.!?]$/.test(step.content || "") ? step.content : `${step.content}.`;
        return `${actor === "I" ? "I start" : "You start"} by saying “${spoken}”`;
      }

      if (step.kind === "expected_response") {
        const spoken = /[.!?]$/.test(step.content || "") ? step.content : `${step.content}.`;
        return `Then I wait for you to say “${spoken}”`;
      }

      if (step.kind === "provide_value") {
        const example = step.example ? `, for example “${step.example}”` : "";
        return `Next I provide a ${step.bind || "setup"} value${example}.`;
      }

      if (step.kind === "expected_pattern") {
        const pattern = step.pattern || step.example || "the expected response";
        const spoken = /[.!?]$/.test(pattern) ? pattern : `${pattern}.`;
        return `You respond using the pattern “${spoken}”`;
      }

      if (step.kind === "deliver_payoff") {
        return `Then I deliver the payoff${step.relationTo ? ` connected to the ${step.relationTo}` : ""}.`;
      }

      return `${actor === "I" ? "I" : "You"} ${String(step.kind || "continue").replace(/_/g," ")}.`;
    }

    function v440ExplainProcedure(procedure) {
      if (!procedure) return null;
      const sentences = (procedure.sequence || []).map(step => v440ProcedureStepEnglish(step, procedure));
      const interaction = procedure.interactionMode === "multi_turn"
        ? `${v41Capitalize(procedure.name)} is a multi-turn procedure.`
        : `${v41Capitalize(procedure.name)} is a procedure.`;

      return `${interaction} ${sentences.join(" ")}`;
    }

    function v440UnitSentence(unit) {
      if (unit.type === "proposition") {
        try {
          const realized = v41RealizeFactProposition(v41FactProposition({ subject: unit.subject, relation: unit.relation, object: unit.object }));
          if (realized) return `${v41Capitalize(stripEndingPunctuation(realized))}.`;
        } catch (_) {}
        return `${v41Capitalize(unit.subject)} ${String(unit.relation).replace(/_/g," ")} ${unit.object}.`;
      }
      if (unit.type === "event") {
        const time = unit.time ? ` in ${unit.time}` : "";
        return `${v41Capitalize(unit.subject)} ${String(unit.action || "changed").replace(/_/g," ")}${unit.object ? ` ${unit.object}` : ""}${time}.`;
      }
      if (unit.type === "change") return `${v41Capitalize(unit.subject)} changed from ${unit.from} to ${unit.to}${unit.time ? ` around ${unit.time}` : ""}.`;
      if (unit.type === "definition") return `${v41Capitalize(unit.concept)} is a ${[...(unit.properties || []), unit.category].filter(Boolean).join(" ")}.`;
      if (unit.type === "property") return `${v41Capitalize(unit.subject)} has the property ${unit.property}: ${unit.value || unit.meaning}.`;
      return null;
    }

    function v440AnswerSemanticQuery(query) {
      if (!query) return null;

      if (query.intent === "inventory") {
        const units = v440MeaningUnits();
        const topics = [...new Set(units.map(v440UnitTopic).filter(Boolean))].slice(0, 12);
        if (!topics.length) return null;
        return `I have ${units.length} persistent learned memory unit${units.length === 1 ? "" : "s"}, represented as semantic meaning units. The learned topics I can identify include ${v41OxfordList(topics)}.`;
      }

      const results = v440RetrieveMeaning(query, { limit: 14, minScore: 3 });
      if (!results.length) return null;

      if (query.intent === "explain_procedure") {
        const procedure = results.find(entry => entry.unit.type === "procedure")?.unit;
        return procedure ? v440ExplainProcedure(procedure) : null;
      }

      if (query.intent === "define") {
        const definition = results.find(entry => entry.unit.type === "definition")?.unit;
        if (definition) return v440UnitSentence(definition);
      }

      const unique = [];
      for (const entry of results) {
        if (entry.unit.type === "guidance" || entry.unit.type === "example") continue;
        const sentence = entry.unit.type === "procedure" ? v440ExplainProcedure(entry.unit) : v440UnitSentence(entry.unit);
        if (sentence && !unique.some(value => canonical(value) === canonical(sentence))) unique.push(sentence);
        if (unique.length >= 5) break;
      }

      if (!unique.length) return null;
      return `I learned about ${query.topic}. From the meaning I have stored, ${unique.join(" ")}`;
    }

    function v440MeaningConceptsForTopic(topic, units, limit = 8) {
      const topicTokens = new Set(v440MeaningTokens(topic));
      const candidates = [];

      for (const unit of units) {
        for (const entity of unit.entities || []) {
          const clean = normalizeWhitespace(entity);
          if (!clean || /^\d{4}$/.test(clean)) continue;
          const tokens = v440MeaningTokens(clean);
          if (!tokens.length || tokens.every(token => topicTokens.has(token))) continue;
          if (/^(?:user|assistant|conversation)$/i.test(clean)) continue;
          candidates.push(clean);
        }
      }

      return [...new Set(candidates.map(value => value.trim()))]
        .filter(value => !/^(?:include|system|modern|example|something)$/i.test(value))
        .slice(0, limit);
    }

    function v440TopicGroups() {
      const units = v440MeaningUnits();
      const groups = new Map();

      for (const unit of units) {
        const topic = v440UnitTopic(unit);
        if (!topic) continue;
        const key = canonical(topic);
        if (!groups.has(key)) groups.set(key, { topic, units: [], learnedAt: 0, sourceIds: new Set() });
        const group = groups.get(key);
        group.units.push(unit);
        group.learnedAt = Math.max(group.learnedAt, unit.learnedAt || 0, unit.updatedAt || 0);
        for (const evidence of unit.evidence || []) if (evidence.sourceId) group.sourceIds.add(evidence.sourceId);
      }

      return [...groups.values()].map(group => ({ ...group, sourceIds: [...group.sourceIds] })).sort((a,b) => b.learnedAt - a.learnedAt);
    }

    // V4.4 makes learned-topic access semantic-first. These names are kept
    // for compatibility with the V4.3 humor/retrieval callers.
    nova4312LearnedTopics = function() {
      const semantic = v440TopicGroups().map(group => ({
        topic: group.topic,
        sourceId: group.sourceIds[0] || null,
        source: null,
        items: group.units.map(unit => ({ bucket: "meaningUnits", item: { ...unit, text: v440MeaningUnitSearchText(unit) } })),
        meaningUnits: group.units,
        learnedAt: group.learnedAt
      }));

      if (semantic.length) return semantic;
      return [];
    };

    nova4312FindLearnedTopic = function(query) {
      return nova4312LearnedTopics()
        .map(item => ({
          ...item,
          score: nova4312Overlap(query, item.topic) * 4 + (
            canonical(query).includes(canonical(item.topic)) || canonical(item.topic).includes(canonical(query)) ? 6 : 0
          )
        }))
        .filter(item => item.score > 0)
        .sort((a,b) => b.score - a.score || b.learnedAt - a.learnedAt)[0] || null;
    };

    nova4312TopicBundle = function(query) {
      const match = nova4312FindLearnedTopic(query);
      if (!match) return null;
      const units = match.meaningUnits || [];
      const facts = units.filter(unit => unit.type === "proposition").map(unit => ({
        subject: unit.subject, relation: unit.relation, object: unit.object,
        sourceId: unit.evidence?.[0]?.sourceId || null,
        source: unit.evidence?.[0]?.sourceName || "semantic meaning unit"
      }));
      const years = [
        ...new Set(
          units.flatMap(unit =>
            [
              unit.time,
              ...(unit.entities || []).filter(value => /^\d{4}$/.test(String(value)))
            ].filter(value => /^\d{4}$/.test(String(value)))
          )
        )
      ].sort((a, b) => Number(a) - Number(b));
      const concepts = v440MeaningConceptsForTopic(match.topic, units, 8);

      return {
        topic: match.topic,
        items: match.items || [],
        facts,
        years,
        concepts,
        meaningUnits: units,
        learnedAt: match.learnedAt
      };
    };

    function v440MeaningUnitSummary(unit) {
      if (!unit) return "unknown meaning";
      if (unit.type === "procedure") return `${unit.name}: ${unit.interactionMode}, ${(unit.sequence || []).length} ordered step${(unit.sequence || []).length === 1 ? "" : "s"}`;
      if (unit.type === "proposition") return `${unit.subject} ${String(unit.relation).replace(/_/g," ")} ${unit.object}`;
      if (unit.type === "event") return `${unit.subject} ${String(unit.action || "event").replace(/_/g," ")}${unit.time ? ` (${unit.time})` : ""}`;
      if (unit.type === "change") return `${unit.subject}: ${unit.from} → ${unit.to}`;
      if (unit.type === "definition") return `${unit.concept} is a ${unit.category}`;
      if (unit.type === "guidance") return `${unit.target || "task"}: ${unit.strength} — ${unit.directive}`;
      if (unit.type === "example") return `example of ${unit.demonstrates || unit.topic || "learned concept"}`;
      return v440MeaningUnitSearchText(unit);
    }


    /* =========================================================
       V4.5.0 SECTION: UNIFIED SEMANTIC KNOWLEDGE STORE

       Breaking change by design:
       - legacy semantic/procedural/abstracted/meaningUnits are not the
         active learned-knowledge representation anymore;
       - examples are evidence, never top-level topics;
       - section labels are context, never concepts;
       - repeated teaching merges into shared concepts with provenance;
       - retrieval resolves a concept/task first, then retrieves relations.
       ========================================================= */

    const NOVA450_SCHEMA_VERSION = 1;

    const NOVA450_META_LABELS = new Set([
      "purpose","important","structure","pattern","example","examples",
      "example pattern","example structure","example mechanism","why it works",
      "when asked","internal process","useful material","useful targets",
      "possible participants","possible styles","positive signals","negative signals",
      "before sending","good","bad","weak","stronger","format","topic","style",
      "begin","later","earlier","current situation","callback","outcome","goal",
      "problem","solution","new problem","expectation","reality","good news","bad news"
    ]);

    const NOVA450_GENERIC_SUBJECTS = new Set([
      "this","that","it","they","these","those","something","anything","everything",
      "example","example pattern","example structure","structure","pattern","format",
      "topic","style","answer","punchline","setup","later","earlier","begin","outcome"
    ]);

    function v450EnsureKnowledgeStore() {
      const memory = v43Memory();

      if (!memory.knowledgeStore || memory.knowledgeStore.version !== NOVA450_SCHEMA_VERSION) {
        memory.knowledgeStore = {
          version: NOVA450_SCHEMA_VERSION,
          concepts: [],
          claims: [],
          procedures: [],
          guidance: [],
          examples: [],
          learnedSourceIds: [],
          lastLearningReport: null,
          lastRetrieval: null
        };

        // These containers remain empty only so older runtime helpers that
        // expect arrays do not crash. They are no longer learned knowledge.
        memory.semantic = [];
        memory.linguistic = [];
        memory.style = [];
        memory.procedural = [];
        memory.abstracted = [];
        delete memory.meaningUnits;
        delete memory.meaningCore;
      }

      return memory.knowledgeStore;
    }

    function v450KnowledgeStore() {
      return v450EnsureKnowledgeStore();
    }

    function v450CanonicalWords(text) {
      return canonical(text || "")
        .replace(/[^a-z0-9']+/g, " ")
        .split(/\s+/)
        .map(word => {
          if (word.length > 5 && /ies$/.test(word)) return word.slice(0,-3) + "y";
          if (word.length > 4 && /s$/.test(word) && !/(?:ss|us|is)$/.test(word)) return word.slice(0,-1);
          return word;
        })
        .filter(word => word.length >= 2 && !NOVA440_MEANING_STOP.has(word));
    }

    function v450CleanHeading(value) {
      let text = normalizeWhitespace(String(value || ""))
        .replace(/^[“"](.+?)[”"]\s+(JOKE|STRUCTURE|VARIATION)$/i, "$1 $2")
        .replace(/^[“”"']+|[“”"']+$/g, "")
        .replace(/^#{1,6}\s*/, "")
        .replace(/^SECTION\s+\d+\s*:\s*/i, "")
        .replace(/^\d+\s*[.)]\s*/, "")
        .replace(/^NOVA\s+(?:LESSON|KNOWLEDGE DOCUMENT)\s*\d*\s*:\s*/i, "")
        .replace(/[=:]{2,}/g, " ")
        .replace(/[_]+/g, " ")
        .replace(/\s+/g, " ")
        .replace(/:$/, "")
        .trim();

      if (!text) return null;
      return text;
    }

    function v450ConceptKey(value) {
      const cleaned = v450CleanHeading(value);
      if (!cleaned) return null;
      const words = v450CanonicalWords(cleaned);
      return words.join(" ") || canonical(cleaned);
    }

    function v450IsMetaConcept(value) {
      const cleaned = canonical(v450CleanHeading(value) || "");
      if (!cleaned) return true;
      if (NOVA450_META_LABELS.has(cleaned)) return true;
      if (/^(?:important distinctions?|timeline summary|summary|final summary|conclusion|final principle|final checklist|quick reference|how to use (?:this|the) (?:knowledge|lesson|document)|how nova should use (?:this|the) (?:knowledge|lesson|document))$/i.test(cleaned)) return true;
      if (/^(?:turn|step|item)\s*\d+$/i.test(cleaned)) return true;
      if (/^\[.+\]$/.test(cleaned)) return true;
      if (/^(?:if|when|do not|don't|should|must|can|may)\b/.test(cleaned)) return true;
      return false;
    }

    function v450Evidence(meta = {}) {
      return {
        sourceId: meta.sourceId || null,
        sourceName: meta.sourceName || null,
        sourceRole: meta.sourceRole || null,
        section: meta.section || null,
        text: normalizeWhitespace(meta.text || ""),
        role: meta.role || "support",
        learnedAt: meta.learnedAt || Date.now()
      };
    }

    function v450MergeEvidence(list, evidence) {
      if (!evidence?.text && !evidence?.sourceId) return list || [];
      const result = list || [];
      const key = canonical(`${evidence.sourceId}|${evidence.section}|${evidence.role}|${evidence.text}`);
      if (!result.some(item => canonical(`${item.sourceId}|${item.section}|${item.role}|${item.text}`) === key)) {
        result.push(evidence);
      }
      return result.slice(-24);
    }

    function v450SourceRole(source) {
      // File names often use underscores/hyphens. Normalize them before
      // classifying the document so `Nova_Joke_..._Lesson.txt` cannot be
      // mistaken for a world-knowledge document.
      const name = canonical(String(source?.name || "").replace(/[_-]+/g, " "));
      const text = canonical(String(source?.text || "").slice(0,1800).replace(/[_-]+/g, " "));
      if (/\bknowledge document\b/.test(text) || /\bhistory and fundamentals\b/.test(name)) return "knowledge";
      if (/\blesson\b/.test(name) || /\bnova lesson\b/.test(text) || /\bteach nova\b/.test(text) || /\bthis lesson teaches\b/.test(text)) return "instruction";
      return "knowledge";
    }

    function v450SourceDomain(source) {
      let name = v450CleanHeading(source?.name || "knowledge") || "knowledge";
      name = name
        .replace(/\.[A-Za-z0-9]{1,8}$/i, "")
        .replace(/^Nova\s+/i, "")
        .replace(/\b(?:lesson|knowledge document)\b.*$/i, "")
        .replace(/\bHistory and Fundamentals\b/i, "")
        .replace(/\bJokes? Callbacks? SelfAware Comedy\b/i, "")
        .replace(/\bStructures? and Formats?\b/i, "structures")
        .replace(/\s+/g, " ")
        .trim();
      return name || "knowledge";
    }

    function v450ConceptNameLooksSemantic(value, strict = false) {
      const clean = v450CleanHeading(value);
      if (!clean) return false;
      if (!strict) return true;
      const words = clean.split(/\s+/).filter(Boolean);
      if (clean.length > 88 || words.length > 12) return false;
      if ((clean.match(/,/g) || []).length > 2) return false;
      if (/[;:](?:\s|$)/.test(clean)) return false;
      if (/^(?:avoid|identify|determine|check|make sure|because|then|instead|however|if|when|while|and|but|so)\b/i.test(clean)) return false;
      if (/\b(?:should|shouldn't|must|mustn't|do not|don't|can also|may also)\b/i.test(clean)) return false;
      return true;
    }

    function v450FindConceptByKey(key) {
      if (!key) return null;
      return v450KnowledgeStore().concepts.find(concept => concept.key === key) || null;
    }

    function v450GetOrCreateConcept(name, options = {}) {
      const clean = v450CleanHeading(name);
      if (!clean || v450IsMetaConcept(clean) || !v450ConceptNameLooksSemantic(clean, Boolean(options.fromClaim))) return null;
      const key = v450ConceptKey(clean);
      if (!key) return null;

      const store = v450KnowledgeStore();
      let concept = store.concepts.find(item => item.key === key);
      // Explicitly taught concepts are allowed to use words that older V4/V5
      // heuristics considered too generic (for example, "example"). The
      // generic list may prevent accidental creation, but it must never erase
      // or make an explicitly taught concept unreachable.
      if (!concept && NOVA450_GENERIC_SUBJECTS.has(key) && !options.explicitConcept) return null;
      const evidence = options.evidence ? v450Evidence(options.evidence) : null;

      if (!concept) {
        concept = {
          id: v43NewId("concept"),
          key,
          name: clean,
          aliases: [],
          kinds: [],
          domainKeys: [],
          subjectEligible: Boolean(options.subjectEligible),
          instructional: Boolean(options.instructional),
          evidence: [],
          createdAt: Date.now(),
          updatedAt: Date.now()
        };
        store.concepts.push(concept);
      }

      if (clean !== concept.name && !concept.aliases.some(alias => canonical(alias) === canonical(clean))) {
        concept.aliases.push(clean);
      }
      for (const alias of options.aliases || []) {
        const a = v450CleanHeading(alias);
        if (a && canonical(a) !== canonical(concept.name) && !concept.aliases.some(old => canonical(old) === canonical(a))) concept.aliases.push(a);
      }
      if (options.kind && !concept.kinds.includes(options.kind)) concept.kinds.push(options.kind);
      if (options.domainKey && !concept.domainKeys.includes(options.domainKey)) concept.domainKeys.push(options.domainKey);
      if (options.subjectEligible) concept.subjectEligible = true;
      if (options.instructional) concept.instructional = true;
      if (evidence) concept.evidence = v450MergeEvidence(concept.evidence, evidence);
      concept.updatedAt = Date.now();
      return concept;
    }

    function v450StoreClaim(claim, evidenceMeta = {}) {
      if (!claim?.subject || !claim?.predicate || claim.object == null) return null;
      const subject = v450GetOrCreateConcept(claim.subject, { ...(claim.subjectOptions || {}), fromClaim: true });
      if (!subject) return null;

      const objectText = normalizeWhitespace(String(claim.object));
      if (!objectText) return null;
      const predicate = canonical(claim.predicate).replace(/\s+/g,"_");
      const qualifiers = claim.qualifiers || {};
      const signature = canonical(`${subject.key}|${predicate}|${objectText}|${JSON.stringify(qualifiers)}|${claim.polarity || "positive"}`);
      const store = v450KnowledgeStore();
      let existing = store.claims.find(item => item.signature === signature);
      const evidence = v450Evidence(evidenceMeta);

      if (!existing) {
        existing = {
          id: v43NewId("claim"),
          signature,
          subjectId: subject.id,
          subjectKey: subject.key,
          subject: subject.name,
          predicate,
          object: objectText,
          objectConceptId: null,
          qualifiers: JSON.parse(JSON.stringify(qualifiers)),
          polarity: claim.polarity || "positive",
          confidence: claim.confidence ?? 0.82,
          evidence: [],
          createdAt: Date.now(),
          updatedAt: Date.now()
        };
        store.claims.push(existing);
      }

      if (claim.objectConcept) {
        const objectConcept = v450GetOrCreateConcept(claim.objectConcept, { ...(claim.objectOptions || {}), fromClaim: true });
        if (objectConcept) existing.objectConceptId = objectConcept.id;
      }
      existing.confidence = Math.max(existing.confidence || 0, claim.confidence || 0);
      existing.evidence = v450MergeEvidence(existing.evidence, evidence);
      existing.updatedAt = Date.now();
      return existing;
    }

    function v450StoreGuidance(rule, evidenceMeta = {}) {
      if (!rule?.directive) return null;
      const store = v450KnowledgeStore();
      const target = v450CleanHeading(rule.target || rule.domain || "general") || "general";
      const polarity = rule.polarity || "positive";
      const broadHumorTarget = /^(?:humor|joke structures|what makes something funny|contextual humor|timing and brevity|when a joke fails|humor feedback|do not force humor|humor with serious answers|safe humor targets|generating a joke from context|joke quality check|do not copy this document|originality test|final principle)$/i.test(target);
      const scope = rule.scope || ((rule.domain === "humor" && broadHumorTarget) ? "general_humor" : "local");
      const signature = canonical(`${rule.domain || "general"}|${scope}|${target}|${rule.strength || "recommended"}|${polarity}|${rule.directive}`);
      let existing = store.guidance.find(item => item.signature === signature);
      const evidence = v450Evidence({ ...evidenceMeta, role: "guidance" });
      if (!existing) {
        existing = {
          id: v43NewId("guidance"),
          signature,
          domain: rule.domain || "general",
          scope,
          target,
          strength: rule.strength || "recommended",
          polarity,
          directive: normalizeWhitespace(rule.directive),
          conditions: rule.conditions || [],
          evidence: [],
          createdAt: Date.now(),
          updatedAt: Date.now()
        };
        store.guidance.push(existing);
      }
      existing.evidence = v450MergeEvidence(existing.evidence, evidence);
      existing.updatedAt = Date.now();
      return existing;
    }

    function v450StoreExample(example, evidenceMeta = {}) {
      if (!example) return null;
      const store = v450KnowledgeStore();
      const demonstratesKey = v450ConceptKey(example.demonstrates || example.topic || "") || null;
      const content = normalizeWhitespace(example.content || "");
      const signature = canonical(`${demonstratesKey}|${content}|${JSON.stringify(example.sequence || [])}`);
      let existing = store.examples.find(item => item.signature === signature);
      const evidence = v450Evidence({ ...evidenceMeta, role: "example" });
      if (!existing) {
        existing = {
          id: v43NewId("example"), signature, demonstratesKey,
          content, sequence: JSON.parse(JSON.stringify(example.sequence || [])),
          evidence: [], createdAt: Date.now(), updatedAt: Date.now()
        };
        store.examples.push(existing);
      }
      existing.evidence = v450MergeEvidence(existing.evidence, evidence);
      existing.updatedAt = Date.now();
      return existing;
    }

    function v450StoreProcedure(procedure, evidenceMeta = {}) {
      if (!procedure?.name) return null;
      const name = v450CleanHeading(procedure.name);
      const key = v450ConceptKey(name);
      if (!key) return null;
      const concept = v450GetOrCreateConcept(name, {
        kind: "procedure",
        subjectEligible: Boolean(procedure.subjectEligible),
        instructional: true,
        aliases: procedure.aliases || [],
        evidence: evidenceMeta
      });
      if (!concept) return null;

      const store = v450KnowledgeStore();
      let existing = store.procedures.find(item => item.conceptKey === key);
      const evidence = v450Evidence(evidenceMeta);
      if (!existing) {
        existing = {
          id: v43NewId("procedure"),
          conceptId: concept.id,
          conceptKey: key,
          name: concept.name,
          interactionMode: procedure.interactionMode || "single_turn",
          roles: [], sequence: [], requirements: [], mechanisms: [], optionalMechanisms: [],
          purpose: procedure.purpose || null,
          confidence: procedure.confidence ?? 0.86,
          evidence: [], createdAt: Date.now(), updatedAt: Date.now(), revision: 1
        };
        store.procedures.push(existing);
      }

      const incomingSequence = procedure.sequence || [];
      if (incomingSequence.length > existing.sequence.length || (procedure.confidence || 0) > (existing.confidence || 0)) {
        existing.sequence = JSON.parse(JSON.stringify(incomingSequence));
        existing.interactionMode = procedure.interactionMode || existing.interactionMode;
      }
      existing.roles = [...new Set([...(existing.roles || []), ...(procedure.roles || [])])];
      existing.requirements = [...new Set([...(existing.requirements || []), ...(procedure.requirements || [])])];
      existing.mechanisms = [...new Set([...(existing.mechanisms || []), ...(procedure.mechanisms || [])])];
      existing.optionalMechanisms = [...new Set([...(existing.optionalMechanisms || []), ...(procedure.optionalMechanisms || [])])];
      existing.confidence = Math.max(existing.confidence || 0, procedure.confidence || 0);
      existing.purpose = procedure.purpose || existing.purpose;
      existing.evidence = v450MergeEvidence(existing.evidence, evidence);
      existing.updatedAt = Date.now();
      existing.revision = (existing.revision || 0) + 1;
      return existing;
    }

    function v450IsDivider(line) {
      return /^[=_-]{3,}$/.test(String(line || "").trim());
    }

    function v450HeadingFromLine(line) {
      const raw = String(line || "").trim();
      if (!raw || v450IsDivider(raw)) return null;
      let m = raw.match(/^SECTION\s+\d+\s*:\s*(.+)$/i);
      if (m) return v450CleanHeading(m[1]);
      m = raw.match(/^\d+\s*[.)]\s+(.+)$/);
      if (m && !/^(?:Turn|Step|Item)\b/i.test(raw)) {
        const candidate=normalizeWhitespace(m[1]);
        const words=candidate.split(/\s+/).filter(Boolean);
        const upper=candidate===candidate.toUpperCase();
        const titleLike=words.length<=7 && words.every((word,index)=>
          /^(?:and|or|of|the|a|an|to|with|vs\.?|from|in)$/i.test(word) || /^[A-Z0-9“"'([]/.test(word)
        );
        const imperative=/^(?:identify|determine|choose|check|find|make|remove|use|add|begin|start|recall|notice|describe|keep|learn|avoid|prefer)\b/i.test(candidate);
        if ((upper || titleLike) && !imperative) return v450CleanHeading(candidate);
      }
      m = raw.match(/^#{1,6}\s+(.+)$/);
      if (m) return v450CleanHeading(m[1]);
      if (raw.length <= 100 && /^[A-Z0-9][A-Z0-9 '&()\/\-–—>]+:?$/.test(raw) && !/[.!?]$/.test(raw)) {
        return v450CleanHeading(raw);
      }
      return null;
    }

    function v450DocumentSections(text, source) {
      const lines = String(text || "").replace(/\r\n?/g,"\n").split("\n");
      const sections = [];
      let current = { title: v450SourceDomain(source), lines: [] };

      function push() {
        const body = current.lines.join("\n").trim();
        if (body || current.title) sections.push({ title: current.title, text: body });
      }

      for (const raw of lines) {
        const line = raw.trim();
        if (!line || v450IsDivider(line)) {
          if (line === "" && current.lines.length) current.lines.push("");
          continue;
        }
        const heading = v450HeadingFromLine(line);
        if (heading && canonical(heading) !== canonical(v450SourceDomain(source))) {
          const blockLabel = /^(?:important|structure|pattern|example|examples|example pattern|example structure|example mechanism|full joke example|why it works|why this works)$/i.test(heading);
          if (blockLabel && !/^SECTION\s+\d+\s*:/i.test(line)) {
            // These are labels *inside* the current semantic section. Keep
            // them in the section so the block parser can classify the
            // following lines without losing the parent topic.
            current.lines.push(line);
            continue;
          }

          // True document-level headings, including non-semantic headings
          // such as SECTION 112: IMPORTANT DISTINCTIONS, still terminate the
          // previous section so their claims cannot contaminate it.
          if (current.lines.some(value => value.trim())) push();
          current = {
            title: heading,
            lines: [],
            meta: v450IsMetaConcept(heading)
          };
          continue;
        }
        current.lines.push(line);
      }
      push();
      return sections.filter((section,index) => section.text || index > 0);
    }

    function v450SentenceList(text) {
      return String(text || "")
        .replace(/\n+/g," ")
        .split(/(?<=[.!?])\s+(?=[A-Z0-9"“])/)
        .map(value => normalizeWhitespace(value))
        .filter(Boolean);
    }

    function v450SectionBlocks(sectionText) {
      const blocks = { body: [], structure: [], pattern: [], example: [], explanation: [], important: [] };
      let mode = "body";
      const lines = String(sectionText || "").replace(/\r\n?/g,"\n").split("\n");
      for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || v450IsDivider(line)) continue;
        const label = canonical(line.replace(/:$/, ""));
        if (/^(?:structure)$/.test(label)) { mode = "structure"; continue; }
        if (/^(?:pattern)$/.test(label)) { mode = "pattern"; continue; }
        if (/^(?:example|example pattern|example structure|example mechanism|full joke example)$/.test(label)) { mode = "example"; continue; }
        if (/^(?:why it works|why this works)$/.test(label)) { mode = "explanation"; continue; }
        if (/^(?:important)$/.test(label)) { mode = "important"; continue; }
        blocks[mode].push(line.replace(/^[-*•]\s*/,""));
      }
      return blocks;
    }

    function v450InstructionDomain(source, sectionTitle) {
      const all = canonical(`${source?.name || ""} ${sectionTitle || ""}`.replace(/[_-]+/g, " "));
      if (/\b(?:joke|humor|comedy|punchline|callback)\b/.test(all)) return "humor";
      if (/\b(?:grammar|conversation|language|speaking|english)\b/.test(all)) return "language";
      if (/\bdebate\b/.test(all)) return "debate";
      return "general";
    }

    function v450MechanismFromTitle(title) {
      const t = canonical(title || "");
      const map = [
        [/knock[- ]?knock/, "knock_knock"],
        [/rule of three/, "rule_of_three"],
        [/one[- ]?liner/, "single_sentence"],
        [/question.*(?:punchline|answer)|why did|what do you call|what happens when/, "question_answer"],
        [/callback/, "callback"],
        [/reversal/, "reversal"],
        [/misdirection|false expectation/, "misdirection"],
        [/anti[- ]?joke/, "anticlimax"],
        [/wordplay|pun/, "wordplay"],
        [/exaggeration|hyperbole/, "exaggeration"],
        [/self[- ]?aware|meta[- ]?humor/, "self_reference"],
        [/comparison|analogy/, "comparison"],
        [/fake dialogue|internal subsystem dialogue/, "dialogue"],
        [/short story|mini[- ]?narrative/, "narrative"]
      ];
      return map.find(([rx]) => rx.test(t))?.[1] || null;
    }

    function v450ExtractKnockKnockProcedure(section, sourceMeta) {
      const title = v450CleanHeading(section.title || "");
      const text = section.text || "";
      const cleanTitle = canonical(title || "");
      if (!/knock[- ]?knock joke/.test(cleanTitle)) return null;
      if (!/multi[- ]?turn|turn\s*1|who(?:'s| is) there/i.test(text)) return null;

      const quoted = v440QuotedSpans(text);
      const opening = quoted.find(item => /^knock[ ,.-]*knock\.?$/i.test(item.text))?.text || "Knock knock.";
      const who = quoted.find(item => /who(?:'s| is) there\??/i.test(item.text))?.text || "Who's there?";
      const setupExample = quoted.find(item => !/^knock[ ,.-]*knock\.?$/i.test(item.text) && !/who(?:'s| is) there/i.test(item.text) && !/\bwho\??$/i.test(item.text))?.text || null;

      const sequence = [
        { order:1, actor:"assistant", kind:"utterance", content: opening },
        { order:2, actor:"user", kind:"expected_response", content: who },
        { order:3, actor:"assistant", kind:"provide_value", bind:"setup", valueType:"name_word_or_phrase", example: setupExample },
        { order:4, actor:"user", kind:"expected_pattern", pattern:"[setup] who?" },
        { order:5, actor:"assistant", kind:"deliver_payoff", relationTo:"setup" }
      ];

      return {
        name: title || "Knock-knock joke",
        aliases: ["knock knock joke", "knock-knock joke"],
        interactionMode: "multi_turn",
        roles: ["assistant","user"],
        sequence,
        requirements: ["interactive_turns","ordered_steps","wait_for_response","preserve_setup_binding","payoff_after_setup"],
        mechanisms: ["knock_knock"],
        optionalMechanisms: [],
        purpose: "perform the learned joke format",
        confidence: 0.99
      };
    }

    function v450ExtractInstructionProcedure(section, source, evidenceBase) {
      const title = v450CleanHeading(section.title || "");
      if (!title || v450IsMetaConcept(title)) return null;
      const knock = v450ExtractKnockKnockProcedure(section, evidenceBase);
      if (knock) return knock;

      const blocks = v450SectionBlocks(section.text);
      const mechanism = v450MechanismFromTitle(title);
      const saysStructure = /\b(?:structure|pattern|format|procedure|process|steps?)\b/i.test(section.text);
      if (!mechanism && !saysStructure) return null;

      let rawSteps = [...blocks.structure, ...blocks.pattern];
      if (!rawSteps.length) {
        rawSteps = blocks.body.filter(line => /^(?:Item|Step|Turn)\s*\d+|^[A-Z][A-Za-z ]+\s*->|^\[.+\]/.test(line));
      }
      const sequence = rawSteps.slice(0,12).map((line,index) => ({
        order:index+1, actor:"assistant", kind:"structure_step", content: normalizeWhitespace(line)
      }));

      const requirements = [];
      if (mechanism) requirements.push(mechanism);
      if (/requires participation|depends on turn-taking|multi[- ]?turn/i.test(section.text)) requirements.push("interactive_turns");
      if (/one sentence|single sentence|one-liner/i.test(section.text) && /should|contains|fit/i.test(section.text)) requirements.push("single_sentence");

      const optionalMechanisms = [];
      for (const trait of ["callback","reversal","rule_of_three","anticlimax","wordplay","exaggeration","misdirection","self_reference"]) {
        const label = trait.replace(/_/g,"[ -]");
        const rx = new RegExp(`\\b(?:can|may|might|could|sometimes|often)\\b[^.]{0,80}\\b${label}\\b`,"i");
        if (rx.test(section.text)) optionalMechanisms.push(trait);
      }

      return {
        name: title,
        aliases: [],
        interactionMode: requirements.includes("interactive_turns") ? "multi_turn" : "single_turn",
        roles: requirements.includes("interactive_turns") ? ["assistant","user"] : ["assistant"],
        sequence,
        requirements: [...new Set(requirements)],
        mechanisms: mechanism ? [mechanism] : [],
        optionalMechanisms: [...new Set(optionalMechanisms)],
        purpose: /joke/i.test(`${title} ${source.name}`) ? "perform a learned humor structure" : "execute a learned procedure",
        confidence: sequence.length || mechanism ? 0.91 : 0.8
      };
    }

    function v450ExtractInstructionGuidance(section, source, evidenceBase) {
      const domain = v450InstructionDomain(source, section.title);
      const sectionTarget = v450CleanHeading(section.title) || v450SourceDomain(source);
      const stored = [];
      let mode = "body";

      const rawLines = String(section.text || "").replace(/\r\n?/g, "\n").split("\n");
      for (const rawLine of rawLines) {
        let line = normalizeWhitespace(rawLine.replace(/^[-*•]\s*/, ""));
        if (!line || v450IsDivider(line)) continue;
        const label = canonical(line.replace(/:$/, ""));
        if (/^(?:structure|pattern)$/.test(label)) { mode = label; continue; }
        if (/^(?:example|examples|example pattern|example structure|example mechanism|full joke example)$/.test(label)) { mode = "example"; continue; }
        if (/^(?:why it works|why this works)$/.test(label)) { mode = "explanation"; continue; }
        if (/^(?:important)$/.test(label)) { mode = "important"; continue; }

        const directiveLike = /^(?:(?:Nova|The answer|The punchline|The comparison|The third element|A callback|A joke|Jokes?|Fake logs?)\s+)?(?:should|must|should not|shouldn't|must not|do not|don't|avoid|prefer|keep|use)\b/i.test(line);
        if (mode === "example" && !directiveLike) continue;
        if (/^(?:Purpose\s*:|This lesson\b|This document\b|NOVA LESSON\b)/i.test(line)) continue;
        if (/^(?:Nova|User|Parser|Reasoner|Memory|Grammar realizer)\s*:\s*[“"]?/i.test(line)) continue;
        if (/^[“"].+[”"]$/.test(line) && !directiveLike) continue;

        for (const sentence of v450SentenceList(line)) {
          const clean = stripEndingPunctuation(sentence);
          if (!clean) continue;
          if (/\bexample\b/i.test(clean) && !/\bexamples? are demonstrations|examples? are demonstration/i.test(clean)) continue;

          let m = clean.match(/^Humor\s+(?:often\s+)?comes?\s+from\s+(.+)$/i);
          if (m) {
            const rule=v450StoreGuidance({
              domain:"humor",
              scope:"general_humor",
              target:"Humor",
              strength:"optional",
              polarity:"positive",
              directive:`may use ${m[1]}`
            },{...evidenceBase,text:sentence});
            if(rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:A\s+)?joke\s+only\s+needs\s+(.+)$/i);
          if (m) {
            const rule=v450StoreGuidance({domain:"humor",scope:"general_humor",target:"Humor",strength:"recommended",polarity:"positive",directive:`use ${m[1]}`},{...evidenceBase,text:sentence});
            if(rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:Nova\s+)?(should\s+not|shouldn't|must\s+not|do\s+not|don't|should|must)\s+(.+)$/i);
          if (m) {
            if (/:$/.test(m[2])) continue;
            const modal = canonical(m[1]);
            const negative = /not|don't/.test(modal);
            const directive = normalizeWhitespace(
              String(m[2] || "")
                .replace(/^NOT\s+/i, "")
                .replace(/^not\s+/i, "")
            );
            const rule = v450StoreGuidance({
              domain,
              target: sectionTarget,
              strength: /must|do not|don't/.test(modal) ? "required" : "recommended",
              polarity: negative ? "negative" : "positive",
              directive
            }, { ...evidenceBase, text: sentence });
            if (rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:avoid)\s+(.+)$/i);
          if (m) {
            if (/:$/.test(m[1])) continue;
            const rule = v450StoreGuidance({ domain, target:sectionTarget, strength:"recommended", polarity:"negative", directive:m[1] }, { ...evidenceBase, text:sentence });
            if (rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:prefer)\s+(.+)$/i);
          if (m) {
            const rule = v450StoreGuidance({ domain, target:sectionTarget, strength:"recommended", polarity:"positive", directive:m[1] }, { ...evidenceBase, text:sentence });
            if (rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:learn)\s+(.+)$/i);
          if (m) {
            const rule=v450StoreGuidance({domain,target:sectionTarget,strength:"recommended",polarity:"positive",directive:`learn ${m[1]}`},{...evidenceBase,text:sentence});
            if(rule) stored.push(rule);
            continue;
          }

          // "A joke does not need all of these" is a constraint, not a
          // positive characteristic that every candidate must satisfy.
          m = clean.match(/^(.+?)\s+does\s+not\s+need\s+(.+)$/i);
          if (m) {
            const rule=v450StoreGuidance({domain,target:sectionTarget,strength:"recommended",polarity:"negative",directive:`require ${m[2]}`},{...evidenceBase,text:sentence});
            if(rule) stored.push(rule);
            continue;
          }

          m = clean.match(/^(?:use|keep)\s+(.+)$/i);
          if (m) {
            const directive=clean;
            if (!/:$/.test(directive)) {
              const rule=v450StoreGuidance({domain,target:sectionTarget,strength:"recommended",polarity:"positive",directive},{...evidenceBase,text:sentence});
              if(rule) stored.push(rule);
            }
            continue;
          }

          m = clean.match(/^(.+?)\s+(should|must|can|may|might|could|sometimes|often)\s+(.+)$/i);
          if (m) {
            if (/:$/.test(m[3])) continue;
            const modal = canonical(m[2]);
            const subject = v450CleanHeading(m[1]);
            if (!subject || v450IsMetaConcept(subject) || NOVA450_GENERIC_SUBJECTS.has(v450ConceptKey(subject)) || !v450ConceptNameLooksSemantic(subject, true)) continue;
            const rule = v450StoreGuidance({
              domain,
              target: sectionTarget,
              strength: /can|may|might|could|sometimes|often/.test(modal) ? "optional" : (/must/.test(modal) ? "required" : "recommended"),
              polarity: "positive",
              directive: `${subject} ${m[3]}`
            }, { ...evidenceBase, text: sentence });
            if (rule) stored.push(rule);
          }
        }
      }
      return stored.filter(Boolean);
    }

    function v450StoreSectionExamples(section, concept, evidenceBase) {
      const blocks = v450SectionBlocks(section.text);
      if (!blocks.example.length) return [];
      const content = blocks.example.join(" ");
      const quotes = v440QuotedSpans(content);
      const example = v450StoreExample({
        demonstrates: concept?.name || section.title,
        content,
        sequence: quotes.length >= 2 ? quotes.map((q,index)=>({actor:index%2===0?"assistant":"user",content:q.text})) : []
      }, { ...evidenceBase, text: content });
      return example ? [example] : [];
    }

    function v450ResolveSubject(rawSubject, sectionConcept) {
      let subject = v450CleanHeading(rawSubject);
      if (!subject) return sectionConcept?.name || null;
      const key = canonical(subject);
      if (/^(?:it|they|this|these|those|most|many|some|common generations|examples|important properties|major responsibilities|areas|possible traits)$/i.test(key)) {
        return sectionConcept?.name || null;
      }
      if (/^(?:most|many|some|common|major|modern|early|later)\s+(.+)$/i.test(subject)) {
        const stripped = subject.replace(/^(?:most|many|some|common|major|modern|early|later)\s+/i,"");
        if (sectionConcept && v450CanonicalWords(stripped).some(word => sectionConcept.key.split(" ").includes(word))) return sectionConcept.name;
      }
      return subject;
    }

    function v450PredicateFromVerb(verb) {
      return canonical(verb).replace(/\s+/g,"_").replace(/ies$/, "y").replace(/s$/, "");
    }

    function v450ParseKnowledgeSentence(sentence, sectionConcept, evidenceBase) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence));
      if (!clean || /^(?:Purpose|Important|Structure|Pattern|Example|Why it works)\s*:/i.test(clean)) return [];
      if (/\b(?:this lesson|this document)\b/i.test(clean)) return [];
      const stored = [];
      const subjectName = sectionConcept?.name || null;
      let m;

      // Explicit distinction: X is not the same as Y.
      m = clean.match(/^(.+?)\s+is\s+not\s+(?:the\s+)?same\s+as\s+(.+)$/i);
      if (m) {
        const subject = v450ResolveSubject(m[1], sectionConcept);
        const claim = v450StoreClaim({subject,predicate:"different_from",object:m[2],objectConcept:m[2],confidence:0.94}, { ...evidenceBase, text: sentence });
        if (claim) stored.push(claim);
        return stored;
      }

      // Language/concept meaning: X can mean Y.
      m = clean.match(/^[“"']?(.+?)[”"']?\s+(?:can\s+)?mean(?:s)?\s+(.+)$/i);
      if (m) {
        const subject = v450ResolveSubject(m[1], sectionConcept);
        const claim=v450StoreClaim({subject,predicate:"means",object:m[2],confidence:0.93},{...evidenceBase,text:sentence});
        if(claim) stored.push(claim);
        return stored;
      }

      // X changed from A to B.
      m = clean.match(/^(.+?)\s+(?:changed|evolved|moved)\s+from\s+(.+?)\s+to\s+(.+)$/i);
      if (m) {
        const subject = v450ResolveSubject(m[1], sectionConcept);
        for (const [predicate,object] of [["changed_from",m[2]],["changed_to",m[3]]]) {
          const claim=v450StoreClaim({subject,predicate,object,confidence:0.93},{...evidenceBase,text:sentence}); if(claim) stored.push(claim);
        }
        return stored;
      }

      // Date-led events.
      m = clean.match(/^(?:In|By)\s+((?:1[5-9]|20)\d{2}),\s+(.+?)\s+(was\s+completed|was\s+introduced|was\s+developed|was\s+created|was\s+built|appeared|arrived|began|started|launched|released|became)\b(?:\s+(.+))?$/i);
      if (m) {
        const subject = v450ResolveSubject(m[2], sectionConcept);
        const claim=v450StoreClaim({subject,predicate:v450PredicateFromVerb(m[3]),object:m[4]||"event",qualifiers:{time:m[1]},confidence:0.94},{...evidenceBase,text:sentence}); if(claim) stored.push(claim);
        return stored;
      }

      // "A RAM is..." / "RAM is...". Keep semantic object, not source sentence.
      m = clean.match(/^(?:a|an|the)?\s*([A-Za-z0-9][A-Za-z0-9 '\-\/]{0,100}?)\s+(?:is|are)\s+(?:a|an)?\s*(.+)$/i);
      if (m) {
        const subject = v450ResolveSubject(m[1], sectionConcept);
        let object = normalizeWhitespace(m[2]);
        if (subject && object && !/^(?:supposed|meant)\s+to\b/i.test(object)) {
          // Capture "volatile, meaning ..." as property + consequence.
          const meaning = object.match(/^([^,;]+),\s*meaning\s+(?:it|they)\s+(.+)$/i);
          if (meaning) {
            const c1=v450StoreClaim({subject,predicate:"has_property",object:meaning[1],confidence:0.93},{...evidenceBase,text:sentence}); if(c1) stored.push(c1);
            const c2=v450StoreClaim({subject,predicate:"means",object:meaning[2],confidence:0.9},{...evidenceBase,text:sentence}); if(c2) stored.push(c2);
            return stored;
          }
          const firstClause = object.split(/,\s*(?:while|but|and)\s+/i)[0];
          const c=v450StoreClaim({subject,predicate:/^(?:machine|device|system|technology|language|interface|memory|computer|program|process|procedure|field|format|architecture)\b/i.test(firstClause)?"is_a":"is",object:firstClause,confidence:0.88},{...evidenceBase,text:sentence}); if(c) stored.push(c);
          // Relative "that/which VERB ..." relation.
          const rel = object.match(/\b(?:that|which)\s+([A-Za-z]+)\s+(.+)$/i);
          if (rel) { const c2=v450StoreClaim({subject,predicate:v450PredicateFromVerb(rel[1]),object:rel[2],confidence:0.84},{...evidenceBase,text:sentence}); if(c2) stored.push(c2); }
          return stored;
        }
      }

      // Generic subject-verb-object relations.
      m = clean.match(/^(.+?)\s+(studies?|describes?|tracks?|controls?|identifies?|maps?|stores?|uses?|provides?|connects?|manages?|executes?|defines?|protects?|contains?|supports?|represents?|converts?|forwards?|allows?|organizes?|combines?|integrates?|translates?|runs?|performs?|includes?|relies\s+on|depends\s+on|refers\s+to)\s+(.+)$/i);
      if (m) {
        const subject = v450ResolveSubject(m[1], sectionConcept);
        const predicate = v450PredicateFromVerb(m[2]);
        const object = m[3];
        const c=v450StoreClaim({subject,predicate,object,confidence:0.88},{...evidenceBase,text:sentence}); if(c) stored.push(c);
        // For include lists, create relation to each item without turning labels into topics.
        if (/include/.test(predicate) && object.includes(",")) {
          for (const item of object.split(/,|\band\b/i).map(x=>normalizeWhitespace(x)).filter(x=>x.length>1).slice(0,16)) {
            const child=v450GetOrCreateConcept(item,{kind:"related",subjectEligible:true,evidence:{...evidenceBase,text:sentence}});
            if (child) { const r=v450StoreClaim({subject,predicate:"includes",object:child.name,objectConcept:child.name,confidence:0.84},{...evidenceBase,text:sentence}); if(r) stored.push(r); }
          }
        }
        return stored;
      }

      // Section-scoped "Common generations include ..." and similar.
      m = clean.match(/^(?:Common|Major|Important|Typical|Examples?|Areas?|Categories?|Generations?)\s+(.+?)\s+(?:include|are)\s+(.+)$/i);
      if (m && subjectName) {
        const list = m[2].split(/,|\band\b/i).map(x=>normalizeWhitespace(x)).filter(Boolean).slice(0,20);
        for (const item of list) {
          const child=v450GetOrCreateConcept(item,{kind:"related",subjectEligible:true,evidence:{...evidenceBase,text:sentence}});
          const c=v450StoreClaim({subject:subjectName,predicate:"includes",object:item,objectConcept:child?.name||null,confidence:0.84},{...evidenceBase,text:sentence}); if(c) stored.push(c);
        }
        return stored;
      }

      // Existing semantic fact extractor is a fallback, but reject generic
      // subjects and force section context for pronouns/labels.
      try {
        const semantic = v4SemanticAnalyze(clean);
        if (semantic.type === "statement" && ["asserted","ability"].includes(semantic.modality?.type)) {
          for (const fact of extractFactsFromText(clean) || []) {
            const subject = v450ResolveSubject(fact.subject, sectionConcept);
            const subjectKey = v450ConceptKey(subject);
            if (!subject || !subjectKey || NOVA450_GENERIC_SUBJECTS.has(subjectKey)) continue;
            const c=v450StoreClaim({subject,predicate:fact.relation,object:fact.object,polarity:semantic.negated?"negative":"positive",confidence:semantic.confidence??0.72},{...evidenceBase,text:sentence}); if(c) stored.push(c);
          }
        }
      } catch (_) {}
      return stored;
    }

    function v450LinkClaimsToSection(claims, sectionConcept, evidenceBase) {
      if (!sectionConcept) return 0;
      let added = 0;
      for (const claim of claims || []) {
        if (!claim?.subjectId || claim.subjectId === sectionConcept.id) continue;
        const relation = v450StoreClaim({
          subject: claim.subject,
          predicate: "topic_context",
          object: sectionConcept.name,
          objectConcept: sectionConcept.name,
          confidence: 0.97
        }, { ...evidenceBase, text: `Topic context: ${claim.subject} is discussed under ${sectionConcept.name}` });
        if (relation) added += 1;
      }
      return added;
    }

    function v450LearnDocument(source, options = {}) {
      if (window.__TAURI__?.core?.invoke) throw new Error("v450LearnDocument is retired on native Nova: Rust Pass 6 owns semantic teaching ingestion.");
      const store = v450KnowledgeStore();
      const sourceRole = v450SourceRole(source);
      const domainName = v450SourceDomain(source);
      const sourceEvidence = { sourceId:source.id, sourceName:source.name, sourceRole, text:"", section:null };
      const domain = v450GetOrCreateConcept(domainName, {
        kind:"domain", subjectEligible:sourceRole === "knowledge", instructional:sourceRole === "instruction",
        aliases:[v450CleanHeading(source.name)], evidence:{...sourceEvidence,text:`Document domain: ${domainName}`}
      });
      const domainKey = domain?.key || null;
      const sections = v450DocumentSections(source.text, source);
      const counts = { concepts:0, claims:0, procedures:0, guidance:0, examples:0 };
      const touched = new Set();

      for (const section of sections) {
        const title = v450CleanHeading(section.title) || domainName;
        const sectionEvidence = { sourceId:source.id, sourceName:source.name, sourceRole, section:title, text:section.text };
        let concept = null;
        if (!v450IsMetaConcept(title)) {
          concept = v450GetOrCreateConcept(title, {
            kind: sourceRole === "instruction" ? "instructional_concept" : "topic",
            subjectEligible: sourceRole === "knowledge",
            instructional: sourceRole === "instruction",
            explicitConcept: section.explicitRole === "concept",
            domainKey,
            evidence: sectionEvidence
          });
          if (concept) {
            touched.add(concept.id);
            if (domain && concept.id !== domain.id) {
              v450StoreClaim({subject:concept.name,predicate:"part_of",object:domain.name,objectConcept:domain.name,confidence:0.96},{...sectionEvidence,text:`${concept.name} belongs to ${domain.name}`});
            }
          }
        }

        if (sourceRole === "instruction") {
          const procedure = v450ExtractInstructionProcedure(section, source, sectionEvidence);
          if (procedure) {
            const stored=v450StoreProcedure(procedure,sectionEvidence); if(stored){counts.procedures++; touched.add(stored.conceptId);}
          }
          counts.guidance += v450ExtractInstructionGuidance(section,source,sectionEvidence).length;
          counts.examples += v450StoreSectionExamples(section,concept,sectionEvidence).length;

          // Explicit lesson roles are part of the current semantic learner.
          // Keep definitions/relationships/guidance as supported semantic
          // evidence; do not turn examples or warnings into answer templates.
          if (section.explicitRole === "concept" && concept) {
            const structured = v501StructuredInstructionBlocks(section.text);
            for (const line of structured.definition) {
              if (v501StoreInstructionDefinition(concept,line,sectionEvidence)) counts.claims++;
            }
            for (const line of structured.relationship) {
              if (v501StoreInstructionRelationship(concept,line,sectionEvidence)) counts.claims++;
            }
            for (const line of structured.guidance) {
              if (v501StoreInstructionGuidance(concept,line,sectionEvidence,false)) counts.guidance++;
            }
            for (const line of structured.warning) {
              if (v501StoreInstructionGuidance(concept,line,sectionEvidence,true)) counts.guidance++;
            }
            if (v501StoreExplicitInstructionProcedure(section,concept,structured.procedure,sectionEvidence)) {
              counts.procedures++;
            }
          }

          // Definitions of mechanisms are semantic too, but example blocks are
          // never parsed as factual subject matter.
          const blocks=v450SectionBlocks(section.text);
          for(const sentence of v450SentenceList(blocks.body.join(" "))){
            if(/\b(?:is|means|refers to|happens when|comes from)\b/i.test(sentence)){
              const claims=v450ParseKnowledgeSentence(sentence,concept,{...sectionEvidence,text:sentence});
              counts.claims += claims.length;
              counts.claims += v450LinkClaimsToSection(claims, concept, {...sectionEvidence,text:sentence});
            }
          }
        } else {
          const blocks=v450SectionBlocks(section.text);
          const factualText=[...blocks.body,...blocks.explanation,...blocks.important].join(" ");
          for(const sentence of v450SentenceList(factualText)){
            const claims=v450ParseKnowledgeSentence(sentence,concept,{...sectionEvidence,text:sentence});
            counts.claims += claims.length;
            counts.claims += v450LinkClaimsToSection(claims, concept, {...sectionEvidence,text:sentence});
          }
          counts.examples += v450StoreSectionExamples(section,concept,sectionEvidence).length;
        }
      }

      counts.concepts = touched.size;
      if (!store.learnedSourceIds.includes(source.id)) store.learnedSourceIds.push(source.id);
      store.learnedSourceIds = store.learnedSourceIds.slice(-120);
      store.lastLearningReport = {
        sourceId:source.id, sourceName:source.name, sourceRole, domain:domain?.name||domainName,
        counts, concepts:counts.concepts, claims:counts.claims, procedures:counts.procedures,
        guidance:counts.guidance, examples:counts.examples,
        createdAt:Date.now(), freshSemanticStore:true
      };
      // Cognitive compilation is invoked from the authoritative document learner,
      // not from a wrapper installed around it. The function is resolved at call
      // time because document_ingestion.js loads after memory.js.
      if (options.skipCognitiveCompile !== true && typeof v534CompileDocumentCognition === "function") {
        try { store.lastLearningReport.currentCognitiveLearning = v534CompileDocumentCognition(source, { synthesize: options.synthesizeCognition !== false }); }
        catch (error) { store.lastLearningReport.currentCognitiveLearning = { error: String(error?.message || error) }; }
      }
      return store.lastLearningReport;
    }

    const v450LegacyDetectTeachingAct = v440DetectTeachingAct;

    function v450DetectTeachingAct(text) {
      const clean = normalizeWhitespace(text);
      if (!clean || /\?$/.test(clean)) return null;
      const legacy = typeof v450LegacyDetectTeachingAct === "function"
        ? v450LegacyDetectTeachingAct(clean)
        : null;
      if (legacy?.procedure) return { type:"procedure", topic:legacy.topic, procedure:legacy.procedure, confidence:legacy.confidence };
      const explicit = /^(?:let me teach you|learn this|remember this|here(?:'s| is) how|this is how)\b/i.test(clean);
      const procedural = /\b(?:works? like this|supposed to work like this|meant to work like this|I would say|you would say|the steps are|procedure is)\b/i.test(clean);
      if (!explicit && !procedural) return null;
      return {type:"teaching",topic:v440ProcedureTopic(clean,{})||state.context.currentTopic||null,procedure:null,confidence:0.82};
    }

    function v450LearnConversation(text, options = {}) {
      if (window.__TAURI__?.core?.invoke) throw new Error("v450LearnConversation is retired on native Nova: Rust Pass 6 owns semantic teaching ingestion.");
      const teaching = options.teaching || v450DetectTeachingAct(text);
      const store = v450KnowledgeStore();
      const learned = { concepts:[], claims:[], procedures:[], guidance:[], examples:[] };
      const evidence={sourceId:null,sourceName:"conversation",sourceRole:"conversation",section:null,text};

      if (teaching?.procedure) {
        const proc = v450StoreProcedure({
          ...teaching.procedure,
          aliases:[teaching.topic].filter(Boolean),
          mechanisms:[v450MechanismFromTitle(teaching.topic)||"interactive_procedure"].filter(Boolean)
        }, evidence);
        if (proc) learned.procedures.push(proc);
        for (const example of teaching.procedure.examples || []) {
          const e=v450StoreExample({demonstrates:teaching.topic,content:"",sequence:example.sequence||[]},evidence); if(e) learned.examples.push(e);
        }
      } else {
        const topic = teaching?.topic || state.context.currentTopic || null;
        const concept = topic ? v450GetOrCreateConcept(topic,{kind:"conversation_topic",subjectEligible:true,evidence}) : null;
        if (concept) learned.concepts.push(concept);
        for (const sentence of v450SentenceList(text)) {
          learned.claims.push(...v450ParseKnowledgeSentence(sentence,concept,{...evidence,text:sentence}));
        }
      }

      store.lastLearningReport={
        sourceId:null,sourceName:"conversation",sourceRole:"conversation",domain:teaching?.topic||null,
        counts:{concepts:learned.concepts.length,claims:learned.claims.length,procedures:learned.procedures.length,guidance:learned.guidance.length,examples:learned.examples.length},
        createdAt:Date.now(),freshSemanticStore:true
      };
      return { teaching, learned, report:store.lastLearningReport,
        units:[...learned.procedures.map(v450ProcedureAsUnit),...learned.claims.map(v450ClaimAsUnit),...learned.guidance.map(v450GuidanceAsUnit),...learned.examples.map(v450ExampleAsUnit)] };
    }

    function v450ConceptAliases(concept) {
      return [concept.name,...(concept.aliases||[])].filter(Boolean);
    }

    function v450ResolveConcept(topic, options = {}) {
      const queryWords = new Set(v450CanonicalWords(topic));
      const requested = canonical(v450CleanHeading(topic)||"");
      const scored=[];
      for(const concept of v450KnowledgeStore().concepts){
        if(options.subjectEligibleOnly && !concept.subjectEligible) continue;
        let score=0; const reasons=[];
        for(const alias of v450ConceptAliases(concept)){
          const a=canonical(alias); const words=new Set(v450CanonicalWords(alias));
          if(a===requested){score=Math.max(score,100);reasons.push("exact concept");}
          else if(a.includes(requested)||requested.includes(a)){score=Math.max(score,82);reasons.push("direct concept phrase");}
          let overlap=0; for(const w of queryWords) if(words.has(w)) overlap++;
          if(overlap){score=Math.max(score,overlap*18 + (overlap===queryWords.size?20:0));reasons.push("concept word overlap");}
        }
        if(score>0) scored.push({concept,score,reasons});
      }
      scored.sort((a,b)=>b.score-a.score || b.concept.updatedAt-a.concept.updatedAt);
      return scored[0]?.score >= (options.minScore ?? 24) ? scored[0] : null;
    }

    function v450ProcedureMatchScore(procedure, topic) {
      const requestedKey=v450ConceptKey(topic);
      if(!requestedKey) return 0;
      const requestedWords=new Set(v450CanonicalWords(topic));
      const store=v450KnowledgeStore();
      const concept=store.concepts.find(c=>c.id===procedure.conceptId||c.key===procedure.conceptKey);
      const labels=[procedure.name,...(concept?.aliases||[])].filter(Boolean);
      let best=0;

      for(const label of labels) {
        const labelKey=v450ConceptKey(label);
        if(!labelKey) continue;
        if(labelKey===requestedKey) best=Math.max(best,100);

        const labelWords=new Set(v450CanonicalWords(label));
        let overlap=0;
        for(const word of requestedWords) if(labelWords.has(word)) overlap++;
        const queryCovered=requestedWords.size>0 && overlap===requestedWords.size;
        const labelCovered=labelWords.size>0 && overlap===labelWords.size;
        if(queryCovered || labelCovered) best=Math.max(best,88);
        if(overlap) best=Math.max(best,overlap*25 + (queryCovered?15:0));

        const compactLabel=labelKey.replace(/\s+/g,"");
        const compactRequested=requestedKey.replace(/\s+/g,"");
        if(compactLabel===compactRequested || compactLabel.includes(compactRequested) || compactRequested.includes(compactLabel)) best=Math.max(best,86);
      }
      return best;
    }

    function v450FindProcedure(topic) {
      if(!topic) return null;
      const ranked=v450KnowledgeStore().procedures
        .map(procedure=>({procedure,score:v450ProcedureMatchScore(procedure,topic)}))
        .filter(entry=>entry.score>=25)
        .sort((a,b)=>b.score-a.score || b.procedure.confidence-a.procedure.confidence);
      return ranked[0]?.procedure || null;
    }

    function v450DetectSemanticQuery(text) {
      const clean=stripEndingPunctuation(normalizeNaturalLanguage(text));
      const selfIntent=typeof v41DetectSelfIntent==="function"?v41DetectSelfIntent(text):null;
      if(selfIntent) return null;
      if(/^(?:what have you learned|what did you learn|what have i taught you|tell me what you have learned)$/i.test(clean)) return {intent:"inventory",topic:null,raw:text};
      let m=clean.match(/^(?:can|could|would) you (?:please )?explain how (.+?) works?$/i)||clean.match(/^explain how (.+?) works?$/i)||clean.match(/^how does (.+?) work$/i)||clean.match(/^how do (.+?) work$/i)||clean.match(/^how is (.+?) supposed to work$/i);
      if(m) return {intent:"explain_procedure",topic:v450CleanHeading(m[1]),raw:text,desired:["procedure"]};
      m=clean.match(/^(?:give me|show me|can you give me|can you show me) (?:an |a )?example of (.+)$/i);
      if(m) return {intent:"example",topic:v450CleanHeading(m[1]),raw:text,desired:["example"]};
      m=clean.match(/^(?:what have you learned about|what did you learn about|what do you know about|tell me about|can you tell me about|could you tell me about|what do you remember about)\s+(.+)$/i);
      if(m) return {intent:"explain_topic",topic:v450CleanHeading(m[1]),raw:text,desired:["claim","procedure"]};
      m=clean.match(/^(?:what does)\s+(.+?)\s+mean$/i);
      if(m) return {intent:"define_meaning",topic:v450CleanHeading(m[1]),raw:text,desired:["claim"]};
      m=clean.match(/^(?:what does|what do)\s+(.+?)\s+do$/i);
      if(m) return {intent:"action",topic:v450CleanHeading(m[1]),raw:text,desired:["claim"]};
      m=clean.match(/^when did\s+(.+?)(?:\s+(?:appear|arrive|begin|start|launch|release|be completed|be introduced|be developed|be created|be built))?$/i);
      if(m) return {intent:"when",topic:v450CleanHeading(m[1]),raw:text,desired:["claim"]};
      m=clean.match(/^why (?:is|are|does|do)\s+(.+)$/i);
      if(m) return {intent:"explain_topic",topic:v450CleanHeading(m[1]),raw:text,desired:["claim"]};
      m=clean.match(/^(?:what is|what are)\s+(.+)$/i);
      if(m) return {intent:"define",topic:v450CleanHeading(m[1]),raw:text,desired:["claim"]};

      // Yes/no property checks resolve the longest already-learned concept
      // at the front of the question, then use the rest as the expected value.
      m=clean.match(/^(?:is|are)\s+(.+)$/i);
      if(m) {
        const rest=normalizeWhitespace(m[1]);
        const candidates=v450KnowledgeStore().concepts
          .flatMap(concept=>v450ConceptAliases(concept).map(alias=>({concept,alias})))
          .sort((a,b)=>b.alias.length-a.alias.length);
        const hit=candidates.find(item=>canonical(rest)===canonical(item.alias)||canonical(rest).startsWith(canonical(item.alias)+" "));
        if(hit) {
          const expected=normalizeWhitespace(rest.slice(hit.alias.length));
          return {intent:"verify",topic:hit.concept.name,expected,raw:text,desired:["claim"]};
        }
      }
      return null;
    }

    function v450RelatedConcepts(concept, max=16) {
      const store=v450KnowledgeStore(); const ids=new Set([concept.id]); const keys=new Set([concept.key]);
      for(const claim of store.claims){
        if(claim.subjectId===concept.id && claim.predicate==="includes" && claim.objectConceptId) ids.add(claim.objectConceptId);
        if(claim.subjectId!==concept.id && ["part_of","topic_context"].includes(claim.predicate) && claim.objectConceptId===concept.id) ids.add(claim.subjectId);
      }
      return store.concepts.filter(c=>ids.has(c.id)||keys.has(c.key)).slice(0,max);
    }

    function v450Retrieve(query, options={}) {
      const store=v450KnowledgeStore();
      if(!query) return [];
      if(query.intent==="inventory") return [];

      if(query.intent==="explain_procedure"){
        const procedure=v450FindProcedure(query.topic);
        const results=procedure?[{kind:"procedure",procedure,score:100,reasons:["requested procedure match"]}]:[];
        store.lastRetrieval={query:JSON.parse(JSON.stringify(query)),results:results.map(r=>({kind:r.kind,id:r.procedure.id,topic:r.procedure.name,score:r.score,reasons:r.reasons})),createdAt:Date.now()};
        v43EnsureCognition().lastRetrieval={query:query.raw||query.topic,results:results.map(r=>({role:"semantic:procedure",type:"procedure",score:r.score,sourceName:r.procedure.evidence?.[0]?.sourceName||null,topic:r.procedure.name}))};
        return results;
      }

      const resolved=v450ResolveConcept(query.topic||query.raw,{minScore:24});
      if(!resolved){
        store.lastRetrieval={query:JSON.parse(JSON.stringify(query)),results:[],createdAt:Date.now()};
        v43EnsureCognition().lastRetrieval={query:query.raw||query.topic,results:[]};
        return [];
      }
      const concepts=v450RelatedConcepts(resolved.concept, options.expandLimit||18);
      const ids=new Set(concepts.map(c=>c.id));
      let results=[];

      for(const claim of store.claims){
        if(!ids.has(claim.subjectId)) continue;
        let score=55 + (claim.subjectId===resolved.concept.id?30:0) + (claim.confidence||0)*8;
        if(query.intent==="define" && ["is","is_a","definition","has_property"].includes(claim.predicate)) score+=18;
        if(query.intent==="define_meaning" && claim.predicate==="means") score+=28;
        if(query.intent==="action" && !["is","is_a","has_property","different_from"].includes(claim.predicate)) score+=16;
        if(query.intent==="when" && claim.qualifiers?.time) score+=28;
        if(query.intent==="verify" && query.expected && canonical(`${claim.predicate} ${claim.object}`).includes(canonical(query.expected))) score+=30;
        if(["part_of","topic_context"].includes(claim.predicate)) continue;
        results.push({kind:"claim",claim,score,reasons:[claim.subjectId===resolved.concept.id?"direct relationship":"related concept relationship"]});
      }
      const proc=store.procedures.find(p=>ids.has(p.conceptId)); if(proc) results.push({kind:"procedure",procedure:proc,score:query.intent==="define"?35:70,reasons:["procedure attached to concept"]});
      if(query.intent==="example"){
        for(const example of store.examples.filter(e=>e.demonstratesKey===resolved.concept.key)) results.push({kind:"example",example,score:90,reasons:["explicit example request"]});
      }
      results=results.sort((a,b)=>b.score-a.score).slice(0,options.limit||14);
      store.lastRetrieval={query:JSON.parse(JSON.stringify(query)),resolvedConcept:{id:resolved.concept.id,name:resolved.concept.name,score:resolved.score},results:results.map(r=>({kind:r.kind,id:r.claim?.id||r.procedure?.id||r.example?.id,topic:r.claim?.subject||r.procedure?.name||resolved.concept.name,score:Number(r.score.toFixed(2)),reasons:r.reasons})),createdAt:Date.now()};
      v43EnsureCognition().lastRetrieval={query:query.raw||query.topic,results:results.map(r=>({role:`semantic:${r.kind}`,type:r.kind,score:r.score,sourceName:(r.claim||r.procedure||r.example)?.evidence?.[0]?.sourceName||null,topic:r.claim?.subject||r.procedure?.name||resolved.concept.name}))};
      return results;
    }

    function v450Article(phrase) {
      return /^[aeiou]/i.test(String(phrase||"").trim())?"an":"a";
    }

    function v450RealizeClaim(claim) {
      const s=v41Capitalize(claim.subject); const o=stripEndingPunctuation(claim.object);
      const p=claim.predicate;
      const time=claim.qualifiers?.time?` in ${claim.qualifiers.time}`:"";
      if(p==="is_a") return `${s} is ${v450Article(o)} ${o}.`;
      if(p==="is") return `${s} is ${o}.`;
      if(p==="has_property") return `${s} is ${o}.`;
      if(p==="means") return `That means ${s} ${o}.`;
      if(p==="different_from") return `${s} is not the same as ${o}.`;
      if(p==="changed_from") return `${s} changed from ${o}.`;
      if(p==="changed_to") return `${s} changed to ${o}.`;
      if(p==="part_of") return `${s} is part of ${o}.`;
      if(/^was_/.test(p)||["appeared","arrived","began","started","launched","released","became"].includes(p)) {
        const detail=o==="event"?"":` ${o}`;
        return `${s} ${p.replace(/_/g," ")}${detail}${time}.`.replace(/\s+/g," ");
      }
      const verbs={
        store:"stores",use:"uses",provide:"provides",connect:"connects",manage:"manages",execute:"executes",define:"defines",protect:"protects",contain:"contains",support:"supports",represent:"represents",convert:"converts",forward:"forwards",allow:"allows",organize:"organizes",combine:"combines",integrate:"integrates",translate:"translates",run:"runs",perform:"performs",include:"includes",relies_on:"relies on",depends_on:"depends on",refer_to:"refers to"
      };
      const verb=verbs[p]||p.replace(/_/g," ");
      return `${s} ${verb} ${o}${time}.`;
    }

    function v450QuoteSentence(value) {
      const clean=normalizeWhitespace(value||"");
      if(!clean) return "";
      return /[.!?]$/.test(clean)?clean:`${clean}.`;
    }

    function v450ProcedureStepEnglish(step) {
      if(step.kind==="utterance") return `I start by saying “${v450QuoteSentence(step.content)}”`;
      if(step.kind==="expected_response") return `Then I wait for you to say “${v450QuoteSentence(step.content)}”`;
      if(step.kind==="provide_value") return `Next I provide a ${step.bind||"setup"}, using a ${String(step.valueType||"value").replace(/_/g," ")}.`;
      if(step.kind==="expected_pattern") return `You answer with the pattern “${v450QuoteSentence(step.pattern)}”`;
      if(step.kind==="deliver_payoff") return `Then I deliver the payoff connected to the ${step.relationTo||"setup"}.`;
      if(step.kind==="structure_step") return `${v41Capitalize(stripEndingPunctuation(step.content))}.`;
      return `${v41Capitalize(String(step.kind||"continue").replace(/_/g," "))}.`;
    }

    function v450ExplainProcedure(procedure) {
      if(!procedure) return null;
      const intro=procedure.interactionMode==="multi_turn"?`${v41Capitalize(procedure.name)} is a multi-turn procedure.`:`${v41Capitalize(procedure.name)} is a learned procedure.`;
      return `${intro} ${(procedure.sequence||[]).map(v450ProcedureStepEnglish).join(" ")}`;
    }

    function v450AnswerSemanticQuery(query) {
      const store=v450KnowledgeStore();
      if(query.intent==="inventory"){
        const meaningful=store.concepts.filter(c=>c.subjectEligible || c.kinds.includes("procedure"));
        if(!meaningful.length) return null;
        const names=meaningful.slice().sort((a,b)=>b.updatedAt-a.updatedAt).slice(0,12).map(c=>c.name);
        return `I currently have ${store.concepts.length} integrated concepts, ${store.claims.length} learned relationships, and ${store.procedures.length} learned procedure${store.procedures.length===1?"":"s"}. Topics I can identify include ${v41OxfordList(names)}.`;
      }
      const results=v450Retrieve(query,{limit:16});
      if(!results.length) return null;
      if(query.intent==="explain_procedure") return v450ExplainProcedure(results[0].procedure);
      if(query.intent==="example"){
        const e=results.find(r=>r.kind==="example")?.example;
        return e?`One stored example demonstrates ${query.topic}: ${e.content}`:null;
      }
      const claims=results.filter(r=>r.kind==="claim").map(r=>r.claim);
      if(query.intent==="define"){
        const direct=claims.find(c=>["is_a","is","has_property"].includes(c.predicate));
        return direct?v450RealizeClaim(direct):null;
      }
      if(query.intent==="define_meaning") {
        const direct=claims.find(c=>c.predicate==="means");
        return direct?`${v41Capitalize(direct.subject)} can mean ${stripEndingPunctuation(direct.object)}.`:null;
      }
      if(query.intent==="verify") {
        const match=claims.find(c=>query.expected && canonical(`${c.predicate} ${c.object}`).includes(canonical(query.expected)));
        return match?`Yes. ${v450RealizeClaim(match)}`:`I know about ${query.topic}, but I do not have a stored relationship that confirms ${query.expected}.`;
      }
      if(query.intent==="when") {
        const timed=claims.find(c=>c.qualifiers?.time);
        return timed?v450RealizeClaim(timed):null;
      }
      if(query.intent==="action") {
        const action=claims.find(c=>!["is","is_a","has_property","different_from"].includes(c.predicate));
        return action?v450RealizeClaim(action):null;
      }
      const sentences=[];
      const aggregateIncludes=new Set(
        claims
          .filter(c=>c.predicate==="include" && /,|\band\b/i.test(c.object))
          .map(c=>c.subjectKey)
      );
      for(const claim of claims){
        if(["part_of","topic_context"].includes(claim.predicate)) continue;
        if(claim.predicate==="includes" && aggregateIncludes.has(claim.subjectKey)) continue;
        const sentence=v450RealizeClaim(claim);
        if(sentence&&!sentences.some(x=>canonical(x)===canonical(sentence))) sentences.push(sentence);
        if(sentences.length>=6) break;
      }
      const proc=results.find(r=>r.kind==="procedure")?.procedure;
      if(proc&&sentences.length<3) sentences.push(v450ExplainProcedure(proc));
      if(!sentences.length) return null;
      return `From the meaning I have stored about ${query.topic}, ${sentences.join(" ")}`;
    }

    function v450ClaimAsUnit(claim){return{ id:claim.id,type:"proposition",topic:claim.subject,subject:claim.subject,relation:claim.predicate,object:claim.object,qualifiers:claim.qualifiers,evidence:claim.evidence,confidence:claim.confidence,entities:[claim.subject,claim.object].filter(Boolean)};}
    function v450ProcedureAsUnit(p){return{id:p.id,type:"procedure",name:p.name,topic:p.name,interactionMode:p.interactionMode,roles:p.roles,sequence:p.sequence,requirements:p.requirements,optionalTraits:p.optionalMechanisms,confidence:p.confidence,evidence:p.evidence,entities:[p.name]};}
    function v450GuidanceAsUnit(g){return{id:g.id,type:"guidance",topic:g.target,target:g.target,scope:g.scope||"local",strength:g.strength,polarity:g.polarity||"positive",directive:g.directive,evidence:g.evidence,confidence:0.9,entities:[g.target]};}
    function v450ExampleAsUnit(e){return{id:e.id,type:"example",topic:e.demonstratesKey,demonstrates:e.demonstratesKey,content:e.content,sequence:e.sequence,evidence:e.evidence,confidence:0.8,entities:[]};}
    function v450ProjectedMeaningUnits(){const s=v450KnowledgeStore();return[...s.procedures.map(v450ProcedureAsUnit),...s.claims.map(v450ClaimAsUnit),...s.guidance.map(v450GuidanceAsUnit),...s.examples.map(v450ExampleAsUnit)];}

    function v450MeaningUnitSummary(unit){
      if(unit.type==="procedure") return `${unit.name}: ${unit.interactionMode}, ${(unit.sequence||[]).length} step${(unit.sequence||[]).length===1?"":"s"}`;
      if(unit.type==="proposition") return `${unit.subject} ${String(unit.relation).replace(/_/g," ")} ${unit.object}`;
      if(unit.type==="guidance") return `${unit.target}: ${unit.strength} — ${unit.polarity==="negative" ? "do not " : ""}${unit.directive}`;
      if(unit.type==="example") return `example attached to ${unit.demonstrates||"a learned concept"}`;
      return unit.topic||unit.type;
    }

    // ----- Compatibility facade. Old storage is gone; old API names now
    // read/write the unified semantic store so downstream code can be
    // migrated incrementally without reintroducing fragment memory. -----
    v440MigrateStoredSources = function() {};
    v440MeaningUnits = function(){ return v450ProjectedMeaningUnits(); };
    v440LearnDocumentMeanings = function(source){ const report=v450LearnDocument(source); return { sourceId:report.sourceId,sourceName:report.sourceName,meaningUnits:report.counts.claims+report.counts.procedures+report.counts.guidance+report.counts.examples,counts:{proposition:report.counts.claims,procedure:report.counts.procedures,guidance:report.counts.guidance,example:report.counts.examples},migrated:false,createdAt:report.createdAt }; };
    v440DetectTeachingAct = function(text){ return v450DetectTeachingAct(text); };
    v440LearnConversationMeaning = function(text,options={}){ return v450LearnConversation(text,options); };
    v440DetectSemanticQuery = function(text){ return v450DetectSemanticQuery(text); };
    v440RetrieveMeaning = function(query,options={}){
      const results=v450Retrieve(query,options).map(entry=>({unit:entry.kind==="procedure"?v450ProcedureAsUnit(entry.procedure):entry.kind==="claim"?v450ClaimAsUnit(entry.claim):entry.kind==="example"?v450ExampleAsUnit(entry.example):null,score:entry.score,reasons:entry.reasons})).filter(entry=>entry.unit);
      const last=v43EnsureCognition().lastRetrieval;
      if(last?.results) last.results=last.results.map(item=>({...item,role:String(item.role||"").replace(/^semantic:/,"meaning:")}));
      return results;
    };
    v440FindProcedure = function(topic){ const p=v450FindProcedure(topic); return p?v450ProcedureAsUnit(p):null; };
    v440ExplainProcedure = function(procedure){ const p=procedure?.conceptKey?procedure:v450FindProcedure(procedure?.name||procedure?.topic); return p?v450ExplainProcedure(p):null; };
    v440AnswerSemanticQuery = function(query){ return v450AnswerSemanticQuery(query); };
    v440MeaningUnitSummary = function(unit){ return v450MeaningUnitSummary(unit); };

    nova4312LearnedTopics = function(){
      const store=v450KnowledgeStore();
      return store.concepts
        .filter(concept=>concept.subjectEligible && !concept.instructional && !v450IsMetaConcept(concept.name))
        .map(concept=>{
          const claims=store.claims.filter(c=>c.subjectId===concept.id);
          const related=v450RelatedConcepts(concept,12).filter(c=>c.id!==concept.id);
          return{
            topic:concept.name,conceptId:concept.id,sourceId:concept.evidence?.[0]?.sourceId||null,source:null,
            items:claims.map(claim=>({bucket:"semanticStore",item:{...v450ClaimAsUnit(claim),text:`${claim.subject} ${claim.predicate} ${claim.object}`}})),
            semanticClaims:claims,relatedConcepts:related,learnedAt:concept.updatedAt
          };
        })
        .sort((a,b)=>b.learnedAt-a.learnedAt);
    };

    nova4312FindLearnedTopic = function(query){
      const resolved=v450ResolveConcept(query,{subjectEligibleOnly:true,minScore:24});
      if(!resolved) return null;
      return nova4312LearnedTopics().find(item=>item.conceptId===resolved.concept.id)||null;
    };

    nova4312TopicBundle = function(query){
      const match=nova4312FindLearnedTopic(query); if(!match)return null;
      const store=v450KnowledgeStore(); const concept=store.concepts.find(c=>c.id===match.conceptId); if(!concept)return null;

      // Direct claims define the topic. Related concepts are useful as
      // material labels, but their own unrelated claims must not leak into
      // this bundle merely because they share a document or broad domain.
      const directClaims=store.claims.filter(c=>
        c.subjectId===concept.id &&
        !["part_of","topic_context"].includes(c.predicate)
      );

      const childIds=new Set(
        directClaims
          .filter(c=>c.predicate==="includes" && c.objectConceptId)
          .map(c=>c.objectConceptId)
      );

      // For a broad domain concept (e.g. Computers), topic_context/part_of
      // links provide a small set of directly discussed subtopics. For an
      // ordinary section concept, only its explicit children are used.
      const contextualIds=new Set();
      if(concept.kinds?.includes("domain")) {
        for(const claim of store.claims) {
          if(
            claim.subjectId!==concept.id &&
            ["part_of","topic_context"].includes(claim.predicate) &&
            claim.objectConceptId===concept.id
          ) contextualIds.add(claim.subjectId);
          if(contextualIds.size>=12) break;
        }
      }

      const relatedIds=[...new Set([...childIds,...contextualIds])].slice(0,12);
      const related=relatedIds
        .map(id=>store.concepts.find(c=>c.id===id))
        .filter(c=>c && !c.instructional && !v450IsMetaConcept(c.name));

      // Pull a few claims from contextual subtopics only for broad domain
      // summaries. Section/topic bundles stay strictly direct.
      const contextualClaims=concept.kinds?.includes("domain")
        ? store.claims.filter(c=>contextualIds.has(c.subjectId) && !["part_of","topic_context"].includes(c.predicate)).slice(0,12)
        : [];
      const claims=[...directClaims,...contextualClaims];

      const years=[...new Set(claims.map(c=>c.qualifiers?.time).filter(v=>/^\d{4}$/.test(String(v))))].sort((a,b)=>Number(a)-Number(b));
      const concepts=related.map(c=>c.name).slice(0,8);
      const facts=claims.slice(0,12).map(c=>({subject:c.subject,relation:c.predicate,object:c.object,sourceId:c.evidence?.[0]?.sourceId||null,source:c.evidence?.[0]?.sourceName||"semantic store"}));
      return{topic:concept.name,items:match.items||[],facts,years,concepts,semanticClaims:claims,meaningUnits:[...claims.map(v450ClaimAsUnit)],learnedAt:concept.updatedAt};
    };

    function v450SemanticStoreSummary(){
      const s=v450KnowledgeStore();
      return{concepts:s.concepts.length,claims:s.claims.length,procedures:s.procedures.length,guidance:s.guidance.length,examples:s.examples.length,sources:(v43Memory().sources||[]).length};
    }


    /* =========================================================
       V4.6.0 SECTION: SEMANTIC STORE AUTHORITY AT BOOT

       The V4.5 knowledgeStore is the only persistent learned-knowledge
       authority. Older arrays remain runtime compatibility containers only.
       Clear any copies that may have been serialized by an older helper,
       after the semantic store itself has been initialized/migrated.
       ========================================================= */

    function v460EnforceSemanticMemoryAuthority() {
      const memory = v43Memory();
      v450KnowledgeStore();

      memory.semantic = [];
      memory.linguistic = [];
      memory.style = [];
      memory.procedural = [];
      memory.abstracted = [];

      delete memory.meaningUnits;
      delete memory.meaningCore;
    }

    v460EnforceSemanticMemoryAuthority();


    /* =========================================================
       V5.0.1 SECTION: LEARNING HYGIENE + INSTRUCTIONAL ROLE PARSING

       V5.0.0 learned useful rules, but its document learner was still too
       willing to promote labels, practice answers, negative examples, and
       numbered action lines into concepts/procedures.  These overrides make
       instructional documents role-aware before semantic storage.
       ========================================================= */

    const NOVA501_INSTRUCTION_LABELS = new Set([
      "concept","definition","relationship","relationships","important",
      "guidance","rule","principle","behavior","meaning","procedure",
      "warning","constraint","validation rule","validation_rule",
      "executable format","executable_format","section role","section_role",
      "possible roles","correct learning","correct interpretation","result"
    ]);

    const NOVA501_NON_SEMANTIC_LABELS = new Set([
      "example","examples","practice","question","answer","reason","not",
      "not enough","incorrect interpretation","overgeneralization",
      "undergeneralization","text","lesson says","valid checks",
      "invalid new requirement","expected reasoning","source","classification",
      "best classification","primary role","secondary role","possible procedure",
      "possible section role","fresh test","new heading","heading","response"
    ]);

    const NOVA501_GENERIC_INSTRUCTIONAL_CONCEPTS = new Set([
      "procedure","guidance","constraint","warning","validation rule",
      "validation_rule","executable format","executable_format","heading",
      "section role","section_role","stored knowledge","executable cognition",
      "lesson","example","practice"
    ]);

    function v501CanonicalLabel(value) {
      return canonical(String(value || "").replace(/_/g, " ").replace(/:$/, "").trim());
    }

    const v501PreviousIsMetaConcept = v450IsMetaConcept;
    v450IsMetaConcept = function(value) {
      const cleaned = v501CanonicalLabel(v450CleanHeading(value) || "");
      if (!cleaned) return true;
      if (v501PreviousIsMetaConcept(value)) return true;
      if (/^(?:nova\s+)?curriculum(?:\s*[–—-]\s*lesson\s*\d+)?$/i.test(cleaned)) return true;
      if (/^(?:nova\s+)?curriculum\s+lesson\s*\d+$/i.test(cleaned)) return true;
      if (/^lesson\s*\d+(?:\s*[:–—-].*)?$/i.test(cleaned)) return true;
      if (/^(?:user|assistant)\s*(?:says?|asks?|response)?$/i.test(cleaned)) return true;
      if (NOVA501_NON_SEMANTIC_LABELS.has(cleaned)) return true;
      return false;
    };

    const v501PreviousHeadingFromLine = v450HeadingFromLine;
    v450HeadingFromLine = function(line) {
      const raw = String(line || "").trim();
      if (!raw) return null;

      // Explicit curriculum labels are block roles, not headings.  `concept:`
      // is handled by v450DocumentSections below because it *does* start a
      // semantic instructional concept.
      if (/^(?:definition|relationship|relationships|important|example|examples|practice|question|answer|reason|meaning|not|not enough|classification|best classification|primary role|secondary role|possible roles|possible procedure|possible section role|expected reasoning|incorrect interpretation|overgeneralization|undergeneralization|correct learning|correct interpretation|text|lesson says|valid checks|invalid new requirement|result|response)\s*:/i.test(raw)) {
        return null;
      }

      // A numbered command such as `1. Stop.` or `3. Validate.` is a step,
      // never a section title merely because it starts with a capital letter.
      const numbered = raw.match(/^\d+\s*[.)]\s+(.+)$/);
      if (numbered) {
        const candidate = normalizeWhitespace(numbered[1]);
        if (/[.!?]$/.test(candidate)) return null;
        if (/^(?:identify|determine|choose|check|validate|find|make|remove|use|add|begin|start|stop|return|regenerate|recall|notice|describe|keep|learn|avoid|prefer|preserve|inspect|store|report|test|classify|resolve|generate|retrieve|answer|wait|compare|apply)\b/i.test(candidate)) return null;
      }

      const heading = v501PreviousHeadingFromLine(line);
      if (!heading) return null;
      if (v450IsMetaConcept(heading)) return heading; // remains a boundary, but won't become a concept
      return heading;
    };

    // Rebuild sectioning so explicit `concept: X` curriculum blocks become X,
    // while labels such as `practice:` and numbered procedure steps remain in
    // the body of their parent concept.
    v450DocumentSections = function(text, source) {
      const lines = String(text || "").replace(/\r\n?/g, "\n").split("\n");
      const sections = [];
      let current = { title: v450SourceDomain(source), lines: [], meta: false, explicitRole: null };

      function push() {
        const body = current.lines.join("\n").trim();
        if (body || current.title) sections.push({ ...current, text: body });
      }

      for (const raw of lines) {
        const line = raw.trim();
        if (!line || v450IsDivider(line)) {
          if (line === "" && current.lines.length) current.lines.push("");
          continue;
        }

        const explicitConcept = line.match(/^concept\s*:\s*(.+)$/i);
        if (explicitConcept) {
          if (current.lines.some(value => value.trim())) push();
          const title = v450CleanHeading(explicitConcept[1]) || "instructional concept";
          current = { title, lines: [], meta: v450IsMetaConcept(title), explicitRole: "concept" };
          continue;
        }

        const heading = v450HeadingFromLine(line);
        if (heading && canonical(heading) !== canonical(v450SourceDomain(source))) {
          const blockLabel = /^(?:important|structure|pattern|example|examples|example pattern|example structure|example mechanism|full joke example|why it works|why this works)$/i.test(heading);
          if (blockLabel && !/^SECTION\s+\d+\s*:/i.test(line)) {
            current.lines.push(line);
            continue;
          }
          if (current.lines.some(value => value.trim())) push();
          current = { title: heading, lines: [], meta: v450IsMetaConcept(heading), explicitRole: null };
          continue;
        }

        current.lines.push(line);
      }
      push();
      return sections.filter((section,index) => section.text || index > 0);
    };

    const v501PreviousExtractInstructionProcedure = v450ExtractInstructionProcedure;
    v450ExtractInstructionProcedure = function(section, source, evidenceBase) {
      const title = v450CleanHeading(section?.title || "");
      if (!title || v450IsMetaConcept(title)) return null;
      const key = v501CanonicalLabel(title);

      // A section *about the concept of a procedure* is not itself a procedure.
      if (NOVA501_GENERIC_INSTRUCTIONAL_CONCEPTS.has(key)) return null;

      const text = String(section?.text || "");
      const hasNamedMechanism = Boolean(v450MechanismFromTitle(title));
      const titleLooksProcedural = /\b(?:procedure|process|workflow|method|steps|protocol)\b/i.test(title);
      const explicitStructure = /^\s*(?:structure|pattern)\s*:/mi.test(text);
      const explicitProcedure = /^\s*procedure\s*:/mi.test(text);
      const looksKnock = /knock[- ]?knock/i.test(`${title} ${text}`);

      // Do not create a procedure merely because the word "procedure" occurs
      // somewhere inside an explanation/example.
      if (!(hasNamedMechanism || titleLooksProcedural || explicitStructure || explicitProcedure || looksKnock)) return null;

      const result = v501PreviousExtractInstructionProcedure(section, source, evidenceBase);
      if (!result) return null;
      if (!(result.sequence || []).length && !(result.mechanisms || []).length) return null;
      return result;
    };

    function v501InstructionArtifactName(name) {
      const raw = normalizeWhitespace(String(name || ""));
      const key = v501CanonicalLabel(raw);
      if (!raw) return true;
      if (v450IsMetaConcept(raw)) return true;
      if (/^(?:user|assistant)$/i.test(key)) return true;
      if (/^(?:what|why|how|which|who|where|when)$/i.test(key)) return true;
      if (/^(?:it|this)\s+(?:is\s+not\s+(?:itself\s+)?|does\s+not\s+mean\b)/i.test(raw)) return true;
      if (/^(?:nova\s+)?curriculum(?:\s*[–—-]\s*lesson\s*\d+)?$/i.test(key)) return true;
      if (/[.!?]$/.test(raw) && /^(?:validate|stop|start|return|regenerate|check|identify|determine|choose|find|use|keep|learn|avoid|prefer|wait|answer|inspect|preserve|store|report|test|classify|resolve|generate|retrieve)\b/i.test(raw)) return true;
      return false;
    }

    function v501SanitizeSemanticKnowledgeStore() {
      if (window.__TAURI__?.core?.invoke) return { removedConcepts:0, removedProcedures:0, removedGuidance:0, reindexed:false, skipped:true, authority:"rust_v7_pass_9" };
      const store = v450KnowledgeStore();
      if (store.learningHygieneVersion === 4) return { removedConcepts:0, removedProcedures:0, removedGuidance:0, reindexed:false };

      const badConceptIds = new Set();
      for (const concept of store.concepts || []) {
        if (concept?.instructional && v501InstructionArtifactName(concept.name)) badConceptIds.add(concept.id);
      }

      const beforeConcepts = (store.concepts || []).length;
      const beforeProcedures = (store.procedures || []).length;
      const beforeGuidance = (store.guidance || []).length;
      store.concepts = (store.concepts || []).filter(c => !badConceptIds.has(c.id));
      store.claims = (store.claims || []).filter(c => !badConceptIds.has(c.subjectId) && !badConceptIds.has(c.objectId));
      store.examples = (store.examples || []).filter(e => !badConceptIds.has(e.conceptId));
      store.guidance = (store.guidance || []).filter(g => !badConceptIds.has(g.conceptId));
      store.procedures = (store.procedures || []).filter(p => !badConceptIds.has(p.conceptId) && !v501InstructionArtifactName(p.name));
      store.guidance = (store.guidance || []).filter(g => !v501StoredGuidanceIsTeachingArtifact(g));

      // Re-index stored instructional sources once with the current structured-role
      // semantic learner so definitions, relationships, guidance, warnings, and
      // procedure steps are available to the Rust semantic executive.  v450* stores merge by semantic key, so valid prior knowledge
      // is retained while missing intended concepts can be recovered.
      let reindexed = false;
      for (const source of v43Memory().sources || []) {
        try {
          if (v450SourceRole(source) !== "instruction") continue;
          v450LearnDocument(source, { reindex: true, learningHygiene: true });
          reindexed = true;
        } catch (_) {}
      }

      store.learningHygieneVersion = 4;
      store.lastHygieneReport = {
        removedConcepts: beforeConcepts - store.concepts.length,
        removedProcedures: beforeProcedures - store.procedures.length,
        removedGuidance: beforeGuidance - store.guidance.length,
        reindexed,
        cleanedAt: Date.now()
      };
      try { saveState(); } catch (_) {}
      return store.lastHygieneReport;
    }

    // Questions, negative examples, and metalinguistic disclaimers in an
    // instructional document are evidence about the lesson, not world facts.
    const v501PreviousParseKnowledgeSentence = v450ParseKnowledgeSentence;
    v450ParseKnowledgeSentence = function(sentence, sectionConcept, evidenceBase) {
      const clean = stripEndingPunctuation(normalizeWhitespace(sentence || ""));
      const raw = normalizeWhitespace(sentence || "");
      if (!clean) return [];
      if (/\?\s*$/.test(raw)) return [];
      if (/^(?:what|why|how|which|who|where|when)\b/i.test(clean)) return [];
      if (/^(?:it|this)\s+(?:is\s+not\s+(?:itself\s+)?|does\s+not\s+mean\b)/i.test(clean)) return [];
      if (evidenceBase?.sourceRole === "instruction" && /^(?:not enough|incorrect interpretation|overgeneralization|undergeneralization)\b/i.test(clean)) return [];
      return v501PreviousParseKnowledgeSentence(sentence, sectionConcept, evidenceBase);
    };

    function v501StructuredInstructionBlocks(sectionText) {
      const blocks = { definition: [], relationship: [], guidance: [], warning: [], procedure: [] };
      let mode = null;
      let stickyPractice = false;
      for (const rawLine of String(sectionText || "").replace(/\r\n?/g,"\n").split("\n")) {
        const line = normalizeWhitespace(String(rawLine || "").replace(/^[-*•]\s*/,""));
        if (!line) continue;
        if (stickyPractice) continue;
        const lm = line.match(/^([A-Za-z][A-Za-z0-9 _-]{0,42})\s*:\s*(.*)$/);
        if (lm) {
          const label = canonical(lm[1].replace(/_/g," "));
          const inline = normalizeWhitespace(lm[2] || "");
          if (label === "practice") { stickyPractice = true; mode = null; continue; }
          if (/^(?:example|examples|not|not enough|incorrect interpretation|overgeneralization|undergeneralization|bad learning|better learning|question|answer|reason|text|lesson says|classification|best classification|primary role|secondary role|expected reasoning|valid checks|invalid new requirement|new understanding|possible responses|possible section role|possible roles|result|response)$/.test(label)) { mode = "skip"; continue; }
          if (["definition","relationship","guidance","warning","procedure"].includes(label)) {
            if (mode === "skip" && label === "procedure") continue;
            mode = label;
            if (inline) blocks[mode].push(inline);
            continue;
          }
          if (/^(?:important|rule|principle|behavior|meaning|correction)$/.test(label)) { mode = null; continue; }
          if (mode === "skip") continue;
          // A colon can occur inside actual procedure prose (for example,
          // "When learning a new section:"). Unknown labels are therefore
          // content unless they are one of the recognized curriculum roles.
          if (mode && mode !== "skip") blocks[mode].push(line);
          continue;
        }
        if (mode && mode !== "skip") blocks[mode].push(line);
      }
      return blocks;
    }

    function v501RegexEscape(text) {
      return String(text || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    }

    function v501StoreInstructionDefinition(concept, line, evidenceBase) {
      if (!concept || !line || /\?$/.test(line)) return null;
      const clean = stripEndingPunctuation(normalizeWhitespace(line));
      if (!clean) return null;
      const name = v450CleanHeading(concept.name) || concept.name;
      const flexibleName = v501RegexEscape(name).replace(/\\ /g, "[ _-]+");
      let m = clean.match(new RegExp(`^(?:a|an|the)?\\s*${flexibleName}\\s+(is|are|means?|refers\\s+to|describes?|explains?|identifies?|checks?|limits?|names?|records?|changes?|applies?|connects?|demonstrates?|tells?)\\s+(.+)$`, "i"));
      let predicate, object;
      if (m) {
        predicate = /^(?:is|are)$/i.test(m[1]) ? "is" : v450PredicateFromVerb(m[1]);
        object = m[2];
      } else {
        // Keep a usable definition without inventing a new subject from the
        // sentence. The section's explicit concept remains the subject.
        predicate = "definition";
        object = clean;
      }
      return v450StoreClaim({subject:name,predicate,object,confidence:0.96},{...evidenceBase,text:line,role:"definition"});
    }

    function v501StoreInstructionRelationship(concept, line, evidenceBase) {
      if (!concept || !line || /\?$/.test(line)) return null;
      const clean = stripEndingPunctuation(normalizeWhitespace(line));
      if (!clean) return null;
      const arrow = clean.match(/^(.+?)\s*->\s*(.+?)\s*->\s*(.+)$/);
      if (arrow) {
        const subject = v450CleanHeading(arrow[1]);
        const relation = canonical(arrow[2]).replace(/\s+/g,"_");
        const object = normalizeWhitespace(arrow[3]);
        if (!subject || !relation || !object || v450IsMetaConcept(subject)) return null;
        return v450StoreClaim({subject,predicate:relation,object,confidence:0.98},{...evidenceBase,text:line,role:"relationship"});
      }

      // Natural relationship prose remains attached to the explicitly taught
      // concept even when the current parser cannot safely decompose every
      // verb/argument. Keeping the supported relationship is better than
      // dropping it and later forcing reasoning to hunt for an exact answer.
      const name = v450CleanHeading(concept.name) || concept.name;
      const flexibleName = v501RegexEscape(name).replace(/\\ /g, "[ _-]+");
      const m = clean.match(new RegExp(`^(?:a|an|the)?\\s*${flexibleName}\\s+(is|are|means?|refers\\s+to|describes?|explains?|identifies?|checks?|limits?|names?|records?|changes?|applies?|connects?|demonstrates?|tells?|determines?|contains?|preserves?|influences?|evaluates?|belongs?)\\s+(.+)$`, "i"));
      if (m) {
        const predicate = /^(?:is|are)$/i.test(m[1]) ? "is" : v450PredicateFromVerb(m[1]);
        return v450StoreClaim({subject:name,predicate,object:m[2],confidence:0.96},{...evidenceBase,text:line,role:"relationship"});
      }
      return v450StoreClaim({subject:name,predicate:"relationship",object:clean,confidence:0.93},{...evidenceBase,text:line,role:"relationship"});
    }

    function v501StoreInstructionGuidance(concept, line, evidenceBase, warning = false) {
      if (!concept || !line || /\?$/.test(line)) return null;
      const clean = stripEndingPunctuation(normalizeWhitespace(line));
      if (!clean) return null;
      return v450StoreGuidance({
        domain:"instruction",
        scope:"learned_concept",
        target:concept.name,
        strength:warning?"required":"recommended",
        polarity:warning?"negative":"positive",
        directive:clean
      },{...evidenceBase,text:line,role:warning?"warning":"guidance"});
    }

    function v501StoreExplicitInstructionProcedure(section, concept, lines, evidenceBase) {
      if (!concept || !Array.isArray(lines) || !lines.length) return null;
      const steps=[];
      let procedureName=null;
      let purpose=null;
      for (const raw of lines) {
        const line=normalizeWhitespace(raw);
        if (!line) continue;
        const bracketed=line.match(/^\[([^\]]+)\]$/);
        if (!procedureName && bracketed) {
          procedureName=v450CleanHeading(bracketed[1]);
          continue;
        }
        const labeledStep=line.match(/^step\s*:\s*["“]?(.+?)["”]?$/i);
        const numbered=line.match(/^\d+\s*[.)]\s*(.+)$/);
        const content=normalizeWhitespace(labeledStep?.[1] || numbered?.[1] || "");
        if (content) {
          steps.push({order:steps.length+1,actor:"assistant",kind:"structure_step",content:stripEndingPunctuation(content)});
          continue;
        }
        if (!purpose) purpose=stripEndingPunctuation(line);
      }
      if (!steps.length) return null;
      const name=procedureName || concept.name;
      const key=v501CanonicalLabel(name);
      if (!name || NOVA501_GENERIC_INSTRUCTIONAL_CONCEPTS.has(key)) return null;
      return v450StoreProcedure({
        name,
        aliases:name===concept.name?[]:[concept.name],
        interactionMode:"single_turn",
        roles:["assistant"],
        sequence:steps,
        requirements:["ordered_steps"],
        mechanisms:[],
        optionalMechanisms:[],
        purpose:purpose || null,
        subjectEligible:false,
        confidence:0.97
      },{...evidenceBase,text:lines.join("\n"),role:"procedure"});
    }

    // Preserve the *role* of lesson text when extracting semantic guidance.
    // A directive-looking sentence inside `practice:`, `example:`, `not:`, a
    // quiz, or an incorrect-interpretation block is evidence ABOUT the lesson,
    // not guidance Nova should adopt.  This is deliberately source/role based
    // rather than an exact-string blacklist.
    const NOVA501_GUIDANCE_SKIP_LABELS = new Set([
      "example","examples","practice","question","answer","reason","not",
      "not enough","incorrect interpretation","overgeneralization","undergeneralization",
      "text","lesson says","classification","best classification","primary role",
      "secondary role","expected reasoning","valid checks","invalid new requirement",
      "new understanding","possible responses","possible section role","possible roles",
      "result","response","fresh test","new heading"
    ]);
    const NOVA501_GUIDANCE_RESUME_LABELS = new Set([
      "definition","relationship","relationships","important","guidance","rule",
      "principle","behavior","meaning","correction","procedure","warning","constraint",
      "validation rule","validation_rule","executable format","executable_format"
    ]);

    function v501GuidanceLineRole(sectionText, evidenceText) {
      const wanted = normalizeWhitespace(String(evidenceText || "").replace(/^[-*•]\s*/, ""));
      if (!wanted) return { role:"unknown", blocked:false };
      let mode="body";
      let stickyPractice=false;
      for (const rawLine of String(sectionText || "").replace(/\r\n?/g,"\n").split("\n")) {
        const line=normalizeWhitespace(String(rawLine||"").replace(/^[-*•]\s*/,""));
        if(!line) continue;
        if(v450IsDivider(line)){mode="body";stickyPractice=false;continue;}
        const lm=line.match(/^([A-Za-z][A-Za-z0-9 _-]{0,42})\s*:\s*(.*)$/);
        if(lm){
          const label=canonical(lm[1].replace(/_/g," "));
          if(label==="practice"){stickyPractice=true;mode="practice";continue;}
          if(stickyPractice){
            if(line===wanted || normalizeWhitespace(lm[2]||"")===wanted) return {role:"practice",blocked:true};
            continue;
          }
          if(NOVA501_GUIDANCE_SKIP_LABELS.has(label)){mode=label;continue;}
          if(NOVA501_GUIDANCE_RESUME_LABELS.has(label)){mode=label; if(normalizeWhitespace(lm[2]||"")===wanted)return {role:label,blocked:false}; continue;}
        }
        if(line===wanted) return {role:mode,blocked:NOVA501_GUIDANCE_SKIP_LABELS.has(mode)||stickyPractice};
      }
      return {role:"unknown",blocked:false};
    }

    function v501GuidanceSentenceReject(text) {
      const raw=normalizeWhitespace(text||"");
      if(!raw) return "empty";
      if(/\?\s*$/.test(raw) || /^(?:what|why|how|which|who|where|when)\b/i.test(raw)) return "question_or_test_prompt";
      if(/^(?:knowledge role|concept|definition|procedure|guidance|warning|constraint|validation rule|heading|section role|stored knowledge|executable cognition)\b.{0,55}\b(?:explains?|describes?|defines?|means?|is|are)\b/i.test(raw)) return "descriptive_lesson_content";
      return null;
    }

    const v501PreviousExtractInstructionGuidance = v450ExtractInstructionGuidance;
    v450ExtractInstructionGuidance = function(section, source, evidenceBase) {
      const stored=v501PreviousExtractInstructionGuidance(section,source,evidenceBase)||[];
      if(!stored.length) return stored;
      const rejectIds=new Set();
      for(const rule of stored){
        const ev=(rule.evidence||[]).slice(-1)[0]||{};
        const sourceText=normalizeWhitespace(ev.text||rule.directive||"");
        const role=v501GuidanceLineRole(section?.text||"",sourceText);
        if(role.blocked || v501GuidanceSentenceReject(ev.text||rule.directive)) rejectIds.add(rule.id);
      }
      if(rejectIds.size){
        const store=v450KnowledgeStore();
        store.guidance=(store.guidance||[]).filter(g=>!rejectIds.has(g.id));
      }
      return stored.filter(g=>!rejectIds.has(g.id));
    };

    function v501StoredGuidanceIsTeachingArtifact(rule){
      if(!rule) return false;
      const evidence=(rule.evidence||[]).filter(Boolean);
      if(!evidence.length) return Boolean(v501GuidanceSentenceReject(rule.directive));
      let sawInstruction=false;
      for(const ev of evidence){
        if(ev.sourceRole!=="instruction") return false;
        sawInstruction=true;
        if(v501GuidanceSentenceReject(ev.text||rule.directive)) continue;
        const source=(v43Memory().sources||[]).find(s=>s.id===ev.sourceId);
        if(!source) return false;
        let sections=[];
        try{sections=v450DocumentSections(source.text||"",source)||[];}catch(_){sections=[];}
        const section=sections.find(sec=>canonical(v450CleanHeading(sec.title)||"")===canonical(v450CleanHeading(ev.section)||""));
        if(!section) return false;
        const role=v501GuidanceLineRole(section.text,ev.text||rule.directive);
        if(!role.blocked) return false;
      }
      return sawInstruction;
    }

    const v501PreviousRealizeClaim = v450RealizeClaim;
    v450RealizeClaim = function(claim) {
      if (!claim) return v501PreviousRealizeClaim(claim);
      const s=v41Capitalize(claim.subject || "");
      const o=stripEndingPunctuation(claim.object || "");
      const map={describe:"describes",explain:"explains",identify:"identifies",check:"checks",limit:"limits",name:"names",record:"records",change:"changes",apply:"applies",connect:"connects",demonstrate:"demonstrates",tell:"tells"};
      if (claim.predicate === "definition") return v450QuoteSentence(claim.object);
      if (map[claim.predicate]) return `${s} ${map[claim.predicate]} ${o}.`;
      return v501PreviousRealizeClaim(claim);
    };

    const v501PreviousAnswerSemanticQuery = v450AnswerSemanticQuery;
    v450AnswerSemanticQuery = function(query) {
      if (query?.intent === "define") {
        const results=v450Retrieve(query,{limit:16});
        const claims=results.filter(r=>r.kind==="claim").map(r=>r.claim);
        const direct=claims.find(c=>["is_a","is","has_property","definition","means","define","describe","explain","identify","check","limit","name","record","change","apply","connect","demonstrate","tell"].includes(c.predicate));
        if (direct) return v450RealizeClaim(direct);
      }
      return v501PreviousAnswerSemanticQuery(query);
    };

    v501SanitizeSemanticKnowledgeStore();
