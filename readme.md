# OP Patch Utility
A command line tool for creating and modifying patches for the OP-1 and OP-Z.

## Installation
E.g. installing the OSX build to `/usr/local/bin`:
```
curl -s https://github.com/AlexCharlton/op-patch-util/releases/latest/download/op-patch-util-1.0.0-osx.tar.gz | sudo tar -zx -C /usr/local/bin
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

## Contributing
If you think the op-patch-util should do something it doesn't, or if you've found a bug, please file a [Github issue](https://github.com/AlexCharlton/op-patch-util/issues).

Pull requests are welcome.
