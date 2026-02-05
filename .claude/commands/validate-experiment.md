---
description: Compare experiment results against research findings
model: sonnet
---

# Validate Experiment

You are tasked with comparing experiment results against research findings, determining if the hypothesis was validated.

## Initial Setup

When invoked:
```
I'm ready to validate your experiment. Please provide:
1. Path to research document
2. Path to experiment design
3. Path to findings from execution

I'll compare results against expectations and research findings.
```

Wait for user input.

## Validation Process

1. Read all three documents completely:
   - Research document (what we expected)
   - Experiment design (how we tested)
   - Findings (what actually happened)

2. Compare results against success criteria:
   - Did automated validation pass?
   - Did manual validation observations match expectations?
   - Did measurements align with research predictions?

3. Assess hypothesis validation:
   - Does evidence support the hypothesis?
   - Do results align with research findings?
   - What unknowns were answered?
   - What new questions emerged?

4. Generate validation report

## Output

Creates file: `experiments/{experiment-slug}/validation.md`

Structure:
```markdown
---
date_validated: [ISO timestamp]
result: passed|failed|inconclusive
---

# Validation: [Hypothesis]

## Summary
[Did the hypothesis hold? Why/why not?]

## Criteria Assessment

### Automated Checks
- [ ] Check 1: [Status, findings]
- [ ] Check 2: [Status, findings]

### Manual Verification
- [ ] Criterion 1: [Status, observations]
- [ ] Criterion 2: [Status, observations]

## Hypothesis Validation
[Did results support/refute the hypothesis?]
- Finding 1 vs Research Prediction 1: [Match? Why/why not?]
- Finding 2 vs Research Prediction 2: [Match? Why/why not?]

## Unknowns Resolved
[What questions from research were answered?]
- Unknown 1: [What we learned]
- Unknown 2: [What we learned]

## New Questions Raised
[What did the experiment reveal that we didn't expect?]
- Question 1: [What surprised us]
- Question 2: [What surprised us]

## Recommendations
[Next steps based on validation]
- If hypothesis validated: [Recommended actions]
- If hypothesis refuted: [Recommended actions]
- If inconclusive: [Recommended refinements]

## Risk Assessment from Research
[Did identified risks materialize?]
- Risk 1: [Did it occur? Impact?]
- Risk 2: [Did it occur? Impact?]
```

## Decision Tree

- **Passed**: Hypothesis validated by evidence
- **Failed**: Evidence contradicts hypothesis
- **Inconclusive**: Results ambiguous or measurements insufficient

Only mark as "passed" if evidence strongly supports hypothesis and success criteria met.
