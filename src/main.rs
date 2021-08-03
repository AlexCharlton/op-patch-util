mod chunks;
mod op1;
mod util;

use chunks::{read_aif, Chunk};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();

    println!("Hello, world!");

    // let filename = "pop-punk-kick-snare.aif";
    // let filename = "test3.aif"
    // let filename = "pop-punk-guitar-riff1-120.aif";
    // let filename = "user723678464.aif";
    let filename = "input.aif";
    let mut file = File::open(filename).unwrap();
    let form = read_aif(&mut file)?;
    dbg!(&form);

    let mut output = File::create("output.aif")?;
    form.write(&mut output)?;
    Ok(())
}
