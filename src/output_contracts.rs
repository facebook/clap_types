// Copyright (c) Meta Platforms, Inc. and affiliates.

use clap::Command;
use clap::builder::CommandExt;

use crate::model::OutputEncoding;
use crate::model::OutputMode;
use crate::model::OutputSchema;
use crate::model::OutputSpec;

/// Output contracts attached to a clap command through clap's unstable extension API.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputContracts {
    contracts: Vec<OutputContract>,
}

impl CommandExt for OutputContracts {}

impl OutputContracts {
    /// Create an empty output-contract collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an output contract.
    #[must_use]
    pub fn with(mut self, contract: OutputContract) -> Self {
        self.contracts.push(contract);
        self
    }

    /// Add multiple output contracts.
    #[must_use]
    pub fn extend(mut self, contracts: OutputContracts) -> Self {
        self.contracts.extend(contracts.contracts);
        self
    }

    /// Add a buffered JSON stdout contract.
    #[must_use]
    pub fn json(type_name: impl Into<String>) -> Self {
        Self::new().with(OutputContract::json(type_name))
    }

    /// Add a JSON-lines stdout contract.
    #[must_use]
    pub fn json_lines(type_name: impl Into<String>) -> Self {
        Self::new().with(OutputContract::json_lines(type_name))
    }

    /// Add an explicitly unstructured text stdout contract.
    #[must_use]
    pub fn text(type_name: impl Into<String>) -> Self {
        Self::new().with(OutputContract::text(type_name))
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &OutputContract> {
        self.contracts.iter()
    }
}

/// A structured-output declaration attached to one command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputContract {
    /// Output stream encoding.
    pub encoding: OutputEncoding,
    /// Buffered, streaming, or interactive execution semantics.
    pub mode: OutputMode,
    /// Target-language type name or symbolic contract name.
    pub type_name: String,
    /// Optional schema payload for languages/runtimes that can consume it.
    pub schema: Option<OutputSchema>,
}

impl OutputContract {
    /// Declare one buffered JSON document on stdout.
    #[must_use]
    pub fn json(type_name: impl Into<String>) -> Self {
        Self {
            encoding: OutputEncoding::Json,
            mode: OutputMode::Buffered,
            type_name: type_name.into(),
            schema: None,
        }
    }

    /// Declare newline-delimited JSON records on stdout.
    #[must_use]
    pub fn json_lines(type_name: impl Into<String>) -> Self {
        Self {
            encoding: OutputEncoding::JsonLines,
            mode: OutputMode::Streaming,
            type_name: type_name.into(),
            schema: None,
        }
    }

    /// Declare explicitly unstructured text output.
    #[must_use]
    pub fn text(type_name: impl Into<String>) -> Self {
        Self {
            encoding: OutputEncoding::Text,
            mode: OutputMode::Buffered,
            type_name: type_name.into(),
            schema: None,
        }
    }

    /// Mark this command as interactive.
    #[must_use]
    pub fn interactive(mut self) -> Self {
        self.mode = OutputMode::Interactive;
        self
    }

    /// Attach a JSON Schema document.
    #[must_use]
    pub fn json_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(OutputSchema::JsonSchema(schema.into()));
        self
    }

    pub(crate) fn to_spec(&self, command_path: Vec<String>) -> OutputSpec {
        OutputSpec {
            command_path,
            encoding: self.encoding,
            mode: self.mode,
            type_name: self.type_name.clone(),
            schema: self.schema.clone(),
        }
    }
}

/// Convenience methods for attaching `clap_types` output contracts to clap commands.
pub trait ClapTypesCommandExt {
    /// Attach one output contract to this command.
    fn output_contract(self, contract: OutputContract) -> Self;

    /// Attach multiple output contracts to this command.
    fn output_contracts(self, contracts: OutputContracts) -> Self;
}

impl ClapTypesCommandExt for Command {
    fn output_contract(self, contract: OutputContract) -> Self {
        let contracts = self
            .get::<OutputContracts>()
            .cloned()
            .unwrap_or_default()
            .with(contract);
        self.add(contracts)
    }

    fn output_contracts(self, contracts: OutputContracts) -> Self {
        let contracts = self
            .get::<OutputContracts>()
            .cloned()
            .unwrap_or_default()
            .extend(contracts);
        self.add(contracts)
    }
}
