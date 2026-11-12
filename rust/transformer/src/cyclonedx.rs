use std::collections::{BTreeMap, BTreeSet};
use std::convert::Into;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use cyclonedx_bom::external_models::normalized_string::NormalizedString;
use cyclonedx_bom::external_models::spdx::SpdxExpression;
use cyclonedx_bom::external_models::uri::{Purl, Uri};
use cyclonedx_bom::models::attached_text::AttachedText;
use cyclonedx_bom::models::bom::{Bom, SpecVersion, UrnUuid};
use cyclonedx_bom::models::code::{Diff, Patch, PatchClassification, Patches};
use cyclonedx_bom::models::component::{Classification, Component, Components, Cpe, Scope};
use cyclonedx_bom::models::component::{
    ComponentEvidence, ConfidenceScore, Identity, IdentityField, Method, Methods, Pedigree,
};
use cyclonedx_bom::models::dependency::{Dependencies, Dependency};
use cyclonedx_bom::models::external_reference::{
    self, ExternalReference, ExternalReferenceType, ExternalReferences,
};
use cyclonedx_bom::models::hash::{Hash, HashAlgorithm, HashValue, Hashes};
use cyclonedx_bom::models::license::{License, LicenseChoice, Licenses};
use cyclonedx_bom::models::metadata::Metadata;
use cyclonedx_bom::models::tool::Tools;
use itertools::Itertools;
use sha2::{Digest, Sha256};

use crate::buildtime_input::BuildtimeInput;
use crate::derivation::{self, Derivation, Meta, Src};
use crate::graph::Graph;
use crate::hash::{self, SriHash};
use crate::runtime_input::RuntimeInput;

/// Strip the `/nix/store/` prefix to match how component bom-refs are derived from store paths.
fn bom_ref(store_path: &str) -> String {
    store_path
        .strip_prefix("/nix/store/")
        .unwrap_or(store_path)
        .to_string()
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct CycloneDXBom(Bom);

impl CycloneDXBom {
    /// Serialize to JSON as bytes.
    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json(&mut output, SpecVersion::V1_7)?;
        Ok(output)
    }

    /// Read a `CycloneDXBom` from a path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read(&path)
            .with_context(|| format!("Failed to open {}", path.as_ref().display()))?;
        let value = serde_json::from_slice(&content).context("Failed to parse cyclonedx")?;
        Ok(Self(Bom::parse_json_value(value)?))
    }

    pub fn build(
        target: Derivation,
        components: CycloneDXComponents,
        dependencies: CycloneDXDependencies,
        serial_number_seed: &str,
    ) -> Self {
        Self(Bom {
            components: Some(components.into()),
            dependencies: (!dependencies.0.0.is_empty()).then_some(dependencies.0),
            metadata: Some(metadata_from_derivation(target)),
            // Derive a reproducible serial number from the seed. When the seed is the Nix outPath
            // of the SBOM derivation, this works because the outPath is input addressed and thus
            // reproducible while still being unique to the SBOM.
            serial_number: Some(derive_serial_number(serial_number_seed.as_bytes())),
            ..Bom::default()
        })
    }
}

/// Derive a serial number from some arbitrary data.
///
/// This data is hashed with SHA256 and the first 16 bytes are used to create a UUID to serve as a
/// serial number.
fn derive_serial_number(data: &[u8]) -> UrnUuid {
    let hash = Sha256::digest(data);
    let array: [u8; 32] = hash.into();
    #[allow(clippy::expect_used)]
    let bytes = array[..16]
        .try_into()
        .expect("Failed to extract 16 bytes from SHA256 hash");
    let uuid = uuid::Builder::from_bytes(bytes).into_uuid();
    UrnUuid::from(uuid)
}

/// The components of a BOM, together with the origin of those created from a store path, keyed by
/// bom-ref.
pub struct CycloneDXComponents(Components, BTreeMap<String, Origin>);

/// What deduplication needs to know about the store path a component was created from.
struct Origin {
    drv_path: Option<String>,
    output_name: Option<String>,
    evaluated: bool,
}

/// Components with the same key are duplicates of each other.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum DuplicateKey {
    Derivation(String),
    StorePath(String),
    Purl(String),
}

