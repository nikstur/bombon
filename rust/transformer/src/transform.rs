use std::collections::BTreeSet;
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

/// What to include in the SBOM besides the runtime closure.
#[derive(Clone, Copy)]
pub struct Include {
    pub buildtime_dependencies: bool,
    pub static_libraries: bool,
}

pub fn transform(
    include: Include,
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

    // Add the libraries that static derivations in the runtime closure link in, and what those
    // libraries propagate in turn, as stdenv does when it builds against them.
    let mut linked_paths = BTreeSet::new();
    if include.static_libraries {
        let mut queue: Vec<&String> = runtime_input
            .paths
            .iter()
            .filter_map(|path| buildtime_input.0.get(path))
            .flat_map(|derivation| &derivation.host_inputs)
            .collect();
        while let Some(path) = queue.pop() {
            if runtime_input.paths.contains(path) || !linked_paths.insert(path.clone()) {
                continue;
            }
            if let Some(derivation) = buildtime_input.0.get(path) {
                queue.extend(&derivation.propagated_host_inputs);
            }
        }
    }

    // Augment the runtime input with information from the buildtime input. The buildtime input,
    // however, is not a strict superset of the runtime input. This has to do with how we query the
    // buildinputs from Nix and how dependencies can "hide" in String Contexts.
    let runtime_derivations = runtime_input
        .paths
        .iter()
        .chain(&linked_paths)
        .map(|store_path| {
            buildtime_input
                .0
                .get(store_path)
                .cloned()
                .unwrap_or_else(|| Derivation::from_store_path(store_path))
        });

    let buildtime_derivations = buildtime_input
        .0
        .clone()
        .into_values()
        .filter(|derivation| !runtime_input.paths.contains(&derivation.path))
        .unique_by(|d| d.name.clone().unwrap_or(d.path.clone()));

    let all_derivations: Box<dyn Iterator<Item = Derivation>> = if include.buildtime_dependencies {
        Box::new(runtime_derivations.chain(buildtime_derivations))
    } else {
        Box::new(runtime_derivations)
    };

    let set = RegexSet::new(exclude).context("Failed to build regex set from exclude patterns")?;

    let all_derivations = all_derivations
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
        .filter(|derivation| derivation.version.is_some());

    let mut components = CycloneDXComponents::from_derivations(all_derivations);

    // Augment the components with those retrieved from the `sbom` passthru attribute of the
    // derivations, preserving the (language-level) dependency edges those SBOMs declare.
    let mut vendored_dependencies = VendoredDependencies::new();
    for derivation in buildtime_input.0.values() {
        if let Some(sbom_path) = &derivation.vendored_sbom {
            vendored_dependencies
                .extend(components.extend_from_directory(sbom_path, &derivation.path)?);
        }
    }

    components.deduplicate();

    let dependencies = CycloneDXDependencies::assemble(
        &components,
        &target_derivation,
        &runtime_input,
        &buildtime_input,
        include,
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
