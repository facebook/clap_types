// Copyright (c) Meta Platforms, Inc. and affiliates.

//! Embeddable `generate-binding` clap subcommand that emits bindings from a
//! host CLI through every supported generator (TypeScript, Flow, Python, Rust,
//! Kotlin). Host CLIs add the subcommand with [`binding_command`] and dispatch
//! to it with [`generate_binding_from_matches`].

use std::io;
use std::path::Path;
use std::path::PathBuf;

use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use clap::Command;

use crate::CliSpec;
use crate::Flow;
use crate::Generator;
use crate::Kotlin;
use crate::OutputSpec;
use crate::Python;
use crate::ReflectOptions;
use crate::Rust;
use crate::TypeScript;
use crate::generate_to_with_options;
use crate::reflect_command_with_options;

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
                .arg(include_hidden_arg())
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
            Command::new("flow")
                .about("Generate JavaScript bindings annotated with Flow types")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args())
                .arg(include_hidden_arg())
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
                        .help("Emit Zod schemas without builder validation")
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
                .arg(include_hidden_arg())
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
                .args(output_contract_args())
                .arg(include_hidden_arg()),
        )
        .subcommand(
            Command::new("kotlin")
                .about("Generate Kotlin/JVM bindings")
                .arg(output_path_arg())
                .arg(module_name_arg())
                .args(output_contract_args())
                .arg(include_hidden_arg())
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
    cmd: &Command,
    bin_name: impl Into<String>,
    matches: &ArgMatches,
) -> io::Result<PathBuf> {
    let bin_name = bin_name.into();
    match matches.subcommand() {
        Some(("typescript", matches)) => generate_typescript(cmd, &bin_name, matches),
        Some(("flow", matches)) => generate_flow(cmd, &bin_name, matches),
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

/// Generate bindings from matches produced by [`binding_command`], injecting
/// caller-provided output specs into the reflected CLI model.
///
/// Unlike [`generate_binding_from_matches`], this variant always enables
/// output-contract generation since the caller explicitly supplies the specs.
pub fn generate_binding_from_matches_with_outputs(
    cmd: &Command,
    bin_name: &str,
    matches: &ArgMatches,
    outputs: Vec<OutputSpec>,
) -> io::Result<PathBuf> {
    let (gen_name, sub_m) = matches.subcommand().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing binding generator subcommand",
        )
    })?;

    let opts = reflect_opts(sub_m);
    let mut spec = reflect_command_with_options(cmd.clone(), bin_name.to_owned(), opts);
    spec.outputs = outputs;
    let out_dir = output_path(sub_m);

    match gen_name {
        "typescript" => write_spec(&build_typescript(sub_m, true), &spec, &out_dir),
        "flow" => write_spec(&build_flow(sub_m, true), &spec, &out_dir),
        "python" => {
            let generator = build_python(sub_m, true);
            if sub_m.get_flag("package") {
                write_spec(&generator.package(), &spec, &out_dir)
            } else {
                write_spec(&generator, &spec, &out_dir)
            }
        }
        "rust" => write_spec(&build_rust(sub_m, true), &spec, &out_dir),
        "kotlin" => write_spec(&build_kotlin(sub_m, true), &spec, &out_dir),
        name => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported binding generator `{name}`"),
        )),
    }
}

fn generate_typescript(cmd: &Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    generate_to_with_options(
        build_typescript(matches, wants_output_contracts(matches)),
        cmd,
        bin_name,
        output_path(matches),
        reflect_opts(matches),
    )
}

fn generate_flow(cmd: &Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    generate_to_with_options(
        build_flow(matches, wants_output_contracts(matches)),
        cmd,
        bin_name,
        output_path(matches),
        reflect_opts(matches),
    )
}

fn generate_python(cmd: &Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    let generator = build_python(matches, wants_output_contracts(matches));
    let out_dir = output_path(matches);
    let opts = reflect_opts(matches);
    if matches.get_flag("package") {
        generate_to_with_options(generator.package(), cmd, bin_name, out_dir, opts)
    } else {
        generate_to_with_options(generator, cmd, bin_name, out_dir, opts)
    }
}

