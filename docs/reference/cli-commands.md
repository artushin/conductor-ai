---
title: CLI Commands Reference
type: reference
layer: 3
---

# CLI Commands Reference

Complete reference for the `conductor` CLI binary (`conductor-cli` crate). The CLI is a thin clap wrapper over `conductor-core` managers. Binary name: `conductor`.

## Top-Level Commands

| Command | Description |
|---------|-------------|
| `repo` | Manage registered repositories |
| `worktree` | Manage git worktrees |
| `tickets` | Manage tickets (sync, list, link, create) |
| `agent` | Run Claude agents (internal, used by tmux spawner) |
| `workflow` | Multi-step workflow engine |
| `statusline` | Claude Code status line integration |
| `mcp` | Model Context Protocol server |

## `conductor repo`

### `repo register <remote_url>`

Register a repository with conductor.

| Flag | Description |
|------|-------------|
| `--slug <SLUG>` | Short slug (auto-derived from URL if omitted) |
| `--local-path <PATH>` | Existing checkout path (skips clone) |
| `--workspace <DIR>` | Workspace directory for worktrees |

### `repo list`

List all registered repositories. No flags.

### `repo discover [<owner>]`

Discover repos from GitHub. Omit `<owner>` to list available orgs; pass an org name to list its repos. Requires `gh` CLI.

### `repo unregister <slug>`

Unregister a repository by slug.

### `repo set-model <slug> [<model>]`

Set or clear the per-repo default model (e.g., `"sonnet"`, `"claude-opus-4-6"`). Omit `<model>` to clear.

### `repo set-plugin-dirs <slug> <dirs...>`

Set plugin directories for a repo. Paths must be absolute. Passed as `--plugin-dir` to all agent sessions in this repo.

### `repo allow-agent-issues <slug>`

Allow or disallow agents to create GitHub issues for a repo.

| Flag | Default | Description |
|------|---------|-------------|
| `--allow <bool>` | `true` | `true` to allow, `false` to disallow |

### `repo sources add <slug>`

Add an issue source to a repo.

| Flag | Description |
|------|-------------|
| `--type <TYPE>` | Source type: `github` or `jira` |
| `--config <JSON>` | JSON config (auto-inferred for GitHub from remote URL if omitted) |

### `repo sources list <slug>`

List issue sources for a repo.

### `repo sources remove <slug>`

Remove an issue source.

| Flag | Description |
|------|-------------|
| `--type <TYPE>` | Source type to remove: `github` or `jira` |

## `conductor worktree`

### `worktree create <repo> <name>`

Create a new worktree (git branch + working directory).

| Flag | Description |
|------|-------------|
| `-f`, `--from <BRANCH>` | Base branch (conflicts with `--from-pr`) |
| `--from-pr <NUMBER>` | Checkout existing PR branch by number (conflicts with `--from`) |
| `--ticket <ID>` | Link to a ticket ID |
| `--auto-agent` | Auto-start an agent after creation (requires `--ticket`) |

### `worktree list [<repo>]`

List worktrees. Filter by repo slug if provided.

### `worktree delete <repo> <name>`

Soft-delete a worktree (marks as merged or abandoned).

### `worktree purge <repo> [<name>]`

Permanently remove completed worktree records. Purges all completed if `<name>` is omitted.

### `worktree push <repo> <name>`

Push worktree branch to origin.

### `worktree pr <repo> <name>`

Create a pull request for the worktree branch.

| Flag | Description |
|------|-------------|
| `--draft` | Create as draft PR |

### `worktree set-model <repo> <name> [<model>]`

Set or clear the per-worktree default model. Omit `<model>` to clear.

## `conductor tickets`

### `tickets sync [<repo>]`

Sync tickets from configured issue sources. Syncs all repos if `<repo>` is omitted.

### `tickets get <ticket>`

Show full details for a single ticket. Accepts ULID or source ID.

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |

### `tickets list [<repo>]`

List cached tickets. Filter by repo slug if provided.

