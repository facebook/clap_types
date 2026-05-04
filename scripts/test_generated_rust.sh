#!/usr/bin/env bash
# Copyright (c) Meta Platforms, Inc. and affiliates.

set -euo pipefail

generated_root="${1:-target/generated}"
smoke_bin="${generated_root}/rust_smoke"

rustc --edition=2021 -D warnings tests/generated/rust_smoke.rs -o "${smoke_bin}"
"${smoke_bin}"
