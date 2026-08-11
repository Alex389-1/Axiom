import { useState, useRef, useEffect } from 'react';
import {
  Pencil,
  Copy,
  Volume2,
  VolumeX,
  Info,
  ThumbsUp,
  ThumbsDown,
  RotateCcw,
  Share2,
  Check,
  ChevronDown,
  ChevronUp,
  CheckCircle2,
  AlertTriangle,
  Loader2,
  X,
} from 'lucide-react';
import type { Message, ToolCall, ToolResult } from '../../store';
import { useAppStore } from '../../store';
import ReactMarkdown from 'react-markdown';

export const markdownComponents = {
  img: ({ node, src, alt, ...props }: any) => {
    if (!src) return null;
    return (
      <img
        src={src}
        alt={alt || 'Attached image'}
        style={{
          maxWidth: '100%',
          maxHeight: '360px',
          borderRadius: '8px',
          marginTop: '6px',
          marginBottom: '6px',
          display: 'block',
          objectFit: 'contain',
        }}
        {...props}
      />
    );
  },
  code: ({ node, inline, className, children, ...props }: any) => {
    const match = /language-(\w+)/.exec(className || '');
    const language = match ? match[1] : '';
    // A block usually has language-x class, or is wrapped in pre (handled natively by react-markdown if we don't override pre, but code is enough).
    // Sometimes no language is specified but it is a block (contains newline).
    const isInline = inline !== undefined ? inline : (!match && !String(children).includes('\n'));

    if (isInline) {
      return (
        <code
          style={{
            backgroundColor: 'rgba(255, 255, 255, 0.1)',
            padding: '2px 6px',
            borderRadius: '4px',
            fontFamily: 'JetBrains Mono, monospace',
            fontSize: '13px',
          }}
          {...props}
        >
          {children}
        </code>
      );
    }

    const codeString = String(children).replace(/\n$/, '');

    return (
      <div style={{ position: 'relative', margin: '16px 0', borderRadius: '8px', overflow: 'hidden', border: '1px solid var(--color-border)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', backgroundColor: '#1e1e1e', padding: '6px 12px', borderBottom: '1px solid var(--color-border)' }}>
          <span style={{ fontSize: '11px', color: '#a3a3a3', fontFamily: 'JetBrains Mono, monospace', textTransform: 'uppercase' }}>
            {language || 'text'}
          </span>
          <button
            onClick={(e) => {
              navigator.clipboard.writeText(codeString);
              const btn = e.currentTarget;
              const originalHtml = btn.innerHTML;
              btn.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: #10b981"><polyline points="20 6 9 17 4 12"></polyline></svg> Copied';
              setTimeout(() => {
                btn.innerHTML = originalHtml;
              }, 2000);
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              background: 'transparent',
              border: 'none',
              color: '#a3a3a3',
              cursor: 'pointer',
              fontSize: '11px',
            }}
            title="Copy Code"
          >
            <Copy size={12} /> Copy
          </button>
        </div>
        <pre style={{ margin: 0, padding: '12px', backgroundColor: '#0d0d0d', overflowX: 'auto' }}>
          <code style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: '13px', color: '#e5e5e5', display: 'block' }} {...props}>
            {children}
          </code>
        </pre>
      </div>
    );
  },
};

// ─── ToolCard ──────────────────────────────────────────────────────────────────

interface ToolCardProps {
  call: ToolCall;
  result?: ToolResult;
}

export function ToolCard({ call, result }: ToolCardProps) {
  const [expanded, setExpanded] = useState(false);

  const outputText = result
    ? result.output.status === 'success'
      ? [(result.output as any).stdout, (result.output as any).stderr].filter(Boolean).join('\n')
      : (result.output as any).message
    : '';

  return (
    <div className="tool-card">
      <div className="tool-card-header" onClick={() => setExpanded((e) => !e)}>
        <span className="tool-card-label">{call.tool}</span>
        {result && (
          <span style={{ fontSize: 11, color: 'var(--color-text-muted)', fontFamily: 'JetBrains Mono' }}>
            {result.duration_ms}ms
          </span>
        )}
        {expanded ? <ChevronUp size={14} style={{ color: 'var(--color-text-muted)' }} /> : <ChevronDown size={14} style={{ color: 'var(--color-text-muted)' }} />}
      </div>

      {expanded && (
        <div className="tool-card-body">
          <div style={{ color: 'var(--color-text-muted)', marginBottom: 6 }}>
            Arguments: {JSON.stringify(call.arguments, null, 2)}
          </div>
          {outputText && <div>{outputText}</div>}
        </div>
      )}
    </div>
  );
}