impl CycloneDXComponents {
    pub fn from_derivations(derivations: impl IntoIterator<Item = Derivation>) -> Self {
        let mut origins = BTreeMap::new();
        let components = derivations
            .into_iter()
            .map(|derivation| {
                origins.insert(
                    bom_ref(&derivation.path),
                    Origin {
                        drv_path: derivation.drv_path.clone(),
                        output_name: derivation.output_name.clone(),
                        evaluated: derivation.meta.is_some(),
                    },
                );
                CycloneDXComponent::from_derivation(derivation).into()
            })
            .collect();
        Self(Components(components), origins)
    }

    /// Extend the `Components` with components read from multiple BOMs inside a directory.
    ///
    /// Returns the `dependencies` graphs declared by those BOMs, so the (language-level)
    /// edges they carry can be preserved in the aggregate SBOM.
    ///
    /// Each vendored BOM's root (`metadata.component`) describes the same artifact as the owning Nix derivation,
    /// but is never itself emitted as a component. Its bom-ref is therefore remapped to the owning derivation's
    /// bom-ref so the language-level graph connects to the store-path component instead of dangling on a root that nothing references.
    pub fn extend_from_directory(
        &mut self,
        path: impl AsRef<Path>,
        owner_path: &str,
    ) -> Result<VendoredDependencies> {
        let target_ref = bom_ref(owner_path);
        let mut m = BTreeMap::new();
        let mut dependencies = Vec::new();

        // Insert the components from the original SBOM
        for component in self.0.0.clone() {
            let key = component
                .bom_ref
                .clone()
                .unwrap_or_else(|| component.name.to_string());
            m.entry(key).or_insert(component);
        }

        // Add the components from the vendored SBOMs
        for entry in fs::read_dir(&path)
            .with_context(|| format!("Failed to read {}", path.as_ref().display()))?
            .flatten()
        {
            let bom = CycloneDXBom::from_file(entry.path())?;
            let root_ref = bom
                .0
                .metadata
                .as_ref()
                .and_then(|meta| meta.component.as_ref())
                .and_then(|component| component.bom_ref.clone());
            if let Some(components) = &bom.0.components {
                for component in &components.0 {
                    let key = component
                        .bom_ref
                        .clone()
                        .unwrap_or_else(|| component.name.to_string());
                    m.entry(key).or_insert_with(|| component.clone());
                }
            }
            if let Some(deps) = bom.0.dependencies {
                for mut dep in deps.0 {
                    if let Some(root) = &root_ref {
                        if dep.dependency_ref == *root {
                            dep.dependency_ref.clone_from(&target_ref);
                        }
                        for sub in &mut dep.dependencies {
                            if sub == root {
                                sub.clone_from(&target_ref);
                            }
                        }
                    }
                    dependencies.push(dep);
                }
            }
        }

        self.0.0 = m.into_values().collect();
        Ok(VendoredDependencies(dependencies))
    }

    /// The set of references of the components currently held.
    /// Keeps the dependency graph valid: edges to/from absent components are dropped.
    pub fn bom_refs(&self) -> BTreeSet<String> {
        self.0.0.iter().map(reference).collect()
    }

    /// Deduplicate components.
    ///
    /// The outputs of a derivation are merged into one component, preferring the `out` output and
    /// then one with evaluated metadata. Different derivations are kept apart, even if they share
    /// a name and version, and so are store paths whose derivation is unknown. All other components
    /// are deduplicated by their PURL, falling back to bom-refs, falling back to the name of the
    /// component.
    ///
    /// Returns the references of the removed components, each mapped to the reference of the
    /// component kept in its place.
    pub fn deduplicate(&mut self) -> BTreeMap<String, String> {
        let mut groups: Vec<Vec<Component>> = Vec::new();
        let mut group_of_key = BTreeMap::new();
        for component in std::mem::take(&mut self.0.0) {
            let group = *group_of_key
                .entry(self.duplicate_key(&component))
                .or_insert_with(|| {
                    groups.push(Vec::new());
                    groups.len() - 1
                });
            groups[group].push(component);
        }

        let mut aliases = BTreeMap::new();
        let mut kept = Vec::with_capacity(groups.len());
        for group in groups {
            let mut group = group.into_iter().sorted_by_key(|c| self.preference(c));
            let Some(representative) = group.next() else {
                continue;
            };
            for dropped in group {
                aliases.insert(reference(&dropped), reference(&representative));
            }
            kept.push(representative);
        }

        self.0.0 = kept;
        aliases
    }

