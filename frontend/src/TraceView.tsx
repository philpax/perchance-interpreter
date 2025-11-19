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
  source_template?: string;
  generator_name?: string;
  inline_list_content?: string;
}

interface TraceViewProps {
  trace: TraceNode;
  onClose: () => void;
}

interface TraceNodeRowProps {
  node: TraceNode;
  depth: number;
  onHover: (node: TraceNode | null) => void;
  hoveredNode: TraceNode | null;
}

function TraceNodeRow({ node, depth, onHover, hoveredNode }: TraceNodeRowProps) {
  const [expandedInline, setExpandedInline] = useState(false);
  const [isExpanded, setIsExpanded] = useState(true);

  const hasChildren = node.children && node.children.length > 0;
  const isHovered = hoveredNode === node;

  // Color based on operation type
  const getTypeBgColor = (type?: string) => {
    switch (type) {
      case 'root': return 'bg-purple-900/40';
      case 'listselect': return 'bg-blue-900/40';
      case 'import': return 'bg-green-900/40';
      case 'range': return 'bg-yellow-900/40';
      case 'choice': return 'bg-pink-900/40';
      case 'methodcall': return 'bg-cyan-900/40';
      default: return 'bg-slate-800/40';
    }
  };

  const bgColor = getTypeBgColor(node.operation_type);
  const depthPadding = `${depth * 1.5}rem`;

  return (
    <>
      {/* Main row */}
      <div
        className={`border-b border-slate-700/50 ${bgColor} ${isHovered ? 'ring-2 ring-purple-500' : ''}`}
        onMouseEnter={() => onHover(node)}
        onMouseLeave={() => onHover(null)}
        style={{ paddingLeft: depthPadding }}
      >
        <div className="px-2 py-1 flex items-center gap-2 text-xs">
          {/* Expand button */}
          {hasChildren && (
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-gray-500 hover:text-gray-300 w-4 text-center"
            >
              {isExpanded ? '▼' : '▶'}
            </button>
          )}
          {!hasChildren && <span className="w-4"></span>}

          {/* Operation */}
          <code className="text-blue-300 font-semibold min-w-[120px]">
            {node.operation}
          </code>

          {/* Type badge */}
          {node.operation_type && (
            <span className="text-[9px] px-1 py-0.5 bg-slate-900/50 rounded text-gray-500">
              {node.operation_type}
            </span>
          )}

          {/* Available items - horizontal scroll */}
          {node.available_items && node.available_items.length > 0 && (
            <div className="flex gap-1 overflow-x-auto max-w-md">
              {node.available_items.map((item, i) => (
                <div
                  key={i}
                  className={`flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] ${
                    i === node.selected_index
                      ? 'bg-purple-600/50 border border-purple-500 text-white font-bold'
                      : 'bg-slate-700/30 border border-slate-600/30 text-gray-500'
                  }`}
                  title={item}
                >
                  {i}:{item.length > 20 ? item.substring(0, 20) + '...' : item}
                </div>
              ))}
            </div>
          )}

          {/* Inline list expand button */}
          {node.inline_list_content && (
            <button
              onClick={() => setExpandedInline(!expandedInline)}
              className="text-[10px] px-1.5 py-0.5 bg-slate-700/50 hover:bg-slate-600/50 rounded text-gray-400"
              title="Expand inline list"
            >
              {expandedInline ? '▼ {...}' : '▶ {...}'}
            </button>
          )}

          {/* Arrow and result */}
          <span className="text-gray-600 text-[10px]">→</span>
          <div className="text-gray-200 bg-slate-900/50 px-2 py-0.5 rounded font-mono text-[11px] flex-1">
            {node.result || <span className="text-gray-600 italic">(empty)</span>}
          </div>

          {/* Span info */}
          {node.span && (
            <span className="text-[9px] text-gray-600">
              {node.span.start}-{node.span.end}
            </span>
          )}
        </div>

        {/* Expanded inline list content */}
        {expandedInline && node.inline_list_content && (
          <div className="px-2 pb-1 ml-8 text-[10px] text-gray-400 bg-slate-900/30 rounded mx-2 mb-1 p-1">
            {node.inline_list_content}
          </div>
        )}
      </div>

      {/* Children - rendered as rows below, not indented */}
      {isExpanded && hasChildren && node.children.map((child, i) => (
        <TraceNodeRow
          key={i}
          node={child}
          depth={depth + 1}
          onHover={onHover}
          hoveredNode={hoveredNode}
        />
      ))}
    </>
  );
}

