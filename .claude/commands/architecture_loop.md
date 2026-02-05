---
description: Run ONE iteration of architecture design loop, then exit (Ralph pattern)
model: opus
---

# Architecture Loop (Single Iteration)

You run ONE iteration of the architecture design loop, update state files, then EXIT. A Stop hook will re-launch you for the next iteration. This is the Ralph pattern — fresh Claude instance per iteration, state persisted in files.

## CRITICAL: ONE ITERATION THEN EXIT

- DO read state from files (ARCHITECTURE.md tracks progress)
- DO complete ONE full iteration (scope → research → experiment → validate)
- DO update ARCHITECTURE.md with findings
- DO emit EXIT_SIGNAL when no open questions remain
- DO exit cleanly after updating state
- DO NOT try to loop internally — the hook handles that
- DO NOT ask for confirmation between phases — run autonomously
- You are ONE iteration of a larger loop managed externally

## How The Ralph Loop Works

```
┌─────────────────────────────────────────────┐
│ Stop Hook (external)                         │
│ - Catches Claude exit                        │
│ - Checks output for EXIT_SIGNAL             │
│ - If no signal: re-launch with same command │
│ - If signal found: stop looping             │
└─────────────────────────────────────────────┘
        ↑ exit              ↓ launch
┌─────────────────────────────────────────────┐
│ This Command (one iteration)                 │
│ 1. Read ARCHITECTURE.md                      │
│ 2. If no open questions → EXIT_SIGNAL       │
│ 3. Scope next hypothesis                     │
│ 4. Research → Create → Run → Validate       │
│ 5. Update ARCHITECTURE.md                   │
│ 6. Exit (hook catches, re-launches)         │
└─────────────────────────────────────────────┘
```

## State Files (Memory Between Instances)

**Primary state**: `thoughts/ARCHITECTURE.md`
```markdown
# Dual Architecture
## Confirmed Decisions
- [Decision]: [Rationale] (experiment: [link])

## Open Questions
- [Next question to address]

## Rejected Approaches
- [Approach]: [Why] (experiment: [link])

## Iteration Log
- [N]: [Question] → [Outcome]
```

**Artifacts**: `experiments/arch-[slug]/` and `thoughts/shared/research/`

## Execution Flow

### Step 1: Read State

Read `thoughts/ARCHITECTURE.md` completely.

Parse:
- Current iteration number (from log)
- Confirmed decisions
- Open questions
- Rejected approaches

### Step 2: Check Exit Condition

If NO open questions remain:

```
═══════════════════════════════════════════════════════════
EXIT_SIGNAL: ARCHITECTURE_COMPLETE
═══════════════════════════════════════════════════════════

Architecture design complete after [N] iterations.

## Final Architecture
[Summary of confirmed decisions]

## Rejected Approaches
[Summary of what didn't work]

All critical questions answered. Ready for implementation.
```

Then EXIT. The hook will see EXIT_SIGNAL and stop looping.

### Step 3: Scope Hypothesis

Spawn **architecture-scoper** agent to analyze the next open question:
- Generate testable hypothesis
- Surface implicit assumptions
- Define success/failure criteria

### Step 4: Research (Parallel)

Spawn in parallel:
- **knowledge-locator**: Find prior art, docs, implementations
- **knowledge-analyst**: Analyze mechanics, constraints, failure modes
- **knowledge-prober**: Verify prerequisites, probe actual behavior
- **knowledge-validator**: Research evidence for/against

Wait for all to complete.

Write research doc to: `thoughts/shared/research/[date]-ARCH-[slug].md`

### Step 5: Create Experiment

Design test plan from research:
- Unknowns → test cases
- Assumptions → validation tests
- Clear pass/fail criteria

Write to: `experiments/arch-[slug]/experiment.md`

### Step 6: Run Experiment

Execute tests, capture raw results.

Write to: `experiments/arch-[slug]/findings.md`

### Step 7: Validate & Update State

Compare results to predictions. Determine verdict.

Update `thoughts/ARCHITECTURE.md`:

**If confirmed**:
```markdown
## Confirmed Decisions
- **[Decision]**: [What we learned]
  - Experiment: experiments/arch-[slug]/
  - Rationale: [Evidence from findings]
```

**If rejected**:
```markdown
## Rejected Approaches
- **[Approach]**: [What we tried]
  - Experiment: experiments/arch-[slug]/
  - Why rejected: [Evidence from findings]
```

**Add new questions** that emerged:
```markdown
## Open Questions
- [New question from experiment]
```

**Log the iteration**:
```markdown
## Iteration Log
- [N]: "[Question]" → [CONFIRMED/REJECTED] ([slug])
```

### Step 8: Exit

After updating state, exit cleanly. Output:

```
═══════════════════════════════════════════════════════════
Iteration [N] complete.
═══════════════════════════════════════════════════════════

Question: "[What was tested]"
Verdict: [CONFIRMED/REJECTED]
New questions: [N] added

State updated in thoughts/ARCHITECTURE.md
Exiting for next iteration...
```

The Stop hook catches this exit and re-launches for iteration N+1.

## Hook Configuration

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [
      {
        "command": "if ! grep -q 'EXIT_SIGNAL' /tmp/claude-last-output; then echo '/architecture_loop'; fi"
      }
    ]
  }
}
```

Or use a script for more control:

```bash
#!/bin/bash
# .claude/hooks/ralph-loop.sh

OUTPUT=$(cat /tmp/claude-last-output)

if echo "$OUTPUT" | grep -q "EXIT_SIGNAL"; then
    echo "Architecture complete. Stopping loop."
    exit 0
fi

# Re-launch with same command
echo "/architecture_loop"
```

## Important Notes

1. **Autonomous execution** — Don't pause for user input mid-iteration
2. **State is in files** — Each instance reads/writes ARCHITECTURE.md
3. **Clean exits** — Always exit after one iteration
4. **EXIT_SIGNAL** — Only emit when truly complete
5. **Artifacts persist** — Research docs and experiments accumulate
