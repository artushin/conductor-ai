---
title: MCP Tools and Resources Reference
type: reference
layer: 3
---

# MCP Tools and Resources Reference

Complete reference for conductor's Model Context Protocol (MCP) server. The server runs via `conductor mcp serve` on stdio transport, exposing 22 tools and dynamic resources to Claude Code sessions. Each request opens a fresh SQLite connection via `spawn_blocking`.

## Tool Summary

| Tool | Category | Description |
|------|----------|-------------|
| `conductor_list_repos` | Repo | List all registered repos with active run counts |
| `conductor_register_repo` | Repo | Register a repo by remote URL |
| `conductor_unregister_repo` | Repo | Unregister a repo (non-destructive) |
| `conductor_list_worktrees` | Worktree | List worktrees for a repo |
| `conductor_create_worktree` | Worktree | Create a new worktree (branch + directory) |
| `conductor_delete_worktree` | Worktree | Delete a worktree (destructive) |
| `conductor_get_worktree` | Worktree | Get rich detail for a single worktree |
| `conductor_push_worktree` | Worktree | Push worktree branch to remote |
| `conductor_list_tickets` | Ticket | List tickets with label/search filters |
| `conductor_sync_tickets` | Ticket | Sync tickets from GitHub/Jira |
| `conductor_list_workflows` | Workflow | List available workflow definitions |
| `conductor_validate_workflow` | Workflow | Validate a workflow definition |
| `conductor_run_workflow` | Workflow | Start a workflow run |
| `conductor_list_runs` | Run | List workflow runs with filters |
| `conductor_get_run` | Run | Get run status and step details |
| `conductor_resume_run` | Run | Resume a failed or paused run |
| `conductor_cancel_run` | Run | Cancel a running or waiting run |
| `conductor_approve_gate` | Gate | Approve a waiting human gate |
| `conductor_reject_gate` | Gate | Reject a waiting human gate |
| `conductor_list_agent_runs` | Agent | List agent runs with filters |
| `conductor_submit_agent_feedback` | Agent | Submit feedback to a waiting agent |
| `conductor_get_step_log` | Run | Get full agent log for a workflow step |
| `conductor_list_prs` | Repo | List open PRs with CI/review status |

## Repo Tools

### `conductor_list_repos`

List all registered repos with active run counts (running, waiting, pending) per repo.

**Parameters**: None.

### `conductor_register_repo`

Register a repo by remote URL. Slug is derived from the URL (e.g., `https://github.com/acme/my-repo` becomes `my-repo`).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `remote_url` | string | Yes | Remote URL (e.g., `https://github.com/acme/my-repo`) |
| `local_path` | string | No | Local checkout path. Default: `~/.conductor/workspaces/<slug>/main` |

### `conductor_unregister_repo`

Remove a repo from conductor's registry. Non-destructive: only removes the DB record.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug to unregister |

### `conductor_list_prs`

List open pull requests for a repo. Returns PR number, title, URL, branch, author, draft status, review decision, and CI status. Includes linked worktree info when available.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |

## Worktree Tools

### `conductor_list_worktrees`

List worktrees for a repo. Defaults to active only.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `status` | string | No | `"active"` (default) or `"all"` |

### `conductor_create_worktree`

Create a new worktree (git branch + working directory).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `name` | string | Yes | Worktree name (e.g., `feat-my-feature`) |
| `ticket_id` | string | No | Ticket ID to link (ULID or external source ID) |

### `conductor_delete_worktree`

Delete a worktree. **Destructive**: removes the git branch and working directory.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `slug` | string | Yes | Worktree slug to delete |

### `conductor_get_worktree`

Rich detail for a single worktree: branch, status, path, model, linked ticket, associated PR with CI status, and latest agent/workflow run.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `slug` | string | Yes | Worktree slug or branch name |

### `conductor_push_worktree`

Push the current branch of a worktree to the remote.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `slug` | string | Yes | Worktree slug |

## Ticket Tools

### `conductor_list_tickets`

List tickets for a repo with optional filters.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `label` | string | No | Filter by label (comma-separated for multiple) |
| `search` | string | No | Text search against title and body |
| `include_closed` | string | No | `"true"` to include closed tickets (default: open only) |

### `conductor_sync_tickets`

Sync tickets from the configured issue source (GitHub/Jira). When `ticket_id` is provided, re-fetches only that ticket.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `ticket_id` | string | No | ULID, source ID, or GitHub PR URL to re-fetch |

## Workflow Tools

### `conductor_list_workflows`

List available workflow definitions for a repo. Returns names, descriptions, triggers, and input schemas.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |

### `conductor_validate_workflow`

