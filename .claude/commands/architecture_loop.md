---
description: Iteratively design Dual's architecture through hypothesis-driven experimentation
model: opus
---

# Architecture Loop

You validate Dual's architecture by reading the unvalidated SPEC.md, extracting testable claims, researching and experimenting on each, and building up a validated ARCHITECTURE.md.

## CRITICAL RULES

- DO NOT ask for user input — run autonomously
- DO reference `.claude/commands/` for guidance (research_experiment.md, create_experiment.md, run_experiment.md, validate_experiment.md)
- DO extract testable claims from SPEC.md automatically
- DO update thoughts/ARCHITECTURE.md with validated/rejected decisions
- DO output `ARCHITECTURE_COMPLETE` when all spec claims are validated (this stops the loop)
- Output `STOP_LOOP` at any time to exit early

## How The Loop Works

A Stop hook monitors for the completion signal. When you finish an iteration and try to stop, the hook will:
1. Check if `ARCHITECTURE_COMPLETE` was output → allow exit
2. Check if max iterations reached → allow exit
3. Otherwise → block exit and instruct you to continue

**First run**: The loop is activated automatically (creates `.claude/.architecture-loop-active`)

## Execution Flow

### Step 0: Activate Loop

On first invocation, run this Bash command to activate the loop:
```bash
touch .claude/.architecture-loop-active
```

### Step 1: Read Current State

Read both documents completely:

**SPEC.md** (the unvalidated specification):
- Parse the frontmatter for validation status
- Identify all architectural claims (explicit and implicit)
- Note the critical invariants and constraints

