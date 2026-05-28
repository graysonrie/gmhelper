# gmhelper

Command-line helpers for GameMaker projects:

- watch `.aseprite` files and export sprite tags
- export game music from a local `music/` folder
- hot-reload `.gml` changes by rebuilding/relaunching
- view or rerun recent `gmhelper` commands

## Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- `ffmpeg` installed and available on your `PATH` (required for music export and MP4 output)
- Aseprite CLI available if you use the `sprites` workflow

## Setup

From the project root:

```bash
cargo build --release
```

Quick CLI setup:

```bash
cargo install --path . --force
```

Run commands with:

```bash
cargo run -- <subcommand> [options]
```

## Quick Usage

### Sprites: watch and export Aseprite tags

Watch the current directory:

```bash
cargo run -- sprites --start
```

Watch a specific directory:

```bash
cargo run -- sprites --directory ./assets/sprites
```

Import directly into a GameMaker project:

```bash
cargo run -- sprites --directory ./assets/sprites --project ./MyGame.yyp
```

### Music: export music assets from current folder

Export GameMaker-ready music from the local `music/` folder:

```bash
cargo run -- music
```

Export MP4 previews:

```bash
cargo run -- music --mp4 --game-name "My Game" --image-path "./cover.png"
```

### Reload: hot-reload GameMaker project

```bash
cargo run -- reload ./MyGame.yyp
```

### Previous: command history

List recent runs:

```bash
cargo run -- previous
```

Re-run the most recent command:

```bash
cargo run -- previous 1
```
