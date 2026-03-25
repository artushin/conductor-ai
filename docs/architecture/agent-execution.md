---
title: Agent Execution Architecture
type: explanation
layer: 2
---

# Agent Execution Architecture

How conductor spawns, monitors, and captures output from Claude agents. This document explains the subprocess model, plugin injection, agent resolution, output capture pipeline, and the design trade-offs behind these choices.

## Execution Model

Conductor runs agents as external processes in tmux windows. The engine never runs Claude in-process. Each agent gets its own tmux window running `conductor agent run`, which in turn executes `claude -p` with the assembled prompt.

```
Workflow Engine                       tmux session
  │                                     │
  ├─ create agent_run record (DB)       │
  ├─ spawn_tmux_window ──────────────► window: conductor agent run --run-id <id>
  │                                       └─ claude -p "<prompt>" --output-format json
  ├─ poll DB every 5s ◄──────────────── update agent_runs on completion
  └─ parse CONDUCTOR_OUTPUT from result_text
```

The engine and the agent process communicate exclusively through the database. There is no streaming, no IPC, no shared memory. The agent writes its results to the `agent_runs` table; the engine polls for status changes.

## Agent Resolution

When a workflow `call` references an agent by name, the engine resolves it to a `.md` file through a search order.

### Short name resolution

Bare identifiers (e.g., `call plan`) are checked in this order (first match wins):

| Priority | Path | Scope |
|----------|------|-------|
| 1 | `.conductor/workflows/<workflow>/agents/<name>.md` | Workflow-local override |
| 2 | `.conductor/agents/<name>.md` | Shared conductor agents |
| 3 | `.claude/agents/<name>.md` | Claude Code agents (fallback) |

Each priority is checked in the worktree path first, then the repo path. This means a worktree can override a repo-level agent for testing without modifying the repo.

### Explicit path resolution

Quoted strings (e.g., `call ".claude/agents/plan.md"`) bypass the search order and resolve directly relative to the repo root. Absolute paths and repo-escaping paths are rejected.

### Plugin-provided agents

Agents from external repos (agent-architecture, fsm-engine) are loaded via `--plugin-dir`. When the `claude` CLI receives a `--plugin-dir` flag, it adds that directory's agents to the resolution path. This is how shared agents from `/usr/local/bsg/agent-architecture/<agent>` become available to workflows.

For the detailed design proposal on extended resolution features, see [agent-path-resolution.md](../workflow/agent-path-resolution.md).

## Agent Definition Parsing

Each agent `.md` file uses YAML frontmatter for metadata and markdown body for the prompt.

```yaml
---
name: review-security
role: reviewer
can_commit: false
model: opus
---

You are a security reviewer. Analyze the code for vulnerabilities...
```

The engine parses the frontmatter into an `AgentDef` struct:

| Frontmatter Field | Type | Default | Used For |
|-------------------|------|---------|----------|
| `role` | `actor` or `reviewer` | `reviewer` | Stored in `workflow_run_steps.role`; affects dry-run behavior |
| `can_commit` | boolean | `false` | If true + dry-run: prompt prepended with "DO NOT commit" |
| `model` | string | none | Model override for this agent (cascades: agent > worktree > repo > config) |

The markdown body (everything after the frontmatter) becomes the agent prompt, with `{{variable}}` substitution applied at execution time.

## Prompt Assembly

The engine builds the final prompt from four sources, in order:

1. **Agent `.md` body** -- with `{{variable}}` substitution (inputs, `prior_context`, `prior_contexts`, `prior_output`, `gate_feedback`, `dry_run`, etc.)
2. **Prompt snippets** (`with`) -- each `.md` file also gets variable substitution; block-level `with` is prepended, per-call `with` is appended
3. **Output instructions** -- schema-specific JSON template if `output` is specified, otherwise the generic `CONDUCTOR_OUTPUT` instruction

For dry-run mode with `can_commit = true` agents, a "DO NOT commit or push" prefix is prepended before the agent body.

Variable substitution is simple string replacement: `{{key}}` is replaced with the variable's value. Unrecognized variables pass through unchanged (by design -- agent prompts may use `{{` for other template systems).

