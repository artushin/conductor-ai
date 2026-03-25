---
title: Conductor-AI Codebase Exploration Report (Archived)
type: reference
layer: 3
archived: true
original_path: docs/EXPLORATION_REPORT.md
archived_at: 2026-03-23
---

# Conductor-AI Codebase Exploration Report

**Date:** 2026-03-21
**Methodology:** 7-phase codebase exploration (fingerprinting, structure, dependencies, patterns, abstractions, data flows, documentation triage)

---

## Part I: User Perspective

### What Is Conductor?

Conductor is a **local-first, multi-repo orchestration tool** for AI-assisted software development. It manages the lifecycle of feature work: from syncing tickets, to creating isolated git worktrees, to running AI agents (Claude) against those worktrees, to executing multi-step workflows that chain agents together.

**Core problem it solves:** Coordinating AI agent work across multiple repositories without a cloud service. Everything runs locally, backed by SQLite.

### Key Concepts

| Concept | What It Is |
|---------|-----------|
| **Repo** | A registered git repository. Conductor tracks its local path, remote URL, default branch, and plugin directories. |
| **Worktree** | An isolated git worktree (branch + working directory) for a feature or fix. Named with `feat-`/`fix-` prefix, auto-normalized to `feat/`/`fix/` branches. |
| **Ticket** | An issue from GitHub or Jira, synced into Conductor's local DB. Can be linked to a worktree. Also supports local-only tickets. |
| **Agent Run** | A single Claude agent execution in a tmux session. Tracks cost, turns, duration, plan steps, and structured events. |
| **Workflow** | A multi-step pipeline defined in `.wf` DSL files. Chains agents together with control flow (if/while/parallel), gates (human approval), and structured output. |
| **Plugin Directory** | A path containing agent definitions (`.md` files). Passed to Claude via `--plugin-dir` to give it specialized capabilities. |
| **Gate** | A pause point in a workflow requiring human approval, review feedback, PR approval, or CI checks before continuing. |

### Three Interfaces

| Interface | Binary | Purpose |
|-----------|--------|---------|
| **CLI** | `conductor` | Command-line tool for all operations |
| **TUI** | `conductor-tui` | Interactive terminal UI (ratatui) with dashboard, worktree detail, workflow monitoring |
| **Web** | `conductor-web` | Axum REST API + embedded React frontend with SSE live updates |

Plus: **MCP server** (`conductor mcp serve`) for Claude Code integration, and a **daemon** (`conductor-service`) for background coordination.

### CLI Command Reference

```
conductor
├── repo
│   ├── register <URL>          Register a repository
│   ├── list                    List registered repos
│   ├── discover [OWNER]        Discover repos from GitHub
│   ├── unregister <SLUG>       Unregister a repo
│   ├── set-model <SLUG> [M]    Set default model for a repo
│   ├── set-plugin-dirs <SLUG>  Configure agent plugin directories
│   ├── allow-agent-issues      Toggle agent issue creation
│   └── sources
│       ├── add <SLUG>          Add GitHub/Jira issue source
│       ├── list <SLUG>         List issue sources
│       └── remove <SLUG>       Remove an issue source
├── worktree
│   ├── create <REPO> <NAME>    Create worktree (+ auto-install deps)
│   ├── list [REPO]             List worktrees
│   ├── delete <REPO> <NAME>    Soft-delete (mark merged/abandoned)
│   ├── purge <REPO> [NAME]     Hard-delete completed worktrees
│   ├── push <REPO> <NAME>      Push branch to origin
│   ├── pr <REPO> <NAME>        Create pull request
│   └── set-model <R> <N> [M]   Set model for worktree
├── tickets
│   ├── sync [REPO]             Sync from GitHub/Jira
│   ├── list [REPO]             List cached tickets
│   ├── link <T> <R> <W>        Link ticket to worktree
│   ├── stats [REPO]            Cost/turn stats per ticket
│   ├── create                  Create local ticket
│   └── update <ID>             Update ticket fields
├── agent
│   ├── run                     Run Claude agent (internal, tmux)
│   ├── orchestrate             Orchestrate child agents
│   └── create-issue            Create GitHub issue from agent
├── workflow
│   ├── list                    List available workflows
│   ├── run <NAME> [R] [W]      Execute a workflow
│   ├── run-show <ID>           Show workflow run details
│   ├── validate <NAME>         Validate workflow definition
│   ├── resume <ID>             Resume failed/waiting run
│   ├── cancel <ID>             Cancel a running workflow
│   ├── runs <REPO> [WT]        List run history
│   ├── gate-approve <ID>       Approve a pending gate
│   ├── gate-reject <ID>        Reject a pending gate
│   ├── gate-feedback <ID> <T>  Provide feedback + approve gate
│   └── purge                   Delete terminal workflow runs
├── statusline
│   ├── install                 Install Claude Code integration
│   └── uninstall               Remove Claude Code integration
└── mcp
    └── serve                   Start MCP server (stdio)
```

