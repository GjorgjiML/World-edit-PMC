use async_trait::async_trait;
use pumpkin::{
    command::{
        args::{simple::SimpleArgConsumer, ConsumedArgs, FindArg},
        dispatcher::CommandError,
        CommandExecutor, CommandResult, CommandSender,
    },
    server::Server,
};
use pumpkin_util::text::{color::NamedColor, TextComponent};

use crate::schematic;
use crate::state::{sender_uuid, ClipboardData, PLAYER_DATA, SCHEMATICS_DIR};

pub const ARG_SCHEM_NAME: &str = "name";

/// Helper: get the schematics directory path.
fn get_schematics_dir() -> Result<std::path::PathBuf, CommandError> {
    SCHEMATICS_DIR
        .get()
        .cloned()
        .ok_or(CommandError::CommandFailed(
            TextComponent::text("Schematics directory not initialized.")
                .color_named(NamedColor::Red),
        ))
}

// ============================================================================
// /we schem load <name>
// ============================================================================

pub struct SchemLoadExecutor;

#[async_trait]
impl CommandExecutor for SchemLoadExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let schem_name = SimpleArgConsumer::find_arg(args, ARG_SCHEM_NAME)?;

            let schematics_dir = get_schematics_dir()?;

            // Resolve path: accept name with or without .schem/.litematic extension
            let file_path = if schem_name.ends_with(".schem") || schem_name.ends_with(".litematic") {
                schematics_dir.join(schem_name)
            } else {
                let schem_path = schematics_dir.join(format!("{schem_name}.schem"));
                let litematic_path = schematics_dir.join(format!("{schem_name}.litematic"));
                if schem_path.exists() {
                    schem_path
                } else if litematic_path.exists() {
                    litematic_path
                } else {
                    schematics_dir.join(format!("{schem_name}.schem")) // will fail below
                }
            };

            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(schem_name);

            if !file_path.exists() {
                return Err(CommandError::CommandFailed(
                    TextComponent::text(format!(
                        "Schematic '{schem_name}' not found (tried .schem and .litematic)."
                    ))
                    .color_named(NamedColor::Red),
                ));
            }

            sender
                .send_message(
                    TextComponent::text(format!("Loading schematic '{filename}'..."))
                        .color_named(NamedColor::Yellow),
                )
                .await;

            // Load schematic (blocking I/O, done on the current task)
            let schem_data = schematic::load_schematic(&file_path).map_err(|e| {
                CommandError::CommandFailed(
                    TextComponent::text(format!("Failed to load schematic: {e}"))
                        .color_named(NamedColor::Red),
                )
            })?;

            let block_count = schem_data.blocks.len();
            let width = schem_data.width;
            let height = schem_data.height;
            let length = schem_data.length;

            // Store in clipboard
            let clipboard = schematic::schematic_to_clipboard(&schem_data);
            {
                let mut state = PLAYER_DATA.lock().unwrap();
                let data = state.entry(player_id).or_default();
                data.clipboard = Some(clipboard);
            }

            sender
                .send_message(
                    TextComponent::text(format!(
                        "Schematic '{filename}' loaded into clipboard ({width}x{height}x{length}, {block_count} blocks). Use /we paste to place it."
                    ))
                    .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(block_count as i32)
        })
    }
}

// ============================================================================
// /we schem save <name>
// ============================================================================

pub struct SchemSaveExecutor;

