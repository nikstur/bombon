use std::path::Path;

use anyhow::Result;
use cyclonedx_bom::external_models::uri::Purl;
use cyclonedx_bom::models::bom::{Bom, UrnUuid};
use cyclonedx_bom::models::component::{Classification, Component, Components};
use cyclonedx_bom::models::external_reference::{
    ExternalReference, ExternalReferenceType, ExternalReferences,
};
use cyclonedx_bom::models::license::{License, LicenseChoice, Licenses};
use cyclonedx_bom::models::metadata::Metadata;
use cyclonedx_bom::models::tool::{Tool, Tools};
use sha2::{Digest, Sha256};

use crate::derivation::{self, Derivation, Meta};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct CycloneDXBom(Bom);

impl CycloneDXBom {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_4(&mut output)?;
        Ok(output)
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
        component.purl = Purl::new("nix", &name, &version).ok();
        if let Some(meta) = derivation.meta {
            component.licenses = convert_licenses(&meta);
            component.external_references = convert_homepage(&meta).map(ExternalReferences);
        }
        Self(component)
    }
}

impl From<CycloneDXComponent> for Component {
    fn from(value: CycloneDXComponent) -> Self {
        value.0
    }
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

fn convert_license(license: derivation::License) -> LicenseChoice {
    match license.spdx_id {
        Some(spdx_id) => match License::license_id(&spdx_id) {
            Ok(license) => LicenseChoice::License(license),
            Err(_) => LicenseChoice::License(License::named_license(&license.full_name)),
        },
        None => LicenseChoice::License(License::named_license(&license.full_name)),
    }
}

fn convert_homepage(meta: &Meta) -> Option<Vec<ExternalReference>> {
    match &meta.homepage {
        Some(homepage) => Some(vec![ExternalReference {
            external_reference_type: ExternalReferenceType::Website,
            url: match homepage.to_owned().try_into() {
                Ok(uri) => uri,
                _ => return None,
            },
            comment: None,
            hashes: None,
        }]),
        _ => None,
    }
}

fn metadata_from_derivation(derivation: Derivation) -> Metadata {
    Metadata {
        timestamp: None,
        tools: Some(Tools(vec![Tool::new("nikstur", "bombon", VERSION)])),
        authors: None,
        component: Some(CycloneDXComponent::from_derivation(derivation).into()),
        manufacture: None,
        supplier: None,
        licenses: None,
        properties: None,
    }
}
