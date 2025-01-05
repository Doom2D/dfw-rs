# dfwad

## Description
dfwad manages your DFWADs. It can extract DFWADs and create them.

DFWAD is a game data format used in [Doom2D Forever](https://repo.or.cz/d2df-sdl), and is similar to WAD.

## Installation

1. Install Rust: follow the instructions on the [official Rust website](https://www.rust-lang.org/tools/install) to install the Rust compiler and related tools.
2. Clone the repository: `git clone https://github.com/Doom2D/dfwad.git`
3. Change into the project directory: `cd dfwad`
4. Build the project: `cargo build`

## Examples

- Pack the `game` directory in a DFWAD called `game.wad`, without Zlib compression: `dfwad game game.wad --zlib-level none pack`
- Extract a DFWAD into `game` directory: `dfwad game.wad game extract`

## Usage notes
- DFWADs support only 1 level of depth, but it can be circumvented by creating an another WAD representing these directories. DFWAD does that.
- Error handling is subpar, so error messages may be unclear.
