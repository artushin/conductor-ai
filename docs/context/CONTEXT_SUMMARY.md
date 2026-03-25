---
title: Conductor-AI Documentation Context Summary
type: reference
layer: 1.5
---

# Conductor-AI Documentation Context Summary

Load this file after CLAUDE.md to understand what documentation exists for conductor-ai and where to find specific information. Conductor is a Cargo workspace (5 crates, ~32K LoC in conductor-core) providing multi-repo orchestration: git worktrees, ticket sync, workflow engine, AI agent execution, and MCP tools -- all backed by SQLite.

## Document Index

### Architecture (`docs/architecture/`)

| Document | Key Topics | Lines |
|----------|-----------|-------|
| [crate-structure.md](../architecture/crate-structure.md) | Workspace layout, 5 crates, module map, manager pattern, dependency graph, trade-offs | ~144 |
| [workflow-engine.md](../architecture/workflow-engine.md) | Execution pipeline, parsing, validation, node dispatch, conditions, loops, gates, composition, resumability | ~324 |
| [structured-output.md](../architecture/structured-output.md) | CONDUCTOR_OUTPUT design, schema ownership, prompt injection, validation pipeline, alternatives | ~172 |
| [agent-execution.md](../architecture/agent-execution.md) | Subprocess model, tmux spawning, agent resolution, prompt assembly, polling, orchestration | ~227 |

### Reference (`docs/reference/`)

| Document | Key Topics | Lines |
|----------|-----------|-------|
| [workflow-dsl.md](../reference/workflow-dsl.md) | Grammar spec, node types (call, parallel, gate, while, always), variable substitution, snippets | ~464 |
| [database-schema.md](../reference/database-schema.md) | 13 tables, 37 migrations, relationship diagram, column specs, migration history | ~348 |
| [mcp-tools.md](../reference/mcp-tools.md) | 22 MCP tools, 9 resource categories, parameter tables, conductor:// URI scheme | ~303 |
| [cli-commands.md](../reference/cli-commands.md) | 7 subcommand groups (repo, worktree, tickets, agent, workflow, statusline, mcp) | ~324 |
| [output-schemas.md](../reference/output-schemas.md) | Schema YAML format, field types, validation rules, marker derivation, resolution order | ~277 |
| [configuration.md](../reference/configuration.md) | config.toml fields, data directory layout, model resolution order, plugin dir resolution | ~152 |

### How-To (`docs/how-to/`)

| Document | Key Topics | Lines |
|----------|-----------|-------|
| [write-workflow.md](../how-to/write-workflow.md) | Create .wf files, add agents, control flow, schemas, validate, dry-run | ~256 |
| [manage-repos.md](../how-to/manage-repos.md) | Register repos, set plugin dirs, configure settings, issue sources | ~155 |
| [use-worktrees.md](../how-to/use-worktrees.md) | Create, push, PR, delete, purge worktrees; branch naming; model overrides | ~181 |

### Other Docs

| Document | Type | Key Topics |
|----------|------|-----------|
| [BSG_INTEGRATION.md](../BSG_INTEGRATION.md) | Integration | Cross-repo binding, daemon, PE agents, BSG workflows |
| [workflow/agent-path-resolution.md](../workflow/agent-path-resolution.md) | Explanation | Agent resolution design, alternatives analysis |
| [workflow/prompt-snippets.md](../workflow/prompt-snippets.md) | Explanation | Prompt snippet system, resolution, composition |
| [workflow/multi-runtime-agents.md](../workflow/multi-runtime-agents.md) | Explanation | Multi-runtime agent idea (draft) |
| [rfc/001-005](../rfc/) | RFC | 5 forward-looking proposals (obligation tracker, workflow targets, path flag, Claude Code experience, diagram workflows) |

## Architecture Summary

