mod chunks;
mod op1;
mod util;

use chunks::{read_aif, ApplicationSpecificChunk, Chunk};
use clap::{
    crate_authors, crate_description, crate_version, value_t_or_exit, App, Arg, ArgMatches,
    SubCommand,
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
            io_command(key_command(
                SubCommand::with_name("silence"),
                "KEYS",
                "keys",
            ))
                .about("Turn sample gain to -inf"),
        )
        .subcommand(
            io_command(key_command(
                SubCommand::with_name("volume"),
                "KEYS",
                "keys",
            ))
                .about("Set sample gain to a value between -1.0 (-inf) and +1.0 (+12 dB)"),
        )
        .subcommand(
            io_command(key_command(
                SubCommand::with_name("reverse"),
                "KEYS",
                "keys",
            ))
                .about("Set sample to play in reverse"),
        )
        .subcommand(
            io_command(key_command(
                SubCommand::with_name("forward"),
                "KEYS",
                "keys",
            ))
                .about("Set sample to play forward"),
        )
        .subcommand(
            io_command(key_command(key_command(
                SubCommand::with_name("copy"),
                "KEYS",
                "keys",
            ), "DST", "dst"))
                .about("Copy samples to DST keys"),
        )
        .subcommand(
            io_command(key_command(
                SubCommand::with_name("pitch"),
                "KEYS",
                "keys",
            ))
                .arg(Arg::with_name("PITCH")
                     .short("p")
                     .value_name("PITCH")
                     .use_delimiter(true)
                     .required(true)
                     .help("A list of comma-separated numbers between -48-+48, representing the number of semitones to shift pitch by. Colons can be used to represent inclusive ranges for whole semitones. Decimal numbers may be used to perform micro-tonal shifts. If more keys are provided than pitch values, the last pitch will be applied to any remaining keys. E.g.: `-k 1:7 -p -7:-1` will shift the lower F to B keys by -7 to -1 semitones; `-k 1-24 -p 0.12` will shift all keys up by 12 cents"))
            .about("Shift the pitch of a given key"),
        )
        .subcommand(
            io_command(SubCommand::with_name("dump"))
                .about("Output the OP metadata associated with a patch"),
        )
        .subcommand(
            io_command(SubCommand::with_name("set"))
                .about("Overwrite the OP metadata with a given JSON file"),
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
        ("pitch", Some(sub_m)) => pitch(sub_m)?,
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

fn key_command<'a, 'b>(
    command: App<'a, 'b>,
    name: &'static str,
    long: &'static str,
) -> App<'a, 'b> {
    command
        .arg(Arg::with_name(name)
             .short(&long[0..1])
             .long(long)
             .value_name(name)
             .use_delimiter(true)
             .required(true)
             .help("One or more comma-separated numbers between 1-24, representing the keys on the OP. Colons can be used to represent inclusive ranges. E.g.: `1,2,13,14` is both F and F# keys; `1:7,13:19` is both sets of F to B keys"))
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

fn matches_keys<'a>(matches: &ArgMatches, key_arg: &str) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let keys: Vec<&str> = matches.values_of(key_arg).unwrap().collect();
    let mut r = vec![];
    for key in keys.iter() {
        let range: Vec<&str> = key.split(":").collect();
        match range.len() {
            1 => r.push(key.parse::<u8>()?),
            2 => {
                let start = range[0].parse::<u8>()?;
                let end = range[1].parse::<u8>()?;
                if start < end {
                    r.extend((start..=end).collect::<Vec<u8>>());
                } else {
                    r.extend((end..=start).rev().collect::<Vec<u8>>());
                }
            }
            _ => Err(format!("Invalid key: {}", key))?,
        }
    }

    Ok(r)
}

fn matches_pitches<'a>(
    matches: &ArgMatches,
    pitch_arg: &str,
) -> Result<Vec<f32>, Box<dyn error::Error>> {
    let pitches: Vec<&str> = matches.values_of(pitch_arg).unwrap().collect();
    let mut r = vec![];
    for pitch in pitches.iter() {
        let range: Vec<&str> = pitch.split(":").collect();
        match range.len() {
            1 => r.push(pitch.parse::<f32>()?),
            2 => {
                let start = range[0].parse::<u8>()?;
                let end = range[1].parse::<u8>()?;
                if start < end {
                    r.extend((start..=end).map(|x| x as f32).collect::<Vec<f32>>());
                } else {
                    r.extend((end..=start).rev().map(|x| x as f32).collect::<Vec<f32>>());
                }
            }
            _ => Err(format!("Invalid pitch: {}", pitch))?,
        }
    }

    Ok(r)
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
    let keys = matches_keys(matches, "KEYS")?;

    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        data.silence(keys)?;
    } else {
        Err("No OP data to silence")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };

    Ok(())
}

fn pitch(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    let pitches = matches_pitches(matches, "PITCH")?;

    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        data.pitch(keys, pitches)?;
    } else {
        Err("No OP data to pitch")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };

    Ok(())
}
