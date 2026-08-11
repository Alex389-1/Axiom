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
  ChevronRight,
  ChevronDown,
  MoreVertical,
  Pin,
  Edit2,
  FolderMinus,
  Folder,
  MessageSquarePlus,
} from 'lucide-react';
import { useAppStore } from '../../store';
import { Dialog } from '../ui/Dialog';

export function Sidebar() {
  const {
    chats,
    activeChatId,
    sidebarCollapsed,
    addChat,
    deleteChat,
    selectChat,
    toggleSidebar,
    currentView,
    setCurrentView,
    folders,
    addFolder,
    toggleFolder,
    deleteFolder,
    activeFolderId,
    setActiveFolderId,
    renameChat,
    renameFolder,
    setChatFolder,
    toggleChatPin,
    toggleFolderPin,
  } = useAppStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [showSearchInput, setShowSearchInput] = useState(false);
  const [openDropdownId, setOpenDropdownId] = useState<string | null>(null);
  const [openFolderDropdownId, setOpenFolderDropdownId] = useState<string | null>(null);
  
  const [dialogState, setDialogState] = useState<{
    isOpen: boolean;
    type: 'new_folder' | 'rename_folder' | 'rename_chat' | 'delete_folder' | null;
    targetId: string | null;
    initialValue: string;
    message: string;
  }>({ isOpen: false, type: null, targetId: null, initialValue: '', message: '' });

  const closeDialog = () => setDialogState({ isOpen: false, type: null, targetId: null, initialValue: '', message: '' });

  const handleDialogConfirm = (val?: string) => {
    if (dialogState.type === 'new_folder' && val && val.trim()) {
      addFolder(val.trim());
    } else if (dialogState.type === 'rename_folder' && dialogState.targetId && val && val.trim()) {
      renameFolder(dialogState.targetId, val.trim());
    } else if (dialogState.type === 'rename_chat' && dialogState.targetId && val && val.trim()) {
      renameChat(dialogState.targetId, val.trim());
    } else if (dialogState.type === 'delete_folder' && dialogState.targetId) {
      deleteFolder(dialogState.targetId);
    }
    closeDialog();
  };

  const handleNewChat = (e: React.MouseEvent) => {
    e.stopPropagation();
    addChat();
  };

  const handleNotes = (e: React.MouseEvent) => {
    e.stopPropagation();
    setCurrentView('notes');
  };

  const handleWorkspace = (e: React.MouseEvent) => {
    e.stopPropagation();
    setCurrentView('workspace');
  };

  const handleNewFolder = (e: React.MouseEvent) => {
    e.stopPropagation();
    setDialogState({ isOpen: true, type: 'new_folder', targetId: null, initialValue: '', message: '' });
  };

  const handleDeleteChat = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    deleteChat(id);
    setOpenDropdownId(null);
  };

  const handleRenameFolder = (e: React.MouseEvent, id: string, currentTitle: string) => {
    e.stopPropagation();
    setDialogState({ isOpen: true, type: 'rename_folder', targetId: id, initialValue: currentTitle, message: '' });
    setOpenFolderDropdownId(null);
  };

  const handleRenameChat = (e: React.MouseEvent, id: string, currentTitle: string) => {
    e.stopPropagation();
    setDialogState({ isOpen: true, type: 'rename_chat', targetId: id, initialValue: currentTitle, message: '' });
    setOpenDropdownId(null);
  };

  const renderChatOptions = (dropdownContextId: string, chatId: string, currentTitle: string, currentFolderId?: string, isPinned?: boolean) => {
    if (openDropdownId !== dropdownContextId) return null;
    return (
      <div 
        style={{
          position: 'absolute',
          right: 24,
          top: 24,
          background: 'var(--color-bg-popover, #2d2d30)',
          border: '1px solid var(--color-border)',
          borderRadius: 8,
          padding: 4,
          zIndex: 100,
          boxShadow: '0 4px 12px rgba(0,0,0,0.2)',
          minWidth: 140,
        }}
        onMouseLeave={() => setOpenDropdownId(null)}
      >
        <div 
          onClick={(e) => { e.stopPropagation(); toggleChatPin(chatId); setOpenDropdownId(null); }}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Pin size={12} /> {isPinned ? 'Unpin' : 'Pin'}
        </div>
        <div 
          onClick={(e) => handleRenameChat(e, chatId, currentTitle)}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Edit2 size={12} /> Rename
        </div>
        <div 
          onClick={(e) => handleDeleteChat(e, chatId)}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-rose)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Trash2 size={12} /> Delete
        </div>
        <div style={{ height: 1, background: 'var(--color-border)', margin: '4px 0' }} />
        <div style={{ padding: '4px 12px', fontSize: 10, color: 'var(--color-text-muted)', textTransform: 'uppercase' }}>
          Move to
        </div>
        {currentFolderId && (
          <div 
            onClick={(e) => { e.stopPropagation(); setChatFolder(chatId, null); setOpenDropdownId(null); }}
            style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, display: 'flex', alignItems: 'center', gap: 8 }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
          >
            <FolderMinus size={12} /> (Remove from folder)
          </div>
        )}
        {folders.filter(f => f.id !== currentFolderId).map(f => (
          <div 
            key={f.id}
            onClick={(e) => { e.stopPropagation(); setChatFolder(chatId, f.id); setOpenDropdownId(null); }}
            style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', display: 'flex', alignItems: 'center', gap: 8 }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
          >
            <Folder size={12} style={{ flexShrink: 0 }} /> {f.name}
          </div>
        ))}
      </div>
    );
  };

  const renderFolderOptions = (dropdownContextId: string, folderId: string, currentTitle: string, isPinned?: boolean) => {
    if (openFolderDropdownId !== dropdownContextId) return null;
    return (
      <div 
        style={{
          position: 'absolute',
          right: 24,
          top: 24,
          background: 'var(--color-bg-popover, #2d2d30)',
          border: '1px solid var(--color-border)',
          borderRadius: 8,
          padding: 4,
          zIndex: 100,
          boxShadow: '0 4px 12px rgba(0,0,0,0.2)',
          minWidth: 140,
        }}
        onMouseLeave={() => setOpenFolderDropdownId(null)}
      >
        <div 
          onClick={(e) => { 
            e.stopPropagation(); 
            addChat(undefined, folderId);
            setOpenFolderDropdownId(null);
          }}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <MessageSquarePlus size={12} /> New Chat
        </div>
        <div 
          onClick={(e) => { e.stopPropagation(); toggleFolderPin(folderId); setOpenFolderDropdownId(null); }}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Pin size={12} /> {isPinned ? 'Unpin' : 'Pin'}
        </div>
        <div 
          onClick={(e) => handleRenameFolder(e, folderId, currentTitle)}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Edit2 size={12} /> Rename
        </div>
        <div style={{ height: 1, background: 'var(--color-border)', margin: '4px 0' }} />
        <div 
          onClick={(e) => {
            e.stopPropagation();
            setDialogState({ isOpen: true, type: 'delete_folder', targetId: folderId, initialValue: '', message: `Delete folder "${currentTitle}"? Chats will be kept.` });
            setOpenFolderDropdownId(null);
          }}
          style={{ padding: '6px 12px', fontSize: 12, cursor: 'pointer', borderRadius: 4, color: 'var(--color-rose)', display: 'flex', alignItems: 'center', gap: 8 }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
        >
          <Trash2 size={12} /> Delete
        </div>
      </div>
    );
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
          className={`sidebar-nav-item ${currentView === 'notes' ? 'active' : ''}`}
          onClick={handleNotes}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <FileText className="icon" size={16} />
          <span>Notes</span>
        </div>

        <div
          className={`sidebar-nav-item ${currentView === 'workspace' ? 'active' : ''}`}
          onClick={handleWorkspace}
          style={{ cursor: 'pointer' }}
          role="button"
        >
          <LayoutGrid className="icon" size={16} />
          <span>Workspace</span>
        </div>
      </div>

      {/* Pinned Section */}
      {(folders.some(f => f.isPinned) || filteredChats.some(c => c.isPinned)) && (
        <div className="sidebar-section" style={{ paddingBottom: 0 }}>
          <div className="sidebar-section-title">
            <span>Pinned</span>
            <Pin size={12} style={{ opacity: 0.7 }} />
          </div>
          <div style={{ paddingLeft: 4, marginTop: 4 }}>
            {folders.filter(f => f.isPinned).map(folder => (
              <div 
                key={`pinned-${folder.id}`}
                className={`sidebar-nav-item ${activeFolderId === folder.id ? 'active' : ''}`}
                onClick={() => setActiveFolderId(activeFolderId === folder.id ? null : folder.id)}
                style={{ cursor: 'pointer', padding: '4px 8px', minHeight: 28, marginBottom: 4 }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, flex: 1, overflow: 'hidden' }} onClick={(e) => { e.stopPropagation(); toggleFolder(folder.id); }}>
                  <FolderPlus size={14} style={{ flexShrink: 0, opacity: 0.7 }} />
                  <span style={{ fontSize: 12, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{folder.name}</span>
                </div>
                <div style={{ position: 'relative' }}>
                  <button
                    onClick={(e) => { e.stopPropagation(); setOpenFolderDropdownId(openFolderDropdownId === `pinned-${folder.id}` ? null : `pinned-${folder.id}`); }}
                    style={{ background: 'none', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer', padding: 2 }}
                  >
                    <MoreVertical size={14} style={{ opacity: 0.7 }} />
                  </button>
                  {renderFolderOptions(`pinned-${folder.id}`, folder.id, folder.name, folder.isPinned)}
                </div>
              </div>
            ))}
            
            {filteredChats.filter(c => c.isPinned).map((chat) => {
              const isActive = chat.id === activeChatId && currentView === 'chat';
              return (
                <div
                  key={`pinned-${chat.id}`}
                  className={`sidebar-nav-item ${isActive ? 'active' : ''}`}
                  onClick={() => selectChat(chat.id)}
                  style={{ justifyContent: 'space-between', cursor: 'pointer', minHeight: 28, marginBottom: 4 }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, overflow: 'hidden' }}>
                    <MessageSquare size={13} style={{ opacity: isActive ? 0.9 : 0.5, flexShrink: 0 }} />
                    <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 12 }}>
                      {chat.title}
                    </span>
                  </div>
                  <div style={{ position: 'relative' }}>
                    <button
                      onClick={(e) => { e.stopPropagation(); setOpenDropdownId(openDropdownId === `pinned-${chat.id}` ? null : `pinned-${chat.id}`); }}
                      style={{ background: 'none', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer', padding: 2 }}
                    >
                      <MoreVertical size={14} style={{ opacity: 0.7 }} />
                    </button>
                    {renderChatOptions(`pinned-${chat.id}`, chat.id, chat.title, chat.folderId, chat.isPinned)}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Folders Section */}
      <div className="sidebar-section" style={{ flex: 1, overflowY: 'auto' }}>
        <div className="sidebar-section-title">
          <span>Folders</span>
          <button
            onClick={handleNewFolder}
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
            title="Create New Folder"
          >
            <FolderPlus size={14} style={{ opacity: 0.7 }} />
          </button>
        </div>

        <div style={{ paddingLeft: 4, marginTop: 4 }}>
          {folders.map(folder => {
            const folderChats = filteredChats.filter(c => c.folderId === folder.id);
            return (
              <div key={folder.id} style={{ marginBottom: 4 }}>
                <div 
                  className={`sidebar-nav-item ${activeFolderId === folder.id ? 'active' : ''}`}
                  onClick={() => setActiveFolderId(activeFolderId === folder.id ? null : folder.id)}
                  style={{ cursor: 'pointer', padding: '4px 8px', minHeight: 28 }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6, flex: 1, overflow: 'hidden' }} onClick={(e) => { e.stopPropagation(); toggleFolder(folder.id); }}>
                    {folder.isOpen ? <ChevronDown size={14} style={{ flexShrink: 0 }} /> : <ChevronRight size={14} style={{ flexShrink: 0 }} />}
                    <span style={{ fontSize: 12, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{folder.name}</span>
                  </div>
                  
                  <div style={{ position: 'relative' }}>
                    <button
                      onClick={(e) => { e.stopPropagation(); setOpenFolderDropdownId(openFolderDropdownId === `normal-${folder.id}` ? null : `normal-${folder.id}`); }}
                      style={{ background: 'none', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer', padding: 2 }}
                    >
                      <MoreVertical size={14} style={{ opacity: 0.7 }} />
                    </button>
                    {renderFolderOptions(`normal-${folder.id}`, folder.id, folder.name, folder.isPinned)}
                  </div>
                </div>
                
                {folder.isOpen && folderChats.length > 0 && (
                  <div style={{ paddingLeft: 16, marginTop: 2 }}>
                    {folderChats.map((chat) => {
                      const isActive = chat.id === activeChatId && currentView === 'chat';
                      return (
                        <div
                          key={chat.id}
                          className={`sidebar-nav-item ${isActive ? 'active' : ''}`}
                          onClick={() => selectChat(chat.id)}
                          style={{ justifyContent: 'space-between', cursor: 'pointer', minHeight: 28 }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', gap: 8, overflow: 'hidden' }}>
                            <MessageSquare size={13} style={{ opacity: isActive ? 0.9 : 0.5, flexShrink: 0 }} />
                            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 12 }}>
                              {chat.title}
                            </span>
                          </div>
                          
                          <div style={{ position: 'relative' }}>
                            <button
                              onClick={(e) => { e.stopPropagation(); setOpenDropdownId(openDropdownId === `normal-${chat.id}` ? null : `normal-${chat.id}`); }}
                              style={{ background: 'none', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer', padding: 2 }}
                            >
                              <MoreVertical size={14} style={{ opacity: 0.7 }} />
                            </button>
                            {renderChatOptions(`normal-${chat.id}`, chat.id, chat.title, chat.folderId, chat.isPinned)}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="sidebar-section-title" style={{ marginTop: 16 }}>
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
          {filteredChats.filter(c => !c.folderId).map((chat) => {
            const isActive = chat.id === activeChatId && currentView === 'chat';
            return (
              <div
                key={chat.id}
                className={`sidebar-nav-item ${isActive ? 'active' : ''}`}
                onClick={() => selectChat(chat.id)}
                style={{ justifyContent: 'space-between', cursor: 'pointer', minHeight: 28 }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, overflow: 'hidden' }}>
                  <MessageSquare size={13} style={{ opacity: isActive ? 0.9 : 0.5, flexShrink: 0 }} />
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 12 }}>
                    {chat.title}
                  </span>
                </div>
                
                <div style={{ position: 'relative' }}>
                  <button
                    onClick={(e) => { e.stopPropagation(); setOpenDropdownId(openDropdownId === `normal-${chat.id}` ? null : `normal-${chat.id}`); }}
                    style={{ background: 'none', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer', padding: 2 }}
                  >
                    <MoreVertical size={14} style={{ opacity: 0.7 }} />
                  </button>
                  {renderChatOptions(`normal-${chat.id}`, chat.id, chat.title, chat.folderId, chat.isPinned)}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <Dialog
        isOpen={dialogState.isOpen}
        title={
          dialogState.type === 'new_folder' ? 'New Folder' :
          dialogState.type === 'rename_folder' ? 'Rename Folder' :
          dialogState.type === 'rename_chat' ? 'Rename Chat' :
          'Confirm Deletion'
        }
        message={dialogState.message}
        initialValue={dialogState.initialValue}
        isPrompt={dialogState.type !== 'delete_folder'}
        isDestructive={dialogState.type === 'delete_folder'}
        onConfirm={handleDialogConfirm}
        onCancel={closeDialog}
      />
    </aside>
  );
}
