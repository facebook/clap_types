# Copyright (c) Meta Platforms, Inc. and affiliates.

from __future__ import annotations

import sys
from pathlib import Path


def assert_equal(actual: object, expected: object) -> None:
    if actual != expected:
        raise AssertionError(f"expected {expected!r}, got {actual!r}")


def main() -> None:
    generated = Path(sys.argv[1])
    sys.path.insert(0, str(generated / "python-package"))

    from repo_agent import AgentRunArgs, RepoAgent
    from repo_agent.agent.run import Args, command

    module_args = Args(
        workspace="/work/repo",
        profile="ci",
        verbose=1,
        task="triage-failing-check",
        model="frontier",
        max_tokens=2048,
        env=["RUST_LOG=debug", "CI=true"],
        dry_run=True,
    )
    root_args = AgentRunArgs(**module_args.__dict__)

    expected_args = (
        "--workspace",
        "/work/repo",
        "--profile",
        "ci",
        "--verbose",
        "agent",
        "run",
        "triage-failing-check",
        "--model",
        "frontier",
        "--max-tokens",
        "2048",
        "--env",
        "RUST_LOG=debug",
        "--env",
        "CI=true",
        "--dry-run",
    )

    assert_equal(command(module_args).argv(), ["repo-agent", *expected_args])
    assert_equal(
        RepoAgent.agent_run_command(root_args).argv(), ["repo-agent", *expected_args]
    )


if __name__ == "__main__":
    main()
