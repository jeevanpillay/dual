# Research Experiment Evaluation Suite

Evaluation framework for testing `/research_experiment` command quality across diverse domains.

## Overview

```
┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│   Test Cases (24)   │     │  Claude Code        │     │   LLM Judge         │
│   research_cases.json│ ──▶│  Headless Mode      │ ──▶│   (Braintrust)      │
│                     │     │  /research_experiment│     │                     │
└─────────────────────┘     └─────────────────────┘     └─────────────────────┘
                                     │                           │
                                     ▼                           ▼
                              Research Output              Score + Feedback
```

## Setup

```bash
cd .claude/evals
npm install

# Set Braintrust API key
export BRAINTRUST_API_KEY=your_key

# Set Anthropic API key (for judge)
export ANTHROPIC_API_KEY=your_key
```

## Usage

### Run Full Evaluation

```bash
npm run eval
```

### Dry Run (Quick Keyword Scoring)

```bash
npm run eval:dry-run
```

### Filter by Domain

```bash
FILTER=sys npm run eval      # Systems domain only
FILTER=web npm run eval      # Web domain only
FILTER=games npm run eval    # Games domain only
```

### Run Single Case

```bash
npm run run-single sys-001-shell-wrapper-docker
npm run run-single web-002-service-worker-caching
```

### Run Specific Case in Eval

```bash
CASE_ID=sys-001-shell-wrapper-docker npm run eval
```

## Test Cases

24 cases across 8 domains:

| Domain | Cases | Description |
|--------|-------|-------------|
| systems | 3 | OS, containers, process management |
| web | 3 | Browsers, protocols, PWA |
| games | 3 | ECS, physics, networking |
| graphics | 3 | Rendering, shaders, GPU |
| databases | 3 | Storage engines, transactions |
| networking | 3 | Protocols, DNS, connections |
| ai_ml | 3 | Models, inference, optimization |
| mobile | 3 | iOS/Android, offline, deep links |

## Scoring

### Metrics

| Metric | Description |
|--------|-------------|
| `overall_score` | Judge's holistic score (0-1) |
| `keyword_coverage` | % of expected keywords found |
| `must_discover_rate` | % of critical items discovered |
| `should_discover_rate` | % of important items discovered |
| `pass` | Binary (score >= 0.7) |

### Rubric

| Score | Rating | Criteria |
|-------|--------|----------|
| 0.9-1.0 | Exceptional | All must_discover, most should_discover |
| 0.7-0.89 | Good | All must_discover, some should_discover |
| 0.5-0.69 | Adequate | Most must_discover, basic coverage |
| 0.3-0.49 | Poor | Missing critical items |
| 0.0-0.29 | Failed | Major gaps |

## Adding Test Cases

Edit `research_experiment_cases.json`:

```json
{
  "id": "domain-nnn-slug",
  "domain": "systems|web|games|graphics|databases|networking|ai_ml|mobile",
  "difficulty": "easy|medium|hard",
  "hypothesis": "The claim to research",
  "context": "Why this matters",
  "expected_findings": {
    "must_discover": ["Critical concept 1", "Critical concept 2"],
    "should_discover": ["Important concept 1"],
    "keywords": ["term1", "term2", "term3"]
  },
  "known_answer_summary": "Ground truth for judging"
}
```

## Architecture

```
.claude/evals/
├── research_experiment_cases.json  # Test cases
├── src/
│   ├── types.ts          # Type definitions
│   ├── runner.ts         # Claude Code headless execution
│   ├── judge.ts          # LLM-as-a-judge scoring
│   ├── eval.ts           # Braintrust eval definition
│   └── run-single.ts     # Single case runner
├── package.json
└── tsconfig.json
```

## Iterating on /research_experiment

1. Run eval: `npm run eval`
2. Check Braintrust dashboard for scores
3. Identify weak areas (domains, difficulty levels)
4. Modify `.claude/commands/research_experiment.md`
5. Re-run eval to measure improvement
6. Repeat until satisfied