// ─── ChatMessage Component ───────────────────────────────────────────────────

interface ChatMessageProps {
  message: Message;
  onRegenerate?: (messageId: string, newContent?: string) => void;
  isLatestUser?: boolean;
}

export function ChatMessage({ message, onRegenerate, isLatestUser }: ChatMessageProps) {
  const editMessage = useAppStore((s) => s.editMessage);

  const [copied, setCopied] = useState(false);
  const [shared, setShared] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [showInfo, setShowInfo] = useState(false);
  const [feedback, setFeedback] = useState<'liked' | 'disliked' | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState(message.content);
  const infoRef = useRef<HTMLDivElement>(null);
  const utteranceRef = useRef<SpeechSynthesisUtterance | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const isUser = message.role === 'user';
  const isTool = message.role === 'tool';
  const modelName = message.model || 'qwen2.5-coder:14b';

  // Close info modal on click outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (infoRef.current && !infoRef.current.contains(e.target as Node)) {
        setShowInfo(false);
      }
    }
    if (showInfo) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showInfo]);

  // 1. Copy
  const handleCopy = () => {
    navigator.clipboard.writeText(message.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // 2. Share
  const handleShare = () => {
    const formatted = `### Assistant Response (${modelName})\n\n${message.content}`;
    navigator.clipboard.writeText(formatted);
    setShared(true);
    setTimeout(() => setShared(false), 2000);
  };

  // 3. Text to Speech
  const handleReadAloud = () => {
    if (isSpeaking) {
      if ('speechSynthesis' in window) {
        window.speechSynthesis.cancel();
      }
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current = null;
      }
      setIsSpeaking(false);
      return;
    }

    const speechText = message.content.replace(/<think>[\s\S]*?<\/think>/g, '').trim();
    if (!speechText) return;

    if ('speechSynthesis' in window) {
      const voices = window.speechSynthesis.getVoices();
      if (voices.length > 0) {
        const utterance = new SpeechSynthesisUtterance(speechText);
        utterance.lang = 'en-US'; 
        utterance.onend = () => { setIsSpeaking(false); utteranceRef.current = null; };
        utterance.onerror = () => { setIsSpeaking(false); utteranceRef.current = null; };
        utteranceRef.current = utterance;
        window.speechSynthesis.speak(utterance);
        setIsSpeaking(true);
        return;
      }
    }

    // API Fallback for systems without native voices (Linux webkit2gtk bug)
    const chunks = speechText.match(/[^.!?\n]+[.!?\n]*/g) || [speechText];
    let currentChunk = 0;

    const playNext = () => {
      if (currentChunk >= chunks.length) {
        setIsSpeaking(false);
        return;
      }

      let chunk = chunks[currentChunk].trim();
      if (chunk.length > 200) chunk = chunk.substring(0, 197) + '...';
      
      if (!chunk) {
        currentChunk++;
        playNext();
        return;
      }

      const url = `https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=en&q=${encodeURIComponent(chunk)}`;
      const audio = new Audio(url);
      audioRef.current = audio;
      
      audio.onended = () => {
        currentChunk++;
        playNext();
      };
      audio.onerror = () => setIsSpeaking(false);
      audio.play().catch(() => setIsSpeaking(false));
    };

    setIsSpeaking(true);
    playNext();
  };

  // 4. Save Edit
  const handleSaveEdit = () => {
    if (editContent.trim()) {
      if (isUser && isLatestUser) {
        onRegenerate?.(message.id, editContent.trim());
        setIsEditing(false);
      } else {
        editMessage(message.id, editContent.trim());
        setIsEditing(false);
      }
    }
  };

  if (isTool && message.tool_call) {
    return (
      <div className="chat-message assistant">
        <ToolCard call={message.tool_call} result={message.tool_result} />
      </div>
    );
  }

  // Parse out internal thinking reasoning trace if present
  let thinkingContent: string | null = null;
  let mainContent = message.content;

  if (!isUser) {
    if (message.content.includes('<think>')) {
      const thinkMatch = message.content.match(/<think>([\s\S]*?)<\/think>/);
      if (thinkMatch) {
        thinkingContent = thinkMatch[1].trim();
        mainContent = message.content.replace(/<think>[\s\S]*?<\/think>/, '').trim();
      }
    }
    // Clean out raw embedded tool call JSON strings and leftover tags
    mainContent = mainContent
      .replace(/\{\s*"tool"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:\s*\{[\s\S]*?\}\s*\}/g, '')
      .replace(/<\/think>/g, '')
      .trim();
  }

  const charCount = message.content.length;
  const wordCount = message.content.trim().split(/\s+/).filter(Boolean).length;
  const estTokens = Math.round(charCount / 4);

  return (
    <div className={`chat-message ${isUser ? 'user' : 'assistant'}`}>
      {/* Thinking / Reasoning trace block */}
      {!isUser && thinkingContent && (
        <div className="thinking-block">
          "{thinkingContent}"
        </div>
      )}

      {/* Message Bubble or Inline Editor */}
      {isEditing ? (
        <div 
          className="inline-message-editor" 
          style={{ 
            display: 'flex', 
            flexDirection: 'column', 
            gap: 8, 
            margin: '8px 0',
            backgroundColor: 'var(--color-bg-card)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            padding: '12px',
            boxShadow: 'var(--shadow-sm)'
          }}
        >
          <textarea
            value={editContent}
            onChange={(e) => setEditContent(e.target.value)}
            style={{
              width: '100%',
              minHeight: '80px',
              backgroundColor: 'transparent',
              color: 'var(--color-text-primary)',
              border: 'none',
              fontSize: '14px',
              lineHeight: 1.5,
              fontFamily: 'inherit',
              outline: 'none',
              resize: 'vertical',
            }}
          />
          <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', paddingTop: '8px' }}>
            <button
              onClick={() => setIsEditing(false)}
              style={{
                background: 'var(--color-bg-hover)',
                color: 'var(--color-text-primary)',
                border: 'none',
                borderRadius: 'var(--radius-pill)',
                padding: '6px 16px',
                fontSize: '13px',
                fontWeight: 500,
                cursor: 'pointer',
                transition: 'background-color var(--duration-fast)',
              }}
              onMouseOver={(e) => (e.currentTarget.style.backgroundColor = 'var(--color-bg-active)')}
              onMouseOut={(e) => (e.currentTarget.style.backgroundColor = 'var(--color-bg-hover)')}
            >
              Cancel
            </button>
            <button
              onClick={handleSaveEdit}
              style={{
                background: 'var(--color-text-primary)',
                color: 'var(--color-bg-base)',
                border: 'none',
                borderRadius: 'var(--radius-pill)',
                padding: '6px 16px',
                fontSize: '13px',
                fontWeight: 600,
                cursor: 'pointer',
                transition: 'opacity var(--duration-fast)',
              }}
              onMouseOver={(e) => (e.currentTarget.style.opacity = '0.8')}
              onMouseOut={(e) => (e.currentTarget.style.opacity = '1')}
            >
              Send
            </button>
          </div>
        </div>
      ) : (
        <div className="message-bubble">
          {isUser ? (
            message.content.includes('![') ? (
              <ReactMarkdown components={markdownComponents}>{message.content}</ReactMarkdown>
            ) : (
              <span style={{ whiteSpace: 'pre-wrap' }}>{message.content}</span>
            )
          ) : (
            <ReactMarkdown components={markdownComponents}>{mainContent}</ReactMarkdown>
          )}
          {message.streaming && <span className="streaming-cursor" />}
        </div>
      )}

      {/* User Action Buttons Toolbar (Only for the latest user message) */}
      {isUser && isLatestUser && (
        <div className="message-actions-toolbar" style={{ position: 'relative' }}>
          {/* 1. Edit */}
          <button
            className={`action-btn ${isEditing ? 'active' : ''}`}
            onClick={() => setIsEditing((v) => !v)}
            title="Edit Message"
          >
            <Pencil size={14} />
          </button>

          {/* 2. Copy */}
          <button className="action-btn" onClick={handleCopy} title="Copy Content">
            {copied ? <Check size={14} style={{ color: '#10b981' }} /> : <Copy size={14} />}
          </button>

          {/* 3. Read Aloud */}
          <button
            className={`action-btn ${isSpeaking ? 'active' : ''}`}
            onClick={handleReadAloud}
            title={isSpeaking ? 'Stop Reading' : 'Read Aloud'}
          >
            {isSpeaking ? (
              <VolumeX size={14} style={{ color: '#10b981' }} />
            ) : (
              <Volume2 size={14} />
            )}
          </button>

          {/* 4. Retry */}
          <button
            className="action-btn"
            onClick={() => onRegenerate?.(message.id)}
            title="Retry Output"
          >
            <RotateCcw size={14} />
          </button>
        </div>
      )}

      {/* Assistant Action Buttons Toolbar */}
      {!isUser && !message.streaming && (
        <div className="message-actions-toolbar" style={{ position: 'relative' }}>
          {/* 1. Edit */}
          <button
            className={`action-btn ${isEditing ? 'active' : ''}`}
            onClick={() => setIsEditing((v) => !v)}
            title="Edit Message"
          >
            <Pencil size={14} />
          </button>

          {/* 2. Copy */}
          <button className="action-btn" onClick={handleCopy} title="Copy Content">
            {copied ? <Check size={14} style={{ color: '#10b981' }} /> : <Copy size={14} />}
          </button>

          {/* 3. Read Aloud */}
          <button
            className={`action-btn ${isSpeaking ? 'active' : ''}`}
            onClick={handleReadAloud}
            title={isSpeaking ? 'Stop Reading' : 'Read Aloud'}
          >
            {isSpeaking ? (
              <VolumeX size={14} style={{ color: '#10b981' }} />
            ) : (
              <Volume2 size={14} />
            )}
          </button>

          {/* 4. Info Modal Popover */}
          <button
            className={`action-btn ${showInfo ? 'active' : ''}`}
            onClick={() => setShowInfo((v) => !v)}
            title="Message Details"
          >
            <Info size={14} />
          </button>

          {/* 5. Good Response (Thumbs Up) */}
          <button
            className={`action-btn ${feedback === 'liked' ? 'active' : ''}`}
            onClick={() => setFeedback((f) => (f === 'liked' ? null : 'liked'))}
            title="Good Response"
          >
            <ThumbsUp size={14} style={{ color: feedback === 'liked' ? '#10b981' : undefined }} />
          </button>

          {/* 6. Bad Response (Thumbs Down) */}
          <button
            className={`action-btn ${feedback === 'disliked' ? 'active' : ''}`}
            onClick={() => setFeedback((f) => (f === 'disliked' ? null : 'disliked'))}
            title="Bad Response"
          >
            <ThumbsDown size={14} style={{ color: feedback === 'disliked' ? '#f43f5e' : undefined }} />
          </button>

          {/* 7. Regenerate */}
          <button
            className="action-btn"
            onClick={() => onRegenerate?.(message.id)}
            title="Regenerate Response"
          >
            <RotateCcw size={14} />
          </button>

          {/* 8. Share */}
          <button className="action-btn" onClick={handleShare} title="Share / Copy Markdown">
            {shared ? <Check size={14} style={{ color: '#10b981' }} /> : <Share2 size={14} />}
          </button>

          {/* Info Popover Modal */}
          {showInfo && (
            <div
              ref={infoRef}
              style={{
                position: 'absolute',
                bottom: 'calc(100% + 6px)',
                left: 0,
                backgroundColor: '#212121',
                border: '1px solid #333',
                borderRadius: '10px',
                padding: '10px 14px',
                boxShadow: '0 10px 25px rgba(0,0,0,0.6)',
                zIndex: 100,
                minWidth: '200px',
                fontSize: '12px',
                color: '#d4d4d4',
                display: 'flex',
                flexDirection: 'column',
                gap: '6px',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontWeight: 600, borderBottom: '1px solid #333', paddingBottom: 4 }}>
                <span>Message Details</span>
                <X size={12} style={{ cursor: 'pointer', color: '#888' }} onClick={() => setShowInfo(false)} />
              </div>
              <div><strong style={{ color: '#888' }}>Model:</strong> {modelName}</div>
              <div><strong style={{ color: '#888' }}>Role:</strong> {message.role}</div>
              <div><strong style={{ color: '#888' }}>Length:</strong> {wordCount} words ({charCount} chars)</div>
              <div><strong style={{ color: '#888' }}>Est. Tokens:</strong> ~{estTokens} tokens</div>
              <div><strong style={{ color: '#888' }}>Time:</strong> {new Date(message.timestamp).toLocaleTimeString()}</div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── AgentTimeline Component ──────────────────────────────────────────────────

interface AgentTimelineProps {
  steps: Array<{ step: number; description: string; status: string }>;
}

export function AgentTimeline({ steps }: AgentTimelineProps) {
  if (steps.length === 0) return null;

  return (
    <div style={{ padding: '8px 12px', display: 'flex', flexDirection: 'column', gap: 4 }}>
      {steps.map((s) => (
        <div key={s.step} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--color-text-secondary)' }}>
          {s.status === 'completed' && <CheckCircle2 size={12} style={{ color: 'var(--color-emerald)' }} />}
          {s.status === 'running' && <Loader2 size={12} className="spin" style={{ color: 'var(--color-amber)' }} />}
          {s.status === 'warning' && <AlertTriangle size={12} style={{ color: 'var(--color-rose)' }} />}
          <span style={{ fontFamily: 'JetBrains Mono' }}>{s.description}</span>
        </div>
      ))}
    </div>
  );
}
