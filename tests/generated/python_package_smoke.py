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

    from repo_agent import AgentRunArgs, build_agent_run_args, RepoAgent
    from repo_agent.agent.run import Args, build_args, command

    args = Args(
        workspace="/work/repo",
        profile="ci",
        verbose=1,
        task="triage-failing-check",
        model="frontier",
        max_tokens=2048,
        env=["RUST_LOG=debug", "CI=true"],
        dry_run=True,
    )

    expected = (
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

    assert_equal(build_args(args), expected)
    assert_equal(command(args).argv(), ["repo-agent", *expected])
    assert_equal(build_agent_run_args(AgentRunArgs(**args.__dict__)), expected)
    assert_equal(
        RepoAgent.build_agent_run_args(AgentRunArgs(**args.__dict__)), expected
    )


if __name__ == "__main__":
    main()
