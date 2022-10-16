use std::collections::HashSet;

use anyhow::Result;
use cyclonedx_bom::models::bom::Bom;
use cyclonedx_bom::models::component::{Classification, Component, Components};
use cyclonedx_bom::models::external_reference::{
    ExternalReference, ExternalReferenceType, ExternalReferences,
};
use cyclonedx_bom::models::license::{License, LicenseChoice, Licenses};
use itertools::Itertools;

use crate::input::{Derivation, Meta};
use crate::BuildtimeInput;

pub struct Output(Bom);

impl Output {
    pub fn serialize(self) -> Result<String> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_3(&mut output)?;
        Ok(String::from_utf8(output)?)
    }

    pub fn convert(buildtime_input: BuildtimeInput, runtime_input: Vec<&str>) -> Result<Self> {
        let owned_runtime_input: Vec<String> =
            runtime_input.into_iter().map(|x| x.to_owned()).collect();
        let runtime_input_set = HashSet::from_iter(owned_runtime_input.into_iter());
        let output = Output(Bom {
            components: Some(input_to_components(buildtime_input, runtime_input_set)),
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
    let mut component = Component::new(
        // Classification::Application is used as per specification when the type is not known
        // as is the case for dependencies from Nix
        Classification::Application,
        &match derivation.pname {
            Some(pname) => pname,
            None => derivation.name.unwrap_or_default(),
        },
        &derivation.version.unwrap_or_default(),
        None,
    );
    if let Some(meta) = derivation.meta {
        component.licenses = convert_licenses(&meta);
        component.external_references = match convert_homepage(&meta) {
            Some(external_references) => Some(ExternalReferences(external_references)),
            None => None,
        };
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

fn convert_license(license: crate::input::License) -> LicenseChoice {
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
        _ => return None,
    }
}