    fn duplicate_key(&self, component: &Component) -> DuplicateKey {
        let reference = reference(component);
        match self.1.get(&reference) {
            Some(Origin {
                drv_path: Some(drv_path),
                ..
            }) => DuplicateKey::Derivation(drv_path.clone()),
            Some(_) => DuplicateKey::StorePath(reference),
            None => DuplicateKey::Purl(
                component
                    .purl
                    .as_ref()
                    .map_or(reference, std::string::ToString::to_string),
            ),
        }
    }

    /// Ordering among duplicates: the smallest is kept.
    fn preference(&self, component: &Component) -> (bool, bool) {
        self.1
            .get(&reference(component))
            .map_or((true, true), |origin| {
                (
                    origin.output_name.as_deref() != Some("out"),
                    !origin.evaluated,
                )
            })
    }
}

fn reference(component: &Component) -> String {
    component
        .bom_ref
        .clone()
        .unwrap_or_else(|| component.name.to_string())
}

impl From<CycloneDXComponents> for Components {
    fn from(value: CycloneDXComponents) -> Self {
        value.0
    }
}

/// The (language-level) dependency edges carried by vendored SBOMs, before they are
/// reconciled against the components that actually end up in the final BOM.
#[derive(Default)]
pub struct VendoredDependencies(Vec<Dependency>);

impl VendoredDependencies {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge in the edges from another set of vendored dependencies.
    pub fn extend(&mut self, other: VendoredDependencies) {
        self.0.extend(other.0);
    }
}

pub struct CycloneDXDependencies(Dependencies);

impl CycloneDXDependencies {
    /// Assemble the `CycloneDX` dependency graph. Three sources of edges are combined:
    ///   - the Nix runtime reference graph (from `exportReferencesGraph`), keyed by store path
    ///   - the Nix build-time reference graph (each derivation's direct build inputs)
    ///   - the language-level graphs carried by the vendored SBOMs, keyed by purl
    ///
    /// The graph is first built over all store paths and refs. Nodes of components removed by
    /// [`CycloneDXComponents::deduplicate`] are then merged into the components kept in their
    /// place (`aliases`), and finally the graph is restricted to the refs that actually exist in
    /// the final BOM, so it stays valid after filtering and deduplication.
    pub fn assemble(
        components: &CycloneDXComponents,
        target_derivation: &Derivation,
        runtime_input: &RuntimeInput,
        buildtime_input: &BuildtimeInput,
        include_buildtime_dependencies: bool,
        vendored_dependencies: VendoredDependencies,
        aliases: &BTreeMap<String, String>,
    ) -> Self {
        let root = bom_ref(&target_derivation.path);
        let mut present = components.bom_refs();
        present.insert(root.clone());

        let mut graph = Graph::default();
        for (path, references) in &runtime_input.references {
            graph.add_node(bom_ref(path));
            for reference in references {
                graph.add_edge(bom_ref(path), bom_ref(reference));
            }
        }
        if include_buildtime_dependencies {
            for derivation in buildtime_input.0.values() {
                graph.add_node(bom_ref(&derivation.path));
                for reference in &derivation.build_references {
                    graph.add_edge(bom_ref(&derivation.path), bom_ref(reference));
                }
            }
        }
        for dependency in vendored_dependencies.0 {
            graph.add_node(dependency.dependency_ref.clone());
            for sub in dependency.dependencies {
                graph.add_edge(dependency.dependency_ref.clone(), sub);
            }
        }

        // The root is described by `metadata.component`, so it must keep its own bom-ref even when
        // its component lost against a duplicate.
        let mut aliases = aliases.clone();
        aliases.remove(&root);
        graph.merge(&aliases);
        graph.restrict(&present);

        // Declare every component as a node, including leaves with no dependencies of
        // their own: the CycloneDX spec (as required by BSI TR-03183-2 §5.1) mandates
        // that such components appear as empty elements in the dependency graph.
        for reference in &present {
            graph.add_node(reference.clone());
        }

        Self(Dependencies(
            graph
                .into_inner()
                .into_iter()
                .map(|(dependency_ref, dependencies)| Dependency {
                    dependency_ref,
                    dependencies: dependencies.into_iter().collect(),
                    provides: Vec::new(),
                })
                .collect(),
        ))
    }
}

struct CycloneDXComponent(Component);

