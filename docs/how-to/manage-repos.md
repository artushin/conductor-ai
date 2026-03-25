---
title: How to Register and Manage Repos
type: how-to
layer: 2
---

# How to Register and Manage Repos

## Context / Trigger

Run this when you need to register a repository with conductor for orchestration, configure its plugin directories for agent resolution, or manage its lifecycle (update settings, unregister).

**GRS**: 4 (Standard weight class). Multiple steps, touches external systems (GitHub, filesystem), but straightforward and reversible.

### Prerequisites

- conductor installed and built (`cargo install --path conductor-cli`)
- Target repo cloned locally (or provide a remote URL for conductor to clone)
- For plugin directories: agent-architecture and/or fsm-engine repos at `/usr/local/bsg/`

## Steps

### 1. Register a repo

Register by remote URL. Conductor derives the slug from the URL automatically.

```bash
conductor repo register https://github.com/acme/my-app
```

To use an existing local checkout (skips clone):

```bash
conductor repo register https://github.com/acme/my-app --local-path /home/dev/my-app
```

**Expected outcome**: `Registered repo 'my-app'`. Conductor creates a workspace directory under `~/.conductor/workspaces/my-app/` and stores the repo record in its database.

### 2. Set plugin directories

Configure which agent plugins are available to all workflows in this repo. Paths must be absolute.

```bash
conductor repo set-plugin-dirs my-app \
  /usr/local/bsg/agent-architecture/planner \
  /usr/local/bsg/agent-architecture/debugger \
  /usr/local/bsg/agent-architecture/review-architecture \
  /usr/local/bsg/agent-architecture/review-security
```

**Expected outcome**: All agent sessions for `my-app` receive `--plugin-dir` flags for the specified directories. Agents from those directories become available to workflow `call` steps.

[INVARIANT] Plugin directories must use absolute paths. Relative paths will not resolve correctly when workflows run from different working directories.

### 3. Configure repo settings

Set optional per-repo defaults:

```bash
# Set default model for agent runs
conductor repo set-model my-app sonnet

# Allow agents to create GitHub issues
conductor repo allow-agent-issues my-app --allow true
```

### 4. Add issue sources for ticket sync

Connect GitHub or Jira as a ticket source:

```bash
# GitHub (auto-inferred from remote URL)
conductor repo sources add my-app --type github

# Jira (requires config JSON)
conductor repo sources add my-app --type jira --config '{"project": "MYAPP", "url": "https://acme.atlassian.net"}'
```

Sync tickets after adding a source:

```bash
conductor tickets sync my-app
```

### 5. Verify registration

```bash
conductor repo list
```

**Expected outcome**: Table showing the repo with slug, local path, remote URL, and configured settings.

### 6. Manage repo lifecycle

```bash
# Remove the default model
conductor repo set-model my-app

# Remove an issue source
conductor repo sources remove my-app --type jira

# Unregister (non-destructive: removes DB record only, does not delete files)
conductor repo unregister my-app
```

## Anti-Patterns

**Anti-pattern: Sourcing from `.conductor/` reference directory**

WRONG:
```bash
conductor repo set-plugin-dirs my-app \
  /usr/local/bsg/conductor-ai/.conductor/agents
```

RIGHT:
```bash
conductor repo set-plugin-dirs my-app \
  /usr/local/bsg/agent-architecture/planner \
  /usr/local/bsg/agent-architecture/review-architecture
```

**Why**: `conductor-ai/.conductor/` contains reference copies, not canonical sources. The BSG ecosystem rule is: agent-architecture owns agent definitions, fsm-engine owns workflow templates. Sourcing from `.conductor/` risks stale or divergent agent versions.

**Anti-pattern: Machine-specific paths**

WRONG:
```bash
conductor repo set-plugin-dirs my-app \
  /home/alice/bsg/agent-architecture/planner
```

RIGHT:
```bash
conductor repo set-plugin-dirs my-app \
  /usr/local/bsg/agent-architecture/planner
```

**Why**: The BSG ecosystem uses `/usr/local/bsg/` as the consistent root across all machines. Machine-specific home paths break when the same repo is used by multiple developers or CI.

## Verification

1. **Repo listed**: `conductor repo list` shows the repo with correct slug, path, and URL
2. **Plugin dirs set**: Verify with `conductor repo list` (shows configured plugin directories)
3. **Issue sources active**: `conductor repo sources list my-app` shows the configured sources
4. **Tickets synced**: `conductor tickets list my-app` returns tickets after sync

## Related Documents

- [CLI Commands Reference](../reference/cli-commands.md) -- full `repo` subcommand reference
- [MCP Tools Reference](../reference/mcp-tools.md) -- `conductor_register_repo`, `conductor_list_repos` MCP equivalents
- [Configuration Reference](../reference/configuration.md) -- per-repo settings stored in the database
- [How to Use Worktrees](use-worktrees.md) -- creating worktrees within registered repos
- For the BSG plugin injection model, see [/usr/local/bsg/CLAUDE.md](/usr/local/bsg/CLAUDE.md)
