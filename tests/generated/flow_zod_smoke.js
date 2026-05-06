// Copyright (c) Meta Platforms, Inc. and affiliates.

// @ts-nocheck
// @flow strict

import {
  IndexArgsSchema,
  buildIndexCommand,
  buildIssueCreateCommand,
} from "../../target/generated/flow-zod/repo-agent-zod.js";
import type {
  IndexArgs,
  IssueCreateArgs,
} from "../../target/generated/flow-zod/repo-agent-zod.js";
import {
  DatasetImportArgsSchema,
  buildDatasetImportCommand,
} from "../../target/generated/flow-zod/data-forge-zod.js";
import type { DatasetImportArgs } from "../../target/generated/flow-zod/data-forge-zod.js";

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

assert(IndexArgsSchema.safeParse(indexArgs).success);
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

const importArgs: DatasetImportArgs = {
  source: "events.json",
  format: "json",
  output: "dist",
  logLevel: "debug",
  sampleRate: 0.5,
  tag: ["events", "daily"],
};

assert(DatasetImportArgsSchema.safeParse(importArgs).success);
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

assert(!IndexArgsSchema.safeParse({ input: "src", threads: 1.5 }).success);
assert(!IndexArgsSchema.safeParse({ input: "src", extra: true }).success);

let rejectedInvalidBuilderInput = false;
try {
  buildIndexCommand(({ input: "src", threads: 1.5 } as any));
} catch {
  rejectedInvalidBuilderInput = true;
}
assert(rejectedInvalidBuilderInput, "Zod builder should reject invalid input");

// $FlowExpectedError[incompatible-type] generated literals should reject unknown priority values
const invalidPriority: IssueCreateArgs = { title: "x", priority: "critical" };
void invalidPriority;

// $FlowExpectedError[incompatible-type] required clap args should remain required in Flow
const missingRequired: IssueCreateArgs = {};
void missingRequired;
