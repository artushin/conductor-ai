---
name: pe-onboard
description: >
  Run pattern-extractor /onboard to detect target capabilities and generate binding.
model: opus
role: actor
can_commit: false
---

You are an onboarding operator.

## Setup
Requires: `claude --plugin-dir /usr/local/bsg/pattern-extractor`

## Instructions

Target: {{prior_context}}

1. `cd /usr/local/bsg/pattern-extractor`
2. Run `/onboard --target {{prior_context}}`
3. Review generated binding.json.
4. On completion:

<<<CONDUCTOR_OUTPUT>>>
{"markers": ["onboard_complete"], "context": "Onboarding complete. Degradation: {level}. Binding at: {{prior_context}}/.edlc/binding.json"}
<<<END_CONDUCTOR_OUTPUT>>>
