// Copyright (c) Meta Platforms, Inc. and affiliates.

//! Flow-annotated JavaScript code generator: emits argv builders, optional
//! Zod schemas, and optional Node `child_process` helpers, with the same
//! shape as the TypeScript backend.
//!
//! `OutputEncoding::Json` causes the generated `parseOutput` helper to
//! deserialize stdout via the runtime's built-in `JSON.parse`, mirroring the
//! TypeScript and Python generators. The Rust and Kotlin generators leave the
//! payload as a raw string instead, since those ecosystems require the caller
//! to choose a deserialization library and target type.

use std::io::Write;
use std::io::{self};

use crate::codegen::capitalize;
use crate::codegen::collapse_lines;
use crate::codegen::combined_args;
use crate::codegen::command_type_prefix;
use crate::codegen::ensure_unique_command_prefixes;
use crate::codegen::inherited_globals;
use crate::codegen::is_grouped_repeated;
use crate::codegen::is_required;
use crate::codegen::lower_join;
use crate::codegen::option_token;
use crate::codegen::output_encoding;
use crate::codegen::output_mode;
use crate::codegen::output_schema;
use crate::codegen::quote_double;
use crate::codegen::safe_identifier;
use crate::codegen::words;
use crate::generate::Generator;
use crate::generate::OutputContractGeneration;
use crate::model::ArgKind;
use crate::model::ArgSpec;
use crate::model::CliSpec;
use crate::model::CommandSpec;
use crate::model::ValueType;

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

    output.push_str(&format!(
        "/**\n * @{}\n * @flow strict\n */\n\n",
        "generated"
    ));
    output.push_str("/* eslint-disable */\n");
    output.push_str("// Generated by clap_types. Do not edit by hand.\n");
    output.push_str("// @ts-nocheck\n");
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
  | {| +success: false, +error: unknown |};

type ZodSchema<T> = {|
  +parse: (input: unknown) => T,
  +safeParse: (input: unknown) => ZodSafeParseResult<T>,
|};

"#,
        );
    }
    output.push_str("type ArgValue = string | number | boolean;\n");
    output.push_str("type ArgValueInput = ArgValue | ReadonlyArray<ArgValue>;\n\n");
    render_helpers(output);
    if runtime.is_node() {
        render_node_helpers(output);
    }
    if options.output_contracts.is_enabled() {
        render_output_types(spec, output);
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
  groups: ReadonlyArray<ReadonlyArray<ArgValue>>,
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

fn render_output_types(spec: &CliSpec, output: &mut String) {
    for contract in &spec.outputs {
        let Some(crate::model::OutputSchema::JsonSchema(json_str)) = &contract.schema else {
            continue;
        };
        let Ok(schema) = serde_json::from_str::<serde_json::Value>(json_str) else {
            continue;
        };
        let defs = schema
            .get("$defs")
            .or_else(|| schema.get("definitions"))
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        render_type_from_schema(&contract.type_name, &schema, &defs, output);

        if let Some(defs_obj) = defs.as_object() {
            for (def_name, def_schema) in defs_obj {
                render_type_from_schema(def_name, def_schema, &defs, output);
            }
        }
    }
}

fn render_type_from_schema(
    name: &str,
    schema: &serde_json::Value,
    defs: &serde_json::Value,
    output: &mut String,
) {
    let Some(props) = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .filter(|p| !p.is_empty())
    else {
        let fallback = json_schema_type_to_flow(schema, defs);
        if fallback != "mixed" {
            output.push_str(&format!("export type {name} = {fallback};\n\n"));
        } else {
            output.push_str(&format!("export type {name} = mixed;\n\n"));
        }
        return;
    };

    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    output.push_str(&format!("export type {name} = {{|\n"));
    for (prop_name, prop_schema) in props {
        let flow_type = json_schema_type_to_flow(prop_schema, defs);
        let optional = if required.contains(&prop_name.as_str()) {
            ""
        } else {
            "?"
        };
        output.push_str(&format!("  +{prop_name}{optional}: {flow_type},\n"));
    }
    output.push_str("|};\n\n");
}

fn json_schema_type_to_flow(schema: &serde_json::Value, defs: &serde_json::Value) -> String {
    if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
        if let Some(type_name) = ref_path.rsplit('/').next() {
            if let Some(def_schema) = defs.get(type_name) {
                return json_schema_type_to_flow(def_schema, defs);
            }
        }
        return "mixed".to_owned();
    }

    if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
        let literals: Vec<String> = enum_values
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| format!("\"{s}\""))
            .collect();
        if !literals.is_empty() {
            return literals.join(" | ");
        }
    }

    if let Some(one_of) = schema.get("oneOf").and_then(|o| o.as_array()) {
        let variants: Vec<String> = one_of
            .iter()
            .map(|v| {
                if let Some(c) = v.get("const").and_then(|c| c.as_str()) {
                    return format!("\"{c}\"");
                }
                json_schema_type_to_flow(v, defs)
            })
            .collect();
        if !variants.is_empty() {
            return variants.join(" | ");
        }
    }

    if let Some(any_of) = schema.get("anyOf").and_then(|a| a.as_array()) {
        let non_null: Vec<&serde_json::Value> = any_of
            .iter()
            .filter(|v| v.get("type").and_then(|t| t.as_str()) != Some("null"))
            .collect();
        if non_null.len() == 1 {
            return json_schema_type_to_flow(non_null[0], defs);
        }
        let variants: Vec<String> = non_null
            .iter()
            .map(|v| json_schema_type_to_flow(v, defs))
            .collect();
        if !variants.is_empty() {
            return variants.join(" | ");
        }
    }

    if let Some(type_array) = schema.get("type").and_then(|t| t.as_array()) {
        let non_null: Vec<&str> = type_array
            .iter()
            .filter_map(|t| t.as_str())
            .filter(|t| *t != "null")
            .collect();
        if non_null.len() == 1 {
            let mut single = serde_json::Map::new();
            single.insert(
                "type".to_owned(),
                serde_json::Value::String(non_null[0].to_owned()),
            );
            return json_schema_type_to_flow(&serde_json::Value::Object(single), defs);
        }
        if !non_null.is_empty() {
            let variants: Vec<String> = non_null
                .iter()
                .map(|t| {
                    let mut s = serde_json::Map::new();
                    s.insert(
                        "type".to_owned(),
                        serde_json::Value::String((*t).to_owned()),
                    );
                    json_schema_type_to_flow(&serde_json::Value::Object(s), defs)
                })
                .collect();
            return variants.join(" | ");
        }
    }

    match schema.get("type").and_then(|t| t.as_str()) {
        Some("integer") | Some("number") => "number".to_owned(),
        Some("boolean") => "boolean".to_owned(),
        Some("string") => "string".to_owned(),
        Some("null") => "null".to_owned(),
        Some("array") => {
            let item_type = schema
                .get("items")
                .map(|items| json_schema_type_to_flow(items, defs))
                .unwrap_or_else(|| "mixed".to_owned());
            format!("Array<{item_type}>")
        }
        _ => "mixed".to_owned(),
    }
}

