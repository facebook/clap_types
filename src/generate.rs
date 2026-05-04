// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use clap::Command;

use crate::model::CliSpec;
use crate::reflect_command_with_name;

/// Controls whether generators emit output-contract metadata and parser helpers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputContractGeneration {
    /// Omit output contracts from generated code.
    #[default]
    Omit,
    /// Emit output-contract metadata and dependency-free framing helpers.
    Emit,
}

impl OutputContractGeneration {
    /// Return whether output contracts should be emitted.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Emit)
    }
}

/// A backend that renders a reflected clap command into a target language.
///
/// The public API mirrors crates like `clap_complete`: callers can generate to
/// any writer at runtime, or write a file from `build.rs` / `cargo xtask`.
pub trait Generator {
    /// Return the file name produced for `bin_name`.
    ///
    /// Single-file generators return a file name. Package generators return the
    /// primary package directory name.
    fn file_name(&self, bin_name: &str) -> String;

    /// Render the target language bindings for a reflected command.
    fn generate(&self, spec: &CliSpec, buf: &mut dyn Write) -> io::Result<()>;

    /// Render one or more files for a reflected command.
    ///
    /// The default implementation preserves the original single-file contract.
    /// Generators that want to emit a package can override this and return
    /// multiple relative paths.
    fn generate_files(&self, spec: &CliSpec) -> io::Result<Vec<GeneratedFile>> {
        let mut contents = Vec::<u8>::new();
        self.generate(spec, &mut contents)?;
        Ok(vec![GeneratedFile::new(
            self.file_name(&spec.bin_name),
            contents,
        )])
    }
}

/// A generated artifact to be written under an output directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Relative path below the caller-provided output directory.
    pub relative_path: PathBuf,
    /// File contents.
    pub contents: Vec<u8>,
}

impl GeneratedFile {
    /// Create a generated file from bytes.
    #[must_use]
    pub fn new(relative_path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) -> Self {
        Self {
            relative_path: relative_path.into(),
            contents: contents.into(),
        }
    }

    /// Create a UTF-8 generated file.
    #[must_use]
    pub fn text(relative_path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self::new(relative_path, contents.into().into_bytes())
    }
}

/// Generate bindings for `cmd` into `buf`.
pub fn generate<G, S>(
    generator: G,
    cmd: &mut Command,
    bin_name: S,
    buf: &mut dyn Write,
) -> io::Result<()>
where
    G: Generator,
    S: Into<String>,
{
    let spec = reflect_command_with_name(cmd.clone(), bin_name);
    generator.generate(&spec, buf)
}

/// Generate bindings for `cmd` under `out_dir`.
pub fn generate_to<G, S, P>(
    generator: G,
    cmd: &mut Command,
    bin_name: S,
    out_dir: P,
) -> io::Result<PathBuf>
where
    G: Generator,
    S: Into<String>,
    P: AsRef<Path>,
{
    let bin_name = bin_name.into();
    let out_dir = out_dir.as_ref();
    let spec = reflect_command_with_name(cmd.clone(), bin_name.clone());
    let files = generator.generate_files(&spec)?;

    fs::create_dir_all(out_dir)?;

    for generated in files {
        let path = safe_output_path(out_dir, &generated.relative_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(&generated.contents)?;
        file.flush()?;
    }

    Ok(out_dir.join(generator.file_name(&bin_name)))
}

fn safe_output_path(out_dir: &Path, relative_path: &Path) -> io::Result<PathBuf> {
    if relative_path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "generated file path cannot be empty",
        ));
    }

    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "generated file path `{}` must be relative and stay inside the output directory",
                relative_path.display()
            ),
        ));
    }

    Ok(out_dir.join(relative_path))
}
