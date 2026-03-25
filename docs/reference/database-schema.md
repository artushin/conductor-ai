---
title: Database Schema Reference
type: reference
layer: 3
---

# Database Schema Reference

Complete schema reference for conductor's SQLite database (`~/.conductor/conductor.db`). The database uses WAL journal mode, foreign keys enabled, and a 5-second busy timeout. Schema is managed by 37 versioned migrations in `conductor-core/src/db/migrations/`.

## Database Configuration

| Setting | Value |
|---------|-------|
| Engine | SQLite (rusqlite with bundled feature) |
| Location | `~/.conductor/conductor.db` |
| Journal mode | WAL |
| Foreign keys | ON |
| Busy timeout | 5000ms |
| ID format | ULID (text, sortable, collision-resistant) |
| Timestamp format | ISO 8601 strings |
| Migration runner | `conductor_core::db::migrations::run()` |

## Table Relationship Diagram

```
repos
 ├──< repo_issue_sources    (repo_id)
 ├──< worktrees              (repo_id)
 │     ├──< agent_runs       (worktree_id)
 │     │     ├──< agent_run_steps     (run_id)
 │     │     ├──< agent_run_events    (run_id)
 │     │     ├──< agent_created_issues (agent_run_id)
 │     │     ├──< feedback_requests   (run_id)
 │     │     └──< workflow_runs       (parent_run_id)
 │     │           └──< workflow_run_steps (workflow_run_id)
 │     └──< workflow_runs    (worktree_id)
 ├──< tickets                (repo_id)
 │     ├──< ticket_labels    (ticket_id)
 │     └──< workflow_runs    (ticket_id)
 └──< agent_created_issues   (repo_id)

_conductor_meta              (standalone key-value)
```

Legend: `──<` = one-to-many foreign key (parent ──< child).

## Table Schemas

### repos

Repository registry. Each registered repo has a unique slug and local path.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| slug | TEXT | NOT NULL, UNIQUE | Short name derived from repo URL |
| local_path | TEXT | NOT NULL | Absolute path to repo on disk |
| remote_url | TEXT | NOT NULL | Git remote URL |
| default_branch | TEXT | NOT NULL, DEFAULT 'main' | Default branch for worktree creation |
| workspace_dir | TEXT | NOT NULL | Root directory for this repo's worktrees |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |
| model | TEXT | | Default model override for this repo |
| allow_agent_issue_creation | INTEGER | NOT NULL, DEFAULT 0 | Whether agents can create issues for this repo |
| plugin_dirs | TEXT | DEFAULT '[]' | JSON array of plugin directory paths |

**Used by**: `RepoManager` (conductor-core/src/repo.rs)

### repo_issue_sources

Per-repo issue tracker configuration (GitHub or Jira).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| repo_id | TEXT | NOT NULL, FK repos(id) CASCADE | Owning repo |
| source_type | TEXT | NOT NULL, CHECK ('github', 'jira') | Issue tracker type |
| config_json | TEXT | NOT NULL | JSON configuration blob (owner/repo for GitHub, project key for Jira) |

**Used by**: `IssueSourceManager` (conductor-core/src/issue_source.rs)

### worktrees

Git worktree lifecycle tracking. Each worktree belongs to a repo and optionally links to a ticket.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| repo_id | TEXT | NOT NULL, FK repos(id) CASCADE | Owning repo |
| slug | TEXT | NOT NULL | Short name (unique per repo) |
| branch | TEXT | NOT NULL | Git branch name |
| path | TEXT | NOT NULL | Absolute filesystem path |
| ticket_id | TEXT | FK tickets(id) SET NULL | Linked ticket (optional) |
| status | TEXT | NOT NULL, DEFAULT 'active', CHECK ('active', 'merged', 'abandoned') | Lifecycle state |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |
| completed_at | TEXT | | When status changed from active |
| model | TEXT | | Per-worktree model override |
| base_branch | TEXT | | Branch this worktree was created from (NULL = repo default) |

