use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::buildtime_input::BuildtimeInput;
use crate::cyclonedx::{CycloneDXBom, CycloneDXComponents};
use crate::runtime_input::RuntimeInput;

pub fn transform(
    include_buildtime_dependencies: bool,
    target_path: PathBuf,
    buildtime_input_path: PathBuf,
    runtime_input_path: PathBuf,
) -> Result<()> {
    let mut buildtime_input: BuildtimeInput =
        serde_json::from_reader(fs::File::open(buildtime_input_path)?)?;

    let target_derivation = buildtime_input.remove_derivation(
        target_path
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert path to string: {:?}", target_path))?,
    );

    let runtime_input = RuntimeInput::build(&runtime_input_path)?;

    let components = if include_buildtime_dependencies {
        CycloneDXComponents::new(buildtime_input.into_iter().unique())
    } else {
        CycloneDXComponents::new(
            buildtime_input
                .into_iter()
                .unique()
                .filter(|derivation| runtime_input.contains(&derivation.path)),
        )
    };

    let bom = CycloneDXBom::build(target_derivation, components)?;
    io::stdout().write_all(&bom.serialize()?)?;

    Ok(())
}
