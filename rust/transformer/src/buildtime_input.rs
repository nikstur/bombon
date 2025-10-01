use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::derivation::Derivation;

#[derive(Clone)]
pub struct BuildtimeInput(pub BTreeMap<String, Derivation>);

impl BuildtimeInput {
    pub fn from_file(path: &Path) -> Result<Self> {
        let buildtime_input_json: Vec<Derivation> = serde_json::from_reader(
            fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?,
        )
        .with_context(|| format!("Failed to parse buildtime input at {}", path.display()))?;
        let mut m = BTreeMap::new();
        for derivation in buildtime_input_json {
            m.insert(derivation.path.clone(), derivation);
        }
        Ok(Self(m))
    }
}
