import { z } from "zod";

export const ExpectedFindingsSchema = z.object({
  must_discover: z.array(z.string()),
  should_discover: z.array(z.string()),
  keywords: z.array(z.string()),
});

export const EvalCaseSchema = z.object({
  id: z.string(),
  domain: z.string(),
  difficulty: z.enum(["easy", "medium", "hard"]),
  hypothesis: z.string(),
  context: z.string(),
  expected_findings: ExpectedFindingsSchema,
  known_answer_summary: z.string(),
});

export const EvalCasesFileSchema = z.object({
  meta: z.object({
    version: z.string(),
    description: z.string(),
    total_cases: z.number(),
    domains: z.array(z.string()),
  }),
  cases: z.array(EvalCaseSchema),
});

export type EvalCase = z.infer<typeof EvalCaseSchema>;
export type ExpectedFindings = z.infer<typeof ExpectedFindingsSchema>;
export type EvalCasesFile = z.infer<typeof EvalCasesFileSchema>;

export interface ResearchOutput {
  content: string;
  filePath?: string;
  duration: number;
  exitCode: number;
}

export interface JudgeResult {
  score: number; // 0-1
  keyword_coverage: number; // 0-1
  must_discover_hits: number; // count
  must_discover_total: number;
  should_discover_hits: number;
  should_discover_total: number;
  reasoning: string;
  strengths: string[];
  weaknesses: string[];
}
