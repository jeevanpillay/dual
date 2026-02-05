---
description: Execute isolated experiment according to design
model: haiku
---

# Run Experiment

You are tasked with executing the experiment according to its design, capturing results and artifacts.

## Initial Setup

When invoked:
```
I'm ready to run your experiment. Please provide:
1. Path to experiment design (from /create-experiment)
2. Any special setup needed
3. Environment variables or configuration

I'll execute the experiment and capture results.
```

Wait for user input.

## Execution

1. Read experiment.md completely
2. Verify prerequisites are met
3. Execute procedure step-by-step
4. Capture all measurements and artifacts
5. Record any deviations from procedure
6. Document results in `findings.md`

## Output

Creates file: `experiments/{experiment-slug}/findings.md`

Structure:
```markdown
---
date_run: [ISO timestamp]
status: complete|failed
duration: [How long it took]
---

# Experiment Results: [Hypothesis]

## Procedure Execution
[Did we follow the plan? Any deviations?]

## Measurements Captured
[Raw results from each metric]
- Metric 1: [Value]
- Metric 2: [Value]

## Artifacts Generated
[Files/outputs produced]
- Artifact 1: [Description, location]
- Artifact 2: [Description, location]

## Observations
[Qualitative findings, surprising behavior, issues encountered]
- Observation 1
- Observation 2

## Raw Data
[If applicable, include or link to raw measurements]
```

## Important
- Record exact values and measurements
- Document any unexpected behavior
- Keep artifacts organized and linked
- Don't interpret yet - that's for validation phase
