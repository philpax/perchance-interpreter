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

type LayoutMode = 'tree' | 'profiler' | 'nested';

interface NodeCardProps {
  node: TraceNode;
  onHover: (node: TraceNode | null) => void;
  hoveredNode: TraceNode | null;
}

function NodeCard({ node, onHover, hoveredNode }: NodeCardProps) {
  const [expandedInline, setExpandedInline] = useState(false);
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

  return (
    <div
      className={`border border-slate-700/50 rounded ${bgColor} ${isHovered ? 'ring-2 ring-purple-500' : ''} p-1.5`}
      onMouseEnter={() => onHover(node)}
      onMouseLeave={() => onHover(null)}
    >
      <div className="flex items-center gap-1.5 text-xs flex-wrap">
        {/* Result FIRST */}
        <div className="text-gray-200 bg-slate-900/50 px-2 py-0.5 rounded font-mono text-[11px]">
          {node.result || <span className="text-gray-600 italic">(empty)</span>}
        </div>

        {/* Arrow */}
        <span className="text-gray-600 text-[10px]">←</span>

        {/* Operation */}
        <code className="text-blue-300 font-semibold text-[11px]">
          {node.operation}
        </code>

        {/* Type badge */}
        {node.operation_type && (
          <span className="text-[9px] px-1 py-0.5 bg-slate-900/50 rounded text-gray-500">
            {node.operation_type}
          </span>
        )}

        {/* Available items */}
        {node.available_items && node.available_items.length > 0 && (
          <div className="flex gap-1 overflow-x-auto max-w-[200px]">
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

        {/* Inline list button */}
        {node.inline_list_content && (
          <button
            onClick={() => setExpandedInline(!expandedInline)}
            className="text-[10px] px-1.5 py-0.5 bg-slate-700/50 hover:bg-slate-600/50 rounded text-gray-400"
          >
            {expandedInline ? '▼ {...}' : '▶ {...}'}
          </button>
        )}

        {/* Span */}
        {node.span && (
          <span className="text-[9px] text-gray-600">
            {node.span.start}-{node.span.end}
          </span>
        )}
      </div>

      {/* Expanded inline list content */}
      {expandedInline && node.inline_list_content && (
        <div className="mt-1 text-[10px] text-gray-400 bg-slate-900/30 rounded p-1">
          {node.inline_list_content}
        </div>
      )}
    </div>
  );
}

// Tree view: nodes at same level side-by-side
function TreeView({ node, onHover, hoveredNode }: { node: TraceNode; onHover: (node: TraceNode | null) => void; hoveredNode: TraceNode | null }) {
  const [isExpanded, setIsExpanded] = useState(true);
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div className="flex flex-col items-center gap-2">
      {/* Current node */}
      <div className="flex flex-col items-center">
        <NodeCard node={node} onHover={onHover} hoveredNode={hoveredNode} />
        {hasChildren && (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="text-gray-500 hover:text-gray-300 text-[10px] mt-0.5"
          >
            {isExpanded ? '▼' : '▶'}
          </button>
        )}
      </div>

      {/* Children side-by-side */}
      {isExpanded && hasChildren && (
        <div className="flex gap-3 items-start flex-wrap justify-center">
          {node.children.map((child, i) => (
            <TreeView key={i} node={child} onHover={onHover} hoveredNode={hoveredNode} />
          ))}
        </div>
      )}
    </div>
  );
}

// Profiler view: rows with depth via padding
function ProfilerRow({ node, depth, onHover, hoveredNode }: { node: TraceNode; depth: number; onHover: (node: TraceNode | null) => void; hoveredNode: TraceNode | null }) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [expandedInline, setExpandedInline] = useState(false);
  const hasChildren = node.children && node.children.length > 0;
  const isHovered = hoveredNode === node;

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
      <div
        className={`border-b border-slate-700/50 ${bgColor} ${isHovered ? 'ring-2 ring-purple-500' : ''}`}
        onMouseEnter={() => onHover(node)}
        onMouseLeave={() => onHover(null)}
        style={{ paddingLeft: depthPadding }}
      >
        <div className="px-2 py-1 flex items-center gap-2 text-xs">
          {hasChildren && (
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-gray-500 hover:text-gray-300 w-4 text-center"
            >
              {isExpanded ? '▼' : '▶'}
            </button>
          )}
          {!hasChildren && <span className="w-4"></span>}

          <div className="text-gray-200 bg-slate-900/50 px-2 py-0.5 rounded font-mono text-[11px]">
            {node.result || <span className="text-gray-600 italic">(empty)</span>}
          </div>

          <span className="text-gray-600 text-[10px]">←</span>

          <code className="text-blue-300 font-semibold">
            {node.operation}
          </code>

          {node.operation_type && (
            <span className="text-[9px] px-1 py-0.5 bg-slate-900/50 rounded text-gray-500">
              {node.operation_type}
            </span>
          )}

          {node.available_items && node.available_items.length > 0 && (
            <div className="flex gap-1 overflow-x-auto">
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

          {node.inline_list_content && (
            <button
              onClick={() => setExpandedInline(!expandedInline)}
              className="text-[10px] px-1.5 py-0.5 bg-slate-700/50 hover:bg-slate-600/50 rounded text-gray-400"
            >
              {expandedInline ? '▼ {...}' : '▶ {...}'}
            </button>
          )}

          {node.span && (
            <span className="text-[9px] text-gray-600">
              {node.span.start}-{node.span.end}
            </span>
          )}
        </div>

        {expandedInline && node.inline_list_content && (
          <div className="px-2 pb-1 ml-8 text-[10px] text-gray-400 bg-slate-900/30 rounded mx-2 mb-1 p-1">
            {node.inline_list_content}
          </div>
        )}
      </div>

      {isExpanded && hasChildren && node.children.map((child, i) => (
        <ProfilerRow
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

// Nested view: traditional depth-grows-right tree
function NestedView({ node, depth, onHover, hoveredNode }: { node: TraceNode; depth: number; onHover: (node: TraceNode | null) => void; hoveredNode: TraceNode | null }) {
  const [isExpanded, setIsExpanded] = useState(true);
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div className="flex flex-col gap-1" style={{ marginLeft: depth > 0 ? '2rem' : 0 }}>
      <div className="flex items-start gap-2">
        {hasChildren && (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="text-gray-500 hover:text-gray-300 text-xs mt-1"
          >
            {isExpanded ? '▼' : '▶'}
          </button>
        )}
        {!hasChildren && <span className="w-4"></span>}
        <div className="flex-1">
          <NodeCard node={node} onHover={onHover} hoveredNode={hoveredNode} />
        </div>
      </div>

      {isExpanded && hasChildren && (
        <div className="flex flex-col gap-1">
          {node.children.map((child, i) => (
            <NestedView
              key={i}
              node={child}
              depth={depth + 1}
              onHover={onHover}
              hoveredNode={hoveredNode}
            />
          ))}
        </div>
      )}
    </div>
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
            {node.span.start}-{node.span.end}
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
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('tree');

  const countNodes = (node: TraceNode): number => {
    return 1 + (node.children?.reduce((acc, child) => acc + countNodes(child), 0) || 0);
  };

  const totalNodes = countNodes(trace);

  const layoutModeLabels: Record<LayoutMode, string> = {
    tree: 'Tree (horizontal)',
    profiler: 'Profiler (vertical)',
    nested: 'Nested (right)'
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-2">
      <div className="bg-slate-900 rounded-lg shadow-2xl border border-slate-700 w-full max-w-[95vw] max-h-[95vh] flex flex-col">
        {/* Header */}
        <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-3 py-2 rounded-t-lg flex items-center justify-between">
          <div>
            <h2 className="text-base font-bold text-white">Execution Trace</h2>
            <p className="text-purple-200 text-[10px]">
              {totalNodes} operations • {layoutModeLabels[layoutMode]}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {/* Layout mode switcher */}
            <select
              value={layoutMode}
              onChange={(e) => setLayoutMode(e.target.value as LayoutMode)}
              className="bg-purple-700 text-white text-xs px-2 py-1 rounded border border-purple-500"
            >
              <option value="tree">Tree View</option>
              <option value="profiler">Profiler View</option>
              <option value="nested">Nested View</option>
            </select>
            <button
              onClick={onClose}
              className="text-white hover:text-gray-200 transition-colors text-xl leading-none px-1"
              aria-label="Close"
            >
              ×
            </button>
          </div>
        </div>

        {/* Main content: Trace (left) + Source (right) */}
        <div className="flex-1 flex overflow-hidden">
          {/* Trace tree (60% width) */}
          <div className="w-[60%] border-r border-slate-700 flex flex-col">
            <div className="px-2 py-1 bg-slate-800/30 border-b border-slate-700 text-[10px] text-gray-400">
              <span className="font-semibold">Trace Tree</span> • Click to expand/collapse • Hover to see source
            </div>
            <div className="flex-1 overflow-auto text-sm p-2">
              {layoutMode === 'tree' && (
                <TreeView node={trace} onHover={setHoveredNode} hoveredNode={hoveredNode} />
              )}
              {layoutMode === 'profiler' && (
                <ProfilerRow node={trace} depth={0} onHover={setHoveredNode} hoveredNode={hoveredNode} />
              )}
              {layoutMode === 'nested' && (
                <NestedView node={trace} depth={0} onHover={setHoveredNode} hoveredNode={hoveredNode} />
              )}
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
