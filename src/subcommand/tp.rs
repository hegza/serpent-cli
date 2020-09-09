//! Subcommand for transpiling files or modules.
use crate::error::CliError;
use crate::{generate_target, TranspileUnit};

use itertools::Itertools;
use std::fs::metadata;
use std::path::{Path, PathBuf};

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

/// Create the clap subcommand for `tp`.
pub fn app() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name(name())
        .about("transpiles INPUTS which are files or modules")
        .arg(
            clap::Arg::with_name("INPUT")
                .help("Sets the input module or file to transpile")
                .required(true)
                .index(1),
        )
        .arg(
            clap::Arg::with_name("lines")
                .long("lines")
                .help("Add line numbers to output"),
        )
}

/// Run the behavior of the `tp` subcommand.
pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    // Calling .unwrap() is safe here because "INPUT" is required
    let input = matches.value_of("INPUT").unwrap();

    // Generate targets that need to be transpiled to get desired output
    let targets = generate_target(input)?;

    let line_numbers = matches.is_present("lines");

    // Collect a transpilation config at this point
    let cfg = Config {
        transpile_unit: targets,
        line_numbers,
    };

    do_work(&cfg);

    Ok(())
}

pub fn name() -> &'static str {
    "tp"
}

struct Config {
    transpile_unit: TranspileUnit,
    line_numbers: bool,
}

fn do_work(cfg: &Config) -> Result<()> {
    match &cfg.transpile_unit {
        TranspileUnit::File(p) => {
            let out = serpent::transpile_file(p)?;
            let s = if cfg.line_numbers {
                add_line_nbs(&out.rust_target)
            } else {
                out.rust_target.clone()
            };

            println!("Transpile result for {:?}:\n```\n{}\n```", p, s);
        }
        TranspileUnit::Module(p) => {
            let out = serpent::transpile_module(p)?;

            for file in out.files() {
                let s = if cfg.line_numbers {
                    add_line_nbs(&file.contents().rust_target)
                } else {
                    file.contents().rust_target.clone()
                };
                println!(
                    "Transpile result for {:?} in {:?}:\n```\n{}\n```",
                    p,
                    file.path(),
                    s
                );
            }
        }
    }
    Ok(())
}

fn add_line_nbs(s: &str) -> String {
    let lines = s.lines();
    let line_count = lines.clone().count();
    let num_digits = if line_count == 0 {
        1
    } else {
        ((line_count as f64).log10() as usize) + 1
    };

    // Add line number at the beginning
    lines
        .enumerate()
        .map(|(line_idx, line)| {
            let line_no = format!("{:>1$}", line_idx + 1, num_digits);
            format!("{} {}", line_no, line,)
        })
        .join("\n")
}
