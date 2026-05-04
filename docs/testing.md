# Testing Strategy

## Rust

Rust CI should run:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

Unit tests should cover:

- clap reflection for options, flags, counters, positionals, enums, defaults, and
  subcommands;
- variadic positionals (`num_args(1..)`) and their ordering relative to options;
- hidden `generate-binding` command dispatch for TypeScript, Zod, Node, Python,
  Python package, Rust, and Kotlin generation;
- `unstable-output-contracts` reflection when running `--all-features`;
- opt-in output-contract rendering for every backend;
- backend output for each supported language;
- file generation through `generate_to`.

## Generated Code

Generated code should be tested as code, not only as strings. For TypeScript, add
fixtures that write generated output into a temporary package and run:

```sh
npm ci
npm run generate:fixtures
npm run check:generated:ts
bun tests/generated/typescript_smoke.ts
bun tests/generated/typescript_zod_smoke.ts
bun tests/generated/typescript_node_smoke.ts
```

The generated libraries should remain dependency-free by default. If a backend grows
an optional validation mode, that mode should have separate fixtures and snapshots.

For Python, CI should generate fixtures and run:

```sh
python -m ruff check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
python -m ruff format --check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
python -m black --check tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients/*.py target/generated/python target/generated/python-package
python -m compileall -q target/generated/python target/generated/python-package tests/generated/python_smoke.py tests/generated/python_package_smoke.py examples/clients
python tests/generated/python_smoke.py target/generated
python tests/generated/python_package_smoke.py target/generated
python examples/clients/python_repo_agent.py target/generated
python examples/clients/python_package_repo_agent.py target/generated
python examples/clients/python_opsctl_derive.py target/generated
```

The CI matrix covers Python 3.10, 3.11, 3.12, 3.13, and 3.14.

For generated Rust and Kotlin, CI should run:

```sh
npm run generate:fixtures
npm run lint:generated:rust
npm run check:generated:rust
npm run lint:generated:kotlin
npm run check:generated:kotlin
```

`lint:generated:rust` checks generated formatting with `rustfmt` and runs generated
Rust through a temporary Cargo crate with `cargo clippy -- -D warnings`.
`check:generated:rust` also compiles and runs `tests/generated/rust_smoke.rs` with
`rustc -D warnings`.

`lint:generated:kotlin` compiles every generated Kotlin fixture with `-Werror`,
`-Wextra`, progressive mode, and bytecode validation. `check:generated:kotlin`
compiles each generated binding with its matching `tests/generated/kotlin*_smoke.kt`
file using the same warning policy and runs the resulting JVM jars. Kotlin compiler
installation happens in CI because not every local contributor machine has `kotlinc`
on `PATH`.

## Examples

Examples should serve two jobs:

- document how a consuming clap app invokes `clap_types`;
- provide realistic command trees for generator tests.

Keep examples broad enough to exercise modern clap features, but small enough that
failures point directly at the generator behavior.

`examples/generate_complex_bindings.rs` defines two larger CLIs and emits Python,
Python package, TypeScript, TypeScript Node, TypeScript Zod, Rust, and Kotlin
fixtures.
`examples/generate_derive_bindings.rs` defines a derive-macro CLI and emits the same
language targets. Those fixtures exercise nested
subcommands, global args, required options and positionals, repeated options, enums,
integers, floats, booleans, path-like values, generated docs, strict Zod validation,
Node process helpers, package imports, subprocess conveniences, rustfmt-clean Rust,
and Kotlin/JVM command builders.
