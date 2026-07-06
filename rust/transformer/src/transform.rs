use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use cyclonedx_bom::models::dependency::{Dependencies, Dependency};
use itertools::Itertools;
use regex::RegexSet;

use crate::buildtime_input::BuildtimeInput;
use crate::cyclonedx::{CycloneDXBom, CycloneDXComponents};
use crate::derivation::Derivation;
use crate::runtime_input::RuntimeInput;

/// Strip the `/nix/store/` prefix to match how component bom-refs are derived from store paths.
fn bom_ref(store_path: &str) -> String {
    store_path
        .strip_prefix("/nix/store/")
        .unwrap_or(store_path)
        .to_string()
}

pub fn transform(
    include_buildtime_dependencies: bool,
    exclude: &[String],
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

    let all_derivations: Box<dyn Iterator<Item = Derivation>> = if include_buildtime_dependencies {
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
    let mut vendored_dependencies = Vec::new();
    for derivation in buildtime_input.0.values() {
        if let Some(sbom_path) = &derivation.vendored_sbom {
            let target_ref = bom_ref(&derivation.path);
            vendored_dependencies.extend(components.extend_from_directory(sbom_path, &target_ref)?);
        }
    }

    components.deduplicate();

    let dependencies = assemble_dependencies(
        &components,
        &target_derivation,
        &runtime_input,
        &buildtime_input,
        include_buildtime_dependencies,
        vendored_dependencies,
    );

    let bom = CycloneDXBom::build(target_derivation, components, dependencies, output);
    let mut file = File::create(output)
        .with_context(|| format!("Failed to create file {}", output.display()))?;
    file.write_all(&bom.serialize()?)?;

    Ok(())
}

// Assemble the CycloneDX dependency graph. Three sources of edges are combined:
//   - the Nix runtime reference graph (from exportReferencesGraph), keyed by store path
//   - the Nix build-time reference graph (each derivation's direct build inputs)
//   - the language-level graphs carried by the vendored SBOMs, keyed by purl
// Edges are restricted to refs that actually exist in the final BOM, so the graph stays
// valid after filtering and deduplication. Entries are merged by ref so no dependency_ref appears twice.
fn assemble_dependencies(
    components: &CycloneDXComponents,
    target_derivation: &Derivation,
    runtime_input: &RuntimeInput,
    buildtime_input: &BuildtimeInput,
    include_buildtime_dependencies: bool,
    vendored_dependencies: Vec<Dependency>,
) -> Dependencies {
    let mut present = components.bom_refs();
    present.insert(bom_ref(&target_derivation.path));

    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, references) in &runtime_input.references {
        let dependent = bom_ref(path);
        if !present.contains(&dependent) {
            continue;
        }
        let entry = graph.entry(dependent).or_default();
        for reference in references {
            let dependency = bom_ref(reference);
            if present.contains(&dependency) {
                entry.insert(dependency);
            }
        }
    }
    if include_buildtime_dependencies {
        for derivation in buildtime_input.0.values() {
            let dependent = bom_ref(&derivation.path);
            if !present.contains(&dependent) {
                continue;
            }
            let entry = graph.entry(dependent.clone()).or_default();
            for reference in &derivation.build_references {
                let dependency = bom_ref(reference);
                if dependency != dependent && present.contains(&dependency) {
                    entry.insert(dependency);
                }
            }
        }
    }
    for dependency in vendored_dependencies {
        if !present.contains(&dependency.dependency_ref) {
            continue;
        }
        let entry = graph.entry(dependency.dependency_ref).or_default();
        for sub in dependency.dependencies {
            if present.contains(&sub) {
                entry.insert(sub);
            }
        }
    }

    // Declare every component as a node, including leaves with no dependencies of
    // their own: the CycloneDX spec (as required by BSI TR-03183-2 §5.1) mandates
    // that such components appear as empty elements in the dependency graph.
    for reference in &present {
        graph.entry(reference.clone()).or_default();
    }

    Dependencies(
        graph
            .into_iter()
            .map(|(dependency_ref, dependencies)| Dependency {
                dependency_ref,
                dependencies: dependencies.into_iter().collect(),
            })
            .collect(),
    )
}
