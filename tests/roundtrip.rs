// Copyright (c) Meta Platforms, Inc. and affiliates.

//! Round-trip integration tests: generate bindings → drive runtime → run binary
//! → assert clap parsed back the values the binding intended.
//!
//! Tests skip with a printed reason when their language runtime isn't on PATH,
//! so CI matrices that lack `python3`, `node`, or `tsc` still report green.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcCommand;
use std::sync::OnceLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use clap_types::Flow;
use clap_types::Python;
use clap_types::TypeScript;
use clap_types::generate_to;

#[path = "fixtures/echo_args.rs"]
mod echo_args;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    env::temp_dir().join(format!("clap_types_roundtrip_{label}_{nanos}"))
}

fn which(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    for dir in env::split_paths(&path) {
        let candidate = dir.join(format!("{program}{suffix}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn parse_kv(stdout: &str) -> HashMap<String, String> {
    stdout
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn fixture_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let manifest = env!("CARGO_MANIFEST_DIR");
        let status = ProcCommand::new(&cargo)
            .current_dir(manifest)
            .args(["build", "--example", "echo-args", "--quiet"])
            .status()
            .expect("invoke cargo");
        assert!(status.success(), "cargo build --example echo-args failed");

        let target_dir = env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(manifest).join("target"));
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        target_dir
            .join("debug")
            .join("examples")
            .join(format!("echo-args{suffix}"))
    })
    .as_path()
}

fn assert_expected_parse(parsed: &HashMap<String, String>) {
    assert_eq!(parsed.get("subcommand").map(String::as_str), Some("greet"));
    assert_eq!(parsed.get("workspace").map(String::as_str), Some("/tmp/ws"));
    assert_eq!(parsed.get("verbose").map(String::as_str), Some("2"));
    assert_eq!(parsed.get("name").map(String::as_str), Some("world"));
    assert_eq!(parsed.get("loud").map(String::as_str), Some("true"));
    assert_eq!(parsed.get("repeat").map(String::as_str), Some("3"));
    assert_eq!(parsed.get("tag").map(String::as_str), Some("a,b"));
    assert_eq!(parsed.get("mode").map(String::as_str), Some("fast"));
    assert_eq!(parsed.get("word").map(String::as_str), Some("hello,again"));
}

