use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::derivation::Derivation;

#[derive(Clone)]
pub struct BuildtimeInput(pub BTreeMap<String, Derivation>);

impl BuildtimeInput {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content =
            fs::read(path).with_context(|| format!("Failed to open {}", path.display()))?;
        let buildtime_input_json: Vec<Derivation> = serde_json::from_slice(&content)
            .with_context(|| format!("Failed to parse buildtime input at {}", path.display()))?;
        Ok(Self::from_derivations(buildtime_input_json))
    }

    /// Index derivations by store path. A derivation found by evaluation takes precedence over
    /// a metadata-only one for the same path.
    fn from_derivations(derivations: impl IntoIterator<Item = Derivation>) -> Self {
        let mut m = BTreeMap::new();
        for mut derivation in derivations {
            match m.entry(derivation.path.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(derivation);
                }
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut();
                    let metadata_from_entry = existing
                        .metadata_from_entry
                        .take()
                        .or(derivation.metadata_from_entry.take());
                    if !derivation.metadata_only {
                        *existing = derivation;
                    }
                    existing.metadata_from_entry = metadata_from_entry;
                }
            }
        }
        Self(m)
    }

    /// The `metadataFrom` entries none of whose outputs are in `closure`.
    pub fn unmatched_metadata_from(&self, closure: &BTreeSet<String>) -> Vec<&str> {
        let mut matched: BTreeMap<&str, bool> = BTreeMap::new();
        for derivation in self.0.values() {
            if let Some(entry) = &derivation.metadata_from_entry {
                *matched.entry(entry).or_default() |= closure.contains(&derivation.path);
            }
        }
        matched
            .into_iter()
            .filter(|(_, matched)| !matched)
            .map(|(entry, _)| entry)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluated(path: &str, version: &str) -> Derivation {
        Derivation {
            path: path.to_string(),
            version: Some(version.to_string()),
            ..Derivation::default()
        }
    }

    fn metadata_only(path: &str, metadata_from_entry: Option<&str>) -> Derivation {
        Derivation {
            path: path.to_string(),
            metadata_only: true,
            metadata_from_entry: metadata_from_entry.map(ToString::to_string),
            ..Derivation::default()
        }
    }

    #[test]
    fn evaluated_derivations_take_precedence_over_metadata_only() {
        let hello = "/nix/store/a-hello-1.0";
        for derivations in [
            [evaluated(hello, "1.0"), metadata_only(hello, Some(hello))],
            [metadata_only(hello, Some(hello)), evaluated(hello, "1.0")],
        ] {
            let buildtime_input = BuildtimeInput::from_derivations(derivations);
            let derivation = &buildtime_input.0[hello];
            assert!(!derivation.metadata_only);
            assert_eq!(derivation.version.as_deref(), Some("1.0"));
            assert_eq!(derivation.metadata_from_entry.as_deref(), Some(hello));
        }
    }

    #[test]
    fn metadata_from_entries_outside_the_closure_are_unmatched() {
        let hello = "/nix/store/a-hello-1.0";
        let hello_dev = "/nix/store/b-hello-1.0-dev";
        let curl = "/nix/store/c-curl-1.0";
        let buildtime_input = BuildtimeInput::from_derivations([
            metadata_only(hello, Some(hello)),
            metadata_only(hello_dev, Some(hello)),
            metadata_only(curl, Some(curl)),
            metadata_only("/nix/store/d-zlib-1.0", None),
        ]);

        let closure = BTreeSet::from([hello_dev.to_string()]);
        assert_eq!(buildtime_input.unmatched_metadata_from(&closure), [curl]);
        assert_eq!(
            buildtime_input.unmatched_metadata_from(&BTreeSet::new()),
            [hello, curl]
        );
    }
}
