# Conductor-AI

Multi-repo orchestration tool: manages git repos, worktrees, tickets, AI agent runs, and multi-step workflows -- all backed by SQLite. Five Rust crates in a Cargo workspace.

## Documentation Access -- Librarian-First (Mandatory)

NEVER read `docs/` files directly via the Read tool when the doc-librarian agent is available. ALWAYS use the doc-librarian to gather documentation context:

- **Session start**: Invoke the doc-librarian (`subagent_type: "doc-librarian:doc-librarian"`) to assemble context relevant to the user's first prompt BEFORE doing any work
- **Subsequent prompts**: If there is any chance that unread documentation has more detail, invoke the doc-librarian again to dig deeper rather than reading docs directly
- **Why**: The doc-librarian navigates progressive disclosure layers (L1 summaries -> L2 domain -> L3 surgical) and knows which documents are relevant. Direct reads bypass this topology, loading unnecessary content and missing related documents
- **Fallback**: Direct reads are permitted ONLY when the doc-librarian agent is not loaded in the current session
- **Conductor workflow exemption**: If `CONDUCTOR_RUN_ID` is set in the environment, skip doc-librarian/doc-writer -- the workflow handles documentation context via dedicated bookend steps. Focus on your assigned task; direct `docs/` reads are permitted for precise facts

## Build and Test

```bash
cargo build                    # Build all crates
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test -p conductor-core   # Test a single crate
cargo clippy -- -D warnings    # Lint (CI enforces -D warnings)
cargo fmt --all                # Format
cargo fmt --all --check        # Check formatting (CI gate)

# Web frontend (required before building conductor-web)
cd conductor-web/frontend && bun install && bun run build

# Full build (frontend + all crates)
./build.sh
```

Pre-commit hook: `git config core.hooksPath .githooks`

# One-time dev setup: enable git hooks (pre-commit fmt check + pre-push E2E tests)
git config core.hooksPath .githooks

# Playwright E2E tests run automatically on push when conductor-web files change.
# To skip: SKIP_E2E=1 git push
# To run manually: cd conductor-web/frontend && bun run test:e2e

## Architecture

All binaries depend on conductor-core. No binary-to-binary dependencies. For the full module map and design trade-offs, see [docs/architecture/crate-structure.md](docs/architecture/crate-structure.md).

## Key Commands

```bash
conductor repo register <url>                         # Register a repo
conductor repo set-plugin-dirs <slug> <dirs...>       # Set agent plugin dirs
conductor worktree create <repo> <name>               # Create a worktree
conductor tickets sync [<repo>]                       # Sync tickets
conductor tickets create <repo> --title "..." [--workflow <name>]  # Create ticket with optional workflow override
conductor tickets update <id> [--workflow <name>]     # Update ticket (set explicit workflow)
conductor workflow run <name> <repo> <wt> [--dry-run] [--background]  # Run a workflow
conductor workflow validate <name> <repo> <wt>        # Validate a workflow
conductor mcp serve                                   # Start MCP server
```

**`--background` flag**: `conductor workflow run --background` forks a detached child process via `fork`/`setsid`, prints the run ID to stdout, and exits immediately. The child drives the workflow to completion independently. Use this when the spawning process should not block on workflow execution.

**`--workflow` flag on tickets**: `conductor tickets create --workflow <name>` and `conductor tickets update --workflow <name>` set an explicit workflow override on the ticket. When set, dispatch uses this workflow directly instead of routing heuristics.

Full CLI reference: [docs/reference/cli-commands.md](docs/reference/cli-commands.md)

## MCP Server

22 tools + 9 resource categories exposed via `conductor mcp serve` (stdio transport for Claude Code). Covers repos, worktrees, tickets, workflows, runs, gates, and agent management.

Full reference: [docs/reference/mcp-tools.md](docs/reference/mcp-tools.md)

## Workflow Engine

Custom `.wf` DSL parsed by a hand-written recursive descent parser. Supports `call`, `parallel`, `if`/`unless`/`while`/`do-while`, `gate`, `always`, and `call workflow` for shallow composition. Agents emit structured output via `CONDUCTOR_OUTPUT` blocks.

- DSL syntax: [docs/reference/workflow-dsl.md](docs/reference/workflow-dsl.md)
- Engine architecture: [docs/architecture/workflow-engine.md](docs/architecture/workflow-engine.md)
- Output schemas: [docs/reference/output-schemas.md](docs/reference/output-schemas.md)
- Writing workflows: [docs/how-to/write-workflow.md](docs/how-to/write-workflow.md)

