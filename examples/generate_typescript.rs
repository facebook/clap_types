// Copyright (c) Meta Platforms, Inc. and affiliates.

use clap::Arg;
use clap::ArgAction;
use clap::Command;
use clap_types::TypeScript;
use clap_types::generate;

fn build_cli() -> Command {
    Command::new("example")
        .about("Example CLI for clap_types")
        .arg(
            Arg::new("config")
                .long("config")
                .value_name("FILE")
                .help("Path to a config file")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase logging verbosity")
                .action(ArgAction::Count),
        )
        .subcommand(
            Command::new("run")
                .about("Run a target")
                .arg(Arg::new("target").required(true)),
        )
}

fn main() -> std::io::Result<()> {
    let cmd = build_cli();
    generate(
        TypeScript::new(),
        &cmd,
        "example",
        &mut std::io::stdout(),
    )
}
