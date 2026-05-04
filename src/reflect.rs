// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::any::TypeId;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::{ValueParser, ValueRange};
use clap::{Arg, ArgAction, Command, ValueHint};

use crate::model::{
    ArgKind, ArgSpec, CliSpec, CommandSpec, EnumValue, OutputSpec, ValueArity, ValueSpec, ValueType,
};
#[cfg(feature = "unstable-output-contracts")]
use crate::output_contracts::OutputContracts;

/// Reflect a clap command into the language-neutral `clap_types` model.
#[must_use]
pub fn reflect_command(cmd: Command) -> CliSpec {
    let bin_name = cmd
        .get_bin_name()
        .unwrap_or_else(|| cmd.get_name())
        .to_owned();
    reflect_command_with_name(cmd, bin_name)
}

/// Reflect a clap command into the language-neutral model with an explicit binary name.
#[must_use]
pub fn reflect_command_with_name(mut cmd: Command, bin_name: impl Into<String>) -> CliSpec {
    cmd = cmd.disable_help_subcommand(true);
    cmd.build();
    CliSpec {
        bin_name: bin_name.into(),
        root: reflect_one_command(&cmd),
        outputs: output_specs(&cmd),
    }
}

#[cfg(feature = "unstable-output-contracts")]
fn output_specs(cmd: &Command) -> Vec<OutputSpec> {
    let mut specs = Vec::<OutputSpec>::new();
    let mut path = Vec::<String>::new();
    collect_output_specs(cmd, &mut path, &mut specs);
    specs
}

#[cfg(feature = "unstable-output-contracts")]
fn collect_output_specs(cmd: &Command, path: &mut Vec<String>, specs: &mut Vec<OutputSpec>) {
    if let Some(contracts) = cmd.get::<OutputContracts>() {
        specs.extend(
            contracts
                .iter()
                .map(|contract| contract.to_spec(path.clone())),
        );
    }

    for subcommand in cmd
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
    {
        path.push(subcommand.get_name().to_owned());
        collect_output_specs(subcommand, path, specs);
        path.pop();
    }
}

#[cfg(not(feature = "unstable-output-contracts"))]
fn output_specs(_cmd: &Command) -> Vec<OutputSpec> {
    Vec::new()
}

fn reflect_one_command(cmd: &Command) -> CommandSpec {
    CommandSpec {
        name: cmd.get_name().to_owned(),
        display_name: cmd.get_display_name().map(str::to_owned),
        about: cmd.get_about().map(ToString::to_string),
        long_about: cmd.get_long_about().map(ToString::to_string),
        args: cmd
            .get_arguments()
            .filter(|arg| should_reflect_arg(arg))
            .map(reflect_arg)
            .collect(),
        subcommands: cmd
            .get_subcommands()
            .filter(|subcommand| !subcommand.is_hide_set())
            .map(reflect_one_command)
            .collect(),
    }
}

fn should_reflect_arg(arg: &Arg) -> bool {
    if arg.is_hide_set() {
        return false;
    }

    !matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

fn reflect_arg(arg: &Arg) -> ArgSpec {
    let action = arg.get_action();
    let arity = value_arity(arg, action);
    let value_names = value_names(arg, arity);

    ArgSpec {
        id: arg.get_id().as_str().to_owned(),
        long: arg.get_long().map(str::to_owned),
        short: arg.get_short(),
        help: arg.get_help().map(ToString::to_string),
        long_help: arg.get_long_help().map(ToString::to_string),
        kind: arg_kind(arg, action),
        required: arg.is_required_set(),
        global: arg.is_global_set(),
        value: ValueSpec {
            names: value_names,
            ty: value_type(arg.get_value_parser()),
            arity,
            hint: value_hint(arg.get_value_hint()),
            repeated: matches!(action, ArgAction::Append | ArgAction::Count),
        },
        defaults: arg
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect(),
        possible_values: arg
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set() && !arg.is_hide_possible_values_set())
            .map(|value| EnumValue {
                name: value.get_name().to_owned(),
                help: value.get_help().map(ToString::to_string),
            })
            .collect(),
    }
}

fn arg_kind(arg: &Arg, action: &ArgAction) -> ArgKind {
    if arg.is_positional() {
        return ArgKind::Positional;
    }

    match action {
        ArgAction::SetTrue => ArgKind::FlagTrue,
        ArgAction::SetFalse => ArgKind::FlagFalse,
        ArgAction::Count => ArgKind::Counter,
        ArgAction::Set | ArgAction::Append => ArgKind::Option,
        _ if action.takes_values() => ArgKind::Option,
        _ => ArgKind::FlagTrue,
    }
}

fn value_arity(arg: &Arg, action: &ArgAction) -> ValueArity {
    let range = arg.get_num_args().unwrap_or_else(|| {
        if action.takes_values() || arg.is_positional() {
            ValueRange::SINGLE
        } else {
            ValueRange::EMPTY
        }
    });

    ValueArity {
        min: range.min_values(),
        max: usize_to_optional_max(range.max_values()),
    }
}

