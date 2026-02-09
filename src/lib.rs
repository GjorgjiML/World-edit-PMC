use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use async_trait::async_trait;
use pumpkin::{
    command::{
        args::{block::BlockArgumentConsumer, ConsumedArgs, FindArg},
        dispatcher::CommandError,
        tree::{
            builder::{argument, literal},
            CommandTree,
        },
        CommandExecutor, CommandResult, CommandSender,
    },
    plugin::Context,
    server::Server,
};
use pumpkin_api_macros::{plugin_impl, plugin_method};
use pumpkin_data::Block;
use pumpkin_util::{
    math::{position::BlockPos, vector3::Vector3},
    permission::{Permission, PermissionDefault},
    text::{color::NamedColor, TextComponent},
};
use pumpkin_world::world::BlockFlags;
use uuid::Uuid;

// ============================================================================
// Player State Management
// ============================================================================

struct PlayerState {
    pos1: Option<BlockPos>,
    pos2: Option<BlockPos>,
    clipboard: Option<ClipboardData>,
    undo_data: Option<Vec<(BlockPos, u16)>>,
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

struct ClipboardData {
    /// Blocks stored as (offset from player position, block state id)
    blocks: Vec<(Vector3<i32>, u16)>,
}

static PLAYER_DATA: LazyLock<Mutex<HashMap<Uuid, PlayerState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const MAX_BLOCKS: i64 = 100_000;

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the normalized selection (min corner, max corner) for a player.
fn get_selection(player_id: &Uuid) -> Result<(BlockPos, BlockPos), CommandError> {
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

/// Convert player's floating-point position to a block position.
fn sender_block_pos(sender: &CommandSender) -> Result<BlockPos, CommandError> {
    let pos = sender
        .position()
        .ok_or(CommandError::InvalidRequirement)?;
    Ok(BlockPos(Vector3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )))
}

/// Get the player's UUID from the sender.
fn sender_uuid(sender: &CommandSender) -> Result<Uuid, CommandError> {
    let player = sender
        .as_player()
        .ok_or(CommandError::InvalidRequirement)?;
    Ok(player.gameprofile.id)
}

/// Calculate the volume of a selection.
fn selection_volume(min: &BlockPos, max: &BlockPos) -> i64 {
    let dx = (max.0.x - min.0.x + 1) as i64;
    let dy = (max.0.y - min.0.y + 1) as i64;
    let dz = (max.0.z - min.0.z + 1) as i64;
    dx * dy * dz
}

/// Check that the selection is not too large.
fn check_selection_size(min: &BlockPos, max: &BlockPos) -> Result<(), CommandError> {
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

// ============================================================================
// Command Executors
// ============================================================================

// --- //pos1 ---
struct Pos1Executor;

#[async_trait]
impl CommandExecutor for Pos1Executor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let block_pos = sender_block_pos(sender)?;
            let player_id = sender_uuid(sender)?;

            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.pos1 = Some(block_pos);
            }

            sender
                .send_message(
                    TextComponent::text(format!(
                        "Position 1 set to ({}, {}, {})",
                        block_pos.0.x, block_pos.0.y, block_pos.0.z
                    ))
                    .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(1)
        })
    }
}

// --- //pos2 ---
struct Pos2Executor;

#[async_trait]
impl CommandExecutor for Pos2Executor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let block_pos = sender_block_pos(sender)?;
            let player_id = sender_uuid(sender)?;

            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.pos2 = Some(block_pos);
            }

            sender
                .send_message(
                    TextComponent::text(format!(
                        "Position 2 set to ({}, {}, {})",
                        block_pos.0.x, block_pos.0.y, block_pos.0.z
                    ))
                    .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(1)
        })
    }
}

// --- //set <block> ---
const ARG_BLOCK: &str = "block";

struct SetExecutor;

#[async_trait]
impl CommandExecutor for SetExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let block = BlockArgumentConsumer::find_arg(args, ARG_BLOCK)?;
            let block_state_id = block.default_state.id;
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            // Save undo data
            let mut undo_blocks = Vec::new();

            let mut count = 0i32;
            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        let pos = BlockPos(Vector3::new(x, y, z));
                        let old_state = world.get_block_state_id(&pos).await;
                        undo_blocks.push((pos, old_state));

                        world
                            .set_block_state(&pos, block_state_id, BlockFlags::FORCE_STATE)
                            .await;
                        count += 1;
                    }
                }
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) changed."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //replace <from> <to> ---
const ARG_FROM: &str = "from";
const ARG_TO: &str = "to";

