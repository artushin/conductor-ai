---
title: Workflow Engine Architecture
type: explanation
layer: 2
---

# Workflow Engine Architecture

How conductor's workflow engine loads, parses, validates, and executes `.wf` workflow definitions. This document explains the execution model, data flow, and design decisions behind the engine. For the syntax of `.wf` files, see [workflow DSL reference](../reference/workflow-dsl.md).

## Engine Overview

The workflow engine transforms a `.wf` file into an execution trace stored in the database. The pipeline has four stages:

```
.wf file  -->  Parse (AST)  -->  Validate  -->  Snapshot  -->  Execute
                workflow_dsl.rs              definition_snapshot    workflow.rs
```

1. **Parse**: The recursive descent parser (`workflow_dsl.rs`, ~4K LoC) reads the `.wf` file and produces a `WorkflowDef` AST -- a tree of `WorkflowNode` variants.
2. **Validate**: Semantic checks verify agent existence, input completeness, cycle-free composition, and snippet resolution. Runs at `conductor workflow validate` and again before execution.
3. **Snapshot**: The `WorkflowDef` is serialized to JSON and stored in `workflow_runs.definition_snapshot`. All subsequent execution uses the snapshot, never the on-disk file.
4. **Execute**: The engine walks the node tree, spawning agents, evaluating conditions, managing loops, and recording results in `workflow_run_steps`.

**Note**: `execute_workflow` creates a parent `agent_run` record as a bookkeeping shell (no `tmux_window`). This record is not a real agent session -- it exists solely to group child step runs under a single parent ID.

## Parsing and AST

The parser is hand-written (not generated) for three reasons: the grammar is small, error messages can be precise, and there are no external parser dependencies. It operates on a token stream produced by a simple lexer that handles identifiers, strings, numbers, and punctuation.

The AST is a tree of `WorkflowNode` enum variants:

| Variant | Children | Description |
|---------|----------|-------------|
| `Call` | None | Single agent invocation |
| `CallWorkflow` | None | Sub-workflow invocation |
| `If` / `Unless` | `Vec<WorkflowNode>` | Conditional body |
| `While` / `DoWhile` | `Vec<WorkflowNode>` | Loop body |
| `Do` | `Vec<WorkflowNode>` | Sequential grouping with inherited options |
| `Parallel` | `Vec<AgentRef>` (calls only) | Concurrent agent execution |
| `Gate` | None | Pause point |
| `Always` | `Vec<WorkflowNode>` | Cleanup body |

The `WorkflowDef` separates `always` nodes from the main `body` at parse time. This simplifies the execution loop -- the engine runs `body` first, then `always` regardless of outcome.

## Validation

Validation runs as a separate pass over the parsed AST before execution begins.

| Check | When | What |
|-------|------|------|
| Agent existence | Validate + run | Every `call` agent can be resolved via the search order |
| Input completeness | Validate + run | Required inputs have values; no undefined inputs referenced |
| Circular references | Validate + run | Static reachability analysis on `call workflow` graph |
| Snippet resolution | Validate + run | Every `with` reference resolves to a `.md` file |
| Depth limit | Run | Sub-workflow nesting does not exceed 5 levels |
| Semantic rules | Validate | `while`/`do-while` require `max_iterations`; gates require `timeout` |

Circular reference detection is static -- it flags any cycle in the `call workflow` graph regardless of whether the cycle is reachable at runtime (e.g., a cycle inside a conditional that might never be true). This is deliberate: the engine cannot prove at parse time that a condition will never be true, and static detection fails fast before any work begins.

**Note**: Pre-flight agent validation does NOT check per-step `plugin_dirs`. Agents referenced only via per-step plugin directories must also be symlinked into `.conductor/agents/` to pass validation, or validation will fail even though the agent would be resolvable at runtime.

## Execution Model

### Snapshot immutability

When a run starts, the parsed `WorkflowDef` is serialized to JSON and stored in `workflow_runs.definition_snapshot`. The engine always executes from the snapshot, never re-parsing the `.wf` file. This guarantees that editing a workflow mid-run does not change in-flight behavior.

### Execution state

The engine maintains an `ExecutionState` struct that carries all runtime context:

| Field | Purpose |
|-------|---------|
| `inputs` | Workflow input parameters + injected variables (`dry_run`, `workflow_status`, etc.) |
| `contexts` | Accumulated `ContextEntry` array (step name, iteration, context string) |
| `step_results` | Per-step marker sets for condition evaluation |
| `position` | Monotonically increasing step counter |
| `all_succeeded` | Tracks whether any step has failed |
| `plugin_dirs` | Repo-level plugin directories |
| `block_output` / `block_with` | Inherited options from enclosing `do` or `parallel` blocks |
| `resume_ctx` | Pre-loaded completed step data for resumable execution |

