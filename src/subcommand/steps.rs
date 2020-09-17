//! Subcommand for showing the intermediate steps for transpiling a line in a
//! file or a module.
use crate::error::CliError;
use crate::{generate_target, TranspileUnit};

use std::fs::metadata;
use std::path::{Path, PathBuf};

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

/// Create the clap subcommand for `steps`.
pub fn app() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name(name())
        .about("shows transpilation steps for a line in INPUT, which is a module or a file")
        .arg(
            clap::Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        )
        .arg(
            clap::Arg::with_name("line")
                .short("l")
                .takes_value(true)
                .required(true)
                .help("show steps for this line"),
        )
        .arg(
            clap::Arg::with_name("module")
                .short("m")
                .takes_value(true)
                .help("transpile this module before resolving steps for the target line")
                .long_help(
                    "Transpile this module before resolving steps for the target line. Can be used to choose a module to be transpiled. The INPUT file will then be a file within the module.",
                ),
        )
}

/// Run the behavior of the `steps` subcommand.
pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    let cfg = resolve_args(matches)?;
    do_work(&cfg)
}

fn resolve_args(matches: &clap::ArgMatches) -> Result<Config> {
    // Calling .unwrap() is safe here because "INPUT" is required
    let input = matches.value_of("INPUT").unwrap();

    // Verify existence
    let path = Path::new(input);
    if !path.exists() || !metadata(path).unwrap().is_file() {
        return Err(CliError::PathIsDirectory(input.to_owned()));
    }

    let module_opt = matches.value_of("module");

    // Generate the target that needs to be transpiled to get desired output
    let transpile_target = match module_opt {
        Some(module_path) => generate_target(module_path)?,
        None => generate_target(input)?,
    };

    // Resolve INPUT path to be relative to module root and verify the existence of
    // that file
    let input_path = match module_opt {
        Some(module_path) => {
            // Try to resolve the more absolute path first
            let path = Path::new(input);
            if path.exists() {
                Some(path.to_path_buf())
            }
            // Try to resolve as a relative path
            else {
                let module_path = Path::new(module_path);
                let file_path = module_path.join(path);
                if file_path.exists() {
                    Some(file_path.to_path_buf())
                } else {
                    return Err(CliError::FileOrDirectoryNotFound(input.to_owned()));
                }
            }
        }
        None => None,
    };

    // Calling .unwrap() is safe here because "line" is required
    let line_str = matches.value_of("line").unwrap();
    let line = line_str
        .parse::<usize>()
        .expect(&format!("cannot parse usize from {}", line_str));

    Ok(Config {
        transpile_module: transpile_target,
        target_file: input_path,
        line,
    })
}

pub fn name() -> &'static str {
    "steps"
}

struct Config {
    transpile_module: TranspileUnit,
    /// The target file relative to module root if separate from the transpiled
    /// module
    target_file: Option<PathBuf>,
    line: usize,
}

fn do_work(cfg: &Config) -> Result<()> {
    let trace = match &cfg.transpile_module {
        TranspileUnit::File(path) => {
            let transpiled = serpent::transpile_file(&path)?;
            transpiled.trace_steps_for_line(cfg.line)
        }
        TranspileUnit::Module(path) => {
            let transpiled = serpent::transpile_module(&path)?;
            // Unwrap is safe because target_file is required for steps for a module
            // transpilation
            let target_path = cfg.target_file.as_ref().unwrap();

            transpiled
                .file_by_file_path(&target_path)
                .unwrap()
                .trace_steps_for_line(cfg.line)
        }
    }?;

    println!("{}:\n{}\n", "Python source", trace[0]);
    println!("{}:\n{:?}\n", "Python AST", trace[1]);
    println!("{}:\n{:?}\n", "Rust AST", trace[2]);
    println!("{}:\n{}\n", "Rust source", trace[3]);

    Ok(())
}