struct ReplaceExecutor;

#[async_trait]
impl CommandExecutor for ReplaceExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let from_block = BlockArgumentConsumer::find_arg(args, ARG_FROM)?;
            let to_block = BlockArgumentConsumer::find_arg(args, ARG_TO)?;
            let to_state_id = to_block.default_state.id;
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        let pos = BlockPos(Vector3::new(x, y, z));
                        let current_block = world.get_block(&pos).await;

                        if current_block.id == from_block.id {
                            let old_state = world.get_block_state_id(&pos).await;
                            undo_blocks.push((pos, old_state));

                            world
                                .set_block_state(&pos, to_state_id, BlockFlags::FORCE_STATE)
                                .await;
                            count += 1;
                        }
                    }
                }
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) replaced."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //walls <block> ---
struct WallsExecutor;

#[async_trait]
impl CommandExecutor for WallsExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let block = BlockArgumentConsumer::find_arg(args, ARG_BLOCK)?;
            let block_state_id = block.default_state.id;
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        // Only place on walls (edges in X and Z, but not top/bottom)
                        let is_wall = x == min.0.x
                            || x == max.0.x
                            || z == min.0.z
                            || z == max.0.z;

                        if is_wall {
                            let pos = BlockPos(Vector3::new(x, y, z));
                            let old_state = world.get_block_state_id(&pos).await;
                            undo_blocks.push((pos, old_state));

                            world
                                .set_block_state(&pos, block_state_id, BlockFlags::FORCE_STATE)
                                .await;
                            count += 1;
                        }
                    }
                }
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) changed."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //copy ---
struct CopyExecutor;

#[async_trait]
impl CommandExecutor for CopyExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let player_pos = sender_block_pos(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let mut blocks = Vec::new();
            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        let pos = BlockPos(Vector3::new(x, y, z));
                        let state_id = world.get_block_state_id(&pos).await;
                        // Store offset relative to player position
                        let offset = Vector3::new(
                            x - player_pos.0.x,
                            y - player_pos.0.y,
                            z - player_pos.0.z,
                        );
                        blocks.push((offset, state_id));
                    }
                }
            }

            let block_count = blocks.len();

            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.clipboard = Some(ClipboardData { blocks });
            }

            sender
                .send_message(
                    TextComponent::text(format!("{block_count} block(s) copied to clipboard."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(block_count as i32)
        })
    }
}

// --- //paste ---
struct PasteExecutor;

#[async_trait]
impl CommandExecutor for PasteExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let player_pos = sender_block_pos(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            // Get clipboard data (clone it to release the lock)
            let clipboard_blocks = {
                let state = PLAYER_DATA.lock().unwrap();
                let data = state.get(&player_id).ok_or(CommandError::CommandFailed(
                    TextComponent::text("Clipboard is empty. Use //copy first.")
                        .color_named(NamedColor::Red),
                ))?;
                let clipboard = data.clipboard.as_ref().ok_or(CommandError::CommandFailed(
                    TextComponent::text("Clipboard is empty. Use //copy first.")
                        .color_named(NamedColor::Red),
                ))?;
                clipboard.blocks.clone()
            };

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for (offset, state_id) in &clipboard_blocks {
                let target = BlockPos(Vector3::new(
                    player_pos.0.x + offset.x,
                    player_pos.0.y + offset.y,
                    player_pos.0.z + offset.z,
                ));

                let old_state = world.get_block_state_id(&target).await;
                undo_blocks.push((target, old_state));

                world
                    .set_block_state(&target, *state_id, BlockFlags::FORCE_STATE)
                    .await;
                count += 1;
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) pasted."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //undo ---
struct UndoExecutor;

#[async_trait]
impl CommandExecutor for UndoExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            // Take undo data (clone and remove from state)
            let undo_blocks = {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.get_mut(&player_id).ok_or(CommandError::CommandFailed(
                    TextComponent::text("Nothing to undo.").color_named(NamedColor::Red),
                ))?;
                data.undo_data.take().ok_or(CommandError::CommandFailed(
                    TextComponent::text("Nothing to undo.").color_named(NamedColor::Red),
                ))?
            };

            let mut count = 0i32;
            for (pos, old_state_id) in &undo_blocks {
                world
                    .set_block_state(pos, *old_state_id, BlockFlags::FORCE_STATE)
                    .await;
                count += 1;
            }

            sender
                .send_message(
                    TextComponent::text(format!("Undo: {count} block(s) restored."))
                        .color_named(NamedColor::Green),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //size ---
struct SizeExecutor;

#[async_trait]
impl CommandExecutor for SizeExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let (min, max) = get_selection(&player_id)?;

            let dx = max.0.x - min.0.x + 1;
            let dy = max.0.y - min.0.y + 1;
            let dz = max.0.z - min.0.z + 1;
            let volume = (dx as i64) * (dy as i64) * (dz as i64);

            sender
                .send_message(
                    TextComponent::text(format!(
                        "Selection: {dx} x {dy} x {dz} ({volume} blocks)"
                    ))
                    .color_named(NamedColor::Aqua),
                )
                .await;

            sender
                .send_message(
                    TextComponent::text(format!(
                        "  From: ({}, {}, {})  To: ({}, {}, {})",
                        min.0.x, min.0.y, min.0.z, max.0.x, max.0.y, max.0.z
                    ))
                    .color_named(NamedColor::Gray),
                )
                .await;

            Ok(1)
        })
    }
}

