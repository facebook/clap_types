# AGENTS.md

## Project Intent

`clap_types` generates strongly-typed argv builders from Rust CLIs defined with `clap`.
Think of it as a sibling to `clap_complete`, `clap_mangen`, and `clap-markdown`, but
for programmatic command invocation in agent, Node, Electron, Python, Kotlin, and
other runtime environments.

Generated libraries should build arguments, not execute processes. Execution belongs
to the caller's runtime.

## Architecture

- `src/reflect.rs`: converts `clap::Command` into the internal model. Exposes
  `reflect_command`, `reflect_command_with_name`, and the
  `reflect_command_with_options` + `ReflectOptions` pair for opting into hidden
  subcommands and args.
- `src/model.rs`: language-neutral IR shared by all generators. All public
  structs and enums are `#[non_exhaustive]`.
- `src/generate.rs`: public `Generator` trait plus the `generate`,
  `generate_to`, and `generate_to_with_options` entry points.
- `src/bindings_cli.rs`: reusable hidden `generate-binding` clap subcommand
  (`binding_command`, `generate_binding_from_matches`, and the
  `_with_outputs` variant for injecting caller-supplied output specs).
- `src/output_contracts.rs`: optional clap extension bridge compiled with
  `unstable-output-contracts`.
- `src/typescript.rs`: TypeScript backend.
- `src/flow.rs`: Flow-annotated JavaScript backend.
- `src/python.rs`: Python 3.10+ backend.
- `src/rust.rs`: dependency-free Rust backend.
- `src/kotlin.rs`: Kotlin/JVM desktop backend.
- `docs/`: architecture, output-contract direction, per-backend notes, and
  test strategy.
- `examples/`: small consuming-clap examples.
- `examples/clients/`: runnable Python, TypeScript, and Flow consumers of
  generated code.
- `tests/generated/`: generated-code smoke tests for Python, TypeScript, Flow,
  Rust, and Kotlin.

Backends should render from `CliSpec`; do not make each backend rediscover clap
reflection details.

## Quality Bar

Before sending a PR, run:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

`--all-features` is important because it compiles the `unstable-output-contracts`
integration against clap's unstable extension API.

If a change affects generated code, run:

```sh
cargo run --example generate_complex_bindings -- target/generated
cargo run --example generate_derive_bindings -- target/generated
rustfmt --check target/generated/rust/*.rs
npm run check:generated:rust
npm run check:generated:kotlin
ruff check tests/generated/python_smoke.py examples/clients/*.py target/generated/python
ruff format --check tests/generated/python_smoke.py examples/clients/*.py target/generated/python
npm run check:generated:ts
npm run test:generated:ts
```

Black is also enforced in CI for generated Python. Default generated outputs should
stay dependency-free. Optional generated-code modes, such as TypeScript Zod
validation or output-contract helpers, must be tested separately from the
dependency-free baseline.

`npm run check:generated:kotlin` requires `kotlinc`; CI installs the standalone
compiler when local machines do not have it.

## Design Principles

- Prefer conservative type generation over false precision.
- Preserve clap semantics where they are visible through public reflection APIs.
- Treat arbitrary output parsing as an explicit contract, not something inferred
  from clap input definitions.
- Keep public APIs small and close to existing clap generator conventions.
- Avoid hidden runtime coupling in generated libraries.
