use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct ClosureEntry {
    path: String,
    references: Vec<String>,
}

pub struct RuntimeInput {
    /// The set of store paths in the runtime closure.
    pub paths: BTreeSet<String>,
    /// For each store path, its direct runtime references (self-references removed).
    pub references: BTreeMap<String, Vec<String>>,
}

impl RuntimeInput {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let entries: Vec<ClosureEntry> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse runtime input at {}", path.display()))?;

        let mut paths = BTreeSet::new();
        let mut references = BTreeMap::new();
        for entry in entries {
            let refs: Vec<String> = entry
                .references
                .into_iter()
                .filter(|r| r != &entry.path)
                .collect();
            paths.insert(entry.path.clone());
            references.insert(entry.path, refs);
        }
        Ok(Self { paths, references })
    }
}
