# Contributing to clap_types

## Welcome

Thanks for your interest in contributing to this repository.

`clap_types` generates type-safe argv builders from Rust CLIs defined with
`clap`. Before contributing, please review our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

Contributions are accepted through pull requests.

1. Fork the repository.
2. Create a feature branch from `main`.
3. Keep changes focused and include tests or generated-output updates when needed.
4. Open a pull request with a clear description of what changed, why it changed,
   and which generators or fixtures were affected.

## Development Notes

When making changes:

- Prefer conservative type generation over false precision.
- Preserve `clap` semantics where they are visible through public reflection APIs.
- Keep backends rendering from `CliSpec` rather than rediscovering reflection details.
- Generated libraries should build arguments, not execute processes.
- Keep default generated outputs dependency-free unless the mode is explicitly optional.

## Validation

Before sending a pull request, run:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

If a change affects generated code, also run:

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

## Reporting Bugs

If you find an issue, open a pull request or use the repository's issue tracker if
it is enabled in your environment.

Please include:

- A clear description of the problem
- Reproduction steps when applicable
- The affected generator, backend, or fixture paths
- The expected and actual behavior

## Contributor License Agreement (CLA)

All contributors must sign the Meta Contributor License Agreement (CLA) before
pull requests can be merged.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
Please report unacceptable behavior to opensource-conduct@meta.com.
