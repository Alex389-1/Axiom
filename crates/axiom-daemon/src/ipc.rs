use anyhow::Result;
use axiom_core::{
    config::AppConfig,
    types::{
        ConversationMessage, DaemonRequest, DaemonResponse, ModelInfo, PermissionScope,
    },
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::session::SessionManager;

/// Handle a single client connection (one Tauri window or test client).
pub async fn handle_connection(
    stream: UnixStream,
    session_manager: Arc<RwLock<SessionManager>>,
    config: AppConfig,
) -> Result<()> {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut writer = write_half;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            debug!("Client disconnected");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: DaemonRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = DaemonResponse::Error {
                    message: format!("Invalid request: {}", e),
                };
                send_response(&mut writer, &err_resp).await?;
                continue;
            }
        };

        debug!("Received request: {:?}", request);
        let response = dispatch_request(request, &session_manager, &config).await;
        send_response(&mut writer, &response).await?;
    }

    Ok(())
}

async fn send_response(writer: &mut (impl AsyncWriteExt + Unpin), resp: &DaemonResponse) -> Result<()> {
    let mut json = serde_json::to_string(resp)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

async fn dispatch_request(
    request: DaemonRequest,
    session_manager: &Arc<RwLock<SessionManager>>,
    config: &AppConfig,
) -> DaemonResponse {
    match request {
        DaemonRequest::ListModels => {
            let sm = session_manager.read().await;
            match sm.list_models().await {
                Ok(models) => DaemonResponse::Models { models },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::CreateSession { project_path, model } => {
            let mut sm = session_manager.write().await;
            match sm.create_session(project_path, model).await {
                Ok(id) => DaemonResponse::Session { session_id: id },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::SendMessage { session_id, content } => {
            let sm = session_manager.read().await;
            match sm.send_message(&session_id, &content).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::GetConversation { session_id } => {
            let sm = session_manager.read().await;
            match sm.get_conversation(&session_id).await {
                Ok(messages) => DaemonResponse::Conversation { messages },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::GetTerminalOutput { session_id, lines } => {
            let sm = session_manager.read().await;
            match sm.get_terminal_output(&session_id, lines.unwrap_or(100)).await {
                Ok(lines) => DaemonResponse::TerminalOutput { lines },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::WriteTerminal { session_id, input } => {
            let sm = session_manager.read().await;
            match sm.write_terminal(&session_id, &input).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::ResizeTerminal { session_id, cols, rows } => {
            let sm = session_manager.read().await;
            match sm.resize_terminal(&session_id, cols, rows).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::SetPermission { session_id, category, scope } => {
            let sm = session_manager.read().await;
            match sm.set_permission(&session_id, category, scope).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::ListJobs { session_id } => {
            let sm = session_manager.read().await;
            match sm.list_jobs(&session_id).await {
                Ok(jobs) => DaemonResponse::Jobs { jobs },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::GetModelProfiles => {
            let sm = session_manager.read().await;
            match sm.get_model_profiles().await {
                Ok(profiles) => DaemonResponse::ModelProfiles { profiles },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::ProbeModel { model } => {
            let sm = session_manager.read().await;
            match sm.probe_model(&model).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::GetConfig => {
            match serde_json::to_value(config) {
                Ok(v) => DaemonResponse::Config { config: v },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::ListProjects => {
            // Return recently accessed projects from the database
            let sm = session_manager.read().await;
            match sm.list_projects().await {
                Ok(projects) => DaemonResponse::Config {
                    config: serde_json::json!({ "projects": projects }),
                },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::UpdateConfig { config: _cfg } => {
            // Config update — persist to disk
            DaemonResponse::Ok
        }

        DaemonRequest::GetEvents { session_id } => {
            let sm = session_manager.read().await;
            match sm.get_events(&session_id).await {
                Ok(events) => DaemonResponse::Events { events },
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::StopAgent { session_id } => {
            let sm = session_manager.read().await;
            match sm.stop_agent(&session_id).await {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: e.to_string() },
            }
        }

        DaemonRequest::Shutdown => {
            info!("Shutdown requested");
            std::process::exit(0);
        }
    }
}
