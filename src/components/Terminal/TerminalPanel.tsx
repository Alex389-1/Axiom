import { useEffect, useRef, useState } from 'react';
import { Terminal as TerminalIcon, Maximize2, Minimize2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store';

let TerminalClass: any = null;
let FitAddon: any = null;
let WebLinksAddon: any = null;

async function loadXterm() {
  if (!TerminalClass) {
    const [{ Terminal }, { FitAddon: FA }, { WebLinksAddon: WLA }] = await Promise.all([
      import('@xterm/xterm'),
      import('@xterm/addon-fit'),
      import('@xterm/addon-web-links'),
    ]);
    TerminalClass = Terminal;
    FitAddon = FA;
    WebLinksAddon = WLA;
    await import('@xterm/xterm/css/xterm.css');
  }
}

export function TerminalPanel() {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<any>(null);
  const fitAddonRef = useRef<any>(null);
  const [expanded, setExpanded] = useState(false);
  const { sessionId, terminalCollapsed, toggleTerminal } = useAppStore();
  const [initialized, setInitialized] = useState(false);

  useEffect(() => {
    if (terminalCollapsed || !containerRef.current || !sessionId) return;

    let cancelled = false;

    (async () => {
      await loadXterm();
      if (cancelled || !containerRef.current) return;

      const term = new TerminalClass({
        theme: {
          background: '#121212',
          foreground: '#ececec',
          cursor: '#ffffff',
          cursorAccent: '#121212',
          selectionBackground: 'rgba(255,255,255,0.2)',
          black: '#121212',
          red: '#f85149',
          green: '#10b981',
          yellow: '#f59e0b',
          blue: '#388bfd',
          magenta: '#bc8cff',
          cyan: '#39c5cf',
          white: '#b1bac4',
        },
        fontFamily: "'JetBrains Mono', monospace",
        fontSize: 12,
        lineHeight: 1.4,
        cursorBlink: true,
        cursorStyle: 'bar',
        scrollback: 10000,
        convertEol: true,
      });

      const fitAddon = new FitAddon();
      const webLinksAddon = new WebLinksAddon();

      term.loadAddon(fitAddon);
      term.loadAddon(webLinksAddon);
      term.open(containerRef.current!);
      fitAddon.fit();

      termRef.current = term;
      fitAddonRef.current = fitAddon;
      setInitialized(true);

      term.onData(async (data: string) => {
        try {
          await invoke('write_terminal', { sessionId, input: data });
        } catch {}
      });

      try {
        const lines = await invoke<string[]>('get_terminal_output', { sessionId, lines: 100 });
        lines.forEach((line) => term.writeln(line));
      } catch {}
    })();

    return () => {
      cancelled = true;
      if (termRef.current) {
        termRef.current.dispose();
        termRef.current = null;
      }
    };
  }, [sessionId, terminalCollapsed]);

  useEffect(() => {
    if (terminalCollapsed || !containerRef.current || !fitAddonRef.current) return;
    const obs = new ResizeObserver(() => {
      if (fitAddonRef.current) fitAddonRef.current.fit();
    });
    obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [initialized, terminalCollapsed]);

  useEffect(() => {
    if (terminalCollapsed || !sessionId || !termRef.current) return;
    let lastLineCount = 0;

    const interval = setInterval(async () => {
      try {
        const lines = await invoke<string[]>('get_terminal_output', { sessionId, lines: 500 });
        if (lines.length > lastLineCount) {
          const newLines = lines.slice(lastLineCount);
          newLines.forEach((line) => termRef.current?.writeln(line));
          lastLineCount = lines.length;
        }
      } catch {}
    }, 300);

    return () => clearInterval(interval);
  }, [sessionId, initialized, terminalCollapsed]);

  // Listen for LLM background command outputs
  useEffect(() => {
    const handleLlmTerminal = (e: Event) => {
      const customEvent = e as CustomEvent<string>;
      console.log('[DEBUG] TerminalPanel received llm_terminal event:', customEvent.detail);
      if (termRef.current) {
        termRef.current.write(customEvent.detail);
      }
    };
    window.addEventListener('llm_terminal', handleLlmTerminal);
    return () => window.removeEventListener('llm_terminal', handleLlmTerminal);
  }, [initialized]);

  if (terminalCollapsed) {
    return null;
  }

  return (
    <div
      className="terminal-panel"
      style={expanded ? {
        position: 'fixed',
        inset: 20,
        bottom: 30,
        zIndex: 50,
        borderRadius: '12px',
        border: '1px solid var(--color-border)',
        boxShadow: 'var(--shadow-lg)',
        height: 'calc(100vh - 60px)',
      } : {}}
    >
      <div className="terminal-panel-header">
        <TerminalIcon size={12} style={{ color: 'var(--color-emerald)' }} />
        <span className="terminal-panel-title">Terminal</span>
        <div style={{ flex: 1 }} />
        <button
          className="dock-btn"
          onClick={() => setExpanded((v) => !v)}
          title={expanded ? 'Minimize' : 'Expand'}
          style={{ width: 24, height: 24 }}
        >
          {expanded ? <Minimize2 size={12} /> : <Maximize2 size={12} />}
        </button>
        <button
          className="dock-btn"
          onClick={toggleTerminal}
          title="Close / Collapse Terminal"
          style={{ width: 24, height: 24 }}
        >
          <X size={12} />
        </button>
      </div>

      <div className="terminal-wrapper" ref={containerRef} style={{ minHeight: 0 }}>
        {!sessionId && (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: 'var(--color-text-muted)',
            fontSize: 12,
            fontFamily: 'JetBrains Mono',
          }}>
            Terminal Ready
          </div>
        )}
      </div>
    </div>
  );
}
