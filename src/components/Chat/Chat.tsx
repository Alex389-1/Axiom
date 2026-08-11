import { useState, useRef, useEffect, useCallback, KeyboardEvent } from 'react';
import {
  Square,
  Plus,
  LayoutGrid,
  Mic,
  MicOff,
  ArrowUp,
  MoreHorizontal,
  SlidersHorizontal,
  ChevronDown,
  BarChart2,
  Loader2,
  Terminal,
  FileText,
  Search,
  Globe,
  Code,
  X,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store';
import { ChatMessage, AgentTimeline, markdownComponents } from './ChatMessage';
import { ModelSelector } from './ModelSelector';
import ReactMarkdown from 'react-markdown';
import { useAudioRecorder } from '../../hooks/useAudioRecorder';

interface AttachedFile {
  id: string;
  name: string;
  type: 'image' | 'text';
  dataUrl?: string;
  textSnippet?: string;
}

export function Chat() {
  const {
    chats,
    activeChatId,
    messages,
    streamingContent,
    streamingChatId,
    isAgentRunning,
    agentSteps,
    sessionId,
    devMode,
    setDevMode,
    clearAgentSteps,
    addMessage,
    setAgentRunning,
    setStreamingChatId,
    selectedModel,
    projectPath,
  } = useAppStore();

  const [input, setInput] = useState('');
  const [attachedFiles, setAttachedFiles] = useState<AttachedFile[]>([]);
  const [showTimeline, setShowTimeline] = useState(false);
  const [showToolsMenu, setShowToolsMenu] = useState(false);
  const [showControlsPopover, setShowControlsPopover] = useState(false);
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(4096);

  const { isRecording, isTranscribing, toggleRecording } = useAudioRecorder((text) => {
    setInput((prev) => (prev ? prev + ' ' + text : text));
  });

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const toolsRef = useRef<HTMLDivElement>(null);
  const controlsRef = useRef<HTMLDivElement>(null);

  const activeChat = chats.find((c) => c.id === activeChatId);

  // Parse streaming content for live reasoning trace
  let streamingThinking: string | null = null;
  let streamingMain: string = streamingContent;

  if (streamingContent.includes('<think>')) {
    if (streamingContent.includes('</think>')) {
      const match = streamingContent.match(/<think>([\s\S]*?)<\/think>/);
      streamingThinking = match ? match[1].trim() : '';
      streamingMain = streamingContent.replace(/<think>[\s\S]*?<\/think>/, '').trim();
    } else {
      const parts = streamingContent.split('<think>');
      streamingThinking = (parts[1] || '').trim();
      streamingMain = '';
    }
  }

  // Close tools popover when clicking outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (toolsRef.current && !toolsRef.current.contains(e.target as Node)) {
        setShowToolsMenu(false);
      }
    }
    if (showToolsMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showToolsMenu]);

  // Close controls popover when clicking outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (controlsRef.current && !controlsRef.current.contains(e.target as Node)) {
        setShowControlsPopover(false);
      }
    }
    if (showControlsPopover) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showControlsPopover]);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingContent]);

  // Auto-resize textarea
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = Math.min(ta.scrollHeight, 160) + 'px';
  }, [input]);

  // 1. File Upload / Context Attachment Handler
  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;

    Array.from(files).forEach((file) => {
      const isImage = file.type.startsWith('image/') || /\.(jpg|jpeg|png|gif|webp|bmp|svg)$/i.test(file.name);

      if (isImage) {
        const reader = new FileReader();
        reader.onload = (event) => {
          const dataUrl = event.target?.result as string;
          if (dataUrl) {
            setAttachedFiles((prev) => [
              ...prev,
              { id: crypto.randomUUID(), name: file.name, type: 'image', dataUrl },
            ]);
          }
        };
        reader.readAsDataURL(file);
      } else {
        const reader = new FileReader();
        reader.onload = (event) => {
          const text = event.target?.result as string;
          if (text) {
            setAttachedFiles((prev) => [
              ...prev,
              { id: crypto.randomUUID(), name: file.name, type: 'text', textSnippet: text },
            ]);
          }
        };
        reader.readAsText(file);
      }
    });

    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleSend = useCallback(async () => {
    const rawInput = input.trim();
    if ((!rawInput && attachedFiles.length === 0) || isAgentRunning) return;

    let fullPrompt = rawInput;
    attachedFiles.forEach((file) => {
      if (file.type === 'image' && file.dataUrl) {
        fullPrompt += `${fullPrompt ? '\n\n' : ''}![${file.name}](${file.dataUrl})`;
      } else if (file.type === 'text' && file.textSnippet) {
        fullPrompt += `${fullPrompt ? '\n\n' : ''}[File: ${file.name}]\n\`\`\`\n${file.textSnippet.slice(0, 4000)}\n\`\`\``;
      }
    });

    setInput('');
    setAttachedFiles([]);
    setAgentRunning(true);
    setStreamingChatId(activeChatId);
    clearAgentSteps();

    // Add user message immediately to current chat
    addMessage({
      id: crypto.randomUUID(),
      role: 'user',
      content: fullPrompt,
      timestamp: new Date().toISOString(),
    });

    let currentSessionId = sessionId;

    if (!currentSessionId) {
      try {
        currentSessionId = await invoke<string>('create_session', {
          projectPath: projectPath || undefined,
          model: selectedModel || undefined,
        });
        useAppStore.getState().setSessionId(currentSessionId);
      } catch (err) {
        console.error('create session failed:', err);
        setAgentRunning(false);
        addMessage({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `⚠️ Failed to initialize session: ${err}`,
          timestamp: new Date().toISOString(),
        });
        return;
      }
    }

    try {
      await invoke('send_message', { sessionId: currentSessionId, content: fullPrompt });
    } catch (err) {
      console.error('send_message failed:', err);
      setAgentRunning(false);
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `⚠️ Failed to send message: ${err}`,
        timestamp: new Date().toISOString(),
      });
    }
  }, [input, attachedFiles, sessionId, isAgentRunning, projectPath, selectedModel, setAgentRunning, clearAgentSteps, addMessage]);

  const handleStop = useCallback(async () => {
    setAgentRunning(false);
    if (sessionId) {
      try {
        await invoke('stop_agent', { sessionId });
      } catch (e) {
        console.error('Failed to stop agent task:', e);
      }
    }
    const content = useAppStore.getState().streamingContent;
    if (content && content.trim()) {
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: content + ' *(stopped)*',
        timestamp: new Date().toISOString(),
        model: selectedModel,
      });
    }
    useAppStore.getState().setStreamingContent('');
  }, [sessionId, setAgentRunning, addMessage, selectedModel]);

  const handleRegenerate = useCallback(
    async (messageId: string, newContent?: string) => {
      if (isAgentRunning) return;
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx === -1) return;

      let userPrompt = '';
      for (let i = idx; i >= 0; i--) {
        if (messages[i].role === 'user') {
          userPrompt = newContent || messages[i].content;
          break;
        }
      }

      if (!userPrompt) return;

      setAgentRunning(true);
      clearAgentSteps();

      let currentSessionId = sessionId;
      if (!currentSessionId) {
        try {
          currentSessionId = await invoke<string>('create_session', {
            projectPath: projectPath || undefined,
            model: selectedModel || undefined,
          });
          useAppStore.getState().setSessionId(currentSessionId);
        } catch (err) {
          console.error('create session failed:', err);
          setAgentRunning(false);
          return;
        }
      }

      try {
        useAppStore.getState().deleteLastTurn();
        
        // Optimistically add the user message back to the UI BEFORE making network calls
        addMessage({
          id: crypto.randomUUID(),
          role: 'user',
          content: userPrompt,
          timestamp: new Date().toISOString(),
        });
        
        setStreamingChatId(useAppStore.getState().activeChatId);

        await invoke('delete_last_turn', { sessionId: currentSessionId });
        await invoke('send_message', { sessionId: currentSessionId, content: userPrompt });
      } catch (err) {
        console.error('regenerate send_message failed:', err);
        setAgentRunning(false);
      }
    },
    [messages, isAgentRunning, sessionId, projectPath, selectedModel, setAgentRunning, clearAgentSteps, addMessage, setStreamingChatId]
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  const availableTools = [
    { name: 'Explain Code', desc: 'Break down complex code step-by-step', icon: Code, snippet: 'Explain this code in detail: ' },
    { name: 'Refactor Code', desc: 'Improve performance and readability', icon: FileText, snippet: 'Refactor this code to be cleaner and faster: ' },
    { name: 'Write Unit Tests', desc: 'Generate test coverage for functions', icon: Terminal, snippet: 'Write unit tests for: ' },
    { name: 'Fix Bug / Error', desc: 'Analyze logs and resolve issues', icon: Search, snippet: 'Analyze and fix this issue: ' },
    { name: 'Documentation', desc: 'Generate docstrings and READMEs', icon: Globe, snippet: 'Write documentation for: ' },
  ];

  return (
    <div className="chat-area">
      {/* Hidden File Input */}
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileSelect}
        multiple
        style={{ display: 'none' }}
      />

      {/* Top Bar */}
      <div className="titlebar">
        <div className="titlebar-left">
          <span className="chat-title-text">
            {activeChat?.title || 'Greeting'}
          </span>
          <button className="dock-btn" style={{ width: 24, height: 24 }}>
            <MoreHorizontal size={14} />
          </button>
        </div>

        <div className="titlebar-right" style={{ position: 'relative' }}>
          <button
            className="dock-btn"
            style={{ width: 28, height: 28 }}
            title="Model Settings"
            onClick={() => setShowControlsPopover(!showControlsPopover)}
          >
            <SlidersHorizontal size={16} />
          </button>

          {showControlsPopover && (
            <div
              ref={controlsRef}
              style={{
                position: 'absolute',
                top: '36px',
                right: '0px',
                width: '260px',
                backgroundColor: '#1e1e1e',
                border: '1px solid rgba(255, 255, 255, 0.15)',
                borderRadius: '12px',
                boxShadow: '0 12px 36px rgba(0, 0, 0, 0.7)',
                padding: '14px',
                zIndex: 1000,
                color: '#e5e5e5',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
                <span style={{ fontSize: '11px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px', color: '#a3a3a3' }}>
                  Model Settings
                </span>
                <X size={13} style={{ cursor: 'pointer', opacity: 0.7 }} onClick={() => setShowControlsPopover(false)} />
              </div>

              <div style={{ marginBottom: '12px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: '#a3a3a3', marginBottom: '4px' }}>
                  <span>Temperature</span>
                  <span style={{ color: '#fff', fontWeight: 600 }}>{temperature}</span>
                </div>
                <input
                  type="range"
                  min="0.0"
                  max="1.0"
                  step="0.1"
                  value={temperature}
                  onChange={(e) => setTemperature(parseFloat(e.target.value))}
                  style={{ width: '100%', accentColor: 'var(--color-indigo)', cursor: 'pointer' }}
                />
              </div>

              <div style={{ marginBottom: '12px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: '#a3a3a3', marginBottom: '4px' }}>
                  <span>Max Tokens</span>
                  <span style={{ color: '#fff', fontWeight: 600 }}>{maxTokens}</span>
                </div>
                <input
                  type="range"
                  min="512"
                  max="8192"
                  step="256"
                  value={maxTokens}
                  onChange={(e) => setMaxTokens(parseInt(e.target.value, 10))}
                  style={{ width: '100%', accentColor: 'var(--color-indigo)', cursor: 'pointer' }}
                />
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', cursor: 'pointer', paddingTop: '8px', borderTop: '1px solid rgba(255,255,255,0.08)' }}>
                <input
                  type="checkbox"
                  id="devModeCheck"
                  checked={devMode}
                  onChange={(e) => setDevMode(e.target.checked)}
                  style={{ accentColor: 'var(--color-indigo)', cursor: 'pointer' }}
                />
                <label htmlFor="devModeCheck" style={{ cursor: 'pointer', color: '#d4d4d4' }}>Developer Mode</label>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Messages Feed */}
      <div className="chat-messages" id="chat-messages">
        <div className="chat-messages-inner">
          {/* Empty State Welcome View when chat has 0 messages */}
          {messages.length === 0 && (!streamingContent || streamingChatId !== activeChatId) && (
            <div className="chat-empty-state">
              <div className="empty-logo-badge">Ax</div>
              <h2 className="empty-title">What can I help you build today?</h2>
              <p className="empty-subtitle">
                Fast local AI assistant powered by your local LLMs.
              </p>

              <div className="suggested-prompts-grid">
                <button
                  type="button"
                  className="suggested-prompt-card"
                  onClick={() => setInput('Write a Rust CLI script for processing files')}
                >
                  <Code size={16} className="prompt-card-icon" style={{ color: '#10b981' }} />
                  <div className="prompt-card-text">
                    <div className="prompt-card-title">Write a Rust CLI script</div>
                    <div className="prompt-card-desc">Parse and transform data files</div>
                  </div>
                </button>

                <button
                  type="button"
                  className="suggested-prompt-card"
                  onClick={() => setInput('Explore project structure and available agent tools')}
                >
                  <Search size={16} className="prompt-card-icon" style={{ color: '#3b82f6' }} />
                  <div className="prompt-card-text">
                    <div className="prompt-card-title">Explore Project & Tools</div>
                    <div className="prompt-card-desc">Analyze codebase & capabilities</div>
                  </div>
                </button>

                <button
                  type="button"
                  className="suggested-prompt-card"
                  onClick={() => setInput('Run cargo check and fix any build warnings')}
                >
                  <Terminal size={16} className="prompt-card-icon" style={{ color: '#f59e0b' }} />
                  <div className="prompt-card-text">
                    <div className="prompt-card-title">Fix Build Warnings</div>
                    <div className="prompt-card-desc">Run checks and resolve errors</div>
                  </div>
                </button>
              </div>
            </div>
          )}

          {(() => {
            let lastUserMessageId = null;
            for (let i = messages.length - 1; i >= 0; i--) {
              if (messages[i].role === 'user') {
                lastUserMessageId = messages[i].id;
                break;
              }
            }
            return messages.map((msg) => (
              <ChatMessage 
                key={msg.id} 
                message={msg} 
                onRegenerate={handleRegenerate}
                isLatestUser={msg.id === lastUserMessageId}
              />
            ));
          })()}

          {/* Live Streaming & Thinking assistant response */}
          {(isAgentRunning || streamingContent) && (!streamingChatId || streamingChatId === activeChatId) && (
            <div className="chat-message assistant">
              {/* 1. Simple Thinking Indicator while waiting for tokens */}
              {!streamingContent && (
                <div className="thinking-status-indicator">
                  <Loader2 size={13} className="spin" style={{ color: '#8e8e8e' }} />
                  <span>
                    {agentSteps.length > 0
                      ? agentSteps[agentSteps.length - 1].description
                      : 'Thinking...'}
                  </span>
                </div>
              )}

              {/* 2. Simple Reasoning trace <think> block */}
              {streamingThinking && (
                <div className="thinking-block">
                  "{streamingThinking}"
                </div>
              )}

              {/* 3. Main response text while streaming */}
              {streamingMain && (
                <div className="message-bubble">
                  <ReactMarkdown components={markdownComponents}>{streamingMain}</ReactMarkdown>
                  {isAgentRunning && <span className="streaming-cursor" />}
                </div>
              )}
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Agent Timeline (if active) */}
      {agentSteps.length > 0 && (
        <div style={{ background: 'var(--color-bg-card-subtle)', borderTop: '1px solid var(--color-border)' }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '4px 12px',
              cursor: 'pointer',
              fontSize: 11,
              color: 'var(--color-text-muted)',
            }}
            onClick={() => setShowTimeline((v) => !v)}
          >
            <BarChart2 size={12} />
            Agent Steps ({agentSteps.length})
            <ChevronDown size={12} style={{ transform: showTimeline ? 'rotate(180deg)' : 'none' }} />
          </div>
          {showTimeline && <AgentTimeline steps={agentSteps} />}
        </div>
      )}

      {/* Floating Input Dock */}
      <div className="floating-input-container">
        <div className="floating-input-card" style={{ position: 'relative' }}>
          {/* Attachment Preview Chips Bar */}
          {attachedFiles.length > 0 && (
            <div className="attached-files-bar">
              {attachedFiles.map((file) => (
                <div key={file.id} className="attached-file-chip">
                  {file.type === 'image' && file.dataUrl ? (
                    <img src={file.dataUrl} alt={file.name} className="attached-chip-img" />
                  ) : (
                    <FileText size={14} className="attached-chip-icon" />
                  )}
                  <span className="attached-chip-name">{file.name}</span>
                  <button
                    type="button"
                    className="attached-chip-remove"
                    onClick={() => setAttachedFiles((prev) => prev.filter((f) => f.id !== file.id))}
                    title="Remove attachment"
                  >
                    <X size={12} />
                  </button>
                </div>
              ))}
            </div>
          )}

          <textarea
            id="chat-input"
            ref={textareaRef}
            className="floating-textarea"
            placeholder={isRecording ? 'Listening... Speak now...' : isTranscribing ? 'Transcribing audio...' : 'Send a Message'}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={1}
          />

          <div className="floating-input-toolbar">
            <div className="toolbar-left" style={{ position: 'relative' }}>
              {/* 1. Add Files / Context (+) */}
              <button
                className="dock-btn"
                title="Add files or context"
                onClick={() => fileInputRef.current?.click()}
              >
                <Plus size={16} />
              </button>

              {/* 2. Tools & Functions (LayoutGrid) */}
              <button
                className={`dock-btn ${showToolsMenu ? 'active' : ''}`}
                title="Tools & Functions"
                onClick={() => setShowToolsMenu((v) => !v)}
              >
                <LayoutGrid size={16} />
              </button>

              {/* Tools Menu Popover */}
              {showToolsMenu && (
                <div
                  ref={toolsRef}
                  style={{
                    position: 'absolute',
                    bottom: 'calc(100% + 8px)',
                    left: 0,
                    width: '240px',
                    backgroundColor: '#212121',
                    border: '1px solid rgba(255, 255, 255, 0.15)',
                    borderRadius: '12px',
                    boxShadow: '0 10px 30px rgba(0, 0, 0, 0.6)',
                    padding: '6px',
                    zIndex: 1000,
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 10px 4px 10px', fontSize: '10px', fontWeight: 600, color: '#737373', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                    <span>Quick Coding Helpers</span>
                    <X size={12} style={{ cursor: 'pointer', color: '#888' }} onClick={() => setShowToolsMenu(false)} />
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', marginTop: '4px' }}>
                    {availableTools.map((t) => {
                      const IconComp = t.icon;
                      return (
                        <button
                          key={t.name}
                          type="button"
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '8px',
                            width: '100%',
                            padding: '8px 10px',
                            background: 'transparent',
                            border: 'none',
                            borderRadius: '8px',
                            color: '#d4d4d4',
                            fontSize: '12px',
                            cursor: 'pointer',
                            textAlign: 'left',
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = '#2a2a2a')}
                          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
                          onClick={() => {
                            setInput((prev) => (prev ? prev + ' ' + t.snippet : t.snippet));
                            setShowToolsMenu(false);
                          }}
                        >
                          <IconComp size={14} style={{ color: '#10b981', flexShrink: 0 }} />
                          <div style={{ overflow: 'hidden' }}>
                            <div style={{ fontWeight: 500, color: '#fff' }}>{t.name}</div>
                            <div style={{ fontSize: '10px', color: '#888' }}>{t.desc}</div>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>

            <div className="toolbar-right">
              {/* Sleek Custom Model Selector Dropdown */}
              <ModelSelector />

              {/* 3. Voice Input (Mic) */}
              <button
                className={`dock-btn ${isRecording ? 'active' : ''}`}
                title={isRecording ? 'Stop Voice Recording' : isTranscribing ? 'Transcribing...' : 'Start Voice Input'}
                onClick={toggleRecording}
                disabled={isTranscribing}
                style={{ color: isRecording ? '#f43f5e' : undefined }}
              >
                {isRecording ? <MicOff size={16} className="spin" /> : <Mic size={16} />}
              </button>

              {/* Send / Terminate Button */}
              <button
                id="send-btn"
                className="dock-send-btn"
                onClick={isAgentRunning ? handleStop : handleSend}
                disabled={!input.trim() && attachedFiles.length === 0 && !isAgentRunning}
                title={isAgentRunning ? 'Terminate Generation' : 'Send Message'}
              >
                {isAgentRunning ? (
                  <Square size={14} style={{ color: '#000', fill: '#000' }} />
                ) : (
                  <ArrowUp size={18} style={{ color: '#000', strokeWidth: 2.5 }} />
                )}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
