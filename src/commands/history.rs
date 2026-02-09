use async_trait::async_trait;
use pumpkin::{
    command::{
        args::ConsumedArgs, dispatcher::CommandError, CommandExecutor, CommandResult, CommandSender,
    },
    server::Server,
};
use pumpkin_util::text::{color::NamedColor, TextComponent};
use pumpkin_world::world::BlockFlags;

use crate::state::{sender_uuid, sender_world, PLAYER_DATA};

// ============================================================================
// //undo
// ============================================================================

pub struct UndoExecutor;

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
            let world = sender_world(sender)?;

            // Take undo data out of state (releases the lock before async work)
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
