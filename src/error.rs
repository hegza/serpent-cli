use thiserror::Error as ThisError;

use std::io;

/// An error that occurred while running the CLI.
#[derive(ThisError, Debug)]
pub enum CliError {
    #[error("File or directory not found for {0:?}")]
    FileOrDirectoryNotFound(String),
    #[error("{0:?} is not a file")]
    PathIsDirectory(String),
    #[error("{0:?} is not a directory")]
    PathIsFile(String),
    #[error("Transpiler error")]
    SerpentError(#[from] serpent::SerpentError),
    /// An I/O error that occurred while reading or writing a file.
    #[error("IO error while reading Python source")]
    Io(#[from] io::Error),
}
