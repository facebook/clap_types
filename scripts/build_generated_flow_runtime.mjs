// Copyright (c) Meta Platforms, Inc. and affiliates.

import fs from "node:fs/promises";
import path from "node:path";
import flowRemoveTypes from "flow-remove-types";

const OUT_DIR = "target/generated-flow-runtime";
const INPUTS = [
  "tests/generated",
  "examples/clients",
  "target/generated/flow",
  "target/generated/flow-node",
  "target/generated/flow-zod",
];

async function copyStrippedFile(sourcePath) {
  const outputPath = path.join(OUT_DIR, sourcePath);
  await fs.mkdir(path.dirname(outputPath), { recursive: true });

  if (sourcePath.endsWith(".js")) {
    const source = await fs.readFile(sourcePath, "utf8");
    const stripped = flowRemoveTypes(source, { all: true }).toString();
    await fs.writeFile(outputPath, stripped);
    return;
  }

  await fs.copyFile(sourcePath, outputPath);
}

async function copyStrippedTree(root) {
  const entries = await fs.readdir(root, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      await copyStrippedTree(fullPath);
    } else if (entry.isFile()) {
      await copyStrippedFile(fullPath);
    }
  }
}

await fs.rm(OUT_DIR, { recursive: true, force: true });
for (const input of INPUTS) {
  await copyStrippedTree(input);
}
