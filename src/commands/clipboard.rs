use async_trait::async_trait;
use pumpkin::{
    command::{
        args::ConsumedArgs, dispatcher::CommandError, CommandExecutor, CommandResult, CommandSender,
    },
    server::Server,
};
use pumpkin_util::{
    math::{position::BlockPos, vector3::Vector3},
    text::{color::NamedColor, TextComponent},
};
use pumpkin_world::world::BlockFlags;

use crate::state::{
    check_selection_size, get_selection, sender_block_pos, sender_uuid, sender_world,
    ClipboardData, PLAYER_DATA,
};

// ============================================================================
// //copy
// ============================================================================

pub struct CopyExecutor;

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
            let world = sender_world(sender)?;

            let (min, max) = get_selection(&player_id)?;
            check_selection_size(&min, &max)?;

            let mut blocks = Vec::new();
            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    for z in min.0.z..=max.0.z {
                        let pos = BlockPos(Vector3::new(x, y, z));
                        let state_id = world.get_block_state_id(&pos).await;
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

// ============================================================================
// //paste
// ============================================================================

pub struct PasteExecutor;

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
            let world = sender_world(sender)?;

            // Clone clipboard data so the lock is released before async work
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