#[async_trait]
impl CommandExecutor for SchemSaveExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let player_id = sender_uuid(sender)?;
            let schem_name = SimpleArgConsumer::find_arg(args, ARG_SCHEM_NAME)?;

            let schematics_dir = get_schematics_dir()?;

            // Get clipboard data
            let clipboard_blocks = {
                let state = PLAYER_DATA.lock().unwrap();
                let data = state.get(&player_id).ok_or(CommandError::CommandFailed(
                    TextComponent::text("Clipboard is empty. Use /we copy first.")
                        .color_named(NamedColor::Red),
                ))?;
                let clipboard =
                    data.clipboard.as_ref().ok_or(CommandError::CommandFailed(
                        TextComponent::text("Clipboard is empty. Use /we copy first.")
                            .color_named(NamedColor::Red),
                    ))?;
                clipboard.blocks.clone()
            };

            // Build file path
            let filename = if schem_name.ends_with(".schem") {
                schem_name.to_string()
            } else {
                format!("{schem_name}.schem")
            };
            let file_path = schematics_dir.join(&filename);

            sender
                .send_message(
                    TextComponent::text(format!("Saving schematic '{filename}'..."))
                        .color_named(NamedColor::Yellow),
                )
                .await;

            let clipboard_data = ClipboardData {
                blocks: clipboard_blocks,
            };

            schematic::save_schematic(&file_path, &clipboard_data).map_err(|e| {
                CommandError::CommandFailed(
                    TextComponent::text(format!("Failed to save schematic: {e}"))
                        .color_named(NamedColor::Red),
                )
            })?;

            sender
                .send_message(
                    TextComponent::text(format!("Schematic saved as '{filename}'."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(1)
        })
    }
}

// ============================================================================
// /we schem list
// ============================================================================

pub struct SchemListExecutor;

#[async_trait]
impl CommandExecutor for SchemListExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let schematics_dir = get_schematics_dir()?;

            if !schematics_dir.exists() {
                sender
                    .send_message(
                        TextComponent::text("No schematics found.")
                            .color_named(NamedColor::Yellow),
                    )
                    .await;
                return Ok(0);
            }

            let entries = std::fs::read_dir(&schematics_dir).map_err(|e| {
                CommandError::CommandFailed(
                    TextComponent::text(format!("Failed to read schematics directory: {e}"))
                        .color_named(NamedColor::Red),
                )
            })?;

            let mut schem_files: Vec<String> = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "schem" || ext == "litematic") {
                    if let Some(name) = path.file_stem() {
                        schem_files.push(name.to_string_lossy().to_string());
                    }
                }
            }

            if schem_files.is_empty() {
                sender
                    .send_message(
                        TextComponent::text("No schematics found.")
                            .color_named(NamedColor::Yellow),
                    )
                    .await;
                return Ok(0);
            }

            schem_files.sort();

            sender
                .send_message(
                    TextComponent::text(format!(
                        "--- Schematics ({}) ---",
                        schem_files.len()
                    ))
                    .color_named(NamedColor::Gold),
                )
                .await;

            for name in &schem_files {
                sender
                    .send_message(
                        TextComponent::text(format!("  - {name}")).color_named(NamedColor::Green),
                    )
                    .await;
            }

            Ok(schem_files.len() as i32)
        })
    }
}

// ============================================================================
// /we schem delete <name>
// ============================================================================

pub struct SchemDeleteExecutor;

#[async_trait]
impl CommandExecutor for SchemDeleteExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let schem_name = SimpleArgConsumer::find_arg(args, ARG_SCHEM_NAME)?;

            let schematics_dir = get_schematics_dir()?;

            // Resolve path: try .schem then .litematic if no extension given
            let file_path = if schem_name.ends_with(".schem") || schem_name.ends_with(".litematic") {
                schematics_dir.join(schem_name)
            } else {
                let schem_path = schematics_dir.join(format!("{schem_name}.schem"));
                let litematic_path = schematics_dir.join(format!("{schem_name}.litematic"));
                if schem_path.exists() {
                    schem_path
                } else if litematic_path.exists() {
                    litematic_path
                } else {
                    schematics_dir.join(format!("{schem_name}.schem"))
                }
            };

            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(schem_name);

            if !file_path.exists() {
                return Err(CommandError::CommandFailed(
                    TextComponent::text(format!(
                        "Schematic '{schem_name}' not found (tried .schem and .litematic)."
                    ))
                    .color_named(NamedColor::Red),
                ));
            }

            std::fs::remove_file(&file_path).map_err(|e| {
                CommandError::CommandFailed(
                    TextComponent::text(format!("Failed to delete schematic: {e}"))
                        .color_named(NamedColor::Red),
                )
            })?;

            sender
                .send_message(
                    TextComponent::text(format!("Schematic '{filename}' deleted."))
                        .color_named(NamedColor::Aqua),
                )
                .await;

            Ok(1)
        })
    }
}
