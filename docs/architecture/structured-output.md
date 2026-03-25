---
title: Structured Output Architecture
type: explanation
layer: 2
---

# Structured Output Architecture

How and why conductor's workflow engine uses structured output to bridge agent results with programmatic workflow logic. This document covers the design decisions, the integration pipeline, and the trade-offs involved. For the schema format specification, see [output schemas reference](../reference/output-schemas.md).

## The Problem

The base `CONDUCTOR_OUTPUT` protocol gives the engine two signals from each agent: markers (string flags) and context (a summary string). This is sufficient for control flow (`if`/`while` conditions) and context threading, but it forces all rich data -- reviewer findings, test coverage numbers, task plans -- into free-text prose.

When downstream consumers need that data (PR comment formatters, cross-agent aggregators, threshold checks, dashboards), they must parse free text heuristically. This is fragile and inconsistent across agents.

Structured output solves this by letting the engine understand agent results at a field level.

## Design Decisions

### Schemas belong to the workflow, not the agent

This is the foundational design choice. An output schema describes how the **workflow consumes** an agent's result, not a property of the agent itself.

A security reviewer might participate in one workflow that wants structured findings and another that wants only markers and context. Embedding the schema in the agent file would:

- Force every consumer to accept the same output shape
- Require duplicating the schema across every reviewer agent
- Mix concerns: agent files define who the agent is (role, prompt); schemas define what shape the output takes

Instead, schemas are standalone files referenced at the `call` site. The three-way separation:

| Artifact | Defines | Lives in |
|----------|---------|----------|
| Agent file (`.md`) | Who the agent is -- role, permissions, prompt | `.conductor/agents/` |
| Schema file (`.yaml`) | What shape the output takes | `.conductor/schemas/` |
| Workflow file (`.wf`) | Which agent uses which schema | `.conductor/workflows/` |

### Same delimiters, different payload

The structured output system reuses the `<<<CONDUCTOR_OUTPUT>>>` / `<<<END_CONDUCTOR_OUTPUT>>>` delimiter pair. When a schema is specified, only the JSON payload between the delimiters changes shape -- from `{"markers": [...], "context": "..."}` to the schema-defined structure.

This means:

- No new parsing protocol to implement
- Agents that forget the schema fall back to generic parsing gracefully
- The delimiter extraction code is shared between both paths

### Simplified type system over JSON Schema

The schema format uses a purpose-built type system (`string`, `number`, `boolean`, `enum(...)`, `array`, `object`) with optional `desc` and `examples` metadata. Standard JSON Schema was considered but rejected:

- JSON Schema is verbose and unfamiliar to workflow authors
- The simplified system covers practical cases (flat objects, arrays of objects, enums) without the learning curve
- `desc` and `examples` are injected directly into agent prompts -- JSON Schema's `description` field would work but `examples` would need post-processing

The simplified system can be revisited if it proves insufficient. The upgrade path is straightforward: add a `json_schema` field type that accepts raw JSON Schema.

### Prompt injection over API constraints

The Claude API's `response_format` parameter can guarantee structured output at the model level. However, conductor invokes `claude -p` via CLI, which does not expose `response_format`. The prompt injection approach -- generating schema-specific instructions and appending them to the agent prompt -- works reliably in practice.

If conductor moves to direct API calls (v2 daemon), `response_format` would be a natural optimization. The prompt instructions would remain as guidance while the API constraint guarantees compliance.

## Integration Pipeline

Structured output integrates with the workflow execution engine at six points:

```
Schema loading --> Prompt building --> Agent execution --> Output parsing --> Validation --> Marker derivation
```

### How it works, step by step

1. **Schema loading**: The engine resolves the `output` reference to a `.yaml` file using the schema search order (workflow-local, then shared). The file is parsed into an `OutputSchema` struct containing field definitions and optional marker rules.

2. **Prompt building** (`build_agent_prompt`): When a call has an `output` schema, the engine generates schema-specific output instructions (a JSON template with field descriptions as inline hints) instead of the generic `CONDUCTOR_OUTPUT` instruction. The instructions are appended after the agent body and prompt snippets.

3. **Agent execution**: The agent runs normally. From the agent's perspective, the only difference is the output instructions at the end of its prompt.

4. **Output parsing**: The engine extracts the last `<<<CONDUCTOR_OUTPUT>>>` block from the agent's output. Lenient normalization strips markdown code fences, trims whitespace, and removes trailing commas before parsing.

