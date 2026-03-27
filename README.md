# Conductor

A local-first orchestration tool for managing multiple git repos, worktrees, tickets, and AI agent runs. Multi-step workflows are defined in a custom `.wf` DSL and executed by a workflow engine that spawns Claude agents in tmux windows. All state is stored in a single SQLite database.

## What is Conductor?

Conductor is a local tool for managing AI-assisted development across multiple git worktrees. If you're already using Claude to write and review code, Conductor is the layer that keeps everything organized when you're running multiple agents in parallel.

**The problem it solves:**

When you let an agent work on a branch, you don't want to sit and watch it. You want to kick off the work and move on to something else. But the moment you have two or three agents running at once — each on its own worktree — the overhead of managing them adds up fast: tracking which terminal has which branch, knowing when something needs your attention, keeping your ticket tracker in sync with what's actually happening in git.

Conductor handles that overhead. It gives you one place to see all your repos, worktrees, and in-flight work, and a workflow system for defining how agents should handle common tasks (fix-ci, review-pr, iterate-pr, etc.) so you're not copy-pasting prompts.

**Key things it does:**
- Manages git worktrees for you — create, push, PR, delete — with branch naming handled automatically
- Runs agent workflows (pre-defined sequences of Claude tasks) against a worktree, PR, or ticket with a single keypress
- Syncs GitHub issues so your tickets live next to your code, not in a separate browser tab
- Lets multiple workflows run in parallel without you babysitting them

**Interfaces:** The primary interface is a TUI (terminal UI), with a CLI for scripting. There's also a web app and a Mac app — both are usable today but still being refined, so if you're not a TUI person they're worth trying.

**What it is:** Local-first, no cloud, no account. Runs on your machine, your Claude API key stays yours.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (for the web UI frontend)
- [GitHub CLI (`gh`)](https://cli.github.com/) — installed and authenticated (for GitHub issue sync)
- [tmux](https://github.com/tmux/tmux) (for AI agent sessions)
- [Claude Code CLI (`claude`)](https://docs.anthropic.com/en/docs/claude-code) — installed and authenticated

### First Commands

```bash
conductor repo register https://github.com/acme/my-app
conductor repo set-plugin-dirs my-app /usr/local/bsg/agent-architecture/planner
conductor worktree create my-app feat-my-feature
conductor workflow run ticket-to-pr my-app feat-my-feature --input ticket_id=42
```

## Features

- **Repo management** -- register repos, configure plugin directories, sync tickets from GitHub/Jira
- **Worktree lifecycle** -- create, push, PR, delete with ticket linking and branch naming conventions
- **Workflow engine** -- custom `.wf` DSL with sequential calls, parallel blocks, conditionals, loops, gates, and sub-workflow composition
- **AI agent execution** -- spawn Claude agents in tmux windows with structured output, retries, and schema validation
- **MCP server** -- 22 tools + 9 resource categories for Claude Code integration via `conductor mcp serve`
- **Multiple interfaces** -- CLI, terminal UI (ratatui), and web UI (axum + React)

## Crate Architecture

| Crate | Binary | Purpose |
|-------|--------|---------|
| conductor-core | (library) | All domain logic (~32K LoC): workflow engine, agent management, repos, worktrees, tickets, DB |
| conductor-cli | `conductor` | CLI (clap subcommands) + MCP server (stdio transport) |
| conductor-tui | `conductor-tui` | Terminal UI (ratatui + crossterm) |
| conductor-web | `conductor-web` | Web UI (axum + embedded React frontend) |
| conductor-service | `conductor-service` | Daemon: Unix socket JSON-RPC, PE watcher (early stage) |

All binaries depend on conductor-core. No binary-to-binary dependencies. Data lives in `~/.conductor/` -- a single SQLite database and per-repo worktree directories.

For design decisions and trade-offs, see [docs/architecture/crate-structure.md](docs/architecture/crate-structure.md).

## Documentation

| Section | Contents |
|---------|----------|
| [docs/architecture/](docs/architecture/) | Crate structure, workflow engine, structured output, agent execution |
| [docs/reference/](docs/reference/) | Workflow DSL syntax, MCP tools, CLI commands, DB schema, output schemas, configuration |
| [docs/how-to/](docs/how-to/) | Write workflows, manage repos, use worktrees |
| [docs/README.md](docs/README.md) | Full documentation index with quick navigation |

### Desktop (macOS)

Native Mac app powered by Tauri, embedding the same web UI. Still being refined but usable today.

```bash
# Dev mode (hot-reload frontend, run from workspace root)
cd conductor-desktop && cargo tauri dev

# Production build — outputs Conductor.app
cd conductor-desktop && cargo tauri build
# App bundle: target/release/bundle/macos/Conductor.app
```

## Workflows

Workflows orchestrate multi-step AI agent pipelines in `.wf` files:

```bash
conductor workflow list                                  # Available workflows
conductor workflow validate <name> <repo> <worktree>     # Validate before running
conductor workflow run <name> <repo> <wt> [--dry-run]    # Execute a workflow
conductor workflow run-show <run-id>                     # Step-by-step detail
```

For syntax reference, see [docs/reference/workflow-dsl.md](docs/reference/workflow-dsl.md). For a step-by-step authoring guide, see [docs/how-to/write-workflow.md](docs/how-to/write-workflow.md).

## MCP Server

```bash
conductor mcp serve    # Start MCP server (stdio transport for Claude Code)
```

Exposes 22 tools covering repos, worktrees, tickets, workflows, runs, gates, and agents. See [docs/reference/mcp-tools.md](docs/reference/mcp-tools.md).

## Related Repositories

| Repo | Purpose | Relationship |
|------|---------|-------------|
| [agent-architecture](/usr/local/bsg/agent-architecture/) | 31 shared agent definitions | Loaded via `--plugin-dir` |
| [fsm-engine](/usr/local/bsg/fsm-engine/) | Workflow templates + FSM definitions | Owns `.wf` templates |
| [pattern-extractor](/usr/local/bsg/pattern-extractor/) | EDLC pattern pipeline | PE commands invoked by workflows |
| [vantage](/usr/local/bsg/vantage/) | Go SDLC coordination | Primary consumer of conductor |

## License

MIT
