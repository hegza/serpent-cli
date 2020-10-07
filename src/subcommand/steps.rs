//! Subcommand for showing the intermediate steps in a transpilation operation.
use log::info;

use crate::{error::CliError, to_file_path_buf};
use crate::{generate_target, TranspileUnit};

use std::path::PathBuf;

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

/// Create the clap subcommand for `steps`.
pub fn app() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name(name())
        .about("Shows transpilation steps for given INPUT, which is a module or a file.")
        .arg(
            clap::Arg::with_name("INPUT")
                .help("Sets the input file or module to use")
                .required(true)
                .index(1),
        )
        .arg(
            clap::Arg::with_name("file")
                .short("-f")
                .takes_value(true)
                .help("show steps for this file only, also used in combination with --line to specify target file"),
        )
        .arg(
            clap::Arg::with_name("top")
                .short("t")
                .long("top")
                .help("show only top level nodes in output where applicable"),
        )
        .arg(
            clap::Arg::with_name("line")
                .short("l")
                .takes_value(true)
                .help("show steps for this line")
                .required_unless_one(&["top"]),
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
    let transpile_target = generate_target(input)?;

    let explicit_target_file = matches
        .value_of("file")
        .map(to_file_path_buf)
        .map_or(Ok(None), |v| v.map(Some))?;

    let top_only = matches.is_present("top");

    let line = matches.value_of("line").map(|line| {
        line.parse::<usize>()
            .expect(&format!("cannot parse usize from {}", line))
    });

    let target_file = explicit_target_file.or(match &transpile_target {
        TranspileUnit::File(p) => Some(p.clone()),
        TranspileUnit::Module(_) => None,
    });

    // Assert that --line is not used without a target
    if line.is_some() && target_file.is_none() {
        return Err(CliError::RedundantParameter(
            "`line` cannot be used without a specific target file".to_owned(),
        ));
    }

    Ok(Config {
        transpile_target,
        target_file,
        line,
        top_only,
    })
}

pub fn name() -> &'static str {
    "steps"
}

struct Config {
    transpile_target: TranspileUnit,
    /// The target file relative to module root if separate from the transpiled
    /// module
    target_file: Option<PathBuf>,
    line: Option<usize>,
    top_only: bool,
}

fn do_work(cfg: &Config) -> Result<()> {
    match &cfg.transpile_target {
        TranspileUnit::File(path) => {
            let transpiled = serpent::transpile_file(&path)?;

            let trace = if cfg.top_only {
                // "Top only" can show output for all lines
                transpiled.trace_top(cfg.line)
            } else {
                // .unwrap() is safe, because line is required when `top_only` is false
                let line = cfg.line.unwrap();
                transpiled.trace_steps_for_line(line, false)
            }?;

            print_trace(&trace);
        }
        TranspileUnit::Module(path) => {
            let transpiled = serpent::transpile_module(&path)?;

            let line = cfg.line;

            match &cfg.target_file {
                Some(p) => {
                    if cfg.top_only {
                        let trace = transpiled.file_by_file_path(&p).unwrap().trace_top(line)?;
                        print_trace(&trace);
                    } else {
                        // .unwrap() is safe, because line is required when `top_only` is false
                        let line = cfg.line.unwrap();
                        let trace = transpiled
                            .file_by_file_path(&p)
                            .unwrap()
                            .trace_steps_for_line(line, false)?;
                        print_trace(&trace);
                    }
                }
                None => {
                    // Trace all files
                    for tp_file in transpiled.files() {
                        let trace = tp_file.trace_top(None)?;

                        info!("Path: {:?}\n", tp_file.source_path());
                        print_trace(&trace);
                    }
                }
            }
        }
    };

    Ok(())
}

fn print_trace(trace: &[String]) {
    info!("{}:\n{}\n", "Python source", trace[0]);
    info!("{}:\n{}\n", "Python AST", trace[1]);
    info!("{}:\n{}\n", "Rust AST", trace[2]);
    info!("{}:\n{}\n", "Rust source", trace[3]);
}
