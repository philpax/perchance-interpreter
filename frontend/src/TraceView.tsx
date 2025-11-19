import { useState } from 'react';

export interface TraceNode {
  operation: string;
  result: string;
  span?: { start: number; end: number };
  rng_seed?: number;
  children: TraceNode[];
  operation_type?: string;
  available_items?: string[];
  selected_index?: number;
  interpolation_context?: string;
}

interface TraceViewProps {
  trace: TraceNode;
  onClose: () => void;
}

interface TraceNodeComponentProps {
  node: TraceNode;
  isRoot?: boolean;
}

function TraceNodeComponent({ node, isRoot = false }: TraceNodeComponentProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  const hasChildren = node.children && node.children.length > 0;

  // Color based on operation type
  const getTypeBgColor = (type?: string) => {
    switch (type) {
      case 'root':
        return 'bg-purple-900/30 border-purple-600/40';
      case 'listselect':
        return 'bg-blue-900/30 border-blue-600/40';
      case 'import':
        return 'bg-green-900/30 border-green-600/40';
      case 'range':
        return 'bg-yellow-900/30 border-yellow-600/40';
      case 'choice':
        return 'bg-pink-900/30 border-pink-600/40';
      case 'methodcall':
        return 'bg-cyan-900/30 border-cyan-600/40';
      default:
        return 'bg-slate-800/50 border-slate-600/40';
    }
  };

  const getTypeTextColor = (type?: string) => {
    switch (type) {
      case 'root':
        return 'text-purple-300';
      case 'listselect':
        return 'text-blue-300';
      case 'import':
        return 'text-green-300';
      case 'range':
        return 'text-yellow-300';
      case 'choice':
        return 'text-pink-300';
      case 'methodcall':
        return 'text-cyan-300';
      default:
        return 'text-gray-300';
    }
  };

  const bgColor = getTypeBgColor(node.operation_type);
  const textColor = getTypeTextColor(node.operation_type);

  return (
    <div className={isRoot ? '' : 'ml-6 border-l-2 border-slate-700/50 pl-3'}>
      <div className={`${bgColor} border rounded px-2 py-1 mb-1`}>
        {/* Header row */}
        <div className="flex items-center gap-2">
          {hasChildren && (
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-xs text-gray-500 hover:text-gray-300 flex-shrink-0"
            >
              {isExpanded ? '▼' : '▶'}
            </button>
          )}
          <code className={`font-mono text-xs ${textColor} font-semibold`}>
            {node.operation}
          </code>
          {node.operation_type && (
            <span className="text-[10px] px-1.5 py-0.5 bg-slate-900/50 rounded text-gray-500">
              {node.operation_type}
            </span>
          )}
        </div>

        {/* Available items (horizontal scrollable) */}
        {node.available_items && node.available_items.length > 0 && (
          <div className="mt-1">
            <div className="flex gap-1 overflow-x-auto pb-1 scrollbar-thin scrollbar-thumb-slate-600 scrollbar-track-slate-800">
              {node.available_items.map((item, i) => (
                <div
                  key={i}
                  className={`flex-shrink-0 px-2 py-0.5 rounded text-xs ${
                    i === node.selected_index
                      ? 'bg-purple-600/40 border border-purple-500/60 text-white font-semibold'
                      : 'bg-slate-700/50 border border-slate-600/40 text-gray-400'
                  }`}
                  title={item}
                >
                  <span className="text-[10px] text-gray-500 mr-1">{i}:</span>
                  {item.length > 30 ? item.substring(0, 30) + '...' : item}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Result */}
        <div className="mt-1 flex items-start gap-1">
          <span className="text-[10px] text-gray-600 flex-shrink-0 mt-0.5">→</span>
          <div className="text-xs text-gray-100 bg-slate-900/70 px-2 py-0.5 rounded flex-1 min-w-0 break-words font-mono">
            {node.result || <span className="text-gray-600 italic">(empty)</span>}
          </div>
        </div>

        {/* Interpolation context */}
        {node.interpolation_context && (
          <div className="mt-1 text-[10px] text-gray-500 italic">
            Context: {node.interpolation_context}
          </div>
        )}

        {/* Span info */}
        {node.span && (
          <div className="mt-0.5 text-[10px] text-gray-600">
            pos: {node.span.start}-{node.span.end}
          </div>
        )}
      </div>

      {/* Children */}
      {hasChildren && isExpanded && (
        <div className="mt-1">
          {node.children.map((child, i) => (
            <TraceNodeComponent key={i} node={child} />
          ))}
        </div>
      )}
    </div>
  );
}

export default function TraceView({ trace, onClose }: TraceViewProps) {
  // Count nodes for stats
  const countNodes = (node: TraceNode): number => {
    return 1 + (node.children?.reduce((acc, child) => acc + countNodes(child), 0) || 0);
  };

  const totalNodes = countNodes(trace);

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-2">
      <div className="bg-slate-900 rounded-lg shadow-2xl border border-slate-700 w-full max-w-6xl max-h-[95vh] flex flex-col">
        {/* Header */}
        <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-2 rounded-t-lg flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-white">Execution Trace</h2>
            <p className="text-purple-200 text-xs">
              {totalNodes} operation{totalNodes !== 1 ? 's' : ''}
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-white hover:text-gray-200 transition-colors text-2xl leading-none px-1"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        {/* Trace Tree */}
        <div className="p-3 overflow-y-auto flex-1 text-sm">
          <TraceNodeComponent node={trace} isRoot={true} />
        </div>

        {/* Footer */}
        <div className="px-3 py-2 bg-slate-800/50 border-t border-slate-700 rounded-b-lg flex justify-between items-center text-xs">
          <div className="text-gray-400 flex gap-3">
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-purple-600"></span>
              Selected item
            </span>
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-slate-600"></span>
              Available options
            </span>
          </div>
          <button
            onClick={onClose}
            className="px-3 py-1 bg-purple-600 hover:bg-purple-700 text-white rounded transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
