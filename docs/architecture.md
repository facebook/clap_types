# Architecture

## Goal

`clap_types` turns a Rust `clap::Command` into type-safe bindings for constructing
argv in other languages. The default generated libraries return argv arrays so each
runtime can choose its own process layer, sandbox, IPC boundary, or agent tool
integration. Optional runtime modes layer thin standard-library process helpers on
top of those same argv builders.

## Flow

```text
Rust CLI using clap
        |
        v
clap::Command reflection
        |
        v
clap_types::CliSpec IR
        |
        +--> TypeScript generator
        +--> Python generator
        +--> Rust generator
        +--> Kotlin generator
```

## Public API Shape

The crate follows the same broad shape as `clap_complete`:

- `Generator`: backend trait.
- `generate`: render to any writer.
- `generate_to`: write the generated artifact or package to a directory.
- `TypeScript`: dependency-free TypeScript argv builders.
- `TypeScript::node()`: Node `child_process` helpers.
- `Python`: Python 3.10+ dataclasses, argv builders, single-file modules, package
  layouts, and subprocess conveniences.
- `Rust`: dependency-free Rust structs, enums, argv builders, and `std::process`
  conveniences.
- `Kotlin`: Kotlin/JVM data classes, enum classes, argv builders, and
  `ProcessBuilder` conveniences for desktop code.
- `binding_command`: a hidden `generate-binding` clap subcommand that consuming
  CLIs can embed to expose every generator through their own executable.

## Intermediate Model

The intermediate representation lives in `src/model.rs`. It is intentionally
language-neutral:

- `CliSpec`: root artifact, binary name, and future output contracts.
- `CommandSpec`: command tree node.
- `ArgSpec`: reflected argument behavior.
- `ValueSpec`: arity, displayed value names, repeated values, and completion hints.
- `EnumValue`: safe enum values from clap possible values.
- `OutputSpec`: explicitly declared structured output contracts, including
  encoding, buffered/streaming/interactive mode, symbolic type names, and optional
  JSON Schema payloads.

The IR should stay boring and serializable in spirit, even if we do not derive
serialization immediately. Backends should not inspect `clap::Command` directly.

## Reflection Boundaries

Clap can reliably expose:

- command names, descriptions, subcommands, and visibility;
- option spellings and positional ids;
- required flags, global flags, arity, defaults, and value names;
- possible values from enum-like parsers;
- coarse actions such as set, append, bool flag, and counter.
- coarse semantic hints such as `ValueHint::FilePath`, `ValueHint::DirPath`, and
  known parser types like `PathBuf`.

Clap cannot currently be treated as a complete cross-language IDL for arbitrary Rust
parser types. The safe default type for non-enum values is therefore `string`.

## Structured Input Metadata

Inputs should follow the same philosophy as outputs: reflect what clap knows, and
make richer contracts explicit. The baseline IR should preserve file path hints,
directory path hints, known parser types, arity, optionality, enums, repeated values,
and defaults. Backends can map those hints into idiomatic types such as
`PathLike[str]`, `PathBuf`, or plain `string` with file-oriented docs.

For semantic shapes beyond clap's native model, the right extension is explicit
schema metadata rather than inference. JSON Schema is a strong candidate because it
can describe both JSON output payloads and structured string fields that appear in
CLI inputs. A future input-contract layer could attach schema payloads to args in
the same spirit that `OutputContract` attaches schema payloads to command output.

Default generators should remain dependency-free. Schema-aware modes can opt into
Zod, serde, kotlinx.serialization, `jsonschema`, or other validators per target
language.

## Embedded Generator Command

`binding_command()` returns a hidden clap subcommand named `generate-binding`.
Applications can attach it to their real CLI and pass the resulting matches into
`generate_binding_from_matches()`. This keeps generation close to the source of
truth: package scripts can invoke `my-tool generate-binding typescript --zod`, while
normal users never see the command in help output.

