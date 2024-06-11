use std::convert::Into;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use cyclonedx_bom::external_models::normalized_string::NormalizedString;
use cyclonedx_bom::external_models::uri::{Purl, Uri};
use cyclonedx_bom::models::bom::{Bom, UrnUuid};
use cyclonedx_bom::models::component::{Classification, Component, Components, Scope};
use cyclonedx_bom::models::external_reference::{
    self, ExternalReference, ExternalReferenceType, ExternalReferences,
};
use cyclonedx_bom::models::hash::{Hash, HashAlgorithm, HashValue, Hashes};
use cyclonedx_bom::models::license::{License, LicenseChoice, Licenses};
use cyclonedx_bom::models::metadata::Metadata;
use cyclonedx_bom::models::tool::{Tool, Tools};
use sha2::{Digest, Sha256};

use crate::derivation::{self, Derivation, Meta, Src};
use crate::hash::{self, SriHash};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct CycloneDXBom(Bom);

impl CycloneDXBom {
    /// Serialize to JSON as bytes.
    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_4(&mut output)?;
        Ok(output)
    }

    /// Read a `CycloneDXBom` from a path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = fs::File::open(path)?;
        Ok(Self(Bom::parse_from_json(file)?))
    }

    pub fn build(target: Derivation, components: CycloneDXComponents, output: &Path) -> Self {
        Self(Bom {
            components: Some(components.into()),
            metadata: Some(metadata_from_derivation(target)),
            // Derive a reproducible serial number from the output path. This works because the Nix
            // outPath of the derivation is input addressed and thus reproducible.
            serial_number: Some(derive_serial_number(output.as_os_str().as_encoded_bytes())),
            ..Bom::default()
        })
    }

    fn components(self) -> Option<Components> {
        self.0.components
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
    pub fn extend_from_directory(&mut self, path: impl AsRef<Path>) -> Result<()> {
        for entry in fs::read_dir(&path)
            .with_context(|| format!("Failed to read {:?}", path.as_ref()))?
            .flatten()
        {
            let bom = CycloneDXBom::from_file(entry.path())?;
            if let Some(component) = bom.components() {
                self.0 .0.extend(component.0);
            }
        }
        Ok(())
    }
}

impl From<CycloneDXComponents> for Components {
    fn from(value: CycloneDXComponents) -> Self {
        value.0
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
            Some(
                derivation
                    .path
                    .strip_prefix("/nix/store/")
                    .unwrap_or(&derivation.path)
                    .to_string(),
            ),
        );
        component.scope = Some(Scope::Required);
        component.purl = Purl::new("nix", &name, &version).ok();

        let mut external_references = Vec::new();

        if let Some(src) = derivation.src {
            external_references.push(convert_src(&src));
        }
        if let Some(meta) = derivation.meta {
            component.licenses = convert_licenses(&meta);
            component.description = meta.description.map(|s| NormalizedString::new(&s));
            if let Some(homepage) = meta.homepage {
                external_references.push(convert_homepage(&homepage));
            }
        }

        if !external_references.is_empty() {
            component.external_references = Some(ExternalReferences(external_references));
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

fn convert_licenses(meta: &Meta) -> Option<Licenses> {
    Some(Licenses(match &meta.license {
        Some(license) => license
            .clone()
            .into_vec()
            .into_iter()
            .map(convert_license)
            .collect(),
        _ => return None,
    }))
}

fn convert_src(src: &Src) -> ExternalReference {
    ExternalReference {
        external_reference_type: ExternalReferenceType::Vcs,
        url: string_to_url(&src.url),
        comment: None,
        hashes: src.hash.clone().and_then(|s| convert_hash(&s)),
    }
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

fn convert_license(license: derivation::License) -> LicenseChoice {
    match license.spdx_id {
        Some(spdx_id) => LicenseChoice::License(License::license_id(&spdx_id)),
        None => LicenseChoice::License(License::named_license(&license.full_name)),
    }
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
        tools: Some(Tools::List(vec![Tool::new("nikstur", "bombon", VERSION)])),
        authors: None,
        component: Some(CycloneDXComponent::from_derivation(derivation).into()),
        manufacture: None,
        supplier: None,
        licenses: None,
        properties: None,
        lifecycles: None,
    }
}
