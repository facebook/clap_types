// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::collections::BTreeSet;
use std::io::Write;
use std::io::{self};
use std::path::Path;
use std::path::PathBuf;

use crate::codegen::collapse_lines;
use crate::codegen::combined_args;
use crate::codegen::command_pieces;
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
use crate::codegen::pascal_case;
use crate::codegen::quote_double;
use crate::codegen::safe_identifier;
use crate::generate::GeneratedFile;
use crate::generate::Generator;
use crate::generate::OutputContractGeneration;
use crate::model::ArgKind;
use crate::model::ArgSpec;
use crate::model::CliSpec;
use crate::model::CommandSpec;
use crate::model::ValueType;

/// Options for the Python backend.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PythonOptions {
    /// Generated module file stem, without `.py`.
    pub module_name: Option<String>,
    /// Optional namespace class that exposes generated functions as static methods.
    pub namespace: Option<String>,
    /// Output-contract metadata and parser helpers.
    pub output_contracts: OutputContractGeneration,
}

/// Generate dependency-free Python 3.10+ command builders and subprocess helpers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Python {
    options: PythonOptions,
}

/// Generate a Python package with one command module per clap command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PythonPackage {
    options: PythonOptions,
}

impl Python {
    /// Create a Python generator with default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Python generator from explicit options.
    #[must_use]
    pub fn with_options(options: PythonOptions) -> Self {
        Self { options }
    }

    /// Set the generated module name, without `.py`.
    #[must_use]
    pub fn module_name(mut self, module_name: impl Into<String>) -> Self {
        self.options.module_name = Some(module_name.into());
        self
    }

    /// Set an optional namespace class for generated functions.
    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.options.namespace = Some(namespace.into());
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

    /// Generate a package layout instead of a single `.py` module.
    #[must_use]
    pub fn package(self) -> PythonPackage {
        PythonPackage {
            options: self.options,
        }
    }
}

impl Generator for Python {
    fn file_name(&self, bin_name: &str) -> String {
        let stem = self
            .options
            .module_name
            .as_deref()
            .map(snake_case)
            .unwrap_or_else(|| snake_case(bin_name));
        format!("{stem}.py")
    }

    fn generate(&self, spec: &CliSpec, buf: &mut dyn Write) -> io::Result<()> {
        ensure_unique_command_prefixes(spec)?;
        let mut output = String::new();
        render_module(spec, &self.options, &mut output);
        buf.write_all(output.as_bytes())
    }
}

impl PythonPackage {
    /// Create a Python package generator with default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Python package generator from explicit options.
    #[must_use]
    pub fn with_options(options: PythonOptions) -> Self {
        Self { options }
    }

    /// Set the generated package name.
    #[must_use]
    pub fn module_name(mut self, module_name: impl Into<String>) -> Self {
        self.options.module_name = Some(module_name.into());
        self
    }

    /// Set an optional namespace class exported from the package root.
    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.options.namespace = Some(namespace.into());
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

impl Generator for PythonPackage {
    fn file_name(&self, bin_name: &str) -> String {
        self.package_name(bin_name)
    }

    fn generate(&self, _spec: &CliSpec, _buf: &mut dyn Write) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PythonPackage emits multiple files; use generate_to to write the package",
        ))
    }

    fn generate_files(&self, spec: &CliSpec) -> io::Result<Vec<GeneratedFile>> {
        ensure_unique_command_prefixes(spec)?;
        Ok(render_package(spec, &self.options))
    }
}

impl PythonPackage {
    fn package_name(&self, bin_name: &str) -> String {
        self.options
            .module_name
            .as_deref()
            .map(package_name)
            .unwrap_or_else(|| package_name(bin_name))
    }
}

#[derive(Debug, Clone)]
struct PackageExport {
    import_path: String,
    args_alias: String,
    build_alias: String,
    command_alias: String,
}

