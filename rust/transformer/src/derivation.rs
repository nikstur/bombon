use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Derivation {
    pub path: String,
    pub name: Option<String>,
    pub pname: Option<String>,
    pub version: Option<String>,
    pub meta: Option<Meta>,
}

impl Derivation {
    pub fn new(store_path: &str) -> Self {
        // Because we only have the store path we have to derive the pname and version from it
        let stripped = store_path.strip_prefix("/nix/store/");
        let pname = stripped
            .and_then(|s| s.split('-').nth(1))
            .map(ToOwned::to_owned);
        let version = stripped
            .and_then(|s| s.split('-').last())
            .map(ToOwned::to_owned);

        Self {
            path: store_path.to_string(),
            pname,
            version,
            ..Self::default()
        }
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
