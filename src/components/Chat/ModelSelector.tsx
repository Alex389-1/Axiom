import { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check, Cpu, RefreshCw } from 'lucide-react';
import { useAppStore, loadModels, ModelInfo } from '../../store';

export function ModelSelector() {
  const { models, selectedModel, setSelectedModel, setModels } = useAppStore();
  const [isOpen, setIsOpen] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const refreshList = async () => {
    setIsRefreshing(true);
    try {
      const fetched = await loadModels();
      if (fetched && fetched.length > 0) {
        setModels(fetched);
        if (!selectedModel || !fetched.some((m) => m.name === selectedModel)) {
          setSelectedModel(fetched[0].name);
        }
      }
    } catch (e) {
      console.error('Failed to refresh models:', e);
    } finally {
      setIsRefreshing(false);
    }
  };

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleToggle = () => {
    if (!isOpen) {
      refreshList();
    }
    setIsOpen((prev) => !prev);
  };

  const displayModels: ModelInfo[] =
    models.length > 0
      ? models
      : [{ name: 'qwen2.5-coder:14b', provider: 'ollama' }];

  return (
    <div className="model-selector-wrapper" ref={dropdownRef}>
      <button
        type="button"
        className={`model-selector-btn ${isOpen ? 'active' : ''}`}
        onClick={handleToggle}
        title="Select Local Model"
      >
        <span className="model-selector-text">{selectedModel || 'Select Model'}</span>
        <ChevronDown size={14} className={`model-selector-chevron ${isOpen ? 'open' : ''}`} />
      </button>

      {isOpen && (
        <div className="model-selector-menu">
          <div className="model-selector-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>Local Ollama Models</span>
            <button
              type="button"
              onClick={refreshList}
              title="Refresh Models"
              style={{ background: 'none', border: 'none', cursor: 'pointer', padding: '2px', color: 'inherit', display: 'flex', alignItems: 'center' }}
            >
              <RefreshCw size={12} className={isRefreshing ? 'spin' : ''} />
            </button>
          </div>
          <div className="model-selector-list">
            {displayModels.map((m) => {
              const isSelected = m.name === selectedModel;
              return (
                <button
                  key={m.name}
                  type="button"
                  className={`model-selector-item ${isSelected ? 'selected' : ''}`}
                  onClick={() => {
                    setSelectedModel(m.name);
                    setIsOpen(false);
                  }}
                >
                  <div className="model-item-info">
                    <Cpu size={13} className="model-item-icon" />
                    <span className="model-item-name">{m.name}</span>
                  </div>
                  {isSelected && <Check size={14} className="model-item-check" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
