use axiom_core::types::{DaemonRequest, DaemonResponse};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tauri::AppHandle;
use tracing::{error, info, warn};

fn daemon_socket_path() -> std::path::PathBuf {
    directories::ProjectDirs::from("com", "axiom", "axiom")
        .map(|pd| {
            pd.runtime_dir()
                .unwrap_or(pd.cache_dir())
                .join("axiom-daemon.sock")
        })
        .unwrap_or_else(|| std::env::temp_dir().join("axiom-daemon.sock"))
}

/// Manages the Unix socket connection to the daemon process.
pub struct DaemonClient {
    stream: Mutex<Option<UnixStream>>,
    /// Pending events queued per session (from daemon push messages).
    events: Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>,
}

impl DaemonClient {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn connect(&self) -> Result<(), String> {
        let socket = daemon_socket_path();
        match UnixStream::connect(&socket).await {
            Ok(s) => {
                *self.stream.lock().await = Some(s);
                Ok(())
            }
            Err(e) => Err(format!(
                "Cannot connect to daemon at {}: {}",
                socket.display(),
                e
            )),
        }
    }

    /// Send a request and wait for the response (single-threaded request/response).
    pub async fn send(&self, request: DaemonRequest) -> Result<DaemonResponse, String> {
        // Ensure connected
        {
            let guard = self.stream.lock().await;
            if guard.is_none() {
                drop(guard);
                self.connect().await?;
            }
        }

        let mut guard = self.stream.lock().await;
        let stream = guard.as_mut().ok_or("Not connected")?;

        // Write request
        let mut json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        json.push('\n');
        stream
            .write_all(json.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;

        // Read response line (collect raw bytes to prevent UTF-8 corruption of multi-byte characters like emojis)
        let mut response_bytes = Vec::new();
        let mut buf = [0u8; 1];
        loop {
            use tokio::io::AsyncReadExt;
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(_) => {
                    if buf[0] == b'\n' {
                        break;
                    }
                    response_bytes.push(buf[0]);
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        let response_line = String::from_utf8(response_bytes).map_err(|e| format!("UTF-8 error: {}", e))?;

        serde_json::from_str(&response_line).map_err(|e| format!("Parse error: {}: {}", e, response_line))
    }

    /// Drain queued events for a session (called by the poll_events command).
    pub async fn drain_events(&self, session_id: &str) -> Vec<serde_json::Value> {
        let mut events = self.events.lock().await;
        events.remove(session_id).unwrap_or_default()
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Start the daemon process if it's not already running.
/// State management is handled by lib.rs setup hook.
pub async fn start_daemon_if_needed(app: &AppHandle) -> anyhow::Result<()> {
    let socket = daemon_socket_path();

    // Try connecting first — maybe it's already running
    if UnixStream::connect(&socket).await.is_ok() {
        info!("Daemon already running at {}", socket.display());
        return Ok(());
    }

    info!("Starting axiom-daemon...");

    // Find the daemon binary next to the current executable
    let daemon_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("axiom-daemon")))
        .unwrap_or_else(|| std::path::PathBuf::from("axiom-daemon"));

    if daemon_bin.exists() {
        tokio::process::Command::new(&daemon_bin)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

        // Wait for socket to appear
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if socket.exists() {
                break;
            }
        }
    } else {
        warn!("Daemon binary not found at {}; skipping auto-start", daemon_bin.display());
    }

    Ok(())
}
