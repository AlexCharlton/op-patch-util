mod chunks;
mod util;

use chunks::read_aif;
use std::fs::File;

fn main() {
    println!("Hello, world!");

    let file = File::open("pop-punk-kick-snare.aif").unwrap();
    let form = read_aif(file).unwrap();
    dbg!(form);
}
