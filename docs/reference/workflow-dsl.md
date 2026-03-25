---
title: Workflow DSL Syntax Reference
type: reference
layer: 3
---

# Workflow DSL Syntax Reference

Complete syntax specification for conductor's `.wf` workflow definition language. The DSL is parsed by a hand-written recursive descent parser (`conductor-core/src/workflow_dsl.rs`). Workflow files live in `.conductor/workflows/<name>.wf`.

## Grammar

```
workflow_file  := "workflow" IDENT "{" meta? inputs? node* "}"
meta           := "meta" "{" kv* "}"
inputs         := "inputs" "{" input_decl* "}"
input_decl     := IDENT ("required" | "default" "=" STRING)
node           := call | call_workflow | if | unless | while | do_while | do | parallel | gate | always
call           := "call" agent_ref ("{" kv* "}")?
call_workflow  := "call" "workflow" IDENT ("{" inputs? kv* "}")?
if             := "if" condition "{" kv* node* "}"
unless         := "unless" condition "{" kv* node* "}"
while          := "while" condition "{" kv* node* "}"
do_while       := "do" "{" kv* node* "}" "while" condition
do             := "do" "{" kv* node* "}"
parallel       := "parallel" "{" kv* call* "}"
gate           := "gate" gate_type "{" kv* "}"
always         := "always" "{" node* "}"
condition      := IDENT "." IDENT
gate_type      := "human_approval" | "human_review" | "pr_approval" | "pr_checks"
kv             := IDENT "=" value
value          := STRING | NUMBER | IDENT | array
array          := "[" (STRING ("," STRING)*)? "]"
agent_ref      := IDENT | STRING
```

## Lexical Rules

| Element | Pattern | Notes |
|---------|---------|-------|
| IDENT | `[a-zA-Z0-9_-]+` | Allows hyphens so agent names like `push-and-pr` work naturally |
| STRING | `"..."` | Double-quoted, no escape sequences |
| NUMBER | `[0-9]+` | Integer only |
| Comments | `#` to end of line | Supported everywhere |
| Whitespace | Spaces, tabs, newlines | Insignificant between tokens |

## File Structure

Every `.wf` file contains exactly one `workflow` block:

```
workflow <name> {
  meta { ... }       # optional
  inputs { ... }     # optional
  <nodes...>         # one or more execution nodes
}
```

## Meta Block

Optional metadata about the workflow.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `description` | STRING | No | Human-readable purpose |
| `trigger` | STRING | No | `"manual"` (default), `"pr"`, or `"scheduled"`. Only `"manual"` is implemented; others are reserved |
| `targets` | array | No | Target types: `["worktree"]`, `["pr"]`, `["pr", "worktree"]` |

```
meta {
  description = "Full development cycle"
  trigger     = "manual"
  targets     = ["worktree"]
}
```

## Inputs Block

Declares workflow parameters. Each input is either `required` or has a `default` value.

```
inputs {
  ticket_id  required
  skip_tests default = "false"
}
```

Inputs are available in agent prompts as `{{ticket_id}}`, `{{skip_tests}}`, etc. Passed at runtime via `--input key=value`.

## Node Types

### `call`

Runs a single agent to completion.

```
call plan
call plan { output = "task-plan" }
call implement { retries = 2  on_fail = diagnose }
call review { with = ["review-diff-scope", "rust-conventions"] }
call pe-discover { plugin_dirs = ["/usr/local/bsg/pattern-extractor"] }
call review-aggregator { output = "review-aggregator" as = "my-bot" }
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `retries` | NUMBER | 0 | Retry count on failure |
| `on_fail` | IDENT | none | Fallback agent when all retries exhausted |
| `output` | STRING | none | Output schema name (from `.conductor/schemas/`) |
| `with` | STRING or array | `[]` | Prompt snippet names (from `.conductor/prompts/`) |
| `plugin_dirs` | array | `[]` | Additional plugin directories appended to repo-level dirs |
| `as` | STRING | none | Named GitHub App bot identity for this step |

**Agent reference**: A bare identifier (e.g., `plan`) is resolved via the search order. A quoted string (e.g., `".claude/agents/plan.md"`) is treated as an explicit path relative to the repo root.

**Note**: There is no per-call timeout field. Step timeout is controlled by the `--step-timeout-secs` CLI flag (default: 604800s / 1 week). Gate timeout is separate and specified via the `timeout` field on the gate node.

### `call workflow`

Invokes a sub-workflow as a single opaque step.

```
call workflow lint-fix
call workflow test-coverage { inputs { pr_url = "{{pr_url}}" } }
call workflow lint-fix { retries = 1  on_fail = notify-lint-failure }
call workflow review-pr { as = "review-bot" }
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `inputs` | block | none | Input values for the sub-workflow; supports `{{variable}}` substitution |
| `retries` | NUMBER | 0 | Retry count (re-runs sub-workflow from beginning) |
| `on_fail` | IDENT | none | Fallback agent when all retries exhausted |
| `as` | STRING | none | Named GitHub App bot identity inherited by child calls |

