import { useEffect, useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, RefreshCw } from 'lucide-react';

import { useAppStore, loadModels } from './store';
import { Sidebar } from './components/Sidebar/Sidebar';
import { Chat } from './components/Chat/Chat';
import { TerminalPanel } from './components/Terminal/TerminalPanel';
import { PermissionDialog } from './components/Chat/PermissionDialog';
import { useEventPoller } from './hooks/useEventPoller';

import './index.css';

// ─── Settings panel ────────────────────────────────────────────────────────
function SettingsPanel() {
  const { devMode, setDevMode } = useAppStore();
  return (
    <div style={{ padding: '24px', color: 'var(--color-text-secondary)', flex: 1, overflowY: 'auto' }}>
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: '16px', color: 'var(--color-text-primary)' }}>
        Settings
      </h2>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', maxWidth: 600 }}>
        <div
          style={{
            background: 'var(--color-bg-card)',
            border: '1px solid var(--color-border)',
            borderRadius: '12px',
            padding: '16px',
          }}
        >
          <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text-primary)', marginBottom: '12px' }}>
            Developer Mode
          </h3>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={devMode}
                onChange={(e) => setDevMode(e.target.checked)}
                style={{ accentColor: 'var(--color-indigo)', width: 16, height: 16 }}
              />
              <span>Show token counts, latency, and raw tool call JSON</span>
            </label>
          </div>
        </div>

        <div
          style={{
            background: 'var(--color-bg-card)',
            border: '1px solid var(--color-border)',
            borderRadius: '12px',
            padding: '16px',
          }}
        >
          <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text-primary)', marginBottom: '8px' }}>
            Axiom Agent Backend
          </h3>
          <p style={{ fontSize: 12, lineHeight: 1.7, color: 'var(--color-text-muted)' }}>
            External tool calling runtime for non-thinking local LLMs.
          </p>
        </div>
      </div>
    </div>
  );
}

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

  const [view] = useState<'chat' | 'settings'>('chat');

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

        {/* Chat or Settings view */}
        {view === 'settings' ? <SettingsPanel /> : <Chat />}

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
              <div className={`status-dot ${terminalCollapsed ? 'offline' : ''}`} />
              <span>Open Terminal {terminalCollapsed ? '(Collapsed)' : ''}</span>
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
