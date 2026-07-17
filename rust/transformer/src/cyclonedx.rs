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
use cyclonedx_bom::models::bom::{Bom, UrnUuid};
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
        self.0.output_as_json_v1_5(&mut output)?;
        Ok(output)
    }

    /// Read a `CycloneDXBom` from a path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = fs::File::open(path)?;
        Ok(Self(Bom::parse_from_json(file)?))
    }

    pub fn build(
        target: Derivation,
        components: CycloneDXComponents,
        dependencies: CycloneDXDependencies,
        output: &Path,
    ) -> Self {
        Self(Bom {
            components: Some(components.into()),
            dependencies: (!dependencies.0.0.is_empty()).then_some(dependencies.0),
            metadata: Some(metadata_from_derivation(target)),
            // Derive a reproducible serial number from the output path. This works because the Nix
            // outPath of the derivation is input addressed and thus reproducible.
            serial_number: Some(derive_serial_number(output.as_os_str().as_encoded_bytes())),
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

pub struct CycloneDXComponents(Components);

impl CycloneDXComponents {
    pub fn from_derivations(derivations: impl IntoIterator<Item = Derivation>) -> Self {
        Self(Components(
            derivations
                .into_iter()
                .map(CycloneDXComponent::from_derivation)
                .map(CycloneDXComponent::into)
                .collect(),
        ))
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
        self.0
            .0
            .iter()
            .map(|c| c.bom_ref.clone().unwrap_or_else(|| c.name.to_string()))
            .collect()
    }

    // Deduplicate components.
    //
    // Remove entries with duplicate PURLs, falling back to bom-refs, falling back to the name of
    // the component.
    pub fn deduplicate(&mut self) {
        self.0.0 = self
            .0
            .0
            .clone()
            .into_iter()
            .unique_by(|c: &Component| {
                c.purl.as_ref().map_or(
                    c.bom_ref.clone().unwrap_or(c.name.to_string()),
                    std::string::ToString::to_string,
                )
            })
            .collect();
    }
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
    /// Edges are restricted to refs that actually exist in the final BOM, so the graph stays
    /// valid after filtering and deduplication. Entries are merged by ref so no `dependency_ref`
    /// appears twice.
    pub fn assemble(
        components: &CycloneDXComponents,
        target_derivation: &Derivation,
        runtime_input: &RuntimeInput,
        buildtime_input: &BuildtimeInput,
        include_buildtime_dependencies: bool,
        vendored_dependencies: VendoredDependencies,
    ) -> Self {
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
        for dependency in vendored_dependencies.0 {
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

        Self(Dependencies(
            graph
                .into_iter()
                .map(|(dependency_ref, dependencies)| Dependency {
                    dependency_ref,
                    dependencies: dependencies.into_iter().collect(),
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
        identity: Some(Identity {
            field: IdentityField::Cpe,
            methods: Some(methods),
            confidence: None,
            tools: None,
        }),
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
