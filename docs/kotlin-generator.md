# Kotlin Generator

The Kotlin backend targets Kotlin/JVM desktop clients. It emits plain `.kt` source
with no generated dependencies beyond the Kotlin and Java standard libraries.

## Shape

- one data class per command or subcommand when the command has arguments;
- one empty class for commands with no arguments;
- one enum class per clap possible-value argument;
- one `build...Command(args)` function per command;
- one `...Command(args, program = PROGRAM)` helper per command;
- a `CommandInvocation` wrapper with `argv()`, `processBuilder()`, and `start()`.

Kotlin options can set the file stem with `module_name` and a package declaration
with `package_name`. This keeps generated files easy to drop into a JVM desktop,
Compose Desktop, IntelliJ plugin, or other local automation project.

## Output Contracts

`Kotlin::new().output_contracts()` emits `OutputContract`, `OUTPUT_CONTRACTS`, and
`parseOutput`. The parser frames output without adding a JSON library: JSON is
returned as a string, JSON-lines output is split into non-empty line strings, and
text output is returned unchanged.

## Validation

CI installs the standalone Kotlin compiler, compiles every generated Kotlin fixture
with `-Werror`, `-Wextra`, progressive mode, and bytecode validation, then compiles
the generated bindings together with their matching `tests/generated/kotlin*_smoke.kt`
files and runs the resulting JVM jars.