impl CycloneDXComponent {
    fn from_derivation(derivation: Derivation) -> Self {
        let name = match derivation.pname {
            Some(pname) => pname,
            None => derivation.name.unwrap_or_default(),
        };
        let version = derivation.version.unwrap_or_default();
        let mut component = Component::new(
            // Classification::Application is used as per specification when the type is not known
            // as is the case for dependencies from Nix
            Classification::Application,
            &name,
            &version,
            Some(bom_ref(&derivation.path)),
        );
        component.scope = Some(Scope::Required);
        component.purl = Purl::new("nix", &name, &version).ok();
        component.hashes = derivation.output_hash.and_then(|s| convert_hash(&s));

        let mut external_references = Vec::new();

        if let Some(src) = derivation.src
            && !src.urls.is_empty()
        {
            external_references.extend(convert_src(&src));
        }
        if let Some(meta) = derivation.meta {
            component.licenses = convert_licenses(&meta);
            component.description = meta.description.map(|s| NormalizedString::new(&s));
            if let Some(identifiers) = meta.identifiers {
                if let Some(cpe) = identifiers.cpe {
                    component.cpe = Some(Cpe::new(&cpe));
                } else if let Some(possible_cpes) = identifiers.possible_cpes {
                    component.evidence = cpes_to_evidence(&possible_cpes);
                }
            }
            if let Some(homepage) = meta.homepage {
                external_references.push(convert_homepage(&homepage));
            }
        }

        if !external_references.is_empty() {
            component.external_references = Some(ExternalReferences(external_references));
        }

        if !derivation.patches.is_empty() {
            component.pedigree = Some(Pedigree {
                ancestors: None,
                descendants: None,
                variants: None,
                commits: None,
                patches: Some(convert_patches(&derivation.patches)),
                notes: None,
            });
        }

        Self(component)
    }
}

impl From<CycloneDXComponent> for Component {
    fn from(value: CycloneDXComponent) -> Self {
        value.0
    }
}

fn string_to_url(s: &str) -> external_reference::Uri {
    external_reference::Uri::Url(Uri::new(s))
}

/// Convert licenses from a derivations meta field.
///
/// Converts a list of licenses to a list of licenses in the ``CycloneDX`` format.
///
/// Converts a single license to a SPDX expression to be able to accommodate the new compound
/// license types.
///
/// Assumes that compound licenses are never inside a list.
fn convert_licenses(meta: &Meta) -> Option<Licenses> {
    Some(Licenses(match &meta.license {
        Some(license) => match license {
            derivation::LicenseField::LicenseList(list) => {
                list.0.clone().into_iter().map(convert_license).collect()
            }
            derivation::LicenseField::LicenseExpression(expr) => {
                vec![LicenseChoice::Expression(SpdxExpression::new(&expr.0))]
            }
            derivation::LicenseField::String(_) => return None,
        },
        _ => return None,
    }))
}

fn convert_license(license: derivation::License) -> LicenseChoice {
    match license.spdx_id {
        Some(spdx_id) => LicenseChoice::License(License::license_id(&spdx_id)),
        None => LicenseChoice::License(License::named_license(&license.full_name)),
    }
}

fn cpes_to_evidence(possible_cpes: &[derivation::Cpe]) -> Option<ComponentEvidence> {
    if possible_cpes.is_empty() {
        return None;
    }
    let methods = Methods(
        possible_cpes
            .iter()
            .map(|cpe| Method {
                // Because we extract this information from the package definition.
                // See https://cyclonedx.org/guides/OWASP_CycloneDX-Authoritative-Guide-to-SBOM-en.pdf p.63
                technique: "manifest-analysis".to_string(),
                // Safety: division could panic here but we've already prevented len() from being 0 above.
                #[allow(clippy::cast_precision_loss)]
                confidence: ConfidenceScore::new(1.0 / possible_cpes.len() as f32),
                value: cpe.cpe.clone(),
            })
            .collect(),
    );
    // ComponentEvidence and Identity do not have Default implementations.
    Some(ComponentEvidence {
        identity: Some(vec![Identity {
            field: IdentityField::Cpe,
            methods: Some(methods),
            confidence: None,
            tools: None,
            concluded_value: None,
        }]),
        licenses: None,
        copyright: None,
        occurrences: None,
        callstack: None,
    })
}