### Typical User Workflow

```
1. Register a repo:           conductor repo register https://github.com/org/repo
2. Configure issue source:    conductor repo sources add repo --type github
3. Sync tickets:              conductor tickets sync repo
4. Create worktree from ticket: conductor worktree create repo feat-new-feature --ticket 42
5. Run a workflow:            conductor workflow run ticket-to-pr repo feat-new-feature
6. Monitor progress:          conductor workflow run-show <run-id>
7. Approve gates:             conductor workflow gate-approve <run-id>
8. Resume on failure:         conductor workflow resume <run-id>
9. Push and PR:               conductor worktree push repo feat-new-feature
                              conductor worktree pr repo feat-new-feature
```

### Configuration

**Global config:** `~/.conductor/config.toml`
```toml
[general]
workspace_root = "~/.conductor/workspaces"
sync_interval_minutes = 5
model = "sonnet"                           # default model (opus/sonnet/haiku)

[defaults]
default_branch = "main"

[github.app]
app_id = 123456
private_key_path = "~/.conductor/conductor-ai.pem"
installation_id = 789012
```

**Data directory:** `~/.conductor/`
```
conductor.db          SQLite database (WAL mode)
config.toml           Global config
agent-logs/           Per-run agent log files
conductor.sock        Daemon Unix socket
conductor.pid         Daemon PID lock
pe-checkpoint.json    Pattern Extractor state
workspaces/
  <repo-slug>/
    main/             Main branch checkout
    <worktree-slug>/  Feature branch worktrees
```

---

## Part II: Technical Architecture

### Crate Structure

```
conductor-ai/              Cargo workspace (edition 2021, resolver v2)
├── conductor-core/         Library crate: ALL domain logic
├── conductor-cli/          Binary: thin clap wrapper + MCP server
├── conductor-tui/          Binary: ratatui + crossterm interactive UI
├── conductor-web/          Binary: axum REST API + embedded React frontend
└── conductor-service/      Binary: tokio daemon (Unix socket, PE watcher)
```

**Design principle:** Library-first. No daemon required in v1. CLI and TUI import `conductor-core` directly. SQLite WAL mode handles concurrency across processes.

### Dependency Map

| Crate | Key Dependencies | Purpose |
|-------|-----------------|---------|
| **conductor-core** | rusqlite (bundled), serde, chrono, ulid, toml, ureq, notify, thiserror | Domain logic, DB, HTTP, file watching |
| **conductor-cli** | clap (derive), anyhow, rmcp, tokio | CLI parsing, MCP server |
| **conductor-tui** | ratatui, crossterm, tui-textarea | Terminal UI rendering |
| **conductor-web** | axum, tokio (full), tower-http, rust-embed, futures-util | Web server, embedded frontend |
| **conductor-service** | tokio (full), tracing-subscriber | Daemon event loop |

### Module Organization (conductor-core/src/)