Validate a workflow definition: checks for missing agents, missing snippets, cycles, and semantic errors.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | Repo slug |
| `workflow` | string | Yes | Workflow name (without `.wf` extension) |

### `conductor_run_workflow`

Start a workflow. Returns `run_id` immediately; poll with `conductor_get_run`.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `workflow` | string | Yes | Workflow name |
| `repo` | string | Yes | Repo slug |
| `worktree` | string | No | Worktree slug or branch name |
| `pr` | string | No | PR number or URL (resolves linked worktree). Mutually exclusive with `worktree` |
| `inputs` | object | No | Input key-value pairs |
| `dry_run` | boolean | No | If true: gates auto-approved, committing agents prefixed with "DO NOT commit", `{{dry_run}}` is `"true"` |

## Run Tools

### `conductor_list_runs`

List recent workflow runs. Supports cross-repo listing when `repo` is omitted.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | No | Repo slug (omit for all repos) |
| `worktree` | string | No | Worktree slug or branch filter |
| `status` | string | No | Filter: `pending`, `running`, `completed`, `failed`, `cancelled`, `waiting` |
| `limit` | string | No | Max runs (default 50) |
| `offset` | string | No | Pagination offset (default 0) |

### `conductor_get_run`

Get status and step details of a workflow run. For conversation log tail, use the `conductor://run/{run_id}` resource instead.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |

### `conductor_resume_run`

Resume a failed or paused workflow run from its last failed step.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |
| `from_step` | string | No | Resume from this named step instead of last failed |
| `model` | string | No | Override Claude model for resumed steps |

### `conductor_cancel_run`

Cancel a running or waiting workflow run. Best-effort cancellation of in-progress steps and child agent runs.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |

### `conductor_get_step_log`

Retrieve the full agent log for a named step. Use to diagnose step failures. Gate steps and skipped steps have no logs.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |
| `step_name` | string | Yes | Step name (as shown in `conductor_get_run` output) |

## Gate Tools

### `conductor_approve_gate`

Approve a waiting human gate in a workflow run.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |
| `feedback` | string | No | Approval message or feedback |

### `conductor_reject_gate`

Reject a waiting human gate (fails the workflow).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Workflow run ID |
| `feedback` | string | No | Rejection reason |

## Agent Tools

### `conductor_list_agent_runs`

List agent runs with optional filters. Use `status=waiting_for_feedback` to find runs awaiting human input.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | No | Repo slug (omit for all repos) |
| `worktree` | string | No | Worktree slug or branch (requires `repo`) |
| `status` | string | No | Filter: `running`, `waiting_for_feedback`, `completed`, `failed`, `cancelled` |
| `limit` | string | No | Max runs (default 50) |
| `offset` | string | No | Pagination offset (default 0) |

### `conductor_submit_agent_feedback`

Submit feedback to an agent run that is `waiting_for_feedback`, resuming the agent.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `run_id` | string | Yes | Agent run ID |
| `feedback` | string | Yes | Feedback or answer to deliver |

## Resources

Resources use a `conductor://` URI scheme. The server dynamically enumerates resources based on registered repos and their contents.

### Static resources

| URI | Description |
|-----|-------------|
| `conductor://repos` | All registered repos (slug, path, URL, default branch) |

### Per-repo resources (generated for each registered repo)

| URI Pattern | Description |
|-------------|-------------|
| `conductor://repo/{slug}` | Single repo detail |
| `conductor://tickets/{slug}` | Open tickets for a repo |
| `conductor://worktrees/{slug}` | Worktrees for a repo |
| `conductor://runs/{slug}` | Recent workflow runs (up to 50) |
| `conductor://workflows/{slug}` | Available workflow definitions |

### Per-entity resources (generated dynamically)

| URI Pattern | Description |
|-------------|-------------|
| `conductor://ticket/{repo}/{source_id}` | Individual ticket with full body |
| `conductor://worktree/{repo}/{slug}` | Individual worktree detail with linked ticket |
| `conductor://run/{run_id}` | Workflow run detail with step log and conversation tail |

The `conductor://run/{run_id}` resource provides richer detail than `conductor_get_run`, including the tail of the most recent Claude Code conversation log (last 20 user/assistant messages from the worktree's `.claude/projects/` JSONL file).

## Related Documents

- [CLI Commands Reference](cli-commands.md) -- CLI equivalents for MCP tools
- [Workflow DSL Syntax Reference](workflow-dsl.md) -- `.wf` file syntax used by `conductor_run_workflow`
- [Workflow Engine Architecture](../architecture/workflow-engine.md) -- execution model behind workflow tools
- [Agent Execution Architecture](../architecture/agent-execution.md) -- agent lifecycle behind agent tools
- [Configuration Reference](configuration.md) -- `config.toml` fields affecting MCP behavior
