// Copyright (c) Meta Platforms, Inc. and affiliates.

#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_types::{Kotlin, Python, Rust, TypeScript, generate_to};

fn main() -> std::io::Result<()> {
    let out_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/generated"));

    generate_app(
        OpsCtl::command(),
        "opsctl",
        "opsctl_bindings",
        "OpsCtl",
        &out_dir,
    )
}

fn generate_app(
    cli: clap::Command,
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

#[derive(Debug, Parser)]
#[command(
    name = "opsctl",
    about = "Operate services, secrets, and background jobs",
    long_about = "A derive-based clap CLI used to demonstrate generated bindings from Parser, Args, Subcommand, and ValueEnum definitions."
)]
struct OpsCtl {
    /// Path to the workspace root.
    #[arg(short = 'C', long, global = true, value_name = "DIR", value_hint = ValueHint::DirPath)]
    workspace: Option<PathBuf>,

    /// Output format for human and machine consumers.
    #[arg(long, global = true, value_enum, default_value = "table")]
    output: OutputFormat,

    /// Increase diagnostic verbosity.
    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: OpsCommand,
}

#[derive(Debug, Subcommand)]
enum OpsCommand {
    /// Deploy a service revision.
    Deploy(DeployArgs),
    /// Manage runtime secrets.
    Secret {
        #[command(subcommand)]
        command: SecretCommand,
    },
    /// Run and inspect background jobs.
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
}

#[derive(Debug, Args)]
struct DeployArgs {
    /// Service name to deploy.
    #[arg(value_name = "SERVICE")]
    service: String,

    /// Container image reference.
    #[arg(long, value_name = "IMAGE", required = true)]
    image: String,

    /// Deployment environment.
    #[arg(long, value_enum, default_value = "dev")]
    environment: Environment,

    /// Number of replicas.
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..=1024))]
    replicas: Option<u16>,

    /// Environment variable to inject.
    #[arg(long, value_name = "KEY=VALUE", action = ArgAction::Append)]
    env: Vec<String>,

    /// Wait for the rollout to finish.
    #[arg(long)]
    wait: bool,

    /// Rollout timeout in seconds.
    #[arg(long, value_parser = clap::value_parser!(f64))]
    timeout_seconds: Option<f64>,
}

#[derive(Debug, Subcommand)]
enum SecretCommand {
    /// Set or replace a secret value.
    Set(SecretSetArgs),
    /// Rotate a secret and notify dependent services.
    Rotate(SecretRotateArgs),
}

#[derive(Debug, Args)]
struct SecretSetArgs {
    /// Secret name.
    name: String,

    /// Inline secret value.
    #[arg(long, conflicts_with = "file")]
    value: Option<String>,

    /// File containing the secret value.
    #[arg(long, value_hint = ValueHint::FilePath, conflicts_with = "value")]
    file: Option<PathBuf>,

    /// Secret visibility scope.
    #[arg(long, value_enum)]
    scope: Option<SecretScope>,

    /// Store the secret as sealed ciphertext.
    #[arg(long)]
    sealed: bool,
}

#[derive(Debug, Args)]
struct SecretRotateArgs {
    /// Secret name.
    name: String,

    /// Services to restart after rotation.
    #[arg(long, value_name = "SERVICE", action = ArgAction::Append)]
    restart: Vec<String>,

    /// Rotation grace period in seconds.
    #[arg(long, value_parser = clap::value_parser!(u64))]
    grace_seconds: Option<u64>,
}

#[derive(Debug, Subcommand)]
enum JobCommand {
    /// Run a background job.
    Run(JobRunArgs),
    /// List background jobs.
    List(JobListArgs),
}

#[derive(Debug, Args)]
struct JobRunArgs {
    /// Job name.
    job: String,

    /// Schedule expression.
    #[arg(long)]
    schedule: Option<String>,

    /// Maximum concurrent workers.
    #[arg(long, value_parser = clap::value_parser!(u16))]
    concurrency: Option<u16>,

    /// Job parameter.
    #[arg(long, value_name = "KEY=VALUE", action = ArgAction::Append)]
    param: Vec<String>,

    /// Validate the job without enqueueing it.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct JobListArgs {
    /// Filter by job status.
    #[arg(long, value_enum)]
    status: Option<JobStatus>,

    /// Maximum rows to return.
    #[arg(long, value_parser = clap::value_parser!(u32))]
    limit: Option<u32>,

    /// Include completed jobs.
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    JsonLines,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Environment {
    Dev,
    Staging,
    Prod,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SecretScope {
    Project,
    Organization,
    Global,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}
