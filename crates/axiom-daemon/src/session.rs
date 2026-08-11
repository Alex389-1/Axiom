use anyhow::Result;
use axiom_core::{
    config::{AppConfig, ProviderKind},
    types::{
        ConversationMessage, DaemonEvent, Job, ModelInfo, ModelProfile, PermissionCategory,
        PermissionScope,
    },
};
use axiom_agent::{
    planner::{AgentPlanner, PermissionResponse, PlannerEvent},
    probe::ModelProber,
    providers::{LlamaCppProvider, ModelProvider, OllamaProvider},
};
use axiom_tools::default_registry;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;
use chrono::Utc;

/// One active session (one conversation + one terminal).
pub struct Session {
    pub id: String,
    pub project_path: Option<PathBuf>,
    pub model: String,
    pub planner: Arc<tokio::sync::Mutex<AgentPlanner>>,
    pub permission_tx: mpsc::Sender<PermissionResponse>,
    pub event_queue: Arc<tokio::sync::Mutex<Vec<DaemonEvent>>>,
    pub terminal: Arc<tokio::sync::Mutex<crate::pty::PtySession>>,
    pub jobs: Arc<tokio::sync::Mutex<Vec<Job>>>,
    pub active_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub struct SessionManager {
    config: AppConfig,
    db: SqlitePool,
    sessions: HashMap<String, Arc<Session>>,
    provider: Arc<dyn ModelProvider>,
    prober: ModelProber,
}

impl SessionManager {
    pub async fn new(config: AppConfig, db: SqlitePool) -> Result<Self> {
        let provider: Arc<dyn ModelProvider> = match config.model.provider {
            ProviderKind::Ollama => Arc::new(OllamaProvider::new(&config.model.ollama_url)),
            ProviderKind::LlamaCpp => Arc::new(LlamaCppProvider::new(&config.model.llamacpp_url)),
        };

        Ok(Self {
            config,
            db,
            sessions: HashMap::new(),
            provider,
            prober: ModelProber::new(),
        })
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(self.provider.list_models().await.map_err(|e| anyhow::anyhow!(e.to_string()))?)
    }

    pub async fn create_session(
        &mut self,
        project_path: Option<String>,
        model: Option<String>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let model = model.unwrap_or_else(|| self.config.model.model.clone());
        let project = project_path.as_ref().map(PathBuf::from);

        // Get or probe model profile (fallback to Structured mode if probe fails)
        let profile = match self.prober.get_cached(&model) {
            Some(p) => p,
            None => {
                info!("No cached profile for {}; probing...", model);
                self.prober.probe(&model, self.provider.as_ref()).await.unwrap_or_else(|_| ModelProfile {
                    model: model.clone(),
                    tool_mode: axiom_core::types::ToolCallingMode::Structured,
                    supports_thinking: false,
                    supports_constrained_decoding: self.provider.supports_constrained_decoding(),
                    detected_at: "fallback".to_string(),
                    last_verified: Utc::now(),
                    model_hash: None,
                })
            }
        };

        // Create tool registry
        let working_dir = project.clone().unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"))
        });
        let registry = Arc::new(default_registry(working_dir.clone()));

        // Create event channel
        let (event_tx, event_rx) = mpsc::channel(256);

        // Create planner
        let planner = AgentPlanner::new(
            self.config.clone(),
            self.provider.clone(),
            registry,
            profile,
            project.clone(),
            event_tx,
        );
        let perm_tx = planner.permission_sender();

        // Create PTY session
        let shell = if self.config.terminal.shell.is_empty() {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
        } else {
            self.config.terminal.shell.clone()
        };

        let pty = crate::pty::PtySession::new(&shell, &working_dir).await
            .map_err(|e| anyhow::anyhow!("PTY error: {}", e))?;

        // Background event queue consumer
        let event_queue = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let eq_bg = event_queue.clone();
        let sid_bg = session_id.clone();
        let db_bg = self.db.clone();

        tokio::spawn(async move {
            let mut rx = event_rx;
            while let Some(evt) = rx.recv().await {
                let daemon_evt = match evt {
                    PlannerEvent::Token(t) => DaemonEvent::Token { session_id: sid_bg.clone(), token: t },
                    PlannerEvent::ToolCallStarted { call, step } => DaemonEvent::ToolCallStarted { session_id: sid_bg.clone(), call, step },
                    PlannerEvent::ToolCallCompleted { result, step } => DaemonEvent::ToolCallCompleted { session_id: sid_bg.clone(), result, step },
                    PlannerEvent::TurnCompleted(msg) => {
                        let _ = sqlx::query(
                            "INSERT INTO messages (id, conversation_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)"
                        )
                        .bind(&msg.id)
                        .bind(&sid_bg)
                        .bind("assistant")
                        .bind(&msg.content)
                        .bind(msg.timestamp.to_rfc3339())
                        .execute(&db_bg)
                        .await;

                        DaemonEvent::TurnCompleted { session_id: sid_bg.clone(), message: msg }
                    },
                    PlannerEvent::PermissionRequired(perm) => DaemonEvent::PermissionRequired {
                        session_id: sid_bg.clone(),
                        call: perm.call,
                        category: perm.category,
                        is_high_risk: perm.is_high_risk,
                    },
                    PlannerEvent::AgentStep { step, description, status } => DaemonEvent::AgentStep {
                        session_id: sid_bg.clone(),
                        step,
                        description,
                        status,
                    },
                    PlannerEvent::Error(err) => DaemonEvent::AgentError { session_id: sid_bg.clone(), message: err },
                };
                eq_bg.lock().await.push(daemon_evt);
            }
        });

