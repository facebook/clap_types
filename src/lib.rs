// Copyright (c) Meta Platforms, Inc. and affiliates.

#![doc = include_str!("../README.md")]

mod bindings_cli;
mod codegen;
mod flow;
mod generate;
mod kotlin;
mod model;
#[cfg(feature = "unstable-output-contracts")]
mod output_contracts;
mod reflect;
mod rust;

pub mod python;
pub mod typescript;

pub use bindings_cli::{BINDING_COMMAND_NAME, binding_command, generate_binding_from_matches};
pub use flow::{Flow, FlowOptions, FlowRuntime, FlowValidation};
pub use generate::{GeneratedFile, Generator, OutputContractGeneration, generate, generate_to};
pub use kotlin::{Kotlin, KotlinOptions};
pub use model::{
    ArgKind, ArgSpec, CliSpec, CommandSpec, EnumValue, OutputEncoding, OutputMode, OutputSchema,
    OutputSpec, ValueArity, ValueSpec, ValueType,
};
#[cfg(feature = "unstable-output-contracts")]
pub use output_contracts::{ClapTypesCommandExt, OutputContract, OutputContracts};
pub use python::{Python, PythonOptions, PythonPackage};
pub use reflect::{reflect_command, reflect_command_with_name};
pub use rust::{Rust, RustOptions};
pub use typescript::{TypeScript, TypeScriptOptions, TypeScriptRuntime, TypeScriptValidation};
