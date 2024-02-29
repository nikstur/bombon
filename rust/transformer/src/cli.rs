use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::transform::transform;

#[derive(Parser)]
pub struct Cli {
    /// Include buildtime dependencies in output
    #[arg(long)]
    include_buildtime_dependencies: bool,

    /// Path to target derivation
    target: PathBuf,

    /// Path to JSON containing buildtime input
    buildtime_input: PathBuf,

    /// Path to JSON containing runtime input
    runtime_input: PathBuf,
}

impl Cli {
    pub fn call(self) -> Result<()> {
        transform(
            self.include_buildtime_dependencies,
            self.target,
            self.buildtime_input,
            self.runtime_input,
        )
    }
}
