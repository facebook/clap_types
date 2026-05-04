// Copyright (c) Meta Platforms, Inc. and affiliates.

import {
  createAgentRunCommand,
  type AgentRunArgs,
} from "../../target/generated/typescript-node/repo-agent-node.ts";

function assertArray(actual: string[], expected: string[]): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`expected ${expectedJson}, got ${actualJson}`);
  }
}

const args: AgentRunArgs = {
  workspace: "/work/repo",
  profile: "ci",
  verbose: 1,
  task: "triage-failing-check",
  model: "frontier",
  maxTokens: 2048,
  env: ["RUST_LOG=debug", "CI=true"],
  dryRun: true,
};

const command = createAgentRunCommand(args);

if (command.file !== "repo-agent") {
  throw new Error(`expected repo-agent, got ${command.file}`);
}

assertArray(command.args, [
  "--workspace",
  "/work/repo",
  "--profile",
  "ci",
  "--verbose",
  "agent",
  "run",
  "triage-failing-check",
  "--model",
  "frontier",
  "--max-tokens",
  "2048",
  "--env",
  "RUST_LOG=debug",
  "--env",
  "CI=true",
  "--dry-run",
]);