5. **Validation**: The parsed JSON is checked against the schema: required field presence, type correctness, enum membership, and recursive array item validation. Validation failures on successful runs are treated as retriable errors. On failed runs, the engine silently falls back to generic parsing.

6. **Marker derivation**: If the schema declares `markers` rules, those are evaluated. Otherwise, built-in defaults apply (e.g., `approved: false` produces `not_approved`). The `summary` field, if present, becomes `{{prior_context}}` for the next step.

### Data flow after parsing

| Destination | Content | Column |
|-------------|---------|--------|
| `workflow_run_steps.structured_output` | Full JSON | TEXT |
| `workflow_run_steps.markers_out` | Derived markers JSON | TEXT |
| `workflow_run_steps.context_out` | Summary field value | TEXT |
| `{{prior_output}}` variable | Full JSON (for downstream agents) | In-memory |
| `{{prior_context}}` variable | Summary field value | In-memory |

The `markers_out` and `context_out` columns are always populated for backward compatibility. Queries and UI that read those columns continue to work unchanged.

## Backward Compatibility

Structured output is additive. No breaking changes to existing workflows:

- Calls without `output` use the generic `CONDUCTOR_OUTPUT` format as before
- Agents that omit the output block entirely still contribute no markers (tolerated, not an error)
- `if`/`while` conditions work identically -- they check the `step_results` map regardless of whether markers were hand-emitted or auto-derived
- The `markers_out` and `context_out` columns are always populated, so existing DB queries work

The bridge between the two systems is marker derivation. When an agent emits structured output, the engine auto-derives markers from field values. Downstream conditions (`if review-security.has_critical_findings`) work without knowing whether the markers came from manual emission or schema derivation.

## Considered Alternatives

### Schema embedded in agent frontmatter

The initial design put `output_schema` in the agent `.md` file's YAML frontmatter. Each agent file would be self-contained.

**Why not chosen**: The schema describes how the workflow consumes output, not a property of the agent. As the number of reviewers grows, every new reviewer would need the same schema duplicated in its frontmatter. A change to the findings format requires editing every reviewer file.

### Inline field descriptions (pipe syntax)

An early design used `category: string | "OWASP category or general area"`.

**Why not chosen**: Does not scale. Adding `examples`, `default`, or future per-field metadata would require increasingly awkward inline syntax. The object form (`type` + `desc` + `examples`) is slightly more verbose but extensible without grammar changes.

### Full JSON Schema

Standard JSON Schema provides comprehensive features including `oneOf`/`anyOf`, regex patterns, and numeric ranges.

**Why not chosen**: Too verbose and unfamiliar to workflow authors. The simplified type system covers the practical cases without the learning curve. The upgrade path exists if needed.

### Response format API parameter

The Claude API's `response_format` parameter guarantees valid JSON structure at the model level.

**Why not chosen for now**: Conductor invokes `claude -p` via CLI, which does not expose this parameter. The prompt injection approach works reliably in practice. A future direct-API integration (v2 daemon) could use `response_format` as an optimization layer.

## Future Extensions

### Schema composition

Allow schemas to include other schemas via an `include` directive:

```yaml
include: review-findings
fields:
  suggested_changes:
    type: array
    items:
      file: string
      diff: string
```

Deferred until schema reuse patterns emerge in practice.

### Typed context threading

Instead of `{{prior_context}}` being a plain string, allow downstream agents to reference typed fields from the prior step's structured output via template expressions:

```
The plan contains {{prior_output.tasks.length}} tasks.
```

This requires a template expression language, adding complexity. Deferred until the simple `{{prior_output}}` JSON blob proves insufficient.

## Related Documents

- [Output Schemas Reference](../reference/output-schemas.md) -- schema file format, field types, validation rules, resolution order
- [Workflow Engine Architecture](workflow-engine.md) -- how structured output fits in the execution pipeline
- [Workflow DSL Syntax Reference](../reference/workflow-dsl.md) -- `output` option syntax on `call` and `parallel`
- [Agent Execution Architecture](agent-execution.md) -- prompt building, output capture, and the subprocess model
- [Database Schema Reference](../reference/database-schema.md) -- `workflow_run_steps.structured_output` column
- For the implementation, see `conductor-core/src/schema_config.rs` (~1.5K LoC) and `conductor-core/src/workflow.rs` (the `interpret_agent_output` function)
