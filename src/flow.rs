// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::io::{self, Write};

use crate::codegen::{
    capitalize, collapse_lines, combined_args, command_type_prefix, ensure_unique_command_prefixes,
    inherited_globals, is_grouped_repeated, is_required, lower_join, option_token, output_encoding,
    output_mode, output_schema, quote_double, safe_identifier, words,
};
use crate::generate::{Generator, OutputContractGeneration};
use crate::model::{ArgKind, ArgSpec, CliSpec, CommandSpec, ValueType};

/// Options for the Flow backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowOptions {
    /// Generated module file stem, without `.js`.
    pub module_name: Option<String>,
    /// Runtime validation strategy for generated args.
    pub validation: FlowValidation,
    /// Optional execution helpers to layer on top of argv builders.
    pub runtime: FlowRuntime,
    /// Output-contract metadata and parser helpers.
    pub output_contracts: OutputContractGeneration,
}

impl Default for FlowOptions {
    fn default() -> Self {
        Self {
            module_name: None,
            validation: FlowValidation::None,
            runtime: FlowRuntime::None,
            output_contracts: OutputContractGeneration::Omit,
        }
    }
}

/// Runtime validation strategy for generated Flow bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowValidation {
    /// Generate dependency-free Flow types and builders.
    #[default]
    None,
    /// Generate Zod schemas alongside Flow types.
    Zod {
        /// Validate builder input with `Schema.parse(args)` before building argv.
        validate_builders: bool,
    },
}

impl FlowValidation {
    fn is_zod(self) -> bool {
        matches!(self, Self::Zod { .. })
    }

    fn validates_builders(self) -> bool {
        matches!(
            self,
            Self::Zod {
                validate_builders: true,
            }
        )
    }
}

/// Runtime helpers emitted by the Flow backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowRuntime {
    /// Generate argv builders only.
    #[default]
    None,
    /// Generate Node.js `child_process` helpers.
    Node,
}

impl FlowRuntime {
    fn is_node(self) -> bool {
        matches!(self, Self::Node)
    }
}

/// Generate JavaScript modules annotated with Flow types.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flow {
    options: FlowOptions,
}

impl Flow {
    /// Create a Flow generator with default dependency-free output.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Flow generator from explicit options.
    #[must_use]
    pub fn with_options(options: FlowOptions) -> Self {
        Self { options }
    }

    /// Set the generated module name, without `.js`.
    #[must_use]
    pub fn module_name(mut self, module_name: impl Into<String>) -> Self {
        self.options.module_name = Some(module_name.into());
        self
    }

    /// Emit Zod schemas and validate builder inputs with `Schema.parse`.
    #[must_use]
    pub fn zod(mut self) -> Self {
        self.options.validation = FlowValidation::Zod {
            validate_builders: true,
        };
        self
    }

    /// Emit Zod schemas without validating inside builders.
    #[must_use]
    pub fn zod_schemas(mut self) -> Self {
        self.options.validation = FlowValidation::Zod {
            validate_builders: false,
        };
        self
    }

    /// Set the runtime validation strategy directly.
    #[must_use]
    pub fn validation(mut self, validation: FlowValidation) -> Self {
        self.options.validation = validation;
        self
    }

    /// Emit Node.js `child_process` helpers alongside argv builders.
    #[must_use]
    pub fn node(mut self) -> Self {
        self.options.runtime = FlowRuntime::Node;
        self
    }

    /// Set the runtime helper strategy directly.
    #[must_use]
    pub fn runtime(mut self, runtime: FlowRuntime) -> Self {
        self.options.runtime = runtime;
        self
    }

    /// Emit output-contract metadata and parser helpers.
    #[must_use]
    pub fn output_contracts(mut self) -> Self {
        self.options.output_contracts = OutputContractGeneration::Emit;
        self
    }

    /// Omit output-contract metadata and parser helpers.
    #[must_use]
    pub fn without_output_contracts(mut self) -> Self {
        self.options.output_contracts = OutputContractGeneration::Omit;
        self
    }
}

