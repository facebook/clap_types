# Examples

The repository keeps examples in two layers: Rust generators that reflect clap apps,
and client examples that consume the generated bindings.

## Rust Generators

- `examples/generate_typescript.rs`: minimal builder-style clap app that prints
  TypeScript bindings to stdout.
- `examples/generate_complex_bindings.rs`: builder-style clap apps with nested
  subcommands, global args, enums, repeated options, paths, integers, floats, and
  booleans.
- `examples/generate_derive_bindings.rs`: derive-style clap app using
  `#[derive(Parser)]`, `Args`, `Subcommand`, `ValueEnum`, and
  `CommandFactory::command()`.
- `examples/generate_binding_subcommand.rs`: app-shaped example that embeds a
  hidden `generate-binding` command so the CLI can generate its own TypeScript,
  TypeScript + Zod, TypeScript + Node, Python, Python package, Rust, and Kotlin
  bindings.

Run all generated fixtures with:

```sh
npm run generate:fixtures
```

This writes single-file Python, package-layout Python, dependency-free TypeScript,
Node TypeScript, Zod TypeScript, Rust, and Kotlin bindings under
`target/generated`.

Run the hidden binding command example directly:

```sh
cargo run --example generate_binding_subcommand -- generate-binding typescript --zod --node --path target/generated/manual-typescript
cargo run --example generate_binding_subcommand -- generate-binding python --full-module --module-name repo_agent --namespace RepoAgent --path target/generated/manual-python
cargo run --example generate_binding_subcommand -- generate-binding rust --module-name repo_agent --output-contracts --path target/generated/manual-rust
cargo run --example generate_binding_subcommand -- generate-binding kotlin --module-name repo_agent --package dev.example.repoagent --path target/generated/manual-kotlin
```

## Client Examples

- `examples/clients/python_repo_agent.py`: consumes Python bindings generated from a
  builder-style Rust CLI.
- `examples/clients/python_package_repo_agent.py`: consumes package-layout Python
  bindings and imports a nested subcommand module directly.
- `examples/clients/python_opsctl_derive.py`: consumes Python bindings generated
  from a derive-style Rust CLI.
- `examples/clients/typescript_repo_agent.ts`: consumes dependency-free TypeScript
  bindings generated from a builder-style Rust CLI.
- `examples/clients/typescript_node_repo_agent.ts`: consumes the Node
  `child_process` command descriptor helpers.
- `examples/clients/typescript_zod_opsctl.ts`: consumes Zod TypeScript bindings
  generated from a derive-style Rust CLI.
- `tests/generated/rust_smoke.rs`: consumes generated Rust bindings and checks a
  complex nested command argv.
- `tests/generated/kotlin*_smoke.kt`: consume generated Kotlin bindings and check
  complex nested command argv.

Run the TypeScript clients with:

```sh
npm run test:generated:ts
```

Run the Python clients with:

```sh
python examples/clients/python_repo_agent.py target/generated
python examples/clients/python_package_repo_agent.py target/generated
python examples/clients/python_opsctl_derive.py target/generated
```
