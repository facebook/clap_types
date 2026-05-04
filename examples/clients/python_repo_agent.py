# Copyright (c) Meta Platforms, Inc. and affiliates.

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType


def load_generated(module_name: str, generated_root: Path) -> ModuleType:
    path = generated_root / "python" / f"{module_name}.py"
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Could not load generated module at {path}")

    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def default_generated_root() -> Path:
    return Path(__file__).resolve().parents[2] / "target" / "generated"


def main() -> None:
    generated_root = (
        Path(sys.argv[1]) if len(sys.argv) > 1 else default_generated_root()
    )
    repo = load_generated("repo_agent_bindings", generated_root)

    args = repo.AgentRunArgs(
        workspace="/work/repo",
        profile="ci",
        verbose=1,
        task="triage-failing-check",
        model="frontier",
        max_tokens=2048,
        env=("RUST_LOG=debug", "CI=true"),
        dry_run=True,
    )
    invocation = repo.RepoAgent.agent_run_command(args, program="repo-agent")

    assert invocation.argv() == [
        "repo-agent",
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
    ]


if __name__ == "__main__":
    main()