The sub-workflow's final `CONDUCTOR_OUTPUT` (markers + context) becomes the output of this step. Downstream conditions can reference sub-workflow markers.

### `if` / `unless`

Conditional execution based on markers from a prior step.

```
if review-aggregator.has_critical_issues {
  call escalate
}

unless build.has_errors {
  call deploy
}
```

**Condition syntax**: `<step>.<marker>`. The engine checks whether the named step's most recent `CONDUCTOR_OUTPUT` includes the marker in its `markers` array. `unless` executes when the marker is absent.

Conditions can reference markers from any prior step, including sub-workflows and parallel blocks.

### `while`

Loop that checks the condition before each iteration.

```
while review.has_review_issues {
  max_iterations = 5
  stuck_after    = 3
  on_max_iter    = fail
  call address-reviews
  call review
}
```

| Option | Type | Required | Default | Description |
|--------|------|----------|---------|-------------|
| `max_iterations` | NUMBER | Yes | -- | Hard iteration cap |
| `stuck_after` | NUMBER | No | none | Fail if marker set is identical for N consecutive iterations |
| `on_max_iter` | IDENT | No | `fail` | `fail` or `continue` when cap is reached |

### `do {} while`

Loop that executes the body at least once, checking the condition after each iteration.

```
do {
  max_iterations = 3
  on_max_iter    = fail
  call analyze-lint
  if analyze-lint.has_lint_errors {
    call lint-fix-impl
  }
} while analyze-lint.has_lint_errors
```

Supports the same options as `while`: `max_iterations`, `stuck_after`, `on_max_iter`.

### `do`

Sequential grouping block with inherited options. Runs body nodes in order.

```
do {
  output = "review-summary"
  with   = ["review-diff-scope"]
  call review-security
  call review-style
}
```

| Option | Type | Description |
|--------|------|-------------|
| `output` | STRING | Output schema applied to every `call` in the block (call-level overrides) |
| `with` | array | Prompt snippets prepended to every call's snippet list |

Unlike `parallel`, `do` runs steps sequentially and its body may contain any node type.

### `parallel`

Runs multiple agents concurrently. Body may only contain `call` nodes.

```
parallel {
  output    = "review-findings"
  with      = ["review-diff-scope"]
  fail_fast = false
  call review-architecture
  call review-security { with = ["extra-rules"] }
  call review-performance
}
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `output` | STRING | none | Output schema applied to all calls (call-level overrides) |
| `with` | array | `[]` | Prompt snippets applied to all calls; per-call `with` is appended after |
| `fail_fast` | IDENT | `true` | Cancel remaining agents when one fails |
| `min_success` | NUMBER | all | Minimum agents that must succeed |

Per-call options (`retries`, `output`, `with`, `plugin_dirs`) are specified inline on each `call` within the block. Block-level `with` snippets are prepended to per-call snippets. Markers from all completed agents are merged for downstream conditions.

### `gate`

Pauses execution until an external condition is met. The workflow run enters `waiting` status.

**Human gates** require action through conductor (CLI, TUI, or web):

```
gate human_approval {
  prompt     = "Review PLAN.md before implementation begins."
  timeout    = "24h"
  on_timeout = fail
}