fn generate_rust(cmd: &Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    generate_to_with_options(
        build_rust(matches, wants_output_contracts(matches)),
        cmd,
        bin_name,
        output_path(matches),
        reflect_opts(matches),
    )
}

fn generate_kotlin(cmd: &Command, bin_name: &str, matches: &ArgMatches) -> io::Result<PathBuf> {
    generate_to_with_options(
        build_kotlin(matches, wants_output_contracts(matches)),
        cmd,
        bin_name,
        output_path(matches),
        reflect_opts(matches),
    )
}

// Per-language generator builders shared between the two top-level
// `generate_binding_from_matches*` entry points. The `output_contracts`
// argument lets the `_with_outputs` variant force-enable contracts when it
// has explicit outputs to inject.

fn build_typescript(sub_m: &ArgMatches, output_contracts: bool) -> TypeScript {
    let mut g = TypeScript::new();
    if let Some(module_name) = sub_m.get_one::<String>("module_name") {
        g = g.module_name(module_name);
    }
    if sub_m.get_flag("zod") {
        g = g.zod();
    } else if sub_m.get_flag("zod_schemas") {
        g = g.zod_schemas();
    }
    if sub_m.get_flag("node") {
        g = g.node();
    }
    if output_contracts {
        g = g.output_contracts();
    }
    g
}

fn build_flow(sub_m: &ArgMatches, output_contracts: bool) -> Flow {
    let mut g = Flow::new();
    if let Some(module_name) = sub_m.get_one::<String>("module_name") {
        g = g.module_name(module_name);
    }
    if sub_m.get_flag("zod") {
        g = g.zod();
    } else if sub_m.get_flag("zod_schemas") {
        g = g.zod_schemas();
    }
    if sub_m.get_flag("node") {
        g = g.node();
    }
    if output_contracts {
        g = g.output_contracts();
    }
    g
}

fn build_python(sub_m: &ArgMatches, output_contracts: bool) -> Python {
    let mut g = Python::new();
    if let Some(module_name) = sub_m.get_one::<String>("module_name") {
        g = g.module_name(module_name);
    }
    if let Some(namespace) = sub_m.get_one::<String>("namespace") {
        g = g.namespace(namespace);
    }
    if output_contracts {
        g = g.output_contracts();
    }
    g
}

fn build_rust(sub_m: &ArgMatches, output_contracts: bool) -> Rust {
    let mut g = Rust::new();
    if let Some(module_name) = sub_m.get_one::<String>("module_name") {
        g = g.module_name(module_name);
    }
    if output_contracts {
        g = g.output_contracts();
    }
    g
}

fn build_kotlin(sub_m: &ArgMatches, output_contracts: bool) -> Kotlin {
    let mut g = Kotlin::new();
    if let Some(module_name) = sub_m.get_one::<String>("module_name") {
        g = g.module_name(module_name);
    }
    if let Some(package_name) = sub_m.get_one::<String>("package_name") {
        g = g.package_name(package_name);
    }
    if output_contracts {
        g = g.output_contracts();
    }
    g
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

fn include_hidden_arg() -> Arg {
    Arg::new("include_hidden")
        .long("include-hidden")
        .help(
            "Include hidden subcommands and args in the generated bindings. \
             Use for 1st-party callers that need to invoke hidden surfaces \
             (e.g. Meta-internal tools embedded in a public CLI).",
        )
        .action(ArgAction::SetTrue)
}

fn reflect_opts(matches: &ArgMatches) -> ReflectOptions {
    let mut opts = ReflectOptions::default();
    if matches.get_flag("include_hidden") {
        opts.include_hidden = true;
    }
    opts
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
    // The two flags `conflicts_with` each other at the clap level, so checking
    // `!no_output_contracts` here would always be true when `output_contracts`
    // is set.
    matches.get_flag("output_contracts")
}

fn write_spec(generator: &impl Generator, spec: &CliSpec, out_dir: &Path) -> io::Result<PathBuf> {
    use std::io::Write;

    let files = generator.generate_files(spec)?;
    std::fs::create_dir_all(out_dir)?;
    for generated in files {
        let path = out_dir.join(&generated.relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&path)?;
        file.write_all(&generated.contents)?;
        file.flush()?;
    }
    Ok(out_dir.join(generator.file_name(&spec.bin_name)))
}
