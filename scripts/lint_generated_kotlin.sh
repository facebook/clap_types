#!/usr/bin/env bash
# Copyright (c) Meta Platforms, Inc. and affiliates.

set -euo pipefail

generated_root="${1:-target/generated}"
kotlin_dir="${generated_root}/kotlin"
lint_dir="${generated_root}/kotlin-lint"

if ! java -version >/dev/null 2>&1 && [[ -x /opt/homebrew/opt/openjdk/bin/java ]]; then
    export PATH="/opt/homebrew/opt/openjdk/bin:${PATH}"
    export JAVA_HOME="/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home"
fi

shopt -s nullglob
kotlin_files=("${kotlin_dir}"/*.kt)
shopt -u nullglob

if ((${#kotlin_files[@]} == 0)); then
    echo "no generated Kotlin files found under ${kotlin_dir}" >&2
    exit 1
fi

mkdir -p "${lint_dir}"

for file in "${kotlin_files[@]}"; do
    stem="$(basename "${file}" .kt)"
    kotlinc \
        -Werror \
        -Wextra \
        -progressive \
        -Xvalidate-bytecode \
        "${file}" \
        -d "${lint_dir}/${stem}"
done