fn run_fixture(argv: &[String]) -> HashMap<String, String> {
    let output = ProcCommand::new(fixture_binary())
        .args(argv)
        .output()
        .expect("run echo-args");
    assert!(
        output.status.success(),
        "echo-args exited with {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    parse_kv(&String::from_utf8(output.stdout).expect("utf-8 stdout"))
}

fn local_node_bin(name: &str) -> Option<PathBuf> {
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("node_modules")
        .join(".bin")
        .join(format!("{name}{suffix}"));
    path.is_file().then_some(path)
}

fn strip_flow_file(strip_flow: &Path, input: &Path, output: &Path) {
    let stripped = ProcCommand::new(strip_flow)
        .arg(input)
        .output()
        .expect("run flow-remove-types");
    assert!(
        stripped.status.success(),
        "flow-remove-types failed: {}",
        String::from_utf8_lossy(&stripped.stderr)
    );
    fs::write(output, stripped.stdout).expect("write stripped Flow output");
}

#[test]
fn python_roundtrip_drives_clap_via_generated_bindings() {
    let Some(python) = which("python3") else {
        eprintln!("skipping python roundtrip: python3 not on PATH");
        return;
    };

    let dir = temp_dir("python");
    fs::create_dir_all(&dir).expect("create temp dir");

    let cmd = echo_args::cli();
    generate_to(
        Python::new().module_name("echo_args"),
        &cmd,
        "echo-args",
        &dir,
    )
    .expect("generate python bindings");

    let harness = r#"
import sys
sys.path.insert(0, sys.argv[1])
from echo_args import GreetArgs, build_greet_args

argv = build_greet_args(GreetArgs(
    workspace="/tmp/ws",
    verbose=2,
    name="world",
    loud=True,
    repeat=3,
    tag=["a", "b"],
    mode="fast",
    word=["hello", "again"],
))
print("\n".join(argv))
"#;
    let harness_path = dir.join("harness.py");
    fs::write(&harness_path, harness).expect("write harness.py");

    let output = ProcCommand::new(&python)
        .arg(&harness_path)
        .arg(&dir)
        .output()
        .expect("run python harness");
    assert!(
        output.status.success(),
        "python harness failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let argv = String::from_utf8(output.stdout)
        .expect("utf-8 harness stdout")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    assert_expected_parse(&run_fixture(&argv));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn typescript_roundtrip_drives_clap_via_generated_bindings() {
    let (Some(node), Some(tsc)) = (which("node"), which("tsc")) else {
        eprintln!("skipping typescript roundtrip: node and/or tsc not on PATH");
        return;
    };

    let dir = temp_dir("typescript");
    fs::create_dir_all(&dir).expect("create temp dir");

    let cmd = echo_args::cli();
    generate_to(TypeScript::new(), &cmd, "echo-args", &dir)
        .expect("generate typescript bindings");

    // Generator names the file `echo-args.ts` (kebab-cased from bin_name).
    // node16 module resolution requires explicit `.js` extension on imports;
    // tsc resolves it to the `.ts` source at compile time.
    let harness_ts = r#"
import { buildGreetCommand } from "./echo-args.js";

const argv = buildGreetCommand({
  workspace: "/tmp/ws",
  verbose: 2,
  name: "world",
  loud: true,
  repeat: 3,
  tag: ["a", "b"],
  mode: "fast",
  word: ["hello", "again"],
});
console.log(argv.join("\n"));
"#;
    fs::write(dir.join("harness.ts"), harness_ts).expect("write harness.ts");

    let tsc_status = ProcCommand::new(&tsc)
        .current_dir(&dir)
        .args([
            "--target",
            "es2022",
            "--module",
            "node16",
            "--moduleResolution",
            "node16",
            "--outDir",
            "dist",
            "harness.ts",
            "echo-args.ts",
        ])
        .status()
        .expect("run tsc");
    assert!(tsc_status.success(), "tsc failed");

    let output = ProcCommand::new(&node)
        .arg(dir.join("dist").join("harness.js"))
        .output()
        .expect("run node harness");
    assert!(
        output.status.success(),
        "node harness failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let argv = String::from_utf8(output.stdout)
        .expect("utf-8 harness stdout")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    assert_expected_parse(&run_fixture(&argv));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn flow_roundtrip_drives_clap_via_generated_bindings() {
    let (Some(node), Some(strip_flow)) = (which("node"), local_node_bin("flow-remove-types"))
    else {
        eprintln!("skipping flow roundtrip: node and/or flow-remove-types not available");
        return;
    };

    let dir = temp_dir("flow");
    fs::create_dir_all(&dir).expect("create temp dir");

    let cmd = echo_args::cli();
    generate_to(Flow::new(), &cmd, "echo-args", &dir).expect("generate flow bindings");

    let harness = r#"
// @flow strict

import { buildGreetCommand } from "./echo-args.js";

const argv = buildGreetCommand({
  workspace: "/tmp/ws",
  verbose: 2,
  name: "world",
  loud: true,
  repeat: 3,
  tag: ["a", "b"],
  mode: "fast",
  word: ["hello", "again"],
});
console.log(argv.join("\n"));
"#;
    let harness_path = dir.join("harness.js");
    fs::write(&harness_path, harness).expect("write harness.js");

    let dist = dir.join("dist");
    fs::create_dir_all(&dist).expect("create dist");
    strip_flow_file(
        &strip_flow,
        &dir.join("echo-args.js"),
        &dist.join("echo-args.js"),
    );
    strip_flow_file(&strip_flow, &harness_path, &dist.join("harness.js"));

    let output = ProcCommand::new(&node)
        .arg(dist.join("harness.js"))
        .output()
        .expect("run node harness");
    assert!(
        output.status.success(),
        "node harness failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let argv = String::from_utf8(output.stdout)
        .expect("utf-8 harness stdout")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    assert_expected_parse(&run_fixture(&argv));
    fs::remove_dir_all(&dir).ok();
}
