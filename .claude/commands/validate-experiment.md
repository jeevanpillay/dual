---
description: Compare experiment results to research predictions and determine verdict
model: opus
---

# Validate Experiment

You are tasked with comparing experiment results against research predictions, determining whether the hypothesis was validated, and identifying what we learned.

## CRITICAL: YOUR JOB IS TO INTERPRET AND CONCLUDE

- DO NOT re-run tests — use the findings from Run phase
- DO NOT add new tests — that would require Create phase
- DO evaluate results against research predictions
- DO determine if unknowns were answered
- DO conclude whether hypothesis is validated/invalidated/inconclusive
- You are a scientist analyzing data, not a technician collecting it

## Understanding the Experiment Flow

```
Research Phase                    Create Phase
┌─────────────────────┐          ┌─────────────────────────────┐
│ Desk research       │          │ Designed empirical tests    │
│ Predicted behavior  │          │ Defined success criteria    │
│ Unknowns identified │          │ Specified measurements      │
└─────────────────────┘          └─────────────────────────────┘
         ↓                                    ↓
                    Validate Phase (YOU ARE HERE)
                    ┌─────────────────────────────┐
    Run Phase   →   │ Compare results to research │
┌─────────────────┐ │ Verdict: pass/fail/unclear  │
│ Raw results     │ │ Unknowns resolved?          │
│ Measurements    │ │ New questions raised?       │
└─────────────────┘ └─────────────────────────────┘
```

## Initial Response

When this command is invoked:

1. **Check if paths were provided**:
   - Research document path
   - Experiment design path
   - Findings path
   - If yes, read all three FULLY

2. **If no parameters**, respond with:
```
I'll validate your experiment results against research predictions.

Please provide:
1. Path to research document
2. Path to experiment design
3. Path to findings from run phase

I'll compare results, determine verdict, and identify what we learned.
```

## Process Steps

### Step 1: Read All Documents

1. Read research document FULLY — understand predictions
2. Read experiment design FULLY — understand success criteria
3. Read findings FULLY — understand actual results

### Step 2: Compare Each Test Result

For each test case:

1. **What did research predict?** (from research doc)
2. **What was the success criteria?** (from experiment design)
3. **What actually happened?** (from findings)
4. **Verdict**: Pass / Fail / Inconclusive

### Step 3: Assess Unknowns

For each unknown from research:

1. **What was the question?**
2. **What test was designed?**
3. **What did we learn?**
4. **Is the unknown now resolved?**

### Step 4: Assess Assumptions

For each assumption from research:

1. **What did we assume?**
2. **What test validated it?**
3. **Was assumption correct?**

### Step 5: Determine Overall Verdict

- **VALIDATED**: Evidence strongly supports hypothesis
- **INVALIDATED**: Evidence contradicts hypothesis
- **INCONCLUSIVE**: Evidence is mixed or insufficient

### Step 6: Write Validation Report

Create file: `experiments/{slug}/validation.md`

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
| 2.1  | [prediction]        | [result]          | ✓/✗/?   |
| ...  | ...                 | ...               | ...     |

## Detailed Analysis

### Test Group 1: Core Functionality

#### Test 1.1: [Name]

**Research predicted**: [Quote from research]
**Success criteria**: [From experiment design]
**Actual result**: [From findings]

**Analysis**: [Your interpretation — why does this result support or contradict?]

**Verdict**: Pass / Fail / Inconclusive

---

### Test Group 2: Unknowns Validation

#### Unknown 1: [Name]

**Research question**: [From research unknowns section]
**Test designed**: [Test 2.1 name]
**Result**: [From findings]

**Answer**: [What we now know that we didn't before]

**Status**: Resolved / Partially Resolved / Still Unknown

---

### Test Group 3: Assumptions Validation

#### Assumption 1: [Name]

**Research assumed**: [From research assumptions section]
**Test designed**: [Test 3.1 name]
**Result**: [From findings]

**Validation**: Assumption Correct / Assumption Incorrect / Inconclusive

**Impact if incorrect**: [What does this mean for the hypothesis?]

---

## Unknowns Resolved

| Unknown | Status | What We Learned |
|---------|--------|-----------------|
| [U1]    | ✓ Resolved | [Answer] |
| [U2]    | ~ Partial  | [What we know, what's still unclear] |
| [U3]    | ✗ Still unknown | [Why test didn't answer it] |

## Assumptions Validated

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

**For hypothesis**: [Count and summary of supporting evidence]
**Against hypothesis**: [Count and summary of contradicting evidence]
**Unclear**: [Count and summary of inconclusive results]

## Conclusion

[Final paragraph synthesizing what we learned and what it means]
```

### Step 7: Present Summary

```
Validation complete.

**Verdict**: [VALIDATED / INVALIDATED / INCONCLUSIVE]

**Key findings**:
- [Most important thing learned]
- [Second most important]

**Unknowns resolved**: X of Y
**Assumptions validated**: X of Y

Full report: `experiments/{slug}/validation.md`
```

## Important Guidelines

1. **Be honest about the verdict**:
   - Don't stretch evidence to claim validation
   - Inconclusive is a valid outcome
   - Partial validation is common

2. **Quote from all three documents**:
   - Show the connection between prediction, design, and result
   - Make reasoning transparent

3. **Distinguish result types**:
   - Core functionality tests prove/disprove hypothesis
   - Unknown tests answer questions (no pass/fail)
   - Assumption tests validate prerequisites

4. **Identify new questions**:
   - Experiments often reveal surprises
   - Document these for future research

5. **Recommend next steps**:
   - What does this verdict mean practically?
   - What should happen next?

## Verdict Criteria

**VALIDATED** requires:
- Core functionality tests pass
- Critical assumptions validated
- No contradicting evidence

**INVALIDATED** requires:
- Core functionality tests fail, OR
- Critical assumption proven wrong, OR
- Strong contradicting evidence

**INCONCLUSIVE** when:
- Mixed results across tests
- Tests didn't run properly
- Measurements insufficient to decide
- Design flaw prevented clear answer
