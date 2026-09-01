pub mod adapters;
mod application;
mod commands;
pub mod domain;

use std::io;

use adapters::{
    CredentialService, ImportStorage, LocalDataPaths, MarkdownVault, SemanticModelPack,
};
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
            let import_storage = ImportStorage::new(&data_paths.imports);
            let markdown_vault = MarkdownVault::new(&data_paths.vault)
                .map_err(|error| io::Error::other(error.to_string()))?;
            markdown_vault
                .recover_interrupted_writes()
                .map_err(|error| io::Error::other(error.to_string()))?;
            import_storage
                .recover_interrupted_writes()
                .map_err(|error| io::Error::other(error.to_string()))?;
            let service =
                KernelService::open_with_backup_dir(&data_paths.database, &data_paths.backups)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            markdown_vault
                .recover_focus_promotion_transactions(
                    &service
                        .list_all_focus_promotion_decision_ids()
                        .map_err(|error| io::Error::other(error.to_string()))?,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
            service
                .recover_knowledge_entity_delete_vault(&markdown_vault)
                .map_err(|error| io::Error::other(error.to_string()))?;
            service
                .recover_discussion_vault(&markdown_vault)
                .map_err(|error| io::Error::other(error.to_string()))?;
            service
                .recover_interrupted_runs()
                .map_err(|error| io::Error::other(error.to_string()))?;
            markdown_vault
                .reconcile_entity_files(
                    &service
                        .list_all_knowledge_entity_ids()
                        .map_err(|error| io::Error::other(error.to_string()))?,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
            import_storage
                .reconcile_unreferenced(
                    &service
                        .list_import_storage_refs()
                        .map_err(|error| io::Error::other(error.to_string()))?,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
            app.manage(KernelState::new(
                service,
                CredentialService::os_default(),
                import_storage,
                markdown_vault,
                SemanticModelPack::new(&data_paths.models),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_kernel,
            commands::get_semantic_model_pack_status,
            commands::install_semantic_model_pack,
            commands::persist_import_bundle,
            commands::import_generic_file,
            commands::list_import_sources,
            commands::get_import_bundle,
            commands::get_raw_import_content,
            commands::create_focus_frame,
            commands::get_focus_frame_query,
            commands::get_focus_promotion_candidates,
            commands::decide_focus_promotion,
            commands::get_focus_promotion_decision,
            commands::list_focus_promotion_decisions,
            commands::save_focused_context_snapshot,
            commands::upsert_knowledge_entity,
            commands::list_knowledge_entities,
            commands::upsert_knowledge_relation,
            commands::list_knowledge_relations,
            commands::retrieve_knowledge,
            commands::rebuild_knowledge_vector_index,
            commands::delete_knowledge_entity,
            commands::upsert_evidence_ref,
            commands::project_knowledge_entity_markdown,
            commands::list_markdown_projections,
            commands::import_markdown_entity_edit,
            commands::project_discussion_log_markdown,
            commands::get_discussion_log,
            commands::list_conversation_discussion_logs,
            commands::list_project_discussion_logs,
            commands::import_discussion_log_edit,
            commands::list_focus_frames,
            commands::close_focus_frame,
            commands::reopen_focus_frame,
            commands::create_conversation,
            commands::load_conversation_graph,
            commands::append_turn,
            commands::complete_turn,
            commands::get_context_snapshot,
            commands::update_node_position,
            commands::save_canvas_viewport,
            commands::get_canvas_viewport,
            commands::set_provider_credential,
            commands::has_provider_credential,
            commands::delete_provider_credential,
            commands::list_providers,
            commands::test_provider_connection,
            commands::run_model,
            commands::start_model_run,
            commands::list_model_runs,
            commands::cancel_model_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the MindScape desktop application");
}
