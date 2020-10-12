use super::Result;
use crate::error::CliError;
use fs_err as fs;
use log::info;

use std::path;

pub fn create_manifest(
    path: impl AsRef<path::Path>,
    overwrite_previous: bool,
    has_bin_target: bool,
    has_lib_target: bool,
) -> Result<()> {
    let path = path.as_ref();
    let manifest_path = path.join("Cargo.toml");
    if manifest_path.exists() {
        if overwrite_previous {
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
    emit_manifest(path, has_bin_target, has_lib_target)
}

pub fn emit_manifest(path: &path::Path, has_bin_target: bool, has_lib_target: bool) -> Result<()> {
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