**Unique constraint**: `(repo_id, slug)`

**Used by**: `WorktreeManager` (conductor-core/src/worktree.rs)

### tickets

Synced tickets from GitHub Issues, Jira, or local creation.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| repo_id | TEXT | NOT NULL, FK repos(id) CASCADE | Owning repo |
| source_type | TEXT | NOT NULL, CHECK ('github', 'jira', 'local') | Origin tracker |
| source_id | TEXT | NOT NULL | ID in the source system (issue number, Jira key) |
| title | TEXT | NOT NULL | Ticket title |
| body | TEXT | NOT NULL, DEFAULT '' | Ticket body/description |
| state | TEXT | NOT NULL, DEFAULT 'open', CHECK ('open', 'in_progress', 'closed') | Current state |
| labels | TEXT | NOT NULL, DEFAULT '[]' | JSON array of label strings (legacy; see ticket_labels) |
| assignee | TEXT | | Assigned user |
| priority | TEXT | | Priority level |
| url | TEXT | NOT NULL, DEFAULT '' | URL to ticket in source system |
| synced_at | TEXT | NOT NULL | Last sync timestamp |
| raw_json | TEXT | NOT NULL, DEFAULT '{}' | Full JSON from source API |

**Unique constraint**: `(repo_id, source_type, source_id)` -- upsert target for idempotent sync.

**Used by**: `TicketSyncer` (conductor-core/src/tickets.rs)

### ticket_labels

Normalized label storage for tickets (added in migration 029).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| ticket_id | TEXT | NOT NULL, FK tickets(id) CASCADE | Owning ticket |
| label | TEXT | NOT NULL | Label name |
| color | TEXT | | Hex color from source system |

**Primary key**: `(ticket_id, label)`

### agent_runs

Individual agent execution records. Agents run as Claude processes in tmux windows.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| worktree_id | TEXT | NOT NULL, FK worktrees(id) CASCADE | Execution context |
| claude_session_id | TEXT | | Claude session identifier |
| prompt | TEXT | NOT NULL | Agent prompt text |
| status | TEXT | NOT NULL, DEFAULT 'running', CHECK ('running', 'completed', 'failed', 'cancelled', 'waiting_for_feedback') | Run state |
| result_text | TEXT | | Final output from agent |
| cost_usd | REAL | | Estimated API cost |
| num_turns | INTEGER | | Number of conversation turns |
| duration_ms | INTEGER | | Execution duration |
| started_at | TEXT | NOT NULL | ISO 8601 timestamp |
| ended_at | TEXT | | Completion timestamp |
| tmux_window | TEXT | | Tmux window name for this run |
| log_file | TEXT | | Path to agent log file |
| plan | TEXT | | JSON plan blob (legacy; see agent_run_steps) |
| model | TEXT | | Model used for this run |
| parent_run_id | TEXT | FK agent_runs(id) SET NULL | Parent run for orchestrated child agents |
| input_tokens | INTEGER | | Input token count |
| output_tokens | INTEGER | | Output token count |
| cache_read_input_tokens | INTEGER | | Cached input tokens read |
| cache_creation_input_tokens | INTEGER | | Cached input tokens created |
| bot_name | TEXT | | GitHub bot name for commits |

**Index**: `idx_agent_runs_parent` on `(parent_run_id)`

**Used by**: `AgentManager` (conductor-core/src/agent.rs)

### agent_run_steps

Durable plan steps for agent runs (replaced the JSON `plan` column).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| run_id | TEXT | NOT NULL, FK agent_runs(id) CASCADE | Parent agent run |
| position | INTEGER | NOT NULL | Step order (0-indexed) |
| description | TEXT | NOT NULL | Human-readable step description |
| status | TEXT | NOT NULL, DEFAULT 'pending', CHECK ('pending', 'in_progress', 'completed', 'failed') | Step state |
| started_at | TEXT | | Step start timestamp |
| completed_at | TEXT | | Step completion timestamp |

**Index**: `idx_agent_run_steps_run_id` on `(run_id)`

