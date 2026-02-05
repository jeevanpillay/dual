/**
 * Braintrust evaluation for /research_experiment command
 *
 * Usage:
 *   npm run eval                    # Run full eval
 *   npm run eval:dry-run            # Quick scoring without LLM judge
 *   FILTER=sys npm run eval         # Filter by domain prefix
 *   CASE_ID=sys-001 npm run eval    # Run single case
 */

import { Eval, wrapTraced } from "braintrust";
import { readFile } from "fs/promises";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { runResearchExperiment } from "./runner.js";
import { judgeResearchOutput, quickScore } from "./judge.js";
import type { EvalCase, EvalCasesFile, JudgeResult } from "./types.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DRY_RUN = process.env.DRY_RUN === "true";
const FILTER = process.env.FILTER;
const CASE_ID = process.env.CASE_ID;

async function loadCases(): Promise<EvalCase[]> {
  const casesPath = join(__dirname, "..", "research_experiment_cases.json");
  const content = await readFile(casesPath, "utf-8");
  const data: EvalCasesFile = JSON.parse(content);

  let cases = data.cases;

  // Filter by case ID if specified
  if (CASE_ID) {
    cases = cases.filter((c) => c.id === CASE_ID);
    if (cases.length === 0) {
      throw new Error(`Case not found: ${CASE_ID}`);
    }
  }

  // Filter by domain prefix if specified
  if (FILTER) {
    cases = cases.filter((c) => c.id.startsWith(FILTER) || c.domain === FILTER);
  }

  return cases;
}

// Wrap the research task for tracing
const tracedResearch = wrapTraced(async function researchExperiment(
  evalCase: EvalCase
) {
  return runResearchExperiment(evalCase);
});

// Wrap the judge for tracing
const tracedJudge = wrapTraced(async function judgeOutput(
  evalCase: EvalCase,
  output: Awaited<ReturnType<typeof runResearchExperiment>>
): Promise<JudgeResult> {
  if (DRY_RUN) {
    // Quick scoring without LLM
    const score = quickScore(evalCase, output);
    return {
      score,
      keyword_coverage: score,
      must_discover_hits: 0,
      must_discover_total: evalCase.expected_findings.must_discover.length,
      should_discover_hits: 0,
      should_discover_total: evalCase.expected_findings.should_discover.length,
      reasoning: "Dry run - quick keyword-based scoring only",
      strengths: [],
      weaknesses: [],
    };
  }
  return judgeResearchOutput(evalCase, output);
});

Eval("research_experiment", {
  experimentName: DRY_RUN ? "dry-run" : undefined,

  data: loadCases,

  task: async (evalCase: EvalCase) => {
    console.log(`\n[${evalCase.id}] Running research for: ${evalCase.hypothesis.slice(0, 60)}...`);

    const output = await tracedResearch(evalCase);

    console.log(`[${evalCase.id}] Research complete (${output.duration}ms, exit: ${output.exitCode})`);

    const judgeResult = await tracedJudge(evalCase, output);

    console.log(`[${evalCase.id}] Score: ${judgeResult.score.toFixed(2)}`);

    return {
      output: output.content,
      judgeResult,
    };
  },

  scores: [
    // Overall score from judge
    {
      name: "overall_score",
      score: (args) => args.output.judgeResult.score,
    },

    // Keyword coverage
    {
      name: "keyword_coverage",
      score: (args) => args.output.judgeResult.keyword_coverage,
    },

    // Must-discover coverage (critical items)
    {
      name: "must_discover_rate",
      score: (args) => {
        const { must_discover_hits, must_discover_total } = args.output.judgeResult;
        return must_discover_total > 0 ? must_discover_hits / must_discover_total : 0;
      },
    },

    // Should-discover coverage (important items)
    {
      name: "should_discover_rate",
      score: (args) => {
        const { should_discover_hits, should_discover_total } = args.output.judgeResult;
        return should_discover_total > 0 ? should_discover_hits / should_discover_total : 0;
      },
    },

    // Binary pass/fail (score >= 0.7)
    {
      name: "pass",
      score: (args) => (args.output.judgeResult.score >= 0.7 ? 1 : 0),
    },
  ],

  metadata: (evalCase: EvalCase) => ({
    domain: evalCase.domain,
    difficulty: evalCase.difficulty,
    case_id: evalCase.id,
  }),

  trialCount: 1, // Research is deterministic-ish, no need for multiple trials
  maxConcurrency: 2, // Limit concurrent Claude Code processes
});
