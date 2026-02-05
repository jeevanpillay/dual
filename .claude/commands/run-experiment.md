---
description: Execute experiment test cases and capture raw results with verification
model: sonnet
---

# Run Experiment

You are tasked with executing an approved experiment design from `experiments/{slug}/experiment.md`. These designs contain test cases with specific procedures and success criteria.

## CRITICAL: YOUR JOB IS TO EXECUTE AND CAPTURE, NOT INTERPRET

- DO NOT interpret whether results are good or bad
- DO NOT modify test procedures unless they fail to run
- DO NOT skip tests — run all of them even if earlier tests fail
- DO NOT draw conclusions — that's the Validate phase
- ONLY execute tests exactly as designed and capture raw output
- You are a lab technician running procedures, not a scientist analyzing results

## Getting Started

When given an experiment design path:
1. Read the experiment design completely
2. Check for any existing results (partially completed runs)
3. Read all prerequisite information mentioned in the design
4. **Read files fully** — never use limit/offset parameters
5. Create a todo list to track test execution progress
6. Verify prerequisites are met before starting

If no experiment design path provided:
```
I'll execute your experiment and capture results.

Please provide:
1. Path to experiment design (from /create-experiment)

Example: `/run-experiment experiments/shell-wrapper-interception/experiment.md`

I'll run each test case and document raw results.
```

## Execution Philosophy

Experiment designs are carefully crafted, but execution can be messy. Your job is to:
- Follow procedures exactly as written
- Capture everything — more data is better for validation
- Continue running all tests even when things go wrong
- Document what actually happened, not what should have happened

When procedures don't work as written:
1. **Try the procedure exactly as written first**
2. **If it fails**, document the failure with full output
3. **If you must adapt**, note the deviation clearly:
   ```
   Deviation in Test [X.Y]:
   Original procedure: [what the design says]
   Actual procedure: [what you ran instead]
   Reason: [why adaptation was necessary]
   ```
4. **Continue to next test** — don't stop the experiment

## Test Execution

For each test case in order:

### 1. Announce the Test
```
Running Test X.Y: [Name]
Testing: [What this tests]
```

### 2. Execute Procedure
- Run commands exactly as written in the design
- Use the Bash tool for all command execution
- Do not modify commands unless they literally cannot run

### 3. Capture Everything
- **Full stdout/stderr**: Copy complete output, don't truncate
- **Exit codes**: Record the exact exit code
- **Timing**: Note duration when relevant
- **Observations**: Factual notes (what happened, not why)

### 4. Document Raw Result
Record without interpretation:
```
**Raw output**:
[exact output here]

**Measurements**:
- Exit code: [N]
- Duration: [Xms]
- [Other measurements from design]

**Observations**: [Factual notes only]
```

### 5. Continue to Next Test
Even if this test "failed", proceed to the next one. Run ALL tests.

## Verification Approach

After completing each test group:

1. **Check automated verification items** from the design
2. **Update your progress** in the findings document
3. **Pause for human verification** if manual checks are needed:

```
Test Group [N] Complete - Ready for Manual Verification

Automated verification results:
- [x] Exit code was 0
- [x] Output contained expected string
- [ ] (Failed) No errors in stderr — got: [error]

Please perform the manual verification steps:
- [ ] [Manual check from design]
- [ ] [Another manual check]

Let me know when manual testing is complete so I can proceed to Test Group [N+1].
```

If instructed to execute all tests consecutively, skip intermediate pauses until the final test group.

## Findings Document

Write results to `experiments/{slug}/findings.md`:

```markdown
---
date_run: [ISO timestamp]
experiment_design: [Path to experiment.md]
status: complete|partial
tests_run: [X of Y]
duration: [Total time]
---

# Experiment Findings: [Hypothesis Title]

## Execution Summary

- **Date**: [When run]
- **Duration**: [Total time]
- **Tests executed**: [X of Y]
- **Environment**: [Relevant env info discovered during execution]

## Setup

**Commands run**:
```bash
[exact commands]
```

**Result**: [What happened — factual]

## Test Results

### Test Group 1: Core Functionality

#### Test 1.1: [Name]

**Procedure executed**:
```bash
[exact commands run]
```

**Raw output**:
```
[full stdout/stderr — do not truncate]
```

**Measurements**:
- Exit code: [N]
- Duration: [Xms]
- [Other measurements from design]

**Observations**: [Factual notes, no interpretation]

**Automated verification**:
- [x] Exit code is 0
- [ ] Output contains "expected" — Actual: [what was found]

---

#### Test 1.2: [Name]
[Same structure]

---

### Test Group 2: Unknown Validation

#### Test 2.1: [Name]
[Same structure]

---

## Teardown

**Commands run**:
```bash
[exact teardown commands]
```

**Result**: [What happened]

## Artifacts

- [List files, logs, screenshots captured]
- Location: `experiments/{slug}/measurements/`

## Deviations from Design

[Any procedures that had to be modified and why]

## Execution Notes

[Environment quirks, timing issues, unexpected behaviors — factual only]
```

## If You Get Stuck

When a procedure doesn't work as expected:

1. **Document exactly what happened** — full output, error messages
2. **Try once more** if it seems like a transient issue
3. **If still stuck**, document the failure and continue:
   ```
   Test [X.Y] could not be executed:
   Attempted procedure: [commands]
   Error encountered: [full error]
   Continuing to next test...
   ```
4. **Don't spend time debugging** — that's not your job. Capture and move on.

Use sub-tasks sparingly — mainly for:
- Running long commands in background
- Parallel test execution if tests are independent

## Resuming Work

If findings.md already exists with partial results:
1. Read the existing findings
2. Trust that completed tests are done
3. Pick up from the first test without results
4. Append new results to the existing document
5. Update the frontmatter (tests_run count, status)

## Completion

After all tests are executed:

```
Experiment execution complete.

**Summary**:
- Tests run: X of Y
- Duration: [time]
- Deviations: [count, if any]
- Findings: `experiments/{slug}/findings.md`

Ready for validation: `/validate-experiment experiments/{slug}/`
```

## What Raw Results Look Like

**Good** (raw, factual):
```
Exit code: 137
Stdout: ""
Stderr: "Killed"
Duration: 120003ms
```

**Bad** (interpreted, explanatory):
```
The test failed because the process was killed after timing out.
```

The validation phase will interpret results. Your job is faithful data capture.
