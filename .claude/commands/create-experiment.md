---
description: Convert research document into structured experiment test plan
model: opus
---

# Create Experiment

You are tasked with converting a research document into a structured experiment test plan. The research phase has already done the heavy lifting — desk research, empirical probing, prerequisite validation. Your job is to extract the unknowns and assumptions and design formal test cases for them.

## CRITICAL: YOUR JOB IS TO DESIGN TESTS, NOT RESEARCH OR RUN THEM

- DO NOT do additional research — that's already in the research doc
- DO NOT run commands to check prerequisites — research already did that
- DO NOT execute tests yourself — that's the Run phase
- DO extract unknowns/assumptions from research and convert them to test cases
- DO define clear success/failure criteria for each test
- You are a test designer converting research findings into executable experiments

## Understanding the Experiment Flow

```
Research Phase (DONE)              Create Phase (YOU ARE HERE)
┌─────────────────────────────┐   ┌─────────────────────────────┐
│ Desk research + probing     │   │ Read research doc           │
│ Prerequisites validated     │ → │ Extract unknowns/assumptions│
│ Unknowns identified         │   │ Design test cases           │
│ Assumptions documented      │   │ Define success criteria     │
└─────────────────────────────┘   └─────────────────────────────┘
                                               ↓
Run Phase                          Validate Phase
┌─────────────────────────────┐   ┌─────────────────────────────┐
│ Execute tests (full send)   │   │ Compare results to research │
│ Capture raw results         │ → │ Verdict: pass/fail/unclear  │
│ No interpretation           │   │ What did we learn?          │
└─────────────────────────────┘   └─────────────────────────────┘
```

## Initial Response

When this command is invoked:

1. **Check if research document path was provided**:
   - If yes, read it FULLY and begin designing
   - If no, ask for it

2. **If no parameters**, respond with:
```
I'll convert your research into a structured experiment.

Please provide the path to the research document (from /research-experiment).

Example: `/create-experiment thoughts/shared/research/2026-02-05-EXP-shell-wrapper.md`
```

## Process Steps

### Step 1: Read Research Document Fully

- Read the entire research document WITHOUT limit/offset
- Extract these key elements:
  - **Hypothesis**: What are we ultimately testing?
  - **Unknowns**: Questions that couldn't be answered (→ become test cases)
  - **Assumptions**: Things assumed true (→ become validation tests)
  - **Prerequisites**: What's already verified present
  - **Success criteria**: What would "working" look like?

### Step 2: Present Extraction to User

```
I've read the research document. Here's what needs structured testing:

**Hypothesis**: [From research]

**Unknowns to test** (research couldn't answer these):
1. [Unknown 1] — will become Test Group 2
2. [Unknown 2] — will become Test Group 2

**Assumptions to validate** (research assumed these):
1. [Assumption 1] — will become Test Group 3
2. [Assumption 2] — will become Test Group 3

**Prerequisites already verified**:
- [Tool/system] ✓ (version X)

Does this look right? Any unknowns/assumptions to add or skip?
```

### Step 3: Design Test Cases

Map research sections directly to test groups:

| Research Section | Test Group |
|------------------|------------|
| Core claims / How It Works | Test Group 1: Core Functionality |
| Unknowns & Open Questions | Test Group 2: Unknown Validation |
| Assumptions Made | Test Group 3: Assumption Validation |
| Edge Cases / Constraints | Test Group 4: Edge Cases |

For each test:
- **Procedure**: Exact commands to run (copy-paste ready)
- **Expected result**: What research predicts
- **Measurements**: What to capture
- **Success/failure criteria**: How to judge the result

### Step 4: Write Experiment Design

Create file: `experiments/{experiment-slug}/experiment.md`

