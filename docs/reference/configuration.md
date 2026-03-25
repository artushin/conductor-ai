---
title: Configuration Reference
type: reference
layer: 3
---

# Configuration Reference

All configuration for conductor. The primary config file is `~/.conductor/config.toml`. Per-repo settings are stored in the database and managed via CLI commands.

## Data Directory

All conductor state lives under `~/.conductor/`:

| Path | Purpose |
|------|---------|
| `config.toml` | Global configuration (this document) |
| `conductor.db` | SQLite database (WAL mode, foreign keys on, 5s busy timeout) |
| `workspaces/` | Default root for repo checkouts and worktrees |
| `agent-logs/` | Agent run log files (`{run_id}.log`) |
| `themes/` | User-supplied TUI theme files (base16 JSON) |
| `conductor.sock` | Unix domain socket for conductor-service daemon |
| `conductor.pid` | PID lock file for conductor-service |

## `config.toml` Fields

### `[general]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `workspace_root` | path | `~/.conductor/workspaces` | Root directory for repo checkouts and worktrees |
| `sync_interval_minutes` | integer | `15` | Ticket sync interval for background sync |
| `model` | string | none | Global default model for agent runs (e.g., `"sonnet"`, `"claude-opus-4-6"`). Overridden by per-repo, per-worktree, and per-run settings |
| `auto_start_agent` | string | `"ask"` | After creating a worktree from a ticket: `"ask"` (prompt), `"always"`, or `"never"` |
| `inject_startup_context` | boolean | `true` | Auto-inject session context (worktree, ticket, prior runs, recent commits) into agent prompts |
| `theme` | string | none | TUI color theme: `"conductor"` (default), `"nord"`, `"gruvbox"`, `"catppuccin_mocha"`, or a filename stem from `~/.conductor/themes/` |
| `work_targets` | array | `[{name: "VS Code", command: "code", type: "editor"}]` | External tools to open worktrees in (TUI "open in" menu) |

### `[defaults]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_branch` | string | `"main"` | Default branch for new repos |
| `worktree_prefix_feat` | string | `"feat-"` | Branch prefix for feature worktrees (auto-normalized to `feat/` in git) |
| `worktree_prefix_fix` | string | `"fix-"` | Branch prefix for fix worktrees (auto-normalized to `fix/` in git) |

### `[github.app]`

Single GitHub App identity for posting comments as a bot. Optional.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `app_id` | integer | Yes | GitHub App ID |
| `client_id` | string | No | Client ID from App settings page (required for newer GitHub Apps; falls back to `app_id`) |
| `private_key_path` | string | Yes | Path to the `.pem` private key file |
| `installation_id` | integer | Yes | GitHub App installation ID |

### `[github.apps.<name>]`

Named GitHub App identities for multi-bot workflows. Same fields as `[github.app]`. Referenced in `.wf` files via `as = "<name>"`.

```toml
[github.apps.developer]
app_id = 111111
private_key_path = "~/.conductor/developer.pem"
installation_id = 222222

[github.apps.reviewer]
app_id = 333333
private_key_path = "~/.conductor/reviewer.pem"
installation_id = 444444
```

## Example `config.toml`

```toml
[general]
workspace_root = "~/.conductor/workspaces"
sync_interval_minutes = 5
model = "sonnet"
theme = "conductor"
auto_start_agent = "ask"

[defaults]
default_branch = "main"

[github.app]
app_id = 123456
client_id = "Iv23liXXXXXXXXXXXXXX"
private_key_path = "~/.conductor/conductor-ai.pem"
installation_id = 789012
```

## Per-Repo Configuration

Per-repo settings are stored in the database, not in `config.toml`. Managed via CLI commands:

| Setting | Command | Description |
|---------|---------|-------------|
| Plugin directories | `conductor repo set-plugin-dirs <slug> <dirs...>` | `--plugin-dir` args for all agent sessions |
| Default model | `conductor repo set-model <slug> [<model>]` | Model override for this repo's agents |
| Agent issue creation | `conductor repo allow-agent-issues <slug>` | Whether agents can create GitHub issues |
| Issue sources | `conductor repo sources add <slug>` | GitHub/Jira ticket sync configuration |

## Per-Worktree Configuration

| Setting | Command | Description |
|---------|---------|-------------|
| Default model | `conductor worktree set-model <repo> <name> [<model>]` | Model override for this worktree's agents |

## Model Resolution Order

When spawning an agent, the model is resolved in this order (first non-empty wins):

1. Per-run `--model` flag (CLI or MCP `conductor_run_workflow`)
2. Agent definition `model` field (frontmatter in `.md` file)
3. Per-worktree model (`conductor worktree set-model`)
4. Per-repo model (`conductor repo set-model`)
5. Global `general.model` in `config.toml`
6. Claude's default model

## Plugin Directory Resolution Order

When spawning an agent, `--plugin-dir` arguments are assembled:

1. Repo-level dirs (from `conductor repo set-plugin-dirs`)
2. `CONDUCTOR_PLUGIN_DIRS` environment variable (colon-separated)
3. Per-step `plugin_dirs` (from `call` block in `.wf` file)

All three levels are concatenated; per-step dirs are appended last.

**Timeout defaults**: `step_timeout_secs` and `child_timeout_secs` both default to 604800 (1 week). These are not in `config.toml` -- they are CLI flags (`--step-timeout-secs`, `--child-timeout-secs`) on `conductor workflow run`.

## Per-Repo `.conductor/` Directory

Each repo using conductor workflows has a `.conductor/` directory:

| Path | Purpose |
|------|---------|
| `.conductor/workflows/` | Workflow `.wf` definition files |
| `.conductor/agents/` | Repo-local agent definition `.md` files |
| `.conductor/schemas/` | Output schemas for structured agent output (YAML) |
| `.conductor/prompts/` | Reusable prompt snippet `.md` files |

These are per-repo working artifacts, not global configuration. See [workflow DSL reference](workflow-dsl.md) for how they are referenced in `.wf` files.

## Related Documents

- [CLI Commands Reference](cli-commands.md) -- commands that manage configuration
- [MCP Tools Reference](mcp-tools.md) -- MCP tools that read configuration
- [Crate Structure](../architecture/crate-structure.md) -- `config.rs` module details
- [Database Schema Reference](database-schema.md) -- database tables storing per-repo settings
- For theme system details, see [themes reference](themes.md)
