# Rust Generator

The Rust backend emits dependency-free Rust source that can be checked into a
consumer crate or generated during a build step.

## Shape

- one args struct per command or subcommand;
- one enum per clap possible-value argument;
- one `build_..._command(&Args) -> Vec<String>` function per command;
- one `..._command(&Args) -> CommandInvocation` helper per command;
- a `CommandInvocation` wrapper with `argv()`, `command()`, and `output()`.

Scalar types are conservative: strings stay `String`, paths become
`std::path::PathBuf`, small integers become `i32`, wide integers become `i128`, and
floating values become `f64`.

## Output Contracts

`Rust::new().output_contracts()` emits `OutputContract`, `OUTPUT_CONTRACTS`, and
`parse_output`. The parser is dependency-free: JSON payloads are returned as framed
strings rather than parsed into `serde_json::Value`. A future serde-aware mode can
layer typed JSON parsing on top without changing the default output.

## Validation

Generated Rust fixtures are checked by CI with `rustfmt`, `cargo clippy -- -D
warnings` in a temporary Cargo crate, and `rustc -D warnings`. The smoke test in
`tests/generated/rust_smoke.rs` imports the generated `repo_agent_bindings.rs`
fixture and asserts that a complex nested command builds the expected argv.
