mod chunks;
mod util;

use chunks::read_aif;
use std::fs::File;

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    println!("Hello, world!");

    let mut file = File::open("pop-punk-kick-snare.aif").unwrap();
    let form = read_aif(&mut file).unwrap();
    dbg!(form);
}
