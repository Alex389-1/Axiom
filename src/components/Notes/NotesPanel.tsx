import { useAppStore } from '../../store';
import { FileText } from 'lucide-react';

export function NotesPanel() {
  const { notesContent, setNotesContent } = useAppStore();

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--color-bg-main)' }}>
      <div style={{ 
        padding: '16px 24px', 
        borderBottom: '1px solid var(--color-border)',
        display: 'flex',
        alignItems: 'center',
        gap: '12px'
      }}>
        <FileText size={20} color="var(--color-indigo)" />
        <h2 style={{ fontSize: 18, fontWeight: 600, color: 'var(--color-text-primary)', margin: 0 }}>
          Global Notes & Scratchpad
        </h2>
      </div>
      
      <div style={{ flex: 1, padding: '24px', display: 'flex', flexDirection: 'column' }}>
        <textarea
          value={notesContent}
          onChange={(e) => setNotesContent(e.target.value)}
          placeholder="Start typing your notes here..."
          style={{
            flex: 1,
            width: '100%',
            maxWidth: '900px',
            margin: '0 auto',
            background: 'transparent',
            border: 'none',
            color: 'var(--color-text-primary)',
            fontSize: '15px',
            lineHeight: '1.6',
            resize: 'none',
            outline: 'none',
            fontFamily: 'var(--font-sans)',
          }}
        />
      </div>
    </div>
  );
}