fn convert_src(src: &Src) -> Vec<ExternalReference> {
    assert!(
        !src.urls.is_empty(),
        "src.urls must contain at least one value to generate ExternalReference",
    );
    assert!(
        !src.urls.iter().any(String::is_empty),
        "All urls in src.urls must not be empty strings to generate ExternalReference",
    );
    src.urls
        .iter()
        .map(|u| ExternalReference {
            external_reference_type: ExternalReferenceType::Vcs,
            url: string_to_url(u),
            comment: None,
            hashes: src.hash.clone().and_then(|s| convert_hash(&s)),
            properties: None,
        })
        .collect()
}

impl From<hash::Algorithm> for HashAlgorithm {
    fn from(value: hash::Algorithm) -> Self {
        match value {
            hash::Algorithm::Md5 => HashAlgorithm::MD5,
            hash::Algorithm::Sha1 => HashAlgorithm::SHA1,
            hash::Algorithm::Sha256 => HashAlgorithm::SHA_256,
            hash::Algorithm::Sha512 => HashAlgorithm::SHA_512,
        }
    }
}

fn convert_hash(s: &str) -> Option<Hashes> {
    // If it's not an SRI hash, we'll return None
    let sri_hash = SriHash::from_str(s).ok()?;
    let hash = Hash {
        content: HashValue(sri_hash.hex_digest()),
        alg: sri_hash.algorithm.into(),
    };
    Some(Hashes(vec![hash]))
}

fn convert_homepage(homepage: &str) -> ExternalReference {
    ExternalReference {
        external_reference_type: ExternalReferenceType::Website,
        url: string_to_url(homepage),
        comment: None,
        hashes: None,
        properties: None,
    }
}

fn metadata_from_derivation(derivation: Derivation) -> Metadata {
    Metadata {
        timestamp: None,
        tools: Some(metadata_tools()),
        authors: None,
        component: Some(CycloneDXComponent::from_derivation(derivation).into()),
        manufacture: None,
        supplier: None,
        licenses: None,
        properties: None,
        lifecycles: None,
        manufacturer: None,
        distribution_constraints: None,
    }
}

fn metadata_tools() -> Tools {
    let mut component = Component::new(Classification::Application, "bombon", VERSION, None);
    component.external_references = Some(ExternalReferences(vec![convert_homepage(
        "https://github.com/nikstur/bombon",
    )]));
    component.description = Some(NormalizedString::new(
        "Nix CycloneDX Software Bills of Materials (SBOMs)",
    ));
    component.licenses = Some(Licenses(vec![LicenseChoice::License(License::license_id(
        "MIT",
    ))]));

    Tools::Object {
        services: None,
        components: Some(Components(vec![component])),
    }
}

