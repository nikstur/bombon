#![forbid(unsafe_code)]

mod buildtime_input;
mod cli;
mod cyclonedx;
mod runtime_input;
mod transform;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

fn main() -> Result<()> {
    Cli::parse().call()
}
