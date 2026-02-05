---
description: Validate experiment results against research predictions, determine verdict
model: opus
---

# Validate Experiment

You are tasked with validating that an experiment was correctly executed, comparing results against research predictions, and determining whether the hypothesis was validated.

## CRITICAL: YOUR JOB IS TO INTERPRET AND CONCLUDE

- DO NOT re-run tests — use the findings from Run phase
- DO NOT add new tests — that would require Create phase
- DO evaluate results against research predictions
- DO determine if unknowns were answered
- DO conclude whether hypothesis is validated/invalidated/inconclusive
- You are a scientist analyzing data, not a technician collecting it

## Getting Started

When invoked:

1. **Determine context** — Are you in an existing conversation or starting fresh?
   - If existing: Review what was researched, designed, and executed in this session
   - If fresh: Need all three document paths

2. **Locate documents**:
   - If experiment slug provided, find all docs in `experiments/{slug}/`
   - Otherwise, ask for paths

3. **If no parameters provided**:
```
I'll validate your experiment results against research predictions.

Please provide the experiment slug or paths:

Option 1 (recommended): `/validate_experiment experiments/shell-wrapper-interception/`

Option 2 (explicit paths):
1. Research document: `thoughts/shared/research/...`
2. Experiment design: `experiments/{slug}/experiment.md`
3. Findings: `experiments/{slug}/findings.md`

I'll compare results, determine verdict, and identify what we learned.
```

## Validation Process

### Step 1: Context Discovery

1. **Read all three documents FULLY**:
   - Research document — understand predictions and unknowns
   - Experiment design — understand success criteria
   - Findings — understand actual results
   - **Never use limit/offset** — you need complete context

2. **Create mental model**:
   - What did research predict would happen?
   - What criteria define success/failure?
   - What actually happened during execution?

3. **Spawn parallel analysis tasks** for thorough validation:
   ```
   Task 1 (knowledge-analyst): "Analyze the technical claims in the research document.
   For each claim, identify what would constitute proof or disproof."

   Task 2 (knowledge-validator): "Compare findings against research predictions.
   For each test, document: predicted vs actual, and whether they match."

   Task 3 (knowledge-comparator): "Compare the experiment design's success criteria
   against the actual measurements in findings. Identify matches and mismatches."
   ```

4. **Wait for all tasks to complete** before proceeding

### Step 2: Systematic Validation

For each test group:

#### Core Functionality Tests (Test Group 1)
These prove or disprove the hypothesis directly.

| Question | Source | Answer |
|----------|--------|--------|
| What did research predict? | Research doc | [Quote] |
| What was success criteria? | Experiment design | [Criteria] |
| What actually happened? | Findings | [Result] |
| Does result match prediction? | Your analysis | Yes/No/Partial |

#### Unknown Validation Tests (Test Group 2)
These answer questions research couldn't answer.

| Question | Source | Answer |
|----------|--------|--------|
| What was the unknown? | Research doc | [Question] |
| What test was designed? | Experiment design | [Test] |
| What did we learn? | Findings + Analysis | [Answer] |
| Is unknown resolved? | Your analysis | Resolved/Partial/Still Unknown |

#### Assumption Validation Tests (Test Group 3)
These validate things research assumed true.

| Question | Source | Answer |
|----------|--------|--------|
| What was assumed? | Research doc | [Assumption] |
| How was it tested? | Experiment design | [Test] |
| Was assumption correct? | Findings + Analysis | Correct/Incorrect/Unclear |
| Impact if wrong? | Your analysis | [Implications] |

### Step 3: Determine Verdict

Apply these criteria strictly:

**VALIDATED** requires:
- Core functionality tests pass
- Critical assumptions validated
- No contradicting evidence
- Unknowns resolved or don't block hypothesis

**INVALIDATED** requires:
- Core functionality tests fail, OR
- Critical assumption proven wrong, OR
- Strong contradicting evidence that undermines hypothesis

**INCONCLUSIVE** when:
- Mixed results across tests
- Tests didn't run properly (execution failures)
- Measurements insufficient to decide
- Design flaw prevented clear answer

Think deeply: Does the evidence actually support the verdict? Be honest.

### Step 4: Generate Validation Report

Write to `experiments/{slug}/validation.md`:

