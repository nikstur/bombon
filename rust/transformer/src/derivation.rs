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
    pub patches: Vec<String>,
}

impl Derivation {
    /// Create a `Derivation` from a store path.
    ///
    /// This can be used if we don't have any information besides the path itself.
    pub fn from_store_path(store_path: &str) -> Self {
        let mut name = None;
        let mut version = None;

        if let Some(s) = store_path.strip_prefix("/nix/store/") {
            let mut split = s.split('-');
            let last_element = split.next_back();
            version = last_element.and_then(|v| v.starts_with(char::is_numeric).then(|| v.into()));
            split.next();
            name = Some({
                let mut s = split.collect::<Vec<_>>();
                if version.is_none() {
                    s.push(last_element.unwrap_or_default());
                }
                s.join("-")
            });
        }

        Self {
            path: store_path.to_string(),
            name,
            version,
            ..Self::default()
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Meta {
    pub license: Option<LicenseField>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub identifiers: Option<Identifiers>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Identifiers {
    pub cpe: Option<String>,
    #[serde(rename = "possibleCPEs")]
    pub possible_cpes: Option<Vec<Cpe>>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Cpe {
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
    pub urls: Vec<String>,
    pub hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_derivation_from_store(store_path: &str, name: Option<&str>, version: Option<&str>) {
        let derivation = Derivation::from_store_path(store_path);
        assert_eq!(derivation.name, name.map(std::string::ToString::to_string));
        assert_eq!(
            derivation.version,
            version.map(std::string::ToString::to_string)
        );
    }

    #[test]
    fn derivation_from_store() {
        check_derivation_from_store(
            "/nix/store/bqwmxjkrkmn1kqivq4pr053j68biq4k4-mailcap-2.1.54",
            Some("mailcap"),
            Some("2.1.54"),
        );
        check_derivation_from_store(
            "/nix/store/bqwmxjkrkmn1kqivq4pr053j68biq4k4-mailcap-extra-2.1.54",
            Some("mailcap-extra"),
            Some("2.1.54"),
        );
        check_derivation_from_store(
            "/nix/store/lcxn67gbjdcr6bjf1rcs03ywa7gcslr2-system-units",
            Some("system-units"),
            None,
        );
        check_derivation_from_store(
            "/nix/store/13cll4sxzxxi2lxbpsg5nvfjyv7qsznr-groupdel.pam",
            Some("groupdel.pam"),
            None,
        );
    }
}
