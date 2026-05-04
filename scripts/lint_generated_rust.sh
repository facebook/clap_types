#!/usr/bin/env bash
# Copyright (c) Meta Platforms, Inc. and affiliates.

set -euo pipefail

generated_root="${1:-target/generated}"
rust_dir="${generated_root}/rust"
lint_dir="${generated_root}/rust-clippy"

shopt -s nullglob
rust_files=("${rust_dir}"/*.rs)
shopt -u nullglob

if ((${#rust_files[@]} == 0)); then
    echo "no generated Rust files found under ${rust_dir}" >&2
    exit 1
fi

mkdir -p "${lint_dir}/src"

cat >"${lint_dir}/Cargo.toml" <<'TOML'
[package]
name = "clap-types-generated-rust-lint"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]
TOML

{
    echo "#![allow(dead_code)]"
    for file in "${rust_files[@]}"; do
        base="$(basename "${file}")"
        module="${base%.rs}"
        echo "#[path = \"../../rust/${base}\"]"
        echo "mod ${module};"
    done
} >"${lint_dir}/src/lib.rs"

rustfmt --check "${rust_files[@]}"
cargo clippy --manifest-path "${lint_dir}/Cargo.toml" --all-targets -- -D warnings
