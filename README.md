# Pumpkin WorldEdit Plugin

A simple WorldEdit plugin for the [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) Minecraft server, written in Rust.

## Features

- **Region selection** with pos1/pos2 commands
- **Block manipulation**: set, replace, walls, clear, hollow
- **Clipboard**: copy and paste with relative positioning
- **Undo** support for all block-modifying operations
- Per-player state (each player has their own selection, clipboard, and undo history)
- Selection size limit (100,000 blocks) to prevent server lag

## Commands

All commands use the `/we` (or `/worldedit`) prefix:

| Command | Description |
|---|---|
| `/we pos1` | Set position 1 at your feet |
| `/we pos2` | Set position 2 at your feet |
| `/we set <block>` | Fill selection with a block |
| `/we replace <from> <to>` | Replace one block type with another |
| `/we walls <block>` | Build walls around selection (X/Z edges) |
| `/we copy` | Copy selection to clipboard |
| `/we paste` | Paste clipboard at your position |
| `/we undo` | Undo the last operation |
| `/we size` | Show selection dimensions |
| `/we clear` | Set all blocks in selection to air |
| `/we hollow` | Remove the interior of the selection |

## Requirements

- [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) Minecraft server (built from source)
- Rust toolchain (for building the plugin)
- Operator permission level 2 to use commands

## Building

```bash
cargo build --release
```

The compiled plugin DLL/SO will be in `target/release/`:

- **Windows**: `pumpkin_worldedit.dll`
- **Linux**: `libpumpkin_worldedit.so`
- **macOS**: `libpumpkin_worldedit.dylib`

## Installation

Copy the compiled plugin binary into the `plugins/` directory of your Pumpkin server and start/restart the server.

## Usage Example

```
/we pos1          # Stand at one corner and set position 1
/we pos2          # Move to opposite corner and set position 2
/we set stone     # Fill the entire selection with stone
/we undo          # Undo if you made a mistake
/we walls oak_planks  # Build walls around the selection
/we copy          # Copy the selection
/we paste         # Paste at a new location
```

## License

MIT
