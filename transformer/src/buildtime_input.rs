use std::hash::{Hash, Hasher};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BuildtimeInput(Vec<Derivation>);

impl BuildtimeInput {
    pub fn remove_derivation(&mut self, derivation_path: &str) -> Derivation {
        let index = self
            .0
            .iter()
            .position(|derivation| derivation.path == derivation_path)
            .expect("Unrecovereable error: buildtime input does not include target");
        self.0.swap_remove(index)
    }
}

impl IntoIterator for BuildtimeInput {
    type Item = Derivation;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Derivation {
    pub path: String,
    pub name: Option<String>,
    pub pname: Option<String>,
    pub version: Option<String>,
    pub meta: Option<Meta>,
}

// Implement Eq and Hash so Itertools::unique can identify unique depdencies by path. The name
// seems to be the best proxy to detect duplicates. Different outputs of the same derivation have
// different paths. Thus, filtering by path alone doesn't adequately remove duplicates.
impl PartialEq for Derivation {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for Derivation {}

impl Hash for Derivation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Meta {
    pub license: Option<LicenseField>,
    pub homepage: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum LicenseField {
    LicenseList(LicenseList),
    License(License),
    // In very rare cases the license is just a String.
    // This mostly serves as a fallback so that serde doesn't panic.
    String(String),
}

impl LicenseField {
    pub fn into_vec(self) -> Vec<License> {
        match self {
            Self::LicenseList(license_list) => license_list.0,
            Self::License(license) => vec![license],
            // Fallback to handle very unusual license fields in Nix.
            _ => vec![],
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LicenseList(Vec<License>);

#[derive(Deserialize, Clone, Debug)]
pub struct License {
    #[serde(rename = "fullName")]
    pub full_name: String,
    #[serde(rename = "spdxId")]
    pub spdx_id: Option<String>,
}