Conductor follows a library-first architecture: `conductor-core` (~32K LoC) contains all domain logic organized into manager structs (`RepoManager`, `WorktreeManager`, `TicketSyncer`, `AgentManager`, `WorkflowManager`) that take `&Connection` + `&Config`. Four binary crates provide different interfaces (CLI, TUI, web, daemon), each depending only on conductor-core.

The workflow engine (~15.6K LoC) is the largest subsystem. It parses `.wf` files via a hand-written recursive descent parser, snapshots the AST to the database, and executes by spawning agents in tmux windows. Agents communicate results back through the database using `CONDUCTOR_OUTPUT` blocks (markers + context). Structured output schemas extend this with type-safe JSON validation and automatic marker derivation.

Key data flow: `.wf` file -> parse -> validate -> snapshot -> execute -> poll agents -> parse output -> evaluate conditions -> thread context -> next step.

## Surgical Reading Directives

For deep-dive topics, use these grep targets:

- For the manager pattern: grep `## Conductor-Core Module Map` in `docs/architecture/crate-structure.md`, read ~15 lines
- For workflow node dispatch: grep `## Node dispatch` in `docs/architecture/workflow-engine.md`, read ~20 lines
- For agent spawning details: grep `## Spawning` in `docs/architecture/agent-execution.md`, read ~25 lines
- For the .wf grammar: grep `## Grammar` in `docs/reference/workflow-dsl.md`, read ~35 lines
- For MCP tool list: grep `## Tool Summary` in `docs/reference/mcp-tools.md`, read ~30 lines
- For DB table relationships: grep `## Table Relationship Diagram` in `docs/reference/database-schema.md`, read ~20 lines
- For config.toml fields: grep `## .config.toml. Fields` in `docs/reference/configuration.md`, read ~40 lines
- For model resolution: grep `## Model Resolution Order` in `docs/reference/configuration.md`, read ~10 lines

## Quick Navigation

| I need to... | Go to |
|--------------|-------|
| Understand the crate layout | [architecture/crate-structure.md](../architecture/crate-structure.md) |
| Write a workflow | [how-to/write-workflow.md](../how-to/write-workflow.md) |
| Look up .wf syntax | [reference/workflow-dsl.md](../reference/workflow-dsl.md) |
| See MCP tool parameters | [reference/mcp-tools.md](../reference/mcp-tools.md) |
| Find a CLI command | [reference/cli-commands.md](../reference/cli-commands.md) |
| Register a repo | [how-to/manage-repos.md](../how-to/manage-repos.md) |
| Manage worktrees | [how-to/use-worktrees.md](../how-to/use-worktrees.md) |
| Check the DB schema | [reference/database-schema.md](../reference/database-schema.md) |
| Configure conductor | [reference/configuration.md](../reference/configuration.md) |
| Understand output schemas | [reference/output-schemas.md](../reference/output-schemas.md) |
| See how agents execute | [architecture/agent-execution.md](../architecture/agent-execution.md) |
| Understand the workflow engine | [architecture/workflow-engine.md](../architecture/workflow-engine.md) |
| Check BSG integration | [BSG_INTEGRATION.md](../BSG_INTEGRATION.md) |

## Cross-Repo Context

| Repo | Relationship | Path |
|------|-------------|------|
| agent-architecture | Provides 31 shared agent definitions loaded via `--plugin-dir` | `/usr/local/bsg/agent-architecture/` |
| fsm-engine | Provides .wf workflow templates and FSM definitions | `/usr/local/bsg/fsm-engine/` |
| pattern-extractor | PE commands invoked by PE agents in workflows | `/usr/local/bsg/pattern-extractor/` |
| vantage | Primary consumer: SDLC coordination managed by conductor workflows | `/usr/local/bsg/vantage/` |

**Ownership rule**: conductor-ai owns the `.wf` execution runtime. It does NOT own agent definitions (agent-architecture) or workflow templates (fsm-engine). The `.conductor/` directory in conductor-ai is reference only -- never the canonical source.

For ecosystem-level context, see [/usr/local/bsg/CLAUDE.md](/usr/local/bsg/CLAUDE.md).
