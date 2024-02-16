use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::Result;

pub struct RuntimeInput(HashSet<String>);

impl RuntimeInput {
    pub fn build(path: &Path) -> Result<Self> {
        let file_content = fs::read_to_string(path)?;
        let set = HashSet::from_iter(file_content.lines().map(|x| x.to_owned()));
        Ok(Self(set))
    }

    pub fn contains(&self, value: &str) -> bool {
        self.0.contains(value)
    }
}
