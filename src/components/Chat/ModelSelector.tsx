import { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check, Cpu } from 'lucide-react';
import { useAppStore, ModelInfo } from '../../store';

export function ModelSelector() {
  const { models, selectedModel, setSelectedModel } = useAppStore();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

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

  const displayModels: ModelInfo[] =
    models.length > 0
      ? models
      : [{ name: 'qwen2.5-coder:14b', provider: 'ollama' }];

  return (
    <div className="model-selector-wrapper" ref={dropdownRef}>
      <button
        type="button"
        className={`model-selector-btn ${isOpen ? 'active' : ''}`}
        onClick={() => setIsOpen((prev) => !prev)}
        title="Select Local Model"
      >
        <span className="model-selector-text">{selectedModel || 'Select Model'}</span>
        <ChevronDown size={14} className={`model-selector-chevron ${isOpen ? 'open' : ''}`} />
      </button>

      {isOpen && (
        <div className="model-selector-menu">
          <div className="model-selector-header">Local Ollama Models</div>
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