### Node dispatch

The `execute_nodes` function iterates the node list, dispatching each node to its type-specific handler:

```
execute_nodes(nodes) {
    for node in nodes:
        if !all_succeeded && fail_fast: break
        match node:
            Call        -> execute_call
            CallWorkflow -> execute_call_workflow
            If          -> execute_if
            While       -> execute_while
            DoWhile     -> execute_do_while
            Do          -> execute_do
            Parallel    -> execute_parallel
            Gate        -> execute_gate
            Always      -> execute_nodes(body)
}
```

The top-level `run_workflow_engine` calls `execute_nodes` on the body, then unconditionally calls `execute_nodes` on the `always` block.

## Step Execution Lifecycle

### Agent invocation (`call`)

Each `call` step follows this sequence:

1. **Check resume**: If the step was already completed (from a prior partial run), skip it and restore its results
2. **Load agent definition**: Resolve the agent `.md` file via the search order
3. **Load schema**: If `output` is specified, load the schema from `.conductor/schemas/`
4. **Load snippets**: Resolve and concatenate `with` prompt snippets
5. **Build prompt**: Assemble the final prompt (agent body + snippets + output instructions) with variable substitution
6. **Insert step record**: Create a `workflow_run_steps` row with `running` status
7. **Spawn agent**: Create an `agent_run` record and spawn a tmux window running `conductor agent run`
8. **Poll for completion**: Check the agent run status every 5 seconds until it reaches a terminal state
9. **Parse output**: Extract `CONDUCTOR_OUTPUT` markers and context (or schema-validated output)
10. **Record results**: Update the step record with markers, context, status, cost, and duration

### How agents run

Agents do not run inside the engine process. The engine spawns a tmux window running `conductor agent run --run-id <id>`, which starts a new Claude session with the assembled prompt. The engine then polls the database for the agent run's status, sleeping 5 seconds between checks.

This subprocess model means:

- The engine process can be killed and restarted -- it resumes by polling existing runs
- Multiple agents (in parallel blocks) can run simultaneously in separate tmux windows
- No streaming or real-time communication between engine and agent -- only the final result matters

### Plugin directory injection

When spawning an agent, the engine builds the `--plugin-dir` argument list:

1. Start with repo-level `plugin_dirs` (from `conductor repo set-plugin-dirs`)
2. Append dirs from `CONDUCTOR_PLUGIN_DIRS` environment variable
3. Append per-step `plugin_dirs` (if specified on the `call` node)

This allows per-step specialization: a PE discovery step can load pattern-extractor plugins while review steps use only the shared agent-architecture plugins.

### Background execution

`conductor workflow run --background` forks a detached child process via `fork`/`setsid`, prints the run ID to stdout, and exits immediately. The child drives the workflow to completion independently. This is used by chief's dispatch to avoid blocking the spawning Claude session on long-running workflows.

### Native agent mode context prompt

In native agent mode (`--agent`), `build_context_prompt()` does **not** substitute `{{template_vars}}` in the agent body. When `fsm_path` is in the variable map, the context prompt injects a resolved `## Mandatory First Action` section at the top of the user message -- before the variable table, before snippets, before prior context.

The task reinforcement wording is: "starting from its stated first action, then proceeding through each step in order. Do not skip ahead based on prior context." An earlier version used "execute all required actions (CLI commands, file writes, etc.)" which primed agents toward implementation behavior rather than FSM-driven step sequencing.

## Condition Evaluation

Conditions (`if`, `unless`, `while`, `do-while`) evaluate `<step>.<marker>` expressions by checking the named step's most recent marker set.

The engine maintains a `step_results` map keyed by step name. After each step completes, its markers are stored in this map. For parallel blocks, all completed agents' markers are merged into the map under their respective step keys.

Markers are intentionally structured data (string arrays), not free-text scanning. An earlier design used `contains()` on agent output text, but an agent saying "there are no has_review_issues" would evaluate to true. Structured markers eliminate this class of bugs.

## Context Threading

Each step receives context from prior steps through variable substitution:

| Variable | Content |
|----------|---------|
| `{{prior_context}}` | Context string from the immediately preceding step |
| `{{prior_contexts}}` | JSON array of all accumulated context entries |
| `{{prior_output}}` | Raw JSON from the last step's structured output (when schema was used) |

