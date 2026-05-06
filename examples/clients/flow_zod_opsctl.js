// Copyright (c) Meta Platforms, Inc. and affiliates.

// @ts-nocheck
// @flow strict

import {
  DeployArgsSchema,
  SecretRotateArgsSchema,
  buildDeployCommand,
  buildSecretRotateCommand,
} from "../../target/generated/flow-zod/opsctl-zod.js";
import type {
  DeployArgs,
  SecretRotateArgs,
} from "../../target/generated/flow-zod/opsctl-zod.js";

function assert(condition: boolean, message?: string): void {
  if (!condition) {
    throw new Error(message ?? "assertion failed");
  }
}

function assertArray(actual: Array<string>, expected: Array<string>): void {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`expected ${expectedJson}, got ${actualJson}`);
  }
}

const deploy: DeployArgs = {
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

assert(DeployArgsSchema.safeParse(deploy).success);
assertArray(buildDeployCommand(deploy), [
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

const rotate: SecretRotateArgs = {
  name: "database-url",
  restart: ["api", "worker"],
  graceSeconds: 60,
};

assert(SecretRotateArgsSchema.safeParse(rotate).success);
assertArray(buildSecretRotateCommand(rotate), [
  "secret",
  "rotate",
  "database-url",
  "--restart",
  "api",
  "--restart",
  "worker",
  "--grace-seconds",
  "60",
]);

assert(!DeployArgsSchema.safeParse({ service: "api", image: "x", replicas: 1.5 }).success);