### `tickets link <ticket> <repo> <worktree>`

Link a ticket to a worktree by source ID, repo slug, and worktree slug.

### `tickets stats [<repo>]`

Show aggregate agent cost, turns, and time per ticket.

### `tickets create`

Create a new local ticket.

| Flag | Required | Description |
|------|----------|-------------|
| `--repo <SLUG>` | Yes | Repository slug |
| `--title <TEXT>` | Yes | Ticket title |
| `--body <TEXT>` | No | Ticket body (default: empty) |
| `--priority <LEVEL>` | No | Priority level |
| `--label <NAME>` | No | Labels (repeatable) |

### `tickets upsert <repo>`

Create or update a ticket from an external source.

| Flag | Required | Description |
|------|----------|-------------|
| `--source-type <TYPE>` | Yes | Source type (e.g., `"github"`, `"jira"`, `"linear"`) |
| `--source-id <ID>` | Yes | Source ID (e.g., issue number or key) |
| `--title <TEXT>` | Yes | Ticket title |
| `--state <STATE>` | Yes | Ticket state: `open`, `in_progress`, or `closed` |
| `--body <TEXT>` | No | Ticket body (default: empty) |
| `--url <URL>` | No | URL to the ticket (default: empty) |
| `--labels <CSV>` | No | Comma-separated labels |
| `--assignee <NAME>` | No | Assignee |
| `--priority <LEVEL>` | No | Priority level |
| `--workflow <NAME>` | No | Workflow name override (bypasses routing heuristics) |
| `--agent-map <JSON>` | No | Agent map JSON (validated against AgentMap schema; empty string clears) |

### `tickets update <ticket>`

Update an existing ticket by ULID or source ID.

| Flag | Description |
|------|-------------|
| `--title <TEXT>` | New title |
| `--body <TEXT>` | New body |
| `--state <STATE>` | New state (e.g., `"open"`, `"closed"`, `"done"`) |
| `--priority <LEVEL>` | New priority |
| `--workflow <NAME>` | Workflow name override (empty string clears) |
| `--agent-map <JSON>` | Agent map JSON (validated against AgentMap schema; empty string clears) |

## `conductor workflow`

### `workflow run <name> [<repo> <worktree>]`

Run a workflow. Positional `<repo>` and `<worktree>` are required unless a targeting flag is used. The command is synchronous -- it blocks until the workflow completes (or fails). There is no `--background` flag; for gate-based workflows the CLI blocks in a polling loop.

| Flag | Description |
|------|-------------|
| `--pr <URL_OR_NUM>` | Target a GitHub PR (resolves linked worktree). Conflicts with positional args |
| `--repo <SLUG>` | Run a repo-targeted workflow without a worktree. Conflicts with positional args |
| `--workflow-run <ID>` | Target a prior workflow run (e.g., for postmortem). Conflicts with positional args |
| `--ticket <ID>` | Target a ticket by ULID. Conflicts with positional args |
| `--model <MODEL>` | Model override for agent steps |
| `--dry-run` | Dry-run mode: no commits, no pushes, gates auto-approved |
| `--no-fail-fast` | Continue past step failures |
| `--step-timeout-secs <N>` | Step timeout in seconds (default: 604800 = 1 week) |
| `--input <KEY=VALUE>` | Input variables (repeatable) |

### `workflow runs <repo> [<worktree>]`

List workflow run history for a repo. Optional worktree filter.

**Known bug**: This command queries the `agent_runs` table instead of `workflow_runs`, so it returns empty results for repos that have workflow runs but no standalone agent runs. A fix is in progress.

### `workflow run-show <id>` (alias: `show`)

Show details of a workflow run by ID.

### `workflow list [<repo> <worktree>]`

List available workflow definitions.

| Flag | Description |
|------|-------------|
| `--path <DIR>` | Path to repo root (skips DB lookup). Conflicts with positional args |

### `workflow validate <name> [<repo> <worktree>]`