Context entries accumulate in a `Vec<ContextEntry>`, where each entry records the step name, loop iteration, and context string. Inside `while` loops, entries from all iterations are included, allowing agents to detect repeated failures on the same issue.

The substitution mechanism is simple string replacement: `{{key}}` in the prompt is replaced with the variable's value. Unrecognized variables pass through unchanged (this is by design -- agent prompts may contain literal `{{` sequences for other template systems).

## Parallel Execution

Parallel blocks spawn all agents simultaneously in separate tmux windows. The engine then enters a polling loop, checking each agent's status every 5 seconds.

Key behaviors:

- **`fail_fast = true`** (default): When one agent fails, the engine cancels remaining agents and marks the block as failed
- **`fail_fast = false`**: All agents run to completion regardless of individual failures
- **`min_success`**: If set, the block succeeds when at least N agents succeed, even if others fail
- **Marker merging**: Markers from all completed agents are merged into a single set for downstream conditions
- **Context accumulation**: Each completed agent adds a context entry to the accumulator

All agents in a parallel block share the same worktree. This is safe because reviewers are read-only by convention. If an actor agent were placed in a parallel block, it could cause file conflicts.

## Loop Execution

### `while` loops

The engine checks the condition before each iteration. If the condition is false on entry, the body never executes.

### `do-while` loops

The body executes once unconditionally, then the condition is checked after each subsequent iteration. This is the natural pattern for "try, then verify, repeat if needed."

### Loop guards

| Guard | Behavior |
|-------|----------|
| `max_iterations` | Hard cap. Required for all loops |
| `stuck_after` | Fails if the marker set is identical for N consecutive iterations |
| `on_max_iter = fail` | Marks the workflow as failed when the cap is reached |
| `on_max_iter = continue` | Continues past the loop, proceeding to the next node |

The `stuck_after` guard prevents infinite loops where the agent keeps producing the same markers without making progress. The engine compares the full marker set between consecutive iterations.

## Gate Execution

Gates pause the workflow until an external condition is met. The engine:

1. Inserts a step record with `waiting` status and gate metadata
2. Updates the workflow run to `waiting` status
3. Polls the database every 10 seconds for gate resolution
4. On resolution, updates the step and resumes execution

### Human gates

`human_approval` and `human_review` gates wait for human action through any conductor interface:

```
conductor workflow gate-approve  <run-id>
conductor workflow gate-reject   <run-id>
conductor workflow gate-feedback <run-id> "<text>"
```

`human_review` captures feedback text, which is injected into the next step as `{{gate_feedback}}`.

### Automated gates

`pr_approval` polls GitHub for PR approvals. Two modes: `min_approvals` counts raw approvals; `review_decision` delegates to GitHub's branch-protection-aware merge status.

`pr_checks` polls GitHub CI status until all required checks pass.

### Timeout handling

Gates have mandatory timeouts. When a timeout expires, the `on_timeout` option determines behavior: `fail` (default) marks the workflow as failed; `continue` proceeds past the gate.

## Workflow Composition

Sub-workflow invocation (`call workflow <name>`) follows a function-call model: inputs go in, output comes out, internals are hidden.

### Execution flow

1. Parent encounters `call workflow <name>`
2. Engine loads and validates the referenced `.wf` file
3. A child workflow run is created in the database, linked to the parent
4. Child executes to completion using the standard engine (recursive call)
5. Child's terminal markers and context bubble up to the parent
6. Parent continues with its next node

### Isolation boundaries

- **No shared context**: The child starts with a fresh `prior_contexts`. Only the final output crosses the boundary. This keeps sub-workflows independently testable
- **No partial execution**: Sub-workflows run from beginning to end. Extract smaller workflows for subset reuse
- **No dynamic dispatch**: The workflow name is a static identifier, not a variable. This keeps the dependency graph statically analyzable

### Error propagation

If the sub-workflow fails, the parent step fails. The parent's `retries` and `on_fail` apply to the entire sub-workflow invocation -- a retry re-runs from the beginning.

Gates in sub-workflows block the parent. The parent's status shows `waiting` with a reference to the child's pending gate. Users interact with the gate through the parent workflow's UI without knowing about the composition boundary.

## Resumability

The engine is fully resumable from database state. On startup, it scans for workflow runs in `running` or `waiting` status and re-enters execution from the last non-terminal step.

This works because:

