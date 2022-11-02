use std::collections::HashSet;

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
use itertools::Itertools;

use crate::buildtime_input::{BuildtimeInput, Derivation, Meta};

const VERSION: &str = env!("GIT_COMMIT");

pub struct CycloneDXBom(Bom);

impl CycloneDXBom {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_3(&mut output)?;
        Ok(output)
    }

    pub fn build(
        target: Derivation,
        buildtime_input: BuildtimeInput,
        runtime_input: Vec<&str>,
    ) -> Result<Self> {
        let owned_runtime_input = runtime_input.into_iter().map(|x| x.to_owned());

        let runtime_input_set = HashSet::from_iter(owned_runtime_input);

        let output = Self(Bom {
            components: Some(input_to_components(buildtime_input, runtime_input_set)),
            metadata: Some(metadata_from_derivation(target)),
            ..Bom::default()
        });
        Ok(output)
    }
}

fn input_to_components(
    buildtime_input: BuildtimeInput,
    runtime_input_set: HashSet<String>,
) -> Components {
    Components(
        buildtime_input
            .into_iter()
            .unique()
            .filter(|derivation| runtime_input_set.contains(&derivation.path))
            .map(derivation_to_component)
            .collect(),
    )
}

fn derivation_to_component(derivation: Derivation) -> Component {
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
    component
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

fn convert_license(license: crate::buildtime_input::License) -> LicenseChoice {
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
        component: Some(derivation_to_component(derivation)),
        manufacture: None,
        supplier: None,
        licenses: None,
        properties: None,
    }
}