fn render_package(spec: &CliSpec, options: &PythonOptions) -> Vec<GeneratedFile> {
    let package = options
        .module_name
        .as_deref()
        .map(package_name)
        .unwrap_or_else(|| package_name(&spec.bin_name));
    let package_root = PathBuf::from(&package);
    let mut files = Vec::<GeneratedFile>::new();
    let mut exports = Vec::<PackageExport>::new();
    let mut package_dirs = BTreeSet::<PathBuf>::new();

    files.push(GeneratedFile::text(
        package_root.join("_runtime.py"),
        render_package_runtime(spec, options),
    ));

    let mut path = Vec::<String>::new();
    let inherited: Vec<&ArgSpec> = Vec::new();
    render_package_command_tree(
        &package_root,
        &spec.root,
        &mut path,
        &inherited,
        &mut files,
        &mut exports,
        &mut package_dirs,
    );

    for dir in package_dirs {
        files.push(GeneratedFile::text(
            dir.join("__init__.py"),
            "# Generated by clap_types. Do not edit by hand.\n",
        ));
    }

    files.push(GeneratedFile::text(
        package_root.join("__init__.py"),
        render_package_init(options, &exports),
    ));

    files
}

fn render_package_runtime(spec: &CliSpec, options: &PythonOptions) -> String {
    let mut output = String::new();
    output.push_str("# Generated by clap_types. Do not edit by hand.\n");
    output.push_str("from __future__ import annotations\n\n");
    output.push_str("from collections.abc import Mapping, Sequence\n");
    output.push_str("from dataclasses import dataclass\n");
    if options.output_contracts.is_enabled() {
        output.push_str("import json\n");
    }
    output.push_str("from os import PathLike\n");
    output.push_str("import subprocess\n");
    output.push_str("from typing import Any, TypeAlias, TypeGuard\n\n");
    output.push_str(&format!(
        "PROGRAM: str = {}\n\n",
        quote_double(&spec.bin_name)
    ));
    render_helpers(&mut output);
    if options.output_contracts.is_enabled() {
        render_output_contracts(spec, &mut output);
    }
    output.push_str("__all__ = [\n");
    output.push_str("    \"ArgScalar\",\n");
    output.push_str("    \"ArgValue\",\n");
    output.push_str("    \"CommandInvocation\",\n");
    if options.output_contracts.is_enabled() {
        output.push_str("    \"OutputContract\",\n");
        output.push_str("    \"OUTPUT_CONTRACTS\",\n");
        output.push_str("    \"parse_output\",\n");
    }
    output.push_str("    \"PathValue\",\n");
    output.push_str("    \"PROGRAM\",\n");
    output.push_str("]\n");
    output
}

fn render_package_command_tree(
    package_root: &Path,
    command: &CommandSpec,
    path: &mut Vec<String>,
    inherited: &[&ArgSpec],
    files: &mut Vec<GeneratedFile>,
    exports: &mut Vec<PackageExport>,
    package_dirs: &mut BTreeSet<PathBuf>,
) {
    let rel_path = command_module_path(package_root, command, path);
    add_package_dirs(package_root, &rel_path, package_dirs);
    let import_path = import_path(package_root, &rel_path);
    let type_prefix = command_type_prefix(command, path);
    let fn_prefix = command_function_prefix(command, path);

    files.push(GeneratedFile::text(
        rel_path,
        render_package_command_module(command, path, inherited),
    ));
    exports.push(PackageExport {
        import_path,
        args_alias: format!("{type_prefix}Args"),
        build_alias: format!("build_{fn_prefix}_args"),
        command_alias: format!("{fn_prefix}_command"),
    });

    let next_inherited = inherited_globals(inherited, command);
    for subcommand in &command.subcommands {
        path.push(subcommand.name.clone());
        render_package_command_tree(
            package_root,
            subcommand,
            path,
            &next_inherited,
            files,
            exports,
            package_dirs,
        );
        path.pop();
    }
}

fn render_package_command_module(
    command: &CommandSpec,
    path: &[String],
    inherited: &[&ArgSpec],
) -> String {
    let mut output = String::new();
    let all_args = combined_args(inherited, command);
    let has_required = all_args.iter().copied().any(is_required);

    output.push_str("# Generated by clap_types. Do not edit by hand.\n");
    output.push_str("from __future__ import annotations\n\n");
    output.push_str("from dataclasses import dataclass\n");
    output.push_str("from os import PathLike\n");
    if all_args.iter().any(|arg| !arg.possible_values.is_empty()) {
        output.push_str("from typing import Literal\n");
    }
    output.push('\n');
    render_package_runtime_imports(&all_args, command, path, &mut output);
    output.push_str("\n\n");

    render_dataclass(command, "Args", &all_args, &mut output);
    output.push_str("\n\n");
    render_build_function(
        command,
        path,
        inherited,
        "Args",
        "build_args",
        has_required,
        &mut output,
    );
    output.push_str("\n\n");
    render_command_function("Args", "build_args", "command", has_required, &mut output);
    output.push_str("\n\n");
    output.push_str("__all__ = [\"Args\", \"build_args\", \"command\"]\n");
    output
}

