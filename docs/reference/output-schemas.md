---
title: Output Schemas Reference
type: reference
layer: 3
---

# Output Schemas Reference

Specification for conductor's structured output schema system. Schemas define the JSON shape that agents emit inside `CONDUCTOR_OUTPUT` blocks. The engine uses schemas for prompt generation, output validation, marker derivation, and context threading.

For the design rationale behind this system, see [structured output architecture](../architecture/structured-output.md).

## CONDUCTOR_OUTPUT Block Format

Every agent prompt is appended with instructions to emit a delimited output block:

```
<<<CONDUCTOR_OUTPUT>>>
{"markers": ["has_review_issues"], "context": "Found 3 issues in auth module"}
<<<END_CONDUCTOR_OUTPUT>>>
```

### Generic format (no schema)

When no `output` schema is specified on a `call`, the engine expects:

| Field | Type | Description |
|-------|------|-------------|
| `markers` | string array | Signal flags for `if`/`while` conditions |
| `context` | string | Summary passed to next step as `{{prior_context}}` |

Both fields default to empty if the agent omits the block entirely (not an error).

### Schema format

When `output = "<schema-name>"` is specified, the JSON between the delimiters must conform to the referenced schema. The `markers` and `context` fields are derived automatically (see [Marker Derivation Rules](#marker-derivation-rules) and [Context Extraction](#context-extraction)).

### Parse rules

The engine extracts the **last** occurrence of the delimiters in the agent's output. This avoids false positives when agents include example CONDUCTOR_OUTPUT blocks in code fences.

**Lenient normalization** applied before validation:

| Normalization | Purpose |
|---------------|---------|
| Strip markdown code fences (```` ```json ... ``` ````) | Agents sometimes wrap JSON in fences |
| Trim whitespace | Leading/trailing whitespace tolerance |
| Remove trailing commas before `}` or `]` | Common LLM output artifact |

Agents that omit the block entirely contribute no markers and no context. This is tolerated, not treated as an error.

## Schema File Format

Schemas are YAML files in `.conductor/schemas/<name>.yaml`. A schema has two top-level keys: `fields` (required) and `markers` (optional).

```yaml
# .conductor/schemas/review-findings.yaml
fields:
  findings:
    type: array
    items:
      file: string
      line: number
      severity: enum(critical, high, medium, low, info)
      category:
        type: string
        desc: "OWASP category or general area"
        examples: ["injection", "auth", "config", "cryptography"]
      message: string
      suggestion?:
        type: string
        desc: "Suggested fix or remediation"
  approved: boolean
  summary: string

markers:
  has_findings: "findings.length > 0"
  has_critical_findings: "findings[severity == critical].length > 0"
  not_approved: "approved == false"
```

## Field Types

| Type | Description | JSON Equivalent | Example |
|------|-------------|-----------------|---------|
| `string` | Free-text string | `"..."` | `file: string` |
| `number` | Integer or float | `42`, `3.14` | `line: number` |
| `boolean` | True/false | `true`, `false` | `approved: boolean` |
| `enum(a, b, c)` | One of the listed values | `"a"` | `severity: enum(critical, high, medium, low, info)` |
| `array` | List of items with `items` sub-schema | `[...]` | See below |
| `object` | Nested structure with named fields | `{...}` | See below |

## Field Definition Forms

Fields can use **short form** (type only) or **object form** (type plus metadata).

### Short form

```yaml
file: string
line: number
approved: boolean
```

### Object form

```yaml
category:
  type: string
  desc: "OWASP category or general area"
  examples: ["injection", "auth", "config", "cryptography"]
```

| Property | Required | Description |
|----------|----------|-------------|
| `type` | Yes | One of the supported types |
| `desc` | No | Description included in the agent prompt as guidance |
| `examples` | No | Sample values included in the prompt for output consistency |
| `items` | No | Sub-field definitions for `array` type |
| `fields` | No | Sub-field definitions for `object` type |

Both forms can be mixed freely within a schema. Fields are **required by default**. Append `?` to the field name to mark it optional:

```yaml
suggestion?:
  type: string
  desc: "Suggested fix or remediation"
```

## Schema Resolution

Schema names specified in `output = "<name>"` are resolved by checking locations in order:

| Priority | Path | Scope |
|----------|------|-------|
| 1 | `.conductor/workflows/<workflow>/schemas/<name>.yaml` | Workflow-local override |
| 2 | `.conductor/schemas/<name>.yaml` | Shared schemas |

Each priority checks the worktree path first, then the repo path.

Quoted strings containing `/` or `\` are treated as explicit paths relative to the repo root:

```
call review-security { output = "./custom/schemas/my-review.yaml" }
```

Absolute paths and paths that escape the repo root are rejected.

## Referencing Schemas from Workflows

The `output` option on `call` binds a schema to an agent invocation:

```
call review-security { output = "review-findings" }
```

For `parallel` and `do` blocks, `output` applies to all contained calls (individual calls can override):

```
parallel {
  output = "review-findings"
  call review-security
  call review-style
  call lint-check { output = "lint-results" }   # overrides block-level
}
```

When no `output` is specified, the generic `CONDUCTOR_OUTPUT` format applies.

## Validation

When parsing schema-validated output, the engine checks:

| Check | Behavior on Failure |
|-------|---------------------|
| JSON syntax | Step fails; retry if `retries` configured |
| Required fields present | Step fails; error names the missing field |
| Type correctness | Step fails; error names the field and expected type |
| Enum membership | Step fails; error lists allowed values |
| Array item validation | Step fails; error includes item index and field path |

Validation failures on successful agent runs are treated as retriable errors. On failed agent runs, the engine falls back to generic `CONDUCTOR_OUTPUT` parsing.

## Marker Derivation Rules

### Custom markers (explicit)

When the `markers` section is present in the schema, only the declared rules apply:

```yaml
markers:
  has_findings: "findings.length > 0"
  has_critical_findings: "findings[severity == critical].length > 0"
  not_approved: "approved == false"
```

The expression language supports: field access, equality checks, array length, and array filtering. It is deliberately minimal -- not a general-purpose evaluator.

### Default markers (implicit)

When `markers` is omitted, the engine applies built-in defaults based on field names and types:

| Schema Field | Derived Marker |
|-------------|----------------|
| `approved: false` | `not_approved` |
| `findings` array non-empty | `has_findings` |
| Any finding with `severity: critical` | `has_critical_findings` |
| Any finding with `severity: high` | `has_high_findings` |

### Context Extraction

The `summary` field (if present in the schema) is used as the `context` value for `{{prior_context}}` threading. The full structured JSON is available as `{{prior_output}}`.

## Storage

Structured output is stored in `workflow_run_steps.structured_output` (TEXT column, full JSON). The derived `markers_out` and `context_out` columns are also populated for backward compatibility.

## Built-In Schema Examples

### review-findings

Used by reviewer agents. Ships with conductor.

```yaml
fields:
  findings:
    type: array
    items:
      file: string
      line: number
      severity: enum(critical, high, medium, low, info)
      category:
        type: string
        desc: "OWASP category or general area"
      message: string
      suggestion?:
        type: string
        desc: "Suggested fix or remediation"
  approved: boolean
  summary: string
```

### task-plan

Used by planning agents.

```yaml
fields:
  tasks:
    type: array
    items:
      id:
        type: string
        desc: "Short identifier for cross-referencing"
      description: string
      files:
        type: array
        desc: "Files this task will create or modify"
      dependencies:
        type: array
        desc: "IDs of tasks that must complete first"
      complexity: enum(low, medium, high)
  estimated_steps: number
  summary: string

markers:
  has_complex_tasks: "tasks[complexity == high].length > 0"
```

### review-aggregator

Used by review aggregation agents to consolidate multiple review outputs.

```yaml
fields:
  reviews:
    type: array
    items:
      agent: string
      approved: boolean
      findings_count: number
      summary: string
  overall_approved: boolean
  blockers:
    type: array
    items:
      agent: string
      finding: string
  summary: string

markers:
  has_blockers: "blockers.length > 0"
  all_approved: "overall_approved == true"
  not_approved: "overall_approved == false"
```

## Related Documents

- [Structured Output Architecture](../architecture/structured-output.md) -- design rationale, considered alternatives, future extensions
- [Workflow DSL Syntax Reference](workflow-dsl.md) -- `output` option syntax on `call` and `parallel` blocks
- [Workflow Engine Architecture](../architecture/workflow-engine.md) -- how the engine processes structured output in the execution pipeline
- [Database Schema Reference](database-schema.md) -- `workflow_run_steps.structured_output` column
- For the implementation, see `conductor-core/src/schema_config.rs` (~1.5K LoC)
