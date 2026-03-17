# BSG Ecosystem Integration

How conductor-ai integrates with the BSG multi-repo ecosystem.

## Overview

Conductor-ai gained four Rust-level enhancements and a set of PE-invoking agents and workflows to orchestrate the BSG pattern extraction and implementation pipeline.

## Rust Changes

| Change | Location | Purpose |
|--------|----------|---------|
| `--plugin-dir` support | AgentManager | Load agents from external plugin directories |
| Recursive agent search | `load_agent_by_name()` | Search subdirectories when resolving agent names |
| Per-repo plugin_dirs | `repos` table + CLI | Store plugin paths per managed repository |
| conductor-service daemon | New crate | Unix socket JSON-RPC daemon with PE checkpoint watcher |

### Daemon Details

- **Socket**: `~/.conductor/conductor.sock`
- **PID file**: `~/.conductor/conductor.pid`
- **JSON-RPC methods**: `ping`, `status`, `pe.checkpoint.status`
- **PE checkpoint watcher**: Polls `~/.conductor/pe-checkpoint.json` every 5 seconds

### Database Migration

File: `conductor-core/src/db/migrations/036_repo_plugin_dirs.sql`
Adds `plugin_dirs TEXT DEFAULT '[]'` to the `repos` table (JSON array of plugin directory paths).

## PE-Invoking Agents

Five agents in `.conductor/agents/` invoke pattern-extractor commands:

| Agent | Model | PE Command | Output Markers |
|-------|-------|-----------|----------------|
| pe-discover | opus | `/discover` | discover_complete, discover_failed |
| pe-assess | opus | `/assess` | assess_complete, fidelity_score_{N}, fidelity_below_80 |
| pe-extract | opus | `/extract` | extract_complete |
| pe-onboard | opus | `/onboard` | onboard_complete |
| pe-implement | opus | `/implement` | implement_complete |

Each agent sets up Claude with `--plugin-dir /usr/local/bsg/pattern-extractor` and passes context via `{{prior_context}}`.

## BSG Workflows

Three `.wf` workflows in `.conductor/workflows/`:

| Workflow | Steps | Purpose |
|----------|-------|---------|
| bsg-pattern-sync | 3 assess calls + 3 human gates | Detect pattern drift across agent-architecture, fsm-engine, vantage |
| extract-patterns | discover → operate → extract → assess | Run full EDLC extraction pipeline against a source repo |
| implement-patterns | onboard → implement | Apply agentic-sdlc-full profile to a target repo |

All workflows use `human_review` gates with 72-hour timeouts.

## TUI PE Status Panel

- **Module**: `conductor-tui/src/pe_status.rs`
- **Reads**: `/usr/local/bsg/pattern-extractor/extraction-roadmap`
- **Displays**: Active extraction cycles with task counts (total/completed/blocked), inferred cycle status

## Registering a Repo with Plugin Dirs

```bash
# Add a repo to conductor
conductor repo add /path/to/repo

# Set plugin directories for that repo
conductor repo set-plugin-dirs <slug> \
  /usr/local/bsg/agent-architecture \
  /usr/local/bsg/fsm-engine
```

## Related Repositories

| Repo | Relationship | Path |
|------|-------------|------|
| agent-architecture | Shared agents loaded via --plugin-dir | [../../agent-architecture/](../../agent-architecture/) |
| fsm-engine | .wf workflows executed natively | [../../fsm-engine/](../../fsm-engine/) |
| pattern-extractor | PE commands invoked by agents | [../../pattern-extractor/](../../pattern-extractor/) |
| vantage | Managed repo consuming all plugins | [../../vantage/](../../vantage/) |

## References

- [BSG ecosystem overview](../../README.md) — root ecosystem documentation
- [Implementation analysis](../../pattern-extractor/analysis/bsg-system-evaluation/) — architectural decisions and gap analysis