```
Domain Logic:
├── repo.rs              RepoManager: register, list, plugin dirs, model override
├── worktree.rs          WorktreeManager: create, delete, push, branch normalization
├── tickets.rs           TicketSyncer: upsert, filter, link, local tickets
├── issue_source.rs      IssueSourceManager: GitHub/Jira source configuration
├── agent.rs             AgentManager: run lifecycle, plan steps, events, feedback
├── agent_config.rs      Agent spec parsing (frontmatter + markdown)
├── agent_runtime.rs     Tmux spawning, polling, child execution
├── orchestrator.rs      Parent-child agent orchestration

Workflow Engine:
├── workflow.rs          Execution engine, context flow, structured output
├── workflow_dsl.rs      Custom DSL parser (recursive descent, ~4000 lines)
├── workflow_config.rs   Workflow definition loading from disk
├── workflow_ephemeral.rs  Ephemeral PR workflow execution

Configuration:
├── config.rs            Config loading/saving, paths, model tiers
├── prompt_config.rs     Agent prompt snippet loading
├── schema_config.rs     JSON schema validation for agent output
├── models.rs            Known models (opus/sonnet/haiku), suggestion heuristics

Integration:
├── github.rs            GitHub API (gh CLI wrapper)
├── github_app.rs        GitHub App JWT authentication
├── jira_acli.rs         Jira CLI integration

Infrastructure:
├── error.rs             ConductorError enum (17 variants)
├── text_util.rs         Frontmatter parsing, string utilities
├── db/mod.rs            SQLite connection (WAL, FK, 5s timeout)
├── db/migrations.rs     Versioned migration runner
└── db/migrations/       37 SQL migration files
```

### Core Domain Types

```rust
Repo { id, slug, local_path, remote_url, default_branch, workspace_dir,
       model, allow_agent_issue_creation, plugin_dirs: Vec<String> }

Worktree { id, repo_id, slug, branch, path, ticket_id, status: Active|Merged|Abandoned,
           model, base_branch }

Ticket { id, repo_id, source_type, source_id, title, body, state, labels,
         assignee, priority, url, synced_at, raw_json }

AgentRun { id, worktree_id, claude_session_id, prompt, status, result_text,
           cost_usd, num_turns, duration_ms, model, plan: Vec<PlanStep>,
           parent_run_id, input_tokens, output_tokens, bot_name }

WorkflowRun { id, workflow_name, worktree_id, parent_run_id, status, dry_run,
              trigger, definition_snapshot, inputs, ticket_id, repo_id,
              parent_workflow_run_id, target_label, default_bot_name }

WorkflowRunStep { id, workflow_run_id, step_name, role, status, child_run_id,
                  position, result_text, context_out, markers_out, iteration,
                  structured_output, gate_type, gate_prompt, gate_feedback }
```

**ID scheme:** ULIDs (sortable, collision-resistant). **Timestamps:** ISO 8601 strings.

### Manager Pattern

All domain logic organized as manager structs:

```rust
// Construction: takes DB connection + config
let mgr = RepoManager::new(&conn, &config);
let wt  = WorktreeManager::new(&conn, &config);
let ts  = TicketSyncer::new(&conn);
let am  = AgentManager::new(&conn);
let wm  = WorkflowManager::new(&conn);

// Methods return Result<T> with ConductorError
let repo = mgr.register("slug", "/path", "https://...", None)?;
let (worktree, warnings) = wt.create("repo", "feat-name", None, None, None)?;
```

### Error Handling

```rust
// conductor-core: domain-specific errors
pub enum ConductorError {
    Database(rusqlite::Error),
    RepoNotFound { slug },
    RepoAlreadyExists { slug },
    WorktreeNotFound { slug },
    WorktreeAlreadyExists { slug },
    Git(String),
    Config(String),
    Io(std::io::Error),
    TicketSync(String),
    IssueSourceAlreadyExists { repo_slug, source_type },
    TicketNotFound { id },
    Agent(String),
    TicketAlreadyLinked,
    Workflow(String),
    AgentConfig(String),
    Schema(String),
    WorkflowRunAlreadyActive { name },
}

// Binaries: anyhow::Result for one-shot reporting
```