// --- //clear ---
struct ClearExecutor;

#[async_trait]
impl CommandExecutor for ClearExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            // Air block state id is 0
            let air_state_id = Block::AIR.default_state.id;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        let pos = BlockPos(Vector3::new(x, y, z));
                        let old_state = world.get_block_state_id(&pos).await;
                        if old_state != air_state_id {
                            undo_blocks.push((pos, old_state));
                            world
                                .set_block_state(&pos, air_state_id, BlockFlags::FORCE_STATE)
                                .await;
                            count += 1;
                        }
                    }
                }
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) cleared."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// --- //hollow ---
struct HollowExecutor;

#[async_trait]
impl CommandExecutor for HollowExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let world = sender.world().ok_or(CommandError::InvalidRequirement)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let air_state_id = Block::AIR.default_state.id;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        // Only hollow the interior (not on any face)
                        let is_interior = x > min.0.x
                            && x < max.0.x
                            && y > min.0.y
                            && y < max.0.y
                            && z > min.0.z
                            && z < max.0.z;

                        if is_interior {
                            let pos = BlockPos(Vector3::new(x, y, z));
                            let old_state = world.get_block_state_id(&pos).await;
                            if old_state != air_state_id {
                                undo_blocks.push((pos, old_state));
                                world
                                    .set_block_state(&pos, air_state_id, BlockFlags::FORCE_STATE)
                                    .await;
                                count += 1;
                            }
                        }
                    }
                }
            }

            // Store undo data
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.undo_data = Some(undo_blocks);
            }

            sender
                .send_message(
                    TextComponent::text(format!("{count} block(s) hollowed out."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(count)
        })
    }
}

// ============================================================================
// Plugin Registration
// ============================================================================

const COMMAND_NAMES: [&str; 2] = ["we", "worldedit"];
const COMMAND_DESCRIPTION: &str = "WorldEdit commands for region editing.";

#[plugin_method]
async fn on_load(&mut self, server: Arc<Context>) -> Result<(), String> {
    server.init_log();

    log::info!("Pumpkin WorldEdit plugin loading...");

    // Build the command tree with all subcommands
    let command = CommandTree::new(COMMAND_NAMES, COMMAND_DESCRIPTION)
        .then(literal("pos1").execute(Pos1Executor))
        .then(literal("pos2").execute(Pos2Executor))
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
        .then(literal("copy").execute(CopyExecutor))
        .then(literal("paste").execute(PasteExecutor))
        .then(literal("undo").execute(UndoExecutor))
        .then(literal("size").execute(SizeExecutor))
        .then(literal("clear").execute(ClearExecutor))
        .then(literal("hollow").execute(HollowExecutor));

    // Register permission
    let permission = Permission::new(
        "pumpkin-worldedit:command.we",
        "Allows the player to use WorldEdit commands",
        PermissionDefault::Op(pumpkin_util::permission::PermissionLvl::Two),
    );

    server
        .register_permission(permission)
        .await
        .map_err(|e| format!("Failed to register permission: {e}"))?;

    // Register command
    server
        .register_command(command, "pumpkin-worldedit:command.we")
        .await;

    log::info!("Pumpkin WorldEdit loaded! Commands: /we <pos1|pos2|set|replace|walls|copy|paste|undo|size|clear|hollow>");

    Ok(())
}

#[plugin_impl]
pub struct MyPlugin {}

impl MyPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MyPlugin {
    fn default() -> Self {
        Self::new()
    }
}