### agent_run_events

Trace/span events within an agent run. Used for granular progress tracking.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| run_id | TEXT | NOT NULL, FK agent_runs(id) CASCADE | Parent agent run |
| kind | TEXT | NOT NULL | Event type identifier |
| summary | TEXT | NOT NULL | Human-readable event description |
| started_at | TEXT | NOT NULL | Event start timestamp |
| ended_at | TEXT | | Event end timestamp (NULL = ongoing) |
| metadata | TEXT | | JSON metadata blob |

**Index**: `idx_agent_run_events_run_id` on `(run_id)`

### agent_created_issues

Issues created by agents during execution.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| agent_run_id | TEXT | NOT NULL, FK agent_runs(id) CASCADE | Creating agent run |
| repo_id | TEXT | NOT NULL, FK repos(id) CASCADE | Target repo for the issue |
| source_type | TEXT | NOT NULL, DEFAULT 'github' | Issue tracker type |
| source_id | TEXT | NOT NULL | Issue ID in the source system |
| title | TEXT | NOT NULL | Issue title |
| url | TEXT | NOT NULL, DEFAULT '' | URL to the created issue |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |

### feedback_requests

Human-in-the-loop feedback for agent runs in `waiting_for_feedback` status.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| run_id | TEXT | NOT NULL, FK agent_runs(id) CASCADE | Agent run requesting feedback |
| prompt | TEXT | NOT NULL | Question/prompt for the human |
| response | TEXT | | Human's response |
| status | TEXT | NOT NULL, DEFAULT 'pending', CHECK ('pending', 'responded', 'dismissed') | Feedback state |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |
| responded_at | TEXT | | When feedback was provided |

**Indexes**: `idx_feedback_requests_run_id` on `(run_id)`, `idx_feedback_requests_status_run_id` on `(status, run_id)`

### workflow_runs

Workflow execution tracking. Each workflow run is associated with a worktree and a parent agent run.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| workflow_name | TEXT | NOT NULL | Name of the .wf workflow |
| worktree_id | TEXT | NOT NULL, FK worktrees(id) CASCADE | Execution context |
| parent_run_id | TEXT | NOT NULL, FK agent_runs(id) CASCADE | Agent run that initiated this workflow |
| status | TEXT | NOT NULL, DEFAULT 'pending', CHECK ('pending', 'running', 'waiting', 'completed', 'failed', 'cancelled') | Workflow state |
| dry_run | INTEGER | NOT NULL, DEFAULT 0 | Whether this is a dry run |
| trigger | TEXT | NOT NULL, DEFAULT 'manual', CHECK ('manual', 'pr', 'scheduled') | How the workflow was started |
| started_at | TEXT | NOT NULL | ISO 8601 timestamp |
| ended_at | TEXT | | Completion timestamp |
| result_summary | TEXT | | Final summary text |
| definition_snapshot | TEXT | | Serialized WorkflowDef JSON (frozen at start) |
| inputs | TEXT | | JSON input parameters |
| ticket_id | TEXT | FK tickets(id) | Associated ticket (optional) |
| repo_id | TEXT | FK repos(id) | Associated repo (optional) |
| parent_workflow_run_id | TEXT | FK workflow_runs(id) | Parent workflow for nested execution |
| target_label | TEXT | | Target label for filtered execution |
| default_bot_name | TEXT | | Default GitHub bot name for workflow steps |

**Indexes**: `idx_workflow_runs_worktree` on `(worktree_id)`, `idx_workflow_runs_parent` on `(parent_run_id)`, `idx_workflow_runs_ticket` on `(ticket_id)`, `idx_workflow_runs_repo` on `(repo_id)`, `idx_workflow_runs_parent_wf` on `(parent_workflow_run_id)`

**Used by**: `WorkflowManager` (conductor-core/src/workflow.rs)

### workflow_run_steps

