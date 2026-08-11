pub mod ipc;
pub mod pty;
pub mod session;

use anyhow::Result;
use axiom_core::config::AppConfig;
use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use crate::ipc::handle_connection;
use crate::session::SessionManager;

pub const SOCKET_NAME: &str = "axiom-daemon.sock";

pub fn socket_path() -> PathBuf {
    ProjectDirs::from("com", "axiom", "axiom")
        .map(|pd| pd.runtime_dir().unwrap_or(pd.cache_dir()).join(SOCKET_NAME))
        .unwrap_or_else(|| {
            std::env::temp_dir().join(SOCKET_NAME)
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ─────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Axiom daemon starting up");

    // ── Config ───────────────────────────────────────────────────────────────
    let config = AppConfig::load(None);

    // ── SQLite database ───────────────────────────────────────────────────────
    let db_path = ProjectDirs::from("com", "axiom", "axiom")
        .map(|pd| {
            let dir = pd.data_dir().to_path_buf();
            std::fs::create_dir_all(&dir).ok();
            dir.join("axiom.db")
        })
        .unwrap_or_else(|| PathBuf::from("/tmp/axiom.db"));

    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db = sqlx::SqlitePool::connect(&db_url).await?;
    sqlx::query(include_str!("schema.sql")).execute(&db).await?;

    // ── Session manager ────────────────────────────────────────────────────────
    let session_manager = Arc::new(RwLock::new(
        SessionManager::new(config.clone(), db.clone()).await?,
    ));

    // ── Unix socket ────────────────────────────────────────────────────────────
    let socket_path = socket_path();
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    info!("Daemon listening on {}", socket_path.display());

    // ── Accept connections ─────────────────────────────────────────────────────
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let sm = session_manager.clone();
                let cfg = config.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, sm, cfg).await {
                        error!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}
