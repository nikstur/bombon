use std::borrow::ToOwned;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub struct RuntimeInput(pub HashSet<String>);

impl RuntimeInput {
    pub fn from_file(path: &Path) -> Result<Self> {
        let file_content =
            fs::read_to_string(path).with_context(|| format!("Failed to read {path:?}"))?;
        let set: HashSet<String> = file_content.lines().map(ToOwned::to_owned).collect();
        Ok(Self(set))
    }
}
