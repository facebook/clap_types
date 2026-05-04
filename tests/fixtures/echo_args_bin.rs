// Copyright (c) Meta Platforms, Inc. and affiliates.

#[path = "echo_args.rs"]
mod echo_args;

fn main() {
    let matches = echo_args::cli().get_matches();
    print!("{}", echo_args::render(&matches));
}
