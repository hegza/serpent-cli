//! Subcommand for transpiling files or modules.
mod cargo_util;
mod transpile;

use self::transpile::*;
use crate::error::CliError;
use crate::{generate_target, TranspileUnit};
use log::info;

use std::path;

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

/// Create the clap subcommand for `tp`.
pub fn app() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name(name()).alias("tp")
        .about("Transpiles INPUT which is a file or a module.")
        .arg(
            clap::Arg::with_name("INPUT")
                .help("sets the input module or file to transpile")
                .required(true)
                .index(1),
        )
        .arg(
            clap::Arg::with_name("lines")
                .long("lines")
                .short("l")
                .help("add line numbers to output"),
        )
        .arg(
            clap::Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true)
                .help("sets an output file or directory")
                .long_help(
                    "Sets an output file or directory. Needs to be the same kind as INPUT: file for an input file or a directory for an input module.",
                )
        )
        .arg(clap::Arg::with_name("omit-manifest").long("omit-manifest").help("omits Cargo.toml manifest from output"))
        .arg(clap::Arg::with_name("emit-manifest").long("emit-manifest").help("also emits Cargo.toml manifest").conflicts_with("omit-manifest"))
        .arg(clap::Arg::with_name("remap-file").long("remap-file").short("m").help("sets the toml file to be used for remapping").long_help("Sets the toml file to be used for remapping and dependencies. If omitted, Remap.toml will be auto-detected from INPUT. If not found, no remapping is used."))
        .arg(clap::Arg::with_name("no-remap").long("no-remap").help("do not auto-detect a Remap.toml").long_help("Explicitly avoid auto-detecting a Remap.toml-file from INPUT.").conflicts_with("remap-file"))
}

/// Run the behavior of the `tp` subcommand.
pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    // Collect a transpilation config at this point
    let cfg = resolve_args(matches)?;
    do_work(&cfg)
}

fn resolve_args(matches: &clap::ArgMatches) -> Result<Config> {
    // Calling .unwrap() is safe here because "INPUT" is required
    let input = matches.value_of("INPUT").unwrap();

    // Generate targets that need to be transpiled to get desired output
    let target = generate_target(input)?;

    let output = matches.value_of("output").map(|out_path| match target {
        TranspileUnit::File(_) => TranspileUnit::File(path::Path::new(out_path).to_path_buf()),
        TranspileUnit::Module(_) => TranspileUnit::Module(path::Path::new(out_path).to_path_buf()),
    });

    let line_numbers = matches.is_present("lines");
    let create_manifest = match (
        matches.is_present("emit-manifest"),
        matches.is_present("omit-manifest"),
    ) {
        (true, false) => true,
        (false, true) => false,
        (false, false) => false,
        (true, true) => unreachable!("should be eliminated by clap"),
    };

    // Assert that create manifest is used with modules only, and only when
    // outputting modules
    if create_manifest {
        match (&target, &output) {
            (TranspileUnit::Module(_), Some(TranspileUnit::Module(_))) => {}
            _ => {
                return Err(CliError::RedundantParameter(
                    "`omit-manifest` only makes sense when transpiling an input module into an output directory".to_owned(),
                ));
            }
        }
    }

    let remap_file=
    // Check input for a remap-file
    if let Some(path) = matches.value_of("remap-file") {
        let path = crate::to_path(path)?.to_path_buf();
        Some(path)
    }
    // else, try to auto-detect a remap-file
    else if !matches.is_present("no-remap") {
        match &target {
            TranspileUnit::File(fpath) => {
                if let Some(parent) = fpath.parent() {
                    detect("Remap.toml", parent)?
                } else {
                    // No parent for file -> return None
                    None
                }
            }
            TranspileUnit::Module(dirpath) => {
                detect("Remap.toml", &dirpath)?.or({
                    if let Some(parent) = dirpath.parent() {
                        detect("Remap.toml", parent)?
                    }
                    else {
                        None
                    }
                })
            }
        }
    }
    // else, do not use a remap file
    else {
        None
    };

    match &remap_file {
        Some(p) => info!("Using remap file: {:?}", p),
        None => info!("Not using a remap file"),
    }

    Ok(Config {
        transpile_unit: target,
        line_numbers,
        output,
        create_manifest,
        overwrite_manifest: true,
        remap_file,
    })
}

pub fn name() -> &'static str {
    "transpile"
}

pub struct Config {
    transpile_unit: TranspileUnit,
    line_numbers: bool,
    // The output file or module directory
    output: Option<TranspileUnit>,
    create_manifest: bool,
    // Should overwrite an existing manifest if found?
    overwrite_manifest: bool,
    remap_file: Option<path::PathBuf>,
}

/// Detects and returns the path of a file or a directory in the given path
fn detect(look_for: &str, in_dir: impl AsRef<path::Path>) -> Result<Option<path::PathBuf>> {
    let in_dir = in_dir.as_ref();
    if !in_dir.is_dir() {
        return Err(CliError::PathIsFile(format!("{:?}", in_dir)));
    }

    let tgt = in_dir.join(look_for);
    if tgt.exists() {
        Ok(Some(tgt))
    } else {
        Ok(None)
    }
}
