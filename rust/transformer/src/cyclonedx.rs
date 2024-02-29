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

use crate::buildtime_input::{self, Derivation, Meta};

const VERSION: &str = env!("GIT_COMMIT");

pub struct CycloneDXBom(Bom);

impl CycloneDXBom {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_3(&mut output)?;
        Ok(output)
    }

    pub fn build(target: Derivation, components: CycloneDXComponents) -> Result<Self> {
        let output = Self(Bom {
            components: Some(components.into()),
            metadata: Some(metadata_from_derivation(target)),
            ..Bom::default()
        });
        Ok(output)
    }
}

pub struct CycloneDXComponents(Components);

impl CycloneDXComponents {
    pub fn new(derivations: impl IntoIterator<Item = Derivation>) -> Self {
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
            Some(UrnUuid::generate().to_string()),
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

fn convert_license(license: buildtime_input::License) -> LicenseChoice {
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
