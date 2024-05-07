use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use itertools::Itertools;

use crate::buildtime_input::BuildtimeInput;
use crate::cyclonedx::{CycloneDXBom, CycloneDXComponents};
use crate::derivation::Derivation;
use crate::runtime_input::RuntimeInput;

pub fn transform(
    include_buildtime_dependencies: bool,
    target_path: &str,
    buildtime_input_path: &Path,
    runtime_input_path: &Path,
    output: &Path,
) -> Result<()> {
    let mut buildtime_input = BuildtimeInput::from_file(buildtime_input_path)?;
    let target_derivation = buildtime_input.0.remove(target_path).with_context(|| {
        format!("Buildtime input doesn't contain target derivation: {target_path}")
    })?;

    let mut runtime_input = RuntimeInput::from_file(runtime_input_path)?;
    runtime_input.0.remove(target_path);

    // Augment the runtime input with information from the buildtime input. The buildtime input,
    // however, is not a strict superset of the runtime input. This has to do with how we query the
    // buildinputs from Nix and how dependencies can "hide" in String Contexts.
    let runtime_derivations = runtime_input.0.iter().map(|store_path| {
        buildtime_input
            .0
            .get(store_path)
            .map(ToOwned::to_owned)
            .unwrap_or(Derivation::from_store_path(store_path))
    });

    let buildtime_derivations = buildtime_input
        .0
        .clone()
        .into_values()
        .filter(|derivation| !runtime_input.0.contains(&derivation.path))
        .unique_by(|d| d.name.clone().unwrap_or(d.path.clone()));

    let components = if include_buildtime_dependencies {
        let all_derivations = runtime_derivations.chain(buildtime_derivations);
        CycloneDXComponents::from_derivations(all_derivations)
    } else {
        CycloneDXComponents::from_derivations(runtime_derivations)
    };

    let bom = CycloneDXBom::build(target_derivation, components, output);
    let mut file =
        File::create(output).with_context(|| format!("Failed to create file {output:?}"))?;
    file.write_all(&bom.serialize()?)?;

    Ok(())
}
