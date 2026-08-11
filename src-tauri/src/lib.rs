mod audio;
mod commands;
mod daemon;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Register the DaemonClient as managed state so commands can access it
            app.manage(daemon::DaemonClient::new());
            app.manage(audio::AudioRecorderState::default());

            // Start the daemon if it's not already running (fire-and-forget)
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = daemon::start_daemon_if_needed(&handle).await {
                    tracing::error!("Failed to start daemon: {}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_models,
            commands::create_session,
            commands::send_message,
            commands::delete_last_turn,
            commands::get_conversation,
            commands::get_terminal_output,
            commands::write_terminal,
            commands::resize_terminal,
            commands::set_permission,
            commands::list_jobs,
            commands::get_model_profiles,
            commands::probe_model,
            commands::list_projects,
            commands::get_config,
            commands::poll_events,
            commands::stop_agent,
            commands::start_recording_cmd,
            commands::stop_recording_and_transcribe_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Axiom");
}