The helper is intentionally a thin adapter over `generate_to`. It does not own
application dispatch, and callers pass a fresh `Command` so the reflected command
tree matches the application rather than the binding-generation command.

## Output Contracts

The stable model can carry structured-output metadata. The clap integration that
attaches those contracts directly to `clap::Command` depends on clap's unstable
extension API and is compiled only with `unstable-output-contracts`.

When that feature is enabled, the reflection layer walks visible commands and
copies attached `OutputContracts` into `CliSpec.outputs`. When it is disabled,
`CliSpec.outputs` remains empty and the crate avoids clap's unstable extension
traits entirely.

Generators omit output contracts by default. Each backend exposes
`.output_contracts()` / `.without_output_contracts()`, and the embedded CLI exposes
`--output-contracts` / `--no-output-contracts`. The generated helpers stay
dependency-free: richer validation can be added later as backend-specific optional
modes.

## TypeScript Backend

The TypeScript backend generates dependency-free TypeScript by default:

- one interface per command or subcommand;
- one `build...Command` function per command;
- string literal unions for clap possible values;
- booleans for flags;
- numbers for counters;
- strings or readonly string arrays for values and repeated args.

The generated code only builds argv. It deliberately does not call `child_process`,
Electron APIs, browser IPC, or any other runtime-specific process executor.

Node mode is the opt-in exception. It keeps `build...Command` as the primitive and
adds `create...Command`, `run...Command`, and `spawn...Command` helpers over
`node:child_process`. This is generated separately from the dependency-free default
so browser, Electron, and sandboxed runtimes do not receive Node imports.

The backend also supports an opt-in Zod validation flavor. Zod mode emits strict
schemas, infers generated types from those schemas, and can validate builder input
with `Schema.parse(args)`. This is deliberately separate from the default backend so
consumers that need a tiny dependency-free artifact keep that path.

## Python Backend

The Python backend generates formatted Python 3.10+ code:

- one frozen keyword-only dataclass per command or subcommand;
- one `build_..._args` function returning `tuple[str, ...]`;
- one `..._command` helper returning a `CommandInvocation`;
- optional namespace classes controlled by generator options;
- `Literal` unions for clap possible values;
- `int`, `float`, `bool`, and path-like annotations when clap exposes those parser
  types.

Python can lean on the standard library more than TypeScript can. The generated
`CommandInvocation.run()` method wraps `subprocess.run` in text mode, while `.argv()`
keeps the raw command available for custom runners.

Python package mode emits `_runtime.py`, `_root.py`, and one module per leaf
subcommand. Subcommands with children become packages with their own `_root.py`,
which lets `tool group` and `tool group action` both have typed APIs without module
name collisions.

## Rust Backend

The Rust backend emits one `.rs` module with public args structs, possible-value
enums, `build_..._command()` argv builders, and `CommandInvocation` helpers over
`std::process::Command`. It uses owned values so generated bindings are easy to
store, clone, and pass across application layers.

Generated Rust is formatted to pass `rustfmt --check` and uses no third-party
dependencies. Output-contract helpers frame stdout without pulling in `serde_json`.

## Kotlin Backend

The Kotlin backend emits one Kotlin/JVM `.kt` file. It uses data classes for command
arguments, enum classes for possible values, `List<String>` argv builders, and a
small `CommandInvocation` wrapper around `ProcessBuilder`.

The backend supports an optional package declaration so desktop applications can
drop generated bindings into their own package hierarchy. Output-contract helpers
frame stdout without adding a JSON dependency.

## Clap Input Styles

The reflection layer operates on `clap::Command`, so consuming CLIs can be written
with either the builder API or derive macros. Builder-style examples construct
`Command` directly. Derive-style examples call `CommandFactory::command()` from a
`#[derive(Parser)]` type and then pass the resulting command into `generate` or
`generate_to`.

## Global Arguments

Backends inherit global clap arguments into generated subcommand builders. Inherited
global values are emitted before the subcommand path, followed by command-local
arguments. This keeps generated argv close to the shape a person would type while
preserving clap's global-arg semantics.
