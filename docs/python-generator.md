# Python Generator

The Python backend generates dependency-free Python 3.10+ modules or packages. The
generated code is designed to pass `ruff`, `ruff format`, and `black`, and to work
on Python 3.10 through 3.14.

## Generated Shape

For each command, the backend emits:

- a frozen keyword-only dataclass for typed args;
- a `build_..._args(args)` function that returns `tuple[str, ...]`;
- a `..._command(args, program=...)` function that returns `CommandInvocation`;
- an optional namespace class when configured.

Example:

```python
args = AgentRunArgs(
    task="summarize",
    workspace="/repo",
    model="frontier",
    max_tokens=2048,
    dry_run=True,
)

invocation = agent_run_command(args, program="repo-agent")
assert invocation.argv()[0] == "repo-agent"
```

`CommandInvocation.run()` is a thin standard-library convenience over
`subprocess.run`. Consumers that need custom process management can use `.argv()`
directly.

## Package Layout

Single-file output is the default. Package output is opt-in:

```rust
use clap_types::Python;

let generator = Python::new()
    .module_name("repo_agent")
    .namespace("RepoAgent")
    .package();
```

When written with `generate_to`, package output returns the package root and writes
a layout like:

```text
repo_agent/
  __init__.py
  _runtime.py
  _root.py
  agent/
    __init__.py
    _root.py
    run.py
```

Leaf subcommands become modules (`agent/run.py`). Subcommands with children become
packages with their own `_root.py` so `agent` and `agent run` can both have typed
surfaces. The package root re-exports stable aliases:

```python
from repo_agent import AgentRunArgs, RepoAgent
from repo_agent.agent.run import Args, command
```

## Type Mapping

- Boolean clap flags become `bool | None`.
- Count flags become `int | None`.
- Possible values become `typing.Literal[...]`.
- Integer parsers become `int`.
- Float parsers become `float`.
- Path-like parsers become `str | os.PathLike[str]`.
- Unknown value parser types become `str`.
- Repeated options and multi-value args become `collections.abc.Sequence[...]`.

Required clap value args are required dataclass fields. Optional clap args are
annotated with `| None = None` so omission is explicit.

## Global Args

Global clap args are inherited by generated subcommand dataclasses and are emitted
before the subcommand path. This supports common CLI shapes like:

```text
tool --workspace /repo nested command --local-option value
```

## Options

The backend supports module naming and namespace controls:

```rust
use clap_types::Python;

let generator = Python::new()
    .module_name("repo_agent_bindings")
    .namespace("RepoAgent");
let with_output_contracts = Python::new().output_contracts();
```

`module_name` controls the output file name for single-file output and the package
directory name for package output. `namespace` emits a class that exposes generated
functions as static methods while keeping module-level functions available.

`output_contracts()` emits an `OutputContract` dataclass, `OUTPUT_CONTRACTS`, and a
`parse_output()` helper. JSON contracts use `json.loads`, JSON-lines contracts parse
each non-empty line, and text contracts return stdout unchanged.
`without_output_contracts()` omits those helpers.