impl Generator for Flow {
    fn file_name(&self, bin_name: &str) -> String {
        let stem = self
            .options
            .module_name
            .as_deref()
            .map(kebab_file_stem)
            .unwrap_or_else(|| kebab_file_stem(bin_name));
        format!("{stem}.js")
    }

    fn generate(&self, spec: &CliSpec, buf: &mut dyn Write) -> io::Result<()> {
        ensure_unique_command_prefixes(spec)?;
        let mut output = String::new();
        render_module(spec, &self.options, &mut output);
        buf.write_all(output.as_bytes())
    }
}

fn render_module(spec: &CliSpec, options: &FlowOptions, output: &mut String) {
    let validation = options.validation;
    let runtime = options.runtime;

    output.push_str("// Generated by clap_types. Do not edit by hand.\n");
    output.push_str("// @ts-nocheck\n");
    output.push_str("// @flow strict\n");
    if runtime.is_node() {
        output.push_str("import { execFile, spawn } from \"node:child_process\";\n");
    }
    if validation.is_zod() {
        output.push_str("import { z } from \"zod\";\n\n");
    } else if runtime.is_node() {
        output.push('\n');
    }
    if runtime.is_node() {
        output.push_str(&format!(
            "const PROGRAM: string = {};\n\n",
            quote_double(&spec.bin_name)
        ));
    }
    if validation.is_zod() {
        output.push_str(
            r#"type ZodSafeParseResult<T> =
  | {| +success: true, +data: T |}
  | {| +success: false, +error: mixed |};

type ZodSchema<T> = {|
  +parse: (input: mixed) => T,
  +safeParse: (input: mixed) => ZodSafeParseResult<T>,
|};

"#,
        );
    }
    output.push_str("type ArgValue = string | number | boolean;\n");
    output.push_str("type ArgValueInput = ArgValue | $ReadOnlyArray<ArgValue>;\n\n");
    render_helpers(output);
    if runtime.is_node() {
        render_node_helpers(output);
    }
    if options.output_contracts.is_enabled() {
        render_output_contracts(spec, output);
    }

    let mut path = Vec::<String>::new();
    let inherited: Vec<&ArgSpec> = Vec::new();
    render_command_tree(&spec.root, &mut path, &inherited, options, output);
}

fn render_helpers(output: &mut String) {
    output.push_str(
        r#"function pushOne(argv: Array<string>, value: ArgValue): void {
  argv.push(String(value));
}

function pushValues(argv: Array<string>, value: ArgValueInput): void {
  if (Array.isArray(value)) {
    for (const item of value) {
      pushOne(argv, item);
    }
    return;
  }

  pushOne(argv, value);
}

function pushOption(argv: Array<string>, option: string, value: ArgValueInput): void {
  argv.push(option);
  pushValues(argv, value);
}

function pushRepeatedOption(argv: Array<string>, option: string, value: ArgValueInput): void {
  if (Array.isArray(value)) {
    for (const item of value) {
      pushOption(argv, option, item);
    }
    return;
  }

  pushOption(argv, option, value);
}

function pushGroupedRepeatedOption(
  argv: Array<string>,
  option: string,
  groups: $ReadOnlyArray<$ReadOnlyArray<ArgValue>>,
): void {
  for (const group of groups) {
    pushOption(argv, option, group);
  }
}

function requireValue<T>(name: string, value: T | void): T {
  if (value === undefined) {
    throw new TypeError(`Missing required CLI argument: ${name}`);
  }

  return value;
}

"#,
    );
}

