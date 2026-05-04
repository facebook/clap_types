// Copyright (c) Meta Platforms, Inc. and affiliates.

import {
  IndexArgsSchema,
  buildIndexCommand,
  buildIssueCreateCommand,
  type IndexArgs,
  type IssueCreateArgs,
} from "../../target/generated/typescript-zod/repo-agent-zod.ts";
import {
  DatasetImportArgsSchema,
  buildDatasetImportCommand,
  type DatasetImportArgs,
} from "../../target/generated/typescript-zod/data-forge-zod.ts";

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
  buildIndexCommand({ input: "src", threads: 1.5 } as unknown as IndexArgs);
} catch {
  rejectedInvalidBuilderInput = true;
}
assert(rejectedInvalidBuilderInput, "Zod builder should reject invalid input");

// @ts-expect-error generated Zod literals should reject unknown priority values
const invalidPriority: IssueCreateArgs = { title: "x", priority: "critical" };
void invalidPriority;

// @ts-expect-error required clap args should remain required in Zod-inferred types
const missingRequired: IssueCreateArgs = {};
void missingRequired;
