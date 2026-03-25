---
title: How to Write a Conductor Workflow
type: how-to
layer: 2
---

# How to Write a Conductor Workflow

## Context / Trigger

Run this when you need to create a new automated multi-step workflow for conductor. Workflows define sequences of agent calls, conditionals, loops, parallel execution, and gates in `.wf` files.

**GRS**: 5 (Standard weight class). Multiple steps, references external systems (agent resolution, schema validation), but single-repo and reversible.

### Prerequisites

- conductor installed and built (`cargo install --path conductor-cli`)
- A registered repo with `.conductor/` directory structure
- Familiarity with the [workflow DSL syntax](../reference/workflow-dsl.md)
- Agents available via `.conductor/agents/` or [plugin directories](../reference/configuration.md#plugin-directory-resolution-order)

## Steps

### 1. Create the `.wf` file

Create a new file in `.conductor/workflows/<name>.wf`. Use a descriptive kebab-case name matching the workflow's action.

```
.conductor/workflows/review-pr.wf
```

**Expected outcome**: Empty `.wf` file at the correct path. Conductor discovers workflows by scanning this directory.

### 2. Define the `meta` block

Add the workflow wrapper and metadata. The `meta` block is optional but recommended.

```
workflow review-pr {
  meta {
    description = "Run review swarm and aggregate findings"
    trigger     = "manual"
    targets     = ["pr", "worktree"]
  }
}
```

| Field | Required | Values | Notes |
|-------|----------|--------|-------|
| `description` | No | Any string | Shown in `conductor workflow list` |
| `trigger` | No | `"manual"` (default) | `"pr"` and `"scheduled"` are reserved, not yet implemented |
| `targets` | No | `["worktree"]`, `["pr"]`, `["pr", "worktree"]` | Determines valid targeting flags on `workflow run` |

### 3. Declare inputs

If the workflow needs runtime parameters, add an `inputs` block. Each input is either `required` or has a `default`.

```
inputs {
  ticket_id required
  skip_tests default = "false"
}
```

Inputs are passed via `--input key=value` at runtime and available as `{{ticket_id}}` in agent prompts. [INVARIANT] Every `required` input must be supplied at run time or the workflow fails immediately.

### 4. Add agent calls

Define the workflow body as a sequence of nodes. The simplest node is `call`, which runs a single agent.

```
call plan { output = "task-plan" }
call implement { retries = 2 }
```

Key `call` options:

| Option | Purpose | Example |
|--------|---------|---------|
| `output` | Bind an output schema for structured results | `output = "review-findings"` |
| `retries` | Retry on failure | `retries = 2` |
| `on_fail` | Fallback agent after all retries exhausted | `on_fail = diagnose` |
| `with` | Append prompt snippets | `with = ["review-diff-scope"]` |
| `plugin_dirs` | Additional plugin directories for this step | `plugin_dirs = ["/usr/local/bsg/pattern-extractor"]` |

Agent names resolve via the [agent search order](../reference/workflow-dsl.md#agent-resolution). Use short names (not paths) for portability.

### 5. Add control flow

Use conditionals, loops, parallel blocks, and gates as needed.

**Parallel review swarm** (real example from `review-pr.wf`):

```
parallel {
  output    = "review-findings"
  with      = ["review-diff-scope", "off-diff-findings"]
  fail_fast = false
  call review-architecture    { retries = 1 }
  call review-security        { retries = 1 }
  call review-performance     { retries = 1 }
}

call review-aggregator { output = "review-aggregator" }
```

**Conditional execution** based on markers from a prior step:

```
if review-aggregator.has_critical_issues {
  call escalate
}
```

**Loop with iteration guard** (real example from `lint-fix.wf`):

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

**Human gate** for review checkpoints:

```
gate human_review {
  prompt     = "Review the assessment and provide clarifications."
  timeout    = "72h"
  on_timeout = fail
}
```

[INVARIANT] All `while`/`do-while` loops must specify `max_iterations`. The engine rejects loops without this guard.

### 6. Add output schemas for structured results

Create YAML schema files in `.conductor/schemas/` to get type-safe structured output from agents.

```yaml
# .conductor/schemas/review-findings.yaml
fields:
  findings:
    type: array
    items:
      file: string
      line: number
      severity: enum(critical, high, medium, low, info)
      message: string
  approved: boolean
  summary: string

markers:
  has_findings: "findings.length > 0"
  has_critical_findings: "findings[severity == critical].length > 0"
```

Reference schemas from `call` blocks via `output = "<schema-name>"`. See [output schemas reference](../reference/output-schemas.md) for the full specification.

### 7. Validate the workflow

Run validation before execution. This checks agent existence, snippet resolution, cycle detection, and semantic rules.

```bash
conductor workflow validate review-pr my-repo my-worktree
```

**Expected outcome**: `Workflow 'review-pr' is valid` or a list of specific errors (missing agents, unresolved snippets, circular references).

**Caveat**: `conductor workflow validate` does not detect missing output schemas. Schema errors (e.g., a `output = "nonexistent"` reference) only surface at runtime. Manually verify that all schema files referenced by `output` exist in `.conductor/schemas/` before running.

### 8. Test with dry-run mode

Run the workflow in dry-run mode first. Gates auto-approve, committing agents get a "DO NOT commit" prefix, and `{{dry_run}}` is `"true"`.

```bash
conductor workflow run review-pr my-repo my-worktree --dry-run
```

**Expected outcome**: Workflow completes without side effects. Check run details with `conductor workflow run-show <run-id>`.

### 9. Run in production

```bash
conductor workflow run review-pr my-repo my-worktree --input ticket_id=TICKET-123
```

Monitor with `conductor workflow runs my-repo` or the TUI dashboard.

## Anti-Patterns

**Anti-pattern: Hardcoded agent paths**

WRONG:
```
call ".conductor/agents/plan.md"
```

RIGHT:
```
call plan
```

**Why**: Hardcoded paths bypass the agent resolution search order, preventing worktree overrides and plugin directory injection.

**Anti-pattern: Missing output schemas**

WRONG:
```
call review-security
# downstream: parse review text manually
```

RIGHT:
```
call review-security { output = "review-findings" }
# downstream: use structured markers for conditions
```

**Why**: Without schemas, downstream conditions rely on generic markers that agents may forget to emit. Schemas enforce structure and auto-derive markers.

**Anti-pattern: No dry-run before production**

WRONG:
```bash
conductor workflow run ticket-to-pr my-repo my-wt --input ticket_id=T-1
```

RIGHT:
```bash
conductor workflow run ticket-to-pr my-repo my-wt --input ticket_id=T-1 --dry-run
# verify results, then:
conductor workflow run ticket-to-pr my-repo my-wt --input ticket_id=T-1
```

**Why**: Dry-run catches missing agents, broken snippets, and schema mismatches without creating commits, PRs, or burning API credits on full agent runs.

## Verification

1. **Validate passes**: `conductor workflow validate <name> <repo> <worktree>` exits with no errors
2. **Dry-run completes**: `conductor workflow run <name> --dry-run` reaches `completed` status
3. **Agents resolve**: Every `call` finds its agent via the search order (validation catches this)
4. **Schemas validate**: Agents produce output conforming to declared schemas
5. **Loops terminate**: `max_iterations` is set on every loop; `stuck_after` is set where appropriate

## Related Documents

- [Workflow DSL Syntax Reference](../reference/workflow-dsl.md) -- complete grammar and node type specifications
- [Output Schemas Reference](../reference/output-schemas.md) -- schema file format, field types, marker derivation
- [Workflow Engine Architecture](../architecture/workflow-engine.md) -- execution model, resumability, design trade-offs
- [CLI Commands Reference](../reference/cli-commands.md) -- `workflow run`, `validate`, `resume` command details
- [Prompt Snippets](../workflow/prompt-snippets.md) -- full prompt snippet specification
- For workflow templates, see [/usr/local/bsg/fsm-engine/](/usr/local/bsg/fsm-engine/) (fsm-engine owns `.wf` templates)
