// Copyright (c) Meta Platforms, Inc. and affiliates.

// @ts-nocheck
// @flow strict

import {
  buildAgentRunCommand,
  buildIndexCommand,
  buildIssueCreateCommand,
} from "../../target/generated/flow/repo-agent.js";
import type {
  AgentRunArgs,
  IndexArgs,
  IssueCreateArgs,
} from "../../target/generated/flow/repo-agent.js";
import { buildDatasetImportCommand } from "../../target/generated/flow/data-forge.js";
import type { DatasetImportArgs } from "../../target/generated/flow/data-forge.js";
import { buildDeployCommand } from "../../target/generated/flow/opsctl.js";
import type { DeployArgs } from "../../target/generated/flow/opsctl.js";

function assert(condition: boolean, message?: string): void {
  if (!condition) {
    throw new Error(message ?? "assertion failed");
  }
}

function assertArray(actual: Array<string>, expected: Array<string>): void {
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

// $FlowExpectedError[incompatible-type] generated literal unions should reject unknown priority values
const invalidPriority: IssueCreateArgs = { title: "x", priority: "critical" };
void invalidPriority;

// $FlowExpectedError[incompatible-type] required clap args should remain required in Flow
const missingRequired: IssueCreateArgs = {};
void missingRequired;
