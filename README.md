# randomprime

This repository implements a "patcher" for the subset of Metroid Prime versions released for the Nintendo Gamecube. In the simplest sense, it takes a game ISO as input, makes modifications defined by a JSON layout description, and then outputs a new ISO. The output ISO is officially compatible with Dolphin, Nintendont and GC Loader.

*This repository contains no intellectual property for Metroid Prime. The only game-specific data present is information regarding the offsets/names of data known to exist on an unmodified copy of the game and custom-made assets. As such, users are required to provide their own legally obtained copy of Metroid Prime to use this patcher.*

## Features

To give you a taste of what's implemented, here are some highlighted features which are available via this program:

- Relocate upgrades (a.k.a randomizer, the namesake of this repository)
- Quality of life changes such as softlock fixes, crash fixes, hud changes, etc...
- Modify scripting objects and scripting connections
- Modify door colors, blast shields, room connections etc...
- Place simple objects such as blocks, platforms, triggers, timers etc...

## Usage

- The application which best makes use of this program is [randovania](https://github.com/randovania/randovania). It's a full-GUI application centered around randomizing the game with various settings, even supporting co-operative multiworld. It's implemented via the py-randomprime wrapper (described below)
- Some "fanhacks" have been made utilizing the features exposed by this program more directly. They can be found in the [metroid-prime-fanhacks](https://github.com/toasterparty/metroid-prime-fanhacks) repository. Be sure to check out the [Creator's Guide](https://github.com/toasterparty/metroid-prime-fanhacks/blob/main/doc/readme.md) for an in-depth dive into the patcher API and how to use it.
- [py-randomprime](https://github.com/randovania/py-randomprime) implements Python bindings for this project's feature set making it much more portable. The py-randomprime repository also builds standalone windows executable files (`.exe`) and attaches them to each release.

## Documentation

The API is documented thoroughly at [randovania.org/randomprime](https://randovania.org/randomprime). Though a bit dated, some auxillary documents which may be useful can be found in the [/doc/](./doc/) folder.

## Changelog

Updates to this program are documented as part of the [py-randomprime Release Process](https://github.com/randovania/py-randomprime/releases). The versioning for py-randomprime follows [Semantic Versioning](https://semver.org/). The version number exposed in the standalone application can be ignored.

## Compiling

1. Install a Rust compiler. It is recommended to use [rustup](https://www.rust-lang.org/tools/install).
2. Add `powerpc-unknown-linux-gnu` as a target, like so: `rustup target add --toolchain 1.85.1 powerpc-unknown-linux-gnu`
3. Clone the repo and all its submodules: `git clone https://github.com/randovania/randomprime --recursive`
4. Run `cargo build`

That should create a standalone executable in `./randomprime/target/debug/randomprime_patcher.exe`.

Occasionally run `rustup update` to keep your toolchain version up-to-date.

## Contributing

In order to pass this project's Pull Request requirements, your proposed change must pass the following checks:

```sh
cargo fmt --check
cargo clippy -- -D warnings
```

You can use these commands to fix most issues automatically:

```sh
cargo fmt
cargo clippy --fix --allow-dirty
```

## Resources

Some helpful resources for those starting out with modding Metroid Prime can be found in the [Metroid Prime Fanhacks](https://github.com/toasterparty/metroid-prime-fanhacks/tree/main/doc#resources) repository. Furthermore there's:

- [PowerPC 750cl ISA](https://fail0verflow.com/media/files/ppc_750cl.pdf) and [Appendix F](https://www.nxp.com/docs/en/user-guide/MPCFPE_AD_R1.pdf)
- [Godbolt](https://godbolt.org/)
- [Metroid Prime Crash Parser](https://metroidprimemodding.github.io/prime-crash-parser/crash.html)
- [Last Metaforce Commit with DATASPEC](https://github.com/AxioDL/metaforce/tree/1655d229cfdfbd5f792a7c3e84adc862653f70a7)
- [Unused Memory Locations](https://github.com/MetroidPrimeModding/prime-practice-native/blob/main/unused_memory.txt)
