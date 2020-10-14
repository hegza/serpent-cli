use crate::error::CliError;

use super::Result;
use fs_err as fs;
use log::info;
use toml::{map::Map as TomlMap, Value as TomlValue};

use std::path;

pub fn create_manifest(
    path: impl AsRef<path::Path>,
    overwrite_previous: bool,
    deps: Option<&TomlMap<String, TomlValue>>,
    bin_target: Option<impl AsRef<path::Path>>,
    lib_target: Option<impl AsRef<path::Path>>,
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
    emit_manifest(&manifest_path, deps, bin_target, lib_target)
}

pub fn emit_manifest(
    manifest_filepath: &path::Path,
    deps: Option<&TomlMap<String, TomlValue>>,
    bin_target: Option<impl AsRef<path::Path>>,
    lib_target: Option<impl AsRef<path::Path>>,
) -> Result<()> {
    use cargo_toml_builder::prelude::*;

    let mut cargo_toml = CargoToml::builder();
    cargo_toml.author("automatically transpiled by serpent");

    // Generate a name
    let name = format!(
        "{}",
        manifest_filepath
            .parent()
            .unwrap()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap(),
    );
    cargo_toml.name(&name);

    if let Some(deps) = deps {
        let deps = toml_into_deps(deps)?;
        cargo_toml.dependencies(&deps);
    }

    // Add bin target
    if let Some(target_path) = bin_target {
        let target_path = target_path.as_ref();

        // Extract stem as target name
        let name = target_path.file_stem().unwrap().to_str().unwrap();

        let target = BinTarget::new()
            .name(name)
            .path(target_path.to_str().unwrap())
            .build();
        cargo_toml.bin(target);
    }

    // Add lib target
    if let Some(target_path) = lib_target {
        let target_path = target_path.as_ref();

        // Extract stem as target name
        let name = target_path.file_stem().unwrap().to_str().unwrap();

        let target = LibTarget::new()
            .name(name)
            .path(target_path.to_str().unwrap())
            .build();
        cargo_toml.lib(target);
    }

    let content = format!("{}", cargo_toml.build()?);

    // Insert `edition = "2018"`
    let mut ncontent = vec![];
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        ncontent.push(line);
        ncontent.push("\n");
        if line.contains("[package]") {
            ncontent.push("edition =\"2018\"\n");
        }
    }
    let content = ncontent.concat();

    use super::write_file;
    write_file(manifest_filepath, &content)
}

use cargo_toml_builder::types::Dependency;
fn toml_into_deps(toml: &TomlMap<String, TomlValue>) -> Result<Vec<Dependency>> {
    toml.iter()
        .map(|(key, value)| match value {
            TomlValue::String(version) => Ok(Dependency::version(key, version)),
            val => return Err(CliError::TomlContentError(val.clone(), "String")),
        })
        .collect::<Result<Vec<Dependency>>>()
}