## Database

SQLite at `~/.conductor/conductor.db`. WAL mode, foreign keys on, 5s busy timeout. 13 tables, 37 versioned migrations. IDs are ULIDs; timestamps are ISO 8601 strings.

Full schema: [docs/reference/database-schema.md](docs/reference/database-schema.md)

## CI

Reference implementations already using this pattern correctly:
- `has_merged_pr()` check before worktree delete (`conductor-tui/src/app/crud_operations.rs`)
- Workflow execution (`conductor-tui/src/app/workflow_management.rs` — `spawn_workflow_in_background`)
- Workflow resume (`conductor-tui/src/app/workflow_management.rs` — `handle_resume_workflow`)
- PR fetch background task (`conductor-tui/src/background.rs`)

## Worktree Workflow (REQUIRED)

**Always create a conductor worktree before starting any fix or feature.** Never make changes directly on `main` or in the primary working directory.

```bash
# Create a worktree (branch auto-normalizes: feat- → feat/, fix- → fix/)
cargo run --bin conductor -- worktree create conductor-ai <name>
# e.g. cargo run --bin conductor -- worktree create conductor-ai fix-800-snapshot-crash
#      cargo run --bin conductor -- worktree create conductor-ai feat-801-new-thing

# Worktree lands at:
~/.conductor/workspaces/conductor-ai/<name>/
```

Do all work — edits, builds, tests, commits — inside the worktree directory. Push and create the PR from there.

```bash
cd ~/.conductor/workspaces/conductor-ai/<name>
# ... make changes, run cargo test, cargo fmt --all ...
git add <files> && git commit -m "..."
git push -u origin <branch>
gh pr create ...
```

## Key Constraints

- **conductor-ai is reference only** -- NEVER source agents, schemas, or prompts from `conductor-ai/.conductor/`. Canonical sources: [agent-architecture](https://github.com/devinrosen/agent-architecture) (agents), [fsm-engine](/usr/local/bsg/fsm-engine/) (workflows, schemas, prompts), target repo (repo-local agents)
- **TUI threading rule** -- Never call blocking operations on the TUI main thread. Pattern: capture data, show progress modal, `std::thread::spawn`, handle result action. See [docs/architecture/crate-structure.md](docs/architecture/crate-structure.md) for details
- **Manager pattern** -- Domain logic uses manager structs taking `&Connection` + `&Config`. Managers never own the connection
- **No daemon in v1** -- CLI and TUI import conductor-core directly. SQLite WAL handles concurrency

## Documentation

For a compressed context summary, load [docs/context/CONTEXT_SUMMARY.md](docs/context/CONTEXT_SUMMARY.md).

| Section | Documents |
|---------|-----------|
| Architecture | [crate-structure](docs/architecture/crate-structure.md), [workflow-engine](docs/architecture/workflow-engine.md), [structured-output](docs/architecture/structured-output.md), [agent-execution](docs/architecture/agent-execution.md) |
| Reference | [workflow-dsl](docs/reference/workflow-dsl.md), [database-schema](docs/reference/database-schema.md), [mcp-tools](docs/reference/mcp-tools.md), [cli-commands](docs/reference/cli-commands.md), [output-schemas](docs/reference/output-schemas.md), [configuration](docs/reference/configuration.md) |
| How-To | [write-workflow](docs/how-to/write-workflow.md), [manage-repos](docs/how-to/manage-repos.md), [use-worktrees](docs/how-to/use-worktrees.md) |
| Integration | [BSG_INTEGRATION](docs/BSG_INTEGRATION.md) |
| Full index | [docs/README.md](docs/README.md) |

## Claude Code Skills

7 skills in `.claude/skills/`: cli-coverage, conductor-workflow-create, conductor-workflow-init, conductor-workflow-update, conductor-workflow-validate, rebase-and-fix-review, tui-keys.

## Related Repos

| Repo | Relationship |
|------|-------------|
| [agent-architecture](/usr/local/bsg/agent-architecture/) | 31 shared agents loaded via `--plugin-dir` |
| [fsm-engine](/usr/local/bsg/fsm-engine/) | .wf workflow templates + FSM definitions |
| [pattern-extractor](/usr/local/bsg/pattern-extractor/) | PE commands invoked by PE agents |
| [vantage](/usr/local/bsg/vantage/) | Primary consumer: SDLC coordination |