fn convert_patches(patches: &[String]) -> Patches {
    let cyclonedx_patches = patches
        .iter()
        .filter_map(|patch| fs::read_to_string(patch).ok())
        .map(|diff| Patch {
            // As we know nothing about the patch at this level, the safest is to assume that it's
            // unofficial
            patch_type: PatchClassification::Unofficial,
            diff: Some(Diff {
                text: Some(AttachedText {
                    content_type: Some(NormalizedString::new("text/plain")),
                    encoding: None,
                    content: diff,
                }),
                url: None,
            }),
            resolves: None,
        })
        .collect::<Vec<_>>();
    Patches(cyclonedx_patches)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(drv_path: &str, path: &str, output_name: &str) -> Derivation {
        Derivation {
            path: path.to_string(),
            drv_path: Some(drv_path.to_string()),
            pname: Some("openssl".to_string()),
            version: Some("3.6.4".to_string()),
            output_name: Some(output_name.to_string()),
            ..Derivation::default()
        }
    }

    fn vendored(bom_ref: &str) -> Component {
        let mut component = Component::new(
            Classification::Library,
            "serde",
            "1.0.219",
            Some(bom_ref.to_string()),
        );
        component.purl = Purl::new("cargo", "serde", "1.0.219").ok();
        component
    }

    #[test]
    fn deduplicate_merges_outputs_of_a_derivation() {
        let drv = "/nix/store/x9fcdk0wv2glz5n5ylx6wsjiq3b8zsvb-openssl-3.6.4.drv";
        let mut components = CycloneDXComponents::from_derivations([
            output(
                drv,
                "/nix/store/44gxi8liiqmd6fbnr2b7bbxl0dkfb52m-openssl-3.6.4-bin",
                "bin",
            ),
            output(
                drv,
                "/nix/store/3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4",
                "out",
            ),
            output(
                drv,
                "/nix/store/0ah2dbbfsc7qmmrhqifaxgvc5zz7bvjp-openssl-3.6.4-dev",
                "dev",
            ),
        ]);

        let aliases = components.deduplicate();

        assert_eq!(
            components.bom_refs(),
            BTreeSet::from(["3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4".into()])
        );
        assert_eq!(
            aliases,
            BTreeMap::from([
                (
                    "44gxi8liiqmd6fbnr2b7bbxl0dkfb52m-openssl-3.6.4-bin".into(),
                    "3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4".into()
                ),
                (
                    "0ah2dbbfsc7qmmrhqifaxgvc5zz7bvjp-openssl-3.6.4-dev".into(),
                    "3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4".into()
                ),
            ])
        );
    }

    #[test]
    fn deduplicate_prefers_evaluated_metadata_without_out() {
        let drv = "/nix/store/x9fcdk0wv2glz5n5ylx6wsjiq3b8zsvb-openssl-3.6.4.drv";
        let bin = output(
            drv,
            "/nix/store/44gxi8liiqmd6fbnr2b7bbxl0dkfb52m-openssl-3.6.4-bin",
            "bin",
        );
        let dev = Derivation {
            meta: Some(Meta {
                license: None,
                homepage: None,
                description: None,
                identifiers: None,
            }),
            ..output(
                drv,
                "/nix/store/0ah2dbbfsc7qmmrhqifaxgvc5zz7bvjp-openssl-3.6.4-dev",
                "dev",
            )
        };
        let mut components = CycloneDXComponents::from_derivations([bin, dev]);

        components.deduplicate();

        assert_eq!(
            components.bom_refs(),
            BTreeSet::from(["0ah2dbbfsc7qmmrhqifaxgvc5zz7bvjp-openssl-3.6.4-dev".into()])
        );
    }

    #[test]
    fn deduplicate_keeps_derivations_with_the_same_name_and_version_apart() {
        let mut components = CycloneDXComponents::from_derivations([
            output(
                "/nix/store/x9fcdk0wv2glz5n5ylx6wsjiq3b8zsvb-openssl-3.6.4.drv",
                "/nix/store/3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4",
                "out",
            ),
            output(
                "/nix/store/4kq2wh3gyjhkqzn8pkxw1dd8q5c1v7sv-openssl-static-x86_64-unknown-linux-musl-3.6.4.drv",
                "/nix/store/ma4f8m4xrrd3hgs1pf3hcbrq4w0ssv1b-openssl-static-x86_64-unknown-linux-musl-3.6.4",
                "out",
            ),
        ]);

        let aliases = components.deduplicate();

        assert_eq!(components.bom_refs().len(), 2);
        assert!(aliases.is_empty());
    }

    #[test]
    fn deduplicate_keeps_store_paths_without_derivation_apart() {
        let mut components = CycloneDXComponents::from_derivations([
            Derivation::from_store_path(
                "/nix/store/3xcc0ljbmnp1nlk0pnjiqrbqzm7m3ag0-openssl-3.6.4",
            ),
            Derivation::from_store_path(
                "/nix/store/ma4f8m4xrrd3hgs1pf3hcbrq4w0ssv1b-openssl-3.6.4",
            ),
        ]);

        let aliases = components.deduplicate();

        assert_eq!(components.bom_refs().len(), 2);
        assert!(aliases.is_empty());
    }

    #[test]
    fn deduplicate_merges_vendored_components_by_purl() {
        let mut components = CycloneDXComponents::from_derivations([]);
        components.0.0.extend([
            vendored("registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219"),
            vendored("pkg:cargo/serde@1.0.219"),
        ]);

        let aliases = components.deduplicate();

        assert_eq!(
            components.bom_refs(),
            BTreeSet::from([
                "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219".into()
            ])
        );
        assert_eq!(
            aliases,
            BTreeMap::from([(
                "pkg:cargo/serde@1.0.219".into(),
                "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219".into()
            )])
        );
    }

    #[test]
    fn serial_number_from_seed() {
        let a = "/nix/store/bqwmxjkrkmn1kqivq4pr053j68biq4k4-hello-2.12.1.cdx.json";
        let b = "/nix/store/lcxn67gbjdcr6bjf1rcs03ywa7gcslr2-git-2.47.0.cdx.json";

        assert_eq!(
            derive_serial_number(a.as_bytes()),
            derive_serial_number(a.as_bytes())
        );
        assert_ne!(
            derive_serial_number(a.as_bytes()),
            derive_serial_number(b.as_bytes())
        );
    }
}