**thoughts/ARCHITECTURE.md** (the validated decisions):
- Parse confirmed decisions (what's already validated)
- Parse rejected approaches (what didn't work)
- Count iteration log entries

If `thoughts/ARCHITECTURE.md` doesn't exist, create it with the initial template (see Step 1a).

### Step 1a: Initialize ARCHITECTURE.md (First Run Only)

If the file doesn't exist, create `thoughts/ARCHITECTURE.md`:

```markdown
---
spec_source: SPEC.md
date_started: [ISO date]
status: in_progress
---

# Dual Architecture Validation

This document tracks the validation of architectural claims from SPEC.md through hypothesis-driven experimentation.

## Spec Reference

Source: `SPEC.md`
Status: Validating

## Confirmed Decisions

[Decisions validated through experimentation]

## Rejected Approaches

[Approaches that failed validation]

## Open Claims

[Claims from SPEC.md not yet validated - auto-populated]

## Iteration Log

[Record of each validation iteration]
```

### Step 2: Extract Testable Claims from SPEC.md

Analyze SPEC.md and extract all testable architectural claims. Claims are statements that can be empirically validated or invalidated.

**Types of claims to extract**:

1. **Mechanism Claims** — "X can do Y"
   - "Shell wrapper can transparently intercept commands"
   - "Docker exec preserves exit codes"
   - "Bind mount makes edits immediately visible"

2. **Performance Claims** — "X meets threshold Y"
   - "Hot reload picks up changes instantly" (what's instant? <500ms?)
   - "15 containers can all bind :3000 simultaneously"

3. **Integration Claims** — "X works with Y"
   - "WebSocket support through reverse proxy"
   - "*.localhost resolves natively in browsers"

4. **Constraint Claims** — "X does NOT leak/expose Y"
   - "Container abstractions don't leak through error messages"
   - "Claude never sees docker exec"

For each claim, note:
- The exact quote from SPEC.md
- The section it came from
- What would validate it
- What would invalidate it

### Step 3: Compare Claims vs Validated Decisions

Cross-reference:
- Claims extracted from SPEC.md
- Confirmed decisions in ARCHITECTURE.md
- Rejected approaches in ARCHITECTURE.md

Identify **unvalidated claims** — claims not yet confirmed or rejected.

### Step 4: Check Exit Condition

If NO unvalidated claims remain:

1. Update SPEC.md frontmatter:
   ```yaml
   status: validated
   validation_progress: X/X
   last_validated: [ISO date]
   last_validated_by: Claude
   ```

2. Output completion:
   ```
   ARCHITECTURE_COMPLETE

   Architecture validation complete after [N] iterations.

   ## Validation Summary
   - Claims validated: [X]
   - Claims rejected: [Y]
   - Spec status: [validated/partially_validated]

   ## Confirmed Architecture
   [Summary of confirmed decisions]

   ## Rejected Approaches
   [Summary of what didn't work]

   ## Spec Changes Needed
   [If any claims were rejected, what needs updating in SPEC.md]

   Ready for implementation.
   ```

3. Then STOP. The hook will see `ARCHITECTURE_COMPLETE` and allow exit.

### Step 5: Select Next Claim

From the unvalidated claims, select the next one to validate. Prioritize:

1. **Foundation claims first** — claims that other claims depend on
2. **Critical invariants** — claims marked as CRITICAL in SPEC.md
3. **Mechanism before performance** — prove it works before measuring how well

Generate a **slug** for this claim:
- "Shell wrapper can intercept commands" → `shell-interception`
- "Docker exec preserves exit codes" → `docker-exec-exitcodes`
- "Bind mount provides instant visibility" → `bind-mount-latency`

### Step 6: Formulate Hypothesis

Convert the spec claim into a testable hypothesis:

1. **Testable Hypothesis**: Restate the claim as a falsifiable statement
   - Spec: "Claude never sees containers"
   - Hypothesis: "A shell wrapper can intercept runtime commands and route them to docker exec without Claude Code detecting any difference in behavior"

2. **Assumptions**: What must be true for the spec claim to hold
   - "Docker exec preserves exit codes"
   - "TTY passthrough works"

3. **Success Criteria**: What would validate the spec claim
   - "Commands run in container, exit codes match, output matches"

4. **Failure Criteria**: What would invalidate the spec claim
   - "Detectable latency >100ms"
   - "Error messages mention docker"

### Step 7: Research

Read `.claude/commands/research_experiment.md` for guidance.

Follow the research workflow:
1. Spawn parallel knowledge agents (locator, analyst, prober, validator)
2. Research the specific claim and its dependencies
3. Identify what's known vs unknown
4. Write research document

Output: `thoughts/shared/research/[date]-ARCH-[slug].md`

### Step 8: Design Experiment

Read `.claude/commands/create_experiment.md` for guidance.

Design tests that validate or invalidate the spec claim:
- Core tests: Does the mechanism work?
- Assumption tests: Are dependencies valid?
- Edge case tests: Does it hold at boundaries?

Output: `experiments/arch-[slug]/experiment.md`

### Step 9: Run Experiment

Read `.claude/commands/run_experiment.md` for guidance.

Execute all tests, capture results.

Output: `experiments/arch-[slug]/findings.md`

### Step 10: Validate Results

Read `.claude/commands/validate_experiment.md` for guidance.

Compare results against the spec claim:
- Does the evidence support the claim?
- Are there caveats or constraints discovered?
- Does the spec need adjustment?

Output: `experiments/arch-[slug]/validation.md`

Determine verdict:
- **CONFIRMED**: Spec claim is valid as written
- **CONFIRMED_WITH_CAVEATS**: Spec claim is valid with modifications
- **REJECTED**: Spec claim is invalid

### Step 11: Update Documents

**Update ARCHITECTURE.md**:

If **CONFIRMED**:
```markdown
- **[Claim summary]**: Validated
  - Spec reference: [Section in SPEC.md]
  - Evidence: experiments/arch-[slug]/validation.md
  - Notes: [Any caveats or constraints discovered]
```

If **CONFIRMED_WITH_CAVEATS**:
```markdown
- **[Claim summary]**: Validated with modifications
  - Spec reference: [Section in SPEC.md]
  - Evidence: experiments/arch-[slug]/validation.md
  - Modification needed: [What SPEC.md should say instead]
```

If **REJECTED**:
```markdown
- **[Claim summary]**: Rejected
  - Spec reference: [Section in SPEC.md]
  - Evidence: experiments/arch-[slug]/validation.md
  - Why rejected: [What failed]
  - Alternative: [If discovered, what might work instead]
```

**Update Iteration Log**:
```markdown
- [N]: "[Claim]" → [CONFIRMED/REJECTED] (arch-[slug])
```

**Update SPEC.md frontmatter**:
```yaml
validation_progress: X/Y
last_validated: [ISO date]
```

### Step 12: Report & Continue

Output iteration summary:

```
═══════════════════════════════════════════════════════════
Iteration [N] complete
═══════════════════════════════════════════════════════════

Spec Claim: "[The claim from SPEC.md]"
Hypothesis: "[The testable hypothesis]"
Verdict: [CONFIRMED/CONFIRMED_WITH_CAVEATS/REJECTED]

Key findings:
- [Finding 1]
- [Finding 2]

Caveats/Modifications: [If any]

Claims validated: [X of Y]
Claims remaining: [N]

Artifacts:
- Research: thoughts/shared/research/[date]-ARCH-[slug].md
- Experiment: experiments/arch-[slug]/experiment.md
- Findings: experiments/arch-[slug]/findings.md
- Validation: experiments/arch-[slug]/validation.md
```

Then continue to next iteration (go back to Step 1).

## Claim Extraction Examples

From SPEC.md:

**Section: Command Routing**
> "Anything npm/pnpm/node/python/curl → container"

Extracted claims:
1. "Shell can selectively route npm commands to container"
2. "Shell can selectively route pnpm commands to container"
3. "curl localhost:X inside routed shell reaches container network"

**Section: Reverse Proxy**
> "MUST support HTTP and WebSocket (for hot reload / HMR)"

Extracted claims:
1. "Reverse proxy can forward HTTP requests by subdomain"
2. "Reverse proxy can forward WebSocket connections"
3. "WebSocket forwarding works for HMR/hot reload"

**Section: Container Lifecycle**
> "File edits on the host are immediately visible inside the container"

Extracted claims:
1. "Bind mount propagates file changes to container"
2. "File change propagation is fast enough for hot reload (<500ms)"

## Prioritization Logic

When multiple claims are unvalidated, process in this order:

1. **Layer 1 - Foundations** (must work for anything else to work):
   - Docker exec basic functionality
   - Bind mount works
   - Container networking basics

2. **Layer 2 - Core Mechanisms** (the main architectural bets):
   - Shell interception transparency
   - Command routing accuracy
   - Reverse proxy routing

3. **Layer 3 - Integration** (things working together):
   - Hot reload through bind mount + container
   - WebSocket through proxy
   - Multiple containers simultaneously

4. **Layer 4 - Polish** (nice to have, not blockers):
   - Performance thresholds
   - Edge cases
   - Error message cleanliness

## Example First Iteration

**Spec claim** (from CRITICAL: The Core Invariant):
> "Claude Code must never know it is running inside a container"

**Extracted testable claim**:
"Shell wrapper can transparently intercept runtime commands and route them to docker exec"

**Hypothesis**:
"A shell wrapper using function overrides or PATH manipulation can intercept commands like `node`, `pnpm`, `npm` and route them to `docker exec <container>` while preserving exit codes, stdout/stderr, and TTY behavior such that Claude Code cannot detect it is not running on the host."

**Research findings**:
- Prior art: direnv, autoenv, devcontainers
- Mechanism: Shell functions take precedence over PATH binaries
- Prober confirms: Docker available, bash 5.2, function override works

**Experiment tests**:
- Test 1.1: Basic interception (function called instead of binary)
- Test 1.2: Exit code preservation
- Test 1.3: TTY passthrough
- Test 2.1: Error message inspection (no "docker" leakage)

**Validation result**: CONFIRMED
- All tests pass
- No docker leakage detected
- Caveat: Need to handle edge case of `command node` bypassing function

**Architecture updated**:
- Confirmed: Shell function interception is viable
- Note: Must also override `command` builtin or document limitation

**Spec status**: 1/N claims validated
