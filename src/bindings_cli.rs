// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::io;
use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::{Kotlin, Python, Rust, TypeScript, generate_to};

/// Name of the generated hidden subcommand.
pub const BINDING_COMMAND_NAME: &str = "generate-binding";

/// Build a hidden `generate-binding` subcommand for embedding in a clap app.
///
/// Add this to your own CLI, then pass the matched subcommand to
/// [`generate_binding_from_matches`] with a fresh copy of your application command.
#[must_use]
pub fn binding_command() -> Command {
    Command::new(BINDING_COMMAND_NAME)
        .hide(true)
        .about("Generate typed client bindings for this CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("typescript")
                .about("Generate TypeScript bindings")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args())
                .arg(
                    Arg::new("zod")
                        .long("zod")
                        .help("Emit Zod schemas and validate builders with Schema.parse")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("zod_schemas"),
                )
                .arg(
                    Arg::new("zod_schemas")
                        .long("zod-schemas")
                        .help("Emit Zod schemas and inferred types without builder validation")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("node")
                        .long("node")
                        .help("Emit Node child_process helpers")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("python")
                .about("Generate Python bindings")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args())
                .arg(
                    Arg::new("namespace")
                        .long("namespace")
                        .value_name("NAME")
                        .help("Namespace class to expose generated functions as static methods")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("package")
                        .long("full-module")
                        .alias("package")
                        .help("Emit a package layout with one module per command")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("rust")
                .about("Generate Rust bindings")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args()),
        )
        .subcommand(
            Command::new("kotlin")
                .about("Generate Kotlin/JVM bindings")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args())
                .arg(
                    Arg::new("package_name")
                        .long("package")
                        .value_name("NAME")
                        .help("Kotlin package declaration")
                        .action(ArgAction::Set),
                ),
        )
}

/// Generate bindings from matches produced by [`binding_command`].
///
/// Returns the primary output path. For single-file generators this is the file;
/// for package generators this is the package directory.
pub fn generate_binding_from_matches(
    cmd: &mut Command,
    bin_name: impl Into<String>,
    matches: &ArgMatches,
) -> io::Result<PathBuf> {
    let bin_name = bin_name.into();
    match matches.subcommand() {
        Some(("typescript", matches)) => generate_typescript(cmd, &bin_name, matches),
        Some(("python", matches)) => generate_python(cmd, &bin_name, matches),
        Some(("rust", matches)) => generate_rust(cmd, &bin_name, matches),
        Some(("kotlin", matches)) => generate_kotlin(cmd, &bin_name, matches),
        Some((name, _)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported binding generator `{name}`"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing binding generator subcommand",
        )),
    }
}

fn generate_typescript(
    cmd: &mut Command,
    bin_name: &str,
    matches: &ArgMatches,
) -> io::Result<PathBuf> {
    let mut generator = TypeScript::new();

    if let Some(module_name) = matches.get_one::<String>("module_name") {
        generator = generator.module_name(module_name);
    }
    if matches.get_flag("zod") {
        generator = generator.zod();
    } else if matches.get_flag("zod_schemas") {
        generator = generator.zod_schemas();
    }
    if matches.get_flag("node") {
        generator = generator.node();
    }
    if wants_output_contracts(matches) {
        generator = generator.output_contracts();
    }

    generate_to(generator, cmd, bin_name, output_path(matches))
}

fn generate_python(cmd: &mut Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    let mut generator = Python::new();

    if let Some(module_name) = matches.get_one::<String>("module_name") {
        generator = generator.module_name(module_name);
    }
    if let Some(namespace) = matches.get_one::<String>("namespace") {
        generator = generator.namespace(namespace);
    }
    if wants_output_contracts(matches) {
        generator = generator.output_contracts();
    }

    let out_dir = output_path(matches);
    if matches.get_flag("package") {
        generate_to(generator.package(), cmd, bin_name, out_dir)
    } else {
        generate_to(generator, cmd, bin_name, out_dir)
    }
}

fn generate_rust(cmd: &mut Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    let mut generator = Rust::new();

    if let Some(module_name) = matches.get_one::<String>("module_name") {
        generator = generator.module_name(module_name);
    }
    if wants_output_contracts(matches) {
        generator = generator.output_contracts();
    }

    generate_to(generator, cmd, bin_name, output_path(matches))
}

fn generate_kotlin(cmd: &mut Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    let mut generator = Kotlin::new();

    if let Some(module_name) = matches.get_one::<String>("module_name") {
        generator = generator.module_name(module_name);
    }
    if let Some(package_name) = matches.get_one::<String>("package_name") {
        generator = generator.package_name(package_name);
    }
    if wants_output_contracts(matches) {
        generator = generator.output_contracts();
    }

    generate_to(generator, cmd, bin_name, output_path(matches))
}

fn output_path(matches: &ArgMatches) -> PathBuf {
    matches
        .get_one::<String>("path")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/generated"))
}

fn output_path_arg() -> Arg {
    Arg::new("path")
        .long("path")
        .short('o')
        .value_name("DIR")
        .help("Output directory")
        .action(ArgAction::Set)
        .default_value("target/generated")
}

fn module_name_arg() -> Arg {
    Arg::new("module_name")
        .long("module-name")
        .value_name("NAME")
        .help("Generated file stem or package name")
        .action(ArgAction::Set)
}

fn output_contract_args() -> [Arg; 2] {
    [
        Arg::new("output_contracts")
            .long("output-contracts")
            .help("Emit output-contract metadata and parser helpers")
            .action(ArgAction::SetTrue)
            .conflicts_with("no_output_contracts"),
        Arg::new("no_output_contracts")
            .long("no-output-contracts")
            .help("Omit output-contract metadata and parser helpers")
            .action(ArgAction::SetTrue),
    ]
}

fn wants_output_contracts(matches: &ArgMatches) -> bool {
    matches.get_flag("output_contracts") && !matches.get_flag("no_output_contracts")
}