```markdown
---
date_validated: [ISO timestamp]
research_doc: [Path]
experiment_design: [Path]
findings: [Path]
verdict: validated|invalidated|inconclusive
---

# Validation Report: [Hypothesis Title]

## Verdict: [VALIDATED / INVALIDATED / INCONCLUSIVE]

[1-2 sentence summary of why this verdict]

## Hypothesis Tested

**Original hypothesis**: [From research]

**What we empirically tested**: [From experiment design]

## Test Results Summary

| Test | Expected (Research) | Actual (Findings) | Verdict |
|------|---------------------|-------------------|---------|
| 1.1  | [prediction]        | [result]          | ✓/✗/?   |
| 1.2  | [prediction]        | [result]          | ✓/✗/?   |
| 2.1  | [unknown]           | [answer]          | Resolved/? |
| 3.1  | [assumption]        | [validation]      | ✓/✗/?   |

## Detailed Analysis

### Test Group 1: Core Functionality

#### Test 1.1: [Name]

**Research predicted**: "[Quote from research]"
**Success criteria**: [From experiment design]
**Actual result**: [From findings]

**Analysis**: [Your interpretation — why does this support or contradict?]

**Verdict**: Pass / Fail / Inconclusive

---

### Test Group 2: Unknowns Resolved

#### Unknown 1: [Name]

**Research question**: "[From research unknowns section]"
**Test designed**: [Test 2.1 name]
**Result**: [From findings]

**Answer**: [What we now know that we didn't before]

**Status**: ✓ Resolved / ~ Partially Resolved / ✗ Still Unknown

---

### Test Group 3: Assumptions Validated

#### Assumption 1: [Name]

**Research assumed**: "[From research assumptions section]"
**Test designed**: [Test 3.1 name]
**Result**: [From findings]

**Validation**: ✓ Correct / ✗ Incorrect / ? Inconclusive

**Impact if incorrect**: [What does this mean for the hypothesis?]

---

### Test Group 4: Edge Cases

[Same structure]

---

## Summary Tables

### Unknowns Resolution

| Unknown | Status | What We Learned |
|---------|--------|-----------------|
| [U1]    | ✓ Resolved | [Answer] |
| [U2]    | ~ Partial  | [What we know, what's still unclear] |
| [U3]    | ✗ Still unknown | [Why test didn't answer it] |

### Assumptions Validation

| Assumption | Status | Notes |
|------------|--------|-------|
| [A1]       | ✓ Correct | [Evidence] |
| [A2]       | ✗ Incorrect | [What was wrong, impact] |
| [A3]       | ? Inconclusive | [Why unclear] |

## New Questions Raised

[Things we didn't expect that need further investigation]

1. [New question from results]
2. [Unexpected behavior observed]

## Recommendations

### If VALIDATED:
- [What this enables]
- [Next steps to proceed with confidence]

### If INVALIDATED:
- [What alternatives to consider]
- [What part of the hypothesis failed]

### If INCONCLUSIVE:
- [What additional tests would resolve it]
- [What was wrong with the experiment design]

## Evidence Summary

| Category | Count | Summary |
|----------|-------|---------|
| For hypothesis | [N] | [Brief summary of supporting evidence] |
| Against hypothesis | [N] | [Brief summary of contradicting evidence] |
| Unclear | [N] | [Brief summary of inconclusive results] |

## Conclusion

[Final paragraph synthesizing what we learned and what it means for the hypothesis and future work]
```

## Working with Existing Context

If you were part of the research/design/execution:
- Review the conversation history for context
- Trust your earlier analysis unless findings contradict it
- Focus on connecting results back to original hypothesis
- Be honest about any shortcuts or incomplete tests

## Important Guidelines

1. **Be honest about the verdict**:
   - Don't stretch evidence to claim validation
   - Inconclusive is a valid outcome
   - Partial validation should still be "inconclusive" if critical tests failed

2. **Quote from all three documents**:
   - Show the connection between prediction, design, and result
   - Make reasoning transparent and traceable

3. **Distinguish result types**:
   - Core functionality tests: prove/disprove hypothesis
   - Unknown tests: answer questions (resolved/unresolved, not pass/fail)
   - Assumption tests: validate prerequisites (correct/incorrect)

4. **Identify new questions**:
   - Experiments often reveal surprises
   - Document these for future research cycles

5. **Think critically**:
   - Does the verdict actually follow from the evidence?
   - Are there alternative explanations?
   - What would change your mind?

## Validation Checklist

Always verify:
- [ ] All three documents read completely
- [ ] Each test compared: research → design → findings
- [ ] Each unknown assessed: resolved or not
- [ ] Each assumption validated: correct or not
- [ ] Verdict criteria applied strictly
- [ ] Evidence quoted with sources
- [ ] New questions documented
- [ ] Recommendations are actionable

## Completion

After generating the validation report:

```
Validation complete.

**Verdict**: [VALIDATED / INVALIDATED / INCONCLUSIVE]

**Key findings**:
- [Most important thing learned]
- [Second most important]

**Unknowns resolved**: X of Y
**Assumptions validated**: X of Y

**What this means**:
[1-2 sentences on practical implications]

Full report: `experiments/{slug}/validation.md`
```

## Relationship to Experiment Flow

Recommended workflow:
1. `/research_experiment` — Investigate feasibility
2. `/create_experiment` — Design test cases
3. `/run_experiment` — Execute and capture results
4. `/validate_experiment` — Compare results to predictions (YOU ARE HERE)

Validation closes the loop. If inconclusive, you may need to iterate:
- Back to `/create_experiment` if test design was flawed
- Back to `/research_experiment` if fundamental assumptions were wrong