### Database

- **Engine:** SQLite with WAL mode, foreign keys ON, 5s busy timeout
- **Location:** `~/.conductor/conductor.db`
- **Schema:** 37 versioned migrations in `conductor-core/src/db/migrations/`
- **Core tables:** repos, worktrees, tickets, ticket_labels, agent_runs, agent_run_steps, agent_run_events, feedback_requests, agent_created_issues, workflow_runs, workflow_run_steps, repo_issue_sources, _conductor_meta

### Model Resolution Hierarchy

```
Per-run override  →  Per-worktree model  →  Per-repo model  →  Global config  →  "sonnet"
```

**Known models:**
| Alias | Model ID | Tier | Use Case |
|-------|----------|------|----------|
| opus | claude-opus-4-6 | Powerful | Planning, architecture, complex analysis |
| sonnet | claude-sonnet-4-6 | Balanced | General implementation (default) |
| haiku | claude-haiku-4-5-20251001 | Fast | Commit messages, formatting, quick edits |

---

## Part III: Workflow Engine Deep-Dive

### DSL Grammar

```
workflow <name> {
  meta { description = "...", trigger = "manual", targets = ["worktree"] }
  inputs { <name> required | <name> default = "value" }

  call <agent>                             Sequential agent execution
  call <agent> { retries = 2, on_fail = <agent>, output = "schema" }
  call workflow <name> { inputs { ... } }  Sub-workflow composition

  if <step>.<marker> { ... }               Conditional (marker present)
  unless <step>.<marker> { ... }           Conditional (marker absent)
  while <step>.<marker> { max_iterations = 5 ... }   Loop with pre-check
  do { ... } while <step>.<marker>         Loop with post-check

  do { output = "schema", with = ["snippet"] ... }   Block grouping

  parallel { call <a>, call <b>, call <c> }           Concurrent execution
  parallel { fail_fast = false, min_success = 2 ... } With thresholds

  gate human_approval { prompt = "...", timeout = "24h" }
  gate pr_checks { auto_pass = true }

  always { ... }                           Runs on success or failure
}
```

### Execution Flow

```
execute_workflow()
├─ Validate all referenced agents exist
├─ Create workflow_run record (status: Pending)
├─ Initialize ExecutionState (step_results, contexts, position, accumulators)
├─ Execute body nodes sequentially:
│   ├─ Call → spawn agent in tmux, poll for completion, parse output
│   ├─ If/Unless → check marker in step_results, conditionally execute body
│   ├─ While/DoWhile → loop with marker check, max_iterations cap
│   ├─ Parallel → spawn all agents concurrently, poll all, merge markers
│   ├─ Gate → pause execution, wait for approval/CI/PR checks
│   └─ CallWorkflow → recursively execute child workflow
├─ Execute always block (regardless of outcome)
└─ Finalize: update status, aggregate cost/turns/duration
```

### Context Flow Between Steps

1. Agent runs and emits `<<<CONDUCTOR_OUTPUT>>>` JSON block
2. Engine extracts `markers: [...]` and `context: "..."` from the block
3. If `output` schema specified, validates structured JSON against schema
4. Results stored in `step_results[step_name]`
5. Next step receives via template variables:
   - `{{prior_context}}` -- context from previous step
   - `{{prior_contexts}}` -- JSON array of all step contexts
   - `{{prior_output}}` -- raw structured JSON (if schema used)
6. Markers available for `if`/`while` conditions: `step_name.marker_name`

### Gate Mechanics

- **Human gates:** Workflow pauses, status changes to `Waiting`. User approves via CLI (`conductor workflow gate-approve <id>`) or TUI. Feedback text injected as `{{gate_feedback}}`.
- **PR gates:** Engine polls GitHub via `gh` CLI for approval count or CI check status. Auto-passes when conditions met.
- **Timeouts:** Configurable per gate. On timeout: fail or continue (configurable).
- **Dry-run mode:** All gates auto-approve.

### Retry and Error Recovery

