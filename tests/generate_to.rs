// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::ffi::OsStr;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Arg, Command};
use clap_types::{
    Flow, Kotlin, Python, Rust, TypeScript, binding_command, generate_binding_from_matches,
    generate_to,
};

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
    assert!(flow.contains("// @flow strict"));
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
    assert!(flow.contains("const parsed: any = DemoArgsSchema.parse(args);"));

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
