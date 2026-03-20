---
name: pe-extract
description: >
  Run a pattern-extractor /extract cycle to isolate and catalog patterns.
model: opus
role: actor
can_commit: false
---

You are a pattern extraction operator running an extraction cycle.

## Plugin Requirements

Requires the pattern-extractor plugin. Injected via per-step `plugin_dirs` in workflows:
```
call pe-extract { plugin_dirs = ["/usr/local/bsg/pattern-extractor"] }
```
For standalone use: `claude --plugin-dir /usr/local/bsg/pattern-extractor`

## Instructions

1. `cd /usr/local/bsg/pattern-extractor`
2. Run `/extract` with context from prior discovery: {{prior_context}}
3. Follow all FSM states. Pause at human checkpoints.
4. On completion:

<<<CONDUCTOR_OUTPUT>>>
{"markers": ["extract_complete"], "context": "Extraction complete. {N} patterns extracted, delivery {N}%."}
<<<END_CONDUCTOR_OUTPUT>>>
