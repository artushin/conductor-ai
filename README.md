# Conductor

A local-first orchestration tool for managing multiple git repos, worktrees, tickets, and AI agent runs. Multi-step workflows are defined in a custom `.wf` DSL and executed by a workflow engine that spawns Claude agents in tmux windows. All state is stored in a single SQLite database.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [GitHub CLI (`gh`)](https://cli.github.com/) -- installed and authenticated
- [tmux](https://github.com/tmux/tmux) -- for agent sessions
- `ANTHROPIC_API_KEY` set in your shell

### Build and Install

```bash
./build.sh                            # Full build (frontend + all crates)
cargo install --path conductor-cli    # Install CLI
cargo install --path conductor-tui    # Install TUI
cargo install --path conductor-web    # Install web UI (requires ./build.sh first)
```

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
