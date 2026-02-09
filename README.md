# Pumpkin WorldEdit Plugin

A WorldEdit-style plugin for the [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) Minecraft server, written in Rust. Supports region selection, block operations, clipboard, undo, and schematic load/save in both Sponge (`.schem`) and Litematica (`.litematic`) formats.

## Features

- **Region selection** — Set two corners with `pos1` and `pos2`
- **Block operations** — Set, replace, walls, clear, hollow
- **Clipboard** — Copy and paste with relative positioning
- **Undo** — Restore the last block-modifying operation
- **Schematics** — Load and save structures from `.schem` (Sponge v2/v3) and `.litematic` (Litematica) files
- Per-player state (selection, clipboard, undo)
- Selection limit of 100,000 blocks to avoid server lag

## Commands

All commands use the `/we` or `/worldedit` prefix.

### Selection

| Command       | Description                          |
|---------------|--------------------------------------|
| `/we pos1`    | Set position 1 at your feet          |
| `/we pos2`    | Set position 2 at your feet          |
| `/we size`    | Show selection dimensions            |

### Region editing

| Command                    | Description                              |
|----------------------------|------------------------------------------|
| `/we set <block>`          | Fill selection with a block              |
| `/we replace <from> <to>`  | Replace one block type with another      |
| `/we walls <block>`        | Build walls on X/Z edges of selection    |
| `/we clear`                | Set all blocks in selection to air       |
| `/we hollow`               | Remove interior, keep walls              |

### Clipboard & history

| Command       | Description                          |
|---------------|--------------------------------------|
| `/we copy`    | Copy selection to clipboard          |
| `/we paste`   | Paste clipboard at your position     |
| `/we undo`    | Undo the last operation              |

### Schematics

| Command                  | Description                                      |
|--------------------------|--------------------------------------------------|
| `/we schem load <name>`  | Load a schematic into clipboard (`.schem` or `.litematic`) |
| `/we schem save <name>`  | Save clipboard as a `.schem` file                |
| `/we schem list`         | List saved schematics                            |
| `/we schem delete <name>`| Delete a schematic file                          |

Schematic files are stored in `plugins/pumpkin-worldedit/schematics/`. For load/delete you can use the name with or without extension (e.g. `castle` or `castle.litematic`).

## Supported schematic formats

- **Sponge Schematic (`.schem`)** — Versions 2 and 3 (gzipped NBT, varint block data). Compatible with WorldEdit and many other tools.
- **Litematica (`.litematic`)** — Gzipped NBT with regions, packed long-array block states, and optional metadata (position/size fallbacks for compatibility).

## Requirements

- [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) server (built from source)
- Rust toolchain to build the plugin
- Permission `pumpkin-worldedit:command.we` (default: OP level 2)

## Building

From the `pumpkin-worldedit` directory (or with path dependencies correct from the Pumpkin repo root):

```bash
cargo build --release
```

Output:

- **Windows**: `target/release/pumpkin_worldedit.dll`
- **Linux**: `target/release/libpumpkin_worldedit.so`
- **macOS**: `target/release/libpumpkin_worldedit.dylib`

## Installation

1. Copy the built plugin into the Pumpkin server `plugins/` folder.
2. Start or restart the server. The plugin will create `plugins/pumpkin-worldedit/schematics/` on first load.

## Usage examples

**Basic region and paste:**

```
/we pos1
/we pos2
/we set stone
/we copy
/we paste
/we undo
```

**Load and paste a schematic:**

```
/we schem load castle
/we paste
```

**Save a selection as schematic:**

```
/we pos1
/we pos2
/we copy
/we schem save my_build
```

## Project structure

```
pumpkin-worldedit/
├── src/
│   ├── lib.rs              # Plugin entry, on_load, command registration
│   ├── state.rs            # Per-player state, selection helpers
│   ├── schematic.rs        # .schem / .litematic load & save
│   └── commands/
│       ├── mod.rs          # Command tree builder
│       ├── selection.rs    # pos1, pos2, size
│       ├── region.rs       # set, replace, walls, clear, hollow
│       ├── clipboard.rs    # copy, paste
│       ├── history.rs      # undo
│       └── schematic.rs    # schem load/save/list/delete
├── Cargo.toml
└── README.md
```

## License

MIT
