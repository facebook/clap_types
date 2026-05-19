// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::ffi::OsStr;
use std::fs;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use clap::Arg;
use clap::ArgAction;
use clap::Command;
use clap_types::Flow;
use clap_types::Kotlin;
use clap_types::Python;
use clap_types::ReflectOptions;
use clap_types::Rust;
use clap_types::TypeScript;
use clap_types::binding_command;
use clap_types::generate_binding_from_matches;
use clap_types::generate_to;
use clap_types::reflect_command_with_name;
use clap_types::reflect_command_with_options;

#[test]
fn generate_to_writes_typescript_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_{unique}"));

    let mut cmd = Command::new("demo").arg(Arg::new("input").required(true));
    let path = generate_to(TypeScript::new(), &mut cmd, "demo", &dir)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo.ts")));
    let typescript = fs::read_to_string(&path)?;
    assert!(typescript.contains("export interface DemoArgs"));
    assert!(typescript.contains("input: string;"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_flow_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_flow_{unique}"));

    let mut cmd = Command::new("demo").arg(Arg::new("input").required(true));
    let path = generate_to(Flow::new(), &mut cmd, "demo", &dir)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo.js")));
    let flow = fs::read_to_string(&path)?;
    assert!(flow.contains("@flow strict"));
    assert!(flow.contains("export type DemoArgs"));
    assert!(flow.contains("+input: string,"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_creates_nested_output_directory() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("clap_types_generate_to_nested_{unique}"));
    let dir = root.join("generated").join("typescript");

    let mut cmd = Command::new("demo");
    let path = generate_to(TypeScript::new(), &mut cmd, "demo", &dir)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo.ts")));
    assert!(path.exists());
    assert!(dir.is_dir());

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn generate_to_writes_python_file_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_python_{unique}"));
    fs::create_dir_all(&dir)?;

    let mut cmd = Command::new("demo-tool").arg(Arg::new("input").required(true));
    let path = generate_to(
        Python::new()
            .module_name("demo_bindings")
            .namespace("DemoTool"),
        &mut cmd,
        "demo-tool",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo_bindings.py")));
    let python = fs::read_to_string(&path)?;
    assert!(python.contains("class DemoToolArgs"));
    assert!(python.contains("input: str"));
    assert!(python.contains("class DemoTool:"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_python_package() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_python_package_{unique}"));

    let mut cmd = Command::new("demo-tool")
        .arg(Arg::new("workspace").long("workspace").global(true))
        .subcommand(Command::new("issue").subcommand(
            Command::new("create").arg(Arg::new("title").long("title").required(true)),
        ));
    let path = generate_to(
        Python::new()
            .module_name("demo_bindings")
            .namespace("DemoTool")
            .package(),
        &mut cmd,
        "demo-tool",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo_bindings")));
    assert!(path.join("__init__.py").is_file());
    assert!(path.join("_runtime.py").is_file());
    assert!(path.join("_root.py").is_file());
    assert!(path.join("issue").join("__init__.py").is_file());
    assert!(path.join("issue").join("_root.py").is_file());
    assert!(path.join("issue").join("create.py").is_file());

    let init = fs::read_to_string(path.join("__init__.py"))?;
    assert!(init.contains("from .issue.create import ("));
    assert!(init.contains("Args as IssueCreateArgs,"));
    assert!(init.contains("class DemoTool:"));

    let create = fs::read_to_string(path.join("issue").join("create.py"))?;
    assert!(create.contains("from .._runtime import ("));
    assert!(create.contains("class Args:"));
    assert!(create.contains("title: str"));
    assert!(create.contains("argv.append(\"issue\")"));
    assert!(create.contains("argv.append(\"create\")"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_zod_typescript_file_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_zod_{unique}"));
    fs::create_dir_all(&dir)?;

    let mut cmd = Command::new("demo").arg(Arg::new("mode").long("mode").value_parser(["fast"]));
    let path = generate_to(
        TypeScript::new().module_name("demo_zod").zod(),
        &mut cmd,
        "demo",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo-zod.ts")));
    let typescript = fs::read_to_string(&path)?;
    assert!(typescript.contains("import { z } from \"zod\";"));
    assert!(typescript.contains("export const DemoArgsSchema"));
    assert!(typescript.contains("const parsed = DemoArgsSchema.parse(args);"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_node_typescript_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_node_{unique}"));

    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_to(TypeScript::new().node(), &mut cmd, "demo", &dir)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo.ts")));
    let typescript = fs::read_to_string(&path)?;
    assert!(typescript.contains("from \"node:child_process\";"));
    assert!(typescript.contains("export function createRunCommand(args: RunArgs"));
    assert!(typescript.contains("export function runRunCommand(args: RunArgs"));
    assert!(typescript.contains("export function spawnRunCommand(args: RunArgs"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_zod_flow_file_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_flow_zod_{unique}"));
    fs::create_dir_all(&dir)?;

    let mut cmd = Command::new("demo").arg(Arg::new("mode").long("mode").value_parser(["fast"]));
    let path = generate_to(
        Flow::new().module_name("demo_zod").zod(),
        &mut cmd,
        "demo",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo-zod.js")));
    let flow = fs::read_to_string(&path)?;
    assert!(flow.contains("import { z } from \"zod\";"));
    assert!(flow.contains("export const DemoArgsSchema"));
    assert!(flow.contains("const parsed: DemoArgs = DemoArgsSchema.parse(args);"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_node_flow_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_flow_node_{unique}"));

    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_to(Flow::new().node(), &mut cmd, "demo", &dir)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo.js")));
    let flow = fs::read_to_string(&path)?;
    assert!(flow.contains("from \"node:child_process\";"));
    assert!(flow.contains("export function createRunCommand(args: RunArgs"));
    assert!(flow.contains("export function runRunCommand(args: RunArgs"));
    assert!(flow.contains("export function spawnRunCommand(args: RunArgs"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn embedded_binding_command_generates_typescript_node_zod() -> Result<(), Box<dyn std::error::Error>>
{
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_binding_cli_ts_{unique}"));

    let matches = binding_command().try_get_matches_from([
        "generate-binding",
        "typescript",
        "--zod",
        "--node",
        "--module-name",
        "demo-node",
        "--path",
        dir.to_str().ok_or("temp path was not valid UTF-8")?,
    ])?;
    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_binding_from_matches(&mut cmd, "demo", &matches)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo-node.ts")));
    let typescript = fs::read_to_string(&path)?;
    assert!(typescript.contains("import { z } from \"zod\";"));
    assert!(typescript.contains("from \"node:child_process\";"));
    assert!(typescript.contains("export function runRunCommand(args: RunArgs"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn embedded_binding_command_generates_flow_node_zod() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_binding_cli_flow_{unique}"));

    let matches = binding_command().try_get_matches_from([
        "generate-binding",
        "flow",
        "--zod",
        "--node",
        "--module-name",
        "demo-node",
        "--path",
        dir.to_str().ok_or("temp path was not valid UTF-8")?,
    ])?;
    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_binding_from_matches(&mut cmd, "demo", &matches)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo-node.js")));
    let flow = fs::read_to_string(&path)?;
    assert!(flow.contains("import { z } from \"zod\";"));
    assert!(flow.contains("from \"node:child_process\";"));
    assert!(flow.contains("export function runRunCommand(args: RunArgs"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn embedded_binding_command_generates_python_package() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_binding_cli_python_{unique}"));

    let matches = binding_command().try_get_matches_from([
        "generate-binding",
        "python",
        "--full-module",
        "--module-name",
        "demo_pkg",
        "--namespace",
        "Demo",
        "--path",
        dir.to_str().ok_or("temp path was not valid UTF-8")?,
    ])?;
    let mut cmd = Command::new("demo")
        .subcommand(Command::new("issue").subcommand(
            Command::new("create").arg(Arg::new("title").long("title").required(true)),
        ));
    let path = generate_binding_from_matches(&mut cmd, "demo", &matches)?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo_pkg")));
    assert!(path.join("__init__.py").is_file());
    assert!(path.join("_runtime.py").is_file());
    assert!(path.join("issue").join("create.py").is_file());

    let init = fs::read_to_string(path.join("__init__.py"))?;
    assert!(init.contains("class Demo:"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_rust_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_rust_{unique}"));

    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_to(
        Rust::new().module_name("demo_bindings"),
        &mut cmd,
        "demo",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("demo_bindings.rs")));
    let rust = fs::read_to_string(&path)?;
    assert!(rust.contains("pub struct RunArgs"));
    assert!(rust.contains("pub fn build_run_command(args: &RunArgs) -> Vec<String>"));
    assert!(rust.contains("pub fn run_command(args: &RunArgs) -> CommandInvocation"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn generate_to_writes_kotlin_file() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("clap_types_generate_to_kotlin_{unique}"));

    let mut cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let path = generate_to(
        Kotlin::new()
            .module_name("demo_bindings")
            .package_name("dev.claptypes.demo"),
        &mut cmd,
        "demo",
        &dir,
    )?;

    assert_eq!(path.file_name(), Some(OsStr::new("DemoBindings.kt")));
    let kotlin = fs::read_to_string(&path)?;
    assert!(kotlin.contains("package dev.claptypes.demo"));
    assert!(kotlin.contains("data class RunArgs("));
    assert!(kotlin.contains("fun buildRunCommand(args: RunArgs): List<String>"));
    assert!(kotlin.contains("fun runCommand(args: RunArgs, program: String = PROGRAM)"));

    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn embedded_binding_command_generates_rust_and_kotlin() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let rust_dir = std::env::temp_dir().join(format!("clap_types_binding_cli_rust_{unique}"));
    let kotlin_dir = std::env::temp_dir().join(format!("clap_types_binding_cli_kotlin_{unique}"));

    let rust_matches = binding_command().try_get_matches_from([
        "generate-binding",
        "rust",
        "--module-name",
        "demo_bindings",
        "--output-contracts",
        "--path",
        rust_dir
            .to_str()
            .ok_or("temp rust path was not valid UTF-8")?,
    ])?;
    let kotlin_matches = binding_command().try_get_matches_from([
        "generate-binding",
        "kotlin",
        "--module-name",
        "demo_bindings",
        "--package",
        "dev.claptypes.demo",
        "--path",
        kotlin_dir
            .to_str()
            .ok_or("temp kotlin path was not valid UTF-8")?,
    ])?;

    let mut rust_cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let rust_path = generate_binding_from_matches(&mut rust_cmd, "demo", &rust_matches)?;
    assert_eq!(rust_path.file_name(), Some(OsStr::new("demo_bindings.rs")));
    let rust = fs::read_to_string(&rust_path)?;
    assert!(rust.contains("pub const OUTPUT_CONTRACTS: &[OutputContract]"));

    let mut kotlin_cmd =
        Command::new("demo").subcommand(Command::new("run").arg(Arg::new("target").required(true)));
    let kotlin_path = generate_binding_from_matches(&mut kotlin_cmd, "demo", &kotlin_matches)?;
    assert_eq!(kotlin_path.file_name(), Some(OsStr::new("DemoBindings.kt")));
    let kotlin = fs::read_to_string(&kotlin_path)?;
    assert!(kotlin.contains("package dev.claptypes.demo"));
    assert!(!kotlin.contains("val OUTPUT_CONTRACTS: List<OutputContract>"));

    fs::remove_dir_all(&rust_dir)?;
    fs::remove_dir_all(&kotlin_dir)?;
    Ok(())
}

// =============================================================================
// `include_hidden` reflect option
// =============================================================================
//
// Default behavior must keep filtering hidden subcommands and args (matches
// `--help` visibility — the right default for OSS clients that mirror their
// CLI's public surface). Opt-in via `ReflectOptions::all()` (or the
// `--include-hidden` CLI flag) flips the filter so 1st-party hosts can call
// hidden surfaces of their embedded CLI.

/// Build a clap tree with mixed-visibility subcommands and args.
///
/// Three top-level subcommands (`pub-cmd`, `hidden-cmd` with `.hide(true)`,
/// `mixed` containing both kinds of arg + a hidden inner subcommand) plus
/// two top-level args (one visible, one hidden). Used by both default-path
/// and include-hidden tests below — keep them assertion-mirrored.
fn mixed_visibility_tree() -> Command {
    Command::new("demo")
        .arg(
            Arg::new("public_arg")
                .long("public-arg")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("hidden_arg")
                .long("hidden-arg")
                .hide(true)
                .action(ArgAction::Set),
        )
        .subcommand(Command::new("pub-cmd").about("public sub"))
        .subcommand(Command::new("hidden-cmd").about("hidden sub").hide(true))
        .subcommand(
            Command::new("mixed")
                .arg(Arg::new("v").long("v").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("h")
                        .long("h")
                        .hide(true)
                        .action(ArgAction::SetTrue),
                )
                .subcommand(Command::new("inner-hidden").hide(true)),
        )
}

#[test]
fn default_reflect_options_filter_hidden_subcommands_and_args() {
    let spec =
        reflect_command_with_options(mixed_visibility_tree(), "demo", ReflectOptions::default());

    let names: Vec<&str> = spec
        .root
        .subcommands
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, vec!["pub-cmd", "mixed"]);

    let arg_ids: Vec<&str> = spec.root.args.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(arg_ids, vec!["public_arg"]);

    let mixed = spec
        .root
        .subcommands
        .iter()
        .find(|c| c.name == "mixed")
        .expect("mixed subcommand present");
    let mixed_arg_ids: Vec<&str> = mixed.args.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(mixed_arg_ids, vec!["v"]);
    assert!(
        mixed.subcommands.is_empty(),
        "hidden inner subcommand should be filtered, got {:?}",
        mixed
            .subcommands
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn include_hidden_reflects_hidden_subcommands_and_args() {
    let spec = reflect_command_with_options(mixed_visibility_tree(), "demo", ReflectOptions::all());

    let names: Vec<&str> = spec
        .root
        .subcommands
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, vec!["pub-cmd", "hidden-cmd", "mixed"]);

    let arg_ids: Vec<&str> = spec.root.args.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(arg_ids, vec!["public_arg", "hidden_arg"]);

    let mixed = spec
        .root
        .subcommands
        .iter()
        .find(|c| c.name == "mixed")
        .expect("mixed subcommand present");
    let mixed_arg_ids: Vec<&str> = mixed.args.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(mixed_arg_ids, vec!["v", "h"]);
    assert_eq!(mixed.subcommands.len(), 1);
    assert_eq!(mixed.subcommands[0].name, "inner-hidden");
}

#[test]
fn reflect_command_with_name_default_excludes_hidden_for_backward_compat() {
    // Backward-compat guarantee: existing callers of the no-options public
    // API see no behavioral change. If this assertion ever fails, we've
    // silently broken every consumer that relies on hidden subcommands
    // being filtered (e.g. an OSS tool that wants its generated client to
    // mirror its `--help` surface).
    let spec = reflect_command_with_name(mixed_visibility_tree(), "demo");
    let names: Vec<&str> = spec
        .root
        .subcommands
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(
        !names.contains(&"hidden-cmd"),
        "default API must keep hidden filtering for backward compat, got {names:?}"
    );
}

#[test]
fn include_hidden_cli_flag_emits_hidden_in_generated_flow() -> Result<(), Box<dyn std::error::Error>>
{
    // End-to-end exercise of the `--include-hidden` CLI flag through the
    // binding_command dispatcher. Confirms the flag is plumbed from clap →
    // ReflectOptions → reflect → generated source. Picks Flow because it's
    // the consumer hzdb-bindings uses; the same plumbing covers all
    // backends since they share `generate_to_with_options`.
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir_default = std::env::temp_dir().join(format!("clap_types_default_{unique}"));
    let dir_hidden = std::env::temp_dir().join(format!("clap_types_hidden_{unique}"));

    // Default flag set (no --include-hidden).
    let matches_default = binding_command().try_get_matches_from([
        "generate-binding",
        "flow",
        "--module-name",
        "demo",
        "--path",
        dir_default.to_str().expect("temp dir path is valid UTF-8"),
    ])?;
    let mut cmd = mixed_visibility_tree();
    let path_default = generate_binding_from_matches(&mut cmd, "demo", &matches_default)?;
    let flow_default = fs::read_to_string(&path_default)?;
    assert!(
        !flow_default.contains("buildHiddenCmdCommand"),
        "default flow output should NOT include hidden subcommand builder"
    );
    assert!(
        flow_default.contains("buildPubCmdCommand"),
        "default flow output must include public subcommand builder"
    );

    // With --include-hidden.
    let matches_hidden = binding_command().try_get_matches_from([
        "generate-binding",
        "flow",
        "--include-hidden",
        "--module-name",
        "demo",
        "--path",
        dir_hidden.to_str().expect("temp dir path is valid UTF-8"),
    ])?;
    let mut cmd = mixed_visibility_tree();
    let path_hidden = generate_binding_from_matches(&mut cmd, "demo", &matches_hidden)?;
    let flow_hidden = fs::read_to_string(&path_hidden)?;
    assert!(
        flow_hidden.contains("buildHiddenCmdCommand"),
        "--include-hidden flow output must include hidden subcommand builder"
    );

    fs::remove_dir_all(&dir_default)?;
    fs::remove_dir_all(&dir_hidden)?;
    Ok(())
}