```
call implement { retries = 2, on_fail = plan-failure-report }
```

1. If agent fails, retry up to `retries` times
2. If all retries exhausted and `on_fail` specified, run fallback agent
3. Fallback agent receives `{{failed_step}}`, `{{failure_reason}}`, `{{retry_count}}`
4. Fallback failure is logged but non-terminal

### Resume Mechanism

```bash
conductor workflow resume <run-id>              # From last failure
conductor workflow resume <run-id> --from-step plan  # From specific step
conductor workflow resume <run-id> --restart    # From beginning
```

On resume: completed steps are skipped (results restored from DB), execution continues from the failure point.

### Sub-Workflow Composition

```
call workflow test-coverage { inputs { pr_url = "{{pr_url}}" } }
```

- Parent variables substituted into child inputs
- Child executes independently, returns markers + context to parent
- Markers bubble up: `test-coverage.has_missing_tests` available for parent conditions
- Max nesting depth: 5. Cycle detection runs at validation time.

---

## Part IV: MCP Server

### Tools (22 total)

| Tool | Description |
|------|-------------|
| `conductor_list_repos` | List registered repos with active run counts |
| `conductor_register_repo` | Register new repo by remote URL |
| `conductor_unregister_repo` | Remove repo registration |
| `conductor_list_worktrees` | List worktrees (filterable by status) |
| `conductor_get_worktree` | Rich detail: branch, ticket, PR, latest runs |
| `conductor_create_worktree` | Create worktree, optionally link ticket |
| `conductor_delete_worktree` | Remove git worktree and branch |
| `conductor_push_worktree` | Push branch to remote |
| `conductor_list_tickets` | Filter tickets by label/search/state |
| `conductor_sync_tickets` | Sync from GitHub/Jira (or re-fetch single ticket) |
| `conductor_list_workflows` | List available workflow definitions |
| `conductor_validate_workflow` | Check agents, prompts, dataflow, cycles |
| `conductor_run_workflow` | Execute workflow (supports --pr, --dry-run, inputs) |
| `conductor_list_runs` | Paginated workflow run listing |
| `conductor_get_run` | Full run detail with steps |
| `conductor_get_step_log` | Retrieve agent log for a workflow step |
| `conductor_resume_run` | Resume failed/paused run |
| `conductor_cancel_run` | Cancel running/waiting workflow |
| `conductor_approve_gate` | Approve pending gate (with optional feedback) |
| `conductor_reject_gate` | Reject pending gate |
| `conductor_list_agent_runs` | List agent runs (filter by status/repo/worktree) |
| `conductor_submit_agent_feedback` | Submit feedback to waiting agent |
| `conductor_list_prs` | List open PRs with CI status |

### Resources

| URI | Content |
|-----|---------|
| `conductor://repos` | All registered repos |
| `conductor://repo/{slug}` | Single repo detail |
| `conductor://tickets/{repo}` | Open tickets (max 100) |
| `conductor://ticket/{repo}/{id}` | Full ticket body |
| `conductor://worktrees/{repo}` | All worktrees for repo |
| `conductor://worktree/{repo}/{slug}` | Single worktree with ticket |
| `conductor://runs/{repo}` | Recent workflow runs (max 50) |
| `conductor://run/{run_id}` | Full run with steps + log tail |
| `conductor://workflows/{repo}` | Available workflow definitions |

### Architecture

```
Claude Code  <-stdio->  conductor mcp serve  ->  spawn_blocking  ->  conductor-core  ->  SQLite
```

Each MCP request opens a fresh SQLite connection. All conductor-core calls wrapped in `tokio::task::spawn_blocking` to bridge sync library to async MCP transport.

---

## Part V: Daemon (conductor-service)

- **Transport:** Unix domain socket at `~/.conductor/conductor.sock`
- **Protocol:** JSON-RPC 2.0 (newline-delimited)
- **Lifecycle:** PID lock at `~/.conductor/conductor.pid` with stale detection
- **Methods:** `ping` (health), `status` (running state), `pe.checkpoint.status` (PE state)
- **Background task:** Polls `~/.conductor/pe-checkpoint.json` every 5 seconds for Pattern Extractor state changes
- **Future:** Will create conductor gates for PE state transitions (v1 logs only)

