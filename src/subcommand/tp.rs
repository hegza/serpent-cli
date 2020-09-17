//! Subcommand for transpiling files or modules.
use crate::error::CliError;
use crate::{generate_target, TranspileUnit};
use itertools::Itertools;

use std::io::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A type alias for `Result<T, crate::error::CliError>`.
pub type Result<T> = std::result::Result<T, CliError>;

/// Create the clap subcommand for `tp`.
pub fn app() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name(name()).alias("tp")
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
        .arg(
            clap::Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true)
                .help("Sets an output file or directory")
                .long_help(
                    "Sets an output file or directory. Needs to be the same kind as INPUT: file for an input file or a directory for an input module.",
                )
        )
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
        TranspileUnit::File(_) => TranspileUnit::File(Path::new(out_path).to_path_buf()),
        TranspileUnit::Module(_) => TranspileUnit::Module(Path::new(out_path).to_path_buf()),
    });

    let line_numbers = matches.is_present("lines");

    Ok(Config {
        transpile_unit: target,
        line_numbers,
        output,
    })
}

pub fn name() -> &'static str {
    "transpile"
}

struct Config {
    transpile_unit: TranspileUnit,
    line_numbers: bool,
    // The output file or module directory
    output: Option<TranspileUnit>,
}

fn do_work(cfg: &Config) -> Result<()> {
    match &cfg.transpile_unit {
        TranspileUnit::File(p) => {
            let transpiled = serpent::transpile_file(p)?;
            let transpiled = if cfg.line_numbers {
                add_line_nbs(&transpiled.rust_target)
            } else {
                transpiled.rust_target.clone()
            };

            match &cfg.output {
                Some(out_file) => {
                    if let TranspileUnit::File(path) = out_file {
                        // Create file
                        let mut file = fs::File::create(path)?;
                        // Output into file
                        println!("Writing into {:?}", &path);
                        file.write_all(transpiled.as_bytes())?;
                    } else {
                        // Unreachable because we verify that this is a file in `resolve_args`
                        unreachable!()
                    }
                }
                None => {
                    println!("Transpile result for {:?}:\n```\n{}\n```", p, transpiled);
                }
            }
        }
        TranspileUnit::Module(mod_in_path) => {
            let transpiled = serpent::transpile_module(mod_in_path)?;

            let transpiled_files = transpiled
                .files()
                .iter()
                .map(|file| {
                    if cfg.line_numbers {
                        (file.path(), add_line_nbs(&file.contents().rust_target))
                    } else {
                        (file.path(), file.contents().rust_target.clone())
                    }
                })
                .collect::<Vec<(&Path, String)>>();

            if let Some(output) = &cfg.output {
                let out_path = match output {
                    TranspileUnit::File(_) => {
                        // Unreachable because we verify that this is a module in `resolve_args`
                        unreachable!()
                    }
                    TranspileUnit::Module(path) => path,
                };
                // Create the output directory
                let mod_out_path = Path::new(out_path);
                fs::create_dir(mod_out_path)?;

                // Translate output file names and output
                for (in_path, transpiled) in transpiled_files {
                    let out_path = translate(in_path, mod_in_path, mod_out_path);
                    let mut file = fs::File::create(&out_path)?;
                    // Output into file
                    println!("Writing into {:?}", &out_path);
                    file.write_all(transpiled.as_bytes())?;
                }
            } else {
                // Output in terminal
                for (path, transpiled) in transpiled_files {
                    println!(
                        "Transpile result for {:?} in {:?}:\n```\n{}\n```",
                        mod_in_path, path, transpiled
                    );
                }
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

/// Replaces `from_stem` in `path` with `to_stem` and swaps ".py" into ".rs"
fn translate(path: &Path, from_stem: &Path, to_stem: &Path) -> PathBuf {
    // Verify that the translation parameters are correct
    debug_assert!(path.starts_with(from_stem));

    // Unwrap should be safe, because we verify `starts_with` above, as documented in [struct.Path.html#method.strip_prefix](https://doc.rust-lang.org/std/path/struct.Path.html#method.strip_prefix)
    let relative = path.strip_prefix(from_stem).unwrap();
    let rs = relative.with_extension("rs");

    to_stem.join(rs)
}
