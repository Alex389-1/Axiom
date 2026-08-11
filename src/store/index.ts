// Central Zustand store — all application state in one place for simplicity.
import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

// ─── Types ────────────────────────────────────────────────────────────────────

export type Role = 'user' | 'assistant' | 'tool' | 'system';

export interface ToolCall {
  tool: string;
  arguments: Record<string, unknown>;
}

export interface ToolResult {
  tool: string;
  arguments: Record<string, unknown>;
  output: ToolOutput;
  duration_ms: number;
  timestamp: string;
}

export type ToolOutput =
  | { status: 'success'; stdout: string; stderr: string; exit_code: number }
  | { status: 'error'; message: string };

export interface Message {
  id: string;
  role: Role;
  content: string;
  tool_call?: ToolCall;
  tool_result?: ToolResult;
  timestamp: string;
  streaming?: boolean;
  model?: string;
}

export interface ChatSession {
  id: string;
  title: string;
  messages: Message[];
  created_at: string;
}

export interface ModelInfo {
  name: string;
  provider: string;
  size_bytes?: number;
}

export interface Job {
  id: string;
  label: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  started_at: string;
  finished_at?: string;
}

export interface AgentStep {
  step: number;
  description: string;
  status: 'running' | 'completed' | 'warning' | 'failed';
}

export type PermissionCategory =
  | 'READ' | 'WRITE' | 'EXECUTE' | 'NETWORK' | 'DELETE' | 'GIT' | 'PROCESS';

export type PermissionScope = 'once' | 'session' | 'project' | 'deny';

export interface PendingPermission {
  call: ToolCall;
  category: PermissionCategory;
  is_high_risk: boolean;
}

// ─── Store ────────────────────────────────────────────────────────────────────

interface AppState {
  // Chats / Sessions
  chats: ChatSession[];
  activeChatId: string;
  sessionId: string | null;
  projectPath: string | null;
  recentProjects: string[];

  // Models
  models: ModelInfo[];
  selectedModel: string;

  // Active Conversation State
  messages: Message[];
  streamingContent: string;
  streamingChatId: string | null;
  isAgentRunning: boolean;
  agentSteps: AgentStep[];

  // Terminal
  terminalCollapsed: boolean;
  terminalOutput: string[];

  // Jobs
  jobs: Job[];

  // Permission
  pendingPermission: PendingPermission | null;

  // UI
  devMode: boolean;
  daemonConnected: boolean;
  sidebarCollapsed: boolean;

  // Actions
  addChat: (title?: string) => void;
  deleteChat: (id: string) => void;
  selectChat: (id: string) => void;
  setSessionId: (id: string) => void;
  setProjectPath: (p: string) => void;
  setModels: (m: ModelInfo[]) => void;
  setSelectedModel: (m: string) => void;
  addMessage: (m: Message, targetChatId?: string) => void;
  editMessage: (id: string, content: string) => void;
  appendStreamToken: (token: string) => void;
  setStreamingContent: (content: string) => void;
  setStreamingChatId: (id: string | null) => void;
  finalizeStreamMessage: () => void;
  setAgentRunning: (v: boolean) => void;
  addAgentStep: (step: AgentStep) => void;
  updateAgentStep: (step: AgentStep) => void;
  clearAgentSteps: () => void;
  toggleTerminal: () => void;
  setTerminalCollapsed: (v: boolean) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (v: boolean) => void;
  appendTerminalOutput: (line: string) => void;
  setJobs: (jobs: Job[]) => void;
  setPendingPermission: (p: PendingPermission | null) => void;
  setDevMode: (v: boolean) => void;
  setDaemonConnected: (v: boolean) => void;
  setRecentProjects: (p: string[]) => void;
}

