mod chunks;
mod op1;
mod util;

use chunks::{read_aif, ApplicationSpecificChunk, Chunk};
use clap::{value_t_or_exit, App, Arg, ArgMatches, SubCommand};
use std::fs::File;
// use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let mut app = App::new("OP-1/Z Patch Utility")
        .version("1.0")
        .author("Alex Charlton")
        .about("A tool for creating and modifying patches for the OP-1 and OP-Z")
        .subcommand(
            SubCommand::with_name("shift")
                .about("Shift the samples up or down by N keys")
                .arg(Arg::with_name("INPUT").index(1).required(true))
                .arg(
                    Arg::with_name("N")
                        .value_name("N")
                        .short("n")
                        .required(true),
                )
                .arg(Arg::with_name("DEBUG").short("d"))
                .arg(
                    Arg::with_name("OUTPUT")
                        .index(2)
                        .short("o")
                        .long("output")
                        .default_value("output.aif"),
                ),
        );

    let mut help = vec![];
    app.write_long_help(&mut help).unwrap();
    let help = std::str::from_utf8(&help).unwrap();

    let matches = app.get_matches();

    match matches.subcommand() {
        ("shift", Some(sub_m)) => shift(sub_m)?,
        _ => {
            eprintln!("Error: subcommand required\n");
            println!("{}", help);
        }
    }

    Ok(())
}

fn shift(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let n = value_t_or_exit!(matches.value_of("N"), i8);
    let input = matches.value_of("INPUT").unwrap();
    let output = matches.value_of("OUTPUT").unwrap();

    let mut input_file = File::open(input).unwrap();
    let mut form = read_aif(&mut input_file)?;

    if matches.is_present("DEBUG") {
        println!("Input file: {:#?}", &form);
    }

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        data.shift_samples(n)?;
    } else {
        Err("No OP data to shift")?;
    }

    let mut output_file = File::create(output)?;
    form.write(&mut output_file)?;
    println!("Wrote {}", &output);
    Ok(())
}
