use flexi_logger::FlexiLoggerError;
use thiserror::Error as ThisError;

use std::io;

/// An error that occurred while running the CLI.
#[derive(ThisError, Debug)]
pub enum CliError {
    #[error("Cannot set logger")]
    LogError(#[from] FlexiLoggerError),
    #[error("File or directory not found for {0:?}")]
    FileOrDirectoryNotFound(String),
    #[error("{0:?} is not a file")]
    PathIsDirectory(String),
    #[error("{0:?} is not a directory")]
    PathIsFile(String),
    #[error("Redundant parameter")]
    RedundantParameter(String),
    #[error("Transpiler error")]
    SerpentError(#[from] serpent::ApiError),
    #[error("cargo-toml-builder error")]
    CargoBuilderError(#[from] cargo_toml_builder::Error),
    #[error("TOML deserialization error")]
    TomlError(#[from] toml::de::Error),
    /// First is input, second is expected, eg. "table"
    #[error("TOML contents are not of expected format {0:?} should be '{}'")]
    TomlContentError(toml::Value, &'static str),
    /// An I/O error that occurred while reading or writing a file.
    #[error("IO error")]
    Io(#[from] io::Error),
}
