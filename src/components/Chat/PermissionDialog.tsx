import { Shield, ShieldAlert, ChevronRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store';
import type { PermissionScope } from '../../store';

export function PermissionDialog() {
  const { pendingPermission, sessionId, setPendingPermission } = useAppStore();

  if (!pendingPermission) return null;

  const { call, category, is_high_risk } = pendingPermission;

  const respond = async (scope: PermissionScope) => {
    setPendingPermission(null);
    if (!sessionId) return;
    try {
      await invoke('set_permission', { sessionId, category, scope });
    } catch (err) {
      console.error('set_permission failed:', err);
    }
  };

  const commandText = (() => {
    const args = call.arguments as Record<string, unknown>;
    if (call.tool === 'terminal.exec') {
      return String(args.command || '');
    }
    if (call.tool === 'filesystem') {
      return `${args.operation} ${args.path}`;
    }
    return `${call.tool}(${JSON.stringify(args)})`;
  })();

  return (
    <div className="modal-overlay" onClick={(e) => e.target === e.currentTarget && respond('deny')}>
      <div className="modal" id="permission-dialog" role="dialog" aria-modal="true">
        <div style={{ marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8, color: is_high_risk ? 'var(--color-rose)' : 'var(--color-amber)' }}>
          {is_high_risk ? <ShieldAlert size={22} /> : <Shield size={22} />}
          <span className="modal-title" style={{ margin: 0 }}>
            {is_high_risk ? '⚠️ High-risk operation' : 'Permission required'}
          </span>
        </div>

        <div style={{ fontSize: 13, color: 'var(--color-text-secondary)', marginBottom: 8 }}>
          The agent wants to <strong>{categoryDescription(category)}</strong>.
        </div>

        <div className="modal-command">{commandText}</div>

        <div className="modal-actions">
          <button className="modal-btn" onClick={() => respond('once')}>
            <ChevronRight size={14} style={{ float: 'right' }} />
            Allow once
          </button>

          {!is_high_risk && (
            <>
              <button className="modal-btn" onClick={() => respond('session')}>
                <ChevronRight size={14} style={{ float: 'right' }} />
                Allow {categoryDescription(category)} for this session
              </button>

              <button className="modal-btn" onClick={() => respond('project')}>
                <ChevronRight size={14} style={{ float: 'right' }} />
                Allow {categoryDescription(category)} for this project
              </button>
            </>
          )}

          <button className="modal-btn deny" onClick={() => respond('deny')}>
            Deny
          </button>
        </div>
      </div>
    </div>
  );
}

function categoryDescription(category: string): string {
  const map: Record<string, string> = {
    READ: 'read files',
    WRITE: 'write files',
    EXECUTE: 'execute commands',
    NETWORK: 'make network requests',
    DELETE: 'delete files',
    GIT: 'perform git operations',
    PROCESS: 'manage processes',
  };
  return map[category] ?? category.toLowerCase();
}