function SourceDisplay({ node }: { node: TraceNode | null }) {
  if (!node) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500 text-sm">
        Hover over a trace node to see its source
      </div>
    );
  }

  // Find the appropriate source template (walk up to find root or import)
  const getSource = (n: TraceNode): { template: string; name: string } | null => {
    if (n.source_template) {
      return {
        template: n.source_template,
        name: n.generator_name || 'unknown'
      };
    }
    return null;
  };

  const sourceInfo = getSource(node);

  if (!sourceInfo) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500 text-sm">
        No source available for this node
      </div>
    );
  }

  // Highlight the relevant part based on span
  const renderHighlightedSource = () => {
    const { template } = sourceInfo;
    const { span } = node;

    if (!span || span.start === span.end) {
      return <pre className="text-xs text-gray-300 font-mono">{template}</pre>;
    }

    const before = template.substring(0, span.start);
    const highlighted = template.substring(span.start, span.end);
    const after = template.substring(span.end);

    return (
      <pre className="text-xs font-mono leading-relaxed">
        <span className="text-gray-400">{before}</span>
        <span className="bg-yellow-500/30 text-yellow-200 font-bold px-0.5">
          {highlighted}
        </span>
        <span className="text-gray-400">{after}</span>
      </pre>
    );
  };

  return (
    <div className="h-full flex flex-col">
      <div className="px-3 py-1.5 bg-slate-800/50 border-b border-slate-700 flex items-center justify-between">
        <span className="text-xs font-semibold text-purple-300">
          {sourceInfo.name}
        </span>
        {node.span && (
          <span className="text-[10px] text-gray-500">
            Lines {node.span.start}-{node.span.end}
          </span>
        )}
      </div>
      <div className="flex-1 overflow-auto p-3 bg-slate-900/50">
        {renderHighlightedSource()}
      </div>
    </div>
  );
}

export default function TraceView({ trace, onClose }: TraceViewProps) {
  const [hoveredNode, setHoveredNode] = useState<TraceNode | null>(null);

  // Count nodes for stats
  const countNodes = (node: TraceNode): number => {
    return 1 + (node.children?.reduce((acc, child) => acc + countNodes(child), 0) || 0);
  };

  const totalNodes = countNodes(trace);

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-2">
      <div className="bg-slate-900 rounded-lg shadow-2xl border border-slate-700 w-full max-w-[95vw] max-h-[95vh] flex flex-col">
        {/* Header */}
        <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-3 py-2 rounded-t-lg flex items-center justify-between">
          <div>
            <h2 className="text-base font-bold text-white">Execution Trace</h2>
            <p className="text-purple-200 text-[10px]">
              {totalNodes} operations • Vertical profiler view
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-white hover:text-gray-200 transition-colors text-xl leading-none px-1"
            aria-label="Close"
          >
            ×
          </button>
        </div>

        {/* Main content: Trace (left) + Source (right) */}
        <div className="flex-1 flex overflow-hidden">
          {/* Trace tree (60% width) */}
          <div className="w-[60%] border-r border-slate-700 flex flex-col">
            <div className="px-2 py-1 bg-slate-800/30 border-b border-slate-700 text-[10px] text-gray-400">
              <span className="font-semibold">Trace Tree</span> • Click to expand/collapse • Hover to see source
            </div>
            <div className="flex-1 overflow-y-auto text-sm">
              <TraceNodeRow
                node={trace}
                depth={0}
                onHover={setHoveredNode}
                hoveredNode={hoveredNode}
              />
            </div>
          </div>

          {/* Source display (40% width) */}
          <div className="w-[40%] flex flex-col">
            <div className="px-2 py-1 bg-slate-800/30 border-b border-slate-700 text-[10px] text-gray-400">
              <span className="font-semibold">Source Code</span> • Highlighted section shows span
            </div>
            <div className="flex-1 overflow-hidden">
              <SourceDisplay node={hoveredNode} />
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-2 py-1.5 bg-slate-800/50 border-t border-slate-700 flex justify-between items-center text-[10px]">
          <div className="text-gray-400 flex gap-2">
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-purple-600"></span>
              Selected
            </span>
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-slate-600"></span>
              Options
            </span>
          </div>
          <button
            onClick={onClose}
            className="px-2 py-1 bg-purple-600 hover:bg-purple-700 text-white text-[11px] rounded transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
