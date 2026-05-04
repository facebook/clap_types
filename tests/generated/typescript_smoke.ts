// Copyright (c) Meta Platforms, Inc. and affiliates.

import {
  buildAgentRunCommand,
  buildIndexCommand,
  buildIssueCreateCommand,
  type AgentRunArgs,
  type IndexArgs,
  type IssueCreateArgs,
} from "../../target/generated/typescript/repo-agent.ts";
import {
  buildDatasetImportCommand,
  type DatasetImportArgs,
} from "../../target/generated/typescript/data-forge.ts";
import {
  buildDeployCommand,
  type DeployArgs,
} from "../../target/generated/typescript/opsctl.ts";

function assert(condition: boolean, message?: string): asserts condition {
  if (!condition) {
    throw new Error(message ?? "assertion failed");
  }
}

function assertArray(actual: string[], expected: string[]): void {
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}

const indexArgs: IndexArgs = {
  input: "src",
  workspace: "/tmp/work",
  profile: "ci",
  verbose: 2,
  color: "never",
  glob: ["*.rs", "*.toml"],
  threads: 4,
  format: "json",
  followSymlinks: true,
};

assertArray(buildIndexCommand(indexArgs), [
  "--workspace",
  "/tmp/work",
  "--profile",
  "ci",
  "--verbose",
  "--verbose",
  "--color",
  "never",
  "index",
  "src",
  "--glob",
  "*.rs",
  "--glob",
  "*.toml",
  "--threads",
  "4",
  "--format",
  "json",
  "--follow-symlinks",
]);

const issueArgs: IssueCreateArgs = {
  title: "Fix parser",
  label: ["bug", "agent"],
  priority: "high",
};

assertArray(buildIssueCreateCommand(issueArgs), [
  "issue",
  "create",
  "--title",
  "Fix parser",
  "--label",
  "bug",
  "--label",
  "agent",
  "--priority",
  "high",
]);

const agentArgs: AgentRunArgs = {
  task: "summarize",
  temperature: 0.2,
  maxTokens: 1024,
  env: ["RUST_LOG=debug"],
  dryRun: true,
};

assertArray(buildAgentRunCommand(agentArgs), [
  "agent",
  "run",
  "summarize",
  "--temperature",
  "0.2",
  "--max-tokens",
  "1024",
  "--env",
  "RUST_LOG=debug",
  "--dry-run",
]);

const importArgs: DatasetImportArgs = {
  source: "events.json",
  format: "json",
  output: "dist",
  logLevel: "debug",
  sampleRate: 0.5,
  tag: ["events", "daily"],
};

assertArray(buildDatasetImportCommand(importArgs), [
  "--output",
  "dist",
  "--log-level",
  "debug",
  "dataset",
  "import",
  "--source",
  "events.json",
  "--format",
  "json",
  "--sample-rate",
  "0.5",
  "--tag",
  "events",
  "--tag",
  "daily",
]);

const deployArgs: DeployArgs = {
  workspace: "/srv/app",
  output: "json",
  verbose: 2,
  service: "api",
  image: "registry.example.com/api:2026-05-04",
  environment: "prod",
  replicas: 8,
  env: ["RUST_LOG=info", "FEATURE_FLAG=true"],
  wait: true,
  timeoutSeconds: 120.5,
};

assertArray(buildDeployCommand(deployArgs), [
  "--workspace",
  "/srv/app",
  "--output",
  "json",
  "--verbose",
  "--verbose",
  "deploy",
  "api",
  "--image",
  "registry.example.com/api:2026-05-04",
  "--environment",
  "prod",
  "--replicas",
  "8",
  "--env",
  "RUST_LOG=info",
  "--env",
  "FEATURE_FLAG=true",
  "--wait",
  "--timeout-seconds",
  "120.5",
]);

// @ts-expect-error generated literal unions should reject unknown priority values
const invalidPriority: IssueCreateArgs = { title: "x", priority: "critical" };
void invalidPriority;

// @ts-expect-error required clap args should remain required in TypeScript
const missingRequired: IssueCreateArgs = {};
void missingRequired;
