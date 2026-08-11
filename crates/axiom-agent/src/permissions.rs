use axiom_core::{
    errors::Result,
    types::{PermissionCategory, PermissionGrant, PermissionScope},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::Utc;
use tracing::{debug, info};

/// Manages tool-call permissions across once/session/project scopes.
///
/// High-risk commands (sudo, rm -rf, etc.) are always re-prompted individually,
/// regardless of what grants exist — this is enforced by the caller passing
/// `is_high_risk = true` to `check_permission`.
#[derive(Clone)]
pub struct PermissionManager {
    /// Session-scoped grants: cleared on daemon restart.
    session_grants: Arc<Mutex<HashMap<PermissionCategory, PermissionScope>>>,
    /// Project root for persisting project-scoped grants.
    project_root: Option<PathBuf>,
    /// Project-scoped grants: loaded from / saved to `.local-agent/permissions.toml`.
    project_grants: Arc<Mutex<HashMap<String, Vec<PermissionGrant>>>>,
}

impl PermissionManager {
    pub fn new(project_root: Option<PathBuf>) -> Self {
        let mut mgr = Self {
            session_grants: Arc::new(Mutex::new(HashMap::new())),
            project_root: project_root.clone(),
            project_grants: Arc::new(Mutex::new(HashMap::new())),
        };
        if let Some(root) = &project_root {
            let _ = mgr.load_project_grants(root);
        }
        mgr
    }

    /// Check if a tool call is permitted under existing grants.
    /// Returns `PermissionStatus` indicating what action to take.
    pub fn check(&self, category: &PermissionCategory, is_high_risk: bool) -> PermissionStatus {
        // High-risk commands always require individual confirmation.
        if is_high_risk {
            return PermissionStatus::RequiresConfirmation { is_high_risk: true };
        }

        // Read operations (reading workspace files, code search) are auto-granted by default.
        if *category == PermissionCategory::Read {
            return PermissionStatus::Granted;
        }

        // Check session-scoped grants.
        let session = self.session_grants.lock().unwrap();
        if let Some(scope) = session.get(category) {
            match scope {
                PermissionScope::Session | PermissionScope::Project => {
                    debug!("Permission granted by grant: {:?}", category);
                    return PermissionStatus::Granted;
                }
                _ => {}
            }
        }
        drop(session);

        // Check project grants.
        if let Some(root) = &self.project_root {
            let project_grants = self.project_grants.lock().unwrap();
            let key = format!("{:?}", category);
            if let Some(grants) = project_grants.get(&key) {
                if grants.iter().any(|g| g.scope == PermissionScope::Project) {
                    debug!("Permission granted by project grant: {:?}", category);
                    return PermissionStatus::Granted;
                }
            }
        }

        PermissionStatus::RequiresConfirmation { is_high_risk: false }
    }

    /// Record a grant from the user's response to a permission dialog.
    pub fn grant(
        &self,
        category: PermissionCategory,
        scope: PermissionScope,
    ) -> Result<()> {
        match &scope {
            PermissionScope::Session => {
                let mut session = self.session_grants.lock().unwrap();
                info!("Session grant: {:?}", category);
                session.insert(category.clone(), PermissionScope::Session);
            }
            PermissionScope::Project => {
                {
                    let mut session = self.session_grants.lock().unwrap();
                    session.insert(category.clone(), PermissionScope::Project);
                }
                let grant = PermissionGrant {
                    category: category.clone(),
                    scope: PermissionScope::Project,
                    granted_at: Utc::now(),
                    project: self.project_root.as_ref().map(|p| p.to_string_lossy().to_string()),
                };
                {
                    let mut project_grants = self.project_grants.lock().unwrap();
                    project_grants
                        .entry(format!("{:?}", category))
                        .or_default()
                        .push(grant);
                }
                if let Some(root) = &self.project_root {
                    let _ = self.save_project_grants(root);
                }
            }
            _ => {
                // PermissionScope::Once — no persistent record needed
            }
        }
        Ok(())
    }

    /// Revoke a session grant (e.g. user changed their mind in Settings).
    pub fn revoke_session(&self, category: &PermissionCategory) {
        let mut session = self.session_grants.lock().unwrap();
        session.remove(category);
    }

    /// Clear all session grants (called on daemon restart / new session).
    pub fn clear_session(&self) {
        let mut session = self.session_grants.lock().unwrap();
        session.clear();
    }

    fn project_grants_path(root: &Path) -> PathBuf {
        root.join(".local-agent").join("permissions.toml")
    }

    fn load_project_grants(&mut self, root: &Path) -> Result<()> {
        let path = Self::project_grants_path(root);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        if let Ok(map) = toml::from_str::<HashMap<String, Vec<PermissionGrant>>>(&content) {
            let mut pg = self.project_grants.lock().unwrap();
            *pg = map;
        }
        Ok(())
    }

    fn save_project_grants(&self, root: &Path) -> Result<()> {
        let path = Self::project_grants_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pg = self.project_grants.lock().unwrap();
        let content = toml::to_string(&*pg)
            .map_err(|e| axiom_core::errors::AxiomError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PermissionStatus {
    /// Already granted — proceed without asking.
    Granted,
    /// Must ask the user before proceeding.
    RequiresConfirmation { is_high_risk: bool },
}