Validate a workflow definition: checks agents, snippets, cycles, semantic rules.

| Flag | Description |
|------|-------------|
| `--path <DIR>` | Path to repo root (skips DB lookup) |

### `workflow resume <id>`

Resume a failed or stalled workflow run.

| Flag | Description |
|------|-------------|
| `--from-step <NAME>` | Resume from a specific step (re-runs from that step onward) |
| `--model <MODEL>` | Model override for resumed agent steps |
| `--restart` | Restart from the beginning (reuses same run record) |

### `workflow cancel <id>`

Cancel a running or waiting workflow.

### `workflow gate-approve <run_id>`

Approve a pending human gate.

### `workflow gate-reject <run_id>`

Reject a pending human gate (fails the workflow).

### `workflow gate-feedback <run_id> <feedback>`

Provide feedback text and approve a pending human gate.

### `workflow purge`

Delete completed, failed, and cancelled workflow runs.

| Flag | Description |
|------|-------------|
| `--repo <SLUG>` | Only purge runs for this repo |
| `--status <STATUS>` | Filter: `completed`, `failed`, `cancelled`, `all` (default: all terminal) |
| `--dry-run` | Print what would be deleted without deleting |

## `conductor agent`

Internal commands used by the tmux spawner. Not intended for direct user invocation.

### `agent run`

Run a Claude agent for a worktree.

| Flag | Description |
|------|-------------|
| `--run-id <ID>` | Agent run ID (from `agent_runs` table) |
| `--worktree-path <PATH>` | Worktree directory path |
| `--prompt <TEXT>` | Prompt text (use `--prompt-file` for large prompts) |
| `--prompt-file <FILE>` | Read prompt from file (avoids OS arg length limits) |
| `--resume <SESSION>` | Resume a previous Claude session |
| `--model <MODEL>` | Model override |
| `--bot-name <NAME>` | Named GitHub App bot identity (matches `[github.apps.<name>]` in config) |
| `--plugin-dir <DIR>` | Additional plugin directories (repeatable) |

### `agent orchestrate`

Spawn child agents for each plan step of a parent agent run.

| Flag | Description |
|------|-------------|
| `--run-id <ID>` | Parent agent run ID |
| `--worktree-path <PATH>` | Worktree directory |
| `--model <MODEL>` | Model for child agents |
| `--fail-fast` | Stop on first child failure |
| `--child-timeout-secs <N>` | Child timeout (default: 604800) |

### `agent create-issue`

Create a GitHub issue (called by agents during a run).

| Flag | Description |
|------|-------------|
| `--title <TEXT>` | Issue title |
| `--body <TEXT>` | Issue body |
| `--run-id <ID>` | Agent run ID (defaults to `$CONDUCTOR_RUN_ID` env var) |

## `conductor statusline`

### `statusline install`

Install the conductor status line into Claude Code.

### `statusline uninstall`

Uninstall the conductor status line from Claude Code.

## `conductor mcp`

### `mcp serve`

Start the MCP server on stdio transport for Claude Code integration. See [MCP Tools Reference](mcp-tools.md) for the 22 tools and 9 resource categories exposed.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `RUST_LOG` | Logging level (default: `info`). Standard `tracing_subscriber` filter syntax |
| `ANTHROPIC_API_KEY` | Required for agent execution (Claude API access) |
| `CONDUCTOR_RUN_ID` | Passed to agent subprocesses; used by `agent create-issue` when `--run-id` is omitted |
| `CONDUCTOR_PLUGIN_DIRS` | Additional plugin directories appended after repo-level dirs |

## Related Documents

- [MCP Tools Reference](mcp-tools.md) -- MCP equivalents for CLI commands
- [Configuration Reference](configuration.md) -- `config.toml` fields controlling CLI behavior
- [Workflow DSL Syntax Reference](workflow-dsl.md) -- `.wf` file format for `workflow run`
- [Crate Structure](../architecture/crate-structure.md) -- how conductor-cli delegates to conductor-core
