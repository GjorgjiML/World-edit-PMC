pub mod clipboard;
pub mod history;
pub mod region;
pub mod schematic;
pub mod selection;

use pumpkin::{
    command::{
        args::{block::BlockArgumentConsumer, simple::SimpleArgConsumer},
        tree::{
            builder::{argument, literal},
            CommandTree,
        },
    },
};

use clipboard::{CopyExecutor, PasteExecutor};
use history::UndoExecutor;
use region::{
    ClearExecutor, HollowExecutor, ReplaceExecutor, SetExecutor, WallsExecutor, ARG_BLOCK,
    ARG_FROM, ARG_TO,
};
use schematic::{
    SchemDeleteExecutor, SchemListExecutor, SchemLoadExecutor, SchemSaveExecutor, ARG_SCHEM_NAME,
};
use selection::{Pos1Executor, Pos2Executor, SizeExecutor};

const COMMAND_NAMES: [&str; 2] = ["we", "worldedit"];
const COMMAND_DESCRIPTION: &str = "WorldEdit commands for region editing.";

/// Build the full `/we` command tree with all subcommands.
pub fn build_command_tree() -> CommandTree {
    CommandTree::new(COMMAND_NAMES, COMMAND_DESCRIPTION)
        // Selection
        .then(literal("pos1").execute(Pos1Executor))
        .then(literal("pos2").execute(Pos2Executor))
        .then(literal("size").execute(SizeExecutor))
        // Region editing
        .then(
            literal("set").then(argument(ARG_BLOCK, BlockArgumentConsumer).execute(SetExecutor)),
        )
        .then(literal("replace").then(
            argument(ARG_FROM, BlockArgumentConsumer)
                .then(argument(ARG_TO, BlockArgumentConsumer).execute(ReplaceExecutor)),
        ))
        .then(
            literal("walls")
                .then(argument(ARG_BLOCK, BlockArgumentConsumer).execute(WallsExecutor)),
        )
        .then(literal("clear").execute(ClearExecutor))
        .then(literal("hollow").execute(HollowExecutor))
        // Clipboard
        .then(literal("copy").execute(CopyExecutor))
        .then(literal("paste").execute(PasteExecutor))
        // History
        .then(literal("undo").execute(UndoExecutor))
        // Schematics
        .then(
            literal("schem")
                .then(
                    literal("load").then(
                        argument(ARG_SCHEM_NAME, SimpleArgConsumer).execute(SchemLoadExecutor),
                    ),
                )
                .then(
                    literal("save").then(
                        argument(ARG_SCHEM_NAME, SimpleArgConsumer).execute(SchemSaveExecutor),
                    ),
                )
                .then(literal("list").execute(SchemListExecutor))
                .then(
                    literal("delete").then(
                        argument(ARG_SCHEM_NAME, SimpleArgConsumer).execute(SchemDeleteExecutor),
                    ),
                ),
        )
}
