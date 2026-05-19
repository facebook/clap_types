# clap_types

`clap_types` turns Rust CLIs into multi-language client libraries.

If your Rust tool is defined with [`clap`](https://crates.io/crates/clap),
`clap_types` reflects its command tree and generates strongly-typed clients for
TypeScript, Flow-annotated JavaScript, Python, Rust, and Kotlin/JVM. Those clients
know how to build argv, optionally invoke the executable, and optionally describe or
parse structured output contracts.

The goal is to make CLIs feel like real programmatic APIs without giving up why CLIs
are great in the first place: they are easy to install, easy to sandbox, easy to call
from agents and skills, and naturally cross-process.

Rust CLIs are often the best packaging format for local tools, but hand-written
wrappers quickly become stringly typed drift:

- callers guess which arguments are required;
- enums and nested subcommands get duplicated by hand;
- file paths, booleans, counters, and repeated values lose their shape;
- stdout parsing is ad hoc and usually undocumented.

`clap_types` treats your `clap::Command` as the input IDL and explicit output
contracts as the output IDL. It generates clients that can be checked, linted, and
shipped with the same care as any other library.

That means callers get an idiomatic library instead of a pile of string
concatenation. The generated code knows the right subcommand order, where global
arguments belong, how varargs and repeated options expand, and which fields are
required before anything is sent to the tool.

## Status

This project is early and pre-1.0. The architecture is intentionally similar to
`clap_complete`, `clap_mangen`, and `clap-markdown`: reflect a `clap::Command`,
convert it into a language-neutral model, then render one or more target-language
artifacts.

Supported backends:

- TypeScript: dependency-free argv builders.
- TypeScript + Zod: opt-in strict schemas and runtime validation.
- TypeScript + Node: opt-in `child_process` command descriptors, `execFile`, and
  `spawn` helpers.
- Flow: dependency-free JavaScript argv builders with Flow exact object types.
- Flow + Zod / Node: the same opt-in validation and `child_process` helpers as the
  TypeScript backend.
- Python 3.10+: dataclasses, argv builders, single-file modules, package layouts,
  and `subprocess.run` convenience.
- Rust: dependency-free structs, enums, argv builders, and `std::process` helpers.
- Kotlin/JVM: desktop-friendly data classes, enums, argv builders, and
  `ProcessBuilder` helpers.

Every backend can opt into generated output-contract metadata and dependency-free
framing helpers with `.output_contracts()` or the hidden command's
`--output-contracts` flag.

## What You Get

- Multi-language input clients from one Rust CLI definition.
- Generated types for subcommands, globals, required args, enums, counters, booleans,
  repeated options, varargs, paths, numbers, and strings.
- Optional standard-library invocation helpers for Node, Python, Rust, and Kotlin/JVM.
- Runtime validation before invocation in opt-in modes such as TypeScript + Zod or
  Flow + Zod.
- Optional output-contract metadata for JSON, JSON-lines, text, streaming, and
  interactive commands, with automatic parsing where the generated language can do
  that cleanly without extra dependencies.
- A path toward typed success and error returns, so callers can know what to expect
  back instead of scraping stdout and exit codes by hand.
- A hidden `generate-binding` command you can embed in your own CLI so the tool can
  generate clients for itself.
- Generated-code tests in CI across Rust, Python 3.10-3.14, TypeScript and Flow on
  Node and Bun, and Kotlin/JVM.

## Install

Add `clap_types` to the crate that defines your CLI:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
clap_types = "0.1"
```

For now, while developing locally, use a path dependency:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
clap_types = { path = "../clap_types" }
```

## Quick Start

```rust
use clap::{Arg, ArgAction, Command};
use clap_types::{generate, Flow, Kotlin, Python, Rust, TypeScript};

fn build_cli() -> Command {
    Command::new("demo")
        .about("Example CLI")
        .arg(Arg::new("config").long("config").action(ArgAction::Set))
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
}

let mut cmd = build_cli();
generate(TypeScript::new(), &mut cmd, "demo", &mut std::io::stdout())?;

let mut cmd = build_cli();
generate(TypeScript::new().zod(), &mut cmd, "demo", &mut std::io::stdout())?;

let mut cmd = build_cli();
generate(TypeScript::new().node(), &mut cmd, "demo", &mut std::io::stdout())?;

let mut cmd = build_cli();
generate(Flow::new().zod().node(), &mut cmd, "demo", &mut std::io::stdout())?;

let mut cmd = build_cli();
generate(
    Python::new()
        .module_name("demo_bindings")
        .namespace("Demo"),
    &mut cmd,
    "demo",
    &mut std::io::stdout(),
)?;

let mut cmd = build_cli();
generate(Rust::new(), &mut cmd, "demo", &mut std::io::stdout())?;

let mut cmd = build_cli();
generate(Kotlin::new(), &mut cmd, "demo", &mut std::io::stdout())?;
# Ok::<(), std::io::Error>(())
```

For file generation, use `generate_to`:

```rust
use clap::Command;
use clap_types::{generate_to, TypeScript};

let mut cmd = Command::new("demo");
let path = generate_to(TypeScript::new(), &mut cmd, "demo", "target/generated")?;
assert_eq!(path.file_name().unwrap(), "demo.ts");
# Ok::<(), std::io::Error>(())
```

## Hidden Binding Command

Applications can expose every `clap_types` backend through a hidden clap subcommand.
This makes bindings generation part of the tool itself, which is handy for
build scripts, package publishing, and agent environments that install the CLI
first and ask it to describe itself later.

```rust
use clap::{Arg, Command};
use clap_types::{BINDING_COMMAND_NAME, binding_command, generate_binding_from_matches};

fn build_cli() -> Command {
    Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)))
}

fn main() -> std::io::Result<()> {
    let matches = build_cli().subcommand(binding_command()).get_matches();

    if let Some((BINDING_COMMAND_NAME, binding_matches)) = matches.subcommand() {
        let mut cmd = build_cli();
        let path = generate_binding_from_matches(&mut cmd, "demo", binding_matches)?;
        eprintln!("generated {}", path.display());
        return Ok(());
    }

    Ok(())
}
```

The hidden command supports the current generators and their main options:

```sh
demo generate-binding typescript --path target/generated/typescript
demo generate-binding typescript --zod --node --module-name demo-node --path target/generated/typescript-node
demo generate-binding flow --path target/generated/flow
demo generate-binding flow --zod --node --module-name demo-node --path target/generated/flow-node
demo generate-binding python --module-name demo_bindings --namespace Demo --path target/generated/python
demo generate-binding python --full-module --module-name demo --namespace Demo --path target/generated/python-package
demo generate-binding rust --module-name demo_bindings --output-contracts --path target/generated/rust
demo generate-binding kotlin --module-name demo_bindings --package dev.example.demo --path target/generated/kotlin
```

See
[`examples/generate_binding_subcommand.rs`](examples/generate_binding_subcommand.rs)
for a complete app-shaped example.

## Builder API CLIs

Builder-style clap apps can pass a `Command` directly:

```rust
use std::path::PathBuf;

use clap::{Arg, ArgAction, Command, ValueHint, value_parser};
use clap_types::{generate_to, Python, TypeScript};

fn cli() -> Command {
    Command::new("repo-agent")
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .global(true)
                .value_hint(ValueHint::DirPath)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .subcommand(
            Command::new("index")
                .arg(Arg::new("input").required(true))
                .arg(Arg::new("threads").long("threads").value_parser(value_parser!(u16))),
        )
}

let mut cmd = cli();
generate_to(TypeScript::new(), &mut cmd, "repo-agent", "target/generated/typescript")?;

let mut cmd = cli();
generate_to(
    Python::new().module_name("repo_agent_bindings").namespace("RepoAgent"),
    &mut cmd,
    "repo-agent",
    "target/generated/python",
)?;
# Ok::<(), std::io::Error>(())
```

## Derive API CLIs

Derive-style clap apps work by using `CommandFactory::command()`:

```rust
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_types::{generate_to, TypeScript};

#[derive(Parser)]
#[command(name = "opsctl")]
struct OpsCtl {
    #[arg(long, global = true, value_enum, default_value = "table")]
    output: OutputFormat,

    #[command(subcommand)]
    command: OpsCommand,
}

#[derive(Subcommand)]
enum OpsCommand {
    Deploy {
        service: String,
        #[arg(long, required = true)]
        image: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

let mut cmd = OpsCtl::command();
generate_to(TypeScript::new().zod(), &mut cmd, "opsctl", "target/generated/typescript-zod")?;
# Ok::<(), std::io::Error>(())
```

The full derive example is in
[`examples/generate_derive_bindings.rs`](examples/generate_derive_bindings.rs).

## Generated TypeScript

Default TypeScript output has no runtime dependencies:

```ts
import {
  buildAgentRunCommand,
  type AgentRunArgs,
} from "./generated/repo-agent";

const args: AgentRunArgs = {
  workspace: "/repo",
  verbose: 2,
  task: "summarize",
  maxTokens: 2048,
  dryRun: true,
};

const argv = buildAgentRunCommand(args);
```

Clap metadata is translated conservatively:

- `ValueEnum` and possible values become string literal unions.
- flags become `boolean`;
- counters become `number`;
- integer and float parsers become `number`;
- repeated options become readonly arrays;
- required clap values become required TypeScript properties;
- global args are inherited into subcommand builders.

## Generated TypeScript + Zod

Use Zod mode when runtime validation is useful:

```rust
use clap_types::TypeScript;

let generator = TypeScript::new()
    .module_name("repo-agent-zod")
    .zod();
```

Zod output emits strict schemas and inferred types:

```ts
import {
  AgentRunArgsSchema,
  buildAgentRunCommand,
  type AgentRunArgs,
} from "./generated/repo-agent-zod";

const parsed = AgentRunArgsSchema.parse({
  task: "summarize",
  model: "frontier",
});

const argv = buildAgentRunCommand(parsed);
```

`TypeScript::new().zod()` validates builder inputs with `Schema.parse(args)`.
`TypeScript::new().zod_schemas()` emits schemas and inferred types without parsing
inside the builder.

## Generated TypeScript + Node

Use Node mode when the generated artifact should include idiomatic
`node:child_process` helpers:

```rust
use clap_types::TypeScript;

let generator = TypeScript::new()
    .module_name("repo-agent-node")
    .node();
```

The generated module still exposes the plain argv builder, and adds process-ready
helpers:

```ts
import {
  createAgentRunCommand,
  runAgentRunCommand,
  type AgentRunArgs,
} from "./generated/repo-agent-node";

const args: AgentRunArgs = {
  task: "summarize",
  model: "frontier",
};

const command = createAgentRunCommand(args);
// { file: "repo-agent", args: ["agent", "run", "summarize", "--model", "frontier"] }

const result = await runAgentRunCommand(args, { cwd: "/repo" });
console.log(result.stdout);
```

`create...Command` returns `{ file, args }`, `run...Command` uses `execFile`, and
`spawn...Command` uses `spawn`. Pass `{ program: "/custom/path" }` to override the
executable while keeping the typed argv builder unchanged.

## Generated Flow

Flow output mirrors the TypeScript backend but emits runnable `.js` modules with
Flow annotations:

```rust
use clap_types::Flow;

let generator = Flow::new().module_name("repo-agent");
let zod_generator = Flow::new().module_name("repo-agent-zod").zod();
let node_generator = Flow::new().module_name("repo-agent-node").node();
```

The generated JavaScript uses exact object types, `$ReadOnlyArray` for repeated
values, optional Zod schemas, and optional Node `child_process` helpers. Runtime
tests strip Flow annotations with `flow-remove-types` before executing under Node
or Bun.

## Generated Python

Python output targets Python 3.10 through 3.14 and uses only the standard library:

```python
from repo_agent_bindings import AgentRunArgs, RepoAgent

args = AgentRunArgs(
    workspace="/repo",
    verbose=2,
    task="summarize",
    model="frontier",
    max_tokens=2048,
    dry_run=True,
)

invocation = RepoAgent.agent_run_command(args, program="repo-agent")
argv = invocation.argv()
```

Python generated clients include:

- frozen keyword-only dataclasses;
- `Literal[...]` for possible values;
- `int`, `float`, `bool`, and path-like annotations when clap exposes those parser
  types;
- `build_..._args()` functions returning `tuple[str, ...]`;
- `..._command()` helpers returning `CommandInvocation`;
- `CommandInvocation.run()` as a thin `subprocess.run` wrapper.

Python can also generate a package layout:

```rust
use clap::Command;
use clap_types::{generate_to, Python};

fn build_cli() -> Command {
    Command::new("repo-agent")
}

let mut cmd = build_cli();
generate_to(
    Python::new()
        .module_name("repo_agent")
        .namespace("RepoAgent")
        .package(),
    &mut cmd,
    "repo-agent",
    "target/generated/python-package",
)?;
# Ok::<(), std::io::Error>(())
```

The package layout emits `_runtime.py`, `_root.py`, and command modules such as
`agent/run.py`:

```python
from repo_agent.agent.run import Args, command

invocation = command(Args(task="summarize", model="frontier"))
assert invocation.argv() == ["repo-agent", "agent", "run", "summarize", "--model", "frontier"]
```

## Generated Rust

Rust output uses only `std`:

```rust,ignore
use repo_agent_bindings::{AgentRunArgs, AgentRunModel, build_agent_run_command};

let args = AgentRunArgs {
    task: "summarize".to_owned(),
    model: Some(AgentRunModel::Frontier),
    dry_run: true,
    ..Default::default()
};

let argv = build_agent_run_command(&args);
```

The backend emits structs for command args, enums for clap possible values,
`build_..._command()` functions, and `CommandInvocation` helpers over
`std::process::Command`.

## Generated Kotlin

Kotlin output targets Kotlin/JVM desktop clients and uses the standard library:

```kotlin
val args =
    AgentRunArgs(
        task = "summarize",
        model = AgentRunModel.Frontier,
        dryRun = true,
    )

val invocation = agentRunCommand(args)
val process = invocation.processBuilder().start()
```

The backend emits data classes, enum classes, `build...Command()` functions, and a
small `CommandInvocation` wrapper around `ProcessBuilder`.

## Output Contracts

All generators omit output contracts by default. Turn them on when the reflected
`CliSpec` includes explicit output metadata:

```rust,ignore
let generator = TypeScript::new().output_contracts();
let generator = Flow::new().output_contracts();
let generator = Python::new().output_contracts();
let generator = Rust::new().output_contracts();
let generator = Kotlin::new().output_contracts();
```

Generated parsers stay dependency-free. They preserve JSON as parsed `unknown` in
TypeScript and `mixed` in Flow, use `json.loads` in Python, and expose framed
JSON/JSON-lines strings in Rust and Kotlin where the standard library has no JSON
parser.

## Structured Type Hints

Clap already exposes useful input metadata, and `clap_types` preserves it wherever
it can:

- `PathBuf`, `OsString`, and `ValueHint::FilePath` / `ValueHint::DirPath` can tell a
  backend that a string-like value is really a filesystem value;
- `ValueEnum` and possible values become language-native enum or literal types;
- integer, wide integer, float, boolean, repeated, grouped, and optional values keep
  their shape in generated clients.

The next step is explicit schema metadata for cases where clap cannot know enough.
That should mirror output contracts: a command author can attach JSON Schema or
small structured hints to an argument or output payload, and each backend can decide
how much to render. TypeScript might emit Zod or JSON Schema validators, Python
might expose `TypedDict` or dataclass adapters, Rust might support serde-aware modes,
and Kotlin might support kotlinx.serialization. The default path should stay
dependency-free, while schema-aware modes can be opt-in.

In other words: clap reflection should provide the baseline type shape; explicit
schemas should provide richer semantic contracts such as "this string is a file",
"this JSON field is a path", or "this output is a stream of objects with this
schema."

## Examples

Generate all fixtures:

```sh
npm run generate:fixtures
```

This runs:

- `examples/generate_complex_bindings.rs`: builder-style clap examples;
- `examples/generate_derive_bindings.rs`: derive-macro clap example.

Client examples:

```sh
python examples/clients/python_repo_agent.py target/generated
python examples/clients/python_opsctl_derive.py target/generated
bun examples/clients/typescript_repo_agent.ts
bun examples/clients/typescript_zod_opsctl.ts
npm run test:generated:flow
```

See [`docs/examples.md`](docs/examples.md) for the full map.

## Development

Rust checks:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

Generated-code checks:

```sh
npm ci
npm run generate:fixtures
npm run check:generated:ts
npm run check:generated:flow
npm run lint:generated:rust
npm run check:generated:rust
npm run lint:generated:kotlin
npm run check:generated:kotlin
npm run test:generated:ts
npm run test:generated:flow
ruff check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
ruff format --check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
black --check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
python -m compileall -q target/generated/python target/generated/python-package tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients
python tests/generated/python_smoke.py target/generated
python tests/generated/python_package_smoke.py target/generated
```

CI runs Rust checks, Python generated-code checks on Python 3.10 through 3.14,
TypeScript and Flow checks on Node plus Bun, generated Rust compilation, and generated
Kotlin/JVM compilation. Generated Rust is checked with `rustfmt`, `cargo clippy`,
and `rustc -D warnings`; generated Kotlin is compiled with `-Werror`, `-Wextra`,
progressive mode, and bytecode validation so compiler deprecations become CI
failures instead of quiet drift.

## Design Notes

`clap_types` treats clap as the source of truth for inputs. It does not infer stdout
or stderr contracts from help text. Structured output should be declared explicitly;
see [`docs/output-contracts.md`](docs/output-contracts.md).

Output contracts that attach metadata directly to `clap::Command` use clap's
unstable extension API and are gated behind `clap_types`' own
`unstable-output-contracts` feature. The stable IR can already carry JSON,
JSON-lines, text, buffered, streaming, interactive, and JSON Schema metadata.

More design detail lives in:

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/typescript-generator.md`](docs/typescript-generator.md)
- [`docs/python-generator.md`](docs/python-generator.md)
- [`docs/rust-generator.md`](docs/rust-generator.md)
- [`docs/kotlin-generator.md`](docs/kotlin-generator.md)
- [`docs/testing.md`](docs/testing.md)

## License

This project is licensed under the MIT license. See [`LICENSE`](LICENSE).
