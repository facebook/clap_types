// Copyright (c) Meta Platforms, Inc. and affiliates.

//! Backend-agnostic helpers shared between language generators.

use std::collections::HashMap;
use std::io;

use crate::model::ArgKind;
use crate::model::ArgSpec;
use crate::model::CliSpec;
use crate::model::CommandSpec;
use crate::model::OutputEncoding;
use crate::model::OutputMode;
use crate::model::OutputSchema;

pub(crate) fn inherited_globals<'a>(
    inherited: &[&'a ArgSpec],
    command: &'a CommandSpec,
) -> Vec<&'a ArgSpec> {
    let mut globals = inherited.to_vec();
    for arg in command.args.iter().filter(|arg| arg.global) {
        if !globals.iter().any(|global| global.id == arg.id) {
            globals.push(arg);
        }
    }
    globals
}

pub(crate) fn combined_args<'a>(
    inherited: &[&'a ArgSpec],
    command: &'a CommandSpec,
) -> Vec<&'a ArgSpec> {
    let mut args = inherited.to_vec();
    for arg in &command.args {
        if !args.iter().any(|existing| existing.id == arg.id) {
            args.push(arg);
        }
    }
    args
}

// Boolean flags and counters always have a defined runtime value (false / 0)
// even when the user supplies nothing, so they are never "missing" — only
// value-bearing args can be required fields in the generated type.
pub(crate) fn is_required(arg: &ArgSpec) -> bool {
    arg.required
        && !matches!(
            arg.kind,
            ArgKind::FlagTrue | ArgKind::FlagFalse | ArgKind::Counter
        )
}

// `ArgAction::Append` combined with `num_args > 1` produces a CLI shape like
// `--pair a b --pair c d` where each occurrence is itself a group of values.
// A flat array can't preserve those occurrence boundaries, so backends emit
// a nested type and the runtime helper iterates groups.
pub(crate) fn is_grouped_repeated(arg: &ArgSpec) -> bool {
    matches!(arg.kind, ArgKind::Option) && arg.value.repeated && arg.value.arity.allows_multiple()
}

pub(crate) fn option_token(arg: &ArgSpec) -> Option<String> {
    arg.long
        .as_ref()
        .map(|long| format!("--{long}"))
        .or_else(|| arg.short.map(|short| format!("-{short}")))
}

pub(crate) fn output_schema(schema: &OutputSchema) -> &str {
    match schema {
        OutputSchema::JsonSchema(schema) => schema,
    }
}

pub(crate) fn output_encoding(encoding: OutputEncoding) -> &'static str {
    match encoding {
        OutputEncoding::Json => "json",
        OutputEncoding::JsonLines => "json-lines",
        OutputEncoding::Text => "text",
    }
}

pub(crate) fn output_mode(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Buffered => "buffered",
        OutputMode::Streaming => "streaming",
        OutputMode::Interactive => "interactive",
    }
}

pub(crate) fn capitalize(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        ),
        None => String::new(),
    }
}

pub(crate) fn pascal_case(input: &str) -> String {
    let output = words(input)
        .iter()
        .map(|word| capitalize(word))
        .collect::<String>();

    if output.is_empty() {
        "Command".to_owned()
    } else {
        output
    }
}

// Splits on non-alphanumerics AND on camelCase boundaries (a lower-or-digit
// followed by an uppercase letter), so `argName` becomes ["arg", "Name"].
pub(crate) fn words(input: &str) -> Vec<String> {
    let mut words = Vec::<String>::new();
    let mut current = String::new();
    let mut previous_was_lower_or_digit = false;

    for character in input.chars() {
        if !character.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous_was_lower_or_digit = false;
            continue;
        }

        if character.is_ascii_uppercase() && previous_was_lower_or_digit && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }

        previous_was_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
        current.push(character);
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

pub(crate) fn lower_join(input: &str, separator: &str) -> String {
    words(input)
        .iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(separator)
}

pub(crate) fn collapse_lines(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// Python and TypeScript both accept double-quoted strings with the same
// C-style escapes used here, so one quoting helper covers both backends.
pub(crate) fn quote_double(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    for character in input.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(crate) fn safe_identifier(input: &str, is_reserved: impl Fn(&str) -> bool) -> String {
    let mut identifier = if input
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
    {
        input.to_owned()
    } else {
        format!("_{input}")
    };

    if is_reserved(&identifier) {
        identifier.push('_');
    }

    identifier
}

pub(crate) fn command_pieces<'a>(command: &'a CommandSpec, path: &'a [String]) -> Vec<&'a str> {
    let mut pieces = if path.is_empty() {
        vec![command.name.as_str()]
    } else {
        path.iter().map(String::as_str).collect::<Vec<_>>()
    };

    if pieces.is_empty() {
        pieces.push("command");
    }

    pieces
}

pub(crate) fn command_type_prefix(command: &CommandSpec, path: &[String]) -> String {
    command_pieces(command, path)
        .iter()
        .map(|piece| pascal_case(piece))
        .collect::<String>()
}

/// Walk the command tree and reject specs where two distinct paths render to
/// the same generated prefix. Paths like `["index-foo"]` and
/// `["index", "foo"]` both pascal-case to `IndexFoo` (and snake-case to
/// `index_foo`) — silently emitting both would produce a TypeScript compile
/// error or a Python `class Foo` that overrides the previous definition.
pub(crate) fn ensure_unique_command_prefixes(spec: &CliSpec) -> io::Result<()> {
    let mut by_prefix: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut path = Vec::<String>::new();
    collect_prefixes(&spec.root, &mut path, &mut by_prefix);

    if let Some((prefix, paths)) = by_prefix.into_iter().find(|(_, paths)| paths.len() > 1) {
        let descriptions = paths
            .iter()
            .map(|path| format!("[{}]", path.join(", ")))
            .collect::<Vec<_>>()
            .join(" and ");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "command name collision: paths {descriptions} both render to `{prefix}`. \
                 Rename one of the subcommands so the generated type/function names are unique."
            ),
        ));
    }

    Ok(())
}

fn collect_prefixes(
    command: &CommandSpec,
    path: &mut Vec<String>,
    out: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let prefix = command_type_prefix(command, path);
    out.entry(prefix).or_default().push(path.clone());
    for sub in &command.subcommands {
        path.push(sub.name.clone());
        collect_prefixes(sub, path, out);
        path.pop();
    }
}
