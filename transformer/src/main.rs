#![forbid(unsafe_code)]

mod buildtime_input;
mod cyclonedx;

use std::env;
use std::fs;

use anyhow::Result;

use buildtime_input::BuildtimeInput;
use cyclonedx::CycloneDXBom;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let target_path = &args[1];
    let buildtime_input_path = &args[2];
    let runtime_input_path = &args[3];

    let mut buildtime_input: BuildtimeInput =
        serde_json::from_reader(fs::File::open(buildtime_input_path)?)?;
    let target_derivation = buildtime_input.remove_derivation(target_path);
    let runtime_input_string = fs::read_to_string(runtime_input_path)?;
    let runtime_input: Vec<&str> = runtime_input_string.lines().collect();

    let output = CycloneDXBom::build(target_derivation, buildtime_input, runtime_input)?;
    println!("{}", output.serialize()?);

    Ok(())
}
