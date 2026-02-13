---
description: Iteratively build Dual's MVP through plan-driven implementation
model: opus
---

# Build Loop

You build Dual's MVP by reading the validated ARCHITECTURE.md, extracting implementable modules, planning and building each, and tracking progress in a living BUILD.md.

## CRITICAL RULES

- DO NOT ask for user input — run autonomously
- DO reference `.claude/commands/` for guidance (research_codebase.md, create_plan.md, implement_plan.md, validate_plan.md)
- DO extract implementable modules from ARCHITECTURE.md confirmed decisions
- DO update thoughts/BUILD.md with built/failed modules
- DO output `BUILD_COMPLETE` when all modules are built and verified (this stops the loop)
- Output `STOP_LOOP` at any time to exit early

## How The Loop Works

A Stop hook monitors for the completion signal. When you finish an iteration and try to stop, the hook will:
1. Check if `BUILD_COMPLETE` was output → allow exit
2. Check if max iterations reached → allow exit
3. Otherwise → block exit and instruct you to continue

**First run**: The loop is activated automatically (creates `.claude/.build-loop-active`)

## Execution Flow

### Step 0: Activate Loop

On first invocation, run this Bash command to activate the loop:
```bash
touch .claude/.build-loop-active
```

### Step 1: Read Current State

Read all three documents completely:

**SPEC.md** (the requirements):
- Parse the frontmatter for validation status
- Identify functional requirements and constraints
- Note the critical invariants

