import { useEffect, useRef } from 'react';

interface AutocompleteDropdownProps {
  items: string[];
  position: { top: number; left: number };
  selectedIndex: number;
  onSelect: (item: string) => void;
  onClose: () => void;
  onNavigate: (direction: 'up' | 'down') => void;
}

export default function AutocompleteDropdown({
  items,
  position,
  selectedIndex,
  onSelect,
  onClose,
  onNavigate,
}: AutocompleteDropdownProps) {
  const dropdownRef = useRef<HTMLDivElement>(null);
  const selectedItemRef = useRef<HTMLDivElement>(null);

  // Scroll selected item into view
  useEffect(() => {
    if (selectedItemRef.current) {
      selectedItemRef.current.scrollIntoView({
        block: 'nearest',
        behavior: 'smooth',
      });
    }
  }, [selectedIndex]);

  // Handle keyboard events
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        onNavigate('down');
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        onNavigate('up');
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (items[selectedIndex]) {
          onSelect(items[selectedIndex]);
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [items, selectedIndex, onSelect, onClose, onNavigate]);

  if (items.length === 0) return null;

  return (
    <div
      ref={dropdownRef}
      className="absolute z-50 bg-slate-800 border border-purple-500/50 rounded-lg shadow-2xl overflow-hidden"
      style={{
        top: `${position.top}px`,
        left: `${position.left}px`,
        maxHeight: '300px',
        width: '280px',
      }}
    >
      <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-3 py-2 flex items-center justify-between">
        <span className="text-xs font-semibold text-white">
          Import Generator ({items.length})
        </span>
        <span className="text-xs text-purple-200">
          ↑↓ navigate • ⏎ select • esc close
        </span>
      </div>
      <div className="overflow-y-auto max-h-[250px]">
        {items.map((item, index) => (
          <div
            key={item}
            ref={index === selectedIndex ? selectedItemRef : null}
            onClick={() => onSelect(item)}
            onMouseEnter={() => onNavigate('down')}
            className={`px-4 py-2 cursor-pointer transition-colors ${
              index === selectedIndex
                ? 'bg-purple-600 text-white'
                : 'text-gray-300 hover:bg-slate-700'
            }`}
          >
            <div className="font-mono text-sm">{item}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
