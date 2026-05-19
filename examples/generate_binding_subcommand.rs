// Copyright (c) Meta Platforms, Inc. and affiliates.

use clap::Arg;
use clap::ArgAction;
use clap::Command;
use clap::ValueHint;
use clap::value_parser;
use clap_types::BINDING_COMMAND_NAME;
use clap_types::binding_command;
use clap_types::generate_binding_from_matches;

fn build_cli() -> Command {
    Command::new("repo-agent")
        .about("Coordinate repository automation for local agents")
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .short('C')
                .global(true)
                .value_name("DIR")
                .help("Path to the workspace root")
                .value_hint(ValueHint::DirPath)
                .value_parser(value_parser!(std::path::PathBuf))
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
        .subcommand(
            Command::new("index")
                .about("Build or refresh a repository index")
                .arg(
                    Arg::new("input")
                        .required(true)
                        .value_hint(ValueHint::AnyPath),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_parser(["json", "sqlite"])
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("follow-symlinks")
                        .long("follow-symlinks")
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn main() -> std::io::Result<()> {
    let matches = build_cli().subcommand(binding_command()).get_matches();

    if let Some((BINDING_COMMAND_NAME, binding_matches)) = matches.subcommand() {
        let cmd = build_cli();
        let path = generate_binding_from_matches(&cmd, "repo-agent", binding_matches)?;
        eprintln!("generated {}", path.display());
        return Ok(());
    }

    // Normal application dispatch would happen here.
    Ok(())
}
