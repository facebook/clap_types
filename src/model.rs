// Copyright (c) Meta Platforms, Inc. and affiliates.

/// A reflected CLI surface that can be rendered into language bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSpec {
    /// The executable name consumers normally invoke.
    pub bin_name: String,
    /// The root clap command.
    pub root: CommandSpec,
    /// Optional structured-output declarations.
    pub outputs: Vec<OutputSpec>,
}

/// A command or subcommand in a clap command tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The command token used on the command line.
    pub name: String,
    /// Optional user-facing display name.
    pub display_name: Option<String>,
    /// Short command description.
    pub about: Option<String>,
    /// Long command description.
    pub long_about: Option<String>,
    /// Arguments accepted by this command.
    pub args: Vec<ArgSpec>,
    /// Child subcommands.
    pub subcommands: Vec<CommandSpec>,
}

/// A reflected clap argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgSpec {
    /// Stable clap argument id.
    pub id: String,
    /// Long option spelling without the `--` prefix.
    pub long: Option<String>,
    /// Short option spelling without the `-` prefix.
    pub short: Option<char>,
    /// Short help text.
    pub help: Option<String>,
    /// Long help text.
    pub long_help: Option<String>,
    /// Programmatic argument kind.
    pub kind: ArgKind,
    /// Whether clap requires this argument.
    pub required: bool,
    /// Whether this argument is global in clap.
    pub global: bool,
    /// Value shape accepted by clap.
    pub value: ValueSpec,
    /// Default values exposed by clap.
    pub defaults: Vec<String>,
    /// Enumerated values exposed by clap.
    pub possible_values: Vec<EnumValue>,
}

/// How a reflected argument behaves when building argv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    /// Boolean flag emitted when the generated value is `true`.
    FlagTrue,
    /// Boolean flag emitted when the generated value is `false`.
    FlagFalse,
    /// Count flag emitted one time per numeric count.
    Counter,
    /// Named option that accepts one or more values.
    Option,
    /// Positional argument.
    Positional,
}

/// Value metadata for an argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSpec {
    /// Names clap displays for values, such as `FILE` or `MODE`.
    pub names: Vec<String>,
    /// Best-effort cross-language value type.
    pub ty: ValueType,
    /// Number of values accepted per occurrence.
    pub arity: ValueArity,
    /// Optional completion-oriented value hint from clap.
    pub hint: Option<String>,
    /// Whether the option can occur multiple times.
    pub repeated: bool,
}

/// Portable value types visible through clap reflection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// Unknown parser type, rendered conservatively.
    Unknown,
    /// UTF-8 string-like value.
    String,
    /// OS string value.
    OsString,
    /// Filesystem path value.
    Path,
    /// Boolean value accepted as an option or positional value.
    Bool,
    /// Signed or unsigned integer that fits in JavaScript's safe-integer range
    /// (≤32-bit). Backends emit a plain numeric type.
    Integer,
    /// Wide integer (`i64`/`u64`/`i128`/`u128`/`isize`/`usize`) whose values
    /// can exceed JavaScript's `Number.MAX_SAFE_INTEGER` (`2^53 - 1`).
    /// Backends should preserve precision (e.g. `string | number` in TS) since
    /// stringifying a rounded float would emit the wrong CLI argument.
    BigInteger,
    /// Floating-point value.
    Float,
}

/// Number of values accepted by an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueArity {
    /// Minimum number of values.
    pub min: usize,
    /// Maximum number of values, or `None` when clap reports an unbounded range.
    pub max: Option<usize>,
}

impl ValueArity {
    /// Create an exact arity.
    #[must_use]
    pub const fn exact(count: usize) -> Self {
        Self {
            min: count,
            max: Some(count),
        }
    }

    /// Report whether the arity accepts any values.
    #[must_use]
    pub const fn takes_values(self) -> bool {
        self.max.is_none() || matches!(self.max, Some(max) if max > 0)
    }

    /// Report whether the arity accepts multiple values in one occurrence.
    #[must_use]
    pub const fn allows_multiple(self) -> bool {
        match self.max {
            Some(max) => max > 1,
            None => true,
        }
    }
}

/// An enumerated value accepted by a clap argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValue {
    /// The value spelling accepted on the command line.
    pub name: String,
    /// Optional help text for this value.
    pub help: Option<String>,
}

/// Structured-output metadata that future generators can render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpec {
    /// Command path this output contract applies to.
    pub command_path: Vec<String>,
    /// Output stream encoding.
    pub encoding: OutputEncoding,
    /// Runtime shape for consuming the output.
    pub mode: OutputMode,
    /// Target-language type name, schema name, or symbolic contract name.
    pub type_name: String,
    /// Optional schema payload for generators that can consume it.
    pub schema: Option<OutputSchema>,
}

/// A declared output encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputEncoding {
    /// A single JSON document on stdout.
    Json,
    /// Newline-delimited JSON records on stdout.
    JsonLines,
    /// Plain text with no structured parser contract.
    Text,
}

/// A declared output consumption mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Output is collected after the process exits.
    Buffered,
    /// Output is consumed incrementally while the process runs.
    Streaming,
    /// The command expects inherited stdio, a TTY, or another interactive channel.
    Interactive,
}

/// Optional schema metadata for an output contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputSchema {
    /// A JSON Schema document encoded as a string.
    JsonSchema(String),
}