---

## Part VI: TUI Architecture

- **Framework:** ratatui 0.29 + crossterm 0.28
- **Threading:** Single main thread for rendering (60 FPS), background thread pool for blocking operations
- **Critical rule:** Never block the main thread. All git/DB/subprocess calls spawn on background threads, post results back via MPSC channel as `Action` variants.
- **Screens:** Dashboard, Repo Detail, Worktree Detail, Tickets, Workflows, Help
- **Code size:** ~170K lines across app.rs (6.3K), state.rs (3K), ui/ modules (modal.rs alone is 49K)

---

## Part VII: BSG Ecosystem Context

Conductor-AI is the orchestration hub in a 5-repo ecosystem:

| Repo | Role | Relationship to Conductor |
|------|------|--------------------------|
| **conductor-ai** | Orchestrator | Owns `.wf` runtime, worktree management, agent execution |
| **agent-architecture** | Agent definitions | 22 shared agents loaded via `--plugin-dir` |
| **fsm-engine** | Workflow templates | 15 `.wf` workflows + 5 FSM definitions |
| **pattern-extractor** | EDLC patterns | PE agents invoked by conductor workflows |
| **vantage** | Go SDLC coordination | Consumer of conductor orchestration |

**Ownership rules:**
- `conductor-ai/.conductor/` is **reference only** -- never source agents/schemas/prompts from it
- Canonical agent source: `agent-architecture/`
- Canonical workflow source: `fsm-engine/`
- Per-step `plugin_dirs` in workflows **append** to repo-level `plugin_dirs`

---

## Part VIII: Documentation Audit

### Assessment Summary

| Document | Freshness | Quality |
|----------|-----------|---------|
| README.md | Current | Complete |
| VISION.md | Archival (2026-02-20) | Excellent historical reference |
| ROADMAP.md | Current (2026-03-12) | Actionable |
| PHILOSOPHY.md | Timeless | Excellent |
| getting-started-cli.md | Current | Good |
| claude-agent-integration.md | Current (2026-03-09) | Thorough research |
| BSG_INTEGRATION.md | Current (2026-03-18) | Complete |
| workflow/engine.md | Current | Comprehensive |
| workflow/agent-path-resolution.md | Current | Complete |
| workflow/prompt-snippets.md | Current | Comprehensive |
| workflow/structured-agent-output.md | Current | Design-complete |
| RFCs 001-005 | Draft stage | Well-reasoned |

### Key Gaps

1. **MCP tools reference** -- no user-facing documentation for the 22 MCP tools
2. **Local ticket management** -- recent feature (2026-03-18) not yet documented
3. **Daemon capabilities** -- running but not user-documented
4. **Database schema** -- last documented 2026-02-20, 37 migrations since
5. **Agent definition reference** -- frontmatter fields not comprehensively documented

**Overall documentation quality: 7.5/10** -- strong architectural and design docs, gaps mainly in recently-added features.

---

## Appendix: Key File Paths

| What | Path |
|------|------|
| Workspace root | `/usr/local/bsg/conductor-ai/Cargo.toml` |
| Core library | `conductor-core/src/lib.rs` |
| CLI entry point | `conductor-cli/src/main.rs` |
| MCP server | `conductor-cli/src/mcp.rs` |
| TUI main loop | `conductor-tui/src/app.rs` |
| Web server | `conductor-web/src/main.rs` |
| Daemon | `conductor-service/src/main.rs` |
| Workflow engine | `conductor-core/src/workflow.rs` |
| DSL parser | `conductor-core/src/workflow_dsl.rs` |
| Migrations | `conductor-core/src/db/migrations/` |
| Sample workflows | `.conductor/workflows/` |
| Sample agents | `.conductor/agents/` |
| CI config | `.github/workflows/ci.yml` |
