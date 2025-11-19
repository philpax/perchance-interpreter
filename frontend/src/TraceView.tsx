import { useState } from 'react';

export interface TraceNode {
  operation: string;
  result: string;
  span?: { start: number; end: number };
  rng_seed?: number;
  children: TraceNode[];
  operation_type?: string;
}

interface TraceViewProps {
  trace: TraceNode;
  onClose: () => void;
}

interface TraceNodeComponentProps {
  node: TraceNode;
  depth: number;
}

function TraceNodeComponent({ node, depth }: TraceNodeComponentProps) {
  const [isExpanded, setIsExpanded] = useState(depth < 2); // Auto-expand first 2 levels

  const hasChildren = node.children && node.children.length > 0;
  const indentClass = depth > 0 ? 'ml-4 pl-4 border-l-2 border-slate-600' : '';

  // Color based on operation type
  const getTypeColor = (type?: string) => {
    switch (type) {
      case 'root':
        return 'text-purple-400';
      case 'listselect':
        return 'text-blue-400';
      case 'import':
        return 'text-green-400';
      case 'range':
        return 'text-yellow-400';
      case 'choice':
        return 'text-pink-400';
      case 'methodcall':
        return 'text-cyan-400';
      default:
        return 'text-gray-400';
    }
  };

  const typeColor = getTypeColor(node.operation_type);

  return (
    <div className={`${indentClass} mb-2`}>
      <div
        className={`bg-slate-800/50 rounded-lg p-3 transition-all ${
          hasChildren ? 'cursor-pointer hover:bg-slate-800' : ''
        }`}
        onClick={() => hasChildren && setIsExpanded(!isExpanded)}
      >
        <div className="flex items-start gap-2">
          {hasChildren && (
            <span className="text-gray-500 select-none flex-shrink-0 mt-0.5">
              {isExpanded ? '▼' : '▶'}
            </span>
          )}
          <div className="flex-1 min-w-0">
            <div className="flex items-start gap-3 flex-wrap">
              <code className={`font-mono text-sm ${typeColor} font-semibold`}>
                {node.operation}
              </code>
              {node.operation_type && (
                <span className="text-xs px-2 py-0.5 bg-slate-700 rounded text-gray-400">
                  {node.operation_type}
                </span>
              )}
            </div>
            <div className="mt-2 flex items-start gap-2">
              <span className="text-xs text-gray-500 flex-shrink-0">→</span>
              <div className="text-sm text-gray-200 bg-slate-900/50 px-3 py-1.5 rounded flex-1 min-w-0 break-words">
                {node.result || <span className="text-gray-600 italic">(empty)</span>}
              </div>
            </div>
            {node.span && (
              <div className="mt-1 text-xs text-gray-600">
                Position: {node.span.start}-{node.span.end}
              </div>
            )}
          </div>
        </div>
      </div>

      {hasChildren && isExpanded && (
        <div className="mt-2 space-y-1">
          {node.children.map((child, i) => (
            <TraceNodeComponent key={i} node={child} depth={depth + 1} />
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
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="bg-slate-900 rounded-lg shadow-2xl border border-slate-700 w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-6 py-4 rounded-t-lg flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-white">Execution Trace</h2>
            <p className="text-purple-200 text-sm mt-1">
              {totalNodes} operation{totalNodes !== 1 ? 's' : ''} traced
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-white hover:text-gray-200 transition-colors text-3xl leading-none px-2"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        {/* Legend */}
        <div className="px-6 py-3 bg-slate-800/30 border-b border-slate-700">
          <div className="flex flex-wrap gap-3 text-xs">
            <div className="flex items-center gap-1">
              <span className="w-3 h-3 rounded-full bg-purple-400"></span>
              <span className="text-gray-400">Root</span>
            </div>
            <div className="flex items-center gap-1">
              <span className="w-3 h-3 rounded-full bg-blue-400"></span>
              <span className="text-gray-400">List Select</span>
            </div>
            <div className="flex items-center gap-1">
              <span className="w-3 h-3 rounded-full bg-green-400"></span>
              <span className="text-gray-400">Import</span>
            </div>
            <div className="flex items-center gap-1">
              <span className="w-3 h-3 rounded-full bg-cyan-400"></span>
              <span className="text-gray-400">Method Call</span>
            </div>
          </div>
        </div>

        {/* Trace Tree */}
        <div className="p-6 overflow-y-auto flex-1">
          <TraceNodeComponent node={trace} depth={0} />
        </div>

        {/* Footer */}
        <div className="px-6 py-3 bg-slate-800/30 border-t border-slate-700 rounded-b-lg flex justify-between items-center">
          <div className="text-sm text-gray-400">
            Click nodes to expand/collapse • Scroll to see more
          </div>
          <button
            onClick={onClose}
            className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
