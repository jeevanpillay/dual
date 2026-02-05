---
description: Iteratively design Dual's architecture through hypothesis-driven experimentation
model: opus
---

# Architecture Loop

You iteratively design Dual's architecture by reading the current state, scoping a hypothesis, researching it, experimenting, and updating the architecture document.

## CRITICAL RULES

- DO NOT use slash commands (/research_experiment, /create_experiment, etc.) — do the work directly
- DO NOT ask for user input — run autonomously
- DO spawn agents directly using the Task tool for research
- DO update thoughts/ARCHITECTURE.md with findings
- DO output `ARCHITECTURE_COMPLETE` when no open questions remain (this stops the loop)
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

Read `thoughts/ARCHITECTURE.md` completely. Parse:
- Iteration number (count entries in Iteration Log, add 1)
- Confirmed decisions (what's already decided)
- Open questions (what needs answering)
- Rejected approaches (what didn't work)

### Step 2: Check Exit Condition

If NO open questions remain:

```
ARCHITECTURE_COMPLETE

Architecture design complete after [N] iterations.

## Final Architecture
[Summary of confirmed decisions]

## Rejected Approaches
[Summary of what didn't work]

Ready for implementation.
```

Then STOP. The hook will see `ARCHITECTURE_COMPLETE` and allow exit.

### Step 3: Scope Hypothesis

For the FIRST open question, analyze it and generate:

1. **Testable Hypothesis**: A specific, falsifiable statement
   - Example: "A shell wrapper can transparently intercept runtime commands and route them to containers"

2. **Assumptions**: What must be true for this to work
   - Example: "Docker exec preserves exit codes", "TTY passthrough works"

3. **Success Criteria**: How we'll know it works
   - Example: "Commands run in container, exit codes preserved, interactive mode works"

4. **Failure Criteria**: What would disprove it
   - Example: "Latency >100ms per command", "Breaks existing shell features"

### Step 4: Research (Spawn Agents in Parallel)

Spawn these agents using the Task tool, ALL IN PARALLEL:

**Task 1 - knowledge-locator**:
```
Find information about [hypothesis topic]:
- Prior art: existing tools that do similar things
- Documentation: official docs for relevant technologies
- Implementations: open source examples
- Return specific URLs and code references
```

**Task 2 - knowledge-analyst**:
```
Analyze the mechanics of [hypothesis approach]:
- How does this technically work?
- What are the constraints and limitations?
- What are the failure modes and edge cases?
- Return technical explanation with specifics
```

**Task 3 - knowledge-prober**:
```
Verify prerequisites and probe actual behavior:
- Check required tools exist (docker, shell, etc.)
- Run quick probes to see how things actually work
- Discover any missing dependencies
- Return probe results with exact commands and outputs
```

**Task 4 - knowledge-validator**:
```
Research evidence for and against [hypothesis]:
- What supports this approach?
- What argues against it?
- What are the risks and tradeoffs?
- Return evidence with sources
```

**WAIT for all 4 agents to complete before proceeding.**

### Step 5: Synthesize Research

Combine agent findings into a research summary:
- What we learned
- Key constraints discovered
- Unknowns that remain
- Recommendation: proceed with experiment or pivot?

Write to: `thoughts/shared/research/[date]-ARCH-[slug].md`

Format:
```markdown
---
date: [ISO date]
hypothesis: "[The hypothesis]"
status: research_complete
---

# Research: [Hypothesis Title]

## Hypothesis
[The testable statement]

## Findings

### Prior Art (from knowledge-locator)
[What exists, with links]

### Technical Analysis (from knowledge-analyst)
[How it works, constraints, failure modes]

### Environment Probing (from knowledge-prober)
[What's available, what's missing, probe results]

### Evidence Assessment (from knowledge-validator)
[For/against, risks, tradeoffs]

## Unknowns
- [Things we still don't know]

## Assumptions
- [Things we're assuming are true]

## Recommendation
[Proceed / Pivot / Need more info]
```

### Step 6: Design & Run Experiment

Based on research, design simple tests:

1. **Core test**: Does the basic mechanism work?
2. **Assumption tests**: Are our assumptions valid?
3. **Edge case tests**: What happens at boundaries?

For each test:
- Write the exact commands to run
- Run them using Bash tool
- Capture the output
- Note pass/fail

Write findings to: `experiments/arch-[slug]/findings.md`

Format:
```markdown
---
date: [ISO date]
hypothesis: "[The hypothesis]"
research: thoughts/shared/research/[date]-ARCH-[slug].md
---

# Experiment Findings: [Hypothesis Title]

## Test Results

### Test 1: [Name]
**Command**:
```bash
[exact command]
```
**Output**:
```
[actual output]
```
**Result**: PASS / FAIL
**Notes**: [observations]

### Test 2: [Name]
[Same format]

## Summary
- Tests passed: [N]
- Tests failed: [N]
- Key observations: [What we learned]

## Verdict
[CONFIRMED / REJECTED / PARTIALLY_CONFIRMED]

## New Questions
- [Questions that emerged from testing]
```

### Step 7: Update Architecture Document

Based on experiment results, update `thoughts/ARCHITECTURE.md`:

**If CONFIRMED**, add to Confirmed Decisions:
```markdown
- **[Decision name]**: [What was decided]
  - Evidence: experiments/arch-[slug]/findings.md
  - Rationale: [Why this works, key evidence]
```

**If REJECTED**, add to Rejected Approaches:
```markdown
- **[Approach name]**: [What was tried]
  - Evidence: experiments/arch-[slug]/findings.md
  - Why rejected: [What failed, key evidence]
```

**Add new questions** that emerged to Open Questions.

**Remove the question** that was just answered from Open Questions.

**Add to Iteration Log**:
```markdown
- [N]: "[Question]" → [CONFIRMED/REJECTED] (arch-[slug])
```

### Step 8: Report & Continue

Output iteration summary:

```
═══════════════════════════════════════════════════════════
Iteration [N] complete
═══════════════════════════════════════════════════════════

Question: "[What was tested]"
Hypothesis: "[The hypothesis]"
Verdict: [CONFIRMED/REJECTED]

Key findings:
- [Finding 1]
- [Finding 2]

New questions added: [N]
Open questions remaining: [N]

Artifacts:
- Research: thoughts/shared/research/[date]-ARCH-[slug].md
- Findings: experiments/arch-[slug]/findings.md
```

Then continue to next iteration (go back to Step 1).

## Slug Naming Convention

Generate slug from the question:
- "What layer should Dual intercept at?" → `interception-layer`
- "How does file system boundary work?" → `filesystem-boundary`
- "What's the latency of docker exec?" → `docker-exec-latency`

## Context: What is Dual?

From CLAUDE.md:
- Terminal workspace orchestrator for parallel multi-repo development
- One workspace = one full clone = one container
- Dev tools (nvim, claude, git) run on HOST
- Runtime processes (pnpm dev, node) run in CONTAINER
- Core invariant: Claude Code must never know commands are routed to containers

## Example First Iteration

**Open question**: "What layer should Dual intercept at?"

**Scoped hypothesis**: "A shell wrapper using PROMPT_COMMAND or custom shell function can transparently intercept runtime commands (node, pnpm, npm) and route them to docker exec while allowing file operations (cat, ls, git) to run on host."

**Research agents find**:
- Prior art: direnv, autoenv, custom shell wrappers
- Mechanics: PROMPT_COMMAND runs before each command, can intercept
- Probing: Docker available, bash 5.2, PROMPT_COMMAND works
- Evidence: Similar approaches used in devcontainers

**Experiment tests**:
- Basic interception works
- Exit codes preserved
- TTY works for interactive

**Result**: CONFIRMED

**Architecture updated**:
- Confirmed: Shell wrapper approach viable
- New questions: "What's the latency overhead?", "How to handle pipes?"
