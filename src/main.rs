mod error;
mod subcommand;

use crate::error::CliError;
use anyhow::Result;
use clap::{App, AppSettings, Arg};
use fs_err as fs;
use log::debug;

use fs::metadata;
use std::path::{Path, PathBuf};

const BIN_NAME: &'static str = env!("CARGO_BIN_NAME");
const PKG_NAME: &'static str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
const PKG_AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> Result<()> {
    let matches = App::new(PKG_NAME)
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about(PKG_DESCRIPTION)
        .arg(
            Arg::with_name("q")
                .short("q")
                .multiple(true)
                .help("Mutes all output, -qq mutes errors as well"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(subcommand::steps::app())
        .subcommand(subcommand::tp::app())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    let (mod_loglevel, all_loglevel) = match matches.occurrences_of("q") {
        1 => (log::LevelFilter::Error, log::LevelFilter::Error),
        2 => (log::LevelFilter::Off, log::LevelFilter::Off),
        _ =>
        // Vary the output based on how many times the user used the "verbose" flag
        // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
        {
            match matches.occurrences_of("v") {
                // Default is "info here", "error everywhere".
                0 => (log::LevelFilter::Info, log::LevelFilter::Error),
                // "-v" is "debug here", "warn everywhere".
                1 => (log::LevelFilter::Debug, log::LevelFilter::Warn),
                // "-vv" is "debug here", "info everywhere".
                2 => (log::LevelFilter::Debug, log::LevelFilter::Info),
                // "-vvv" is "trace here", "info everywhere".
                3 => (log::LevelFilter::Trace, log::LevelFilter::Info),
                // "-vvvv" is "trace everyhwere"
                4 | _ => (log::LevelFilter::Trace, log::LevelFilter::Trace),
            }
        }
    };

    /*SimpleLogger::new()
    .with_level(all_loglevel)
    .with_module_level("serpent", mod_loglevel)
    .init()?;*/
    let log_spec = flexi_logger::LogSpecification::default(all_loglevel)
        .module(&format!("{}::subcommand", BIN_NAME), mod_loglevel)
        .build();
    flexi_logger::Logger::with(log_spec).start()?;

    match mod_loglevel {
        log::LevelFilter::Off | log::LevelFilter::Error | log::LevelFilter::Warn => {}
        _ => debug!(
            "Log level is {:?} (local), {:?} (global)",
            mod_loglevel, all_loglevel
        ),
    }

    // Run subcommands
    if let Some(matches) = matches.subcommand_matches(subcommand::steps::name()) {
        subcommand::steps::run(&matches)?;
    }

    if let Some(matches) = matches.subcommand_matches(subcommand::tp::name()) {
        subcommand::tp::run(&matches)?;
    }

    Ok(())
}

/// Represents something that can be the input or an output of a transpilation
/// process eg. a directory / module, file or a string.
#[derive(Debug, Clone)]
pub enum TranspileUnit {
    File(PathBuf),
    Module(PathBuf),
}

impl TranspileUnit {
    pub fn path(&self) -> &PathBuf {
        match self {
            TranspileUnit::File(path) => &path,
            TranspileUnit::Module(path) => &path,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            TranspileUnit::File(_) => false,
            TranspileUnit::Module(_) => true,
        }
    }
}

/// Generates a transpile target from given input. Input can be a file or a
/// directory. Directories become module targets and files become file targets.
pub fn generate_target(input: &str) -> Result<TranspileUnit, CliError> {
    let path = to_path(input)?;

    // Unwrapping here is safe because we have verified that the file exists
    let md = metadata(path).unwrap();
    if md.is_dir() {
        Ok(TranspileUnit::Module(path.to_path_buf()))
    } else if md.is_file() {
        Ok(TranspileUnit::File(path.to_path_buf()))
    } else {
        // A path that exists is either a file or a directory
        unreachable!()
    }
}

/// Maps the input string to an existing directory path
pub fn to_dir_path_buf(input: &str) -> Result<PathBuf, CliError> {
    let path = to_path(input)?;

    // Unwrapping here is safe because we have verified that the file exists
    let md = metadata(path).unwrap();
    if md.is_dir() {
        Ok(path.to_path_buf())
    } else if md.is_file() {
        Err(CliError::PathIsFile(input.to_owned()))
    } else {
        // A path that exists is either a file or a directory
        unreachable!()
    }
}

/// Maps the input string to an existing file path
pub fn to_file_path_buf(input: &str) -> Result<PathBuf, CliError> {
    let path = to_path(input)?;

    // Unwrapping here is safe because we have verified that the file exists
    let md = metadata(path).unwrap();
    if md.is_dir() {
        Err(CliError::PathIsDirectory(input.to_owned()))
    } else if md.is_file() {
        Ok(path.to_path_buf())
    } else {
        // A path that exists is either a file or a directory
        unreachable!()
    }
}

/// Maps the input to an existing path.
pub fn to_path(input: &str) -> Result<&Path, CliError> {
    let path = Path::new(input);

    if !path.exists() {
        return Err(CliError::FileOrDirectoryNotFound(input.to_owned()));
    }

    Ok(path)
}