fn render_output_contracts(spec: &CliSpec, output: &mut String) {
    output.push_str(
        r#"export type OutputEncoding = "json" | "json-lines" | "text";
export type OutputMode = "buffered" | "streaming" | "interactive";

export type OutputContract = {|
  +commandPath: $ReadOnlyArray<string>,
  +encoding: OutputEncoding,
  +mode: OutputMode,
  +typeName: string,
  +schema?: string,
|};

export type ParsedOutput = mixed | $ReadOnlyArray<mixed> | string;

"#,
    );
    output.push_str("export const OUTPUT_CONTRACTS: $ReadOnlyArray<OutputContract> = [\n");
    for contract in &spec.outputs {
        output.push_str("  {\n");
        output.push_str(&format!(
            "    commandPath: [{}],\n",
            contract
                .command_path
                .iter()
                .map(|piece| quote_double(piece))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        output.push_str(&format!(
            "    encoding: {},\n",
            quote_double(output_encoding(contract.encoding))
        ));
        output.push_str(&format!(
            "    mode: {},\n",
            quote_double(output_mode(contract.mode))
        ));
        output.push_str(&format!(
            "    typeName: {},\n",
            quote_double(&contract.type_name)
        ));
        if let Some(schema) = contract.schema.as_ref().map(output_schema) {
            output.push_str(&format!("    schema: {},\n", quote_double(schema)));
        }
        output.push_str("  },\n");
    }
    output.push_str(
        r#"];

export function parseOutput(contract: OutputContract, stdout: string): ParsedOutput {
  if (contract.mode === "interactive") {
    return stdout;
  }

  switch (contract.encoding) {
    case "json":
      return JSON.parse(stdout);
    case "json-lines":
      return stdout
        .split(/\r?\n/)
        .filter((line) => line.length > 0)
        .map((line) => JSON.parse(line));
    case "text":
      return stdout;
    default:
      return stdout;
  }
}

"#,
    );
}

fn render_node_helpers(output: &mut String) {
    output.push_str(
        r#"export type ChildProcess = any;

export type CliCommand = {|
  +file: string,
  +args: Array<string>,
|};

export type CliCommandOptions = {
  +program?: string,
  ...
};

export type CliRunOptions = {
  +program?: string,
  ...
};

export type CliSpawnOptions = {
  +program?: string,
  ...
};

export type CliRunResult = {|
  +stdout: string,
  +stderr: string,
|};

function runCliCommand(command: CliCommand, options: CliRunOptions = {}): Promise<CliRunResult> {
  const execOptions: any = { ...options };
  delete execOptions.program;

  return new Promise((resolve, reject) => {
    execFile(command.file, command.args, { encoding: "utf8", ...execOptions }, (error, stdout, stderr) => {
      const result = { stdout: String(stdout), stderr: String(stderr) };
      if (error) {
        const enrichedError: any = error;
        enrichedError.stdout = result.stdout;
        enrichedError.stderr = result.stderr;
        reject(enrichedError);
        return;
      }

      resolve(result);
    });
  });
}

function spawnCliCommand(command: CliCommand, options: CliSpawnOptions = {}): ChildProcess {
  const spawnOptions: any = { ...options };
  delete spawnOptions.program;
  return spawn(command.file, command.args, spawnOptions);
}

"#,
    );
}

fn render_command_tree(
    command: &CommandSpec,
    path: &mut Vec<String>,
    inherited: &[&ArgSpec],
    options: &FlowOptions,
    output: &mut String,
) {
    render_command(command, path, inherited, options, output);

    let next_inherited = inherited_globals(inherited, command);

    for subcommand in &command.subcommands {
        path.push(subcommand.name.clone());
        render_command_tree(subcommand, path, &next_inherited, options, output);
        path.pop();
    }
}

fn render_command(
    command: &CommandSpec,
    path: &[String],
    inherited: &[&ArgSpec],
    options: &FlowOptions,
    output: &mut String,
) {
    let validation = options.validation;
    let type_prefix = command_type_prefix(command, path);
    let args_name = format!("{type_prefix}Args");
    let schema_name = format!("{args_name}Schema");
    let function_name = format!("build{type_prefix}Command");
    let all_args = combined_args(inherited, command);
    let has_required = all_args.iter().copied().any(is_required);

    render_type_alias(&args_name, &all_args, output);
    output.push('\n');
    if validation.is_zod() {
        render_zod_schema(&schema_name, &args_name, &all_args, output);
        output.push('\n');
    }

    if has_required {
        output.push_str(&format!(
            "export function {function_name}(args: {args_name}): Array<string> {{\n"
        ));
    } else {
        output.push_str(&format!(
            "export function {function_name}(args: {args_name} = {{}}): Array<string> {{\n"
        ));
    }

    let source = if validation.validates_builders() {
        output.push_str(&format!(
            "  const parsed: {args_name} = {schema_name}.parse(args);\n"
        ));
        "parsed"
    } else {
        "args"
    };

    output.push_str("  const argv: Array<string> = [];\n");

    for &arg in inherited {
        render_arg_builder(arg, source, output);
    }

    for token in path {
        output.push_str(&format!("  argv.push({});\n", quote_double(token)));
    }

    for arg in &command.args {
        if !inherited.iter().any(|existing| existing.id == arg.id) {
            render_arg_builder(arg, source, output);
        }
    }

    output.push_str("  return argv;\n");
    output.push_str("}\n\n");

    if options.runtime.is_node() {
        render_node_command_helpers(
            &type_prefix,
            &args_name,
            &function_name,
            has_required,
            output,
        );
    }
}

fn render_type_alias(args_name: &str, args: &[&ArgSpec], output: &mut String) {
    output.push_str(&format!("export type {args_name} = {{|\n"));

    for &arg in args {
        let prop = property_name(arg);
        let marker = if is_required(arg) { "" } else { "?" };
        let flow_type = flow_type(arg);

        if let Some(help) = arg.help.as_deref().or(arg.long_help.as_deref()) {
            output.push_str(&format!("  /** {} */\n", doc_comment(help)));
        }

        output.push_str(&format!("  +{prop}{marker}: {flow_type},\n"));
    }

    output.push_str("|};\n");
}

fn render_node_command_helpers(
    type_prefix: &str,
    args_name: &str,
    build_fn: &str,
    has_required: bool,
    output: &mut String,
) {
    let create_fn = format!("create{type_prefix}Command");
    let run_fn = format!("run{type_prefix}Command");
    let spawn_fn = format!("spawn{type_prefix}Command");
    let args_param = if has_required {
        format!("args: {args_name}")
    } else {
        format!("args: {args_name} = {{}}")
    };

    output.push_str(&format!(
        "export function {create_fn}({args_param}, options: CliCommandOptions = {{}}): CliCommand {{\n"
    ));
    output.push_str("  return {\n");
    output.push_str("    file: options.program ?? PROGRAM,\n");
    output.push_str(&format!("    args: {build_fn}(args),\n"));
    output.push_str("  };\n");
    output.push_str("}\n\n");

    output.push_str(&format!(
        "export function {run_fn}({args_param}, options: CliRunOptions = {{}}): Promise<CliRunResult> {{\n"
    ));
    output.push_str(&format!(
        "  return runCliCommand({create_fn}(args, options), options);\n"
    ));
    output.push_str("}\n\n");

    output.push_str(&format!(
        "export function {spawn_fn}({args_param}, options: CliSpawnOptions = {{}}): ChildProcess {{\n"
    ));
    output.push_str(&format!(
        "  return spawnCliCommand({create_fn}(args, options), options);\n"
    ));
    output.push_str("}\n\n");
}

fn render_zod_schema(schema_name: &str, args_name: &str, args: &[&ArgSpec], output: &mut String) {
    output.push_str(&format!(
        "export const {schema_name}: ZodSchema<{args_name}> = z.strictObject({{\n"
    ));

    for &arg in args {
        let prop = property_name(arg);
        let schema = zod_schema(arg);

        if let Some(help) = arg.help.as_deref().or(arg.long_help.as_deref()) {
            output.push_str(&format!("  /** {} */\n", doc_comment(help)));
        }

        output.push_str(&format!("  {prop}: {schema},\n"));
    }

    output.push_str("});\n");
}

fn render_arg_builder(arg: &ArgSpec, source: &str, output: &mut String) {
    let prop = property_name(arg);
    let option = option_token(arg);
    let accessor = format!("{source}.{prop}");

    match arg.kind {
        ArgKind::FlagTrue => {
            if let Some(option) = option {
                output.push_str(&format!("  if ({accessor} === true) {{\n"));
                output.push_str(&format!("    argv.push({});\n", quote_double(&option)));
                output.push_str("  }\n");
            }
        }
        ArgKind::FlagFalse => {
            if let Some(option) = option {
                output.push_str(&format!("  if ({accessor} === false) {{\n"));
                output.push_str(&format!("    argv.push({});\n", quote_double(&option)));
                output.push_str("  }\n");
            }
        }
        ArgKind::Counter => {
            if let Some(option) = option {
                output.push_str(&format!(
                    "  for (let index = 0; index < ({accessor} ?? 0); index += 1) {{\n"
                ));
                output.push_str(&format!("    argv.push({});\n", quote_double(&option)));
                output.push_str("  }\n");
            }
        }
        ArgKind::Option => {
            if let Some(option) = option {
                let helper = if is_grouped_repeated(arg) {
                    "pushGroupedRepeatedOption"
                } else if arg.value.repeated {
                    "pushRepeatedOption"
                } else {
                    "pushOption"
                };
                if is_required(arg) {
                    output.push_str(&format!(
                        "  {helper}(argv, {}, requireValue({}, {accessor}));\n",
                        quote_double(&option),
                        quote_double(&arg.id)
                    ));
                } else {
                    output.push_str(&format!("  if ({accessor} !== undefined) {{\n"));
                    output.push_str(&format!(
                        "    {helper}(argv, {}, {accessor});\n",
                        quote_double(&option)
                    ));
                    output.push_str("  }\n");
                }
            }
        }
        ArgKind::Positional => {
            if is_required(arg) {
                output.push_str(&format!(
                    "  pushValues(argv, requireValue({}, {accessor}));\n",
                    quote_double(&arg.id)
                ));
            } else {
                output.push_str(&format!("  if ({accessor} !== undefined) {{\n"));
                output.push_str(&format!("    pushValues(argv, {accessor});\n"));
                output.push_str("  }\n");
            }
        }
    }
}

fn flow_type(arg: &ArgSpec) -> String {
    let scalar = match arg.kind {
        ArgKind::FlagTrue | ArgKind::FlagFalse => "boolean".to_owned(),
        ArgKind::Counter => "number".to_owned(),
        ArgKind::Option | ArgKind::Positional => enum_or_string_type(arg),
    };

    if !matches!(arg.kind, ArgKind::Option | ArgKind::Positional) {
        return scalar;
    }

    if is_grouped_repeated(arg) {
        format!("$ReadOnlyArray<$ReadOnlyArray<{scalar}>>")
    } else if arg.value.repeated || arg.value.arity.allows_multiple() {
        format!("$ReadOnlyArray<{scalar}>")
    } else {
        scalar
    }
}

fn zod_schema(arg: &ArgSpec) -> String {
    let scalar = zod_scalar_schema(arg);
    let is_array = matches!(arg.kind, ArgKind::Option | ArgKind::Positional)
        && (arg.value.repeated || arg.value.arity.allows_multiple());

    let schema = if is_grouped_repeated(arg) {
        let mut inner = format!("z.array({scalar})");
        if arg.value.arity.min > 0 {
            inner.push_str(&format!(".min({})", arg.value.arity.min));
        }
        if let Some(max) = arg.value.arity.max {
            inner.push_str(&format!(".max({max})"));
        }
        format!("z.array({inner})")
    } else if is_array {
        let mut array_schema = format!("z.array({scalar})");
        if arg.value.arity.min > 0 {
            array_schema.push_str(&format!(".min({})", arg.value.arity.min));
        }
        if !arg.value.repeated {
            if let Some(max) = arg.value.arity.max {
                array_schema.push_str(&format!(".max({max})"));
            }
        }
        array_schema
    } else {
        scalar
    };

    if is_required(arg) {
        schema
    } else {
        format!("{schema}.optional()")
    }
}

fn zod_scalar_schema(arg: &ArgSpec) -> String {
    if !arg.possible_values.is_empty() {
        return format!(
            "z.enum([{}])",
            arg.possible_values
                .iter()
                .map(|value| quote_double(&value.name))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    match arg.kind {
        ArgKind::FlagTrue | ArgKind::FlagFalse => "z.boolean()".to_owned(),
        ArgKind::Counter => "z.number().int().nonnegative()".to_owned(),
        ArgKind::Option | ArgKind::Positional => match arg.value.ty {
            ValueType::Bool => "z.boolean()".to_owned(),
            ValueType::Integer => "z.number().int()".to_owned(),
            ValueType::BigInteger => {
                "z.union([z.number().int(), z.string().regex(/^-?\\d+$/)])".to_owned()
            }
            ValueType::Float => "z.number()".to_owned(),
            ValueType::Unknown | ValueType::String | ValueType::OsString | ValueType::Path => {
                "z.string()".to_owned()
            }
        },
    }
}

fn enum_or_string_type(arg: &ArgSpec) -> String {
    if arg.possible_values.is_empty() {
        match arg.value.ty {
            ValueType::Bool => "boolean".to_owned(),
            ValueType::Integer | ValueType::Float => "number".to_owned(),
            ValueType::BigInteger => "string | number".to_owned(),
            ValueType::Unknown | ValueType::String | ValueType::OsString | ValueType::Path => {
                "string".to_owned()
            }
        }
    } else {
        arg.possible_values
            .iter()
            .map(|value| quote_double(&value.name))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

fn property_name(arg: &ArgSpec) -> String {
    safe_identifier(&camel_case(&arg.id), is_reserved)
}

fn camel_case(input: &str) -> String {
    let mut output = String::new();

    for (index, word) in words(input).iter().enumerate() {
        if index == 0 {
            output.push_str(&word.to_ascii_lowercase());
        } else {
            output.push_str(&capitalize(word));
        }
    }

    if output.is_empty() {
        "value".to_owned()
    } else {
        output
    }
}

fn is_reserved(identifier: &str) -> bool {
    RESERVED.contains(&identifier)
}

const RESERVED: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "of",
    "opaque",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

fn doc_comment(input: &str) -> String {
    collapse_lines(input).replace("*/", "* /")
}

fn kebab_file_stem(input: &str) -> String {
    lower_join(input, "-")
}

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, Command, value_parser};

    use crate::{Flow, generate};

    #[test]
    fn generates_flow_command_builders() {
        let mut cmd = Command::new("demo-tool")
            .arg(
                Arg::new("config")
                    .long("config")
                    .help("Path to config")
                    .action(ArgAction::Set),
            )
            .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
            .arg(
                Arg::new("dry-run")
                    .long("dry-run")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("jobs")
                    .long("jobs")
                    .value_parser(value_parser!(u16))
                    .action(ArgAction::Set),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("// @flow strict"));
        assert!(output.contains("export type DemoToolArgs = {|"));
        assert!(output.contains("+config?: string,"));
        assert!(output.contains("+mode?: \"fast\" | \"slow\","));
        assert!(output.contains("+jobs?: number,"));
        assert!(output.contains("for (let index = 0; index < (args.verbose ?? 0); index += 1)"));
        assert!(output.contains("export function buildRunCommand(args: RunArgs): Array<string>"));
        assert!(output.contains("argv.push(\"run\");"));
        assert!(output.contains("pushValues(argv, requireValue(\"target\", args.target));"));
    }

    #[test]
    fn generates_zod_schemas_and_validating_builders() {
        let mut cmd = Command::new("demo-tool")
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("jobs")
                    .long("jobs")
                    .value_parser(value_parser!(u16))
                    .action(ArgAction::Set),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod(), &mut cmd, "demo-tool", &mut output)
            .expect("zod flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("import { z } from \"zod\";"));
        assert!(output.contains("type ZodSchema<T> = {|"));
        assert!(output.contains("  +safeParse: (input: mixed) => ZodSafeParseResult<T>,\n|};"));
        assert!(!output.contains("||};"));
        assert!(output.contains(
            "export const DemoToolArgsSchema: ZodSchema<DemoToolArgs> = z.strictObject({"
        ));
        assert!(output.contains("mode: z.enum([\"fast\", \"slow\"]).optional(),"));
        assert!(output.contains("jobs: z.number().int().optional(),"));
        assert!(output.contains("export type DemoToolArgs = {|"));
        assert!(output.contains("const parsed: RunArgs = RunArgsSchema.parse(args);"));
        assert!(output.contains("pushValues(argv, requireValue(\"target\", parsed.target));"));
    }

    #[test]
    fn sanitizes_reserved_words_used_as_arg_ids() {
        let mut cmd = Command::new("demo-tool")
            .arg(Arg::new("interface").long("interface"))
            .arg(Arg::new("private").long("private"))
            .arg(Arg::new("let").long("let"));

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("+interface_?: string,"));
        assert!(output.contains("+private_?: string,"));
        assert!(output.contains("+let_?: string,"));
        assert!(output.contains("args.interface_"));
        assert!(output.contains("args.private_"));
        assert!(output.contains("args.let_"));
    }

    #[test]
    fn generates_zod_schemas_without_validating_builders() {
        let mut cmd = Command::new("demo-tool")
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Set),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(
            Flow::new().zod_schemas(),
            &mut cmd,
            "demo-tool",
            &mut output,
        )
        .expect("zod schema-only flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("import { z } from \"zod\";"));
        assert!(output.contains(
            "export const DemoToolArgsSchema: ZodSchema<DemoToolArgs> = z.strictObject({"
        ));
        assert!(!output.contains(".parse(args)"));
        assert!(output.contains("pushValues(argv, requireValue(\"target\", args.target));"));
    }

    #[test]
    fn zod_array_schema_enforces_both_arity_bounds() {
        let mut cmd = Command::new("demo-tool").arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .action(ArgAction::Set),
        );

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod(), &mut cmd, "demo-tool", &mut output)
            .expect("zod flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("pair: z.array(z.string()).min(2).max(2).optional(),"),
            "expected min(2).max(2) on fixed-arity array, got:\n{output}"
        );
    }

    #[test]
    fn array_types_render_union_scalars() {
        let mut cmd = Command::new("demo-tool")
            .arg(
                Arg::new("range")
                    .long("range")
                    .num_args(2)
                    .value_parser(value_parser!(u64))
                    .action(ArgAction::Append),
            )
            .arg(
                Arg::new("kind")
                    .long("kind")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Append),
            )
            .arg(
                Arg::new("ids")
                    .long("ids")
                    .value_parser(value_parser!(u64))
                    .action(ArgAction::Append),
            );

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+range?: $ReadOnlyArray<$ReadOnlyArray<string | number>>,"),
            "expected grouped array of `string | number`, got:\n{output}"
        );
        assert!(
            output.contains("+kind?: $ReadOnlyArray<\"fast\" | \"slow\">,"),
            "expected flat array of enum literals, got:\n{output}"
        );
        assert!(
            output.contains("+ids?: $ReadOnlyArray<string | number>,"),
            "expected flat array of `string | number`, got:\n{output}"
        );
    }

    #[test]
    fn grouped_repeated_option_emits_nested_array_and_dispatches_correctly() {
        let mut cmd = Command::new("demo-tool").arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .action(ArgAction::Append),
        );

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+pair?: $ReadOnlyArray<$ReadOnlyArray<string>>,"),
            "expected nested-array type for grouped repeated option, got:\n{output}"
        );
        assert!(
            output.contains("pushGroupedRepeatedOption(argv, \"--pair\", args.pair);"),
            "expected pushGroupedRepeatedOption dispatch, got:\n{output}"
        );
    }

    #[test]
    fn variadic_positionals_emit_required_arrays_in_order() {
        let mut cmd = Command::new("demo-tool").subcommand(
            Command::new("copy")
                .arg(Arg::new("source").required(true))
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_parser(["fast", "safe"])
                        .action(ArgAction::Set),
                )
                .arg(Arg::new("paths").num_args(1..).required(true)),
        );

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+paths: $ReadOnlyArray<string>,"),
            "expected variadic positional array type, got:\n{output}"
        );
        assert!(
            output.contains("pushValues(argv, requireValue(\"paths\", args.paths));"),
            "expected variadic positional builder, got:\n{output}"
        );
        let source_index = output
            .find("pushValues(argv, requireValue(\"source\", args.source));")
            .expect("source positional builder");
        let mode_index = output
            .find("if (args.mode !== undefined)")
            .expect("mode option builder");
        let paths_index = output
            .find("pushValues(argv, requireValue(\"paths\", args.paths));")
            .expect("variadic positional builder");
        assert!(source_index < mode_index);
        assert!(mode_index < paths_index);
    }

    #[test]
    fn node_runtime_generates_child_process_helpers() {
        let mut cmd = Command::new("demo-tool")
            .arg(Arg::new("config").long("config").action(ArgAction::Set))
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().node(), &mut cmd, "demo-tool", &mut output)
            .expect("node flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("from \"node:child_process\";"));
        assert!(output.contains("const PROGRAM: string = \"demo-tool\";"));
        assert!(output.contains("export type CliCommand"));
        assert!(output.contains("export function createRunCommand(args: RunArgs"));
        assert!(output.contains("args: buildRunCommand(args),"));
        assert!(
            output.contains(
                "export function runRunCommand(args: RunArgs, options: CliRunOptions = {})"
            )
        );
        assert!(output.contains(
            "export function spawnRunCommand(args: RunArgs, options: CliSpawnOptions = {})"
        ));
    }

    #[test]
    fn rejects_colliding_subcommand_prefixes() {
        let mut cmd = Command::new("demo-tool")
            .subcommand(Command::new("index-foo"))
            .subcommand(Command::new("index").subcommand(Command::new("foo")));

        let mut output = Vec::<u8>::new();
        let err = generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect_err("collision");

        let message = err.to_string();
        assert!(message.contains("IndexFoo"));
        assert!(message.contains("index-foo") && message.contains("index, foo"));
    }

    #[test]
    fn wide_integer_emits_string_or_number_to_preserve_precision() {
        let mut cmd = Command::new("demo-tool")
            .arg(
                Arg::new("epoch_ns")
                    .long("epoch-ns")
                    .value_parser(value_parser!(u64))
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("threads")
                    .long("threads")
                    .value_parser(value_parser!(u32))
                    .action(ArgAction::Set),
            );

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &mut cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+epochNs?: string | number,"),
            "expected u64 to render as `string | number`, got:\n{output}"
        );
        assert!(
            output.contains("+threads?: number,"),
            "expected u32 to stay `number`, got:\n{output}"
        );
    }

    #[test]
    fn zod_array_schema_skips_max_for_repeated_options() {
        let mut cmd =
            Command::new("demo-tool").arg(Arg::new("tag").long("tag").action(ArgAction::Append));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod(), &mut cmd, "demo-tool", &mut output)
            .expect("zod flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            !output.contains(".max("),
            "expected no .max() on repeated option, got:\n{output}"
        );
    }

    #[test]
    fn emits_output_contracts_when_enabled() {
        let spec = crate::CliSpec {
            bin_name: "demo".to_owned(),
            root: crate::CommandSpec {
                name: "demo".to_owned(),
                display_name: None,
                about: None,
                long_about: None,
                args: Vec::new(),
                subcommands: Vec::new(),
            },
            outputs: vec![crate::OutputSpec {
                command_path: vec!["watch".to_owned()],
                encoding: crate::OutputEncoding::JsonLines,
                mode: crate::OutputMode::Streaming,
                type_name: "WatchEvent".to_owned(),
                schema: Some(crate::OutputSchema::JsonSchema(
                    "{\"type\":\"object\"}".to_owned(),
                )),
            }],
        };

        let mut output = Vec::<u8>::new();
        crate::Generator::generate(&Flow::new().output_contracts(), &spec, &mut output)
            .expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("export type OutputContract = {|"));
        assert!(output.contains("encoding: \"json-lines\""));
        assert!(output.contains("export function parseOutput("));
        assert!(output.contains("if (contract.mode === \"interactive\")"));
    }
}
