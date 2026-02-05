/**
 * Runner for executing /research_experiment via Claude Code headless mode
 */

import { spawn } from "child_process";
import { mkdtemp, rm, readdir, readFile } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import type { EvalCase, ResearchOutput } from "./types.js";

const TIMEOUT_MS = 5 * 60 * 1000; // 5 minutes per research task

export async function runResearchExperiment(
  evalCase: EvalCase
): Promise<ResearchOutput> {
  const startTime = Date.now();

  // Create isolated temp workspace
  const workspace = await mkdtemp(join(tmpdir(), "research-eval-"));

  // Create minimal directory structure
  await createWorkspaceStructure(workspace);

  const prompt = formatPrompt(evalCase);

  try {
    const result = await executeClaudeHeadless(workspace, prompt);
    const researchContent = await extractResearchOutput(workspace);

    return {
      content: researchContent || result.stdout,
      filePath: await findResearchFile(workspace),
      duration: Date.now() - startTime,
      exitCode: result.exitCode,
    };
  } finally {
    // Cleanup workspace
    await rm(workspace, { recursive: true, force: true });
  }
}

function formatPrompt(evalCase: EvalCase): string {
  return `/research_experiment

**Hypothesis**: ${evalCase.hypothesis}

**Context**: ${evalCase.context}

Please research this hypothesis thoroughly and produce a research document.`;
}

interface ClaudeResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

async function executeClaudeHeadless(
  workspace: string,
  prompt: string
): Promise<ClaudeResult> {
  return new Promise((resolve, reject) => {
    const args = [
      "--print", // Print output (headless mode)
      "--dangerously-skip-permissions", // Skip permission prompts for eval
      "--max-turns", "50", // Limit turns for research task
      "-p", prompt, // The prompt
    ];

    const proc = spawn("claude", args, {
      cwd: workspace,
      env: {
        ...process.env,
        // Ensure clean environment
        CLAUDE_CODE_DISABLE_TELEMETRY: "1",
      },
      timeout: TIMEOUT_MS,
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    proc.on("close", (code) => {
      resolve({
        stdout,
        stderr,
        exitCode: code ?? 1,
      });
    });

    proc.on("error", (err) => {
      reject(err);
    });

    // Handle timeout
    setTimeout(() => {
      proc.kill("SIGTERM");
      reject(new Error(`Claude process timed out after ${TIMEOUT_MS}ms`));
    }, TIMEOUT_MS);
  });
}

async function createWorkspaceStructure(workspace: string): Promise<void> {
  const { mkdir, writeFile } = await import("fs/promises");

  // Create directory structure expected by research_experiment
  await mkdir(join(workspace, "thoughts", "shared", "research"), {
    recursive: true,
  });
  await mkdir(join(workspace, ".claude", "commands"), { recursive: true });

  // Copy research_experiment command to workspace
  // In real implementation, you'd copy from the actual command file
  // For now, create a minimal marker
  await writeFile(
    join(workspace, ".claude", "commands", "research_experiment.md"),
    "# Placeholder - actual command loaded from main repo"
  );
}

async function extractResearchOutput(
  workspace: string
): Promise<string | null> {
  const researchDir = join(workspace, "thoughts", "shared", "research");

  try {
    const files = await readdir(researchDir);
    const mdFiles = files.filter((f) => f.endsWith(".md"));

    if (mdFiles.length === 0) return null;

    // Get the most recent research file
    const latestFile = mdFiles.sort().pop()!;
    const content = await readFile(join(researchDir, latestFile), "utf-8");
    return content;
  } catch {
    return null;
  }
}

async function findResearchFile(workspace: string): Promise<string | undefined> {
  const researchDir = join(workspace, "thoughts", "shared", "research");

  try {
    const files = await readdir(researchDir);
    const mdFiles = files.filter((f) => f.endsWith(".md"));
    return mdFiles.length > 0 ? join(researchDir, mdFiles.sort().pop()!) : undefined;
  } catch {
    return undefined;
  }
}

// For testing individual cases
export async function runSingleCase(caseId: string): Promise<void> {
  const casesFile = await readFile(
    join(import.meta.dirname, "..", "research_experiment_cases.json"),
    "utf-8"
  );
  const cases = JSON.parse(casesFile);
  const evalCase = cases.cases.find((c: EvalCase) => c.id === caseId);

  if (!evalCase) {
    console.error(`Case not found: ${caseId}`);
    process.exit(1);
  }

  console.log(`Running case: ${evalCase.id}`);
  console.log(`Hypothesis: ${evalCase.hypothesis}\n`);

  const result = await runResearchExperiment(evalCase);

  console.log(`Duration: ${result.duration}ms`);
  console.log(`Exit code: ${result.exitCode}`);
  console.log(`\n--- Research Output ---\n`);
  console.log(result.content);
}
