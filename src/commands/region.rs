use async_trait::async_trait;
use pumpkin::{
    command::{
        args::{block::BlockArgumentConsumer, ConsumedArgs, FindArg},
        CommandExecutor, CommandResult, CommandSender,
    },
    server::Server,
};
use pumpkin_data::Block;
use pumpkin_util::{
    math::{position::BlockPos, vector3::Vector3},
    text::{color::NamedColor, TextComponent},
};
use pumpkin_world::world::BlockFlags;

use crate::state::{
    check_selection_size, get_selection, sender_uuid, sender_world, PLAYER_DATA,
};

/// Argument name used for single-block commands (set, walls).
pub const ARG_BLOCK: &str = "block";
/// Argument name for the source block in replace.
pub const ARG_FROM: &str = "from";
/// Argument name for the target block in replace.
pub const ARG_TO: &str = "to";

// ============================================================================
// //set <block>
// ============================================================================

pub struct SetExecutor;

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
            let world = sender_world(sender)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

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

// ============================================================================
// //replace <from> <to>
// ============================================================================

pub struct ReplaceExecutor;

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
            let world = sender_world(sender)?;

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

// ============================================================================
// //walls <block>
// ============================================================================

pub struct WallsExecutor;

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
            let world = sender_world(sender)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
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

// ============================================================================
// //clear
// ============================================================================

pub struct ClearExecutor;

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
            let world = sender_world(sender)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

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

// ============================================================================
// //hollow
// ============================================================================

pub struct HollowExecutor;

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
            let world = sender_world(sender)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let air_state_id = Block::AIR.default_state.id;

            let mut undo_blocks = Vec::new();
            let mut count = 0i32;

            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
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
