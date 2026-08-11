use axiom_core::types::{
    DaemonRequest, DaemonResponse, PermissionCategory, PermissionScope,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::daemon::DaemonClient;

fn map_response<T, F>(resp: DaemonResponse, extract: F) -> Result<T, String>
where
    F: Fn(DaemonResponse) -> Option<T>,
{
    match resp {
        DaemonResponse::Error { message } => Err(message),
        other => extract(other).ok_or_else(|| "Unexpected response".to_string()),
    }
}

#[tauri::command]
pub async fn list_models(client: State<'_, DaemonClient>) -> Result<serde_json::Value, String> {
    let resp = client.send(DaemonRequest::ListModels).await?;
    match resp {
        DaemonResponse::Models { models } => Ok(serde_json::to_value(models).unwrap()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn create_session(
    client: State<'_, DaemonClient>,
    project_path: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    let resp = client
        .send(DaemonRequest::CreateSession { project_path, model })
        .await?;
    match resp {
        DaemonResponse::Session { session_id } => Ok(session_id),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn send_message(
    client: State<'_, DaemonClient>,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let resp = client
        .send(DaemonRequest::SendMessage { session_id, content })
        .await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn stop_agent(
    client: State<'_, DaemonClient>,
    session_id: String,
) -> Result<(), String> {
    let resp = client
        .send(DaemonRequest::StopAgent { session_id })
        .await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_conversation(
    client: State<'_, DaemonClient>,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let resp = client
        .send(DaemonRequest::GetConversation { session_id })
        .await?;
    match resp {
        DaemonResponse::Conversation { messages } => Ok(serde_json::to_value(messages).unwrap()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_terminal_output(
    client: State<'_, DaemonClient>,
    session_id: String,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let resp = client
        .send(DaemonRequest::GetTerminalOutput { session_id, lines })
        .await?;
    match resp {
        DaemonResponse::TerminalOutput { lines } => Ok(lines),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn write_terminal(
    client: State<'_, DaemonClient>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let resp = client
        .send(DaemonRequest::WriteTerminal { session_id, input })
        .await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn resize_terminal(
    client: State<'_, DaemonClient>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let resp = client
        .send(DaemonRequest::ResizeTerminal { session_id, cols, rows })
        .await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn set_permission(
    client: State<'_, DaemonClient>,
    session_id: String,
    category: PermissionCategory,
    scope: PermissionScope,
) -> Result<(), String> {
    let resp = client
        .send(DaemonRequest::SetPermission { session_id, category, scope })
        .await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn list_jobs(
    client: State<'_, DaemonClient>,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let resp = client
        .send(DaemonRequest::ListJobs { session_id })
        .await?;
    match resp {
        DaemonResponse::Jobs { jobs } => Ok(serde_json::to_value(jobs).unwrap()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_model_profiles(
    client: State<'_, DaemonClient>,
) -> Result<serde_json::Value, String> {
    let resp = client.send(DaemonRequest::GetModelProfiles).await?;
    match resp {
        DaemonResponse::ModelProfiles { profiles } => Ok(serde_json::to_value(profiles).unwrap()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn probe_model(
    client: State<'_, DaemonClient>,
    model: String,
) -> Result<(), String> {
    let resp = client.send(DaemonRequest::ProbeModel { model }).await?;
    match resp {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn list_projects(
    client: State<'_, DaemonClient>,
) -> Result<serde_json::Value, String> {
    let resp = client.send(DaemonRequest::ListProjects).await?;
    match resp {
        DaemonResponse::Config { config } => Ok(config),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_config(
    client: State<'_, DaemonClient>,
) -> Result<serde_json::Value, String> {
    let resp = client.send(DaemonRequest::GetConfig).await?;
    match resp {
        DaemonResponse::Config { config } => Ok(config),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

/// Poll for pending events from the daemon (streaming tokens, tool results, permission requests).
/// The frontend polls this on a tight interval to receive live updates.
#[tauri::command]
pub async fn poll_events(
    client: State<'_, DaemonClient>,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let resp = client.send(DaemonRequest::GetEvents { session_id }).await?;
    match resp {
        DaemonResponse::Events { events } => Ok(serde_json::to_value(events).unwrap()),
        DaemonResponse::Error { message } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}
