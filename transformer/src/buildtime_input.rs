use std::hash::{Hash, Hasher};
use std::vec::IntoIter;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BuildtimeInput(Vec<Derivation>);

impl BuildtimeInput {
    pub fn into_iter(self) -> IntoIter<Derivation> {
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

// Implement Eq and Hash so Itertools::unique can identify unique depdencies by name
impl PartialEq for Derivation {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for Derivation {}

impl Hash for Derivation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
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
}

impl LicenseField {
    pub fn into_vec(self) -> Vec<License> {
        match self {
            Self::LicenseList(license_list) => license_list.0,
            Self::License(license) => vec![license],
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
