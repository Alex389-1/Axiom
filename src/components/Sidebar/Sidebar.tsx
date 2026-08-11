import React, { useState } from 'react';
import {
  PanelLeft,
  Plus,
  Search,
  FileText,
  LayoutGrid,
  FolderPlus,
  Trash2,
  MessageSquare,
  X,
} from 'lucide-react';
import { useAppStore } from '../../store';

export function Sidebar() {
  const {
    chats,
    activeChatId,
    sidebarCollapsed,
    projectPath,
    addChat,
    deleteChat,
    selectChat,
    toggleSidebar,
  } = useAppStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [showSearchInput, setShowSearchInput] = useState(false);

  const handleNewChat = (e: React.MouseEvent) => {
    e.stopPropagation();
    addChat();
  };

  const handleNotes = (e: React.MouseEvent) => {
    e.stopPropagation();
    const existingNotes = chats.find((c) => c.title.toLowerCase().includes('notes'));
    if (existingNotes) {
      selectChat(existingNotes.id);
    } else {
      addChat('Notes & Scratchpad');
    }
  };

  const handleWorkspace = (e: React.MouseEvent) => {
    e.stopPropagation();
    addChat('Workspace Overview');
  };

  const handleDeleteChat = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    deleteChat(id);
  };

  const filteredChats = chats.filter((c) =>
    c.title.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (sidebarCollapsed) {
    return (
      <aside className="sidebar collapsed" style={{ width: '48px', padding: '12px 6px', alignItems: 'center' }}>
        <button
          className="dock-btn"
          title="Open Sidebar"
          onClick={toggleSidebar}
          style={{ width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center' }}
        >
          <PanelLeft size={18} />
        </button>
        <button
          className="dock-btn"
          title="New Chat"
          onClick={handleNewChat}
          style={{ width: 32, height: 32, marginTop: 12, display: 'flex', alignItems: 'center', justifyContent: 'center' }}
        >
          <Plus size={18} />
        </button>
      </aside>
    );
  }

  return (
    <aside className="sidebar">
      {/* Sidebar Header with Axiom branding */}
      <div className="sidebar-header">
        <div className="sidebar-brand">
          <div className="brand-icon">Ax</div>
          <span>Axiom</span>
        </div>
        <button
          className="dock-btn"
          title="Collapse Sidebar"
          onClick={toggleSidebar}
          style={{ width: 28, height: 28 }}
        >
          <PanelLeft size={16} />
        </button>
      </div>

      {/* Main Actions */}
      <div className="sidebar-section">
        <div
          className="sidebar-nav-item"
          onClick={handleNewChat}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <Plus className="icon" size={16} />
          <span>New Chat</span>
        </div>

        <div
          className="sidebar-nav-item"
          onClick={() => setShowSearchInput(!showSearchInput)}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <Search className="icon" size={16} />
          <span>Search</span>
        </div>

        {showSearchInput && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, margin: '4px 8px', background: 'rgba(255,255,255,0.06)', borderRadius: 6, padding: '4px 8px' }}>
            <input
              type="text"
              placeholder="Filter chats..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              autoFocus
              style={{ background: 'transparent', border: 'none', color: '#fff', fontSize: 12, outline: 'none', width: '100%' }}
            />
            {searchQuery && (
              <X size={12} style={{ cursor: 'pointer', opacity: 0.7 }} onClick={() => setSearchQuery('')} />
            )}
          </div>
        )}

        <div
          className="sidebar-nav-item"
          onClick={handleNotes}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <FileText className="icon" size={16} />
          <span>Notes</span>
        </div>

        <div
          className="sidebar-nav-item"
          onClick={handleWorkspace}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <LayoutGrid className="icon" size={16} />
          <span>Workspace</span>
        </div>
      </div>

      {/* Folders Section */}
      <div className="sidebar-section">
        <div className="sidebar-section-title">
          <span>Folders</span>
          <FolderPlus
            size={14}
            style={{ cursor: 'pointer', opacity: 0.7 }}
            onClick={handleNewChat}
          />
        </div>
        {projectPath && (
          <div style={{ fontSize: 11, color: 'var(--color-text-muted)', padding: '4px 8px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            📁 {projectPath.split('/').pop()}
          </div>
        )}
      </div>

      {/* Dynamic User Chats Section */}
      <div className="sidebar-section" style={{ flex: 1, overflowY: 'auto' }}>
        <div className="sidebar-section-title">
          <span>Chats</span>
          <button
            onClick={handleNewChat}
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
            title="Create New Chat"
          >
            <Plus size={14} style={{ opacity: 0.7 }} />
          </button>
        </div>

        <div style={{ paddingLeft: 4, marginTop: 4 }}>
          <div style={{ fontSize: 11, color: 'var(--color-text-muted)', margin: '6px 8px 4px 8px' }}>
            Today
          </div>

          {filteredChats.map((chat) => {
            const isActive = chat.id === activeChatId;
            return (
              <div
                key={chat.id}
                className={`sidebar-nav-item ${isActive ? 'active' : ''}`}
                onClick={() => selectChat(chat.id)}
                style={{ justifyContent: 'space-between', cursor: 'pointer' }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, overflow: 'hidden' }}>
                  <MessageSquare size={14} style={{ opacity: isActive ? 0.9 : 0.5, flexShrink: 0 }} />
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {chat.title}
                  </span>
                </div>

                <button
                  onClick={(e) => handleDeleteChat(e, chat.id)}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: 'var(--color-text-muted)',
                    cursor: 'pointer',
                    padding: 2,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                  title="Delete Chat"
                >
                  <Trash2 size={13} style={{ opacity: 0.6 }} />
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </aside>
  );
}
