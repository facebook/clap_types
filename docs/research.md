# Research Notes

## Related clap generators

`clap_complete` is the closest API model. It exposes a `Generator` trait plus
runtime `generate` and file-oriented `generate_to` functions. Its docs explicitly
support both runtime generation and compile-time generation from `build.rs` or
`cargo xtask`.

`clap_mangen` is a strong reflection model. It accepts a `clap::Command`, calls
`build`, walks visible subcommands, and renders the command into another artifact.
It also has a `generate_to` helper for writing all command and subcommand manpages.

`clap-markdown` is useful for project workflow conventions. It shows that a CLI can
add a hidden generation flag, write generated docs into `docs/`, and track CLI API
changes through source control.

## Practical lessons

- Keep the public API tiny: `Generator`, `generate`, and `generate_to`.
- Reflect `clap::Command` once into a language-neutral model, then render language
  backends from that model.
- Prefer `cargo xtask` or explicit generation commands for committed artifacts; use
  `build.rs` only when a consuming project genuinely wants compile-time output.
- Skip hidden clap nodes by default. Hidden flags are usually implementation details
  or generation hooks, not programmatic API.
- Treat clap's value parser type information conservatively. Enumerated possible
  values are visible and safe to turn into language unions; arbitrary Rust parser
  types are not yet a portable IDL.
- Use the current setup actions for generated-code CI: `actions/setup-python@v6`,
  `actions/setup-node@v6`, and `oven-sh/setup-bun@v2`.

## Sources

- [`clap_complete` docs](https://docs.rs/clap_complete/latest/clap_complete/)
- [`clap_complete::Generator`](https://docs.rs/clap_complete/latest/clap_complete/aot/trait.Generator.html)
- [`clap_complete::generate_to`](https://docs.rs/clap_complete/latest/clap_complete/aot/fn.generate_to.html)
- [`clap_mangen` docs](https://docs.rs/clap_mangen/latest/clap_mangen/)
- [`clap_mangen` source on docs.rs](https://docs.rs/clap_mangen/latest/src/clap_mangen/lib.rs.html)
- [`clap-markdown` docs](https://docs.rs/clap-markdown/latest/clap_markdown/)
- [`clap-markdown` repository](https://github.com/ConnorGray/clap-markdown)
- [`clap::Arg` reflection methods](https://docs.rs/clap/latest/clap/struct.Arg.html)
- [`clap::ArgAction`](https://docs.rs/clap/latest/clap/enum.ArgAction.html)
- [`actions/setup-python`](https://github.com/actions/setup-python)
- [`actions/setup-node`](https://github.com/actions/setup-node)
- [`oven-sh/setup-bun`](https://github.com/oven-sh/setup-bun)