fn usize_to_optional_max(max: usize) -> Option<usize> {
    if max == usize::MAX { None } else { Some(max) }
}

fn value_names(arg: &Arg, arity: ValueArity) -> Vec<String> {
    match arg.get_value_names() {
        Some(names) => names.iter().map(ToString::to_string).collect(),
        None if arity.takes_values() => vec![arg.get_id().as_str().to_uppercase()],
        None => Vec::new(),
    }
}

fn value_hint(hint: ValueHint) -> Option<String> {
    match hint {
        ValueHint::Unknown => None,
        _ => Some(format!("{hint:?}")),
    }
}

fn value_type(parser: &ValueParser) -> ValueType {
    let id = parser.type_id();

    if id == TypeId::of::<String>() {
        ValueType::String
    } else if id == TypeId::of::<OsString>() {
        ValueType::OsString
    } else if id == TypeId::of::<PathBuf>() {
        ValueType::Path
    } else if id == TypeId::of::<bool>() {
        ValueType::Bool
    } else if is_small_integer_type(&id) {
        ValueType::Integer
    } else if is_big_integer_type(&id) {
        ValueType::BigInteger
    } else if id == TypeId::of::<f32>() || id == TypeId::of::<f64>() {
        ValueType::Float
    } else {
        ValueType::Unknown
    }
}

// Integers that always fit in JavaScript's 53-bit safe range, so backends can
// emit a plain numeric type without losing precision.
fn is_small_integer_type(id: &impl PartialEq<TypeId>) -> bool {
    *id == TypeId::of::<i8>()
        || *id == TypeId::of::<i16>()
        || *id == TypeId::of::<i32>()
        || *id == TypeId::of::<u8>()
        || *id == TypeId::of::<u16>()
        || *id == TypeId::of::<u32>()
}

// Integers that can exceed Number.MAX_SAFE_INTEGER. usize/isize are platform-
// dependent (32-bit on wasm32, 64-bit elsewhere); treat them conservatively as
// wide so generated bindings stay correct on 64-bit hosts.
fn is_big_integer_type(id: &impl PartialEq<TypeId>) -> bool {
    *id == TypeId::of::<i64>()
        || *id == TypeId::of::<i128>()
        || *id == TypeId::of::<isize>()
        || *id == TypeId::of::<u64>()
        || *id == TypeId::of::<u128>()
        || *id == TypeId::of::<usize>()
}

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, Command, value_parser};

    use super::reflect_command_with_name;
    use crate::model::{ArgKind, ValueType};

    #[cfg(feature = "unstable-output-contracts")]
    use crate::{ClapTypesCommandExt, OutputContract, OutputEncoding, OutputMode, OutputSchema};

    #[test]
    fn reflects_visible_args_and_subcommands() {
        let cmd = Command::new("demo")
            .arg(
                Arg::new("config")
                    .long("config")
                    .value_name("FILE")
                    .action(ArgAction::Set),
            )
            .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
            .arg(Arg::new("mode").long("mode").value_parser(["fast", "slow"]))
            .arg(
                Arg::new("threads")
                    .long("threads")
                    .value_parser(value_parser!(u16)),
            )
            .subcommand(Command::new("run").arg(Arg::new("target").required(true)));

        let spec = reflect_command_with_name(cmd, "demo");

        assert_eq!(spec.bin_name, "demo");
        assert_eq!(spec.root.args.len(), 4);
        assert_eq!(spec.root.args[0].id, "config");
        assert_eq!(spec.root.args[1].kind, ArgKind::Counter);
        assert_eq!(
            spec.root.args[2]
                .possible_values
                .iter()
                .map(|value| value.name.as_str())
                .collect::<Vec<_>>(),
            ["fast", "slow"]
        );
        assert_eq!(spec.root.args[3].value.ty, ValueType::Integer);
        assert_eq!(spec.root.subcommands[0].args[0].kind, ArgKind::Positional);
    }

    #[cfg(feature = "unstable-output-contracts")]
    #[test]
    fn reflects_unstable_output_contracts() {
        let cmd = Command::new("demo").subcommand(
            Command::new("watch").output_contract(
                OutputContract::json_lines("WatchEvent")
                    .json_schema(r#"{"type":"object","required":["event"]}"#),
            ),
        );

        let spec = reflect_command_with_name(cmd, "demo");

        assert_eq!(spec.outputs.len(), 1);
        assert_eq!(spec.outputs[0].command_path, ["watch"]);
        assert_eq!(spec.outputs[0].encoding, OutputEncoding::JsonLines);
        assert_eq!(spec.outputs[0].mode, OutputMode::Streaming);
        assert_eq!(spec.outputs[0].type_name, "WatchEvent");
        assert!(matches!(
            &spec.outputs[0].schema,
            Some(OutputSchema::JsonSchema(schema)) if schema.contains("\"required\"")
        ));
    }
}
