import { useState, useRef, useEffect } from 'react';

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

type LayoutMode = 'tree' | 'profiler';

interface NodeCardProps {
  node: TraceNode;
  onHover: (node: TraceNode | null) => void;
  onClick: (node: TraceNode) => void;
  isHovered: boolean;
  isSelected: boolean;
}

function NodeCard({ node, onHover, onClick, isHovered, isSelected }: NodeCardProps) {
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
  const ringClass = isSelected ? 'ring-2 ring-yellow-400' : (isHovered ? 'ring-2 ring-purple-500' : '');

  return (
    <div
      className={`border border-slate-700/50 rounded ${bgColor} ${ringClass} p-1.5 cursor-pointer`}
      onMouseEnter={() => onHover(node)}
      onMouseLeave={() => onHover(null)}
      onClick={() => onClick(node)}
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

        {/* Available items - NO TRUNCATION */}
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
                {i}:{item}
              </div>
            ))}
          </div>
        )}

        {/* Span */}
        {node.span && (
          <span className="text-[9px] text-gray-600">
            {node.span.start}-{node.span.end}
          </span>
        )}
      </div>
    </div>
  );
}

// Tree view with SVG connections, zoom, and pan
function TreeView({
  node,
  onHover,
  onClick,
  hoveredNode,
  selectedNode
}: {
  node: TraceNode;
  onHover: (node: TraceNode | null) => void;
  onClick: (node: TraceNode) => void;
  hoveredNode: TraceNode | null;
  selectedNode: TraceNode | null;
}) {
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);

  // Zoom with mouse wheel
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setZoom(z => Math.max(0.1, Math.min(3, z * delta)));
  };

  // Pan with drag
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 0) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPan({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Collect node positions for drawing lines
  const [nodePositions, setNodePositions] = useState<Map<TraceNode, { x: number; y: number; width: number; height: number }>>(new Map());

  useEffect(() => {
    if (contentRef.current) {
      const positions = new Map<TraceNode, { x: number; y: number; width: number; height: number }>();

      const collectPositions = (el: Element, n: TraceNode) => {
        const rect = el.getBoundingClientRect();
        const containerRect = contentRef.current!.getBoundingClientRect();
        positions.set(n, {
          x: (rect.left - containerRect.left) / zoom,
          y: (rect.top - containerRect.top) / zoom,
          width: rect.width / zoom,
          height: rect.height / zoom
        });
      };

      // Walk DOM to collect positions
      const walk = (el: Element, n: TraceNode) => {
        if (el.hasAttribute('data-node-id')) {
          collectPositions(el, n);
        }
        if (n.children) {
          const childContainer = el.querySelector('[data-children]');
          if (childContainer) {
            Array.from(childContainer.children).forEach((child, i) => {
              if (i < n.children.length) {
                walk(child, n.children[i]);
              }
            });
          }
        }
      };

      const rootEl = contentRef.current.querySelector('[data-node-id]');
      if (rootEl) {
        walk(rootEl, node);
      }

      setNodePositions(positions);
    }
  }, [node, zoom]);

  return (
    <div
      ref={containerRef}
      className="relative w-full h-full overflow-hidden"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      style={{ cursor: isDragging ? 'grabbing' : 'grab' }}
    >
      {/* Zoom controls */}
      <div className="absolute top-2 right-2 z-10 flex gap-1">
        <button
          onClick={() => setZoom(z => Math.min(3, z * 1.2))}
          className="bg-slate-700 hover:bg-slate-600 text-white px-2 py-1 rounded text-xs"
        >
          +
        </button>
        <button
          onClick={() => setZoom(z => Math.max(0.1, z / 1.2))}
          className="bg-slate-700 hover:bg-slate-600 text-white px-2 py-1 rounded text-xs"
        >
          −
        </button>
        <button
          onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}
          className="bg-slate-700 hover:bg-slate-600 text-white px-2 py-1 rounded text-xs"
        >
          Reset
        </button>
      </div>

      <div
        ref={contentRef}
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: '0 0',
          transition: isDragging ? 'none' : 'transform 0.1s'
        }}
        className="p-4"
      >
        {/* SVG for connection lines */}
        <svg
          className="absolute top-0 left-0 pointer-events-none"
          style={{ width: '100%', height: '100%', overflow: 'visible' }}
        >
          {Array.from(nodePositions.entries()).map(([n, pos]) =>
            n.children?.map((child, i) => {
              const childPos = nodePositions.get(child);
              if (!childPos) return null;

              const startX = pos.x + pos.width / 2;
              const startY = pos.y + pos.height;
              const endX = childPos.x + childPos.width / 2;
              const endY = childPos.y;

              return (
                <g key={`${n.operation}-${i}`}>
                  <line
                    x1={startX}
                    y1={startY}
                    x2={endX}
                    y2={endY}
                    stroke="#64748b"
                    strokeWidth="1.5"
                    markerEnd="url(#arrowhead)"
                  />
                </g>
              );
            })
          )}
          <defs>
            <marker
              id="arrowhead"
              markerWidth="10"
              markerHeight="10"
              refX="5"
              refY="5"
              orient="auto"
            >
              <polygon points="0 0, 10 5, 0 10" fill="#64748b" />
            </marker>
          </defs>
        </svg>

        <TreeNode
          node={node}
          onHover={onHover}
          onClick={onClick}
          hoveredNode={hoveredNode}
          selectedNode={selectedNode}
        />
      </div>
    </div>
  );
}