const initialDemoChats: ChatSession[] = [
  {
    id: 'chat-1',
    title: 'Greeting',
    created_at: new Date().toISOString(),
    messages: [
      {
        id: 'demo-user-1',
        role: 'user',
        content: 'hii',
        timestamp: new Date().toISOString(),
      },
      {
        id: 'demo-assistant-1',
        role: 'assistant',
        content: `<think>Okay, the user said "hii". That's a greeting. I need to respond appropriately. Let me check the functions available to see if any are needed here. The tools provided include functions for notes, tasks, automations, and calendar events. Since the user is just greeting, there's no specific function required. I should reply with a friendly message, maybe ask how I can assist them. No need to call any functions here. Just a simple response to acknowledge their greeting.</think>

Hello! How can I assist you today? 😊`,
        timestamp: new Date().toISOString(),
      },
    ],
  },
  {
    id: 'chat-2',
    title: 'Project Assistant',
    created_at: new Date().toISOString(),
    messages: [],
  },
];

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      chats: initialDemoChats,
      activeChatId: 'chat-1',
      sessionId: null,
      projectPath: null,
      recentProjects: [],
      models: [],
      selectedModel: 'qwen2.5-coder:14b',
      messages: initialDemoChats[0].messages,
      streamingContent: '',
      streamingChatId: null,
      isAgentRunning: false,
      agentSteps: [],
      terminalCollapsed: true,
      terminalOutput: [],
      jobs: [],
      pendingPermission: null,
      devMode: false,
      daemonConnected: false,
      sidebarCollapsed: false,

      addChat: (title) => {
        const newId = `chat-${Date.now()}`;
        const newChat: ChatSession = {
          id: newId,
          title: title || `New Chat ${get().chats.length + 1}`,
          messages: [],
          created_at: new Date().toISOString(),
        };
        set((s) => ({
          chats: [newChat, ...s.chats],
          activeChatId: newId,
          messages: [],
          agentSteps: [],
        }));
      },

      deleteChat: (id) => {
        const currentChats = get().chats;
        const filtered = currentChats.filter((c) => c.id !== id);
        let nextActiveId = get().activeChatId;
        let nextMessages = get().messages;

        if (get().activeChatId === id) {
          if (filtered.length > 0) {
            nextActiveId = filtered[0].id;
            nextMessages = filtered[0].messages;
          } else {
            const fallbackId = `chat-${Date.now()}`;
            const fallbackChat: ChatSession = {
              id: fallbackId,
              title: 'New Chat',
              messages: [],
              created_at: new Date().toISOString(),
            };
            filtered.push(fallbackChat);
            nextActiveId = fallbackId;
            nextMessages = [];
          }
        }

        set({
          chats: filtered,
          activeChatId: nextActiveId,
          messages: nextMessages,
        });
      },

      selectChat: (id) => {
        const chat = get().chats.find((c) => c.id === id);
        if (chat) {
          set({
            activeChatId: id,
            messages: chat.messages,
            agentSteps: [],
          });
        }
      },

      setSessionId: (id) => set({ sessionId: id }),
      setProjectPath: (p) => set({ projectPath: p }),
      setModels: (m) => set({ models: m }),
      setSelectedModel: (m) => set({ selectedModel: m }),

      addMessage: (m, targetChatId) => {
        const destChatId = targetChatId || get().streamingChatId || get().activeChatId;
        set((s) => {
          const isCurrentView = destChatId === s.activeChatId;
          const updatedMessages = isCurrentView ? [...s.messages, m] : s.messages;
          const updatedChats = s.chats.map((c) => {
            if (c.id === destChatId) {
              const title = c.messages.length === 0 && m.role === 'user'
                ? m.content.slice(0, 24)
                : c.title;
              return { ...c, title, messages: [...c.messages, m] };
            }
            return c;
          });
          return { messages: updatedMessages, chats: updatedChats };
        });
      },

      editMessage: (id, content) => {
        const activeId = get().activeChatId;
        set((s) => {
          const updatedMessages = s.messages.map((m) =>
            m.id === id ? { ...m, content } : m
          );
          const updatedChats = s.chats.map((c) => {
            if (c.id === activeId) {
              return { ...c, messages: updatedMessages };
            }
            return c;
          });
          return { messages: updatedMessages, chats: updatedChats };
        });
      },

      appendStreamToken: (token) =>
        set((s) => ({ streamingContent: s.streamingContent + token })),

      setStreamingContent: (content) => set({ streamingContent: content }),
      setStreamingChatId: (id) => set({ streamingChatId: id }),

      finalizeStreamMessage: () => {
        const content = get().streamingContent;
        const targetChatId = get().streamingChatId || get().activeChatId;
        if (!content.trim()) {
          set({ streamingContent: '', streamingChatId: null });
          return;
        }
        const msg: Message = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content,
          timestamp: new Date().toISOString(),
          model: get().selectedModel,
        };
        get().addMessage(msg, targetChatId);
        set({ streamingContent: '', streamingChatId: null });
      },

      setAgentRunning: (v) => set({ isAgentRunning: v }),

      addAgentStep: (step) =>
        set((s) => ({ agentSteps: [...s.agentSteps, step] })),

      updateAgentStep: (step) =>
        set((s) => ({
          agentSteps: s.agentSteps.map((st) =>
            st.step === step.step ? step : st
          ),
        })),

      clearAgentSteps: () => set({ agentSteps: [] }),

      toggleTerminal: () => set((s) => ({ terminalCollapsed: !s.terminalCollapsed })),
      setTerminalCollapsed: (v) => set({ terminalCollapsed: v }),
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),

      appendTerminalOutput: (line) =>
        set((s) => ({
          terminalOutput: [...s.terminalOutput.slice(-9999), line],
        })),

      setJobs: (jobs) => set({ jobs }),
      setPendingPermission: (p) => set({ pendingPermission: p }),
      setDevMode: (v) => set({ devMode: v }),
      setDaemonConnected: (v) => set({ daemonConnected: v }),
      setRecentProjects: (p) => set({ recentProjects: p }),
    }),
    {
      name: 'axiom-chats-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        chats: state.chats,
        activeChatId: state.activeChatId,
        selectedModel: state.selectedModel,
        recentProjects: state.recentProjects,
        devMode: state.devMode,
      }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          const active = state.chats.find((c) => c.id === state.activeChatId) || state.chats[0];
          if (active) {
            state.activeChatId = active.id;
            state.messages = active.messages;
          }
        }
      },
    }
  )
);

export async function initializeSession(
  projectPath: string | null,
  model: string
): Promise<string> {
  const sessionId = await invoke<string>('create_session', {
    projectPath,
    model,
  });
  return sessionId;
}

export async function loadModels(): Promise<ModelInfo[]> {
  try {
    const raw = await invoke<ModelInfo[]>('list_models');
    return raw;
  } catch {
    return [];
  }
}
