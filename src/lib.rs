//! Pumpkin WorldEdit plugin: region selection, block operations, clipboard, undo, and schematics (.schem / .litematic).

mod commands;
mod schematic;
mod state;

use std::sync::Arc;

use pumpkin::plugin::Context;
use pumpkin_api_macros::{plugin_impl, plugin_method};
use pumpkin_util::permission::{Permission, PermissionDefault};

#[plugin_method]
async fn on_load(&mut self, server: Arc<Context>) -> Result<(), String> {
    server.init_log();

    log::info!("Pumpkin WorldEdit plugin loading...");

    // Set up schematics directory
    let schematics_dir = server.get_data_folder().join("schematics");
    if !schematics_dir.exists() {
        std::fs::create_dir_all(&schematics_dir)
            .map_err(|e| format!("Failed to create schematics directory: {e}"))?;
    }
    let _ = state::SCHEMATICS_DIR.set(schematics_dir.clone());
    log::info!("Schematics directory: {}", schematics_dir.display());

    // Build command tree
    let command = commands::build_command_tree();

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

    log::info!(
        "Pumpkin WorldEdit loaded! Commands: /we <pos1|pos2|set|replace|walls|copy|paste|undo|size|clear|hollow|schem>"
    );

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
