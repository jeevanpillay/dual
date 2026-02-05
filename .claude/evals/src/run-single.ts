#!/usr/bin/env tsx
/**
 * Run a single eval case for testing
 *
 * Usage:
 *   npm run run-single sys-001-shell-wrapper-docker
 *   npm run run-single web-001-websocket-vs-sse
 */

import { runSingleCase } from "./runner.js";

const caseId = process.argv[2];

if (!caseId) {
  console.error("Usage: npm run run-single <case-id>");
  console.error("Example: npm run run-single sys-001-shell-wrapper-docker");
  process.exit(1);
}

runSingleCase(caseId).catch((err) => {
  console.error("Error:", err);
  process.exit(1);
});
