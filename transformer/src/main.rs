#![forbid(unsafe_code)]

mod cyclonedx;
mod input;

use std::env;
use std::fs;

use anyhow::Result;

use cyclonedx::Output;
use input::BuildtimeInput;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let buildtime_input_path = &args[1];
    let runtime_input_path = &args[2];

    let buildtime_input: BuildtimeInput =
        serde_json::from_reader(fs::File::open(buildtime_input_path)?)?;
    let runtime_input_string = fs::read_to_string(runtime_input_path)?;
    let runtime_input: Vec<&str> = runtime_input_string.lines().collect();

    let output = Output::convert(buildtime_input, runtime_input)?;
    println!("{}", output.serialize()?);

    Ok(())
}