function TreeNode({
  node,
  onHover,
  onClick,
  hoveredNode,
  selectedNode
}: {
  node: TraceNode;
  onHover: (node: TraceNode | null) => void;
  onClick: (node: TraceNode) => void;
  hoveredNode: TraceNode | null;
  selectedNode: TraceNode | null;
}) {
  const [isExpanded, setIsExpanded] = useState(true);
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div className="flex flex-col items-center gap-2" data-node-id>
      {/* Current node */}
      <div className="flex flex-col items-center">
        <NodeCard
          node={node}
          onHover={onHover}
          onClick={onClick}
          isHovered={hoveredNode === node}
          isSelected={selectedNode === node}
        />
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
        <div className="flex gap-3 items-start flex-wrap justify-center" data-children>
          {node.children.map((child, i) => (
            <TreeNode
              key={i}
              node={child}
              onHover={onHover}
              onClick={onClick}
              hoveredNode={hoveredNode}
              selectedNode={selectedNode}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// Profiler view: rows with depth via padding
function ProfilerRow({
  node,
  depth,
  onHover,
  onClick,
  hoveredNode,
  selectedNode
}: {
  node: TraceNode;
  depth: number;
  onHover: (node: TraceNode | null) => void;
  onClick: (node: TraceNode) => void;
  hoveredNode: TraceNode | null;
  selectedNode: TraceNode | null;
}) {
  const [isExpanded, setIsExpanded] = useState(true);
  const hasChildren = node.children && node.children.length > 0;
  const isHovered = hoveredNode === node;
  const isSelected = selectedNode === node;

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
  const ringClass = isSelected ? 'ring-2 ring-yellow-400' : (isHovered ? 'ring-2 ring-purple-500' : '');
  const depthPadding = `${depth * 1.5}rem`;

  return (
    <>
      <div
        className={`border-b border-slate-700/50 ${bgColor} ${ringClass} cursor-pointer`}
        onMouseEnter={() => onHover(node)}
        onMouseLeave={() => onHover(null)}
        onClick={() => onClick(node)}
        style={{ paddingLeft: depthPadding }}
      >
        <div className="px-2 py-1 flex items-center gap-2 text-xs">
          {hasChildren && (
            <button
              onClick={(e) => { e.stopPropagation(); setIsExpanded(!isExpanded); }}
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
                  {i}:{item}
                </div>
              ))}
            </div>
          )}

          {node.span && (
            <span className="text-[9px] text-gray-600">
              {node.span.start}-{node.span.end}
            </span>
          )}
        </div>
      </div>

      {isExpanded && hasChildren && node.children.map((child, i) => (
        <ProfilerRow
          key={i}
          node={child}
          depth={depth + 1}
          onHover={onHover}
          onClick={onClick}
          hoveredNode={hoveredNode}
          selectedNode={selectedNode}
        />
      ))}
    </>
  );
}

function SourceDisplay({ node }: { node: TraceNode | null }) {
  if (!node) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500 text-sm">
        Hover or click a trace node to see its source
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
      return <pre className="text-xs text-gray-300 font-mono whitespace-pre-wrap">{template}</pre>;
    }

    const before = template.substring(0, span.start);
    const highlighted = template.substring(span.start, span.end);
    const after = template.substring(span.end);

    return (
      <pre className="text-xs font-mono leading-relaxed whitespace-pre-wrap">
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
      <div className="px-3 py-1.5 bg-slate-800/50 border-b border-slate-700 flex items-center justify-between flex-shrink-0">
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
  const [selectedNode, setSelectedNode] = useState<TraceNode | null>(null);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('tree');

  const handleNodeClick = (node: TraceNode) => {
    setSelectedNode(selectedNode === node ? null : node);
  };

  const displayNode = selectedNode || hoveredNode;

  const countNodes = (node: TraceNode): number => {
    return 1 + (node.children?.reduce((acc, child) => acc + countNodes(child), 0) || 0);
  };

  const totalNodes = countNodes(trace);

  const layoutModeLabels: Record<LayoutMode, string> = {
    tree: 'Tree (horizontal)',
    profiler: 'Profiler (vertical)'
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-2">
      {/* Fixed size container */}
      <div className="bg-slate-900 rounded-lg shadow-2xl border border-slate-700 flex flex-col" style={{ width: '95vw', height: '95vh' }}>
        {/* Header - fixed height */}
        <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-3 py-2 rounded-t-lg flex items-center justify-between flex-shrink-0">
          <div>
            <h2 className="text-base font-bold text-white">Execution Trace</h2>
            <p className="text-purple-200 text-[10px]">
              {totalNodes} operations • {layoutModeLabels[layoutMode]} • {selectedNode ? 'Locked' : 'Hover mode'}
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

        {/* Main content: Trace (left) + Source (right) - fixed size */}
        <div className="flex-1 flex overflow-hidden min-h-0">
          {/* Trace tree (60% width) */}
          <div className="w-[60%] border-r border-slate-700 flex flex-col min-h-0">
            <div className="px-2 py-1 bg-slate-800/30 border-b border-slate-700 text-[10px] text-gray-400 flex-shrink-0">
              <span className="font-semibold">Trace Tree</span> • Click to expand/collapse • Hover/click to see source
            </div>
            <div className="flex-1 overflow-auto scrollbar-hide min-h-0">
              {layoutMode === 'tree' && (
                <TreeView
                  node={trace}
                  onHover={setHoveredNode}
                  onClick={handleNodeClick}
                  hoveredNode={hoveredNode}
                  selectedNode={selectedNode}
                />
              )}
              {layoutMode === 'profiler' && (
                <ProfilerRow
                  node={trace}
                  depth={0}
                  onHover={setHoveredNode}
                  onClick={handleNodeClick}
                  hoveredNode={hoveredNode}
                  selectedNode={selectedNode}
                />
              )}
            </div>
          </div>

          {/* Source display (40% width) */}
          <div className="w-[40%] flex flex-col min-h-0">
            <div className="px-2 py-1 bg-slate-800/30 border-b border-slate-700 text-[10px] text-gray-400 flex-shrink-0">
              <span className="font-semibold">Source Code</span> • Highlighted section shows span
            </div>
            <div className="flex-1 overflow-hidden min-h-0">
              <SourceDisplay node={displayNode} />
            </div>
          </div>
        </div>

        {/* Footer - fixed height */}
        <div className="px-2 py-1.5 bg-slate-800/50 border-t border-slate-700 flex justify-between items-center text-[10px] flex-shrink-0">
          <div className="text-gray-400 flex gap-2">
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-purple-600"></span>
              Hover
            </span>
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-yellow-400"></span>
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

      {/* Global styles for hiding scrollbar backgrounds */}
      <style>{`
        .scrollbar-hide::-webkit-scrollbar {
          width: 8px;
          height: 8px;
        }
        .scrollbar-hide::-webkit-scrollbar-track {
          background: transparent;
        }
        .scrollbar-hide::-webkit-scrollbar-thumb {
          background: #475569;
          border-radius: 4px;
        }
        .scrollbar-hide::-webkit-scrollbar-thumb:hover {
          background: #64748b;
        }
        .scrollbar-hide {
          scrollbar-width: thin;
          scrollbar-color: #475569 transparent;
        }
      `}</style>
    </div>
  );
}
