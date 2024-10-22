use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::transform::transform;

#[derive(Parser)]
pub struct Cli {
    /// Include buildtime dependencies in output
    #[arg(long)]
    include_buildtime_dependencies: bool,

    /// Regex pattern of store paths to exclude from the final SBOM.
    ///
    /// Can be given multiple times to exclude multiple patterns.
    #[arg(short, long)]
    exclude: Vec<String>,

    /// Path to target derivation
    target: String,

    /// Path to JSON containing the buildtime input
    buildtime_input: PathBuf,

    /// Path to a newline separated .txt file containing the runtime input
    runtime_input: PathBuf,

    /// Path to write the SBOM to
    output: PathBuf,
}

impl Cli {
    pub fn call(self) -> Result<()> {
        transform(
            self.include_buildtime_dependencies,
            &self.exclude,
            &self.target,
            &self.buildtime_input,
            &self.runtime_input,
            &self.output,
        )
    }
}
