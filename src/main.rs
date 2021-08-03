mod chunks;
mod op1;
mod util;

use chunks::{read_aif, Chunk};
use clap::{App, Arg, SubCommand};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let matches = App::new("OP-1/Z Patch Utility")
        .version("1.0")
        .author("Alex Charlton")
        .about("A tool for creating and modifying patches for the OP-1 and OP-Z")
        .arg(Arg::with_name("INPUT").index(1).required(true))
        .arg(
            Arg::with_name("OUTPUT")
                .index(2)
                .short("o")
                .long("output")
                .default_value("output.aif"),
        )
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let output = matches.value_of("OUTPUT").unwrap();

    let mut input_file = File::open(input).unwrap();
    let form = read_aif(&mut input_file)?;
    // dbg!(&form);

    let mut output_file = File::create(output)?;
    form.write(&mut output_file)?;
    Ok(())
}
