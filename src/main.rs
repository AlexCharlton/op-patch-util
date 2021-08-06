mod chunks;
mod op1;
mod util;

use chunks::{read_aif, ApplicationSpecificChunk, Chunk};
use clap::{
    crate_authors, crate_description, crate_version, value_t_or_exit, values_t_or_exit, App, Arg,
    ArgMatches, SubCommand,
};
use std::error;
use std::fs::{self, File};
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
                .help("Increase message verbosity."),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Silence all output."),
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
                .arg(Arg::with_name("VOLUME")
                     .short("g")
                     .long("gain")
                     .value_name("VOLUME")
                     .use_delimiter(true)
                     .required(true)
                     .help("A list of comma-separated numbers between -1-+1, representing the amount of gain to apply. If more keys are provided than gain values, the last gain will be applied to any remaining keys."))
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
            io_command(key_command(
                SubCommand::with_name("copy"),
                "KEYS",
                "keys",
            ))
                .arg(Arg::with_name("SRC")
                     .short("s")
                     .long("src")
                     .value_name("SRC")
                     .use_delimiter(true)
                     .required(true)
                     .help("Same as KEYS, but this is the key that is being copied. If there are more KEYS that SRCs, then the last SRC will be copied to all remaining destinations."))
                .about("Copy samples from one set of keys to another"),
        )
        .subcommand(
            io_command(key_command(
                SubCommand::with_name("pitch"),
                "KEYS",
                "keys",
            ))
                .arg(Arg::with_name("PITCH")
                     .short("p")
                     .long("pitch")
                     .value_name("PITCH")
                     .use_delimiter(true)
                     .required(true)
                     .help("A list of comma-separated numbers between -48-+48, representing the number of semitones to shift pitch by. Colons can be used to represent inclusive ranges for whole semitones. Only whole numbers may be used. If more keys are provided than pitch values, the last pitch will be applied to any remaining keys. E.g.: `-k 1:7 -p -7:-1` will shift the lower F to B keys by -7 to -1 semitones; `-k 1-24 -p 2` will shift all keys up by 2 semitones."))
                .about("Shift the pitch of a given key"),
        )
        .subcommand(
            io_command(SubCommand::with_name("dump"))
                .about("Output the OP metadata associated with a patch"),
        )
        .subcommand(
            io_command(SubCommand::with_name("set"))
                .arg(
                    Arg::with_name("JSON")
                        .value_name("JSON")
                        .short("j")
                        .long("json")
                        .required(true)
                        .help("The JSON file with which to overwrite the OP metadata. Must be valid OP metadata.")
                )
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
        ("volume", Some(sub_m)) => volume(sub_m)?,
        ("reverse", Some(sub_m)) => reverse(sub_m)?,
        ("forward", Some(sub_m)) => forward(sub_m)?,
        ("copy", Some(sub_m)) => copy(sub_m)?,
        ("dump", Some(sub_m)) => dump(sub_m)?,
        ("set", Some(sub_m)) => set(sub_m)?,
        _ => {
            eprintln!("Error: subcommand required\n");
            println!("{}", help);
        }
    }

    Ok(())
}

fn io_command<'a, 'b>(command: App<'a, 'b>) -> App<'a, 'b> {
    command
        .arg(Arg::with_name("INPUT").index(1).help("Omit to use STDIN."))
        .arg(
            Arg::with_name("OUTPUT")
                .index(2)
                .help("Use `-` to send output to STDOUT."),
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
             .help("One or more comma-separated numbers between 1-24, representing the keys on the OP that are to be modified. Colons can be used to represent inclusive ranges. E.g.: `1,2,13,14` is both F and F# keys; `1:7,13:19` is both sets of F to B keys."))
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
) -> Result<Vec<i8>, Box<dyn error::Error>> {
    let pitches: Vec<&str> = matches.values_of(pitch_arg).unwrap().collect();
    let mut r = vec![];
    for pitch in pitches.iter() {
        let range: Vec<&str> = pitch.split(":").collect();
        match range.len() {
            1 => r.push(pitch.parse::<i8>()?),
            2 => {
                let start = range[0].parse::<i8>()?;
                let end = range[1].parse::<i8>()?;
                if start < end {
                    r.extend((start..=end).collect::<Vec<i8>>());
                } else {
                    r.extend((end..=start).rev().collect::<Vec<i8>>());
                }
            }
            _ => Err(format!("Invalid pitch: {}", pitch))?,
        }
    }

    Ok(r)
}

fn op<F>(matches: &ArgMatches, f: F) -> Result<(), Box<dyn error::Error>>
where
    F: Fn(&mut op1::OP1Data) -> Result<(), String>,
{
    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        f(data)?;
    } else {
        Err("No OP data to alter")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };
    Ok(())
}

fn shift(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let n = value_t_or_exit!(matches.value_of("N"), i8);
    op(matches, |data| data.shift_samples(n))
}

fn silence(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    op(matches, |data| data.gain(&keys, &[-1.0]))
}

fn pitch(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    let pitches = matches_pitches(matches, "PITCH")?;
    op(matches, |data| data.pitch(&keys, &pitches))
}

fn volume(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    let gains = values_t_or_exit!(matches.values_of("VOLUME"), f32);
    op(matches, |data| data.gain(&keys, &gains))
}

fn forward(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    op(matches, |data| data.reverse(&keys, false))
}

fn reverse(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    op(matches, |data| data.reverse(&keys, true))
}

fn copy(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let keys = matches_keys(matches, "KEYS")?;
    let src = matches_keys(matches, "SRC")?;
    op(matches, |data| data.copy(&keys, &src))
}

fn dump(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    let json = if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        serde_json::to_vec_pretty(data)?
    } else {
        Err("No OP data to dump")?
    };

    use std::io::Write;
    match o {
        Output::Stdout(mut stdout) => stdout.write_all(&json)?,
        Output::File(mut file) => file.write_all(&json)?,
    };
    Ok(())
}

fn set(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let (i, o) = matches_io(matches)?;
    let mut form = match i {
        Input::Stdin(mut stdin) => read_aif(&mut stdin)?,
        Input::File(mut file) => read_aif(&mut file)?,
    };

    log::info!("Input file: {:#?}", &form);

    let json = fs::read(matches.value_of("JSON").unwrap())?;
    let new_data: op1::OP1Data = serde_json::from_slice(&json)?;

    if let Some(ApplicationSpecificChunk::OP1 { data }) = form.app.first_mut() {
        *data = new_data;
    } else {
        Err("No OP data to alter")?;
    }

    match o {
        Output::Stdout(mut stdout) => form.write(&mut stdout)?,
        Output::File(mut file) => form.write(&mut file)?,
    };
    Ok(())
}
