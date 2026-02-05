/**
 * LLM-as-a-Judge for evaluating research experiment outputs
 */

import Anthropic from "@anthropic-ai/sdk";
import type { EvalCase, ResearchOutput, JudgeResult } from "./types.js";

const anthropic = new Anthropic();

const JUDGE_SYSTEM_PROMPT = `You are an expert evaluator assessing the quality of technical research documents.

Your job is to evaluate how well a research document answers a given hypothesis by comparing it against expected findings.

You must be:
- STRICT about "must_discover" items - these are critical and missing them is a significant failure
- FAIR about "should_discover" items - these improve quality but aren't mandatory
- THOROUGH in checking keyword coverage - indicates depth of research
- OBJECTIVE in your scoring - use the rubric precisely

Scoring rubric (0-1 scale):
- 0.9-1.0: Exceptional - All must_discover, most should_discover, comprehensive coverage
- 0.7-0.89: Good - All must_discover, some should_discover, solid coverage
- 0.5-0.69: Adequate - Most must_discover, basic coverage
- 0.3-0.49: Poor - Missing critical must_discover items
- 0.0-0.29: Failed - Major gaps, doesn't address hypothesis

Output your evaluation as JSON matching this schema:
{
  "score": <number 0-1>,
  "keyword_coverage": <number 0-1>,
  "must_discover_hits": <number>,
  "must_discover_total": <number>,
  "should_discover_hits": <number>,
  "should_discover_total": <number>,
  "reasoning": "<detailed explanation of score>",
  "strengths": ["<strength 1>", "<strength 2>"],
  "weaknesses": ["<weakness 1>", "<weakness 2>"]
}`;

export async function judgeResearchOutput(
  evalCase: EvalCase,
  output: ResearchOutput
): Promise<JudgeResult> {
  const userPrompt = formatJudgePrompt(evalCase, output);

  const response = await anthropic.messages.create({
    model: "claude-sonnet-4-20250514",
    max_tokens: 2000,
    system: JUDGE_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: userPrompt,
      },
    ],
  });

  const content = response.content[0];
  if (content.type !== "text") {
    throw new Error("Unexpected response type from judge");
  }

  // Extract JSON from response
  const jsonMatch = content.text.match(/\{[\s\S]*\}/);
  if (!jsonMatch) {
    throw new Error("Failed to extract JSON from judge response");
  }

  const result = JSON.parse(jsonMatch[0]) as JudgeResult;
  return result;
}

function formatJudgePrompt(evalCase: EvalCase, output: ResearchOutput): string {
  return `## Evaluation Task

Evaluate the following research document against the expected findings.

### Hypothesis Being Researched
${evalCase.hypothesis}

### Context
${evalCase.context}

### Expected Findings

**Must Discover (Critical):**
${evalCase.expected_findings.must_discover.map((item, i) => `${i + 1}. ${item}`).join("\n")}

**Should Discover (Important):**
${evalCase.expected_findings.should_discover.map((item, i) => `${i + 1}. ${item}`).join("\n")}

**Keywords to Check For:**
${evalCase.expected_findings.keywords.join(", ")}

### Known Correct Answer Summary
${evalCase.known_answer_summary}

---

### Research Document to Evaluate

${output.content}

---

Now evaluate this research document. Check each must_discover and should_discover item.
Count how many keywords appear in the document.
Compare the depth and accuracy against the known answer summary.

Provide your evaluation as JSON.`;
}

/**
 * Quick keyword-based scoring (no LLM call)
 * Useful for fast filtering or dry runs
 */
export function quickScore(evalCase: EvalCase, output: ResearchOutput): number {
  const content = output.content.toLowerCase();

  // Check keyword coverage
  const keywordHits = evalCase.expected_findings.keywords.filter((kw) =>
    content.includes(kw.toLowerCase())
  ).length;
  const keywordCoverage =
    keywordHits / evalCase.expected_findings.keywords.length;

  // Check must_discover (simple string matching)
  const mustHits = evalCase.expected_findings.must_discover.filter((item) => {
    const words = item.toLowerCase().split(/\s+/);
    // At least half the words should appear
    const matches = words.filter((w) => content.includes(w)).length;
    return matches >= words.length / 2;
  }).length;
  const mustCoverage =
    mustHits / evalCase.expected_findings.must_discover.length;

  // Weighted score: 60% must_discover, 40% keywords
  return mustCoverage * 0.6 + keywordCoverage * 0.4;
}
