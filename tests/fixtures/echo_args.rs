// Copyright (c) Meta Platforms, Inc. and affiliates.

//! Shared fixture CLI used by the round-trip integration test.
//!
//! `cli()` is invoked from both the example binary (which prints parsed values
//! back to stdout) and `tests/roundtrip.rs` (which generates bindings from the
//! same `Command` definition). Sharing one source guarantees the bindings being
//! tested match the binary doing the parsing.

// Each `#[path]` includer of this file uses only one of cli()/render(); the
// unused half looks like dead code to that crate.
#![allow(dead_code)]

use std::path::PathBuf;

use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use clap::Command;
use clap::value_parser;

pub(crate) fn cli() -> Command {
    Command::new("echo-args")
        .about("Echo parsed clap args as key=value lines")
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .short('w')
                .global(true)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .global(true)
                .action(ArgAction::Count),
        )
        .subcommand(
            Command::new("greet")
                .about("Greet someone with optional tags")
                .arg(Arg::new("name").required(true).action(ArgAction::Set))
                .arg(Arg::new("loud").long("loud").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("repeat")
                        .long("repeat")
                        .value_parser(value_parser!(u32))
                        .default_value("1")
                        .action(ArgAction::Set),
                )
                .arg(Arg::new("tag").long("tag").action(ArgAction::Append))
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_parser(["fast", "slow"])
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("word")
                        .help("One or more words to include in the greeting")
                        .num_args(1..)
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
}

pub(crate) fn render(matches: &ArgMatches) -> String {
    let mut out = String::new();
    let Some(("greet", sub)) = matches.subcommand() else {
        return out;
    };

    out.push_str("subcommand=greet\n");

    if let Some(workspace) = sub.get_one::<PathBuf>("workspace") {
        out.push_str(&format!("workspace={}\n", workspace.display()));
    }
    out.push_str(&format!("verbose={}\n", sub.get_count("verbose")));

    if let Some(name) = sub.get_one::<String>("name") {
        out.push_str(&format!("name={name}\n"));
    }
    out.push_str(&format!("loud={}\n", sub.get_flag("loud")));
    if let Some(repeat) = sub.get_one::<u32>("repeat") {
        out.push_str(&format!("repeat={repeat}\n"));
    }
    if let Some(tags) = sub.get_many::<String>("tag") {
        let joined = tags.cloned().collect::<Vec<_>>().join(",");
        out.push_str(&format!("tag={joined}\n"));
    }
    if let Some(mode) = sub.get_one::<String>("mode") {
        out.push_str(&format!("mode={mode}\n"));
    }
    if let Some(words) = sub.get_many::<String>("word") {
        let joined = words.cloned().collect::<Vec<_>>().join(",");
        out.push_str(&format!("word={joined}\n"));
    }

    out
}
