use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Derivation {
    pub path: String,
    pub name: Option<String>,
    pub pname: Option<String>,
    pub version: Option<String>,
    pub meta: Option<Meta>,
    pub output_name: Option<String>,
    pub output_hash: Option<String>,
    pub src: Option<Src>,
    pub vendored_sbom: Option<String>,
}

impl Derivation {
    /// Create a `Derivation` from a store path.
    ///
    /// This can be used if we don't have any information besides the path itself.
    pub fn from_store_path(store_path: &str) -> Self {
        // Because we only have the store path we have to derive the name from it
        let name = store_path.strip_prefix("/nix/store/").map(|s| {
            let mut split = s.split('-');
            split.next();
            split.join("-")
        });

        Self {
            path: store_path.to_string(),
            name,
            ..Self::default()
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Meta {
    pub license: Option<LicenseField>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub cpe: Option<String>,
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
            Self::String(_) => vec![],
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

#[derive(Deserialize, Clone, Debug)]
pub struct Src {
    pub url: Option<String>,
    pub hash: Option<String>,
}
