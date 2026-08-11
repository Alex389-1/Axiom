import React from 'react';
import { useAppStore } from '../../store';
import { LayoutGrid, Server, Cpu, Clock, FolderOpen } from 'lucide-react';

export function WorkspacePanel() {
  const { daemonConnected, selectedModel, recentProjects, setProjectPath, toggleTerminal } = useAppStore();

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--color-bg-main)', overflowY: 'auto' }}>
      <div style={{ 
        padding: '24px 32px', 
        borderBottom: '1px solid var(--color-border)',
        display: 'flex',
        alignItems: 'center',
        gap: '12px'
      }}>
        <LayoutGrid size={24} color="var(--color-indigo)" />
        <h2 style={{ fontSize: 22, fontWeight: 600, color: 'var(--color-text-primary)', margin: 0 }}>
          Workspace Overview
        </h2>
      </div>
      
      <div style={{ padding: '32px', display: 'flex', flexDirection: 'column', gap: '32px', maxWidth: '1000px', margin: '0 auto', width: '100%' }}>
        
        {/* Status Cards */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '20px' }}>
          
          <div style={{
            background: 'var(--color-bg-card)',
            border: '1px solid var(--color-border)',
            borderRadius: '16px',
            padding: '24px',
            display: 'flex',
            alignItems: 'flex-start',
            gap: '16px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.1)'
          }}>
            <div style={{ background: daemonConnected ? 'rgba(34, 197, 94, 0.1)' : 'rgba(239, 68, 68, 0.1)', padding: '12px', borderRadius: '12px' }}>
              <Server size={24} color={daemonConnected ? '#22c55e' : '#ef4444'} />
            </div>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--color-text-muted)', marginBottom: '4px' }}>Daemon Status</h3>
              <p style={{ fontSize: 18, fontWeight: 600, color: 'var(--color-text-primary)', margin: 0 }}>
                {daemonConnected ? 'Online & Ready' : 'Offline'}
              </p>
            </div>
          </div>

          <div style={{
            background: 'var(--color-bg-card)',
            border: '1px solid var(--color-border)',
            borderRadius: '16px',
            padding: '24px',
            display: 'flex',
            alignItems: 'flex-start',
            gap: '16px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.1)'
          }}>
            <div style={{ background: 'rgba(99, 102, 241, 0.1)', padding: '12px', borderRadius: '12px' }}>
              <Cpu size={24} color="var(--color-indigo)" />
            </div>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--color-text-muted)', marginBottom: '4px' }}>Active Model</h3>
              <p style={{ fontSize: 18, fontWeight: 600, color: 'var(--color-text-primary)', margin: 0, textOverflow: 'ellipsis', overflow: 'hidden', whiteSpace: 'nowrap' }}>
                {selectedModel || 'None Selected'}
              </p>
            </div>
          </div>

        </div>

        {/* Recent Projects */}
        <div style={{ marginTop: '16px' }}>
          <h3 style={{ fontSize: 18, fontWeight: 600, color: 'var(--color-text-primary)', marginBottom: '16px', display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Clock size={20} color="var(--color-text-muted)" />
            Recent Projects
          </h3>
          
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: '16px' }}>
            {recentProjects.length === 0 ? (
              <p style={{ color: 'var(--color-text-muted)', fontSize: 14, fontStyle: 'italic' }}>No recent projects found.</p>
            ) : (
              recentProjects.map((path, idx) => (
                <div
                  key={idx}
                  onClick={() => setProjectPath(path)}
                  style={{
                    background: 'var(--color-bg-card)',
                    border: '1px solid var(--color-border)',
                    borderRadius: '12px',
                    padding: '16px',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '12px',
                    transition: 'all 0.2s ease',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = 'var(--color-indigo)';
                    e.currentTarget.style.background = 'rgba(255,255,255,0.05)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = 'var(--color-border)';
                    e.currentTarget.style.background = 'var(--color-bg-card)';
                  }}
                >
                  <FolderOpen size={20} color="var(--color-text-muted)" />
                  <div style={{ overflow: 'hidden' }}>
                    <div style={{ fontSize: 14, fontWeight: 500, color: 'var(--color-text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {path.split('/').pop()}
                    </div>
                    <div style={{ fontSize: 12, color: 'var(--color-text-muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {path}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
        
        {/* Quick Actions */}
        <div style={{ marginTop: '32px' }}>
           <button 
             onClick={toggleTerminal}
             style={{
               background: 'var(--color-indigo)',
               color: '#fff',
               border: 'none',
               padding: '12px 24px',
               borderRadius: '8px',
               fontSize: 14,
               fontWeight: 600,
               cursor: 'pointer',
               display: 'flex',
               alignItems: 'center',
               gap: '8px',
               boxShadow: '0 4px 12px rgba(99, 102, 241, 0.3)'
             }}
           >
             Open Integrated Terminal
           </button>
        </div>

      </div>
    </div>
  );
}
