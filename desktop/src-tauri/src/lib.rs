pub mod adapters;
mod application;
mod commands;
pub mod domain;

use std::io;

use adapters::{CredentialService, LocalDataPaths};
use application::KernelService;
use commands::KernelState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let data_paths = LocalDataPaths::new(app_data_dir);
            data_paths
                .prepare()
                .map_err(|error| io::Error::other(error.to_string()))?;
            let service =
                KernelService::open_with_backup_dir(&data_paths.database, &data_paths.backups)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            app.manage(KernelState::new(service, CredentialService::os_default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_kernel,
            commands::create_conversation,
            commands::load_conversation_graph,
            commands::append_turn,
            commands::complete_turn,
            commands::get_context_snapshot,
            commands::update_node_position,
            commands::set_provider_credential,
            commands::has_provider_credential,
            commands::delete_provider_credential,
            commands::list_providers,
            commands::run_model,
            commands::cancel_model_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the MindScape desktop application");
}
