use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use itertools::Itertools;
use regex::RegexSet;

use crate::buildtime_input::BuildtimeInput;
use crate::cyclonedx::{
    CycloneDXBom, CycloneDXComponents, CycloneDXDependencies, VendoredDependencies,
};
use crate::derivation::Derivation;
use crate::runtime_input::RuntimeInput;

pub fn transform(
    include_buildtime_dependencies: bool,
    exclude: &[String],
    serial_number_seed: &str,
    target_path: &str,
    buildtime_input_path: &Path,
    runtime_input_path: &Path,
    output: &Path,
) -> Result<()> {
    let buildtime_input = BuildtimeInput::from_file(buildtime_input_path)?;
    let target_derivation = buildtime_input
        .0
        .get(target_path)
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!("Buildtime input doesn't contain target derivation: {target_path}")
        })?;

    let runtime_input = RuntimeInput::from_file(runtime_input_path)?;

    // Augment the runtime input with information from the buildtime input. The buildtime input,
    // however, is not a strict superset of the runtime input. This has to do with how we query the
    // buildinputs from Nix and how dependencies can "hide" in String Contexts.
    let runtime_derivations = runtime_input.paths.iter().map(|store_path| {
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
        .filter(|derivation| !runtime_input.paths.contains(&derivation.path))
        .unique_by(|d| d.name.clone().unwrap_or(d.path.clone()));

    let mut all_derivations = runtime_derivations.collect::<Vec<_>>();
    if include_buildtime_dependencies {
        all_derivations.extend(buildtime_derivations);
    }

    let set = RegexSet::new(exclude).context("Failed to build regex set from exclude patterns")?;

    let filtered_derivations = all_derivations
        .into_iter()
        // Filter out all doc and man outputs.
        .filter(|derivation| {
            !matches!(
                derivation
                    .output_name
                    .as_ref()
                    .unwrap_or(&String::new())
                    .as_ref(),
                "doc" | "man"
            )
        })
        // Filter out derivations that match one of the exclude patterns.
        .filter(|derivation| !set.is_match(&derivation.path))
        .filter(|derivation| derivation.version.is_some())
        .collect::<Vec<_>>();

    // Find the SBOMS to use for the vendored dependencies.
    let mut sboms_to_include = Vec::new();
    for derivation in &filtered_derivations {
        if let Some(sbom_path) = derivation.vendored_sbom.as_ref() {
            sboms_to_include.push((sbom_path.clone(), derivation.path.clone()));
        }
    }

    let mut components = CycloneDXComponents::from_derivations(filtered_derivations);

    // Augment the components with those retrieved from the `sbom` passthru attribute of the
    // derivations, preserving the (language-level) dependency edges those SBOMs declare.
    let mut vendored_dependencies = VendoredDependencies::new();
    for (sbom_path, derivation_path) in sboms_to_include {
        vendored_dependencies
            .extend(components.extend_from_directory(sbom_path, &derivation_path)?);
    }

    components.deduplicate();

    let dependencies = CycloneDXDependencies::assemble(
        &components,
        &target_derivation,
        &runtime_input,
        &buildtime_input,
        include_buildtime_dependencies,
        vendored_dependencies,
    );

    let bom = CycloneDXBom::build(
        target_derivation,
        components,
        dependencies,
        serial_number_seed,
    );
    let mut file = File::create(output)
        .with_context(|| format!("Failed to create file {}", output.display()))?;
    file.write_all(&bom.serialize()?)?;

    Ok(())
}
