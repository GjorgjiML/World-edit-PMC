use async_trait::async_trait;
use pumpkin::{
    command::{args::ConsumedArgs, CommandExecutor, CommandResult, CommandSender},
    server::Server,
};
use pumpkin_util::text::{color::NamedColor, TextComponent};

use crate::state::{
    get_selection, sender_block_pos, sender_uuid, selection_volume, PLAYER_DATA,
};

// ============================================================================
// //pos1
// ============================================================================

pub struct Pos1Executor;

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

// ============================================================================
// //pos2
// ============================================================================

pub struct Pos2Executor;

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

// ============================================================================
// //size
// ============================================================================

pub struct SizeExecutor;

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
            let volume = selection_volume(&min, &max);

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
