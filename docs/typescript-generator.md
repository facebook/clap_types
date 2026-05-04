# TypeScript Generator

The TypeScript backend covers Node, Electron, editor extensions, and many agent
runtimes while still letting consumers choose their own process execution layer.
The default output is dependency-free. Zod validation and Node `child_process`
helpers are available as opt-in flavors.

## Generated Shape

For each command, the backend emits:

- an `Args` interface;
- a `build...Command(args)` function returning `string[]`.

For subcommands, the returned argv includes the subcommand path:

```ts
const argv = buildRunCommand({ target: "tests" });
// ["run", "tests"]
```

The caller can prepend the executable or pass the returned array to whichever runtime
executes the CLI.

## Options

The backend supports output file naming and runtime validation controls:

```rust
use clap_types::TypeScript;

let plain = TypeScript::new();
let zod_validated = TypeScript::new()
    .module_name("repo-agent-zod")
    .zod();
let zod_schema_only = TypeScript::new().zod_schemas();
let node_runner = TypeScript::new()
    .module_name("repo-agent-node")
    .node();
let with_output_contracts = TypeScript::new().output_contracts();
```

`module_name` controls the output file name from `generate_to`.

`zod()` emits Zod schemas, infers TypeScript types from those schemas, and validates
builder inputs with `Schema.parse(args)` before constructing argv.

`zod_schemas()` emits Zod schemas and inferred types but does not parse inside
builders. This is useful when consumers want schemas for agent/tool metadata or
external validation while keeping the builder path free of runtime checks.

`output_contracts()` emits `OUTPUT_CONTRACTS`, an `OutputContract` interface, and a
dependency-free `parseOutput()` helper for explicitly declared output metadata.
`without_output_contracts()` forces the generated module back to argv builders only.

## Type Mapping

- Boolean clap flags become `boolean`.
- Count flags become `number`.
- Possible values become string literal unions.
- Unknown value parser types become `string`.
- Multiple values and repeated options become readonly arrays.
- Global clap args are inherited into subcommand interfaces.

This is intentionally conservative. A Rust `PathBuf`, `u64`, or custom parser may
have richer semantics, but a generated cross-language wrapper cannot safely assume
that without more metadata.

## Zod Mode

Zod mode replaces generated interfaces with schemas and inferred types:

```ts
export const IssueCreateArgsSchema = z.object({
  title: z.string(),
  priority: z.enum(["low", "normal", "high", "urgent"]).optional(),
}).strict();

export type IssueCreateArgs = z.infer<typeof IssueCreateArgsSchema>;
```

When builder validation is enabled, invalid runtime input is rejected before argv is
built. Schemas are strict so unexpected keys fail validation instead of silently
falling through.

## Node Runtime Mode

Node mode layers typed process helpers over the same argv builders:

```rust
use clap_types::TypeScript;

let generator = TypeScript::new().node();
```

Generated modules import `node:child_process` and emit:

- `create...Command(args, { program })` returning `{ file, args }`;
- `run...Command(args, options)` using `execFile`;
- `spawn...Command(args, options)` using `spawn`;
- `CliCommand`, `CliRunOptions`, `CliSpawnOptions`, and `CliRunResult` types.

The generated code still avoids shell strings. Consumers can pass the command
descriptor to their own runner, or use the convenience methods when plain Node
execution is enough:

```ts
const command = createAgentRunCommand({ task: "summarize" });
const result = await runAgentRunCommand({ task: "summarize" }, { cwd: "/repo" });
```

## Output Contracts

When enabled, output contracts are rendered separately from argument builders. JSON
contracts use `JSON.parse`, JSON-lines contracts split non-empty lines and parse each
line, and text contracts return stdout unchanged. JSON Schema payloads are emitted as
schema strings for consumers that want to hand them to their own validator.

## Future Enhancements

- Emit `d.ts` plus implementation split when useful.
- Add optional runtime validators that translate declared JSON Schema payloads into
  Zod or another validation framework.
