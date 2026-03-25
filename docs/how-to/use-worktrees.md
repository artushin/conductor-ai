---
title: How to Use Conductor Worktrees
type: how-to
layer: 2
---

# How to Use Conductor Worktrees

## Context / Trigger

Run this when you need to work on multiple tasks in parallel using git worktrees managed by conductor. Conductor wraps git worktree operations with ticket linking, branch naming conventions, automatic dependency installation, and workflow integration.

**GRS**: 5 (Standard weight class). Multiple steps, touches git and external services (GitHub for PRs), but single-repo and recoverable.

### Prerequisites

- conductor installed and built (`cargo install --path conductor-cli`)
- A repo registered with conductor (`conductor repo register` -- see [manage repos](manage-repos.md))
- Tickets synced if linking worktrees to tickets (`conductor tickets sync`)

## Steps

### 1. Create a worktree

Create a new worktree (git branch + working directory) for a repo:

```bash
conductor worktree create my-app feat-auth-flow
```

This creates:
- A git branch `feat/auth-flow` (conductor normalizes `feat-` prefix to `feat/` branch convention)
- A working directory at `~/.conductor/workspaces/my-app/feat-auth-flow/`

**Options**:

```bash
# Branch from a specific base branch
conductor worktree create my-app feat-auth-flow --from develop

# Checkout an existing PR branch
conductor worktree create my-app fix-login --from-pr 42

# Link to a ticket and optionally auto-start an agent
conductor worktree create my-app feat-auth-flow --ticket TICKET-123 --auto-agent
```

**Expected outcome**: `Created worktree 'feat-auth-flow' for my-app`. Conductor also auto-installs JS dependencies if a lockfile is detected (bun > pnpm > yarn > npm).

[INVARIANT] Worktree names must be unique per repo. Conductor rejects duplicates.

### 2. Work in the worktree

Navigate to the worktree directory for manual work, or run a workflow targeting it:

```bash
# Manual work
cd ~/.conductor/workspaces/my-app/feat-auth-flow/

# Run a workflow
conductor workflow run ticket-to-pr my-app feat-auth-flow --input ticket_id=TICKET-123
```

### 3. Check worktree status

```bash
# List all active worktrees for a repo
conductor worktree list my-app

# List worktrees across all repos
conductor worktree list
```

**Expected outcome**: Table showing worktree name, branch, status (active/merged/abandoned), linked ticket, and path.

### 4. Push and create a PR

When work is ready for review:

```bash
# Push the branch to origin
conductor worktree push my-app feat-auth-flow

# Create a pull request
conductor worktree pr my-app feat-auth-flow

# Create as draft PR
conductor worktree pr my-app feat-auth-flow --draft
```

**Expected outcome**: Branch pushed to remote and PR created on GitHub. The PR is linked to the worktree record in conductor's database.

### 5. Run review workflows on the PR

```bash
conductor workflow run review-pr my-app feat-auth-flow
# Or target by PR number:
conductor workflow run review-pr --pr 42
```

### 6. Complete and clean up

After merging or abandoning the work:

```bash
# Soft delete (marks as merged or abandoned, keeps git history)
conductor worktree delete my-app feat-auth-flow

# Permanently remove completed worktree records
conductor worktree purge my-app feat-auth-flow

# Purge all completed worktrees for a repo
conductor worktree purge my-app
```

[INVARIANT] `delete` performs a soft delete (status change only). Use `purge` to permanently remove records and clean up disk.

### 7. Set per-worktree model override

Override the model for all agent runs in a specific worktree:

```bash
conductor worktree set-model my-app feat-auth-flow claude-opus-4-6

# Clear the override
conductor worktree set-model my-app feat-auth-flow
```

This overrides the repo-level and global model settings. See [model resolution order](../reference/configuration.md#model-resolution-order) for the full priority chain.

## Anti-Patterns

**Anti-pattern: Manual git worktrees outside conductor**

WRONG:
```bash
git worktree add ../feat-auth-flow -b feat/auth-flow
```

RIGHT:
```bash
conductor worktree create my-app feat-auth-flow
```

**Why**: Conductor tracks worktrees in its database for ticket linking, workflow targeting, status monitoring, and cleanup. Manual git worktrees are invisible to conductor -- workflows cannot target them, the TUI does not display them, and they bypass automatic dependency installation.

**Anti-pattern: Leaving stale worktrees after completion**

WRONG:
```bash
# PR merged, worktree forgotten
# Weeks later: dozens of orphaned worktree directories consuming disk
```

RIGHT:
```bash
# After merge
conductor worktree delete my-app feat-auth-flow
conductor worktree purge my-app
```

**Why**: Each worktree is a full repo checkout consuming disk space. Conductor's purge command cleans up both the database record and the filesystem. The TUI also provides bulk cleanup for completed worktrees.

## Verification

1. **Worktree created**: `conductor worktree list my-app` shows the new worktree with `active` status
2. **Directory exists**: The worktree directory exists at the path shown in `worktree list`
3. **Branch created**: `git -C <worktree-path> branch --show-current` returns the expected branch name
4. **Ticket linked**: `conductor worktree list my-app` shows the linked ticket ID (if `--ticket` was used)
5. **Push succeeds**: `conductor worktree push` exits without error; branch visible on remote
6. **Cleanup complete**: After `purge`, `conductor worktree list my-app` no longer shows the worktree

## Related Documents

- [CLI Commands Reference](../reference/cli-commands.md) -- full `worktree` subcommand reference
- [MCP Tools Reference](../reference/mcp-tools.md) -- `conductor_create_worktree`, `conductor_list_worktrees` MCP equivalents
- [Configuration Reference](../reference/configuration.md) -- `workspace_root`, branch prefix settings
- [How to Register and Manage Repos](manage-repos.md) -- prerequisite repo registration
- [How to Write a Workflow](write-workflow.md) -- workflows that target worktrees
- [Database Schema Reference](../reference/database-schema.md) -- `worktrees` table schema
