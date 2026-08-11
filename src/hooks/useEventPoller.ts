import { useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../store';
import type { AgentStep, Message, PendingPermission } from '../store';

type DaemonEvent =
  | { type: 'token'; session_id: string; token: string }
  | { type: 'tool_call_started'; session_id: string; call: { tool: string; arguments: Record<string,unknown> }; step: number }
  | { type: 'tool_call_completed'; session_id: string; result: unknown; step: number }
  | { type: 'turn_completed'; session_id: string; message: Message }
  | { type: 'permission_required'; session_id: string; call: { tool: string; arguments: Record<string,unknown> }; category: string; is_high_risk: boolean }
  | { type: 'agent_step'; session_id: string; step: number; description: string; status: string }
  | { type: 'agent_error'; session_id: string; message: string };

export function useEventPoller() {
  const sessionId = useAppStore((s) => s.sessionId);
  const {
    appendStreamToken,
    finalizeStreamMessage,
    addMessage,
    addAgentStep,
    updateAgentStep,
    setAgentRunning,
    setPendingPermission,
  } = useAppStore.getState();

  const pollRef = useRef<number | null>(null);

  const poll = useCallback(async () => {
    if (!sessionId) return;
    if (!useAppStore.getState().isAgentRunning) return;
    try {
      const events = await invoke<DaemonEvent[]>('poll_events', { sessionId });
      if (!useAppStore.getState().isAgentRunning) return;
      for (const event of events) {
        handleEvent(event, {
          appendStreamToken,
          finalizeStreamMessage,
          addMessage,
          addAgentStep,
          updateAgentStep,
          setAgentRunning,
          setPendingPermission,
        });
      }
    } catch {
      // Daemon unreachable
    }
  }, [sessionId, appendStreamToken, finalizeStreamMessage, addMessage, addAgentStep, updateAgentStep, setAgentRunning, setPendingPermission]);

  useEffect(() => {
    pollRef.current = window.setInterval(poll, 200);
    return () => {
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
  }, [poll]);
}

function handleEvent(
  event: DaemonEvent,
  actions: {
    appendStreamToken: (t: string) => void;
    finalizeStreamMessage: () => void;
    addMessage: (m: Message) => void;
    addAgentStep: (s: AgentStep) => void;
    updateAgentStep: (s: AgentStep) => void;
    setAgentRunning: (v: boolean) => void;
    setPendingPermission: (p: PendingPermission | null) => void;
  }
) {
  switch (event.type) {
    case 'token':
      actions.appendStreamToken(event.token);
      break;

    case 'tool_call_started':
      if (event.call.tool === 'terminal.exec' && event.call.arguments.command) {
        console.log('[DEBUG] Dispatching llm_terminal start for:', event.call.arguments.command);
        window.dispatchEvent(new CustomEvent('llm_terminal', { detail: `\r\n\x1b[35m[Axiom]\x1b[0m 🚀 Executing: ${event.call.arguments.command}\r\n` }));
      }
      actions.addAgentStep({
        step: event.step,
        description: `${event.call.tool}(${summarizeArgs(event.call.arguments)})`,
        status: 'running',
      });
      break;

    case 'tool_call_completed':
      const res = event.result as any;
      if (res && typeof res === 'object' && ('stdout' in res || 'stderr' in res)) {
        console.log('[DEBUG] Dispatching llm_terminal completed for:', res);
        if (res.stdout) {
          window.dispatchEvent(new CustomEvent('llm_terminal', { detail: `\x1b[90m${res.stdout.replace(/\n/g, '\r\n')}\x1b[0m\r\n` }));
        }
        if (res.stderr) {
          window.dispatchEvent(new CustomEvent('llm_terminal', { detail: `\x1b[31m${res.stderr.replace(/\n/g, '\r\n')}\x1b[0m\r\n` }));
        }
      }
      actions.updateAgentStep({
        step: event.step,
        description: `Step ${event.step}`,
        status: 'completed',
      });
      break;

    case 'turn_completed':
      actions.finalizeStreamMessage();
      if (event.message && event.message.content) {
        const state = useAppStore.getState();
        const exists = state.messages.some(
          (m) => m.id === event.message.id || (m.content === event.message.content && m.role === 'assistant')
        );
        if (!exists) {
          actions.addMessage({
            id: event.message.id || crypto.randomUUID(),
            role: 'assistant',
            content: event.message.content,
            timestamp: event.message.timestamp || new Date().toISOString(),
            model: state.selectedModel,
          });
        }
      }
      actions.setAgentRunning(false);
      break;

    case 'permission_required':
      actions.setPendingPermission({
        call: event.call,
        category: event.category as any,
        is_high_risk: event.is_high_risk,
      });
      break;

    case 'agent_step':
      actions.updateAgentStep({
        step: event.step,
        description: event.description,
        status: event.status as any,
      });
      break;

    case 'agent_error':
      actions.finalizeStreamMessage();
      actions.setAgentRunning(false);
      actions.addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `⚠️ Agent error: ${event.message}`,
        timestamp: new Date().toISOString(),
      });
      break;
  }
}

function summarizeArgs(args: Record<string, unknown>): string {
  const values = Object.values(args);
  if (values.length === 0) return '';
  const first = String(values[0]);
  return first.length > 40 ? first.slice(0, 40) + '…' : first;
}
