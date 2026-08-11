import { useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, RefreshCw } from 'lucide-react';

import { useAppStore, loadModels } from './store';
import { Sidebar } from './components/Sidebar/Sidebar';
import { Chat } from './components/Chat/Chat';
import { TerminalPanel } from './components/Terminal/TerminalPanel';
import { PermissionDialog } from './components/Chat/PermissionDialog';
import { NotesPanel } from './components/Notes/NotesPanel';
import { WorkspacePanel } from './components/Workspace/WorkspacePanel';
import { useEventPoller } from './hooks/useEventPoller';

import './index.css';

// ─── Main App Component ───────────────────────────────────────────────────────

export default function App() {
  const {
    selectedModel,
    sidebarCollapsed,
    daemonConnected,
    terminalCollapsed,
    toggleTerminal,
    setDaemonConnected,
    setModels,
    setSelectedModel,
    setRecentProjects,
    currentView,
  } = useAppStore();

  // Start event polling
  useEventPoller();

  // Load models on startup
  const initApp = useCallback(async () => {
    try {
      const models = await loadModels();
      setModels(models);
      if (models.length > 0 && !selectedModel) {
        const preferred = models.find((m) => m.name.includes('qwen2.5-coder')) ?? models[0];
        setSelectedModel(preferred.name);
      }
      setDaemonConnected(true);
    } catch (err) {
      console.error('Failed to load models:', err);
      setDaemonConnected(false);
    }

    try {
      const result = await invoke<{ projects: string[] }>('list_projects');
      setRecentProjects(result.projects ?? []);
    } catch {}
  }, [selectedModel, setDaemonConnected, setModels, setSelectedModel, setRecentProjects]);

  useEffect(() => {
    initApp();
    const interval = setInterval(() => {
      if (!daemonConnected) initApp();
    }, 5000);
    return () => clearInterval(interval);
  }, [daemonConnected, initApp]);

  // Render Main View
  const renderMainView = () => {
    switch (currentView) {
      case 'notes':
        return <NotesPanel />;
      case 'workspace':
        return <WorkspacePanel />;
      case 'chat':
      default:
        return <Chat />;
    }
  };

  return (
    <div className={`app-layout ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
      {/* Sidebar with Axiom Branding */}
      <Sidebar />

      {/* Main Content Area */}
      <div className="main-area">
        {/* Daemon offline alert banner */}
        {!daemonConnected && (
          <div
            style={{
              padding: '6px 16px',
              background: 'var(--color-rose-dim)',
              borderBottom: '1px solid var(--color-rose)',
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              fontSize: 12,
              color: 'var(--color-rose)',
            }}
          >
            <AlertCircle size={14} />
            Cannot reach Axiom daemon — make sure daemon is running.
            <button
              className="btn-ghost"
              style={{ marginLeft: 'auto', padding: '2px 8px', fontSize: 11 }}
              onClick={initApp}
            >
              <RefreshCw size={11} style={{ marginRight: 4 }} />
              Retry
            </button>
          </div>
        )}

        {/* Chat, Notes, or Workspace view */}
        {renderMainView()}

        {/* Integrated Terminal Panel */}
        <TerminalPanel />

        {/* Bottom Status Bar with clickable Terminal Toggle */}
        <footer className="bottom-status-bar">
          <div className="status-dots">
            <div className="status-dot-item">
              <div className={`status-dot ${daemonConnected ? '' : 'offline'}`} />
              <span>Axiom Daemon</span>
            </div>

            {/* Clickable Terminal Toggle Option */}
            <div
              className="status-dot-item"
              onClick={toggleTerminal}
              style={{
                cursor: 'pointer',
                color: terminalCollapsed ? 'var(--color-text-muted)' : 'var(--color-text-primary)',
                fontWeight: terminalCollapsed ? 400 : 500,
              }}
              title="Click to collapse / uncollapse terminal"
            >
              <div className="status-dot" />
              <span>Open Terminal</span>
            </div>

            <div className="status-dot-item">
              <div className="status-dot" />
              <span>Ollama</span>
            </div>
          </div>

          <div style={{ fontSize: 10, color: 'var(--color-text-muted)' }}>
            v0.0.20
          </div>
        </footer>
      </div>

      {/* Permission Dialog Modal */}
      <PermissionDialog />
    </div>
  );
}
