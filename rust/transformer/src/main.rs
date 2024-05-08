mod buildtime_input;
mod cli;
mod cyclonedx;
mod derivation;
mod hash;
mod runtime_input;
mod transform;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

fn main() -> Result<()> {
    Cli::parse().call()
}
