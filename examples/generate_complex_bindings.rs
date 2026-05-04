// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command, ValueHint, value_parser};
use clap_types::{Kotlin, Python, Rust, TypeScript, generate_to};

fn main() -> std::io::Result<()> {
    let out_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/generated"));

    generate_app(
        repo_agent_cli(),
        "repo-agent",
        "repo_agent_bindings",
        "RepoAgent",
        &out_dir,
    )?;
    generate_app(
        data_forge_cli(),
        "data-forge",
        "data_forge_bindings",
        "DataForge",
        &out_dir,
    )?;

    Ok(())
}

fn generate_app(
    cli: Command,
    bin_name: &str,
    python_module: &str,
    python_namespace: &str,
    out_dir: &Path,
) -> std::io::Result<()> {
    let python_dir = out_dir.join("python");
    let python_package_dir = out_dir.join("python-package");
    let typescript_dir = out_dir.join("typescript");
    let typescript_node_dir = out_dir.join("typescript-node");
    let typescript_zod_dir = out_dir.join("typescript-zod");
    let rust_dir = out_dir.join("rust");
    let kotlin_dir = out_dir.join("kotlin");
    fs::create_dir_all(&python_dir)?;
    fs::create_dir_all(&python_package_dir)?;
    fs::create_dir_all(&typescript_dir)?;
    fs::create_dir_all(&typescript_node_dir)?;
    fs::create_dir_all(&typescript_zod_dir)?;
    fs::create_dir_all(&rust_dir)?;
    fs::create_dir_all(&kotlin_dir)?;

    let mut python_cli = cli.clone();
    generate_to(
        Python::new()
            .module_name(python_module)
            .namespace(python_namespace),
        &mut python_cli,
        bin_name,
        &python_dir,
    )?;

    let python_package = python_module
        .strip_suffix("_bindings")
        .unwrap_or(python_module);
    let mut python_package_cli = cli.clone();
    generate_to(
        Python::new()
            .module_name(python_package)
            .namespace(python_namespace)
            .package(),
        &mut python_package_cli,
        bin_name,
        &python_package_dir,
    )?;

    let mut typescript_cli = cli.clone();
    generate_to(
        TypeScript::new(),
        &mut typescript_cli,
        bin_name,
        &typescript_dir,
    )?;

    let mut typescript_node_cli = cli.clone();
    generate_to(
        TypeScript::new()
            .module_name(format!("{bin_name}-node"))
            .node(),
        &mut typescript_node_cli,
        bin_name,
        &typescript_node_dir,
    )?;

    let mut typescript_zod_cli = cli.clone();
    generate_to(
        TypeScript::new()
            .module_name(format!("{bin_name}-zod"))
            .zod(),
        &mut typescript_zod_cli,
        bin_name,
        &typescript_zod_dir,
    )?;

    let mut rust_cli = cli.clone();
    generate_to(
        Rust::new().module_name(python_module).output_contracts(),
        &mut rust_cli,
        bin_name,
        &rust_dir,
    )?;

    let mut kotlin_cli = cli;
    generate_to(
        Kotlin::new()
            .module_name(format!("{bin_name}-bindings"))
            .output_contracts(),
        &mut kotlin_cli,
        bin_name,
        &kotlin_dir,
    )?;

    Ok(())
}

