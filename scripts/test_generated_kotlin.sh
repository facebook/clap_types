#!/usr/bin/env bash
# Copyright (c) Meta Platforms, Inc. and affiliates.

set -euo pipefail

generated_root="${1:-target/generated}"
kotlin_dir="${generated_root}/kotlin"

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

for file in "${kotlin_files[@]}"; do
    base="$(basename "${file}")"
    stem="${base%.kt}"
    smoke_file="tests/generated/kotlin_${stem%Bindings}_smoke.kt"

    case "${base}" in
        RepoAgentBindings.kt)
            smoke_file="tests/generated/kotlin_smoke.kt"
            ;;
        DataForgeBindings.kt)
            smoke_file="tests/generated/kotlin_data_forge_smoke.kt"
            ;;
        OpsctlBindings.kt)
            smoke_file="tests/generated/kotlin_opsctl_smoke.kt"
            ;;
    esac

    if [[ ! -f "${smoke_file}" ]]; then
        echo "no Kotlin smoke test found for ${base}: ${smoke_file}" >&2
        exit 1
    fi

    jar="${generated_root}/kotlin-smoke-${stem}.jar"
    kotlinc \
        -Werror \
        -Wextra \
        -progressive \
        -Xvalidate-bytecode \
        "${file}" \
        "${smoke_file}" \
        -include-runtime \
        -d "${jar}"

    java -jar "${jar}"
done
