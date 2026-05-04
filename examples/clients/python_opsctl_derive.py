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
    ops = load_generated("opsctl_bindings", generated_root)

    deploy = ops.DeployArgs(
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
    assert ops.OpsCtl.build_deploy_args(deploy) == (
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

    rotate = ops.SecretRotateArgs(
        name="database-url",
        restart=("api", "worker"),
        grace_seconds=60,
    )
    assert ops.secret_rotate_command(rotate, program="opsctl").argv() == [
        "opsctl",
        "secret",
        "rotate",
        "database-url",
        "--restart",
        "api",
        "--restart",
        "worker",
        "--grace-seconds",
        "60",
    ]


if __name__ == "__main__":
    main()