### Startup context

`build_startup_context` includes the full ticket body (not just title) for both worktree and ephemeral runs. Workflow template variables include `ticket_id` and `ticket_title` but NOT `ticket_body`. The body is available via `build_startup_context` injection or the `fetch-ticket-context` agent.

## Spawning

### The tmux subprocess chain

When the engine reaches a `call` step:

1. **Create `agent_run` record**: Insert a row in `agent_runs` with `running` status, the assembled prompt, and the tmux window name
2. **Spawn tmux window**: Execute `tmux new-window -d -n <window-name> -- conductor agent run --run-id <id> --worktree-path <path> --prompt <prompt>`
3. **Verify window**: After a 100ms delay, check that the tmux window exists via `list_live_tmux_windows()`

If no tmux server is running, the engine creates a detached `conductor` session automatically, so agents can run without a pre-existing tmux environment.

### What `conductor agent run` does

The `conductor agent run` internal subcommand:

1. Verifies the run record exists in the database
2. Executes `claude -p <prompt> --output-format json --verbose --permission-mode acceptEdits`
3. Pipes stdout (JSON result) and inherits stderr (visible in the tmux terminal)
4. Parses the `ClaudeJsonResult` from stdout
5. Updates the `agent_runs` row with status, session ID, cost, turns, duration, and token counts

### Plugin directory injection

The engine assembles `--plugin-dir` flags for the Claude session:

| Level | Source | Scope |
|-------|--------|-------|
| Repo-level | `conductor repo set-plugin-dirs <slug> <paths...>` | All steps in all workflows for this repo |
| Environment | `CONDUCTOR_PLUGIN_DIRS` env var | Appended after repo-level |
| Per-step | `plugin_dirs = [...]` on the `call` block | Appended after env dirs, this step only |

This allows a single workflow to use different agent sets per step. For example, a PE discovery step loads pattern-extractor plugins while review steps use agent-architecture plugins:

```
call pe-discover { plugin_dirs = ["/usr/local/bsg/pattern-extractor"] }
call review-architecture    # uses repo-level plugin dirs only
```

## Polling and Output Capture

### Status polling

The engine polls the `agent_runs` table every 5 seconds. For parallel blocks, all spawned agents are polled in the same loop. The poll checks for terminal states (`completed`, `failed`, `cancelled`).

The `poll_child_completion` function also supports:

- **Shutdown flag**: An `AtomicBool` checked each cycle; allows the engine to abort polling when a shutdown is requested
- **Timeout**: A maximum duration after which polling returns a `Timeout` error

### Output parsing pipeline

When an agent completes, the engine processes its output through a two-stage pipeline:

**Stage 1: Extract the CONDUCTOR_OUTPUT block**

The engine finds the last `<<<CONDUCTOR_OUTPUT>>>` / `<<<END_CONDUCTOR_OUTPUT>>>` pair in the agent's result text. Using the last occurrence avoids false positives when agents include example blocks in code fences.

**Stage 2: Interpret (schema-aware or generic)**

| Schema specified? | Agent succeeded? | Behavior |
|-------------------|-----------------|----------|
| Yes | Yes | Parse JSON, validate against schema, derive markers. Validation failure = retriable error |
| Yes | No | Fall back to generic parsing (extract `markers`/`context` if present) |
| No | Any | Generic parsing: extract `markers` array and `context` string |

The `interpret_agent_output` function handles this dispatch. For schema-validated output, the engine stores the full JSON in `workflow_run_steps.structured_output` and populates both `markers_out` and `context_out` for backward compatibility.

## Error Handling

### Agent failures

When an agent fails (Claude exits with error, or the result has `is_error: true`):

1. The `agent_runs` row is updated to `failed` status with the error text
2. The `workflow_run_steps` row is updated to `failed`
3. If `retries > 0`, the step is re-executed with a fresh agent run
4. If all retries are exhausted and `on_fail` is specified, the fallback agent runs
5. If no `on_fail`, the step fails. If `fail_fast = true` (default), the workflow fails

