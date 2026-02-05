---
description: Design isolated experiment methodology and success criteria
model: opus
---

# Create Experiment

You are tasked with designing a replicable experiment that tests the hypothesis researched in `/research-experiment`.

## Initial Setup

When invoked:
```
I'm ready to design your experiment. Please provide:
1. Path to the research document (from /research-experiment)
2. Environment/tools available for testing
3. Any constraints on the experiment (time, resources, safety)

I'll design the experiment methodology and success criteria.
```

Wait for user input.

## Design Output

Creates file: `experiments/{experiment-slug}/experiment.md`

Structure:
```markdown
---
hypothesis: [From research phase]
date_created: [ISO date]
status: design_complete
---

# Experiment Design: [Hypothesis]

## Research Reference
[Link to corresponding research document]

## Experiment Objective
[Clear, testable objective derived from hypothesis]

## Methodology

### Setup
[How to prepare for the experiment]
- Prerequisite 1
- Prerequisite 2

### Procedure
[Step-by-step how to run the experiment]
1. Step 1
2. Step 2
...

### Measurements
[What metrics will we capture?]
- Metric 1: [How measured, success threshold]
- Metric 2: [How measured, success threshold]

### Edge Cases to Test
[From research phase - what operations must still work?]
- Case 1: [Description, expected behavior]
- Case 2: [Description, expected behavior]

## Success Criteria

### Automated Validation
[What can be checked programmatically]
- [ ] Criteria 1
- [ ] Criteria 2

### Manual Validation
[What requires human verification]
- [ ] Criteria 1
- [ ] Criteria 2

## Failure Modes
[How we'll know if it doesn't work]
- Failure 1: [What indicates failure]
- Failure 2: [What indicates failure]

## Expected Artifacts
[What the experiment should produce]
- Artifact 1: [Type, location]
- Artifact 2: [Type, location]
```

## Timeline
- Read research document fully
- Ask clarifying questions if needed
- Design experiment structure
- Present design for approval
- Iterate based on feedback