**thoughts/ARCHITECTURE.md** (the validated decisions):
- Parse confirmed decisions (what's architecturally proven)
- Note caveats and constraints discovered during validation
- Identify implementation implications from each decision

**thoughts/BUILD.md** (the implementation tracker):
- Parse built modules (what's already implemented and verified)
- Parse failed modules (what didn't work)
- Count iteration log entries

If `thoughts/BUILD.md` doesn't exist, create it with the initial template (see Step 1a).

### Step 1a: Initialize BUILD.md (First Run Only)

If the file doesn't exist, create `thoughts/BUILD.md`:

```markdown
---
spec_source: SPEC.md
arch_source: thoughts/ARCHITECTURE.md
date_started: [ISO date]
status: in_progress
build_progress: 0/N
---

# Dual MVP Build

This document tracks the implementation of Dual's MVP modules, informed by validated architectural decisions from ARCHITECTURE.md.

## Architecture Reference

Source: `thoughts/ARCHITECTURE.md`
Status: Complete (24/24 validated)

## Built Modules

[Modules implemented and verified]

## Failed Modules

[Modules that failed implementation — needs rework]

## Unbuilt Modules

Modules extracted from ARCHITECTURE.md and SPEC.md, organized by dependency order:

### Layer 1 - Foundations (no upstream dependencies)

1. **cli**: Entry point, arg parsing (`dual`, `dual list`, `dual destroy`)
   - Depends on: nothing
   - Blocks: everything (entry point)

2. **config**: Workspace config format and discovery (`dual.toml`)
   - Depends on: nothing
   - Blocks: clone, container, shell, tmux

### Layer 2 - Core Mechanisms (depend on foundations)

3. **clone**: Full clone management (`git clone --local`, filesystem layout)
   - Depends on: config (workspace definitions)
   - Blocks: container (needs clone dir for bind mount)
   - Architecture: full-clone-no-contention (CONFIRMED WITH CAVEATS)

4. **container**: Docker lifecycle (create, start, stop, destroy)
   - Depends on: config (image config), clone (bind mount source)
   - Blocks: shell (needs container target for docker exec)
   - Architecture: docker-exec-basic (CONFIRMED), bind-mount-visibility (CONFIRMED WITH CAVEATS), container-network-isolation (CONFIRMED), node-modules-isolation (CONFIRMED)

### Layer 3 - Integration (compose mechanisms)

5. **shell**: RC injection + command routing (shell functions → docker exec)
   - Depends on: container (needs running container to route to)
   - Blocks: tmux (shell interceptors must be active in tmux panes)
   - Architecture: shell-interception (CONFIRMED WITH CAVEATS), shell-interception-transparency (CONFIRMED WITH CAVEATS), command-routing-accuracy (CONFIRMED)

6. **tmux**: Session create/attach/detach/destroy + workspace switching
   - Depends on: shell (panes need interceptors), clone (CWD), container (lifecycle)
   - Blocks: nothing (terminal module)
   - Architecture: tmux-backend-viable (CONFIRMED), progressive-enhancement (CONFIRMED)

## Iteration Log

[Record of each build iteration]
```

### Step 2: Extract Implementable Modules

Analyze ARCHITECTURE.md confirmed decisions and SPEC.md requirements to identify all modules needed for MVP.

**Types of modules to extract**:

1. **Foundation Modules** — No upstream dependencies
   - Entry points, config parsing, data structures
   - Must be built first

2. **Mechanism Modules** — Core functionality
   - Clone management, container lifecycle
   - Depend on foundations

3. **Integration Modules** — Compose mechanisms
   - Shell interception, tmux session management
   - Depend on mechanisms working

4. **Polish Modules** — User-facing refinement (defer for MVP)
   - Reverse proxy, TUI, auto-detection
   - Depend on integration working

For each module, note:
- The architecture decisions that constrain it
- The caveats discovered during validation
- What it depends on
- What depends on it

### Step 3: Compare Modules vs Built

Cross-reference:
- Modules extracted from ARCHITECTURE.md + SPEC.md
- Built modules in BUILD.md
- Failed modules in BUILD.md

Identify **unbuilt modules** — modules not yet built or verified.

### Step 4: Check Exit Condition

If NO unbuilt modules remain:

1. Update BUILD.md frontmatter:
   ```yaml
   status: complete
   build_progress: N/N
   date_completed: [ISO date]
   ```

2. Output completion:
   ```
   BUILD_COMPLETE

   MVP build complete after [N] iterations.

   ## Build Summary
   - Modules built: [X]
   - Modules failed: [Y]
   - Build status: [complete/partial]

   ## Built Modules
   [Summary of what was built]

   ## Failed Modules
   [Summary of what failed and why]

   ## Integration Status
   [How well the modules work together]

   Ready for integration testing.
   ```

3. Then STOP. The hook will see `BUILD_COMPLETE` and allow exit.

### Step 5: Select Next Module

From the unbuilt modules, select the next one to build. Prioritize:

1. **Foundation modules first** — modules that other modules depend on
2. **Dependency order** — never build a module before its dependencies
3. **Mechanism before integration** — prove the parts work before composing them

Generate a **slug** for this module:
- "Entry point and arg parsing" → `cli`
- "Workspace config format" → `config`
- "Git clone management" → `clone`

### Step 6: Formulate Build Target

Convert the module into a build target:

1. **Build Target**: Restate the module as a specific deliverable
   - Module: "config"
   - Target: "Implement workspace config parsing from dual.toml with repo definitions, branch lists, and base image configuration"

2. **Architecture Constraints**: Pull relevant validated decisions
   - "full-clone-no-contention" → branches are independent workspaces
   - "monorepo-single-container" → one base image per repo

3. **Success Criteria**: What would verify the module works
   - "Config file parses without error"
   - "Invalid config produces clear error messages"
   - "Workspace definitions accessible to downstream modules"

4. **Failure Criteria**: What would indicate the module is broken
   - "Cannot parse valid config"
   - "Missing validation for required fields"

### Step 7: Research

Read `.claude/commands/research_codebase.md` for guidance.

Follow the research workflow:
1. Spawn parallel agents (codebase-locator, codebase-analyzer, codebase-pattern-finder)
2. Research existing code, patterns, and conventions in the codebase
3. Identify what exists vs what needs to be written
4. Identify Rust crate dependencies needed
5. Write research document

Output: `thoughts/shared/research/[date]-BUILD-[slug].md`

### Step 8: Create Plan

Read `.claude/commands/create_plan.md` for guidance.

Create an implementation plan for this module:
- Current state: what exists in the codebase
- Desired end state: what the module delivers
- Phase breakdown: incremental, testable steps
- Success criteria: automated + manual verification
- Architecture constraints: validated decisions that bound the implementation

Output: `thoughts/shared/plans/[date]-BUILD-[slug].md`

### Step 9: Implement Plan

Read `.claude/commands/implement_plan.md` for guidance.

Execute the implementation plan:
- Follow the plan's phases sequentially
- Run success criteria after each phase
- Fix issues before proceeding
- Update checkboxes in the plan

Output: Working code in `src/`

### Step 10: Validate Implementation

Read `.claude/commands/validate_plan.md` for guidance.

Verify the implementation against the plan:
- Run all automated verification (cargo build, cargo test, cargo clippy)
- Compare actual code against planned code
- Check that architecture constraints are respected
- Identify deviations or issues

Output: Validation report

Determine verdict:
- **BUILT**: Module implemented and verified
- **BUILT_WITH_CAVEATS**: Module works with limitations
- **FAILED**: Module does not meet success criteria

### Step 11: Update Documents

**Update BUILD.md**:

If **BUILT**:
```markdown
- **[slug]**: [Module description] - BUILT
  - Plan: thoughts/shared/plans/[date]-BUILD-[slug].md
  - Evidence: cargo test passes, manual verification complete
  - Notes: [Any implementation notes]
```

If **BUILT_WITH_CAVEATS**:
```markdown
- **[slug]**: [Module description] - BUILT WITH CAVEATS
  - Plan: thoughts/shared/plans/[date]-BUILD-[slug].md
  - Evidence: cargo test passes with limitations
  - Caveats: [What doesn't fully work yet]
```

If **FAILED**:
```markdown
- **[slug]**: [Module description] - FAILED
  - Plan: thoughts/shared/plans/[date]-BUILD-[slug].md
  - Why failed: [What went wrong]
  - Next steps: [What to try differently]
```

**Update Iteration Log**:
```markdown
- [N]: "[slug]" → [BUILT/FAILED] (plans/[date]-BUILD-[slug].md)
```

**Update BUILD.md frontmatter**:
```yaml
build_progress: X/Y
```

### Step 12: Report & Continue

Output iteration summary:

```
═══════════════════════════════════════════════════════════
Iteration [N] complete
═══════════════════════════════════════════════════════════

Module: "[slug]"
Build Target: "[The specific deliverable]"
Verdict: [BUILT/BUILT_WITH_CAVEATS/FAILED]

Key outcomes:
- [Outcome 1]
- [Outcome 2]

Caveats/Limitations: [If any]

Modules built: [X of Y]
Modules remaining: [N]

Artifacts:
- Research: thoughts/shared/research/[date]-BUILD-[slug].md
- Plan: thoughts/shared/plans/[date]-BUILD-[slug].md
- Code: src/[relevant paths]
```

Then continue to next iteration (go back to Step 1).

## Module Extraction Examples

From ARCHITECTURE.md confirmed decisions:

**Decision: docker-exec-basic (CONFIRMED)**
> Docker exec can run commands in a container

Extracted module:
1. "Container lifecycle management — create, start, stop, destroy via Docker API"
2. "Command execution — docker exec wrapper with exit code preservation"

**Decision: shell-interception (CONFIRMED WITH CAVEATS)**
> Shell wrapper can intercept runtime commands

Extracted module:
1. "Shell RC injection — generate shell functions that intercept runtime commands"
2. "Command router — classify commands as host vs container, route accordingly"

Caveat to respect:
- "Functions work for interactive + Claude Code. Absolute paths bypass (acceptable). Needs rc injection + PATH shims."

**Decision: tmux-backend-viable (CONFIRMED)**
> tmux backend provides session management

Extracted module:
1. "tmux backend — create/attach/detach/destroy sessions, manage panes"

## Prioritization Logic

When multiple modules are unbuilt, process in this order:

1. **Layer 1 - Foundations** (must exist for anything else to work):
   - CLI entry point
   - Config parsing

2. **Layer 2 - Core Mechanisms** (the main implementation work):
   - Clone management
   - Container lifecycle

3. **Layer 3 - Integration** (composing mechanisms):
   - Shell interception + command routing
   - tmux session management

4. **Layer 4 - Polish** (defer for MVP):
   - Reverse proxy
   - Auto image generation
   - TUI

## Example First Iteration

**Module** (from Layer 1):
> cli — entry point, arg parsing

**Build target**:
"Implement the `dual` CLI binary with subcommand parsing (no subcommand = launch, `list` = show workspaces, `destroy` = teardown workspace) using clap, with help text and version output."

**Research findings**:
- Cargo.toml already exists with edition 2024
- No existing CLI code in src/
- clap is the standard Rust CLI framework
- Architecture requires: `dual`, `dual list`, `dual destroy`

**Plan phases**:
- Phase 1: Add clap dependency, define subcommands
- Phase 2: Implement arg parsing with validation
- Phase 3: Wire up stub handlers for each subcommand

**Implementation**:
- src/main.rs with clap derive macros
- Subcommand enum: Launch (default), List, Destroy
- Stub handlers that print "not yet implemented"

**Validation result**: BUILT
- cargo build succeeds
- cargo test passes
- cargo clippy clean
- `dual --help` shows correct usage
- `dual list` prints stub message

**BUILD.md updated**:
- Built: cli module
- Note: Stub handlers only — real implementation comes from downstream modules