1. Every step result is durably recorded in `workflow_run_steps`
2. The `definition_snapshot` eliminates dependency on the `.wf` file
3. The `ResumeContext` pre-loads completed step keys and results into a skip-set
4. Each step checks the skip-set before executing -- completed steps are restored from DB rather than re-run

This is critical because conductor has no persistent daemon in v1. If the process exits (intentionally or not), the next invocation picks up where it left off.

**Reaper behavior**: The reaper (dead-run cleanup) runs within the workflow execution loop, not as a separate daemon. Reaping only occurs when another workflow executes or when the TUI/web UI polls. The reaper checks each workflow's parent agent run status; if the parent is not in `running` or `waiting_for_feedback` state, the workflow is cancelled -- even if it is legitimately paused at a gate.

## Dry-Run Mode

`conductor workflow run <name> --dry-run` modifies execution:

| Construct | Dry-run behavior |
|-----------|------------------|
| `call` with `can_commit = false` | Runs normally; `{{dry_run}}` is `"true"` |
| `call` with `can_commit = true` | Prepends "DO NOT commit or push" to prompt |
| Human gates | Auto-approved; `{{gate_feedback}}` is empty |
| Automated gates | Skipped (treated as satisfied) |
| `always` | Runs normally |

Dry-run status is stored on the run record so history clearly identifies dry runs.

## Design Trade-Offs

### Custom DSL over TOML/YAML/JSON

**GAIN**: Workflows read like pseudocode. The DSL naturally expresses polymorphic node types (call, parallel, gate, loop) without awkward type tags or deep nesting. `.wf` files are scannable at a glance.

**COST**: A custom parser to maintain (~4K LoC). No ecosystem tooling (syntax highlighting, linters, formatters). Contributors must learn a new syntax rather than using familiar formats.

### Markers over free-text scanning

**GAIN**: Conditions are deterministic. No false positives from agents mentioning marker names in natural language. Markers are structured data that can be inspected, logged, and compared.

**COST**: Agents must follow the `CONDUCTOR_OUTPUT` protocol. Agents that omit the block contribute no markers (tolerable -- treated as "no issues"). The protocol adds boilerplate to every agent output.

### Shallow composition over deep nesting

**GAIN**: Encapsulation. A parent cannot depend on a child's internal step names. Resumability is simpler (no arbitrary-depth call stack to snapshot). Sub-workflows are independently testable.

**COST**: Cannot inject context between a child's steps or react to intermediate markers. If you need that level of control, you must inline the steps. More verbose than an `import` mechanism.

### Subprocess spawning over in-process execution

**GAIN**: Crash isolation. An agent failure does not crash the engine. The engine can be restarted without losing progress. Multiple agents run in separate tmux windows with independent resource usage.

**COST**: No streaming communication between engine and agent. The 5-second polling interval adds latency. Spawning tmux windows requires tmux to be available on the system.

### Snapshot-based execution over live parsing

**GAIN**: Workflow edits during a run do not affect in-flight behavior. Each run is self-contained and reproducible from its snapshot.

**COST**: Database storage for the serialized AST. Cannot hot-fix a running workflow -- must cancel and re-run with the updated definition.

## Known Limitations

- **Gate timeout vs. reaper race**: The reaper's dead-parent check can override a gate's configured timeout. If the parent agent run exits before the gate timeout expires, the reaper cancels the workflow prematurely. A fix is in progress.

## Related Documents

- [Workflow DSL Syntax Reference](../reference/workflow-dsl.md) -- complete syntax specification for `.wf` files
- [Crate Structure and Dependency Graph](crate-structure.md) -- how the workflow engine fits into conductor-core
- [Database Schema Reference](../reference/database-schema.md) -- `workflow_runs` and `workflow_run_steps` table schemas
- [Prompt Snippets](../workflow/prompt-snippets.md) -- full prompt snippet system specification
- [Agent Path Resolution](../workflow/agent-path-resolution.md) -- agent resolution design and alternatives
- [Structured Output Architecture](structured-output.md) -- CONDUCTOR_OUTPUT design and schema validation
- [How to Write a Workflow](../how-to/write-workflow.md) -- step-by-step guide for creating workflows
- For the execution engine implementation, see `conductor-core/src/workflow.rs` (~10.5K LoC)
- For the DSL parser implementation, see `conductor-core/src/workflow_dsl.rs` (~4K LoC)
- For workflow templates, see [/usr/local/bsg/fsm-engine/](/usr/local/bsg/fsm-engine/) (fsm-engine owns workflow templates)
