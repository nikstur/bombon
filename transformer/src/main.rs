#![forbid(unsafe_code)]

mod cyclonedx;
mod input;

use std::env;
use std::fs;

use anyhow::Result;

use cyclonedx::Output;
use input::Input;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let buildtime_input_path = &args[1];
    // let runtime_input_path = &args[2];

    let buildtime_deps: Input = serde_json::from_reader(fs::File::open(buildtime_input_path)?)?;
    // let runtime: Input = serde_json::from_reader(fs::File::open(runtime_input)?)?;

    let output = Output::try_from(buildtime_deps)?;
    println!("{}", output.serialize()?);

    Ok(())
}
