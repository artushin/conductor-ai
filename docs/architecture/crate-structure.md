---
title: Crate Structure and Dependency Graph
type: explanation
layer: 2
---

# Crate Structure and Dependency Graph

Conductor is a Cargo workspace with five crates. One library crate (`conductor-core`) holds all domain logic; four binary crates provide different interfaces to it. This document explains the workspace architecture, per-crate responsibilities, the module layout within conductor-core, and the design trade-offs behind this structure.

## Workspace Overview

The workspace is defined in the root `Cargo.toml` with `resolver = "2"`:

```
conductor-ai/
├── Cargo.toml              # workspace definition
├── conductor-core/         # library -- ALL domain logic
├── conductor-cli/          # binary -- CLI + MCP server
├── conductor-tui/          # binary -- terminal UI
├── conductor-web/          # binary -- web UI + REST API
└── conductor-service/      # binary -- daemon (early stage)
```

## Crate Dependency Graph

```
conductor-core  <---+--- conductor-cli      (conductor)
                    +--- conductor-tui      (conductor-tui)
                    +--- conductor-web      (conductor-web)
                    +--- conductor-service  (conductor-service)
```

All four binaries depend on conductor-core. No binary depends on another binary. conductor-core has zero workspace dependencies -- it depends only on external crates.

## Per-Crate Responsibility Table

| Crate | Binary Name | ~LoC | Entry Point | Primary Responsibility | Key External Dependencies |
|-------|-------------|------|-------------|------------------------|--------------------------|
| conductor-core | (library) | 32K | `src/lib.rs` | All domain logic: workflow engine, agent management, repo/worktree lifecycle, ticket sync, database layer | rusqlite, serde, thiserror, chrono, ulid, ureq, notify |
| conductor-cli | `conductor` | 2.5K | `src/main.rs` | Thin CLI wrapper (clap subcommands) + MCP server (stdio transport) | clap, rmcp, tokio, anyhow |
| conductor-tui | `conductor-tui` | 65K | `src/main.rs` | Terminal UI: dashboard, repo/worktree/ticket/workflow views, modals | ratatui, crossterm, tui-textarea |
| conductor-web | `conductor-web` | 3K | `src/main.rs` | Web UI: axum REST API + embedded React frontend (Vite + Tailwind) | axum, tokio, rust-embed, tower-http |
| conductor-service | `conductor-service` | 261 | `src/main.rs` | Daemon: Unix socket JSON-RPC, PE checkpoint watcher, PID lock management | tokio, tracing |

## Conductor-Core Module Map

conductor-core is the largest crate (~32K LoC). Its modules group into six domains:

| Domain | Modules | Combined LoC | Purpose |
|--------|---------|-------------|---------|
| Workflow engine | `workflow.rs`, `workflow_dsl.rs`, `workflow_config.rs`, `workflow_ephemeral.rs` | ~15,600 | Custom DSL parser, execution engine, context flow, structured output, ephemeral PR runs |
| Agent management | `agent.rs`, `agent_config.rs`, `agent_runtime.rs`, `orchestrator.rs` | ~6,500 | Agent lifecycle, frontmatter parsing, tmux spawning/polling, parent-child orchestration |
| Ticket management | `tickets.rs`, `issue_source.rs` | ~2,300 | Ticket sync (GitHub/Jira/local), upsert, filtering, worktree linking |
| Repo and worktree | `repo.rs`, `worktree.rs` | ~2,200 | Repo registry CRUD, worktree lifecycle (create, push, PR, delete), branch normalization |
| External integrations | `github.rs`, `github_app.rs`, `jira_acli.rs` | ~1,700 | GitHub API via `gh` CLI, GitHub App JWT auth, Jira CLI integration |
| Config and schema | `config.rs`, `prompt_config.rs`, `schema_config.rs`, `models.rs` | ~2,900 | Config loading, prompt snippet system, JSON schema validation, model tier resolution |
| Infrastructure | `db/`, `error.rs`, `text_util.rs` | ~1,100 | Database open/migrate, `ConductorError` enum (17 variants), frontmatter parsing utilities |

### Key infrastructure details

- **Database**: `db/mod.rs` opens SQLite with WAL mode, foreign keys on, and 5s busy timeout. `db/migrations.rs` runs a versioned migration sequence (37 migrations). See [database schema reference](../reference/database-schema.md) for the complete schema.
- **Error handling**: `ConductorError` (via `thiserror`) provides structured error variants. Binary crates use `anyhow::Result` at the top level for one-shot error reporting.
- **Manager pattern**: Domain logic is organized into manager structs that take `&Connection` + `&Config`. Each manager owns one domain (repos, worktrees, tickets, agents, workflows). Managers never own the connection -- callers control connection lifetime.

