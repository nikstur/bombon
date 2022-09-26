use std::convert::TryFrom;

use anyhow::Result;
use cyclonedx_bom::external_models::normalized_string::NormalizedString;
use cyclonedx_bom::models::bom::Bom;
use cyclonedx_bom::models::component::{Classification, Component, Components};
use cyclonedx_bom::models::license::{License, LicenseChoice, Licenses};
use itertools::Itertools;

use crate::input::{Derivation, Meta};

pub struct Output(Bom);

impl Output {
    pub fn serialize(self) -> Result<String> {
        let mut output = Vec::<u8>::new();
        self.0.output_as_json_v1_3(&mut output)?;
        Ok(String::from_utf8(output)?)
    }
}

impl TryFrom<crate::Input> for Output {
    type Error = anyhow::Error;

    fn try_from(value: crate::Input) -> Result<Self, Self::Error> {
        let output = Output(Bom {
            components: Some(input_to_components(value)),
            ..Bom::default()
        });
        Ok(output)
    }
}

fn input_to_components(input: crate::Input) -> Components {
    Components(
        input
            .into_iter()
            .unique()
            .map(derivation_to_component)
            .collect(),
    )
}

fn derivation_to_component(derivation: Derivation) -> Component {
    let mut component = Component::new(
        Classification::Library,
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
                .map(|license| LicenseChoice::License(License::named_license(&license)))
                .collect(),
            _ => return None,
        },
        _ => return None,
    }))
}
