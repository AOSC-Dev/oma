# AGENTS.md

<!-- START liyi agent instructions rev. 1 -->
<!-- DON'T EDIT THIS BLOCK. REFRESH WITH `liyi migrate` FROM A NEWER LIYI. -->
## The 立意 (Intent Specs) design pattern for agents

When writing or modifying code:

1. For each non-trivial item (function, struct, macro invocation, decorated endpoint, etc.), infer what it SHOULD do (not what it does). Write intent to a sidecar file named `<source_filename>.liyi.jsonc` (e.g., `money.rs` → `money.rs.liyi.jsonc`). Record `source_span` (start/end lines). Do not write `source_hash` or `source_anchor` — the tool fills them in. Do not write `"reviewed"` — that is set by the human via CLI or IDE. **If you change the `intent` text of a spec that has `"reviewed": true`, you must set `"reviewed"` to `false`** (or delete the field). The human reviewed the *old* intent; your rewrite has not been reviewed. Failing to unset it silently self-approves agent-written intent, bypassing human review. Use `"intent": "=doc"` only when the docstring contains behavioral requirements (constraints, error conditions, properties), not just a functional summary — a docstring that says "Returns the sum" is not adequate; one that says "Must reject mismatched currencies with an error" is. For trivial items (simple getters, one-line wrappers, and simple expressions with no load-bearing operand order, short-circuit side effects, nested-call effects, or cost), annotate with `@liyi:trivial` instead of writing a spec. Alternatively, when working sidecar-first, use `"intent": "=trivial"` to mark items as trivial without requiring a source annotation. Use `"intent": "=self-doc"` for an item that is **not** trivial (it may be substantial — a plain data struct, a recursive helper, a container module) but whose intent is fully intrinsic to its surface form, such that any prose would tautologically restate the name/signature/fields. `=trivial` is a *risk* claim (small enough that nothing hides); `=self-doc` is an *epistemic* claim (nothing implicit hides behind the form) — do not stretch `=trivial` onto substantial code. A `=self-doc` claim holds ONLY if a reader competent in the language but a stranger to this codebase, domain, and session — holding only the item's name, signature, and durable co-located context (enclosing module, in-scope types, a governing `@liyi:note`) — could recover every property any observer (caller, attacker, scheduler, profiler, regulator) could rely on, INCLUDING non-value-domain ones: timing, resource use, termination, information flow, nondeterminism, ordering, idempotency. Side effects are presumptively disqualifying; a sequence `A; B; C` is not self-documenting when its ordering, non-idempotency, or partial-failure semantics are load-bearing and invisible in the sequence. Claiming `=self-doc` obliges you to have checked for these disqualifiers and found none — a false `=self-doc` is a lie about work done, not a judgment call. When `liyi init` emits `_hints` in a sidecar scaffold, use them to prioritize which items warrant deeper investigation (e.g., `git log`, reading tests) within your token/call budget; for unhinted items, infer from source alone.
2. When module-level invariants or governing context are apparent, write a `@liyi:note` block — in the directory's existing module doc (`README.md`, `doc.go`, `mod.rs` doc comment, etc.) or in a dedicated `LIYI.md`. Use the doc markup language's comment syntax for the marker, and close embedded blocks with `@liyi:end-note`. A note is a **context primitive, not a tracked spec**: it is marker-only — never write it to a `.liyi.jsonc`, never hash it, never add a `"reviewed"` flag. Notes apply to their directory subtree (shadowed by deeper notes); to pull a named note into a specific item's context regardless of location, annotate the item with `@liyi:see <name>`. (Optionally name a note: `@liyi:note <name>`; same-named blocks aggregate.)
3. If a source item has a `@liyi:related <name>` annotation, record the dependency in `.liyi.jsonc` as `"related": {"<name>": null}`. The tool fills in the requirement's current hash.
4. For each `@liyi:requirement <name>` block encountered (closed by `@liyi:end-requirement <name>`), ensure it has a corresponding entry in the co-located `.liyi.jsonc` with `"requirement"` and `"source_span"`. (The tool fills in `"source_hash"`.)
5. If a spec has `"related"` edges referencing a requirement, do not overwrite the requirement text during inference. Update the spec (update `source_span`) but preserve the `"related"` edges. Do not write `source_hash` — the tool fills it in.
6. Only generate adversarial tests from items that have a `@liyi:intent` annotation in source or `"reviewed": true` in the sidecar (i.e., human-reviewed intent), excluding sentinel intents (`=trivial`, `=self-doc`), which carry no agent-authored intent for a second model to attack. When `@liyi:intent` is present in source, use its prose (or the docstring for `=doc`) as the authoritative intent for test generation.
7. Tests should target boundary conditions, error-handling gaps, property violations, and semantic mismatches. Prioritize tests a subtly wrong implementation would fail.
8. Skip items annotated with `@liyi:ignore` or `@liyi:trivial`, files matched by `.liyiignore`, and files bearing a `@liyi:file ignore` directive. Respect `@liyi:nontrivial` — if present, always infer a spec for that item and never override with `@liyi:trivial` or `"=self-doc"`. Honor `@liyi:file language=<lang>` as the authoritative language for a file when present (it overrides extension-based detection). **Exclude clearly-derivative items from inference entirely** — write no sidecar entry for an item whose whole purpose is to exercise or assert another already-specced item, adding no invariant of its own (chiefly tests: a test named for the behavior it checks derives its intent from that behavior's spec). This targets the derivative *shape*, not "tests" by name — a test that encodes a novel invariant the code does not otherwise state does carry independent intent and should be specced. The end state for a derivative item is no entry; do not reach for `"=trivial"` or `"=self-doc"` to file it away.
9. Use a different model for test generation than the one that wrote the code, when possible.
10. When `liyi check` reports stale items, choose one of two paths:
    - **Direct re-inference** (preferred during interactive editing with few stale items): re-read the source, update `source_span` and `intent` in the sidecar, leave `"reviewed"` unset. Appropriate when you are the agent that just made the change, the number of stale items is small, and the changes are straightforward.
    - **Triage** (preferred for batch workflows, CI, or when many items are stale): assess each item — is the change cosmetic, semantic, or an intent violation? Write the assessment to `.liyi/triage.json` following the triage report schema. For cosmetic changes, run `liyi triage --apply` to auto-fix. For semantic changes, propose updated intent in `suggested_intent`. For intent violations, flag for human review. Prefer triage when stale items have `"reviewed": true` or `@liyi:intent` in source — these carry human-vouched intent that deserves explicit assessment, not silent re-inference.

    In either path, **never compute `source_hash` manually** and **never leave `"reviewed": true` on a spec whose `intent` you changed** (see rule 1). When `liyi check --fix` cannot auto-rehash a stale item (e.g., because it has `"reviewed": true`), update `source_span` and `intent`, **set `"reviewed"` to `false`**, and **delete** the `source_hash` field (or set it to `null`). The human will run `liyi check --fix` or `liyi approve` after reviewing the updated intent. This rule exists because agents consistently produce wrong hashes (trailing-newline differences, encoding mismatches) — the tool reads the actual bytes and is the only authority.
11. Before committing, run `liyi check`. If it reports coverage gaps (missing requirement specs, missing related edges), resolve **all** gaps in the same commit. When running as an agent, prefer `liyi check --prompt` for structured JSON output with per-gap resolution instructions. Do not commit with unresolved coverage gaps — CI will reject it.

### Key principles of the intent protocol

- **Adversarial, not confirmatory.** Find bugs, not confirm correctness.
- **Spec is the referee.** If the spec says one thing and the code does another, the test exposes the gap. The human decides who's right.
- **Model diversity.** Different model for tests than for code, when possible.
- **Never modify source code logic** during the protocol. Only create/update `.liyi.jsonc` files, `@liyi:note` blocks (in docs or doc comments), test files, and annotation comments (`@liyi:trivial`, `@liyi:ignore`, `@liyi:requirement`, `@liyi:related`). Annotation comments are metadata, not logic — adding them does not change program behavior.

### Resolving rule conflicts

When two normative instructions here appear to conflict, or a task's correct completion turns on an ambiguity these instructions do not resolve:

1. **Do not guess.** Name the conflicting rules (and any design-doc sections they reference).
2. **If a human is reachable:** ask once — state the candidate resolutions and your recommendation. Consolidate into a single question rather than a series.
3. **If no human is reachable** (batch, CI): take the most conservative reading, leave the affected specs `"reviewed": false`, complete the unambiguous parts, and report the unresolved conflict prominently.
4. **Do not over-apply this.** Where the standard delegates judgment to you — e.g. whether an item qualifies for `=self-doc` under the observer test — reason it out and state your reasoning so a human can veto. That is not a conflict.

### `.liyi.jsonc` Schema (v0.1)

Sidecar files must conform to the following JSON Schema. The top-level object has three required fields: `"version"` (must be `"0.1"`), `"source"` (repo-relative path to the source file), and `"specs"` (array of item or requirement entries). Each spec entry is either an **item spec** or a **requirement spec**, distinguished by the presence of `"item"` vs `"requirement"`.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://liyi.run/schema/0.1/liyi.schema.json",
  "title": "立意 sidecar spec file",
  "type": "object",
  "required": ["version", "source", "specs"],
  "additionalProperties": false,
  "properties": {
    "version": {
      "type": "string",
      "const": "0.1",
      "description": "Schema version. The linter rejects unknown versions."
    },
    "source": {
      "type": "string",
      "description": "Path to the source file, relative to the repository root."
    },
    "specs": {
      "type": "array",
      "items": {
        "oneOf": [
          { "$ref": "#/$defs/itemSpec" },
          { "$ref": "#/$defs/requirementSpec" }
        ]
      }
    }
  },
  "$defs": {
    "sourceSpan": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1 },
      "minItems": 2,
      "maxItems": 2,
      "description": "Closed interval of 1-indexed line numbers [start, end]. start must be <= end."
    },
    "sourceHash": {
      "type": "string",
      "pattern": "^sha256:[0-9a-f]+$",
      "description": "SHA-256 hex digest of the source lines in the span, prefixed with 'sha256:'."
    },
    "itemSpec": {
      "type": "object",
      "required": ["item", "intent", "source_span"],
      "additionalProperties": false,
      "properties": {
        "item": {
          "type": "string",
          "description": "Display name of the item (function, struct, macro, etc.). Not a unique key — identity is item + source_span."
        },
        "reviewed": {
          "type": "boolean",
          "default": false,
          "description": "Optional. Whether a human has reviewed and accepted this intent via sidecar approval. Defaults to false when absent. The linter also considers an item reviewed if @liyi:intent is present in source."
        },
        "intent": {
          "type": "string",
          "description": "Natural-language description of what the item SHOULD do, or the sentinel value '=doc' meaning the source docstring captures intent, '=trivial' meaning the item is trivial and needs no intent spec, or '=self-doc' meaning the item is self-documenting (intent intrinsic to its surface form) and needs no intent spec."
        },
        "source_span": { "$ref": "#/$defs/sourceSpan" },
        "tree_path": {
          "type": "string",
          "default": "",
          "description": "Optional. Structural AST path for tree-sitter-based span recovery (e.g., 'fn.add_money', 'impl.Money::fn.new'). When non-empty, the tool uses tree-sitter to locate the item by structural identity. When empty or absent, falls back to line-number-based span matching. Tool-managed — agents MAY write this but the tool overwrites with the canonical form."
        },
        "source_hash": {
          "$ref": "#/$defs/sourceHash",
          "description": "Tool-managed. SHA-256 hex digest of the source lines in the span. Computed by liyi check --fix — agents should not produce this."
        },
        "source_anchor": {
          "type": "string",
          "description": "Literal text of the first line of the span. Tool-managed — agents should not produce this."
        },
        "confidence": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "description": "Optional. Agent's self-assessed confidence in the inferred intent. May be removed after review."
        },
        "related": {
          "type": "object",
          "additionalProperties": {
            "oneOf": [
              { "$ref": "#/$defs/sourceHash" },
              { "type": "null" }
            ]
          },
          "description": "Optional. Maps requirement names to their source_hash at time of last review. Agents write null; the tool fills in hashes."
        },
        "_hints": {
          "type": "object",
          "description": "Transient inference aids emitted by liyi init for cold-start scenarios. LLM-readable, intentionally unstructured. Stripped by liyi check --fix after initial review. Tools MUST NOT rely on any specific shape."
        }
      }
    },
    "requirementSpec": {
      "type": "object",
      "required": ["requirement", "source_span"],
      "additionalProperties": false,
      "properties": {
        "requirement": {
          "type": "string",
          "description": "Name of the requirement. Unique per repository."
        },
        "source_span": { "$ref": "#/$defs/sourceSpan" },
        "tree_path": {
          "type": "string",
          "default": "",
          "description": "Optional. Structural AST path for tree-sitter-based span recovery. When non-empty, the tool uses tree-sitter to locate the requirement by structural identity. When empty or absent, falls back to line-number-based span matching. Tool-managed."
        },
        "source_hash": {
          "$ref": "#/$defs/sourceHash",
          "description": "Tool-managed. Computed by liyi check --fix."
        },
        "source_anchor": {
          "type": "string",
          "description": "Literal text of the first line of the span. Tool-managed."
        }
      }
    }
  }
}
```

### Triage Report Schema (v0.1)

When `liyi check` reports stale items, the agent assesses each and writes the result to `.liyi/triage.json`. The report must conform to the following JSON Schema (also available at `schema/triage.schema.json`):

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://liyi.run/schema/0.1/triage.schema.json",
  "title": "立意 triage report",
  "description": "Agent-produced assessment of stale intent specs, validated and consumed by liyi triage subcommands.",
  "type": "object",
  "required": ["version", "generated", "model", "summary", "items"],
  "additionalProperties": false,
  "properties": {
    "version": {
      "type": "string",
      "const": "0.1",
      "description": "Schema version. Must match the version understood by the liyi binary."
    },
    "generated": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp of when the triage report was produced."
    },
    "model": {
      "type": "string",
      "description": "Identifier of the model that produced the assessments (e.g. 'claude-sonnet-4-20260514')."
    },
    "root": {
      "type": "string",
      "default": ".",
      "description": "Repository root relative to the working directory. Usually '.'."
    },
    "summary": { "$ref": "#/$defs/summary" },
    "items": {
      "type": "array",
      "items": { "$ref": "#/$defs/triageItem" }
    }
  },
  "$defs": {
    "sourceSpan": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1 },
      "minItems": 2,
      "maxItems": 2,
      "description": "Closed interval of 1-indexed line numbers [start, end]. start must be <= end."
    },
    "verdict": {
      "type": "string",
      "enum": ["cosmetic", "semantic", "intent-violation", "unclear"],
      "description": "cosmetic: no behavioral change (rename, reformat). semantic: code evolved, intent is stale but code is correct. intent-violation: code contradicts declared intent. unclear: model cannot determine with sufficient confidence."
    },
    "action": {
      "type": "string",
      "enum": ["auto-fix", "update-intent", "fix-code-or-update-intent", "manual-review"],
      "description": "Recommended action. auto-fix for cosmetic, update-intent for semantic, fix-code-or-update-intent for intent-violation, manual-review for unclear."
    },
    "summary": {
      "type": "object",
      "required": ["total_stale", "cosmetic", "semantic", "intent_violation", "unassessed"],
      "additionalProperties": false,
      "properties": {
        "total_stale": {
          "type": "integer",
          "minimum": 0,
          "description": "Total number of stale items assessed."
        },
        "cosmetic": {
          "type": "integer",
          "minimum": 0,
          "description": "Items with verdict 'cosmetic'."
        },
        "semantic": {
          "type": "integer",
          "minimum": 0,
          "description": "Items with verdict 'semantic'."
        },
        "intent_violation": {
          "type": "integer",
          "minimum": 0,
          "description": "Items with verdict 'intent-violation'."
        },
        "unassessed": {
          "type": "integer",
          "minimum": 0,
          "description": "Stale items not yet assessed (should be 0 in a complete report)."
        },
        "impacted_transitively": {
          "type": "integer",
          "minimum": 0,
          "description": "Items affected transitively via the related graph."
        }
      }
    },
    "impactEntry": {
      "type": "object",
      "required": ["source", "item", "relationship", "impact_summary"],
      "additionalProperties": false,
      "properties": {
        "source": {
          "type": "string",
          "description": "Repo-relative path of the transitively affected source file."
        },
        "item": {
          "type": "string",
          "description": "Name of the transitively affected item."
        },
        "relationship": {
          "type": "string",
          "description": "The related edge that propagates the impact (e.g. 'related:multi-currency-addition')."
        },
        "impact_summary": {
          "type": "string",
          "description": "Why this item is affected (1–2 sentences)."
        }
      }
    },
    "triageItem": {
      "type": "object",
      "required": ["source", "item", "source_span", "verdict", "confidence", "change_summary", "invariant_summary", "reasoning", "action"],
      "additionalProperties": false,
      "properties": {
        "source": {
          "type": "string",
          "description": "Repo-relative source path."
        },
        "item": {
          "type": "string",
          "description": "Item name (matches the sidecar spec)."
        },
        "source_span": {
          "$ref": "#/$defs/sourceSpan",
          "description": "Current span from the sidecar."
        },
        "verdict": { "$ref": "#/$defs/verdict" },
        "confidence": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "description": "Model's self-assessed confidence in the verdict (0–1)."
        },
        "change_summary": {
          "type": "string",
          "description": "What changed in the code (1–2 sentences)."
        },
        "invariant_summary": {
          "type": "string",
          "description": "What stayed the same (1–2 sentences)."
        },
        "reasoning": {
          "type": "string",
          "description": "Why the verdict was assigned (2–3 sentences, citable in reviews)."
        },
        "action": { "$ref": "#/$defs/action" },
        "suggested_intent": {
          "type": ["string", "null"],
          "default": null,
          "description": "Proposed new intent text. Expected for 'semantic' verdicts; null otherwise."
        },
        "impact": {
          "type": "array",
          "items": { "$ref": "#/$defs/impactEntry" },
          "default": [],
          "description": "Transitively affected items via the related graph."
        }
      }
    }
  }
}
```
<!-- END liyi agent instructions -->