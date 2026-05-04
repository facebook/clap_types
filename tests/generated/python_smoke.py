# Copyright (c) Meta Platforms, Inc. and affiliates.

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


def load_module(module_name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Could not load {path}")

    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    generated_root = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target/generated")
    )
    python_root = generated_root / "python"

    repo = load_module("repo_agent_bindings", python_root / "repo_agent_bindings.py")
    data = load_module("data_forge_bindings", python_root / "data_forge_bindings.py")
    ops = load_module("opsctl_bindings", python_root / "opsctl_bindings.py")

    index_args = repo.IndexArgs(
        input="src",
        workspace="/tmp/work",
        profile="ci",
        verbose=2,
        color="never",
        glob=("*.rs", "*.toml"),
        threads=4,
        format="json",
        follow_symlinks=True,
    )
    assert repo.build_index_args(index_args) == (
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
    )

    issue_args = repo.IssueCreateArgs(
        title="Fix parser",
        label=("bug", "agent"),
        priority="high",
    )
    assert repo.RepoAgent.build_issue_create_args(issue_args) == (
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
    )

    agent_invocation = repo.agent_run_command(
        repo.AgentRunArgs(
            task="summarize",
            temperature=0.2,
            max_tokens=1024,
            env=("RUST_LOG=debug",),
            dry_run=True,
        ),
        program="repo-agent",
    )
    assert agent_invocation.argv() == [
        "repo-agent",
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
    ]

    import_args = data.DatasetImportArgs(
        source="events.json",
        format="json",
        output="dist",
        log_level="debug",
        sample_rate=0.5,
        tag=("events", "daily"),
    )
    assert data.build_dataset_import_args(import_args) == (
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
    )

    deploy_args = ops.DeployArgs(
        workspace="/srv/app",
        output="json",
        verbose=2,
        service="api",
        image="registry.example.com/api:2026-05-04",
        environment="prod",
        replicas=8,
        env=("RUST_LOG=info", "FEATURE_FLAG=true"),
        wait=True,
        timeout_seconds=120.5,
    )
    assert ops.OpsCtl.build_deploy_args(deploy_args) == (
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
    )

    result = repo.CommandInvocation(
        sys.executable,
        ("-c", "print('clap_types python ok')"),
    ).run(check=True, capture_output=True)
    assert result.stdout.strip() == "clap_types python ok"


if __name__ == "__main__":
    main()
