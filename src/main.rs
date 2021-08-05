mod chunks;
mod op1;
mod util;

use chunks::{read_aif, ApplicationSpecificChunk, Chunk};
use clap::{
    crate_authors, crate_description, crate_version, value_t_or_exit, values_t_or_exit, App, Arg,
    ArgMatches, SubCommand,
};
use std::error;
use std::fs::File;
use std::io::{self, StdinLock, StdoutLock};

fn main() -> Result<(), Box<dyn error::Error>> {
    let mut app = App::new("OP-1/Z Patch Utility")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Silence all output"),
        )
        .subcommand(
            io_command(key_command(SubCommand::with_name("silence")))
                .about("Turn sample gain to -inf"),
        )
        .subcommand(
            io_command(SubCommand::with_name("dump"))
                .about("Output the OP metadata associated with a patch"),
        )
        .subcommand(
            io_command(SubCommand::with_name("shift"))
                .about("Shift the samples up or down by N keys")
                .arg(
                    Arg::with_name("N")
                        .value_name("N")
                        .short("n")
                        .required(true),
                ),
        );

    let mut help = vec![];
    app.write_long_help(&mut help).unwrap();
    let help = std::str::from_utf8(&help).unwrap();

    let matches = app.get_matches();

    let verbose = matches.occurrences_of("verbosity") as usize;
    let quiet = matches.is_present("quiet");
    stderrlog::new()
        .module(module_path!())
        .quiet(quiet)
        .verbosity(verbose)
        .init()
        .unwrap();

    match matches.subcommand() {
        ("shift", Some(sub_m)) => shift(sub_m)?,
        ("silence", Some(sub_m)) => silence(sub_m)?,
        _ => {
            eprintln!("Error: subcommand required\n");
            println!("{}", help);
        }
    }

    Ok(())
}

fn io_command<'a, 'b>(command: App<'a, 'b>) -> App<'a, 'b> {
    command
        .arg(Arg::with_name("INPUT").index(1).help("Omit to use STDIN"))
        .arg(
            Arg::with_name("OUTPUT")
                .index(2)
                .help("Use `-` to send output to STDOUT"),
        )
        .arg(
            Arg::with_name("OUTPUT_FILE")
                .short("o")
                .long("output")
                .default_value("output.aif"),
        )
}

fn key_command<'a, 'b>(command: App<'a, 'b>) -> App<'a, 'b> {
    command
        .arg(Arg::with_name("KEYS")
             .short("k")
             .long("keys")
             .value_name("KEYS")
             .use_delimiter(true)
             .required(true)
             .help("A list of comma-separated numbers between 1-24, representing the keys on the OP. E.g.: `1,2,13,14` is both F and F# keys"))
}

enum Input<'a> {
    File(File),
    Stdin(StdinLock<'a>),
}
enum Output<'a> {
    File(File),
    Stdout(StdoutLock<'a>),
}

fn matches_io<'a>(matches: &ArgMatches) -> Result<(Input<'a>, Output<'a>), Box<dyn error::Error>> {
    let stdin = Box::leak(Box::new(io::stdin()));
    let stdout = Box::leak(Box::new(io::stdout()));

    Ok(
        match (
            matches.value_of("INPUT"),
            matches.value_of("OUTPUT"),
            matches.value_of("OUTPUT_FILE"),
        ) {
            (Some("-"), None, Some("output.aif")) => {
                (Input::Stdin(stdin.lock()), Output::Stdout(stdout.lock()))
            }
            (Some("-"), None, Some(output)) | (Some("-"), Some(output), _) => (
                Input::Stdin(stdin.lock()),
                Output::File(File::create(output)?),
            ),
            (Some(input), Some("-"), _) => (
                Input::File(File::open(input)?),
                Output::Stdout(stdout.lock()),
            ),
            (Some(input), Some(output), _) => (
                Input::File(File::open(input)?),
                Output::File(File::create(output)?),
            ),
            (None, Some(output), _) => (
                Input::Stdin(stdin.lock()),
                Output::File(File::create(output)?),
            ),
            (Some(input), None, Some(output)) => (
                Input::File(File::open(input)?),
                Output::File(File::create(output)?),
            ),
            (None, None, Some(output)) => (
                Input::Stdin(stdin.lock()),
                Output::File(File::create(output)?),
            ),
            _ => panic!("This should not be possible"),
        },
    )
}

fn shift(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let n = value_t_or_exit!(matches.value_of("N"), i8);

    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        data.shift_samples(n)?;
    } else {
        Err("No OP data to shift")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };
    Ok(())
}

fn silence(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = values_t_or_exit!(matches.values_of("KEYS"), u8);

    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        data.silence(keys)?;
    } else {
        Err("No OP data to shift")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };

    Ok(())
}