fn render_package_runtime_imports(
    args: &[&ArgSpec],
    command: &CommandSpec,
    path: &[String],
    output: &mut String,
) {
    let mut imports = vec!["CommandInvocation", "PROGRAM"];
    if args
        .iter()
        .any(|arg| python_type(arg).contains("PathValue"))
    {
        imports.push("PathValue");
    }
    if args
        .iter()
        .any(|arg| matches!(arg.kind, ArgKind::Positional))
    {
        imports.push("_push_values");
    }
    if args.iter().any(|arg| {
        matches!(arg.kind, ArgKind::Option) && !arg.value.repeated && !is_grouped_repeated(arg)
    }) {
        imports.push("_push_option");
    }
    if args.iter().any(|arg| {
        matches!(arg.kind, ArgKind::Option) && arg.value.repeated && !is_grouped_repeated(arg)
    }) {
        imports.push("_push_repeated_option");
    }
    if args
        .iter()
        .any(|arg| matches!(arg.kind, ArgKind::Option) && is_grouped_repeated(arg))
    {
        imports.push("_push_grouped_repeated_option");
    }

    output.push_str(&format!(
        "from {}_runtime import (\n",
        runtime_relative_prefix(command, path)
    ));
    for import in imports {
        output.push_str(&format!("    {import},\n"));
    }
    output.push_str(")\n");
}

fn render_package_init(options: &PythonOptions, exports: &[PackageExport]) -> String {
    let mut names = vec!["CommandInvocation".to_owned()];
    let mut functions = Vec::<String>::new();
    let mut output = String::new();

    output.push_str("# Generated by clap_types. Do not edit by hand.\n");
    output.push_str("from __future__ import annotations\n\n");
    if options.output_contracts.is_enabled() {
        output.push_str(
            "from ._runtime import CommandInvocation, OutputContract, OUTPUT_CONTRACTS, parse_output\n",
        );
        names.push("OutputContract".to_owned());
        names.push("OUTPUT_CONTRACTS".to_owned());
        names.push("parse_output".to_owned());
    } else {
        output.push_str("from ._runtime import CommandInvocation\n");
    }

    for export in exports {
        output.push_str(&format!(
            "from .{} import (\n    Args as {},\n    build_args as {},\n    command as {},\n)\n",
            export.import_path, export.args_alias, export.build_alias, export.command_alias
        ));
        names.push(export.args_alias.clone());
        names.push(export.build_alias.clone());
        names.push(export.command_alias.clone());
        functions.push(export.build_alias.clone());
        functions.push(export.command_alias.clone());
    }
    output.push_str("\n\n");

    if let Some(namespace) = options.namespace.as_deref() {
        let namespace = safe_pascal_identifier(namespace);
        render_namespace(&namespace, &functions, &mut output);
        names.push(namespace);
    }

    render_all(names, &mut output);
    output
}

fn command_module_path(package_root: &Path, command: &CommandSpec, path: &[String]) -> PathBuf {
    let mut rel_path = package_root.to_path_buf();
    if path.is_empty() {
        rel_path.push("_root.py");
        return rel_path;
    }

    let stems = path
        .iter()
        .map(|piece| module_stem(piece))
        .collect::<Vec<_>>();
    if command.subcommands.is_empty() {
        for stem in &stems[..stems.len() - 1] {
            rel_path.push(stem);
        }
        rel_path.push(format!("{}.py", stems.last().expect("non-empty path")));
    } else {
        for stem in stems {
            rel_path.push(stem);
        }
        rel_path.push("_root.py");
    }

    rel_path
}

fn add_package_dirs(package_root: &Path, file_path: &Path, package_dirs: &mut BTreeSet<PathBuf>) {
    let mut current = file_path.parent();
    while let Some(dir) = current {
        if dir == package_root {
            break;
        }
        package_dirs.insert(dir.to_path_buf());
        current = dir.parent();
    }
}