Individual step execution within a workflow run.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | TEXT | PRIMARY KEY | ULID |
| workflow_run_id | TEXT | NOT NULL, FK workflow_runs(id) CASCADE | Parent workflow run |
| step_name | TEXT | NOT NULL | Step identifier from .wf definition |
| role | TEXT | NOT NULL, CHECK ('actor', 'reviewer', 'gate') | Step role |
| can_commit | INTEGER | NOT NULL, DEFAULT 0 | Whether the step can commit changes |
| condition_expr | TEXT | | Conditional expression (if/unless) |
| status | TEXT | NOT NULL, DEFAULT 'pending', CHECK ('pending', 'running', 'waiting', 'completed', 'failed', 'skipped') | Step state |
| child_run_id | TEXT | FK agent_runs(id) SET NULL | Agent run spawned for this step |
| position | INTEGER | NOT NULL | Step order |
| started_at | TEXT | | Step start timestamp |
| ended_at | TEXT | | Step completion timestamp |
| result_text | TEXT | | Step output text |
| condition_met | INTEGER | | Whether the conditional was satisfied |
| iteration | INTEGER | NOT NULL, DEFAULT 0 | Loop iteration counter |
| parallel_group_id | TEXT | | Shared ID for parallel step group |
| context_out | TEXT | | CONDUCTOR_OUTPUT context JSON |
| markers_out | TEXT | | CONDUCTOR_OUTPUT markers JSON |
| retry_count | INTEGER | NOT NULL, DEFAULT 0 | Number of retries executed |
| structured_output | TEXT | | Schema-validated JSON output |
| gate_type | TEXT | | Gate type identifier |
| gate_prompt | TEXT | | Gate prompt for human approval |
| gate_timeout | TEXT | | Gate timeout duration |
| gate_approved_by | TEXT | | Who approved the gate |
| gate_approved_at | TEXT | | Gate approval timestamp |
| gate_feedback | TEXT | | Feedback provided at gate |

**Index**: `idx_workflow_run_steps_run` on `(workflow_run_id)`

### _conductor_meta

Internal metadata key-value store. Used by the migration runner to track schema version.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| key | TEXT | PRIMARY KEY | Metadata key |
| value | TEXT | NOT NULL | Metadata value |

**Current keys**: `schema_version` (integer as string, current value: 37)

## Migration History

37 migrations, applied sequentially. Key evolutionary milestones:

| Migration | Description |
|-----------|-------------|
| 001 | Initial schema: repos, repo_issue_sources, worktrees, tickets, sessions |
| 003 | Agent runs table |
| 006 | Drop sessions/session_worktrees (unused) |
| 007 | Agent run events (trace model) + agent-created issues |
| 012 | Parent-child agent run relationships |
| 015 | Agent run steps (replaces JSON plan blob) |
| 017-019 | Review configs added then dropped (moved to file-based config) |
| 020 | Workflow runs + workflow run steps |
| 021 | Workflow redesign: definition snapshots, parallel groups, structured output, gate columns |
| 028 | Drop merge queue (unused) |
| 029 | Ticket labels (normalized) |
| 030 | Workflow run targets (ticket_id, repo_id) |
| 032 | Agent run token counts |
| 036 | Repo plugin directories |
| 037 | Local ticket source type |

The migration runner uses column-existence checks (not just version numbers) to handle databases that jumped versions via feature branches.

## Dropped Tables

These tables were created and later removed:

| Table | Created | Dropped | Reason |
|-------|---------|---------|--------|
| sessions | 001 | 006 | Session tracking replaced by agent runs |
| session_worktrees | 001 | 006 | Junction table for sessions (dropped with sessions) |
| review_configs | 017 | 019 | Review configuration moved to file-based `.conductor/reviewers/*.md` |
| merge_queue | (unknown) | 028 | Merge queue functionality removed |

## Related Documents

- [Crate Structure and Dependency Graph](../architecture/crate-structure.md) -- how the database layer fits into conductor-core
- For database configuration settings, grep `## Database Configuration` in this file
- For the migration runner implementation, see `conductor-core/src/db/migrations.rs`
