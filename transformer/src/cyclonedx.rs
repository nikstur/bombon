use std::collections::HashSet;

use anyhow::Result;
use cyclonedx_bom::external_models::normalized_string::NormalizedString;
use cyclonedx_bom::models::bom::Bom;
use cyclonedx_bom::models::component::{Classification, Component, Components};
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
        match derivation.pname {
            Some(pname) => NormalizedString::new(&pname),
            None => NormalizedString::new(&derivation.name.unwrap_or_default()),
        },
        NormalizedString::new(&derivation.version.unwrap_or_default()),
        None,
    );
    component.licenses = extract_license(derivation.meta);
    component
}

fn extract_license(meta: Option<Meta>) -> Option<Licenses> {
    Some(Licenses(match meta {
        Some(meta) => match meta.license {
            Some(license) => license
                .into_vec()
                .into_iter()
                .map(license_to_license_choice)
                .collect(),
            _ => return None,
        },
        _ => return None,
    }))
}

fn license_to_license_choice(license: crate::input::License) -> LicenseChoice {
    match license.spdx_id {
        // cyclonedx-bom currently does not allow to create License using an SPDX identifier
        Some(spdx_id) => LicenseChoice::License(License::named_license(&spdx_id)),
        None => LicenseChoice::License(License::named_license(&license.full_name)),
    }
}
