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
    target: String,

    /// Path to JSON containing the buildtime input
    buildtime_input: PathBuf,

    /// Path to a newline separated .txt file containing the runtime input
    runtime_input: PathBuf,
}

impl Cli {
    pub fn call(self) -> Result<()> {
        transform(
            self.include_buildtime_dependencies,
            &self.target,
            &self.buildtime_input,
            &self.runtime_input,
        )
    }
}
