# OP Patch Utility
A command line tool for creating and modifying patches for the OP-1 and OP-Z.

## Installation
E.g. installing the OSX build to `/usr/local/bin`:
```
curl -Ls https://github.com/AlexCharlton/op-patch-util/releases/latest/download/op-patch-util-1.0.0-osx.tar.gz | sudo tar -zx -C /usr/local/bin
```

### Installing with the Rust toolchain
```
$ git clone https://github.com/AlexCharlton/op-patch-util.git
$ cargo install --path ./op-patch-util
```

## Usage
```
USAGE:
    op-patch-util [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -q               Silence all output.
    -V, --version    Prints version information
    -v               Increase message verbosity.

SUBCOMMANDS:
    copy       Copy samples from one set of keys to another
    drum       Create a drum patch from up to 24 WAV files
    dump       Output the OP metadata associated with a patch
    forward    Set sample to play forward
    help       Prints this message or the help of the given subcommand(s)
    pitch      Shift the pitch of a given key
    reverse    Set sample to play in reverse
    set        Overwrite the OP metadata with a given JSON file
    shift      Shift the samples up or down by N keys
    silence    Turn sample gain to -inf
    synth      Create a synth sampler from a WAV file
    volume     Set sample gain to a value between -1.0 (-inf) and +1.0 (+12 dB)
```

### Examples
#### Creating a 12 key patch
Given a directory `samples` with 12 samples from C-B:
```
$ op-patch-util drum samples/*.wav -s7 -p
```
This will create a new drum patch using the input samples. The samples are shifted over by 7 keys (`-s7`) to align with the C key, then the first and last samples are pitched to fill in the remaining keys (`-p`).

#### Adjusting the gain on a patch
```
$ op-patch-util volume --keys 1-12 --gain 0.5 input.aif
```
This will create a new `output.aif` with +3 dB more gain (1.0 is +6 dB) than the input on the first octave `--keys 1-12`.

#### Editing metadata using jq
If you need to edit metadata that isn't directly supported by op-patch-util, you can use the excellent [jq](https://stedolan.github.io/jq/):
```
$ op-patch-util dump input.aif - | jq '.octave = 1' > new.json
$ op-patch-util set  -j new.json input.aif
```
This creates a new `output.aif` with an octave value of 1.

## Contributing
If you think the op-patch-util should do something it doesn't, or if you've found a bug, please file a [Github issue](https://github.com/AlexCharlton/op-patch-util/issues).

Pull requests are welcome.