## How the Crates Compose

### conductor-cli

The primary interface for automation and scripting. Provides seven top-level subcommand groups:

| Subcommand | Delegates to | Purpose |
|------------|-------------|---------|
| `repo` | `RepoManager` | Register, list, remove repos; set plugin dirs |
| `worktree` | `WorktreeManager` | Create, push, PR, delete worktrees |
| `tickets` | `TicketSyncer` | Sync, list, link tickets |
| `agent` | `AgentManager` | Run agents in tmux, track runs |
| `workflow` | `WorkflowManager` | Run, resume, cancel workflows; validate .wf files |
| `statusline` | (statusline module) | Claude Code status line integration |
| `mcp` | (mcp module) | MCP server: 22 tools + 9 resource categories |

The MCP server (`conductor mcp serve`) runs on stdio transport for Claude Code integration. Each MCP request opens a fresh SQLite connection via `spawn_blocking` to avoid blocking the tokio runtime.

### conductor-tui

A synchronous single-thread ratatui application. Opens one SQLite connection at startup and polls the database for updates. All blocking operations (git, network, subprocess) run on background `std::thread::spawn` threads, communicating results back via an action channel.

The TUI has a strict threading invariant: never call blocking operations on the main thread. The pattern is: (1) capture data, (2) show progress modal, (3) spawn thread, (4) handle result action.

### conductor-web

An async axum server with an embedded React frontend. Uses `Arc<Mutex<Connection>>` for shared database access across async handlers. Reaps orphaned agent runs, stale worktrees, and orphaned workflow runs on startup. Provides SSE (Server-Sent Events) for real-time UI updates.

### conductor-service

An early-stage daemon providing:

- Unix domain socket listener at `~/.conductor/conductor.sock`
- JSON-RPC 2.0 protocol (newline-delimited) with three methods: `ping`, `status`, `pe.checkpoint.status`
- PE checkpoint file watcher (polls every 5s)
- PID file lock with stale process detection

## Ecosystem Integration

conductor-cli is the primary entry point for the BSG ecosystem. Other tools invoke it as follows:

| Consumer | Invocation | Purpose |
|----------|-----------|---------|
| Workflow .wf files | `conductor workflow run` | Execute multi-step workflows |
| Claude Code sessions | `conductor mcp serve` | Provide MCP tools and resources |
| BSG agent-architecture | `conductor repo set-plugin-dirs` | Register agent plugin directories |
| Pattern-extractor | `conductor workflow run` + PE agents | Pattern discovery and implementation |
| vantage | Managed by conductor workflows | SDLC coordination |

For ecosystem ownership rules, see [BSG_INTEGRATION.md](../BSG_INTEGRATION.md).

## Design Trade-Offs

### Library-first architecture

**GAIN**: All domain logic testable without binary entry points. Any new interface (daemon, WASM, etc.) imports conductor-core without duplication. SQLite WAL mode handles concurrent access from CLI + TUI + web without IPC.

**COST**: No shared in-memory state across binaries. Each binary opens its own SQLite connection. The TUI must poll the database for changes rather than receiving push notifications. conductor-service was added later to address this gap but is still early-stage.

### Single library crate (conductor-core)

**GAIN**: Simple dependency graph. All domain logic co-located -- managers can reference each other's types directly. Refactoring moves code within one crate, not across crate boundaries.

**COST**: Compile times scale with the full crate (~32K LoC). No fine-grained dependency control -- binaries that need only `RepoManager` still compile the full workflow engine. As the crate grows, internal module boundaries become the primary modularity mechanism.

### Synchronous subprocess model

**GAIN**: Simple mental model. Git operations, `gh` CLI calls, and agent spawning use `std::process::Command` -- no async complexity, no callback chains. Agent processes run in tmux windows and are polled for completion.

**COST**: Blocking calls require careful threading in the TUI. The MCP server needs `spawn_blocking` to wrap synchronous core calls. Future high-throughput scenarios (many concurrent agent runs) may require an async rewrite of core operations.

## Related Documents

- [Database Schema Reference](../reference/database-schema.md) -- complete table schema from 37 migrations
- [BSG_INTEGRATION.md](../BSG_INTEGRATION.md) -- cross-repo ownership boundaries and integration patterns
- [Workflow Engine Architecture](workflow-engine.md) -- design decisions for the workflow DSL and execution engine
- [Agent Execution Architecture](agent-execution.md) -- agent lifecycle, tmux spawning, orchestration
- For ecosystem context, see [/usr/local/bsg/CLAUDE.md](/usr/local/bsg/CLAUDE.md)
