import React, { useState, useEffect } from 'react';

interface DialogProps {
  isOpen: boolean;
  title: string;
  message?: string;
  initialValue?: string;
  isPrompt?: boolean;
  isDestructive?: boolean;
  onConfirm: (value?: string) => void;
  onCancel: () => void;
}

export function Dialog({ isOpen, title, message, initialValue = '', isPrompt = false, isDestructive = false, onConfirm, onCancel }: DialogProps) {
  const [inputValue, setInputValue] = useState(initialValue);

  useEffect(() => {
    if (isOpen) {
      setInputValue(initialValue);
    }
  }, [isOpen, initialValue]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onConfirm(isPrompt ? inputValue : undefined);
  };

  return (
    <div style={{
      position: 'fixed',
      top: 0, left: 0, right: 0, bottom: 0,
      background: 'rgba(0, 0, 0, 0.6)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 9999,
      backdropFilter: 'blur(4px)',
    }}>
      <form 
        onSubmit={handleSubmit}
        style={{
          background: 'var(--color-bg-main, #212121)',
          border: '1px solid var(--color-border)',
          borderRadius: 12,
          padding: 20,
          width: '100%',
          maxWidth: 320,
          boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
        }}
      >
        <h3 style={{ margin: '0 0 12px 0', fontSize: 16, fontWeight: 600, color: 'var(--color-text-primary)' }}>
          {title}
        </h3>
        
        {message && (
          <p style={{ margin: '0 0 16px 0', fontSize: 14, color: 'var(--color-text-muted)', lineHeight: 1.5 }}>
            {message}
          </p>
        )}

        {isPrompt && (
          <input
            autoFocus
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            style={{
              width: '100%',
              padding: '10px 12px',
              borderRadius: 6,
              border: '1px solid var(--color-border)',
              background: 'var(--color-bg-base, #171717)',
              color: 'var(--color-text-primary)',
              fontSize: 14,
              marginBottom: 16,
              outline: 'none',
              boxSizing: 'border-box',
            }}
          />
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12, marginTop: 8 }}>
          <button
            type="button"
            onClick={onCancel}
            style={{
              padding: '8px 16px',
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              color: 'var(--color-text-muted)',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: 500,
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.05)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
          >
            Cancel
          </button>
          <button
            type="submit"
            style={{
              padding: '8px 16px',
              borderRadius: 6,
              border: 'none',
              background: isDestructive ? 'var(--color-rose, #f43f5e)' : 'var(--color-emerald, #10b981)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: 500,
            }}
            onMouseEnter={(e) => e.currentTarget.style.opacity = '0.9'}
            onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
          >
            {isPrompt ? 'Save' : (isDestructive ? 'Delete' : 'Confirm')}
          </button>
        </div>
      </form>
    </div>
  );
}
