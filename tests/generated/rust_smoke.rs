// Copyright (c) Meta Platforms, Inc. and affiliates.

#[path = "../../target/generated/rust/data_forge_bindings.rs"]
mod data_forge_bindings;
#[path = "../../target/generated/rust/opsctl_bindings.rs"]
mod opsctl_bindings;
#[path = "../../target/generated/rust/repo_agent_bindings.rs"]
mod repo_agent_bindings;

use std::path::PathBuf;

use data_forge_bindings::DatasetImportArgs;
use data_forge_bindings::DatasetImportFormat;
use data_forge_bindings::DatasetImportLogLevel;
use data_forge_bindings::build_dataset_import_command;
use opsctl_bindings::DeployArgs;
use opsctl_bindings::DeployEnvironment;
use opsctl_bindings::DeployOutput;
use opsctl_bindings::build_deploy_command;
use repo_agent_bindings::AgentRunArgs;
use repo_agent_bindings::AgentRunModel;
use repo_agent_bindings::build_agent_run_command;

fn main() {
    smoke_data_forge();
    smoke_opsctl();
    smoke_repo_agent();
}

fn smoke_data_forge() {
    let args = DatasetImportArgs {
        source: PathBuf::from("data/input.csv"),
        format: DatasetImportFormat::Csv,
        output: Some(PathBuf::from("target/out")),
        log_level: Some(DatasetImportLogLevel::Debug),
        no_color: true,
        schema: Some(PathBuf::from("schema.json")),
        sample_rate: Some(0.25),
        tag: vec!["raw".to_owned(), "daily".to_owned()],
    };

    let argv = build_dataset_import_command(&args);
    assert_eq!(
        argv,
        vec![
            "--output",
            "target/out",
            "--log-level",
            "debug",
            "--no-color",
            "dataset",
            "import",
            "--source",
            "data/input.csv",
            "--format",
            "csv",
            "--schema",
            "schema.json",
            "--sample-rate",
            "0.25",
            "--tag",
            "raw",
            "--tag",
            "daily",
        ],
    );
}

fn smoke_opsctl() {
    let args = DeployArgs {
        service: "api".to_owned(),
        image: "registry.example.com/api:1".to_owned(),
        workspace: Some(PathBuf::from("/workspace")),
        output: Some(DeployOutput::Json),
        verbose: 1,
        environment: Some(DeployEnvironment::Prod),
        replicas: Some(3),
        env: vec!["RUST_LOG=info".to_owned()],
        wait: true,
        timeout_seconds: Some(30.5),
    };

    let argv = build_deploy_command(&args);
    assert_eq!(
        argv,
        vec![
            "--workspace",
            "/workspace",
            "--output",
            "json",
            "--verbose",
            "deploy",
            "api",
            "--image",
            "registry.example.com/api:1",
            "--environment",
            "prod",
            "--replicas",
            "3",
            "--env",
            "RUST_LOG=info",
            "--wait",
            "--timeout-seconds",
            "30.5",
        ],
    );
}

fn smoke_repo_agent() {
    let args = AgentRunArgs {
        task: "summarize".to_owned(),
        workspace: Some(PathBuf::from("/repo")),
        profile: None,
        verbose: 2,
        color: None,
        model: Some(AgentRunModel::Frontier),
        temperature: Some(0.2),
        max_tokens: Some(2048),
        env: vec!["RUST_LOG=info".to_owned()],
        dry_run: true,
    };

    let argv = build_agent_run_command(&args);
    assert_eq!(
        argv,
        vec![
            "--workspace",
            "/repo",
            "--verbose",
            "--verbose",
            "agent",
            "run",
            "summarize",
            "--model",
            "frontier",
            "--temperature",
            "0.2",
            "--max-tokens",
            "2048",
            "--env",
            "RUST_LOG=info",
            "--dry-run",
        ],
    );
}