fn repo_agent_cli() -> Command {
    Command::new("repo-agent")
        .about("Coordinate repository automation for local agents")
        .long_about(
            "Builds indexes, manages issue workflows, and launches local agent tasks against a workspace.",
        )
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .short('C')
                .global(true)
                .value_name("DIR")
                .help("Path to the workspace root")
                .value_hint(ValueHint::DirPath)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("profile")
                .long("profile")
                .global(true)
                .value_name("NAME")
                .default_value("dev")
                .help("Configuration profile")
                .value_parser(["dev", "ci", "prod"])
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .global(true)
                .help("Increase diagnostic verbosity")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .global(true)
                .default_value("auto")
                .help("Color output policy")
                .value_parser(["auto", "always", "never"])
                .action(ArgAction::Set),
        )
        .subcommand(
            Command::new("index")
                .about("Build or refresh a repository index")
                .arg(
                    Arg::new("input")
                        .help("Directory or file to index")
                        .required(true)
                        .value_hint(ValueHint::AnyPath)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("glob")
                        .long("glob")
                        .help("Glob pattern to include")
                        .value_name("PATTERN")
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("threads")
                        .long("threads")
                        .help("Worker thread count")
                        .value_parser(value_parser!(u16))
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .help("Index output format")
                        .value_parser(["json", "sqlite", "tantivy"])
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("follow-symlinks")
                        .long("follow-symlinks")
                        .help("Follow symbolic links while indexing")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("issue")
                .about("Manage issue workflows")
                .subcommand(
                    Command::new("create")
                        .about("Create an issue")
                        .arg(
                            Arg::new("title")
                                .long("title")
                                .required(true)
                                .help("Issue title")
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("body")
                                .long("body")
                                .help("Issue body markdown")
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("label")
                                .long("label")
                                .help("Label to apply")
                                .action(ArgAction::Append),
                        )
                        .arg(
                            Arg::new("priority")
                                .long("priority")
                                .help("Issue priority")
                                .value_parser(["low", "normal", "high", "urgent"])
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("assignee")
                                .long("assignee")
                                .help("User to assign")
                                .action(ArgAction::Set),
                        ),
                )
                .subcommand(
                    Command::new("close")
                        .about("Close an issue")
                        .arg(
                            Arg::new("id")
                                .help("Issue id")
                                .required(true)
                                .value_parser(value_parser!(u64)),
                        )
                        .arg(
                            Arg::new("reason")
                                .long("reason")
                                .help("Close reason")
                                .value_parser(["completed", "duplicate", "wont-fix"])
                                .action(ArgAction::Set),
                        ),
                ),
        )
        .subcommand(
            Command::new("agent").about("Run and inspect local agents").subcommand(
                Command::new("run")
                    .about("Run a local agent task")
                    .arg(
                        Arg::new("task")
                            .help("Task prompt or task id")
                            .required(true)
                            .action(ArgAction::Set),
                    )
                    .arg(
                        Arg::new("model")
                            .long("model")
                            .help("Model family")
                            .value_parser(["small", "standard", "frontier"])
                            .action(ArgAction::Set),
                    )
                    .arg(
                        Arg::new("temperature")
                            .long("temperature")
                            .help("Sampling temperature")
                            .value_parser(value_parser!(f32))
                            .action(ArgAction::Set),
                    )
                    .arg(
                        Arg::new("max-tokens")
                            .long("max-tokens")
                            .help("Maximum generated tokens")
                            .value_parser(value_parser!(u32))
                            .action(ArgAction::Set),
                    )
                    .arg(
                        Arg::new("env")
                            .long("env")
                            .value_name("KEY=VALUE")
                            .help("Environment variable for the agent process")
                            .action(ArgAction::Append),
                    )
                    .arg(
                        Arg::new("dry-run")
                            .long("dry-run")
                            .help("Build the task without executing it")
                            .action(ArgAction::SetTrue),
                    ),
            ),
        )
}

fn data_forge_cli() -> Command {
    Command::new("data-forge")
        .about("Prepare datasets and run repeatable data pipelines")
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .global(true)
                .help("Output directory")
                .value_hint(ValueHint::DirPath)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .global(true)
                .default_value("info")
                .help("Logging level")
                .value_parser(["trace", "debug", "info", "warn", "error"])
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .global(true)
                .help("Disable terminal color")
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("dataset")
                .about("Import and export datasets")
                .subcommand(
                    Command::new("import")
                        .about("Import a dataset")
                        .arg(
                            Arg::new("source")
                                .long("source")
                                .required(true)
                                .help("Source file or directory")
                                .value_hint(ValueHint::AnyPath)
                                .value_parser(value_parser!(PathBuf))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .required(true)
                                .help("Input format")
                                .value_parser(["csv", "json", "parquet"])
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("schema")
                                .long("schema")
                                .help("Schema file")
                                .value_hint(ValueHint::FilePath)
                                .value_parser(value_parser!(PathBuf))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("sample-rate")
                                .long("sample-rate")
                                .help("Fraction of rows to sample")
                                .value_parser(value_parser!(f64))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("tag")
                                .long("tag")
                                .help("Dataset tag")
                                .action(ArgAction::Append),
                        ),
                )
                .subcommand(
                    Command::new("export")
                        .about("Export a dataset")
                        .arg(
                            Arg::new("dataset")
                                .help("Dataset name")
                                .required(true)
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("destination")
                                .long("destination")
                                .required(true)
                                .help("Destination path")
                                .value_hint(ValueHint::AnyPath)
                                .value_parser(value_parser!(PathBuf))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("compression")
                                .long("compression")
                                .help("Compression format")
                                .value_parser(["none", "gzip", "zstd"])
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("partition")
                                .long("partition")
                                .help("Partition column")
                                .action(ArgAction::Append),
                        ),
                ),
        )
        .subcommand(
            Command::new("pipeline")
                .about("Run data pipelines")
                .subcommand(
                    Command::new("run")
                        .about("Run a pipeline")
                        .arg(
                            Arg::new("pipeline")
                                .help("Pipeline id")
                                .required(true)
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("param")
                                .long("param")
                                .value_name("KEY=VALUE")
                                .help("Pipeline parameter")
                                .action(ArgAction::Append),
                        )
                        .arg(
                            Arg::new("concurrency")
                                .long("concurrency")
                                .help("Maximum concurrent tasks")
                                .value_parser(value_parser!(u16))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("timeout-seconds")
                                .long("timeout-seconds")
                                .help("Pipeline timeout in seconds")
                                .value_parser(value_parser!(u64))
                                .action(ArgAction::Set),
                        )
                        .arg(
                            Arg::new("watch")
                                .long("watch")
                                .help("Watch until completion")
                                .action(ArgAction::SetTrue),
                        ),
                ),
        )
}
