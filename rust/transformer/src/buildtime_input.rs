use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::derivation::Derivation;

#[derive(Clone)]
pub struct BuildtimeInput(pub HashMap<String, Derivation>);

impl BuildtimeInput {
    pub fn from_file(path: &Path) -> Result<Self> {
        let buildtime_input_json: Vec<Derivation> = serde_json::from_reader(
            fs::File::open(path).with_context(|| format!("Failed to open {path:?}"))?,
        )
        .context("Failed to parse buildtime input")?;
        let mut m = HashMap::new();
        for derivation in buildtime_input_json {
            m.insert(derivation.path.clone(), derivation);
        }
        Ok(Self(m))
    }
}