fn render_output_contracts(spec: &CliSpec, output: &mut String) {
    output.push_str(
        r#"export type OutputEncoding = "json" | "json-lines" | "text";
export type OutputMode = "buffered" | "streaming" | "interactive";

export type OutputContract = {|
  +commandPath: ReadonlyArray<string>,
  +encoding: OutputEncoding,
  +mode: OutputMode,
  +typeName: string,
  +schema?: string,
|};

export type ParsedOutput = unknown | ReadonlyArray<unknown> | string;

"#,
    );
    output.push_str("export const OUTPUT_CONTRACTS: ReadonlyArray<OutputContract> = [\n");
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
        format!("ReadonlyArray<ReadonlyArray<{scalar}>>")
    } else if arg.value.repeated || arg.value.arity.allows_multiple() {
        format!("ReadonlyArray<{scalar}>")
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
    use clap::Arg;
    use clap::ArgAction;
    use clap::Command;
    use clap::value_parser;

    use super::json_schema_type_to_flow;
    use super::render_type_from_schema;
    use crate::Flow;
    use crate::generate;

    #[test]
    fn generates_flow_command_builders() {
        let cmd = Command::new("demo-tool")
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
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("@flow strict"));
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
        let cmd = Command::new("demo-tool")
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
        generate(Flow::new().zod(), &cmd, "demo-tool", &mut output)
            .expect("zod flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(output.contains("import { z } from \"zod\";"));
        assert!(output.contains("type ZodSchema<T> = {|"));
        assert!(output.contains("  +safeParse: (input: unknown) => ZodSafeParseResult<T>,\n|};"));
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
        let cmd = Command::new("demo-tool")
            .arg(Arg::new("interface").long("interface"))
            .arg(Arg::new("private").long("private"))
            .arg(Arg::new("let").long("let"));

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
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
        let cmd = Command::new("demo-tool")
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Set),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod_schemas(), &cmd, "demo-tool", &mut output)
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
        let cmd = Command::new("demo-tool").arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .action(ArgAction::Set),
        );

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod(), &cmd, "demo-tool", &mut output)
            .expect("zod flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("pair: z.array(z.string()).min(2).max(2).optional(),"),
            "expected min(2).max(2) on fixed-arity array, got:\n{output}"
        );
    }

    #[test]
    fn array_types_render_union_scalars() {
        let cmd = Command::new("demo-tool")
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
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+range?: ReadonlyArray<ReadonlyArray<string | number>>,"),
            "expected grouped array of `string | number`, got:\n{output}"
        );
        assert!(
            output.contains("+kind?: ReadonlyArray<\"fast\" | \"slow\">,"),
            "expected flat array of enum literals, got:\n{output}"
        );
        assert!(
            output.contains("+ids?: ReadonlyArray<string | number>,"),
            "expected flat array of `string | number`, got:\n{output}"
        );
    }

    #[test]
    fn grouped_repeated_option_emits_nested_array_and_dispatches_correctly() {
        let cmd = Command::new("demo-tool").arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .action(ArgAction::Append),
        );

        let mut output = Vec::<u8>::new();
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+pair?: ReadonlyArray<ReadonlyArray<string>>,"),
            "expected nested-array type for grouped repeated option, got:\n{output}"
        );
        assert!(
            output.contains("pushGroupedRepeatedOption(argv, \"--pair\", args.pair);"),
            "expected pushGroupedRepeatedOption dispatch, got:\n{output}"
        );
    }

    #[test]
    fn variadic_positionals_emit_required_arrays_in_order() {
        let cmd = Command::new("demo-tool").subcommand(
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
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
        let output = String::from_utf8(output).expect("flow is utf-8");

        assert!(
            output.contains("+paths: ReadonlyArray<string>,"),
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
        let cmd = Command::new("demo-tool")
            .arg(Arg::new("config").long("config").action(ArgAction::Set))
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().node(), &cmd, "demo-tool", &mut output)
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
        let cmd = Command::new("demo-tool")
            .subcommand(Command::new("index-foo"))
            .subcommand(Command::new("index").subcommand(Command::new("foo")));

        let mut output = Vec::<u8>::new();
        let err = generate(Flow::new(), &cmd, "demo-tool", &mut output).expect_err("collision");

        let message = err.to_string();
        assert!(message.contains("IndexFoo"));
        assert!(message.contains("index-foo") && message.contains("index, foo"));
    }

    #[test]
    fn wide_integer_emits_string_or_number_to_preserve_precision() {
        let cmd = Command::new("demo-tool")
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
        generate(Flow::new(), &cmd, "demo-tool", &mut output).expect("flow generation works");
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
        let cmd =
            Command::new("demo-tool").arg(Arg::new("tag").long("tag").action(ArgAction::Append));

        let mut output = Vec::<u8>::new();
        generate(Flow::new().zod(), &cmd, "demo-tool", &mut output)
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

    #[test]
    fn json_schema_renders_object_type_with_required_and_optional_fields() {
        let empty = serde_json::json!({});
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "level": {"type": "integer"},
                "message": {"type": "string"},
                "optional_flag": {"type": "boolean"},
            },
            "required": ["level", "message"],
        });

        let mut output = String::new();
        render_type_from_schema("BatteryInfo", &schema, &empty, &mut output);

        assert!(
            output.contains("+level: number,"),
            "expected required number field, got:\n{output}"
        );
        assert!(
            output.contains("+message: string,"),
            "expected required string field, got:\n{output}"
        );
        assert!(
            output.contains("+optional_flag?: boolean,"),
            "expected optional boolean field, got:\n{output}"
        );
    }

    #[test]
    fn json_schema_type_to_flow_handles_basic_types() {
        let empty = serde_json::json!({});

        assert_eq!(
            json_schema_type_to_flow(&serde_json::json!({"type": "integer"}), &empty),
            "number"
        );
        assert_eq!(
            json_schema_type_to_flow(&serde_json::json!({"type": "string"}), &empty),
            "string"
        );
        assert_eq!(
            json_schema_type_to_flow(&serde_json::json!({"type": "boolean"}), &empty),
            "boolean"
        );
        assert_eq!(
            json_schema_type_to_flow(
                &serde_json::json!({"type": "array", "items": {"type": "string"}}),
                &empty
            ),
            "Array<string>"
        );
    }

    #[test]
    fn json_schema_type_to_flow_handles_enum_and_one_of() {
        let empty = serde_json::json!({});

        let enum_schema = serde_json::json!({
            "enum": ["device", "offline", "unauthorized"],
        });
        assert_eq!(
            json_schema_type_to_flow(&enum_schema, &empty),
            "\"device\" | \"offline\" | \"unauthorized\""
        );

        let one_of_schema = serde_json::json!({
            "oneOf": [
                {"type": "string", "const": "fast"},
                {"type": "string", "const": "safe"},
            ],
        });
        assert_eq!(
            json_schema_type_to_flow(&one_of_schema, &empty),
            "\"fast\" | \"safe\""
        );
    }

    #[test]
    fn json_schema_type_to_flow_handles_nullable_types() {
        let empty = serde_json::json!({});

        let nullable = serde_json::json!({
            "type": ["string", "null"],
        });
        assert_eq!(json_schema_type_to_flow(&nullable, &empty), "string");

        let any_of_nullable = serde_json::json!({
            "anyOf": [
                {"type": "string"},
                {"type": "null"},
            ],
        });
        assert_eq!(json_schema_type_to_flow(&any_of_nullable, &empty), "string");
    }
}
