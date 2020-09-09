mod error;
mod subcommand;

use crate::error::CliError;
use clap::{App, Arg, SubCommand};

use std::fs::metadata;
use std::path::{Path, PathBuf};

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

const PKG_NAME: &'static str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
const PKG_AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");

fn main() {
    let matches = App::new(PKG_NAME)
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about(PKG_DESCRIPTION)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("controls testing features")
                .version("1.3")
                .author("Someone E. <someone_else@other.com>")
                .arg(
                    Arg::with_name("debug")
                        .short("d")
                        .help("print debug information verbosely"),
                ),
        )
        .subcommand(subcommand::steps::app())
        .subcommand(subcommand::tp::app())
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let config = matches.value_of("config").unwrap_or("default.conf");

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("v") {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    // You can handle information about subcommands by requesting their matches by
    // name (as below), requesting just the name used, or both at the same time
    if let Some(matches) = matches.subcommand_matches("test") {
        if matches.is_present("debug") {
            println!("Printing debug info...");
        } else {
            println!("Printing normally...");
        }
    }

    if let Some(matches) = matches.subcommand_matches(subcommand::steps::name()) {
        subcommand::steps::run(&matches).unwrap();
    }

    if let Some(matches) = matches.subcommand_matches(subcommand::tp::name()) {
        subcommand::tp::run(&matches).unwrap();
    }
}

#[derive(Debug)]
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
}

pub fn generate_target(input: &str) -> Result<TranspileUnit> {
    let path = Path::new(input);

    if !path.exists() {
        return Err(CliError::FileOrDirectoryNotFound(input.to_owned()));
    }

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