gate human_review {
  prompt     = "Review agent findings. Add notes if needed."
  timeout    = "48h"
  on_timeout = continue
}
```

`human_review` accepts written feedback via `conductor workflow gate-feedback <run_id> "<text>"`. The `{{gate_feedback}}` template variable is injected into the prompt of the next step after the gate is approved -- it is only available in that immediately following step, not globally.

**Automated gates** poll an external signal:

```
gate pr_approval { min_approvals = 1  timeout = "72h"  on_timeout = fail }
gate pr_checks   { timeout = "2h"  on_timeout = fail }
gate pr_approval { mode = "review_decision"  timeout = "72h"  on_timeout = fail }
```

| Option | Applies To | Type | Default | Description |
|--------|-----------|------|---------|-------------|
| `prompt` | human gates | STRING | none | Message shown to the approver |
| `min_approvals` | `pr_approval` | NUMBER | 1 | GitHub approvals required |
| `mode` | `pr_approval` | STRING | `"min_approvals"` | `"min_approvals"` or `"review_decision"` |
| `timeout` | all | STRING | required | Duration: `"2h"`, `"24h"`, `"72h"` |
| `on_timeout` | all | IDENT | `fail` | `fail` or `continue` when timeout expires |
| `as` | all | STRING | none | Named bot identity for `gh` calls |

**Gate types**: `human_approval`, `human_review`, `pr_approval`, `pr_checks`.

### `always`

Runs after the main body regardless of success or failure.

```
always {
  call notify-result
}
```

The `{{workflow_status}}` variable is available inside `always` blocks (`"completed"` or `"failed"`). Failures in `always` steps are logged but do not change the workflow's terminal status.

## Variable Substitution

Agent prompts and prompt snippets support `{{variable}}` substitution. Variables are replaced with their string values; unrecognized variables pass through unchanged.

### Built-in Variables

| Variable | Source | Description |
|----------|--------|-------------|
| `{{prior_context}}` | Previous step | Context string from the immediately preceding step's `CONDUCTOR_OUTPUT` |
| `{{prior_contexts}}` | All prior steps | JSON array of all context entries: `[{"step": "...", "iteration": N, "context": "..."}]` |
| `{{prior_output}}` | Previous step | Raw JSON from the last step's structured output (when schema was used) |
| `{{gate_feedback}}` | `human_review` gate | Feedback text from the most recent gate |
| `{{dry_run}}` | Runtime flag | `"true"` or `"false"` reflecting `--dry-run` flag |
| `{{workflow_status}}` | Engine | Available in `always` blocks: `"completed"` or `"failed"` |
| `{{failed_step}}` | Engine | Available in `on_fail` agent: name of the failed step |
| `{{failure_reason}}` | Engine | Available in `on_fail` agent: error message from the failed step |
| `{{retry_count}}` | Engine | Available in `on_fail` agent: number of retries attempted |

All workflow `inputs` are also available as `{{input_name}}`.

## Structured Output (CONDUCTOR_OUTPUT)

Every agent prompt is automatically appended with instructions to emit a `CONDUCTOR_OUTPUT` block:

```
<<<CONDUCTOR_OUTPUT>>>
{"markers": ["has_review_issues"], "context": "Found 3 issues in auth module"}
<<<END_CONDUCTOR_OUTPUT>>>
```

| Field | Type | Description |
|-------|------|-------------|
| `markers` | string array | Signal flags consumed by `if`/`while` conditions |
| `context` | string | Summary passed to the next step as `{{prior_context}}` |

The engine finds the **last** occurrence of the delimiters (avoiding false positives in code blocks). Agents that omit the block are treated as emitting no markers and no context -- this is not an error.

When an `output` schema is specified, the engine uses schema-specific parsing instructions instead of the generic `CONDUCTOR_OUTPUT` format. See [output schemas reference](output-schemas.md) for schema details.

## Prompt Snippets (`with`)

Reusable `.md` instruction blocks appended to agent prompts at execution time.

### Resolution order

Short names (no `/` or `\`) resolve in this order (first match wins):

| Priority | Path |
|----------|------|
| 1 | `.conductor/workflows/<workflow>/prompts/<name>.md` |
| 2 | `.conductor/prompts/<name>.md` |

Each priority checks the worktree path first, then the repo path.

Values containing `/` or `\` are treated as explicit paths relative to the repo root. Absolute paths and repo-escaping paths are rejected.

### Composition order

For each agent invocation, the prompt is assembled:

1. Agent `.md` body (with `{{variable}}` substitution)
2. `with` snippets (each also substituted), separated by `\n\n`
3. Schema output instructions or generic `CONDUCTOR_OUTPUT` instruction

Block-level `with` (from `parallel` or `do`) is prepended; per-call `with` is appended after.

For the full specification, see [prompt-snippets.md](../workflow/prompt-snippets.md).

## Agent Resolution

When `call` receives a bare identifier, the engine resolves it by checking these locations in order (first match wins):

| Priority | Path | Scope |
|----------|------|-------|
| 1 | `.conductor/workflows/<workflow>/agents/<name>.md` | Workflow-local override |
| 2 | `.conductor/agents/<name>.md` | Shared conductor agents |

Each priority is checked in the worktree path first, then the repo path. Quoted strings bypass this search order and resolve as explicit paths relative to the repo root.

For the planned extended resolution features, see [agent-path-resolution.md](../workflow/agent-path-resolution.md).

## Plugin Directory Injection

Two levels of plugin directory configuration:

| Level | Set Via | Scope |
|-------|---------|-------|
| Repo-level | `conductor repo set-plugin-dirs` | All steps in all workflows for this repo |
| Per-step | `plugin_dirs = [...]` in `call` block | Appended to repo-level dirs for this step only |

Resolution order: repo-level dirs + `CONDUCTOR_PLUGIN_DIRS` env + per-step dirs (appended). Plugin dirs are passed as `--plugin-dir` arguments to the Claude session.

## Workflow Composition

Workflows invoke other workflows via `call workflow <name>`. This is **shallow composition** -- the sub-workflow runs to completion as a single opaque step.

### Constraints

| Constraint | Value | Enforcement |
|------------|-------|-------------|
| Maximum nesting depth | 5 | Runtime check |
| Circular references | Forbidden | Static reachability analysis at validate + run time |
| Dynamic dispatch | Not supported | Workflow name must be a static identifier, not `{{variable}}` |
| Shared state | None | Child starts with fresh context; only final output crosses the boundary |

### Input and output

Sub-workflows have their own `inputs` block. The parent must supply all `required` inputs. Input values support `{{variable}}` substitution from the parent's scope.

The sub-workflow's final step's `CONDUCTOR_OUTPUT` becomes the output of the `call workflow` step in the parent.

## Complete Example

```
workflow ticket-to-pr {
  meta {
    description = "Full development cycle"
    trigger     = "manual"
    targets     = ["worktree"]
  }

  inputs {
    ticket_id required
  }

  call plan { output = "task-plan" }

  call implement { retries = 2 }

  call workflow lint-fix

  call push-and-pr

  parallel {
    output    = "review-findings"
    with      = ["review-diff-scope"]
    fail_fast = false
    call review-architecture
    call review-security
    call review-performance
  }

  call review-aggregator { output = "review-aggregator" }

  while review-aggregator.has_review_issues {
    max_iterations = 3
    on_max_iter    = fail
    call address-reviews

    parallel {
      output    = "review-findings"
      with      = ["review-diff-scope"]
      fail_fast = false
      call review-architecture
      call review-security
      call review-performance
    }

    call review-aggregator { output = "review-aggregator" }
  }

  always {
    call notify-result
  }
}
```

## Related Documents

- [Workflow Engine Architecture](../architecture/workflow-engine.md) -- how the engine executes these constructs
- [Output Schemas Reference](output-schemas.md) -- schema format, field types, validation rules
- [Prompt Snippets](../workflow/prompt-snippets.md) -- full prompt snippet specification
- [Agent Path Resolution](../workflow/agent-path-resolution.md) -- extended agent resolution design
- [How to Write a Workflow](../how-to/write-workflow.md) -- step-by-step guide for creating workflows
- [Database Schema Reference](database-schema.md) -- `workflow_runs` and `workflow_run_steps` tables
- For the DSL parser implementation, see `conductor-core/src/workflow_dsl.rs`