        // Persist to DB
        sqlx::query(
            "INSERT INTO conversations (id, project_path, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)"
        )
        .bind(&session_id)
        .bind(project_path.as_deref())
        .bind(&model)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.db)
        .await?;

        let session = Arc::new(Session {
            id: session_id.clone(),
            project_path: project,
            model,
            planner: Arc::new(tokio::sync::Mutex::new(planner)),
            permission_tx: perm_tx,
            event_queue,
            terminal: Arc::new(tokio::sync::Mutex::new(pty)),
            jobs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            active_task: Arc::new(tokio::sync::Mutex::new(None)),
        });

        self.sessions.insert(session_id.clone(), session);
        info!("Created session {}", session_id);
        Ok(session_id)
    }

    pub async fn send_message(&self, session_id: &str, content: &str) -> Result<()> {
        let session = self.get_session(session_id)?;
        let planner = session.planner.clone();
        let content_str = content.to_string();

        // Persist user message to DB
        let user_msg_id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)"
        )
        .bind(&user_msg_id)
        .bind(session_id)
        .bind("user")
        .bind(content)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.db)
        .await;

        // Run agent in background task and track handle
        let active_task_clone = session.active_task.clone();
        let mut active = session.active_task.lock().await;
        if let Some(handle) = active.take() {
            handle.abort();
        }
        let handle = tokio::spawn(async move {
            let mut planner = planner.lock().await;
            if let Err(e) = planner.process_message(&content_str).await {
                error!("Agent error: {}", e);
            }
            let mut active = active_task_clone.lock().await;
            *active = None;
        });
        *active = Some(handle);

        Ok(())
    }

    pub async fn stop_agent(&self, session_id: &str) -> Result<()> {
        let session = self.get_session(session_id)?;
        let mut active = session.active_task.lock().await;
        if let Some(handle) = active.take() {
            handle.abort();
        }
        let mut eq = session.event_queue.lock().await;
        eq.clear();
        info!("Stopped agent task for session {}", session_id);
        Ok(())
    }

    pub async fn get_events(&self, session_id: &str) -> Result<Vec<DaemonEvent>> {
        let session = self.get_session(session_id)?;
        let mut eq = session.event_queue.lock().await;
        let events = std::mem::take(&mut *eq);
        Ok(events)
    }

    pub async fn delete_last_turn(&self, session_id: &str) -> Result<()> {
        let session = self.get_session(session_id)?;
        let mut planner = session.planner.lock().await;
        
        // Pop from in-memory conversation
        let _ = planner.pop_last_turn();

        // Delete from SQLite database (the last user message and anything after it)
        let _ = sqlx::query(
            "DELETE FROM messages 
             WHERE conversation_id = ?1 
             AND timestamp >= (
                 SELECT timestamp FROM messages 
                 WHERE conversation_id = ?1 AND role = 'user' 
                 ORDER BY timestamp DESC LIMIT 1
             )"
        )
        .bind(session_id)
        .execute(&self.db)
        .await;

        info!("Deleted last turn for session {}", session_id);
        Ok(())
    }

    pub async fn get_conversation(&self, session_id: &str) -> Result<Vec<ConversationMessage>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, role, content, tool_call_json, tool_result_json, timestamp FROM messages WHERE conversation_id = ?1 ORDER BY timestamp"
        )
        .bind(session_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.to_message()).collect())
    }

    pub async fn get_terminal_output(&self, session_id: &str, lines: usize) -> Result<Vec<String>> {
        let session = self.get_session(session_id)?;
        let pty = session.terminal.lock().await;
        Ok(pty.recent_output(lines))
    }

    pub async fn write_terminal(&self, session_id: &str, input: &str) -> Result<()> {
        let session = self.get_session(session_id)?;
        let mut pty = session.terminal.lock().await;
        pty.write_input(input).await.map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn resize_terminal(&self, session_id: &str, cols: u16, rows: u16) -> Result<()> {
        let session = self.get_session(session_id)?;
        let mut pty = session.terminal.lock().await;
        pty.resize(cols, rows).await.map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn set_permission(
        &self,
        session_id: &str,
        _category: PermissionCategory,
        scope: PermissionScope,
    ) -> Result<()> {
        let session = self.get_session(session_id)?;
        let resp = PermissionResponse { scope };
        let _ = session.permission_tx.send(resp).await;
        Ok(())
    }

    pub async fn list_jobs(&self, session_id: &str) -> Result<Vec<Job>> {
        let session = self.get_session(session_id)?;
        let jobs = session.jobs.lock().await;
        Ok(jobs.clone())
    }

    pub async fn get_model_profiles(&self) -> Result<HashMap<String, ModelProfile>> {
        Ok(self.prober.get_all_cached())
    }

    pub async fn probe_model(&self, model: &str) -> Result<ModelProfile> {
        self.prober.probe(model, self.provider.as_ref()).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn list_projects(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT project_path FROM conversations WHERE project_path IS NOT NULL ORDER BY updated_at DESC LIMIT 20"
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows)
    }

    fn get_session(&self, session_id: &str) -> Result<Arc<Session>> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: String,
    role: String,
    content: String,
    tool_call_json: Option<String>,
    tool_result_json: Option<String>,
    timestamp: String,
}

impl MessageRow {
    fn to_message(self) -> Option<ConversationMessage> {
        use axiom_core::types::Role;
        let role = match self.role.as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            "system" => Role::System,
            _ => return None,
        };
        Some(ConversationMessage {
            id: self.id,
            role,
            content: self.content,
            tool_call: self
                .tool_call_json
                .and_then(|j| serde_json::from_str(&j).ok()),
            tool_result: self
                .tool_result_json
                .and_then(|j| serde_json::from_str(&j).ok()),
            timestamp: chrono::DateTime::parse_from_rfc3339(&self.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
