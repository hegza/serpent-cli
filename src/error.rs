use thiserror::Error as ThisError;

/// An error that occurred while running the CLI.
#[derive(ThisError, Debug)]
pub enum CliError {
    #[error("File or directory not found for {0:?}")]
    FileOrDirectoryNotFound(String),
    #[error("Transpiler error")]
    SerpentError(#[from] serpent::SerpentError),
}