```markdown
---
date_created: [ISO timestamp]
research_doc: [Path to research document]
hypothesis: "[From research]"
status: design_complete
---

# Experiment: [Hypothesis Title]

## Research Reference

- **Document**: [Path to research doc]
- **Hypothesis**: [From research]
- **Date researched**: [From research doc]

## Experiment Objective

[What we're empirically testing — derived from hypothesis]

### What Success Looks Like
[From research success criteria]

### What Failure Looks Like
[What would disprove the hypothesis]

## Prerequisites

[Copied from research — already verified]
| Tool/System | Version | Verified |
|-------------|---------|----------|
| [tool]      | [ver]   | ✓ Research confirmed |

## Test Cases

### Test Group 1: Core Functionality

Tests the primary claims from research.

#### Test 1.1: [Descriptive Name]

**Tests**: [What aspect of hypothesis]
**Research said**: "[Quote from research findings]"

**Procedure**:
```bash
[exact commands — copy-paste ready]
```

**Expected result**: [What research predicts]

**Measurements**:
- [ ] [What to capture]
- [ ] [What to capture]

**Pass**: [Specific criteria]
**Fail**: [Specific criteria]

---

### Test Group 2: Unknown Validation

Tests questions research couldn't answer.

#### Test 2.1: [Unknown Name]

**Tests**: Unknown from research
**Research asked**: "[Quote from Unknowns section]"

**Procedure**:
```bash
[commands to answer the unknown]
```

**Expected result**: Unknown — this test will tell us

**Measurements**:
- [ ] [What to capture]

**Resolves unknown if**: [What would answer it]
**Still unknown if**: [What would leave it unclear]

---

### Test Group 3: Assumption Validation

Tests things research assumed true.

#### Test 3.1: [Assumption Name]

**Tests**: Assumption from research
**Research assumed**: "[Quote from Assumptions section]"

**Procedure**:
```bash
[commands to verify assumption]
```

**Expected result**: [What would confirm assumption]

**Measurements**:
- [ ] [What to capture]

**Assumption valid if**: [Criteria]
**Assumption invalid if**: [Criteria]

---

### Test Group 4: Edge Cases

Tests constraints and edge cases from research.

#### Test 4.1: [Edge Case Name]

**Tests**: [Edge case from research]
**Research identified**: "[Quote from constraints/edge cases]"

**Procedure**:
```bash
[commands to test edge case]
```

**Expected behavior**: [What should happen]

**Measurements**:
- [ ] [What to capture]

**Handles correctly if**: [Criteria]
**Breaks if**: [Criteria]

---

## Execution Order

1. Test Group 1 (core functionality first)
2. Test Group 2 (unknowns)
3. Test Group 3 (assumptions)
4. Test Group 4 (edge cases)

## Expected Artifacts

- `findings.md` — Raw results from Run phase
- `measurements/` — Captured outputs, logs

## Notes for Run Phase

- Run all tests even if earlier ones fail
- Capture exact output, don't interpret
- Note any deviations from procedure
```

### Step 5: Present and Confirm

```
I've created the experiment design at:
`experiments/{slug}/experiment.md`

**Summary**:
- [N] core functionality tests
- [N] unknown validation tests
- [N] assumption validation tests
- [N] edge case tests

Please review:
- Are test procedures correct and executable?
- Are success/failure criteria clear?
- Any tests to add or remove?

When ready, run with: `/run-experiment experiments/{slug}/experiment.md`
```

## Important Guidelines

1. **Every unknown needs a test**: Research identified these as unanswered — test them
2. **Every assumption needs validation**: Research assumed these — verify them
3. **Tests must be executable**: Real commands, copy-paste ready
4. **Criteria must be measurable**: "exit code is 0" not "it works"
5. **Quote the research**: Show where each test comes from
6. **Don't re-research**: Trust the research doc, just convert to tests
7. **Keep it simple**: One test per unknown/assumption when possible

## What You DON'T Do

- ❌ Run commands to check prerequisites (research did this)
- ❌ Do additional desk research (research did this)
- ❌ Probe the environment (research did this)
- ❌ Execute the tests (Run phase does this)
- ❌ Interpret results (Validate phase does this)

## What You DO

- ✓ Read research document fully
- ✓ Extract unknowns, assumptions, edge cases
- ✓ Convert each to a formal test case
- ✓ Define clear success/failure criteria
- ✓ Write executable procedures
- ✓ Create the experiment.md file
