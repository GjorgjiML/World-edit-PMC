use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, OnceLock};

use pumpkin::{
    command::{dispatcher::CommandError, CommandSender},
    world::World,
};
use pumpkin_util::{
    math::{position::BlockPos, vector3::Vector3},
    text::{color::NamedColor, TextComponent},
};
use std::sync::Arc;
use uuid::Uuid;

/// Maximum number of blocks that can be modified in a single operation.
pub const MAX_BLOCKS: i64 = 100_000;

/// Schematics directory path, set during plugin load.
pub static SCHEMATICS_DIR: OnceLock<PathBuf> = OnceLock::new();

// ============================================================================
// Data Structures
// ============================================================================

/// Per-player WorldEdit state: selection, clipboard, and undo history.
pub struct PlayerState {
    pub pos1: Option<BlockPos>,
    pub pos2: Option<BlockPos>,
    pub clipboard: Option<ClipboardData>,
    pub undo_data: Option<Vec<(BlockPos, u16)>>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            pos1: None,
            pos2: None,
            clipboard: None,
            undo_data: None,
        }
    }
}

/// Blocks stored in the clipboard as (offset from player position, block state id).
pub struct ClipboardData {
    pub blocks: Vec<(Vector3<i32>, u16)>,
}

/// Global thread-safe storage for all player states.
pub static PLAYER_DATA: LazyLock<Mutex<HashMap<Uuid, PlayerState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the normalized selection (min corner, max corner) for a player.
pub fn get_selection(player_id: &Uuid) -> Result<(BlockPos, BlockPos), CommandError> {
    let state = PLAYER_DATA.lock().unwrap();
    let data = state.get(player_id).ok_or(CommandError::CommandFailed(
        TextComponent::text("No selection set. Use //pos1 and //pos2 first.")
            .color_named(NamedColor::Red),
    ))?;
    let pos1 = data.pos1.ok_or(CommandError::CommandFailed(
        TextComponent::text("Position 1 not set. Use //pos1 first.").color_named(NamedColor::Red),
    ))?;
    let pos2 = data.pos2.ok_or(CommandError::CommandFailed(
        TextComponent::text("Position 2 not set. Use //pos2 first.").color_named(NamedColor::Red),
    ))?;
    Ok((
        BlockPos(Vector3::new(
            pos1.0.x.min(pos2.0.x),
            pos1.0.y.min(pos2.0.y),
            pos1.0.z.min(pos2.0.z),
        )),
        BlockPos(Vector3::new(
            pos1.0.x.max(pos2.0.x),
            pos1.0.y.max(pos2.0.y),
            pos1.0.z.max(pos2.0.z),
        )),
    ))
}

/// Convert the sender's floating-point position to a block position.
pub fn sender_block_pos(sender: &CommandSender) -> Result<BlockPos, CommandError> {
    let pos = sender
        .position()
        .ok_or(CommandError::InvalidRequirement)?;
    Ok(BlockPos(Vector3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )))
}

/// Extract the player's UUID from the command sender.
pub fn sender_uuid(sender: &CommandSender) -> Result<Uuid, CommandError> {
    let player = sender
        .as_player()
        .ok_or(CommandError::InvalidRequirement)?;
    Ok(player.gameprofile.id)
}

/// Get the player's world from the command sender.
pub fn sender_world(sender: &CommandSender) -> Result<Arc<World>, CommandError> {
    sender.world().ok_or(CommandError::InvalidRequirement)
}

/// Calculate the volume of a selection.
pub fn selection_volume(min: &BlockPos, max: &BlockPos) -> i64 {
    let dx = (max.0.x - min.0.x + 1) as i64;
    let dy = (max.0.y - min.0.y + 1) as i64;
    let dz = (max.0.z - min.0.z + 1) as i64;
    dx * dy * dz
}

/// Check that the selection does not exceed the block limit.
pub fn check_selection_size(min: &BlockPos, max: &BlockPos) -> Result<(), CommandError> {
    let volume = selection_volume(min, max);
    if volume > MAX_BLOCKS {
        return Err(CommandError::CommandFailed(
            TextComponent::text(format!(
                "Selection too large ({volume} blocks). Maximum is {MAX_BLOCKS}."
            ))
            .color_named(NamedColor::Red),
        ));
    }
    Ok(())
}
