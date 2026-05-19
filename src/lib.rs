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

pub use bindings_cli::BINDING_COMMAND_NAME;
pub use bindings_cli::binding_command;
pub use bindings_cli::generate_binding_from_matches;
pub use bindings_cli::generate_binding_from_matches_with_outputs;
pub use flow::Flow;
pub use flow::FlowOptions;
pub use flow::FlowRuntime;
pub use flow::FlowValidation;
pub use generate::GeneratedFile;
pub use generate::Generator;
pub use generate::OutputContractGeneration;
pub use generate::generate;
pub use generate::generate_to;
pub use generate::generate_to_with_options;
pub use kotlin::Kotlin;
pub use kotlin::KotlinOptions;
pub use model::ArgKind;
pub use model::ArgSpec;
pub use model::CliSpec;
pub use model::CommandSpec;
pub use model::EnumValue;
pub use model::OutputEncoding;
pub use model::OutputMode;
pub use model::OutputSchema;
pub use model::OutputSpec;
pub use model::ValueArity;
pub use model::ValueSpec;
pub use model::ValueType;
#[cfg(feature = "unstable-output-contracts")]
pub use output_contracts::ClapTypesCommandExt;
#[cfg(feature = "unstable-output-contracts")]
pub use output_contracts::OutputContract;
#[cfg(feature = "unstable-output-contracts")]
pub use output_contracts::OutputContracts;
pub use python::Python;
pub use python::PythonOptions;
pub use python::PythonPackage;
pub use reflect::ReflectOptions;
pub use reflect::reflect_command;
pub use reflect::reflect_command_with_name;
pub use reflect::reflect_command_with_options;
pub use rust::Rust;
pub use rust::RustOptions;
pub use typescript::TypeScript;
pub use typescript::TypeScriptOptions;
pub use typescript::TypeScriptRuntime;
pub use typescript::TypeScriptValidation;
