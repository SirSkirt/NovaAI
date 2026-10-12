"use strict";

/*
 * Nova's English/parser layer and lower-level symbolic language reasoning.
 * This contains normalization, parsing, contextual language handling, Q&A primitives,
 * semantic frames, grammar-related interpretation, arithmetic, and inference helpers.
 */



    /* =========================================================
       JS SECTION: UTILITIES
       Future file: src/utils.js
       ========================================================= */

    function normalizeWhitespace(text) {
      return text.trim().replace(/\s+/g, " ");
    }

    function stripEndingPunctuation(text) {
      return text.replace(/[.!?]+$/g, "").trim();
    }

    function randomChoice(items) {
      return items[Math.floor(Math.random() * items.length)];
    }

    function randomDelay() {
      return Math.floor(
        Math.random() *
        (APP_CONFIG.typingDelayMax - APP_CONFIG.typingDelayMin + 1)
      ) + APP_CONFIG.typingDelayMin;
    }

    function wait(ms) {
      return new Promise(resolve => setTimeout(resolve, ms));
    }

    function formatTime(timestamp = Date.now()) {
      return new Intl.DateTimeFormat([], {
        hour: "numeric",
        minute: "2-digit"
      }).format(new Date(timestamp));
    }

    function currentDateText() {
      return new Intl.DateTimeFormat([], {
        weekday: "long",
        year: "numeric",
        month: "long",
        day: "numeric"
      }).format(new Date());
    }

    function currentTimeText() {
      return new Intl.DateTimeFormat([], {
        hour: "numeric",
        minute: "2-digit",
        second: "2-digit"
      }).format(new Date());
    }

    function escapeHtml(text) {
      return String(text)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;");
    }

    function titleCase(text) {
      return text
        .trim()
        .replace(/\b[a-z]/g, c => c.toUpperCase());
    }

    function cleanEntity(text) {
      const value = stripEndingPunctuation(text).trim();

      if (/^(me|myself|i)$/i.test(value)) return "user";
      if (/^(you|yourself)$/i.test(value)) return "assistant";

      return value;
    }

    function displayEntity(entity) {
      if (entity === "user") return "you";
      if (entity === "assistant") return "me";
      return entity;
    }

    function canonical(value) {
      return String(value ?? "")
        .trim()
        .toLowerCase()
        .replace(/^(?:a|an|the)\s+/, "")
        .replace(/\s+/g, " ");
    }

    function uniqueBy(items, keyFn) {
      const seen = new Set();
      return items.filter(item => {
        const key = keyFn(item);
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
    }



    /* =========================================================
       JS SECTION: SYMBOLIC INFERENCE ENGINE
       Future file: src/brain/inference.js

       This is where "reasoning" lives for now.

       It repeatedly applies rules until no new facts appear.
       This is called forward chaining.
       ========================================================= */

    function runInference() {
      state.inferredFacts = [];

      const inferredMap = new Map();

      const addInference = (subject, relation, object, because) => {
        const fact = makeFact(subject, relation, object, {
          source: "inferred",
          because
        });

        const key = factKey(fact);

        const explicitExists = state.facts.some(existing =>
          factsEqual(existing, fact)
        );

        if (explicitExists || inferredMap.has(key)) {
          return false;
        }

        inferredMap.set(key, fact);
        return true;
      };

      let changed = true;
      let rounds = 0;

      while (changed && rounds < 12) {
        changed = false;
        rounds++;

        const workingFacts = [
          ...state.facts,
          ...inferredMap.values()
        ];

        // -------------------------------------------------------
        // RULE: sibling relationships are symmetric.
        // Alice sibling_of Bob -> Bob sibling_of Alice
        // -------------------------------------------------------
        for (const fact of workingFacts) {
          if (fact.relation === "sibling_of") {
            changed = addInference(
              fact.object,
              "sibling_of",
              fact.subject,
              `${displayEntity(fact.subject)} is a sibling of ${displayEntity(fact.object)}`
            ) || changed;
          }
        }

        // -------------------------------------------------------
        // RULE: parent and child are inverse relationships.
        // Alice parent_of Bob -> Bob child_of Alice
        // -------------------------------------------------------
        for (const fact of workingFacts) {
          if (fact.relation === "parent_of") {
            changed = addInference(
              fact.object,
              "child_of",
              fact.subject,
              `${displayEntity(fact.subject)} is a parent of ${displayEntity(fact.object)}`
            ) || changed;
          }

          if (fact.relation === "child_of") {
            changed = addInference(
              fact.object,
              "parent_of",
              fact.subject,
              `${displayEntity(fact.subject)} is a child of ${displayEntity(fact.object)}`
            ) || changed;
          }
        }

        // -------------------------------------------------------
        // RULE: spouse relationships are symmetric.
        // -------------------------------------------------------
        for (const fact of workingFacts) {
          if (fact.relation === "spouse_of") {
            changed = addInference(
              fact.object,
              "spouse_of",
              fact.subject,
              `${displayEntity(fact.subject)} is the spouse of ${displayEntity(fact.object)}`
            ) || changed;
          }
        }

        // -------------------------------------------------------
        // RULE: grandparent from a two-step parent chain.
        // A parent_of B AND B parent_of C -> A grandparent_of C
        // -------------------------------------------------------
        const parentFacts = workingFacts.filter(
          fact => fact.relation === "parent_of"
        );

        for (const first of parentFacts) {
          for (const second of parentFacts) {
            if (
              canonical(first.object) === canonical(second.subject) &&
              canonical(first.subject) !== canonical(second.object)
            ) {
              changed = addInference(
                first.subject,
                "grandparent_of",
                second.object,
                `${displayEntity(first.subject)} is a parent of ${displayEntity(first.object)}, and ${displayEntity(second.subject)} is a parent of ${displayEntity(second.object)}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: grandchild is inverse of grandparent.
        // -------------------------------------------------------
        for (const fact of workingFacts) {
          if (fact.relation === "grandparent_of") {
            changed = addInference(
              fact.object,
              "grandchild_of",
              fact.subject,
              `${displayEntity(fact.subject)} is a grandparent of ${displayEntity(fact.object)}`
            ) || changed;
          }
        }

        // -------------------------------------------------------
        // RULE: aunt/uncle-like generic relation.
        // If A is sibling of B and B is parent of C,
        // then A is parent's_sibling_of C.
        // -------------------------------------------------------
        const siblingFacts = workingFacts.filter(
          fact => fact.relation === "sibling_of"
        );

        for (const sibling of siblingFacts) {
          for (const parent of parentFacts) {
            if (
              canonical(sibling.object) === canonical(parent.subject) &&
              canonical(sibling.subject) !== canonical(parent.object)
            ) {
              changed = addInference(
                sibling.subject,
                "parents_sibling_of",
                parent.object,
                `${displayEntity(sibling.subject)} is a sibling of ${displayEntity(sibling.object)}, who is a parent of ${displayEntity(parent.object)}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: category inheritance.
        // Rex is_a dog AND dog is_a animal -> Rex is_a animal
        // -------------------------------------------------------
        const categoryFacts = workingFacts.filter(
          fact => fact.relation === "is_a"
        );

        for (const first of categoryFacts) {
          for (const second of categoryFacts) {
            if (
              canonical(first.object) === canonical(second.subject) &&
              canonical(first.subject) !== canonical(second.object)
            ) {
              changed = addInference(
                first.subject,
                "is_a",
                second.object,
                `${displayEntity(first.subject)} is ${articleFor(displayEntity(first.object))} ${displayEntity(first.object)}, and ${displayEntity(second.subject)} is ${articleFor(displayEntity(second.object))} ${displayEntity(second.object)}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: descriptive properties inherit through categories.
        // florp described_as blue + Max is_a florp -> Max blue.
        // -------------------------------------------------------
        const descriptionFacts = workingFacts.filter(
          fact => fact.relation === "described_as"
        );

        for (const category of categoryFacts) {
          for (const description of descriptionFacts) {
            if (canonical(category.object) === canonical(description.subject)) {
              changed = addInference(
                category.subject,
                "described_as",
                description.object,
                `${displayEntity(category.subject)} is ${articleFor(displayEntity(category.object))} ${displayEntity(category.object)}, and ${displayEntity(description.subject)} is described as ${description.object}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: transitive "inside / located in".
        // A in B AND B in C -> A in C
        // -------------------------------------------------------
        const locationFacts = workingFacts.filter(
          fact => fact.relation === "located_in"
        );

        for (const first of locationFacts) {
          for (const second of locationFacts) {
            if (
              canonical(first.object) === canonical(second.subject) &&
              canonical(first.subject) !== canonical(second.object)
            ) {
              changed = addInference(
                first.subject,
                "located_in",
                second.object,
                `${displayEntity(first.subject)} is in ${displayEntity(first.object)}, and ${displayEntity(second.subject)} is in ${displayEntity(second.object)}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: older-than inverse.
        // Alice older_than Bob -> Bob younger_than Alice
        // -------------------------------------------------------
        for (const fact of workingFacts) {
          if (fact.relation === "older_than") {
            changed = addInference(
              fact.object,
              "younger_than",
              fact.subject,
              `${displayEntity(fact.subject)} is older than ${displayEntity(fact.object)}`
            ) || changed;
          }

          if (fact.relation === "younger_than") {
            changed = addInference(
              fact.object,
              "older_than",
              fact.subject,
              `${displayEntity(fact.subject)} is younger than ${displayEntity(fact.object)}`
            ) || changed;
          }
        }

        // -------------------------------------------------------
        // RULE: older-than is transitive.
        // A > B and B > C -> A > C
        // -------------------------------------------------------
        const olderFacts = workingFacts.filter(
          fact => fact.relation === "older_than"
        );

        for (const first of olderFacts) {
          for (const second of olderFacts) {
            if (
              canonical(first.object) === canonical(second.subject) &&
              canonical(first.subject) !== canonical(second.object)
            ) {
              changed = addInference(
                first.subject,
                "older_than",
                second.object,
                `${displayEntity(first.subject)} is older than ${displayEntity(first.object)}, and ${displayEntity(second.subject)} is older than ${displayEntity(second.object)}`
              ) || changed;
            }
          }
        }

        // -------------------------------------------------------
        // RULE: derive event ordering from numeric years.
        // If event A ends before event B begins, A happened before B.
        // -------------------------------------------------------
        const temporalFacts = workingFacts.filter(fact =>
          ["began_at", "ended_at", "occurred_during"].includes(fact.relation)
        );

        const temporalSubjects = [...new Set(
          temporalFacts.map(fact => canonical(fact.subject))
        )];

        function eventBoundary(subjectKey, edge) {
          const related = temporalFacts.filter(fact =>
            canonical(fact.subject) === subjectKey
          );

          const preferred = edge === "start"
            ? related.find(f => f.relation === "began_at") ||
              related.find(f => f.relation === "occurred_during") ||
              related.find(f => f.relation === "ended_at")
            : related.find(f => f.relation === "ended_at") ||
              related.find(f => f.relation === "occurred_during") ||
              related.find(f => f.relation === "began_at");

          if (!preferred) return null;
          const year = extractYear(preferred.object);
          return year === null ? null : { fact: preferred, year };
        }

        for (const aKey of temporalSubjects) {
          for (const bKey of temporalSubjects) {
            if (aKey === bKey) continue;

            const aEnd = eventBoundary(aKey, "end");
            const bStart = eventBoundary(bKey, "start");

            if (aEnd && bStart && aEnd.year < bStart.year) {
              const aName = aEnd.fact.subject;
              const bName = bStart.fact.subject;

              changed = addInference(
                aName,
                "before",
                bName,
                `${displayEntity(aName)} is associated with ${aEnd.year}, which is earlier than ${displayEntity(bName)} at ${bStart.year}`
              ) || changed;

              changed = addInference(
                bName,
                "after",
                aName,
                `${displayEntity(bName)} is associated with ${bStart.year}, which is later than ${displayEntity(aName)} at ${aEnd.year}`
              ) || changed;
            }
          }
        }

        // before/after chains can themselves be transitive.
        const beforeFacts = workingFacts.filter(f => f.relation === "before");
        for (const first of beforeFacts) {
          for (const second of beforeFacts) {
            if (
              canonical(first.object) === canonical(second.subject) &&
              canonical(first.subject) !== canonical(second.object)
            ) {
              changed = addInference(
                first.subject,
                "before",
                second.object,
                `${displayEntity(first.subject)} is before ${displayEntity(first.object)}, and ${displayEntity(second.subject)} is before ${displayEntity(second.object)}`
              ) || changed;
            }
          }
        }
      }

      state.inferredFacts = [...inferredMap.values()];
      renderBrainInspector();
    }



    /* =========================================================
       JS SECTION: NATURAL LANGUAGE FACT EXTRACTION
       Future file: src/brain/parser.js

       This is intentionally pattern-based rather than AI-based.
       The goal is to turn ordinary sentences into structured facts.
       ========================================================= */

    function normalizeNaturalLanguage(text) {
      return normalizeWhitespace(text)
        .replace(/[“”]/g, '"')
        .replace(/[‘’]/g, "'")
        .replace(/^\s*(?:well|so|okay|ok|actually|basically|honestly|apparently|you know)[, ]+\s*/i, "")
        .replace(/\bcan't\b/gi, "cannot")
        .replace(/\bwon't\b/gi, "will not")
        .replace(/\bdon't\b/gi, "do not")
        .replace(/\bdoesn't\b/gi, "does not")
        .replace(/\bdidn't\b/gi, "did not")
        .replace(/\bisn't\b/gi, "is not")
        .replace(/\baren't\b/gi, "are not")
        .replace(/\bwasn't\b/gi, "was not")
        .replace(/\bweren't\b/gi, "were not")
        .replace(/\bi'm\b/gi, "I am")
        .replace(/\bi've\b/gi, "I have")
        .replace(/\bi'd\b/gi, "I would")
        .replace(/\bit's\b/gi, "it is")
        .replace(/\bthat's\b/gi, "that is")
        .replace(/\bwhat's\b/gi, "what is")
        .replace(/\bwho's\b/gi, "who is")
        .replace(/\bwhere's\b/gi, "where is")
        .replace(/\bwhen's\b/gi, "when is")
        .replace(/\s+/g, " ")
        .trim();
    }

    function looksLikeTimeExpression(value) {
      const text = canonical(value);

      return (
        /\b(?:1[0-9]{3}|20[0-9]{2}|21[0-9]{2})\b/.test(text) ||
        /\b(?:today|tomorrow|yesterday|tonight|morning|afternoon|evening|night|week|month|year|century)\b/.test(text) ||
        /\b(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/.test(text) ||
        /\b(?:january|february|march|april|may|june|july|august|september|october|november|december)\b/.test(text) ||
        /\b\d{1,2}:\d{2}\b/.test(text)
      );
    }

    function extractYear(value) {
      const match = String(value).match(/\b(1[0-9]{3}|20[0-9]{2}|21[0-9]{2})\b/);
      return match ? Number(match[1]) : null;
    }


    /* =========================================================
       JS SECTION: CONTEXTUAL ENTITY RESOLUTION
       Future file: src/brain/coreference.js

       Goal:
       Let natural references such as:
         "it"
         "the war"
         "that event"
         "this dog"
       resolve back to an entity already in working memory.

       This deliberately prefers RECENT entities so a generic phrase
       does not randomly bind to an old fact from much earlier.
       ========================================================= */

    function getKnownEntities() {
      const entities = [];

      const recent = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      entities.push(...recent);

      if (state.context.lastEntity) entities.push(state.context.lastEntity);
      if (state.context.lastTopic) entities.push(state.context.lastTopic);

      const relationshipRelations = new Set([
        "sibling_of", "parent_of", "child_of", "spouse_of",
        "grandparent_of", "grandchild_of", "parents_sibling_of",
        "older_than", "younger_than", "before", "after"
      ]);

      for (const fact of getAllFacts()) {
        if (
          fact.subject &&
          !["user", "assistant"].includes(canonical(fact.subject))
        ) {
          entities.push(fact.subject);
        }

        if (
          relationshipRelations.has(fact.relation) &&
          fact.object &&
          !["user", "assistant"].includes(canonical(fact.object))
        ) {
          entities.push(fact.object);
        }
      }

      return uniqueBy(
        entities.filter(Boolean),
        entity => canonical(entity)
      );
    }

    function entityWords(value) {
      const stopWords = new Set([
        "a", "an", "the", "this", "that", "these", "those",
        "my", "your", "his", "her", "their", "our",
        "one", "thing"
      ]);

      return canonical(value)
        .replace(/[^a-z0-9\s'-]/gi, " ")
        .split(/\s+/)
        .map(word => word.trim())
        .filter(word => word && !stopWords.has(word));
    }

    function entityHeadWord(value) {
      const words = entityWords(value);
      return words.length ? words.at(-1) : "";
    }

    function entityLooksTemporal(entity) {
      return getAllFacts().some(fact =>
        canonical(fact.subject) === canonical(entity) &&
        ["began_at", "ended_at", "occurred_during", "occurs_at", "before", "after"]
          .includes(fact.relation)
      );
    }

    function entityLooksLikePerson(entity) {
      return getAllFacts().some(fact =>
        canonical(fact.subject) === canonical(entity) &&
        [
          "gender", "age", "name", "sibling_of", "parent_of",
          "child_of", "spouse_of", "grandparent_of", "grandchild_of"
        ].includes(fact.relation)
      );
    }

    function scoreEntityReference(query, candidate, recentIndex) {
      const q = canonical(query);
      const c = canonical(candidate);

      if (!q || !c) return -Infinity;
      if (q === c) return 1000;

      let score = 0;

      const qWords = entityWords(query);
      const cWords = entityWords(candidate);
      const qSet = new Set(qWords);
      const cSet = new Set(cWords);

      // Strong match when the natural shorthand is contained in the
      // fuller entity name:
      // "the war" -> "The first world war"
      if (qWords.length && qWords.every(word => cSet.has(word))) {
        score += 70 + (qWords.length * 8);
      }

      // General token overlap.
      const overlap = qWords.filter(word => cSet.has(word)).length;
      score += overlap * 14;

      // Matching head nouns are very useful for definite descriptions.
      const qHead = entityHeadWord(query);
      const cHead = entityHeadWord(candidate);

      if (qHead && qHead === cHead) {
        score += 32;
      }

      // Generic category words can refer to an entity by its role.
      const genericEventWords = new Set([
        "event", "war", "conflict", "battle", "incident", "period"
      ]);

      if (genericEventWords.has(qHead) && entityLooksTemporal(candidate)) {
        score += 18;
      }

      const genericPersonWords = new Set([
        "person", "man", "woman", "guy", "girl", "boy"
      ]);

      if (genericPersonWords.has(qHead) && entityLooksLikePerson(candidate)) {
        score += 15;
      }

      // Prefer things mentioned recently.
      if (recentIndex >= 0) {
        score += Math.max(0, 30 - (recentIndex * 4));
      }

      return score;
    }

    function resolveDescriptiveReference(value) {
      const query = cleanEntity(value);
      const lower = canonical(query);

      // Only use fuzzy descriptive resolution when the phrase looks
      // like a reference rather than a brand-new proper entity.
      const looksReferential =
        /^(?:the|this|that|these|those)\b/i.test(String(value).trim()) ||
        entityWords(query).length === 1;

      if (!looksReferential) return null;

      const known = getKnownEntities();
      const recent = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      let best = null;
      let bestScore = 0;

      for (const candidate of known) {
        const recentIndex = recent.findIndex(
          item => canonical(item) === canonical(candidate)
        );

        const score = scoreEntityReference(
          query,
          candidate,
          recentIndex
        );

        if (score > bestScore) {
          best = candidate;
          bestScore = score;
        }
      }

      // Require enough evidence so "the thing" doesn't grab a random noun.
      return bestScore >= 46 ? best : null;
    }

    function resolveEntityReference(value) {
      const clean = cleanEntity(value);
      const lower = canonical(clean);

      if (["me", "myself", "i"].includes(lower)) return "user";
      if (["you", "yourself"].includes(lower)) return "assistant";

      // Literal values are not discourse entities. A year such as "1900"
      // must stay 1900 instead of being resolved to whatever topic happens
      // to be most salient.
      if (
        looksLikeTimeExpression(clean) ||
        /^-?\d+(?:\.\d+)?$/.test(clean)
      ) {
        return clean;
      }

      const recent = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      function recentWithGender(gender) {
        for (const entity of recent) {
          if (hasFact(entity, "gender", gender)) return entity;
        }
        return null;
      }

      if (["she", "her"].includes(lower)) {
        return recentWithGender("female") ||
          (state.context.lastEntity && state.context.lastEntity !== "user"
            ? state.context.lastEntity
            : state.context.lastTopic) || clean;
      }

      if (["he", "him"].includes(lower)) {
        return recentWithGender("male") ||
          (state.context.lastEntity && state.context.lastEntity !== "user"
            ? state.context.lastEntity
            : state.context.lastTopic) || clean;
      }

      if (["they", "them", "it", "that", "this"].includes(lower)) {
        const candidate = state.context.lastEntity;
        if (candidate && !["user", "assistant"].includes(canonical(candidate))) {
          return candidate;
        }

        if (state.context.lastTopic) return state.context.lastTopic;
      }

      const descriptive = resolveDescriptiveReference(clean);
      if (descriptive) return descriptive;

      return clean;
    }

    function simpleSingular(word) {
      const value = String(word).trim();
      if (/ies$/i.test(value)) return value.replace(/ies$/i, "y");
      if (/sses$/i.test(value)) return value.replace(/es$/i, "");
      if (/s$/i.test(value) && !/ss$/i.test(value)) return value.slice(0, -1);
      return value;
    }

    function splitNaturalClauses(text) {
      const normalized = normalizeNaturalLanguage(text);

      const basic = normalized
        .split(/\s*(?:[.;]+|\s+but\s+|\s+however,?\s+)\s*/i)
        .map(part => part.trim())
        .filter(Boolean);

      const clauses = [];

      for (const part of basic) {
        // Split "I like pizza and I hate olives" but avoid splitting
        // noun phrases such as "mac and cheese" or "Alice and Bob are siblings".
        let pieces = part.split(/\s+and\s+(?=(?:I|he|she|they|we|you|it)\s+(?:am|is|are|was|were|like|love|enjoy|hate|dislike|cannot|can|have|has|want|wants|need|needs|work|works|live|lives|started|began|ended|finished|happened|occurred)\b)/i);

        // Also allow omitted repeated subjects:
        // "WWI started in 1914 and ended in 1918"
        // "I like pizza and hate olives"
        pieces = pieces.flatMap(piece =>
          piece.split(/\s+and\s+(?=(?:started|began|ended|finished|concluded|happened|occurred|likes|loves|enjoys|hates|dislikes|like|love|enjoy|hate|dislike|work|works|live|lives|has|have|can|cannot)\b)/i)
        );

        // Separate a second explicit named subject:
        // "Alice is Bob's sister and Bob is Charlie's father."
        // We intentionally do not include "are" here, so
        // "Alice and Bob are siblings" stays together.
        pieces = pieces.flatMap(piece =>
          piece.split(/\s+and\s+(?=(?:[A-Z][A-Za-z0-9'_-]*(?:\s+[A-Z][A-Za-z0-9'_-]*){0,3})\s+(?:is|was|likes|loves|hates|dislikes|has|owns|can|cannot|started|began|ended|happened|occurred)\b)/)
        );

        clauses.push(...pieces.map(piece => piece.trim()).filter(Boolean));
      }

      return clauses;
    }

    function extractFactsFromClause(clause) {
      const facts = [];
      let clean = stripEndingPunctuation(normalizeNaturalLanguage(clause));

      // Harmless conversational wrappers should not block parsing.
      clean = clean
        .replace(/^I (?:think|guess|suppose|believe) (?:that )?/i, "")
        .replace(/^from what I know,?\s*/i, "")
        .replace(/^as far as I know,?\s*/i, "")
        .trim();

      function add(subject, relation, object) {
        subject = resolveEntityReference(subject);
        object = resolveEntityReference(object);

        if (!subject || !relation || !object) return;
        facts.push({ subject, relation, object });
      }

      function capture(pattern, builder) {
        const match = clean.match(pattern);
        if (!match) return false;

        const result = builder(match);
        if (Array.isArray(result)) {
          for (const item of result) {
            if (item) add(item.subject, item.relation, item.object);
          }
        } else if (result) {
          add(result.subject, result.relation, result.object);
        }

        return true;
      }

      // -------------------------------------------------------
      // USER IDENTITY / PERSONAL INFO
      // -------------------------------------------------------
      capture(/^my name (?:is|'s) (.+)$/i,
        m => ({ subject: "user", relation: "name", object: titleCase(m[1]) }));

      capture(/^(?:call me|you can call me|people call me|I go by) (.+)$/i,
        m => ({ subject: "user", relation: "name", object: titleCase(m[1]) }));

      capture(/^I (?:am|live|currently live|reside) (?:in|around|near) (.+)$/i,
        m => ({ subject: "user", relation: "lives_in", object: m[1] }));

      capture(/^I am from (.+)$/i,
        m => ({ subject: "user", relation: "from", object: m[1] }));

      capture(/^I (?:work|am working) (?:at|for|over at|with) (.+)$/i,
        m => ({ subject: "user", relation: "works_at", object: m[1] }));

      capture(/^(?:I work as|my job is|I am employed as) (?:a|an)?\s*(.+)$/i,
        m => ({ subject: "user", relation: "job", object: m[1] }));

      capture(/^I am (\d{1,3})(?: years old)?$/i,
        m => ({ subject: "user", relation: "age", object: m[1] }));

      // -------------------------------------------------------
      // PREFERENCES / OPINIONS
      // -------------------------------------------------------
      capture(/^I (?:(?:really|quite|pretty|absolutely|definitely|kind of|sort of) )?(?:like|love|enjoy|am a fan of|prefer) (.+)$/i,
        m => ({ subject: "user", relation: "likes", object: m[1] }));

      capture(/^(.+?) is (?:one of )?my favou?rite(?:s| things| foods| games| movies| shows)?$/i,
        m => ({ subject: "user", relation: "likes", object: m[1] }));

      capture(/^my favou?rite(?:\s+\w+)? is (.+)$/i,
        m => ({ subject: "user", relation: "likes", object: m[1] }));

      capture(/^I (?:do not (?:really )?like|dislike|hate|cannot stand|am not (?:really )?a fan of|do not care for) (.+)$/i,
        m => ({ subject: "user", relation: "dislikes", object: m[1] }));

      capture(/^(.+?) (?:(?:really|quite|pretty|absolutely|definitely|kind of|sort of) )?(?:likes|loves|enjoys|prefers|is into|is a fan of) (.+)$/i,
        m => ({ subject: m[1], relation: "likes", object: m[2] }));

      capture(/^(.+?) (?:(?:really|quite|pretty|absolutely|definitely) )?(?:dislikes|hates|cannot stand|does not (?:really )?like) (.+)$/i,
        m => ({ subject: m[1], relation: "dislikes", object: m[2] }));

      // -------------------------------------------------------
      // GOALS / NEEDS / POSSESSION / ABILITIES
      // -------------------------------------------------------
      capture(/^I (?:want|would like|hope) to (.+)$/i,
        m => ({ subject: "user", relation: "wants_to", object: m[1] }));

      capture(/^I (?:need|have got|have) to (.+)$/i,
        m => ({ subject: "user", relation: "needs_to", object: m[1] }));

      capture(/^I (?:have|own|got) (?:a|an|the)?\s*(.+)$/i,
        m => ({ subject: "user", relation: "has", object: m[1] }));

      capture(/^(.+?) (?:has|owns) (?:a|an|the)?\s*(.+)$/i,
        m => ({ subject: m[1], relation: "has", object: m[2] }));

      capture(/^(.+?) can (.+)$/i,
        m => ({ subject: m[1], relation: "capable_of", object: m[2] }));

      capture(/^(.+?) cannot (.+)$/i,
        m => ({ subject: m[1], relation: "cannot", object: m[2] }));

      // -------------------------------------------------------
      // USER STATE / CAUSE
      // -------------------------------------------------------
      capture(/^I am (.+?) because (.+)$/i,
        m => ([
          { subject: "user", relation: "state", object: m[1] },
          { subject: "user", relation: "state_reason", object: `${m[1]} because ${m[2]}` }
        ]));

      capture(/^(.+?) happened because (.+)$/i,
        m => ({ subject: m[1], relation: "caused_by", object: m[2] }));

      capture(/^(.+?) caused (.+)$/i,
        m => ({ subject: m[1], relation: "causes", object: m[2] }));

      // -------------------------------------------------------
      // FAMILY / SOCIAL RELATIONSHIPS
      // Kinship words also teach a small amount of gender information,
      // which helps pronouns like "she" and "he" resolve naturally.
      // -------------------------------------------------------
      function kinGenderFact(subject, role) {
        const female = new Set(["sister", "mother", "daughter", "wife"]);
        const male = new Set(["brother", "father", "son", "husband"]);

        if (female.has(canonical(role))) {
          return { subject, relation: "gender", object: "female" };
        }

        if (male.has(canonical(role))) {
          return { subject, relation: "gender", object: "male" };
        }

        return null;
      }

      capture(/^(.+?) is (.+?)'s (sister|brother|sibling)$/i, m => ([
        { subject: m[1], relation: "sibling_of", object: m[2] },
        kinGenderFact(m[1], m[3])
      ]));

      capture(/^(.+?)'s (sister|brother|sibling) is (.+)$/i, m => ([
        { subject: m[3], relation: "sibling_of", object: m[1] },
        kinGenderFact(m[3], m[2])
      ]));

      capture(/^(.+?) is my (sister|brother|sibling)$/i, m => ([
        { subject: m[1], relation: "sibling_of", object: "user" },
        kinGenderFact(m[1], m[2])
      ]));

      capture(/^my (sister|brother|sibling) is (.+)$/i, m => ([
        { subject: m[2], relation: "sibling_of", object: "user" },
        kinGenderFact(m[2], m[1])
      ]));

      capture(/^I have (?:a|an) (sister|brother|sibling) (?:named|called) (.+)$/i, m => ([
        { subject: m[2], relation: "sibling_of", object: "user" },
        kinGenderFact(m[2], m[1])
      ]));

      capture(/^(.+?) and (.+?) are siblings$/i,
        m => ({ subject: m[1], relation: "sibling_of", object: m[2] }));

      capture(/^(.+?) is (.+?)'s (mother|father|parent)$/i, m => ([
        { subject: m[1], relation: "parent_of", object: m[2] },
        kinGenderFact(m[1], m[3])
      ]));

      capture(/^(.+?)'s (mother|father|parent) is (.+)$/i, m => ([
        { subject: m[3], relation: "parent_of", object: m[1] },
        kinGenderFact(m[3], m[2])
      ]));

      capture(/^(.+?) is my (mother|father|parent)$/i, m => ([
        { subject: m[1], relation: "parent_of", object: "user" },
        kinGenderFact(m[1], m[2])
      ]));

      capture(/^my (mother|father|parent) is (.+)$/i, m => ([
        { subject: m[2], relation: "parent_of", object: "user" },
        kinGenderFact(m[2], m[1])
      ]));

      capture(/^(.+?) is (.+?)'s (daughter|son|child)$/i, m => ([
        { subject: m[1], relation: "child_of", object: m[2] },
        kinGenderFact(m[1], m[3])
      ]));

      capture(/^(.+?) is (?:married to|the spouse of) (.+)$/i,
        m => ({ subject: m[1], relation: "spouse_of", object: m[2] }));

      capture(/^(.+?) is the (husband|wife) of (.+)$/i, m => ([
        { subject: m[1], relation: "spouse_of", object: m[3] },
        kinGenderFact(m[1], m[2])
      ]));

      capture(/^(.+?) is (.+?)'s (wife|husband|spouse)$/i, m => ([
        { subject: m[1], relation: "spouse_of", object: m[2] },
        kinGenderFact(m[1], m[3])
      ]));

      // -------------------------------------------------------
      // AGE / COMPARISON
      // -------------------------------------------------------
      capture(/^(.+?) is older than (.+)$/i,
        m => ({ subject: m[1], relation: "older_than", object: m[2] }));

      capture(/^(.+?) is younger than (.+)$/i,
        m => ({ subject: m[1], relation: "younger_than", object: m[2] }));

      capture(/^(.+?) is (?:bigger|larger|greater) than (.+)$/i,
        m => ({ subject: m[1], relation: "greater_than", object: m[2] }));

      capture(/^(.+?) is (?:smaller|less) than (.+)$/i,
        m => ({ subject: m[1], relation: "less_than", object: m[2] }));

      // -------------------------------------------------------
      // EVENTS + TIME
      // -------------------------------------------------------
      capture(/^(.+?) (?:began|started) (?:in|on|at|during) (.+)$/i,
        m => ({ subject: m[1], relation: "began_at", object: m[2] }));

      capture(/^(.+?) (?:ended|finished|concluded|stopped) (?:in|on|at|during) (.+)$/i,
        m => ({ subject: m[1], relation: "ended_at", object: m[2] }));

      capture(/^(.+?) (?:happened|occurred|took place) (?:in|on|at|during) (.+)$/i,
        m => ({ subject: m[1], relation: "occurred_during", object: m[2] }));

      capture(/^(.+?) (?:lasted|ran|went) from (.+?) (?:to|until|through) (.+)$/i,
        m => ([
          { subject: m[1], relation: "began_at", object: m[2] },
          { subject: m[1], relation: "ended_at", object: m[3] }
        ]));

      capture(/^(.+?) was from (.+?) (?:to|until|through) (.+)$/i,
        m => ([
          { subject: m[1], relation: "began_at", object: m[2] },
          { subject: m[1], relation: "ended_at", object: m[3] }
        ]));

      capture(/^(.+?) was (?:in|during|around) (.+)$/i, m => {
        if (!looksLikeTimeExpression(m[2])) return null;
        return { subject: m[1], relation: "occurred_during", object: m[2] };
      });

      capture(/^(.+?) is (?:on|at) (.+)$/i, m => {
        if (!looksLikeTimeExpression(m[2])) return null;
        return { subject: m[1], relation: "occurs_at", object: m[2] };
      });

      // -------------------------------------------------------
      // LOCATION
      // -------------------------------------------------------
      capture(/^(.+?) (?:is|was) (?:located |based )?(?:in|inside|within|near) (.+)$/i, m => {
        if (looksLikeTimeExpression(m[2])) return null;
        return { subject: m[1], relation: "located_in", object: m[2] };
      });

      capture(/^(.+?) (?:lives|resides) in (.+)$/i,
        m => ({ subject: m[1], relation: "lives_in", object: m[2] }));

      capture(/^(.+?) is from (.+)$/i,
        m => ({ subject: m[1], relation: "from", object: m[2] }));

      // -------------------------------------------------------
      // CATEGORY / DESCRIPTION
      // -------------------------------------------------------
      capture(/^(.+?) is (?:a|an) (.+)$/i,
        m => ({ subject: m[1], relation: "is_a", object: m[2] }));

      capture(/^(.+?) are (.+)$/i, m => {
        // Simple category statement: "Dogs are animals".
        if (/\b(?:happy|sad|tired|blue|red|large|small|ready|broken|open|closed)\b/i.test(m[2])) {
          return { subject: m[1], relation: "described_as", object: m[2] };
        }

        return {
          subject: simpleSingular(m[1]),
          relation: "is_a",
          object: simpleSingular(m[2])
        };
      });

      capture(/^(.+?) is (?:known for|famous for) (.+)$/i,
        m => ({ subject: m[1], relation: "known_for", object: m[2] }));

      if (facts.length === 0) {
        capture(/^(.+?) is (.+)$/i, m => {
          // Generic fallback for ordinary descriptive statements.
          // More specific rules above get a chance first.
          if (looksLikeTimeExpression(m[2])) {
            return { subject: m[1], relation: "occurred_during", object: m[2] };
          }

          return { subject: m[1], relation: "described_as", object: m[2] };
        });
      }

      return uniqueBy(
        facts.filter(fact => fact.subject && fact.relation && fact.object),
        fact => [
          canonical(fact.subject),
          canonical(fact.relation),
          canonical(fact.object)
        ].join("|")
      );
    }

    function extractFactsFromText(text) {
      const clauses = splitNaturalClauses(text);
      const combined = [];
      let carrySubject = null;

      for (let clause of clauses) {
        // Restore omitted first-person subject in conversational compounds:
        // "I like pizza but hate olives" -> "I hate olives".
        if (
          /^(?:like|love|enjoy|prefer|hate|dislike|cannot stand|have|need|want|work|live)\b/i.test(clause)
        ) {
          clause = `I ${clause}`;
        } else if (
          carrySubject &&
          /^(?:started|began|ended|finished|concluded|happened|occurred|likes|loves|enjoys|hates|dislikes|has|can|cannot|is|was)\b/i.test(clause)
        ) {
          // Natural speech often leaves the repeated subject implicit.
          // "WWI started in 1914 and ended in 1918."
          clause = `${carrySubject} ${clause}`;
        }

        const parsed = extractFactsFromClause(clause);
        combined.push(...parsed);

        const meaningful = parsed.find(f =>
          f.subject && f.relation !== "gender"
        );
        if (meaningful) carrySubject = meaningful.subject;
      }

      return uniqueBy(
        combined,
        fact => [
          canonical(fact.subject),
          canonical(fact.relation),
          canonical(fact.object)
        ].join("|")
      );
    }



    /* =========================================================
       JS SECTION: LANGUAGE / INTENT ANALYSIS
       Future file: src/brain/intent.js
       ========================================================= */

    function analyzeMessage(text) {
      const clean = normalizeWhitespace(text);
      const lower = clean.toLowerCase();

      const analysis = {
        text: clean,
        lower,
        isQuestion: /\?$/.test(clean) || /^(who|what|where|when|why|how|do|does|did|is|are|am|can|could|would|should)\b/i.test(clean),
        isGreeting: /^(hi|hello|hey|good morning|good afternoon|good evening)\b/i.test(clean),
        isThanks: /\b(thanks|thank you|appreciate it)\b/i.test(clean),
        isFarewell: /\b(bye|goodbye|see you|later)\b/i.test(clean),
        asksHelp: /^(help|\/help)$/i.test(clean) || /what can you do/i.test(clean),
        positive: /\b(good|great|awesome|nice|happy|excited|love)\b/i.test(clean),
        negative: /\b(bad|awful|annoyed|angry|sad|tired|frustrated|hate|upset)\b/i.test(clean),
        uncertainty: /\b(maybe|perhaps|probably|i think|not sure|unsure)\b/i.test(clean)
      };

      return analysis;
    }



    /* =========================================================
       JS SECTION: QUESTION ANSWERING
       Future file: src/brain/questions.js
       ========================================================= */

    function explainFact(fact) {
      if (!fact) return null;

      if (fact.source === "explicit") {
        return "You told me that directly.";
      }

      return fact.because
        ? `I inferred it because ${fact.because}.`
        : "I inferred it from other facts.";
    }

    function getTemporalFacts(subject) {
      return getAllFacts().filter(fact =>
        canonical(fact.subject) === canonical(subject) &&
        ["began_at", "ended_at", "occurred_during", "occurs_at"].includes(fact.relation)
      );
    }

    function describeEventTime(subject) {
      const facts = getTemporalFacts(subject);

      if (!facts.length) return null;

      const start = facts.find(f => f.relation === "began_at");
      const end = facts.find(f => f.relation === "ended_at");
      const during = facts.find(f => f.relation === "occurred_during");
      const occurs = facts.find(f => f.relation === "occurs_at");

      if (start && end) {
        return `${displayEntity(subject)} ran from ${start.object} to ${end.object}.`;
      }

      if (start) return `${displayEntity(subject)} began ${start.object}.`;
      if (end) return `${displayEntity(subject)} ended ${end.object}.`;
      if (during) return `${displayEntity(subject)} occurred during ${during.object}.`;
      if (occurs) return `${displayEntity(subject)} occurs at ${occurs.object}.`;
      return null;
    }

    function resolveQuestionEntity(value) {
      return resolveEntityReference(stripEndingPunctuation(value));
    }

    function answerKnownQuestion(text) {
      let clean = stripEndingPunctuation(normalizeNaturalLanguage(text));

      // Remove polite/question wrappers without requiring command-like wording.
      clean = clean
        .replace(/^(?:do you know|do you remember|can you tell me|could you tell me|would you tell me|any idea)\s+/i, "")
        .replace(/^please tell me\s+/i, "")
        .trim();

      // -------------------------------------------------------
      // USER MEMORY QUESTIONS
      // -------------------------------------------------------
      if (/^(what is my name|who am i)$/i.test(clean)) {
        const fact = findFacts({ subject: "user", relation: "name" })[0];
        return fact
          ? `You told me your name is ${fact.object}.`
          : "You haven't told me your name yet.";
      }

      if (/^(where do i live|where am i living)$/i.test(clean)) {
        const fact = findFacts({ subject: "user", relation: "lives_in" })[0];
        return fact
          ? `You live in ${fact.object}, based on what you told me.`
          : "I don't know where you live yet.";
      }

      if (/^(where do i work|who do i work for)$/i.test(clean)) {
        const fact = findFacts({ subject: "user", relation: "works_at" })[0];
        return fact
          ? `You work at ${fact.object}.`
          : "I don't know where you work yet.";
      }

      if (/^(what do i like|what kind of things do i like)$/i.test(clean)) {
        const likes = findFacts({ subject: "user", relation: "likes" });
        if (!likes.length) return "I don't know what you like yet.";
        return `You’ve told me you like ${naturalList(likes.map(f => f.object))}.`;
      }

      if (/^(what do i not like|what do i dislike)$/i.test(clean)) {
        const dislikes = findFacts({ subject: "user", relation: "dislikes" });
        if (!dislikes.length) return "I don't have any dislikes saved for you.";
        return `You’ve told me you dislike ${naturalList(dislikes.map(f => f.object))}.`;
      }

      if (/^(what do you know about me|tell me what you know about me)$/i.test(clean)) {
        return summarizeEntity("user");
      }

      // -------------------------------------------------------
      // NATURAL YES / NO PREFERENCE QUESTIONS
      // -------------------------------------------------------
      let match = clean.match(/^(?:do i|am i someone who) (?:like|love|enjoy) (.+)$/i);
      if (match) {
        const object = match[1];
        const yes = findFacts({ subject: "user", relation: "likes", object })[0];
        const no = findFacts({ subject: "user", relation: "dislikes", object })[0];
        if (yes) return `Yes. You told me you like ${yes.object}.`;
        if (no) return `No—or at least you told me you dislike ${no.object}.`;
        return `I don't know whether you like ${object}.`;
      }

      match = clean.match(/^does (.+?) (?:like|love|enjoy|prefer) (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const object = resolveQuestionEntity(match[2]);
        const yes = findFacts({ subject, relation: "likes", object })[0];
        const no = findFacts({ subject, relation: "dislikes", object })[0];
        if (yes) return `Yes. I have ${displayEntity(subject)} as liking ${yes.object}.`;
        if (no) return `No. I have ${displayEntity(subject)} as disliking ${no.object}.`;
        return `I don't know whether ${displayEntity(subject)} likes ${object}.`;
      }

      // -------------------------------------------------------
      // TIME / EVENT QUESTIONS
      // -------------------------------------------------------
      match = clean.match(/^what year was (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const answer = describeEventTime(subject);
        return answer || `I don't know what year ${displayEntity(subject)} was associated with.`;
      }

      match = clean.match(/^when (.+?) (?:happened|occurred|took place)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const answer = describeEventTime(subject);
        return answer || `I don't know when ${displayEntity(subject)} happened.`;
      }

      match = clean.match(/^when (.+?) (?:began|started)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "began_at" })[0];
        return fact
          ? `${displayEntity(subject)} began ${fact.object}.`
          : `I don't know when ${displayEntity(subject)} began.`;
      }

      match = clean.match(/^when (.+?) (?:ended|finished|concluded)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "ended_at" })[0];
        return fact
          ? `${displayEntity(subject)} ended ${fact.object}.`
          : `I don't know when ${displayEntity(subject)} ended.`;
      }

      match = clean.match(/^when (.+?) was$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const answer = describeEventTime(subject);
        return answer || `I don't know when ${displayEntity(subject)} was.`;
      }

      match = clean.match(/^(?:when was|when did) (.+?) (?:happen|occur|take place)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const answer = describeEventTime(subject);
        return answer || `I don't know when ${displayEntity(subject)} happened.`;
      }

      match = clean.match(/^when (?:was|is) (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const answer = describeEventTime(subject);
        if (answer) return answer;
      }

      match = clean.match(/^(?:when did|what year did) (.+?) (?:begin|start)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "began_at" })[0];
        return fact
          ? `${displayEntity(subject)} began ${fact.object}.`
          : `I don't know when ${displayEntity(subject)} began.`;
      }

      match = clean.match(/^(?:when did|what year did) (.+?) (?:end|finish|conclude)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "ended_at" })[0];
        return fact
          ? `${displayEntity(subject)} ended ${fact.object}.`
          : `I don't know when ${displayEntity(subject)} ended.`;
      }

      match = clean.match(
        /^(?:how long did|how many years did) (.+?) (?:last|run|go on|continue)(?: for)?$/i
      );

      if (!match) {
        match = clean.match(
          /^how long (?:was|is) (.+?)(?: going on| around| happening)?$/i
        );
      }

      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const startFact = findFacts({ subject, relation: "began_at" })[0];
        const endFact = findFacts({ subject, relation: "ended_at" })[0];

        if (startFact && endFact) {
          const startYear = extractYear(startFact.object);
          const endYear = extractYear(endFact.object);

          if (startYear !== null && endYear !== null && endYear >= startYear) {
            const years = endYear - startYear;
            return `${displayEntity(subject)} ran from ${startFact.object} to ${endFact.object}, about ${years} year${years === 1 ? "" : "s"} by simple year subtraction.`;
          }

          return `${displayEntity(subject)} ran from ${startFact.object} to ${endFact.object}.`;
        }

        return `I don't have both a start and end time for ${displayEntity(subject)} yet.`;
      }

      match = clean.match(/^(?:did|was) (.+?) (?:happen |occur |take place )?(?:in|during|on|at) (.+)$/i);
      if (match && looksLikeTimeExpression(match[2])) {
        const subject = resolveQuestionEntity(match[1]);
        const time = resolveQuestionEntity(match[2]);
        const temporal = getTemporalFacts(subject);
        const exact = temporal.find(f => canonical(f.object) === canonical(time));

        if (exact) {
          return `Yes. I have ${displayEntity(subject)} associated with ${exact.object}.`;
        }

        const start = temporal.find(f => f.relation === "began_at");
        const end = temporal.find(f => f.relation === "ended_at");
        const queryYear = extractYear(time);
        const startYear = start ? extractYear(start.object) : null;
        const endYear = end ? extractYear(end.object) : null;

        if (
          queryYear !== null &&
          startYear !== null &&
          endYear !== null &&
          queryYear >= startYear &&
          queryYear <= endYear
        ) {
          return `Yes. ${displayEntity(subject)} spans ${start.object} to ${end.object}, so ${time} falls within it.`;
        }

        return `I can't confirm that ${displayEntity(subject)} happened in ${time} from what I know.`;
      }

      match = clean.match(/^what (?:happened|occurred) (?:in|during|on) (.+)$/i);
      if (match) {
        const time = match[1];
        const matching = getAllFacts().filter(fact =>
          ["began_at", "ended_at", "occurred_during", "occurs_at"].includes(fact.relation) &&
          canonical(fact.object) === canonical(time)
        );

        if (!matching.length) {
          return `I don't currently have any events stored for ${time}.`;
        }

        return `I have ${naturalList(matching.map(f => displayEntity(f.subject)))} associated with ${time}.`;
      }

      match = clean.match(/^did (.+?) happen before (.+)$/i);
      if (match) {
        const a = resolveQuestionEntity(match[1]);
        const b = resolveQuestionEntity(match[2]);
        const direct = findFacts({ subject: a, relation: "before", object: b })[0];
        if (direct) return `Yes. ${displayEntity(a)} happened before ${displayEntity(b)}. ${explainFact(direct)}`;

        const aFacts = getTemporalFacts(a);
        const bFacts = getTemporalFacts(b);
        const aEnd = aFacts.find(f => f.relation === "ended_at") || aFacts.find(f => f.relation === "occurred_during") || aFacts.find(f => f.relation === "began_at");
        const bStart = bFacts.find(f => f.relation === "began_at") || bFacts.find(f => f.relation === "occurred_during") || bFacts.find(f => f.relation === "ended_at");
        const ay = aEnd ? extractYear(aEnd.object) : null;
        const by = bStart ? extractYear(bStart.object) : null;

        if (ay !== null && by !== null) {
          return ay < by
            ? `Yes. The time I have for ${displayEntity(a)} is earlier than the time I have for ${displayEntity(b)}.`
            : `No. The time I have for ${displayEntity(a)} is not earlier than the time I have for ${displayEntity(b)}.`;
        }

        return `I don't have enough timing information to compare ${displayEntity(a)} and ${displayEntity(b)}.`;
      }

      // -------------------------------------------------------
      // CAUSE QUESTIONS
      // -------------------------------------------------------
      match = clean.match(/^why did (.+?) happen$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "caused_by" })[0];
        return fact
          ? `${displayEntity(subject)} happened because ${fact.object}.`
          : `I don't know what caused ${displayEntity(subject)}.`;
      }

      match = clean.match(/^what caused (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const fact = findFacts({ subject, relation: "caused_by" })[0];
        if (fact) return `${fact.object} caused ${displayEntity(subject)}, based on what I was told.`;

        const reverse = findFacts({ relation: "causes", object: subject });
        if (reverse.length) return `${naturalList(reverse.map(f => displayEntity(f.subject)))} caused ${displayEntity(subject)}.`;

        return `I don't know what caused ${displayEntity(subject)}.`;
      }

      // -------------------------------------------------------
      // COMPARISONS
      // -------------------------------------------------------
      match = clean.match(/^is (.+?) older than (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const object = resolveQuestionEntity(match[2]);
        const fact = findFacts({ subject, relation: "older_than", object })[0];
        if (fact) return `Yes. ${displayEntity(subject)} is older than ${displayEntity(object)}. ${explainFact(fact)}`;

        const reverse = findFacts({ subject: object, relation: "older_than", object: subject })[0];
        if (reverse) return `No. I actually have ${displayEntity(object)} as older than ${displayEntity(subject)}.`;

        return `I don't have enough information to compare ${displayEntity(subject)} and ${displayEntity(object)}.`;
      }

      // -------------------------------------------------------
      // CATEGORY / DESCRIPTION / CAPABILITY
      // -------------------------------------------------------
      match = clean.match(/^is (.+?) (?:a|an) (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const object = resolveQuestionEntity(match[2]);
        const fact = findFacts({ subject, relation: "is_a", object })[0];
        if (fact) return `Yes. ${displayEntity(subject)} is ${articleFor(object)} ${object}. ${explainFact(fact)}`;
        return `I don't currently know whether ${displayEntity(subject)} is ${articleFor(object)} ${object}.`;
      }

      match = clean.match(/^can (.+?) (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const action = match[2];
        const yes = findFacts({ subject, relation: "capable_of", object: action })[0];
        const no = findFacts({ subject, relation: "cannot", object: action })[0];
        if (yes) return `Yes. I have ${displayEntity(subject)} as able to ${action}.`;
        if (no) return `No. I have ${displayEntity(subject)} as unable to ${action}.`;
        return `I don't know whether ${displayEntity(subject)} can ${action}.`;
      }

      // -------------------------------------------------------
      // LOCATION
      // -------------------------------------------------------
      match = clean.match(/^where (?:is|was) (.+)$/i);
      if (match) {
        const subject = resolveQuestionEntity(match[1]);
        const facts = findFacts({ subject, relation: "located_in" });
        if (facts.length) {
          const best = facts[facts.length - 1];
          return `${displayEntity(subject)} is in ${best.object}. ${explainFact(best)}`;
        }
      }

      // -------------------------------------------------------
      // ENTITY SUMMARIES / RELATIONSHIPS
      // -------------------------------------------------------
      match = clean.match(/^what is (.+?) to (.+)$/i);
      if (match) {
        const a = resolveQuestionEntity(match[1]);
        const b = resolveQuestionEntity(match[2]);
        const direct = getAllFacts().filter(fact =>
          canonical(fact.subject) === canonical(a) &&
          canonical(fact.object) === canonical(b)
        );

        if (!direct.length) {
          return `I don't currently know a relationship between ${displayEntity(a)} and ${displayEntity(b)}.`;
        }

        const best = direct[0];
        return `${displayEntity(a)} is ${relationPhrase(best.relation, b)}. ${explainFact(best)}`;
      }

      match = clean.match(/^(?:who|what) is (.+)$/i);
      if (match) {
        const entity = resolveQuestionEntity(match[1]);
        const temporal = describeEventTime(entity);
        const summary = summarizeEntity(entity);

        if (temporal && /don't know anything/i.test(summary)) {
          return temporal;
        }

        return summary;
      }

      match = clean.match(/^(?:what do you know about|tell me about) (.+)$/i);
      if (match) {
        return summarizeEntity(resolveQuestionEntity(match[1]));
      }

      match = clean.match(/^why do you think (.+?) is older than (.+)$/i);
      if (match) {
        const fact = findFacts({
          subject: resolveQuestionEntity(match[1]),
          relation: "older_than",
          object: resolveQuestionEntity(match[2])
        })[0];

        return fact
          ? explainFact(fact)
          : "I don't actually have that conclusion stored.";
      }

      return null;
    }


    function relationPhrase(relation, object) {
      const target = displayEntity(object);

      if (canonical(object) === "user") {
        const userMap = {
          sibling_of: "your sibling",
          parent_of: "your parent",
          child_of: "your child",
          spouse_of: "your spouse",
          grandparent_of: "your grandparent",
          grandchild_of: "your grandchild"
        };

        if (userMap[relation]) return userMap[relation];
      }

      const map = {
        sibling_of: `a sibling of ${target}`,
        parent_of: `a parent of ${target}`,
        child_of: `a child of ${target}`,
        spouse_of: `the spouse of ${target}`,
        grandparent_of: `a grandparent of ${target}`,
        grandchild_of: `a grandchild of ${target}`,
        parents_sibling_of: `a sibling of one of ${target}'s parents`,
        older_than: `older than ${target}`,
        younger_than: `younger than ${target}`,
        located_in: `located in ${target}`,
        likes: `someone who likes ${target}`,
        dislikes: `someone who dislikes ${target}`,
        is_a: `${articleFor(target)} ${target}`,
        began_at: `an event that began ${target}`,
        ended_at: `an event that ended ${target}`,
        occurred_during: `an event associated with ${target}`,
        occurs_at: `scheduled for ${target}`,
        before: `before ${target}`,
        after: `after ${target}`,
        from: `from ${target}`,
        has: `someone or something that has ${target}`,
        capable_of: `able to ${target}`,
        cannot: `unable to ${target}`,
        caused_by: `caused by ${target}`,
        causes: `something that causes ${target}`,
        described_as: `${target}`,
        known_for: `known for ${target}`,
        greater_than: `greater than ${target}`,
        less_than: `less than ${target}`
      };

      return map[relation] || `${relation.replaceAll("_", " ")} ${target}`;
    }

    function naturalList(items) {
      const clean = [...new Set(items)];

      if (!clean.length) return "";
      if (clean.length === 1) return clean[0];
      if (clean.length === 2) return `${clean[0]} and ${clean[1]}`;

      return `${clean.slice(0, -1).join(", ")}, and ${clean.at(-1)}`;
    }

    function articleFor(word) {
      return /^[aeiou]/i.test(String(word).trim()) ? "an" : "a";
    }

    function summarizeEntity(entity) {
      const facts = getAllFacts().filter(fact =>
        canonical(fact.subject) === canonical(entity)
      );

      if (!facts.length) {
        return entity === "user"
          ? "I don't know much about you yet."
          : `I don't know anything about ${displayEntity(entity)} yet.`;
      }

      const descriptions = [];

      for (const fact of facts.slice(0, 12)) {
        switch (fact.relation) {
          case "name":
            descriptions.push(`name: ${fact.object}`);
            break;
          case "age":
            descriptions.push(`${fact.object} years old`);
            break;
          case "lives_in":
            descriptions.push(`lives in ${fact.object}`);
            break;
          case "works_at":
            descriptions.push(`works at ${fact.object}`);
            break;
          case "job":
            descriptions.push(`works as ${fact.object}`);
            break;
          case "likes":
            descriptions.push(`likes ${fact.object}`);
            break;
          case "dislikes":
            descriptions.push(`dislikes ${fact.object}`);
            break;
          case "has":
            descriptions.push(`has ${fact.object}`);
            break;
          case "needs_to":
            descriptions.push(`needs to ${fact.object}`);
            break;
          case "wants_to":
            descriptions.push(`wants to ${fact.object}`);
            break;
          case "is_a":
            descriptions.push(`is ${articleFor(fact.object)} ${fact.object}`);
            break;
          case "located_in":
            descriptions.push(`is in ${fact.object}`);
            break;
          case "older_than":
            descriptions.push(`is older than ${displayEntity(fact.object)}`);
            break;
          case "younger_than":
            descriptions.push(`is younger than ${displayEntity(fact.object)}`);
            break;
          case "sibling_of":
            descriptions.push(`is a sibling of ${displayEntity(fact.object)}`);
            break;
          case "parent_of":
            descriptions.push(`is a parent of ${displayEntity(fact.object)}`);
            break;
          case "child_of":
            descriptions.push(`is a child of ${displayEntity(fact.object)}`);
            break;
          case "grandparent_of":
            descriptions.push(`is a grandparent of ${displayEntity(fact.object)}`);
            break;
          case "began_at":
            descriptions.push(`began ${fact.object}`);
            break;
          case "ended_at":
            descriptions.push(`ended ${fact.object}`);
            break;
          case "occurred_during":
            descriptions.push(`occurred during ${fact.object}`);
            break;
          case "occurs_at":
            descriptions.push(`occurs at ${fact.object}`);
            break;
          case "from":
            descriptions.push(`is from ${fact.object}`);
            break;
          case "capable_of":
            descriptions.push(`can ${fact.object}`);
            break;
          case "cannot":
            descriptions.push(`cannot ${fact.object}`);
            break;
          case "caused_by":
            descriptions.push(`was caused by ${fact.object}`);
            break;
          case "causes":
            descriptions.push(`causes ${fact.object}`);
            break;
          case "described_as":
            descriptions.push(`is ${fact.object}`);
            break;
          case "known_for":
            descriptions.push(`is known for ${fact.object}`);
            break;
          case "before":
            descriptions.push(`is before ${displayEntity(fact.object)}`);
            break;
          case "after":
            descriptions.push(`is after ${displayEntity(fact.object)}`);
            break;
          default:
            break;
        }
      }

      if (!descriptions.length) {
        return `I have some data about ${displayEntity(entity)}, but nothing I know how to phrase nicely yet.`;
      }

      const opener = entity === "user"
        ? "Here’s what I currently know about you:"
        : `Here’s what I currently know about ${displayEntity(entity)}:`;

      return `${opener}\n• ${descriptions.join("\n• ")}`;
    }



    /* =========================================================
       JS SECTION: CONTEXT / FOLLOW-UP HANDLING
       Future file: src/brain/context.js

       Very primitive discourse memory:
       it remembers what the recent conversation was about.
       ========================================================= */

    function updateContextFromFacts(facts) {
      if (!facts.length) return;

      const fact = [...facts].reverse().find(item => item.relation !== "gender") || facts.at(-1);
      state.context.lastEntity = fact.subject;
      state.context.lastTopic = fact.object;
      state.context.lastIntent = "statement";

      const relationshipRelations = new Set([
        "sibling_of", "parent_of", "child_of", "spouse_of",
        "grandparent_of", "grandchild_of", "older_than",
        "younger_than", "before", "after"
      ]);

      const mentioned = [];
      for (const item of facts) {
        if (item.subject && !["user", "assistant"].includes(canonical(item.subject))) {
          mentioned.push(item.subject);
        }
        if (
          relationshipRelations.has(item.relation) &&
          item.object &&
          !["user", "assistant"].includes(canonical(item.object))
        ) {
          mentioned.push(item.object);
        }
      }

      const previous = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      state.context.recentEntities = uniqueBy(
        [...mentioned.reverse(), ...previous],
        entity => canonical(entity)
      ).slice(0, 10);
    }

    function answerContextualFollowUp(text) {
      const clean = stripEndingPunctuation(normalizeWhitespace(text));

      // Example:
      // User: "I like pizza."
      // User: "What about burgers?"
      // Assistant interprets this as asking about the same relation.
      const whatAbout = clean.match(/^what about (.+)$/i);

      if (
        whatAbout &&
        state.context.lastQuestionRelation &&
        state.context.lastEntity
      ) {
        const object = cleanEntity(whatAbout[1]);
        const relation = state.context.lastQuestionRelation;
        const subject = state.context.lastEntity;

        const fact = findFacts({
          subject,
          relation,
          object
        })[0];

        if (fact) {
          return `Yes. I have ${displayEntity(subject)} as ${relation.replaceAll("_", " ")} ${object}.`;
        }

        return `I don't have enough information about ${object} in that context yet.`;
      }

      // "And Bob?" after an older-than question.
      const andEntity = clean.match(/^and (.+)$/i);

      if (
        andEntity &&
        state.context.lastQuestionRelation === "older_than" &&
        state.context.lastEntity &&
        state.context.lastTopic
      ) {
        const newObject = cleanEntity(andEntity[1]);

        const fact = findFacts({
          subject: state.context.lastEntity,
          relation: "older_than",
          object: newObject
        })[0];

        if (fact) {
          return `Yes. ${displayEntity(state.context.lastEntity)} is also older than ${displayEntity(newObject)}. ${explainFact(fact)}`;
        }

        return `I can't establish that from the facts I have.`;
      }

      return null;
    }



    /* =========================================================
       JS SECTION: BASIC ARITHMETIC REASONER
       Future file: src/brain/math.js
       ========================================================= */

    function calculateExpression(expression) {
      const cleaned = expression
        .replace(/,/g, "")
        .replace(/\s+/g, "");

      if (!/^[0-9+\-*/().%]+$/.test(cleaned)) return null;

      try {
        const tokens = tokenizeMath(cleaned);
        const rpn = toRpn(tokens);
        const result = evaluateRpn(rpn);

        return Number.isFinite(result) ? result : null;
      } catch {
        return null;
      }
    }

    function tokenizeMath(expression) {
      const tokens = [];
      let buffer = "";

      const pushNumber = () => {
        if (!buffer) return;

        const value = Number(buffer);

        if (!Number.isFinite(value)) {
          throw new Error("Bad number");
        }

        tokens.push({ type: "number", value });
        buffer = "";
      };

      for (const char of expression) {
        if (/[0-9.]/.test(char)) {
          buffer += char;
          continue;
        }

        pushNumber();

        if ("+-*/()%".includes(char)) {
          tokens.push({ type: "operator", value: char });
        } else {
          throw new Error("Bad token");
        }
      }

      pushNumber();
      return tokens;
    }

    function toRpn(tokens) {
      const output = [];
      const operators = [];

      const precedence = {
        "+": 1,
        "-": 1,
        "*": 2,
        "/": 2,
        "%": 2
      };

      for (const token of tokens) {
        if (token.type === "number") {
          output.push(token);
          continue;
        }

        if (token.value === "(") {
          operators.push(token);
          continue;
        }

        if (token.value === ")") {
          while (
            operators.length &&
            operators.at(-1).value !== "("
          ) {
            output.push(operators.pop());
          }

          if (!operators.length) {
            throw new Error("Mismatched parentheses");
          }

          operators.pop();
          continue;
        }

        while (
          operators.length &&
          operators.at(-1).value !== "(" &&
          precedence[operators.at(-1).value] >= precedence[token.value]
        ) {
          output.push(operators.pop());
        }

        operators.push(token);
      }

      while (operators.length) {
        const op = operators.pop();

        if (op.value === "(" || op.value === ")") {
          throw new Error("Mismatched parentheses");
        }

        output.push(op);
      }

      return output;
    }

    function evaluateRpn(tokens) {
      const stack = [];

      for (const token of tokens) {
        if (token.type === "number") {
          stack.push(token.value);
          continue;
        }

        if (stack.length < 2) {
          throw new Error("Bad expression");
        }

        const b = stack.pop();
        const a = stack.pop();

        switch (token.value) {
          case "+": stack.push(a + b); break;
          case "-": stack.push(a - b); break;
          case "*": stack.push(a * b); break;
          case "/": stack.push(a / b); break;
          case "%": stack.push(a % b); break;
          default: throw new Error("Unsupported operator");
        }
      }

      if (stack.length !== 1) {
        throw new Error("Bad expression");
      }

      return stack[0];
    }



    /* =========================================================
       JS SECTION: CONVERSATIONAL RESPONSE GENERATOR
       Future file: src/brain/conversation.js
       ========================================================= */

    function acknowledgementForFacts(facts, originalText) {
      if (!facts.length) return null;

      const primary = facts[0];

      // Natural acknowledgement for paired preference statements.
      const userLike = facts.find(f => f.subject === "user" && f.relation === "likes");
      const userDislike = facts.find(f => f.subject === "user" && f.relation === "dislikes");

      if (userLike && userDislike) {
        return `Got it — you like ${userLike.object}, but you don't like ${userDislike.object}.`;
      }

      // Natural acknowledgement for event ranges.
      const start = facts.find(f => f.relation === "began_at");
      const end = facts.find(f => f.relation === "ended_at");

      if (
        start && end &&
        canonical(start.subject) === canonical(end.subject)
      ) {
        return `Got it — ${displayEntity(start.subject)} ran from ${start.object} to ${end.object}.`;
      }

      if (primary.subject === "user") {
        switch (primary.relation) {
          case "name":
            return `Got it — I'll call you ${primary.object}.`;

          case "likes":
            return randomChoice([
              `Got it. You like ${primary.object}.`,
              `${primary.object}. Noted.`,
              `Okay, I'll remember that you like ${primary.object}.`
            ]);

          case "dislikes":
            return randomChoice([
              `Got it. ${primary.object} is not your thing.`,
              `Noted — you dislike ${primary.object}.`,
              `Okay. I'll keep ${primary.object} on the "no thanks" side of the ledger.`
            ]);

          case "lives_in":
            return `Got it. You live in ${primary.object}.`;

          case "from":
            return `Got it. You're from ${primary.object}.`;

          case "works_at":
            return `Okay — you work at ${primary.object}.`;

          case "job":
            return `Got it. You work as ${primary.object}.`;

          case "age":
            return `Got it. You're ${primary.object}.`;

          case "wants_to":
            return `Got it — you want to ${primary.object}.`;

          case "needs_to":
            return `Got it. You need to ${primary.object}.`;

          case "state":
            if (/\b(tired|exhausted)\b/i.test(primary.object)) {
              return `Makes sense. I'll keep in mind that you're ${primary.object}.`;
            }
            return `Okay. I understand that you're ${primary.object}.`;
        }
      }

      switch (primary.relation) {
        case "began_at":
          return `Got it — ${displayEntity(primary.subject)} began ${primary.object}.`;

        case "ended_at":
          return `Got it — ${displayEntity(primary.subject)} ended ${primary.object}.`;

        case "occurred_during":
          return `Got it — I'll remember ${displayEntity(primary.subject)} as occurring during ${primary.object}.`;

        case "occurs_at":
          return `Got it — ${displayEntity(primary.subject)} is set for ${primary.object}.`;

        case "located_in":
          return `Got it — ${displayEntity(primary.subject)} is in ${primary.object}.`;

        case "sibling_of":
          return canonical(primary.object) === "user"
            ? `Got it — ${displayEntity(primary.subject)} is your sibling.`
            : `Got it — ${displayEntity(primary.subject)} is a sibling of ${displayEntity(primary.object)}.`;

        case "parent_of":
          return canonical(primary.object) === "user"
            ? `Got it — ${displayEntity(primary.subject)} is your parent.`
            : `Got it — ${displayEntity(primary.subject)} is a parent of ${displayEntity(primary.object)}.`;

        case "child_of":
          return canonical(primary.object) === "user"
            ? `Got it — ${displayEntity(primary.subject)} is your child.`
            : `Got it — ${displayEntity(primary.subject)} is a child of ${displayEntity(primary.object)}.`;

        case "spouse_of":
          return canonical(primary.object) === "user"
            ? `Got it — ${displayEntity(primary.subject)} is your spouse.`
            : `Got it — ${displayEntity(primary.subject)} is married to ${displayEntity(primary.object)}.`;

        case "older_than":
          return `Got it — ${displayEntity(primary.subject)} is older than ${displayEntity(primary.object)}.`;

        case "younger_than":
          return `Got it — ${displayEntity(primary.subject)} is younger than ${displayEntity(primary.object)}.`;

        case "is_a":
          return `Got it — ${displayEntity(primary.subject)} is ${articleFor(primary.object)} ${primary.object}.`;

        case "likes":
          return `Got it — ${displayEntity(primary.subject)} likes ${primary.object}.`;

        case "dislikes":
          return `Got it — ${displayEntity(primary.subject)} dislikes ${primary.object}.`;

        case "capable_of":
          return `Got it — ${displayEntity(primary.subject)} can ${primary.object}.`;

        case "cannot":
          return `Got it — ${displayEntity(primary.subject)} cannot ${primary.object}.`;

        case "described_as":
          return `Got it — ${displayEntity(primary.subject)} is ${primary.object}.`;

        case "caused_by":
          return `Got it — ${displayEntity(primary.subject)} happened because ${primary.object}.`;
      }

      if (facts.length === 1) {
        return `Got it. I'll remember that.`;
      }

      return `Got it. I understood ${facts.length} pieces of information in that.`;
    }


    function naturalFallback(analysis) {
      const recentUserMessages = state.messages
        .filter(message => message.role === "user")
        .slice(-4)
        .map(message => message.content);

      if (analysis.negative) {
        return randomChoice([
          "I understand the general tone, but I don't yet have enough language understanding to reason about all of that. I can still remember concrete facts you tell me and use them later.",
          "I can tell that's negative or frustrating, but my current language layer is still primitive. If there are concrete facts in it, I'll try to store those separately."
        ]);
      }

      if (analysis.positive) {
        return randomChoice([
          "I follow the general idea. My current brain is better at facts and relationships than open-ended conversation, but we're getting closer.",
          "I get the positive direction of that. I can't fully interpret arbitrary language yet, but I can keep building context from concrete statements."
        ]);
      }

      if (analysis.isQuestion) {
        return randomChoice([
          "I understand the question, but I don't have enough knowledge or reasoning rules to answer it yet.",
          "That's outside what I've learned so far. Emphasis on 'so far.'"
        ]);
      }

      if (recentUserMessages.length >= 2) {
        return "I'm following the conversation at a basic level, but I don't have a strong enough interpretation of that sentence to answer naturally yet. I did keep the recent context, though.";
      }

      return randomChoice([
        "I only partially understood that. My language parser is still learning how people naturally phrase things.",
        "I couldn't turn that into a meaning I trust yet, so I'm not going to pretend I understood it."
      ]);
    }



    /* =========================================================
       V4 SECTION: ENGLISH LANGUAGE CORE
       Future files:
         src/language/lexicon.js
         src/language/grammar.js

       V4's only built-in KNOWLEDGE is the English language.

       This table describes language behavior such as verb families,
       pronouns, modality, discourse cues, and grammatical roles.
       It deliberately contains no historical/scientific/geographic facts.
       ========================================================= */

    const V4_ENGLISH = {
      pronouns: {
        first: new Set(["i", "me", "myself"]),
        second: new Set(["you", "yourself"]),
        male: new Set(["he", "him", "his"]),
        female: new Set(["she", "her", "hers"]),
        neutral: new Set(["it", "its"]),
        plural: new Set(["they", "them", "their", "theirs"]),
        demonstrative: new Set(["this", "that", "these", "those"])
      },

      modals: {
        can: "ability",
        could: "possibility",
        may: "possibility",
        might: "possibility",
        must: "necessity",
        should: "recommendation",
        would: "conditional",
        will: "future",
        shall: "future"
      },

      verbFamilies: {
        begin: new Set(["begin", "begins", "began", "begun", "start", "starts", "started", "commence", "commences", "commenced"]),
        end: new Set(["end", "ends", "ended", "finish", "finishes", "finished", "conclude", "concludes", "concluded", "stop", "stops", "stopped"]),
        happen: new Set(["happen", "happens", "happened", "occur", "occurs", "occurred"]),
        like: new Set(["like", "likes", "liked", "love", "loves", "loved", "enjoy", "enjoys", "enjoyed", "prefer", "prefers", "preferred"]),
        dislike: new Set(["dislike", "dislikes", "disliked", "hate", "hates", "hated"]),
        live: new Set(["live", "lives", "lived", "reside", "resides", "resided"]),
        move: new Set(["move", "moves", "moved", "go", "goes", "went", "gone"]),
        stay: new Set(["stay", "stays", "stayed"]),
        have: new Set(["have", "has", "had", "own", "owns", "owned"]),
        spend: new Set(["spend", "spends", "spent"]),
        receive: new Set(["receive", "receives", "received", "get", "gets", "got"]),
        tell: new Set(["tell", "tells", "told"]),
        ask: new Set(["ask", "asks", "asked"]),
        know: new Set(["know", "knows", "knew", "known"]),
        remember: new Set(["remember", "remembers", "remembered"])
      },

      discourse: {
        returnToTopic: [
          /^anyway\b/i,
          /^anyhow\b/i,
          /^back to\b/i,
          /^going back to\b/i,
          /^returning to\b/i
        ],
        explicitReturn: /^(?:anyway,?\s*)?(?:back to|going back to|returning to)\s+(.+?)[.!?]?$/i
      },

      standardQuestionOpeners: new Set([
        "who", "what", "where", "when", "why", "how",
        "is", "are", "was", "were", "do", "does", "did",
        "can", "could", "would", "will", "should", "have", "has"
      ])
    };



    /* =========================================================
       V4 SECTION: CONTEXT WORKING MEMORY
       Future files:
         src/context/working-memory.js
         src/context/salience.js
         src/context/topic-stack.js

       Important distinction:
         MEMORY = things learned as facts.
         CONTEXT = what this conversation is currently about.

       "the war" should not become a permanent alias for one war.
       It should resolve to the currently salient entity that fits "war".
       ========================================================= */

    function ensureV4Context() {
      const defaults = {
        currentTopic: null,
        topicStack: [],
        salience: {},
        turn: 0,
        lastSemanticFrame: null,
        lastResolvedReferences: [],
        lastParseConfidence: 0
      };

      for (const [key, value] of Object.entries(defaults)) {
        if (state.context[key] === undefined || state.context[key] === null && key === "salience") {
          state.context[key] = Array.isArray(value)
            ? [...value]
            : (value && typeof value === "object" ? { ...value } : value);
        }
      }

      if (!Array.isArray(state.context.topicStack)) state.context.topicStack = [];
      if (!state.context.salience || typeof state.context.salience !== "object") state.context.salience = {};
      if (!Array.isArray(state.context.lastResolvedReferences)) state.context.lastResolvedReferences = [];
    }

    function v4DecaySalience() {
      ensureV4Context();

      for (const key of Object.keys(state.context.salience)) {
        const next = Number(state.context.salience[key] || 0) * 0.84;

        if (next < 0.03) delete state.context.salience[key];
        else state.context.salience[key] = next;
      }
    }

    function v4Mention(entity, boost = 0.35) {
      ensureV4Context();
      if (!entity || ["user", "assistant"].includes(canonical(entity))) return;

      const key = canonical(entity);
      state.context.salience[key] = Math.min(
        1,
        Number(state.context.salience[key] || 0) + boost
      );

      const previous = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      state.context.recentEntities = uniqueBy(
        [entity, ...previous],
        item => canonical(item)
      ).slice(0, 12);
    }

    function v4SetCurrentTopic(entity, options = {}) {
      ensureV4Context();
      if (!entity || ["user", "assistant"].includes(canonical(entity))) return;

      const old = state.context.currentTopic;

      if (
        old &&
        canonical(old) !== canonical(entity) &&
        !options.noPush
      ) {
        state.context.topicStack = uniqueBy(
          [old, ...state.context.topicStack],
          item => canonical(item)
        ).slice(0, 10);
      }

      state.context.currentTopic = entity;
      v4Mention(entity, 0.6);
    }

    function v4ReturnToPreviousTopic() {
      ensureV4Context();
      const previous = state.context.topicStack.shift();
      if (!previous) return null;

      v4SetCurrentTopic(previous, { noPush: true });
      return previous;
    }

    function v4ApplyDiscourseCue(text) {
      ensureV4Context();
      const clean = normalizeNaturalLanguage(text);

      const explicit = clean.match(V4_ENGLISH.discourse.explicitReturn);
      if (explicit) {
        const resolved = resolveEntityReference(explicit[1]);

        if (resolved && canonical(resolved) !== canonical(explicit[1]) || getKnownEntities().some(e => canonical(e) === canonical(resolved))) {
          v4SetCurrentTopic(resolved);
          return {
            handled: true,
            response: `Okay — back to ${displayEntity(resolved)}.`
          };
        }
      }

      if (
        V4_ENGLISH.discourse.returnToTopic.some(pattern => pattern.test(clean))
      ) {
        const previous = v4ReturnToPreviousTopic();
        if (previous) {
          return {
            handled: true,
            response: `Okay — back to ${displayEntity(previous)}.`
          };
        }
      }

      return { handled: false };
    }



    /* =========================================================
       V4 SECTION: SEMANTIC FRAME
       Future files:
         src/language/semantic-frame.js
         src/language/modality.js
         src/language/tense.js

       The semantic frame describes what a sentence MEANS before
       anything is allowed into permanent memory.

       This prevents:
         "If Alice is tired..."
       from becoming:
         Alice -> tired = TRUE
       ========================================================= */

    function v4ExtractQuotedText(text) {
      const results = [];
      const regex = /"([^"]+)"|'([^']+)'/g;
      let match;

      while ((match = regex.exec(text))) {
        results.push(match[1] ?? match[2]);
      }

      return results;
    }

    function v4DetectTense(text) {
      const lower = canonical(text);

      if (/\b(?:will|shall)\b/.test(lower)) return "future";

      if (/\b(?:was|were|had|began|started|ended|finished|concluded|died|went|moved|stayed|spent|received|gave|worked|lived|liked|hated)\b/.test(lower)) {
        return "past";
      }

      return "present";
    }

    function v4DetectModality(text) {
      const lower = canonical(text);

      if (/^if\b/.test(lower)) {
        return { type: "hypothetical", marker: "if" };
      }

      for (const [word, type] of Object.entries(V4_ENGLISH.modals)) {
        if (new RegExp(`\\b${word}\\b`, "i").test(lower)) {
          return { type, marker: word };
        }
      }

      if (/\b(?:probably|possibly|perhaps|maybe)\b/.test(lower) || /^(?:i think|i believe|i suppose|i guess)\b/.test(lower)) {
        return { type: "uncertain", marker: "uncertainty_word" };
      }

      return { type: "asserted", marker: null };
    }

    function v4SemanticAnalyze(text) {
      const normalized = normalizeNaturalLanguage(text);
      const clean = stripEndingPunctuation(normalized);
      const firstWord = canonical(clean).split(/\s+/)[0] || "";
      const quoted = v4ExtractQuotedText(text);
      const modality = v4DetectModality(clean);
      const negated = /\b(?:not|never|no longer)\b/i.test(clean);
      const tense = v4DetectTense(clean);

      const frame = {
        type: "statement",
        raw: text,
        normalized,
        quoted,
        modality,
        negated,
        tense,
        confidence: 0.76,
        intent: "inform",
        notes: []
      };

      // A question about a hypothetical conversation/tool capability.
      if (
        /^if\b/i.test(clean) &&
        quoted.length &&
        /\b(?:could|would|can)\s+you\b/i.test(clean)
      ) {
        frame.type = "capability_hypothetical";
        frame.intent = "ask_capability";
        frame.quotedQuestion = quoted.at(-1);
        frame.confidence = 0.9;
        frame.notes.push("Hypothetical quoted question; do not store scenario as fact.");
        return frame;
      }

      if (/^if\b/i.test(clean)) {
        frame.type = "hypothetical";
        frame.intent = "hypothesize";
        frame.confidence = 0.9;
        frame.notes.push("Conditional content is non-asserted.");
        return frame;
      }

      if (
        /\?$/.test(normalized) ||
        V4_ENGLISH.standardQuestionOpeners.has(firstWord)
      ) {
        frame.type = "question";
        frame.intent = "ask";
        frame.confidence = 0.9;
        return frame;
      }

      if (modality.type !== "asserted") {
        frame.notes.push(`Modality detected: ${modality.type}.`);
      }

      return frame;
    }



    /* =========================================================
       V4 SECTION: CONTEXTUAL REFERENCE RESOLUTION
       Future file: src/context/coreference.js

       Resolution signals:
         1. explicit name match
         2. current topic
         3. noun/head-word compatibility
         4. recent mentions
         5. salience
         6. pronoun/gender compatibility

       Ambiguity is allowed. A smart system should sometimes ask.
       ========================================================= */

    function v4ReferenceCandidates(referenceText) {
      ensureV4Context();
      const known = getKnownEntities();
      const refWords = entityWords(referenceText);
      const refHead = entityHeadWord(referenceText);
      const results = [];

      for (const candidate of known) {
        if (!candidate || ["user", "assistant"].includes(canonical(candidate))) continue;

        const cWords = entityWords(candidate);
        const cSet = new Set(cWords);
        const cHead = entityHeadWord(candidate);
        let score = 0;

        if (canonical(candidate) === canonical(referenceText)) score += 10;

        const overlap = refWords.filter(word => cSet.has(word)).length;
        score += overlap * 1.25;

        if (refHead && cHead && refHead === cHead) score += 2.1;

        // A recent temporal entity is a good candidate for generic nouns
        // such as "war" or "event" if the literal entity name does not
        // contain the head word.
        if (
          ["event", "war", "conflict", "battle", "period"].includes(refHead) &&
          entityLooksTemporal(candidate)
        ) {
          score += 1.25;
        }

        if (state.context.currentTopic && canonical(candidate) === canonical(state.context.currentTopic)) {
          score += 3.0;
        }

        const recentIndex = state.context.recentEntities.findIndex(
          item => canonical(item) === canonical(candidate)
        );

        if (recentIndex >= 0) {
          score += Math.max(0.1, 1.6 - recentIndex * 0.16);
        }

        score += Number(state.context.salience[canonical(candidate)] || 0) * 2.2;

        results.push({ candidate, score });
      }

      return results.sort((a, b) => b.score - a.score);
    }

    function v4ResolveDescriptiveReference(referenceText) {
      const clean = cleanEntity(referenceText);
      const lower = canonical(clean);

      // "the other war" explicitly avoids the current topic.
      if (/^the other\b/i.test(String(referenceText).trim())) {
        const base = clean.replace(/^other\s+/i, "");
        const candidates = v4ReferenceCandidates(base)
          .filter(item => !state.context.currentTopic || canonical(item.candidate) !== canonical(state.context.currentTopic));

        if (candidates[0] && candidates[0].score >= 2.2) {
          return candidates[0].candidate;
        }
      }

      const looksReferential =
        /^(?:the|this|that|these|those)\b/i.test(String(referenceText).trim()) ||
        entityWords(clean).length === 1;

      if (!looksReferential) return null;

      const candidates = v4ReferenceCandidates(clean);
      if (!candidates.length || candidates[0].score < 2.0) return null;

      const best = candidates[0];
      const second = candidates[1];

      // A multiword definite phrase can introduce a NEW specific entity.
      // "The Blue War" must not collapse into the current "The Red War"
      // merely because both share the head noun "war". Generic references
      // such as "the war" still use conversational context.
      const meaningfulRefWords = entityWords(clean);
      const bestWords = new Set(entityWords(best.candidate));

      if (
        meaningfulRefWords.length > 1 &&
        !meaningfulRefWords.every(word => bestWords.has(word))
      ) {
        return null;
      }

      // Keep ambiguity information for the Brain inspector and question
      // answering. We still return the best candidate only when there is a
      // useful margin.
      if (second && best.score - second.score < 0.55 && second.score >= 2.0) {
        state.context.lastResolvedReferences = [
          { reference: referenceText, candidate: best.candidate, score: best.score },
          { reference: referenceText, candidate: second.candidate, score: second.score }
        ];
        return null;
      }

      state.context.lastResolvedReferences = [
        { reference: referenceText, candidate: best.candidate, score: best.score }
      ];

      return best.candidate;
    }

    // V4 override of the V3 resolver.
    function resolveEntityReference(value) {
      ensureV4Context();

      const clean = cleanEntity(value);
      const lower = canonical(clean);

      if (["me", "myself", "i"].includes(lower)) return "user";
      if (["you", "yourself"].includes(lower)) return "assistant";

      // Literal values are not discourse entities. Keep years/numbers
      // as values even when a topic is highly salient.
      if (
        looksLikeTimeExpression(clean) ||
        /^-?\d+(?:\.\d+)?$/.test(clean)
      ) {
        return clean;
      }

      const recent = Array.isArray(state.context.recentEntities)
        ? state.context.recentEntities
        : [];

      function recentWithGender(gender) {
        return recent.find(entity => hasFact(entity, "gender", gender)) || null;
      }

      if (V4_ENGLISH.pronouns.female.has(lower)) {
        return recentWithGender("female") ||
          state.context.currentTopic ||
          recent[0] ||
          clean;
      }

      if (V4_ENGLISH.pronouns.male.has(lower)) {
        return recentWithGender("male") ||
          state.context.currentTopic ||
          recent[0] ||
          clean;
      }

      if (
        V4_ENGLISH.pronouns.neutral.has(lower) ||
        V4_ENGLISH.pronouns.plural.has(lower) ||
        V4_ENGLISH.pronouns.demonstrative.has(lower)
      ) {
        return state.context.currentTopic ||
          recent[0] ||
          state.context.lastEntity ||
          state.context.lastTopic ||
          clean;
      }

      const exactKnown = getKnownEntities().find(
        entity => canonical(entity) === canonical(clean)
      );
      if (exactKnown) return exactKnown;

      const contextual = v4ResolveDescriptiveReference(value);
      if (contextual) return contextual;

      return clean;
    }



    /* =========================================================
       V4 SECTION: CONTEXT UPDATE
       Future file: src/context/update.js

       The SUBJECT of a statement normally becomes the current topic.
       This fixes the "another war" case: the new war replaces the old
       war as conversational focus without deleting the old knowledge.
       ========================================================= */

    function updateContextFromFacts(facts) {
      ensureV4Context();
      if (!facts.length) return;

      const meaningful = facts.filter(item => item.relation !== "gender");
      const primary = meaningful[0] || facts[0];
      const last = meaningful.at(-1) || facts.at(-1);

      state.context.lastEntity = last.subject;
      state.context.lastTopic = last.object;
      state.context.lastIntent = "statement";

      for (const fact of meaningful) {
        if (fact.subject && !["user", "assistant"].includes(canonical(fact.subject))) {
          v4Mention(fact.subject, 0.42);
        }

        if (
          [
            "sibling_of", "parent_of", "child_of", "spouse_of",
            "grandparent_of", "grandchild_of", "older_than",
            "younger_than", "before", "after", "located_in"
          ].includes(fact.relation) &&
          fact.object &&
          !["user", "assistant"].includes(canonical(fact.object))
        ) {
          v4Mention(fact.object, 0.18);
        }
      }

      if (
        primary.subject &&
        !["user", "assistant"].includes(canonical(primary.subject))
      ) {
        v4SetCurrentTopic(primary.subject);
      }

      saveState();
    }



    /* =========================================================
       V4 SECTION: QUESTION GENERALIZATION
       Future file: src/reasoning/questions-v4.js

       These helpers add phrasings around the existing V3 question engine.
       The goal is not to memorize exact sentences, but to normalize common
       standard-English question structures into a canonical form.
       ========================================================= */

    function v4CanonicalizeQuestion(text) {
      let clean = stripEndingPunctuation(normalizeNaturalLanguage(text));

      // Polite wrappers are grammatical framing, not part of the core query.
      clean = clean
        .replace(/^(?:please\s+)?(?:can|could|would)\s+you\s+(?:please\s+)?(?:tell me|explain|say)\s+/i, "")
        .replace(/^(?:do you know|do you remember)\s+/i, "")
        .trim();

      // Natural duration paraphrases -> canonical V3 duration form.
      let match = clean.match(/^how many years (?:was|did) (.+?)(?: around| last| run| continue| go on)(?: for)?$/i);
      if (match) return `How long did ${match[1]} last?`;

      match = clean.match(/^for how long did (.+?) (?:last|run|continue|go on)$/i);
      if (match) return `How long did ${match[1]} last?`;

      match = clean.match(/^how long was (.+?) (?:going on|happening|around)$/i);
      if (match) return `How long did ${match[1]} last?`;

      return clean + "?";
    }

    function v4AnswerQuestion(text) {
      const canonicalQuestion = v4CanonicalizeQuestion(text);

      // Standard-English descriptive property question. V4 has no color
      // dictionary; it answers only from descriptions learned in context.
      const propertyMatch = stripEndingPunctuation(canonicalQuestion).match(
        /^what (?:color|colour) (?:is|was|would) (.+?)(?: probably)?(?: be)?$/i
      );

      if (propertyMatch) {
        const subject = resolveQuestionEntity(propertyMatch[1]);
        const descriptions = findFacts({ subject, relation: "described_as" });

        if (descriptions.length === 1) {
          const fact = descriptions[0];
          return `${displayEntity(subject)} would be ${fact.object}. ${explainFact(fact)}`;
        }

        if (descriptions.length > 1) {
          return `From what you've taught me, ${displayEntity(subject)} is described as ${naturalList(descriptions.map(f => f.object))}.`;
        }

        return `I know which ${displayEntity(subject)} you mean, but you haven't taught me a usable description for that yet.`;
      }

      const answer = answerKnownQuestion(canonicalQuestion);

      if (answer) return answer;

      // If the resolver found two almost-equal candidates, ask instead of
      // pretending certainty.
      if ((state.context.lastResolvedReferences || []).length > 1) {
        const names = state.context.lastResolvedReferences
          .slice(0, 2)
          .map(item => displayEntity(item.candidate));

        return `I can see more than one plausible thing you might mean. Do you mean ${naturalList(names)}?`;
      }

      return null;
    }



    /* =========================================================
       V4 SECTION: HYPOTHETICAL / CAPABILITY REASONING
       Future file: src/reasoning/hypotheticals.js

       V4 does not commit hypothetical statements to factual memory.
       It can, however, inspect a quoted hypothetical question and reason
       about what information would be required to answer it.
       ========================================================= */

    function v4AnswerCapabilityHypothetical(frame) {
      const quoted = frame.quotedQuestion;
      if (!quoted) {
        return "I understand that you're asking about a hypothetical ability, but I can't identify the hypothetical question itself.";
      }

      const quotedAnalysis = v4SemanticAnalyze(quoted);
      const normalizedQuestion = v4CanonicalizeQuestion(quoted);

      // Duration is a general semantic requirement: duration needs two
      // boundaries. No historical knowledge is assumed.
      if (/^how long did .+? last\?$/i.test(normalizedQuestion)) {
        const subjectMatch = normalizedQuestion.match(/^how long did (.+?) last\?$/i);
        const reference = subjectMatch ? subjectMatch[1] : "that event";

        return [
          "Yes, conditionally.",
          "",
          `If you introduced a new war and it became the current topic, I should interpret \"${reference}\" from that conversation context rather than permanently mapping those words to the war we're discussing now.`,
          "",
          "To calculate how long it lasted, I would still need enough information to know its start and end."
        ].join("\n");
      }

      if (quotedAnalysis.type === "question") {
        return "I understand the hypothetical structure, and I can recognize the quoted part as a question. Whether I could answer it would depend on what information you gave me in that hypothetical conversation.";
      }

      return "I understand that you're testing a hypothetical conversation, but I can't yet simulate that particular quoted sentence reliably.";
    }



    /* =========================================================
       V4 SECTION: SAFE FACT COMMIT
       Future file: src/reasoning/commit.js

       Assertions may become facts.
       Questions and hypotheticals may not.
       Uncertain/modal statements are understood but not promoted to
       permanent truth in V4.
       ========================================================= */

    function v4CommitAssertion(text, semanticFrame) {
      if (semanticFrame.type !== "statement") return [];

      if (!["asserted", "ability"].includes(semanticFrame.modality.type)) {
        return [];
      }

      const clean = stripEndingPunctuation(normalizeNaturalLanguage(text));
      const specialFacts = [];

      // English noun-phrase decomposition, with no world knowledge:
      //   "A florp is a blue animal"
      // becomes:
      //   florp -> is_a -> animal
      //   florp -> described_as -> blue
      //
      // The parser understands adjective + head noun grammar. It does not
      // know what "florp", "blue", or "animal" mean unless taught.
      const genericDefinition = clean.match(
        /^(?:a|an)\s+([A-Za-z][A-Za-z'-]*)\s+(?:is|was)\s+(?:a|an)\s+(.+)$/i
      );

      if (genericDefinition) {
        const subject = simpleSingular(genericDefinition[1]);
        const words = genericDefinition[2].trim().split(/\s+/).filter(Boolean);
        const headNoun = simpleSingular(words.at(-1));
        const modifiers = words.slice(0, -1).filter(word =>
          !["very", "really", "quite", "extremely", "fairly", "rather"].includes(canonical(word))
        );

        specialFacts.push({ subject, relation: "is_a", object: headNoun });

        for (const modifier of modifiers) {
          specialFacts.push({ subject, relation: "described_as", object: modifier });
        }
      }

      const extracted = specialFacts.length
        ? specialFacts
        : extractFactsFromText(text);

      for (const fact of extracted) {
        addExplicitFact(fact.subject, fact.relation, fact.object);
      }

      if (extracted.length) {
        updateContextFromFacts(extracted);
      }

      return extracted;
    }
