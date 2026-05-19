# Structured Output Contracts

Clap describes command-line inputs. It does not describe stdout, stderr, exit status,
or streaming protocols. Because of that, `clap_types` should not pretend it can parse
arbitrary CLI output reliably just by inspecting `clap::Command`.

## Direction

Structured output should be explicit. The likely path is an opt-in API layered on top
of clap reflection. The current experimental API uses clap's unstable extension
mechanism, so it is gated behind `clap_types`' own feature flag:

```toml
[dependencies]
clap_types = { version = "0.1", features = ["unstable-output-contracts"] }
```

```rust
use clap::Command;
use clap_types::{ClapTypesCommandExt, OutputContract};

fn cli() -> Command {
    Command::new("repo-agent").subcommand(
        Command::new("watch").output_contract(
            OutputContract::json_lines("WatchEvent").json_schema(
                r#"{"type":"object","required":["event"],"properties":{"event":{"type":"string"}}}"#,
            ),
        ),
    )
}
```

With `unstable-output-contracts` enabled, `reflect_command` records those contracts
in `CliSpec.outputs`. With the feature disabled, the crate does not compile against
clap's unstable extension traits and reflection leaves `outputs` empty.

The same pattern should apply to richer input semantics. Clap gives us the baseline
input shape, including parser types and value hints, while explicit schema metadata
can describe higher-level meaning such as "this string is a file path", "this option
accepts a JSON object", or "this argument is a path-like field inside a JSON Schema
document."

Generators can then render optional output-contract sections:

- TypeScript contract metadata and JSON / JSON-lines parse helpers;
- Python contract metadata and `json.loads` / JSON-lines parse helpers;
- Rust contract metadata and dependency-free framed stdout helpers;
- Kotlin contract metadata and dependency-free framed stdout helpers;
- runtime-specific helpers that choose buffered, streaming, or inherited stdio
  execution based on the contract.

Every backend omits this section by default. Use `.output_contracts()` in Rust code
or `--output-contracts` on the embedded `generate-binding` command to include it.
Use `.without_output_contracts()` or `--no-output-contracts` to force omission.

## Supported Contract Shapes

Initial output contract shapes should be small:

- `Json`: one JSON document on stdout.
- `JsonLines`: newline-delimited JSON records on stdout.
- `Text`: explicitly unstructured text.
- `Interactive`: a command that expects inherited stdio or a TTY.
- `Streaming`: a command whose stdout should be consumed incrementally.
- `JsonSchema`: optional schema metadata attached to a structured contract.

Exit status should be modeled separately from stdout payload. Stderr should be
treated as diagnostics by default unless a command explicitly documents otherwise.
This likely wants its own explicit contract type: success payloads, expected
non-zero statuses, machine-readable error payloads, retryability, and whether stderr
is human diagnostics or structured data. Generated clients can then expose idiomatic
result/error shapes instead of forcing every caller to rediscover the same
exit-code table.

Node runtime generation already gives us a natural place to consume these future
contracts: buffered contracts can use `execFile`, streaming contracts can use
`spawn`, and interactive contracts can select inherited stdio. Until contracts are
explicitly declared, generated runners should return raw `stdout`/`stderr` strings
and avoid pretending to know output schemas.

JSON Schema is the first schema payload because it is language-neutral and can be
consumed by TypeScript, Python, Kotlin, and external tool registries. Generators
should treat schema emission as optional: the default generated argv builders should
stay dependency-free, while validation-oriented modes can translate or embed schemas
where that is idiomatic.

For inputs, JSON Schema should be additive rather than a replacement for clap
reflection. A path argument reflected from `PathBuf` should still be a path-like
type even without schema metadata. A schema should enrich cases where the Rust
parser accepts a string but the author knows the string has a deeper structure.

TypeScript and Python have standard or common JSON parsing paths available today, so
their opt-in helpers can parse JSON framing immediately. Rust and Kotlin standard
libraries do not include JSON parsers, so their default helpers preserve JSON
payloads as strings and split JSON-lines into non-empty line strings. Serde,
kotlinx.serialization, Zod-from-schema, or other validation adapters should remain
explicit backend modes rather than default generated dependencies.

## Non-goals

- Inferring output schemas from help text.
- Parsing human-oriented tables as a stable API.
- Requiring generated libraries to depend on a validation framework.

Generated validation can be offered later as an optional backend mode, but the default
should stay dependency-free.