fn import_path(package_root: &Path, file_path: &Path) -> String {
    let without_package = file_path
        .strip_prefix(package_root)
        .expect("command module lives under package root");
    let mut components = without_package
        .iter()
        .map(|component| component.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if let Some(last) = components.last_mut() {
        if let Some(stripped) = last.strip_suffix(".py") {
            *last = stripped.to_owned();
        }
    }
    components.join(".")
}

fn runtime_relative_prefix(command: &CommandSpec, path: &[String]) -> String {
    let parent_depth = if path.is_empty() {
        0
    } else if command.subcommands.is_empty() {
        path.len().saturating_sub(1)
    } else {
        path.len()
    };
    ".".repeat(parent_depth + 1)
}

fn render_module(spec: &CliSpec, options: &PythonOptions, output: &mut String) {
    let mut exports = Vec::<String>::new();
    let mut functions = Vec::<String>::new();

    output.push_str("# Generated by clap_types. Do not edit by hand.\n");
    output.push_str("from __future__ import annotations\n\n");
    output.push_str("from collections.abc import Mapping, Sequence\n");
    output.push_str("from dataclasses import dataclass\n");
    if options.output_contracts.is_enabled() {
        output.push_str("import json\n");
    }
    output.push_str("from os import PathLike\n");
    output.push_str("import subprocess\n");
    output.push_str("from typing import Any, Literal, TypeAlias, TypeGuard\n\n");
    output.push_str(&format!(
        "PROGRAM: str = {}\n\n",
        quote_double(&spec.bin_name)
    ));
    render_helpers(output);

    exports.push("CommandInvocation".to_owned());
    if options.output_contracts.is_enabled() {
        render_output_contracts(spec, output);
        exports.push("OutputContract".to_owned());
        exports.push("OUTPUT_CONTRACTS".to_owned());
        exports.push("parse_output".to_owned());
    }

    let mut path = Vec::<String>::new();
    let inherited: Vec<&ArgSpec> = Vec::new();
    render_command_tree(
        &spec.root,
        &mut path,
        &inherited,
        output,
        &mut exports,
        &mut functions,
    );

    if let Some(namespace) = options.namespace.as_deref() {
        let namespace = safe_pascal_identifier(namespace);
        render_namespace(&namespace, &functions, output);
        exports.push(namespace);
    }

    render_all(exports, output);
}

fn render_helpers(output: &mut String) {
    output.push_str(
        r#"ArgScalar: TypeAlias = str | int | float | bool | PathLike[str]
ArgValue: TypeAlias = ArgScalar | Sequence[ArgScalar]
PathValue: TypeAlias = str | PathLike[str]


@dataclass(frozen=True)
class CommandInvocation:
    """A concrete CLI invocation built from generated arguments."""

    program: str
    args: tuple[str, ...]

    def argv(self) -> list[str]:
        """Return the full argv list including the executable."""
        return [self.program, *self.args]

    def run(
        self,
        *,
        check: bool = False,
        capture_output: bool = False,
        cwd: str | PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        input: str | None = None,
        timeout: float | None = None,
        **kwargs: Any,
    ) -> subprocess.CompletedProcess[str]:
        """Run the invocation with `subprocess.run` using text mode."""
        return subprocess.run(
            self.argv(),
            check=check,
            capture_output=capture_output,
            text=True,
            cwd=cwd,
            env=env,
            input=input,
            timeout=timeout,
            **kwargs,
        )


def _stringify(value: ArgScalar) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def _is_sequence(value: ArgValue) -> TypeGuard[Sequence[ArgScalar]]:
    return isinstance(value, Sequence) and not isinstance(
        value,
        (str, bytes, bytearray),
    )


def _push_values(argv: list[str], value: ArgValue) -> None:
    if _is_sequence(value):
        for item in value:
            argv.append(_stringify(item))
        return

    argv.append(_stringify(value))


def _push_option(argv: list[str], option: str, value: ArgValue) -> None:
    argv.append(option)
    _push_values(argv, value)


def _push_repeated_option(argv: list[str], option: str, value: ArgValue) -> None:
    if _is_sequence(value):
        for item in value:
            _push_option(argv, option, item)
        return

    _push_option(argv, option, value)


def _push_grouped_repeated_option(
    argv: list[str],
    option: str,
    groups: Sequence[Sequence[ArgScalar]],
) -> None:
    for group in groups:
        _push_option(argv, option, list(group))


"#,
    );
}

fn render_output_contracts(spec: &CliSpec, output: &mut String) {
    output.push_str(
        r#"@dataclass(frozen=True)
class OutputContract:
    """Structured output metadata for one command."""

    command_path: tuple[str, ...]
    encoding: str
    mode: str
    type_name: str
    schema: str | None = None


OUTPUT_CONTRACTS: tuple[OutputContract, ...] = (
"#,
    );
    for contract in &spec.outputs {
        output.push_str("    OutputContract(\n");
        output.push_str(&format!(
            "        command_path={},\n",
            python_tuple(&contract.command_path)
        ));
        output.push_str(&format!(
            "        encoding={},\n",
            quote_double(output_encoding(contract.encoding))
        ));
        output.push_str(&format!(
            "        mode={},\n",
            quote_double(output_mode(contract.mode))
        ));
        output.push_str(&format!(
            "        type_name={},\n",
            quote_double(&contract.type_name)
        ));
        if let Some(schema) = contract.schema.as_ref().map(output_schema) {
            output.push_str(&format!("        schema={},\n", quote_double(schema)));
        }
        output.push_str("    ),\n");
    }
    output.push_str(
        r#")


def parse_output(contract: OutputContract, stdout: str) -> object:
    """Parse stdout according to dependency-free framing metadata."""
    if contract.mode == "interactive":
        return stdout
    if contract.encoding == "json":
        return json.loads(stdout)
    if contract.encoding == "json-lines":
        return [json.loads(line) for line in stdout.splitlines() if line]
    if contract.encoding == "text":
        return stdout
    raise ValueError(f"Unsupported output encoding: {contract.encoding}")


"#,
    );
}

fn python_tuple(values: &[String]) -> String {
    match values {
        [] => "()".to_owned(),
        [value] => format!("({},)", quote_double(value)),
        _ => format!(
            "({},)",
            values
                .iter()
                .map(|value| quote_double(value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_command_tree(
    command: &CommandSpec,
    path: &mut Vec<String>,
    inherited: &[&ArgSpec],
    output: &mut String,
    exports: &mut Vec<String>,
    functions: &mut Vec<String>,
) {
    render_command(command, path, inherited, output, exports, functions);

    let next_inherited = inherited_globals(inherited, command);

    for subcommand in &command.subcommands {
        path.push(subcommand.name.clone());
        render_command_tree(
            subcommand,
            path,
            &next_inherited,
            output,
            exports,
            functions,
        );
        path.pop();
    }
}

fn render_command(
    command: &CommandSpec,
    path: &[String],
    inherited: &[&ArgSpec],
    output: &mut String,
    exports: &mut Vec<String>,
    functions: &mut Vec<String>,
) {
    let type_prefix = command_type_prefix(command, path);
    let args_name = format!("{type_prefix}Args");
    let fn_prefix = command_function_prefix(command, path);
    let build_fn = format!("build_{fn_prefix}_args");
    let command_fn = format!("{fn_prefix}_command");
    let all_args = combined_args(inherited, command);
    let has_required = all_args.iter().copied().any(is_required);

    render_dataclass(command, &args_name, &all_args, output);
    output.push_str("\n\n");
    render_build_function(
        command,
        path,
        inherited,
        &args_name,
        &build_fn,
        has_required,
        output,
    );
    output.push_str("\n\n");
    render_command_function(&args_name, &build_fn, &command_fn, has_required, output);
    output.push_str("\n\n");

    exports.push(args_name);
    exports.push(build_fn.clone());
    exports.push(command_fn.clone());
    functions.push(build_fn);
    functions.push(command_fn);
}

fn render_dataclass(
    command: &CommandSpec,
    args_name: &str,
    args: &[&ArgSpec],
    output: &mut String,
) {
    output.push_str("@dataclass(frozen=True, kw_only=True)\n");
    output.push_str(&format!("class {args_name}:\n"));

    if let Some(doc) = command.about.as_deref().or(command.long_about.as_deref()) {
        output.push_str(&format!("    \"\"\"{}\"\"\"\n", docstring(doc)));
    } else {
        output.push_str("    \"\"\"Arguments for this command.\"\"\"\n");
    }

    if args.is_empty() {
        output.push_str("\n    pass\n");
        return;
    }

    for arg in sorted_dataclass_args(args) {
        output.push('\n');
        for comment in field_comments(arg) {
            output.push_str(&format!("    # {comment}\n"));
        }

        let name = property_name(arg);
        let ty = python_type(arg);
        if is_required(arg) {
            output.push_str(&format!("    {name}: {ty}\n"));
        } else {
            output.push_str(&format!("    {name}: {ty} | None = None\n"));
        }
    }
}

fn sorted_dataclass_args<'a>(args: &[&'a ArgSpec]) -> Vec<&'a ArgSpec> {
    let mut sorted = args.to_vec();
    sorted.sort_by_key(|arg| !is_required(arg));
    sorted
}

fn render_build_function(
    command: &CommandSpec,
    path: &[String],
    inherited: &[&ArgSpec],
    args_name: &str,
    build_fn: &str,
    has_required: bool,
    output: &mut String,
) {
    if has_required {
        output.push_str(&format!(
            "def {build_fn}(args: {args_name}) -> tuple[str, ...]:\n"
        ));
    } else {
        output.push_str(&format!(
            "def {build_fn}(args: {args_name} | None = None) -> tuple[str, ...]:\n"
        ));
    }

    output.push_str("    \"\"\"Build argv arguments for this command.\"\"\"\n");

    if has_required {
        output.push_str("    argv: list[str] = []\n");
    } else {
        output.push_str(&format!("    args = args or {args_name}()\n"));
        output.push_str("    argv: list[str] = []\n");
    }

    for &arg in inherited {
        render_arg_builder(arg, output);
    }

    for token in path {
        output.push_str(&format!("    argv.append({})\n", quote_double(token)));
    }

    for arg in &command.args {
        if !inherited.iter().any(|existing| existing.id == arg.id) {
            render_arg_builder(arg, output);
        }
    }

    output.push_str("    return tuple(argv)\n");
}

fn render_command_function(
    args_name: &str,
    build_fn: &str,
    command_fn: &str,
    has_required: bool,
    output: &mut String,
) {
    if has_required {
        output.push_str(&format!(
            "def {command_fn}(\n    args: {args_name},\n    *,\n    program: str | PathLike[str] = PROGRAM,\n) -> CommandInvocation:\n"
        ));
    } else {
        output.push_str(&format!(
            "def {command_fn}(\n    args: {args_name} | None = None,\n    *,\n    program: str | PathLike[str] = PROGRAM,\n) -> CommandInvocation:\n"
        ));
    }

    output.push_str("    \"\"\"Build a subprocess-ready command invocation.\"\"\"\n");
    output.push_str(&format!(
        "    return CommandInvocation(str(program), {build_fn}(args))\n"
    ));
}

fn render_arg_builder(arg: &ArgSpec, output: &mut String) {
    let prop = property_name(arg);
    let option = option_token(arg);
    let accessor = format!("args.{prop}");

    match arg.kind {
        ArgKind::FlagTrue => {
            if let Some(option) = option {
                output.push_str(&format!("    if {accessor} is True:\n"));
                output.push_str(&format!("        argv.append({})\n", quote_double(&option)));
            }
        }
        ArgKind::FlagFalse => {
            if let Some(option) = option {
                output.push_str(&format!("    if {accessor} is False:\n"));
                output.push_str(&format!("        argv.append({})\n", quote_double(&option)));
            }
        }
        ArgKind::Counter => {
            if let Some(option) = option {
                output.push_str(&format!("    for _ in range({accessor} or 0):\n"));
                output.push_str(&format!("        argv.append({})\n", quote_double(&option)));
            }
        }
        ArgKind::Option => {
            if let Some(option) = option {
                let helper = if is_grouped_repeated(arg) {
                    "_push_grouped_repeated_option"
                } else if arg.value.repeated {
                    "_push_repeated_option"
                } else {
                    "_push_option"
                };

                if is_required(arg) {
                    output.push_str(&format!(
                        "    {helper}(argv, {}, {accessor})\n",
                        quote_double(&option)
                    ));
                } else {
                    output.push_str(&format!("    if {accessor} is not None:\n"));
                    output.push_str(&format!(
                        "        {helper}(argv, {}, {accessor})\n",
                        quote_double(&option)
                    ));
                }
            }
        }
        ArgKind::Positional => {
            if is_required(arg) {
                output.push_str(&format!("    _push_values(argv, {accessor})\n"));
            } else {
                output.push_str(&format!("    if {accessor} is not None:\n"));
                output.push_str(&format!("        _push_values(argv, {accessor})\n"));
            }
        }
    }
}

fn render_namespace(namespace: &str, functions: &[String], output: &mut String) {
    output.push_str(&format!("class {namespace}:\n"));
    output.push_str("    \"\"\"Namespace for generated command builders.\"\"\"\n\n");

    for function in functions {
        output.push_str(&format!("    {function} = staticmethod({function})\n"));
    }

    output.push_str("\n\n");
}

fn render_all(mut exports: Vec<String>, output: &mut String) {
    exports.sort();
    exports.dedup();

    output.push_str("__all__ = [\n");
    for export in exports {
        output.push_str(&format!("    {},\n", quote_double(&export)));
    }
    output.push_str("]\n");
}

fn field_comments(arg: &ArgSpec) -> Vec<String> {
    let mut comments = Vec::new();

    if let Some(help) = arg.help.as_deref().or(arg.long_help.as_deref()) {
        comments.push(collapse_lines(help));
    }

    if !arg.defaults.is_empty() {
        comments.push(format!("Clap default: {}.", arg.defaults.join(", ")));
    }

    comments
}

fn python_type(arg: &ArgSpec) -> String {
    let scalar = python_scalar_type(arg);

    if !matches!(arg.kind, ArgKind::Option | ArgKind::Positional) {
        return scalar;
    }

    if is_grouped_repeated(arg) {
        // Each inner sequence is one occurrence's values; the outer sequence
        // holds all occurrences. Both layers accept tuple or list so callers
        // can mix shapes naturally.
        let inner = format!("tuple[{scalar}, ...] | list[{scalar}]");
        format!("tuple[{inner}, ...] | list[{inner}]")
    } else if arg.value.repeated || arg.value.arity.allows_multiple() {
        format!("tuple[{scalar}, ...] | list[{scalar}]")
    } else {
        scalar
    }
}

fn python_scalar_type(arg: &ArgSpec) -> String {
    if !arg.possible_values.is_empty() {
        return format!(
            "Literal[{}]",
            arg.possible_values
                .iter()
                .map(|value| quote_double(&value.name))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    match arg.kind {
        ArgKind::FlagTrue | ArgKind::FlagFalse => "bool".to_owned(),
        ArgKind::Counter => "int".to_owned(),
        ArgKind::Option | ArgKind::Positional => match arg.value.ty {
            ValueType::Bool => "bool".to_owned(),
            // Python `int` is arbitrary precision, so wide integers are fine.
            ValueType::Integer | ValueType::BigInteger => "int".to_owned(),
            ValueType::Float => "float".to_owned(),
            ValueType::Path | ValueType::OsString => "PathValue".to_owned(),
            ValueType::Unknown | ValueType::String => "str".to_owned(),
        },
    }
}

fn command_function_prefix(command: &CommandSpec, path: &[String]) -> String {
    let joined = command_pieces(command, path)
        .iter()
        .map(|piece| snake_case(piece))
        .collect::<Vec<_>>()
        .join("_");
    safe_identifier(&joined, is_reserved)
}

fn property_name(arg: &ArgSpec) -> String {
    safe_identifier(&snake_case(&arg.id), is_reserved)
}

fn snake_case(input: &str) -> String {
    let output = lower_join(input, "_");
    if output.is_empty() {
        "value".to_owned()
    } else {
        output
    }
}

fn package_name(input: &str) -> String {
    safe_identifier(&snake_case(input), is_reserved)
}

fn module_stem(input: &str) -> String {
    safe_identifier(&snake_case(input), is_reserved)
}

fn safe_pascal_identifier(input: &str) -> String {
    // Python's hard keywords (`None`, `True`, `False`) are PascalCase, so even
    // a class name that survives identifier rules can collide. Reuse the same
    // reserved-word check applied to fields and functions.
    safe_identifier(&pascal_case(input), is_reserved)
}

fn is_reserved(identifier: &str) -> bool {
    RESERVED.contains(&identifier)
}

const RESERVED: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

fn docstring(input: &str) -> String {
    collapse_lines(input).replace("\"\"\"", "\\\"\\\"\\\"")
}

#[cfg(test)]
mod tests {
    use clap::Arg;
    use clap::ArgAction;
    use clap::Command;
    use clap::value_parser;

    use crate::Python;
    use crate::generate;

    #[test]
    fn generates_typed_python_builders() {
        let mut cmd = Command::new("demo-tool")
            .about("Demo tool")
            .arg(
                Arg::new("config")
                    .long("config")
                    .help("Path to config")
                    .value_parser(value_parser!(std::path::PathBuf))
                    .action(ArgAction::Set),
            )
            .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
            .arg(
                Arg::new("dry-run")
                    .long("dry-run")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jobs")
                    .long("jobs")
                    .value_parser(value_parser!(u16))
                    .action(ArgAction::Set),
            )
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .value_parser(["fast", "slow"])
                    .action(ArgAction::Set),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let mut output = Vec::<u8>::new();
        generate(
            Python::new()
                .module_name("demo_bindings")
                .namespace("DemoTool"),
            &mut cmd,
            "demo-tool",
            &mut output,
        )
        .expect("python generation works");
        let output = String::from_utf8(output).expect("python is utf-8");

        assert!(output.contains("class DemoToolArgs:"));
        assert!(output.contains("config: PathValue | None = None"));
        assert!(output.contains("jobs: int | None = None"));
        assert!(output.contains("mode: Literal[\"fast\", \"slow\"] | None = None"));
        assert!(output.contains("def run_command(\n    args: RunArgs,"));
        assert!(output.contains("argv.append(\"run\")"));
        assert!(output.contains("class DemoTool:"));
        assert!(!output.contains("help_command"));
    }

    #[test]
    fn grouped_repeated_option_emits_nested_type_and_dispatches_correctly() {
        // ArgAction::Append + num_args(2) is `--pair a b --pair c d`; the
        // generated type must allow nested sequences and the builder must
        // dispatch to _push_grouped_repeated_option so each group is emitted
        // as a separate `--pair v1 v2` occurrence.
        let mut cmd = Command::new("demo-tool").arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .action(ArgAction::Append),
        );

        let mut output = Vec::<u8>::new();
        generate(Python::new(), &mut cmd, "demo-tool", &mut output)
            .expect("python generation works");
        let output = String::from_utf8(output).expect("python is utf-8");

        assert!(
            output.contains(
                "pair: tuple[tuple[str, ...] | list[str], ...] | list[tuple[str, ...] | list[str]] | None = None"
            ),
            "expected nested-sequence type for grouped repeated option, got:\n{output}"
        );
        assert!(
            output.contains("_push_grouped_repeated_option(argv, \"--pair\", args.pair)"),
            "expected _push_grouped_repeated_option dispatch, got:\n{output}"
        );
    }

    #[test]
    fn variadic_positionals_emit_required_sequences_in_order() {
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
        generate(Python::new(), &mut cmd, "demo-tool", &mut output)
            .expect("python generation works");
        let output = String::from_utf8(output).expect("python is utf-8");

        assert!(
            output.contains("paths: tuple[str, ...] | list[str]"),
            "expected variadic positional sequence type, got:\n{output}"
        );
        assert!(
            output.contains("_push_values(argv, args.paths)"),
            "expected variadic positional builder, got:\n{output}"
        );
        let source_index = output
            .find("_push_values(argv, args.source)")
            .expect("source positional builder");
        let mode_index = output
            .find("if args.mode is not None:")
            .expect("mode option builder");
        let paths_index = output
            .find("_push_values(argv, args.paths)")
            .expect("variadic positional builder");
        assert!(source_index < mode_index);
        assert!(mode_index < paths_index);
    }

    #[test]
    fn sanitizes_python_reserved_namespace() {
        let mut cmd = Command::new("demo-tool").arg(Arg::new("input").required(true));
        let mut output = Vec::<u8>::new();
        generate(
            Python::new().namespace("None"),
            &mut cmd,
            "demo-tool",
            &mut output,
        )
        .expect("python generation works");
        let output = String::from_utf8(output).expect("python is utf-8");

        assert!(
            output.contains("class None_:"),
            "expected `class None_:` after reserved-word sanitization, got:\n{output}"
        );
        assert!(!output.contains("class None:"));
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
        crate::Generator::generate(&Python::new().output_contracts(), &spec, &mut output)
            .expect("python generation works");
        let output = String::from_utf8(output).expect("python is utf-8");

        assert!(output.contains("class OutputContract:"));
        assert!(output.contains("encoding=\"json-lines\""));
        assert!(output.contains("def parse_output("));
        assert!(output.contains("if contract.mode == \"interactive\":"));
    }
}
