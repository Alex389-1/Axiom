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
    app_handle: Option<AppHandle>,
    /// Pending events queued per session (from daemon push messages).
    events: Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>,
}

impl DaemonClient {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            stream: Mutex::new(None),
            app_handle: Some(app_handle),
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn connect(&self) -> Result<(), String> {
        let socket = daemon_socket_path();
        if let Ok(s) = UnixStream::connect(&socket).await {
            *self.stream.lock().await = Some(s);
            return Ok(());
        }

        if let Some(app) = &self.app_handle {
            info!("Daemon connection failed; attempting auto-start...");
            if let Err(e) = start_daemon_if_needed(app).await {
                return Err(format!("Failed to start daemon: {}", e));
            }
            if let Ok(s) = UnixStream::connect(&socket).await {
                *self.stream.lock().await = Some(s);
                return Ok(());
            }
        }

        Err(format!("Cannot connect to daemon at {}", socket.display()))
    }

    /// Send a request and wait for the response (auto-reconnects and retries on failure).
    pub async fn send(&self, request: DaemonRequest) -> Result<DaemonResponse, String> {
        for attempt in 0..2 {
            {
                let guard = self.stream.lock().await;
                if guard.is_none() {
                    drop(guard);
                    if let Err(e) = self.connect().await {
                        if attempt == 1 {
                            return Err(e);
                        }
                        continue;
                    }
                }
            }

            let mut guard = self.stream.lock().await;
            let stream = match guard.as_mut() {
                Some(s) => s,
                None => continue,
            };

            let mut json = match serde_json::to_string(&request) {
                Ok(j) => j,
                Err(e) => return Err(e.to_string()),
            };
            json.push('\n');

            if stream.write_all(json.as_bytes()).await.is_err() || stream.flush().await.is_err() {
                *guard = None;
                continue;
            }

            let mut response_bytes = Vec::new();
            let mut buf = [0u8; 1];
            let mut read_ok = true;
            loop {
                use tokio::io::AsyncReadExt;
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        read_ok = false;
                        break;
                    }
                    Ok(_) => {
                        if buf[0] == b'\n' {
                            break;
                        }
                        response_bytes.push(buf[0]);
                    }
                    Err(_) => {
                        read_ok = false;
                        break;
                    }
                }
            }

            if !read_ok || response_bytes.is_empty() {
                *guard = None;
                continue;
            }

            let response_line = match String::from_utf8(response_bytes) {
                Ok(l) => l,
                Err(e) => return Err(format!("UTF-8 error: {}", e)),
            };

            match serde_json::from_str(&response_line) {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if attempt == 1 {
                        return Err(format!("Parse error: {}: {}", e, response_line));
                    }
                    *guard = None;
                    continue;
                }
            }
        }

        Err("Daemon communication failed after retries".to_string())
    }

    /// Drain queued events for a session (called by the poll_events command).
    pub async fn drain_events(&self, session_id: &str) -> Vec<serde_json::Value> {
        let mut events = self.events.lock().await;
        events.remove(session_id).unwrap_or_default()
    }
}

/// Start the daemon process if it's not already running.
/// State management is handled by lib.rs setup hook.
pub async fn start_daemon_if_needed(app: &AppHandle) -> anyhow::Result<()> {
    let socket = daemon_socket_path();

    // Try connecting — if connect succeeds, daemon is up and listening
    match UnixStream::connect(&socket).await {
        Ok(_) => {
            info!("Daemon already running at {}", socket.display());
            return Ok(());
        }
        Err(e) => {
            // If socket file exists but connection was refused, remove the stale socket file
            if socket.exists() {
                warn!("Removing stale daemon socket at {}: {}", socket.display(), e);
                let _ = std::fs::remove_file(&socket);
            }
        }
    }

    info!("Starting axiom-daemon...");

    // Ensure the socket directory exists before spawning
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Find the daemon binary across multiple candidate locations
    use tauri::Manager;
    let mut candidate_paths = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidate_paths.push(parent.join("axiom-daemon"));
            candidate_paths.push(parent.join("../lib/Axiom/axiom-daemon"));
            candidate_paths.push(parent.join("../lib/Axiom/_up_/target/release/axiom-daemon"));
            candidate_paths.push(parent.join("../lib/axiom-tauri/axiom-daemon"));
            candidate_paths.push(parent.join("../lib/axiom-tauri/_up_/target/release/axiom-daemon"));
        }
    }
    if let Ok(res_dir) = app.path().resource_dir() {
        candidate_paths.push(res_dir.join("axiom-daemon"));
        candidate_paths.push(res_dir.join("_up_/target/release/axiom-daemon"));
    }
    candidate_paths.push(std::path::PathBuf::from("/opt/Axiom/usr/bin/axiom-daemon"));
    candidate_paths.push(std::path::PathBuf::from("/opt/Axiom/usr/lib/Axiom/_up_/target/release/axiom-daemon"));
    candidate_paths.push(std::path::PathBuf::from("/usr/bin/axiom-daemon"));
    candidate_paths.push(std::path::PathBuf::from("/usr/local/bin/axiom-daemon"));

    let daemon_bin = candidate_paths.into_iter().find(|p| p.exists());

    if let Some(daemon_bin) = daemon_bin {
        info!("Spawning daemon from {}", daemon_bin.display());
        tokio::process::Command::new(&daemon_bin)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

        // Poll until we can actually connect (not just until the socket file appears)
        let mut connected = false;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if UnixStream::connect(&socket).await.is_ok() {
                connected = true;
                break;
            }
        }

        if connected {
            info!("Daemon ready at {}", socket.display());
        } else {
            warn!("Daemon did not become ready within 10s at {}", socket.display());
        }
    } else {
        warn!("Daemon binary not found in candidate paths; skipping auto-start");
    }

    Ok(())
}
