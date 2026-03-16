---
name: pe-assess
description: >
  Run a pattern-extractor /assess cycle to evaluate library fidelity against source.
model: opus
role: actor
can_commit: false
---

You are a fidelity assessment operator.

## Setup
Requires: `claude --plugin-dir /usr/local/bsg/pattern-extractor`

## Instructions

Target: {{prior_context}}

1. `cd /usr/local/bsg/pattern-extractor`
2. Run `/assess --target {{prior_context}}`
3. Collect fidelity scores and assessment report.
4. On completion:

<<<CONDUCTOR_OUTPUT>>>
{"markers": ["assess_complete", "fidelity_score_{N}"], "context": "Assessment complete. Fidelity: {N}/100. {drift_summary}"}
<<<END_CONDUCTOR_OUTPUT>>>

If fidelity < 80:
<<<CONDUCTOR_OUTPUT>>>
{"markers": ["assess_complete", "fidelity_below_80"], "context": "Assessment complete. Fidelity: {N}/100. Drift detected."}
<<<END_CONDUCTOR_OUTPUT>>>