### [NEEDS_FEEDBACK] behavior

In headless/workflow mode, agent feedback requests (`[NEEDS_FEEDBACK]`) stall the session silently. The conductor feedback protocol requires active monitoring (TUI or MCP `submit_agent_feedback` tool) to deliver responses. Without a listener, the agent blocks indefinitely until the step timeout expires.

### Tmux window failures

If the tmux window exits immediately after spawn (agent process crashed on startup), the `verify_tmux_window` check catches this within 100ms and the step fails with a descriptive error.

If the tmux window disappears during execution (OOM kill, manual `tmux kill-window`), the engine detects this during the next poll cycle. The agent run transitions to `failed` or `cancelled` status.

### Output parsing failures

If the agent completes successfully but the `CONDUCTOR_OUTPUT` block is missing, malformed, or fails schema validation, the behavior depends on context:

- **No schema**: Missing block is tolerated (empty markers, empty context)
- **With schema, agent succeeded**: Validation failure triggers a retry if `retries` is configured
- **With schema, agent failed**: Falls back to generic parsing silently

## Orchestration and Child Agents

The `orchestrator.rs` module supports parent-child agent relationships. A parent agent can spawn child agents, each tracked via `agent_runs.parent_run_id`.

The orchestration model:

1. Parent agent creates child agent runs in the database
2. Child agents spawn in separate tmux windows using the same `spawn_child_tmux` function
3. Parent polls for child completion using `poll_child_completion`
4. Child results (markers, context) are available to the parent

This is separate from workflow composition (`call workflow`). Orchestration happens within a single workflow step; workflow composition chains entire workflows.

## Design Trade-Offs

### Subprocess spawning over in-process execution

**GAIN**: Crash isolation -- an agent failure does not crash the engine. The engine can be killed and restarted, then resume by polling existing agent runs. Multiple agents run in separate tmux windows with independent resource usage. Users can attach to tmux windows and watch agents work in real time.

**COST**: No streaming communication between engine and agent. The 5-second polling interval adds latency to step transitions. Spawning tmux windows requires tmux to be installed. Orphan detection logic is needed for agents whose engine process disappeared.

### Tmux over headless subprocesses

**GAIN**: Users can attach to agent windows (`press a to attach`), providing real-time visibility into agent execution with full scrollback. Tmux windows survive TUI crashes -- the agent keeps running independently. Human-in-the-loop interactions happen naturally in the terminal.

**COST**: Tmux is an external dependency. Orphan tmux windows require cleanup logic. No structured streaming output -- the engine must poll the database rather than receiving real-time events. The web UI has no terminal to attach to (a hybrid approach with headless subprocesses is the planned solution for the web backend).

### Database polling over event streams

**GAIN**: Simple and robust. No event bus to manage, no message ordering concerns, no reconnection logic. The database is the single source of truth. Any conductor process (CLI, TUI, web) can read the current state.

**COST**: 5-second latency on state transitions. Higher database read load (one query per agent per poll cycle). Not suitable for high-throughput scenarios with many concurrent agents.

## Related Documents

- [Crate Structure and Dependency Graph](crate-structure.md) -- agent management modules within conductor-core
- [Structured Output Architecture](structured-output.md) -- how agent output is validated and routed
- [Output Schemas Reference](../reference/output-schemas.md) -- schema file format and validation rules
- [Workflow Engine Architecture](workflow-engine.md) -- the execution engine that drives agent invocation
- [Agent Path Resolution](../workflow/agent-path-resolution.md) -- extended agent resolution design
- [Database Schema Reference](../reference/database-schema.md) -- `agent_runs`, `agent_run_steps`, `agent_run_events` tables
- For the implementation: `conductor-core/src/agent.rs` (~4.5K LoC), `agent_config.rs` (~940 LoC), `agent_runtime.rs` (~225 LoC), `orchestrator.rs` (~813 LoC)
- For ecosystem agent loading, see [/usr/local/bsg/CLAUDE.md](/usr/local/bsg/CLAUDE.md) (plugin injection rules)
