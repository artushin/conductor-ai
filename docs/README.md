---
title: Conductor-AI Documentation Index
type: reference
layer: 2
---

# Conductor-AI Documentation

Navigational index for all conductor-ai documentation. For a compressed summary optimized for LLM context loading, see [context/CONTEXT_SUMMARY.md](context/CONTEXT_SUMMARY.md).

## Contents

### Architecture (`architecture/`)

| Document | Purpose |
|----------|---------|
| [crate-structure.md](architecture/crate-structure.md) | Workspace layout, 5-crate dependency graph, conductor-core module map, design trade-offs |
| [workflow-engine.md](architecture/workflow-engine.md) | Execution pipeline, parsing, validation, conditions, loops, gates, composition, resumability |
| [structured-output.md](architecture/structured-output.md) | CONDUCTOR_OUTPUT design rationale, schema ownership, validation pipeline |
| [agent-execution.md](architecture/agent-execution.md) | Subprocess model, tmux spawning, agent resolution, prompt assembly, output capture |

### Reference (`reference/`)

| Document | Purpose |
|----------|---------|
| [workflow-dsl.md](reference/workflow-dsl.md) | Complete .wf grammar, all node types, variable substitution, prompt snippets |
| [database-schema.md](reference/database-schema.md) | All 13 tables, column specs, relationship diagram, 37-migration history |
| [mcp-tools.md](reference/mcp-tools.md) | 22 MCP tools and 9 resource categories with parameter tables |
| [cli-commands.md](reference/cli-commands.md) | All CLI subcommands: repo, worktree, tickets, agent, workflow, statusline, mcp |
| [output-schemas.md](reference/output-schemas.md) | Schema YAML format, field types, validation rules, marker derivation |
| [configuration.md](reference/configuration.md) | config.toml fields, data directory, model and plugin resolution order |

### How-To (`how-to/`)

| Document | Purpose |
|----------|---------|
| [write-workflow.md](how-to/write-workflow.md) | Create a .wf workflow file from scratch with validation and dry-run |
| [manage-repos.md](how-to/manage-repos.md) | Register repos, set plugin dirs, configure issue sources |
| [use-worktrees.md](how-to/use-worktrees.md) | Create, push, PR, and clean up worktrees |
| [getting-started-cli.md](how-to/getting-started-cli.md) | Install conductor, register a repo, run first workflow |

### Other

| Document | Purpose |
|----------|---------|
| [themes.md](reference/themes.md) | Theme definitions for workflow-scoped context |
| [roadmap.md](roadmap.md) | Development roadmap and planned features |
| [BSG_INTEGRATION.md](BSG_INTEGRATION.md) | Cross-repo integration with agent-architecture, fsm-engine, chief |

Also: [workflow/](workflow/) (agent resolution, prompt snippets), [rfc/](rfc/) (proposals 001-005).

## Quick Navigation

| I need to... | Go to |
|--------------|-------|
| Understand the crate structure | [architecture/crate-structure.md](architecture/crate-structure.md) |
| Write a new workflow | [how-to/write-workflow.md](how-to/write-workflow.md) |
| Look up .wf syntax | [reference/workflow-dsl.md](reference/workflow-dsl.md) |
| Find MCP tool parameters | [reference/mcp-tools.md](reference/mcp-tools.md) |
| Find a CLI command | [reference/cli-commands.md](reference/cli-commands.md) |
| Register and configure a repo | [how-to/manage-repos.md](how-to/manage-repos.md) |
| Create and manage worktrees | [how-to/use-worktrees.md](how-to/use-worktrees.md) |
| Check the database schema | [reference/database-schema.md](reference/database-schema.md) |
| Configure conductor | [reference/configuration.md](reference/configuration.md) |
