use super::{Config, Result};
use crate::{error::CliError, TranspileUnit};
use fs_err as fs;
use itertools::Itertools;
use log::{error, info};
use serpent::{
    output::TranspiledFileKind, Transpile, TranspileConfig, TranspileFileBuilder,
    TranspileModuleBuilder, TranspiledFile,
};
use toml;

use std::{io::Write, path};

fn read_remap_file(
    path: impl AsRef<path::Path>,
) -> Result<(
    toml::map::Map<String, toml::Value>,
    toml::map::Map<String, toml::Value>,
)> {
    let path = path.as_ref();
    let remap_file = fs::read_to_string(path)?;

    let mut deps_and_remaps = match remap_file.parse::<toml::Value>()? {
        toml::Value::Table(table) => table,
        value => {
            error!("The remap-toml file has to parse into a table.");
            return Err(CliError::TomlContentError(value, "table"));
        }
    };

    let deps = match deps_and_remaps
        .remove("dependencies")
        .expect("dependencies not found in remap file")
    {
        toml::Value::Table(table) => table,
        value => {
            return Err(CliError::TomlContentError(value, "table"));
        }
    };
    let remaps: toml::map::Map<String, toml::Value> = deps_and_remaps.into();

    Ok((deps, remaps))
}

pub fn do_work(cfg: &Config) -> Result<()> {
    let t_cfg = TranspileConfig::default();

    match &cfg.transpile_unit {
        TranspileUnit::File(p) => {
            let deps_and_remaps = match &cfg.remap_file {
                Some(f) => Some(read_remap_file(f)?),
                None => None,
            };

            let transpiled = TranspileFileBuilder::new(p).config(t_cfg).transpile()?;
            let transpiled = if cfg.line_numbers {
                add_line_nbs(&transpiled.rust_target)
            } else {
                transpiled.rust_target.clone()
            };

            match &cfg.output {
                Some(out_file) => {
                    if let TranspileUnit::File(path) = out_file {
                        write_file(path, &transpiled)?;
                    } else {
                        // Unreachable because we verify that this is a file in `resolve_args`
                        unreachable!()
                    }
                }
                None => {
                    info!("Transpile result for {:?}:\n```\n{}\n```", p, transpiled);
                }
            }
        }
        TranspileUnit::Module(module_input_path) => {
            transpile_module(module_input_path, t_cfg, cfg)?;
        }
    }
    Ok(())
}

pub fn transpile_module(
    path: impl AsRef<path::Path>,
    t_cfg: TranspileConfig,
    cfg: &Config,
) -> Result<()> {
    let module_input_path = path.as_ref();

    let deps_and_remaps = match &cfg.remap_file {
        Some(f) => Some(read_remap_file(f)?),
        None => None,
    };

    let mut transpiled = TranspileModuleBuilder::new(&module_input_path)
        .config(t_cfg)
        .remap(remap_file)
        .transpile()?;

    // Add line numbers if necessary
    transpiled.files_mut().iter_mut().for_each(|file| {
        if cfg.line_numbers {
            file.content.rust_target = add_line_nbs(&file.content().rust_target);
        }
    });

    // Output module in a directory
    if let Some(output) = &cfg.output {
        let out_path = match output {
            TranspileUnit::File(_) => {
                // Unreachable because we verify that this is a module in `resolve_args`
                unreachable!()
            }
            TranspileUnit::Module(path) => path,
        };

        // Create the output directory
        let mod_out_path = path::Path::new(&out_path);
        if !mod_out_path.exists() {
            fs::create_dir(mod_out_path)?;
        }
        let src_out_path = mod_out_path.join("src");
        if !src_out_path.exists() {
            fs::create_dir(src_out_path)?;
        }

        let mut has_bin_target = false;
        let mut has_lib_target = false;

        // Translate output file names and output
        for TranspiledFile {
            source_path: in_path,
            content: transpiled,
            kind,
        } in transpiled.files()
        {
            let mut out_path = translate(in_path, module_input_path, mod_out_path);

            // Replace special file paths if detected
            match kind {
                TranspiledFileKind::LibRs => {
                    out_path.set_file_name("lib.rs");
                    has_lib_target = true;
                }
                TranspiledFileKind::MainRs => {
                    out_path.set_file_name("main.rs");
                    has_bin_target = true;
                }
                _ => {}
            };

            // Output into file
            info!("Transpiled {:?} into {:?}", &in_path, &out_path);
            write_file(out_path, &transpiled.rust_target)?;
        }

        // Create a manifest
        if cfg.create_manifest {
            let manifest_path = mod_out_path.join("Cargo.toml");
            if manifest_path.exists() {
                if cfg.overwrite_manifest {
                    info!("{:?} already exists, deleting previous", &manifest_path);
                    fs::remove_file(&manifest_path)?;
                } else {
                    info!(
                        "{:?} already exists, skipping because overwrite_manifest = false",
                        &manifest_path
                    );
                }
            }
            info!("Writing manifest into {:?}", &manifest_path);
            emit_manifest(mod_out_path, has_bin_target, has_lib_target)?;
        }
    }
    // Output in terminal
    else {
        for TranspiledFile {
            source_path: path,
            content: transpiled,
            ..
        } in transpiled.files()
        {
            println!(
                "Transpile result for {:?} in {:?}:\n```\n{}\n```",
                module_input_path, path, transpiled.rust_target
            );
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

fn emit_manifest(path: &path::Path, has_bin_target: bool, has_lib_target: bool) -> Result<()> {
    let path = path.canonicalize()?;

    let opts = cargo::ops::NewOptions::new(
        Some(cargo::ops::VersionControl::NoVcs),
        has_bin_target,
        has_lib_target,
        path.to_path_buf(),
        None,
        Some(String::from("2018")),
        None,
    )
    .map_err(|err| CliError::CargoError(err))?;

    cargo::ops::init(
        &opts,
        &cargo::Config::default().map_err(|err| CliError::CargoError(err))?,
    )
    .map_err(|err| CliError::CargoError(err))?;

    Ok(())
}

fn write_file<P>(path: P, contents: &str) -> Result<()>
where
    P: AsRef<path::Path>,
{
    let path = path.as_ref();

    // Create file
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    // Output into file
    file.write_all(contents.as_bytes())?;

    Ok(())
}

/// Replaces `from_stem` in `path` with `to_stem`, adds 'src/' and swaps ".py"
/// into ".rs"
fn translate(path: &path::Path, from_stem: &path::Path, to_stem: &path::Path) -> path::PathBuf {
    // Verify that the translation parameters are correct
    debug_assert!(path.starts_with(from_stem));

    // Unwrap should be safe, because we verify `starts_with` above, as documented in [struct.Path.html#method.strip_prefix](https://doc.rust-lang.org/std/path/struct.Path.html#method.strip_prefix)
    let relative = path.strip_prefix(from_stem).unwrap();
    let rs = relative.with_extension("rs");

    to_stem.join("src").join(rs)
}
