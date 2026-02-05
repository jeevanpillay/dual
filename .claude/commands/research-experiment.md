---
description: Research technical feasibility of an experiment hypothesis using parallel sub-agents
model: opus
---

# Research Experiment

You are tasked with conducting comprehensive research to assess the technical feasibility of an experiment hypothesis by spawning parallel sub-agents and synthesizing their findings.

## CRITICAL: YOUR ONLY JOB IS TO DOCUMENT AND EXPLAIN WHAT EXISTS

- DO NOT suggest improvements or changes unless the user explicitly asks for them
- DO NOT perform root cause analysis unless the user explicitly asks for them
- DO NOT propose future enhancements unless the user explicitly asks for them
- DO NOT critique implementations or identify "problems"
- DO NOT recommend refactoring, optimization, or architectural changes
- ONLY describe what exists, where it exists, how it works, and how components interact
- You are creating a technical map/documentation of existing systems and their feasibility

## Initial Setup

When this command is invoked, respond with:
```
I'm ready to research your experiment hypothesis. Please provide:
1. Your hypothesis (what are we testing?)
2. Context (why does this matter? what's the use case?)
3. Scope (what operations/features need to keep working alongside this?)

You can also provide file paths, Linear issues, or documentation links for context.

I'll research feasibility, constraints, and unknowns comprehensively.
```

Then wait for the user's input.

## Steps to follow after receiving the hypothesis:

### 1. Read any directly mentioned files first
- If the user mentions specific files (tickets, docs, JSON, Linear issues), read them FULLY first
- **IMPORTANT**: Use the Read tool WITHOUT limit/offset parameters to read entire files
- **CRITICAL**: Read these files yourself in the main context before spawning any sub-tasks
- This ensures you have full context before decomposing the research

### 2. Analyze and decompose the research question
- Break down the hypothesis into composable research areas:
  - Core feasibility: Can this technically work?
  - Contextual requirements: What operations must continue working?
  - Known approaches: How have similar problems been solved?
  - Constraints: What technical limitations exist?
- Take time to ultrathink about the underlying patterns, connections, and implications
- Identify specific technologies, patterns, or concepts to investigate
- Create a research plan using TodoWrite to track all subtasks
- Consider which documentation, codebases, or implementations are relevant

### 3. Spawn parallel sub-agent tasks for comprehensive research
- Create multiple Task agents to research different aspects concurrently

We have specialized agents that know how to do specific research tasks:
- **knowledge-locator**: Find documentation, implementations, existing solutions, specifications
- **knowledge-analyst**: Explain how technologies work, their mechanics, constraints, failure modes
- **knowledge-comparator**: Compare approaches, identify tradeoffs between alternatives
- **knowledge-validator**: Research evidence for/against claims, validate assumptions

The key is to use these agents intelligently:
- Start with locator agents to find what exists
- Then use analyst agents on the most promising findings
- Use comparator agents when multiple approaches exist
- Use validator agents to test critical assumptions
- Run multiple agents in parallel when they're researching different things
- Each agent knows its job - just tell it what you're looking for
- Don't write detailed prompts about HOW to research - the agents already know

### 4. Wait for all sub-agents to complete and synthesize findings
- **IMPORTANT**: Wait for ALL sub-agent tasks to complete before proceeding
- Compile all sub-agent results
- Prioritize primary source findings (official docs, specifications) as source of truth
- Use secondary findings (articles, discussions) as supplementary context
- Connect findings across different research areas
- Include specific references (URLs, file paths, line numbers) for all claims
- Highlight patterns, constraints, and technical decisions discovered
- Document what exists and how it works with concrete evidence
- Identify unknowns that couldn't be answered through research

### 5. Gather metadata for the research document
- Get current git information: `git rev-parse HEAD`, `git branch --show-current`
- Get repository name from directory or git remote
- Filename: `thoughts/shared/research/YYYY-MM-DD-EXP-description.md`
  - Format: `YYYY-MM-DD-EXP-description.md` where:
    - YYYY-MM-DD is today's date
    - EXP indicates experiment research
    - description is a brief kebab-case description of the hypothesis
  - Examples:
    - `2025-02-05-EXP-shell-docker-interception.md`
    - `2025-02-05-EXP-reverse-proxy-websocket-latency.md`
    - `2025-02-05-EXP-virtio-fs-event-propagation.md`

### 6. Generate research document
- Use the metadata gathered in step 5
- Structure the document with YAML frontmatter followed by content:

