// Copyright (c) Meta Platforms, Inc. and affiliates.

declare module "zod" {
  declare export var z: any;
}

declare module "node:child_process" {
  declare export var execFile: any;
  declare export var spawn: any;
}