```markdown
---
date: [Current date and time with timezone in ISO format]
researcher: Claude
git_commit: [Current commit hash]
branch: [Current branch name]
repository: [Repository name]
hypothesis: "[Concise hypothesis statement]"
tags: [experiment, research, relevant-technology-names]
status: research_complete
last_updated: [Current date in YYYY-MM-DD format]
last_updated_by: Claude
---

# Research: [Hypothesis Title]

**Date**: [Current date and time with timezone]
**Researcher**: Claude
**Git Commit**: [Current commit hash]
**Branch**: [Current branch name]
**Repository**: [Repository name]

## Hypothesis
[Clear statement of what we're testing]

## Why This Matters
[Why does this hypothesis matter? What does it enable?]

## What We're Testing
- **Primary claim**: [Main technical assertion]
- **Success criteria**: [How would we measure success?]
- **Scope boundary**: [What counts as "working"?]

## Contextual Requirements
[What operations/features must continue working alongside this?]
- Requirement 1: [What it is, why it matters]
- Requirement 2: [What it is, why it matters]
- Edge case 1: [What could break?]

## Feasibility Assessment

### Technical Foundation
[What exists that makes this possible or impossible?]
- Finding 1: [What we discovered] ([source](link))
- Finding 2: [What we discovered] ([source](link))

### How It Works
[Technical explanation of the core mechanisms involved]
- Mechanism 1: [How it functions]
- Mechanism 2: [How it functions]

### Known Approaches
[How have similar problems been solved?]
- Approach A: [Description, how it works]
- Approach B: [Description, how it works]

### Constraints & Limitations
[What technical factors constrain this?]
- Constraint 1: [What it is, why it matters]
- Constraint 2: [What it is, why it matters]

## Detailed Findings

### [Research Area 1]
- Finding with reference ([source](link))
- How components interact
- Technical details as they exist

### [Research Area 2]
...

## Evidence Assessment

### Supporting Evidence
[What supports the hypothesis being feasible?]
- Evidence 1: [What it shows] ([source](link))
- Evidence 2: [What it shows] ([source](link))

### Contradicting Evidence
[What suggests limitations or challenges?]
- Evidence 1: [What it shows] ([source](link))
- Evidence 2: [What it shows] ([source](link))

## References
- `path/to/file.ext:123` - Description of what's there
- [Documentation Title](URL) - What it covers
- [Article/Discussion](URL) - Relevant insight

## Historical Context
[Relevant insights from thoughts/ directory or prior research]
- `thoughts/shared/research/related.md` - Prior research on X

## Unknowns & Open Questions
[What we couldn't answer through research]
- Unknown 1: [Question, why it matters, how to find out]
- Unknown 2: [Question, why it matters, how to find out]

## Assumptions Made
[What we're assuming to be true that should be validated]
- Assumption 1: [What we assume, why, how to validate]
- Assumption 2: [What we assume, why, how to validate]
```

### 7. Add GitHub permalinks (if applicable)
- Check if on main branch or if commit is pushed: `git branch --show-current` and `git status`
- If on main/master or pushed, generate GitHub permalinks:
  - Get repo info: `gh repo view --json owner,name`
  - Create permalinks: `https://github.com/{owner}/{repo}/blob/{commit}/{file}#L{line}`
- Replace local file references with permalinks in the document

### 8. Present findings
- Present a concise summary of findings to the user
- Include key references for easy navigation
- Highlight: Is this feasible? What are the critical unknowns?
- Ask if they have follow-up questions or need clarification

### 9. Handle follow-up questions
- If the user has follow-up questions, append to the same research document
- Update the frontmatter fields `last_updated` and `last_updated_by` to reflect the update
- Add `last_updated_note: "Added follow-up research for [brief description]"` to frontmatter
- Add a new section: `## Follow-up Research [timestamp]`
- Spawn new sub-agents as needed for additional investigation
- Continue updating the document

## Important notes:
- Always use parallel Task agents to maximize efficiency and minimize context usage
- Always run fresh research - never rely solely on existing research documents
- The thoughts/ directory provides historical context to supplement live findings
- Focus on finding concrete references (URLs, file paths, line numbers) for all claims
- Research documents should be self-contained with all necessary context
- Each sub-agent prompt should be specific and focused on read-only documentation operations
- Document cross-component connections and how systems interact
- Include temporal context (when the research was conducted)
- Link to GitHub when possible for permanent references
- Keep the main agent focused on synthesis, not deep file reading
- Have sub-agents document examples and usage patterns as they exist
- Explore all of thoughts/ directory for historical context
- **CRITICAL**: You and all sub-agents are documentarians, not evaluators
- **REMEMBER**: Document what IS, not what SHOULD BE
- **NO RECOMMENDATIONS**: Only describe the current state of technologies and their capabilities
- **File reading**: Always read mentioned files FULLY (no limit/offset) before spawning sub-tasks
- **Critical ordering**: Follow the numbered steps exactly
  - ALWAYS read mentioned files first before spawning sub-tasks (step 1)
  - ALWAYS wait for all sub-agents to complete before synthesizing (step 4)
  - ALWAYS gather metadata before writing the document (step 5 before step 6)
  - NEVER write the research document with placeholder values
- **Frontmatter consistency**:
  - Always include frontmatter at the beginning of research documents
  - Keep frontmatter fields consistent across all research documents
  - Update frontmatter when adding follow-up research
  - Use snake_case for multi-word field names (e.g., `last_updated`, `git_commit`)
  - Tags should be relevant to the research topic and technologies studied
