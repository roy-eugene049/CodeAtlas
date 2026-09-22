import type { GraphEdge, GraphNode } from "../types";

interface Props {
  nodes: GraphNode[];
  edges: GraphEdge[];
  focusId?: string;
  selectedId?: string;
  onSelect?: (id: string) => void;
}

const NODE_W = 168;
const NODE_H = 52;

export function GraphCanvas({ nodes, edges, focusId, selectedId, onSelect }: Props) {
  if (nodes.length === 0) {
    return <p className="empty">Select a symbol to query its neighborhood.</p>;
  }

  const xs = nodes.map((node) => node.x);
  const ys = nodes.map((node) => node.y);
  const minX = Math.min(...xs) - NODE_W;
  const maxX = Math.max(...xs) + NODE_W;
  const minY = Math.min(...ys) - NODE_H;
  const maxY = Math.max(...ys) + NODE_H;
  const byId = new Map(nodes.map((node) => [node.id, node]));

  return (
    <div className="graph-canvas">
      <svg
        viewBox={`${minX} ${minY} ${Math.max(maxX - minX, 320)} ${Math.max(maxY - minY, 220)}`}
        role="img"
        aria-label="Symbol graph"
      >
        <defs>
          <marker
            id="graph-arrow"
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="#5f7288" />
          </marker>
        </defs>
        {edges.map((edge) => {
          const from = byId.get(edge.source);
          const to = byId.get(edge.target);
          if (!from || !to) return null;
          const midY = (from.y + to.y) / 2;
          return (
            <path
              key={`${edge.source}-${edge.target}-${edge.kind}`}
              className="graph-edge"
              d={`M ${from.x} ${from.y} C ${from.x} ${midY}, ${to.x} ${midY}, ${to.x} ${to.y}`}
              markerEnd="url(#graph-arrow)"
            >
              <title>{edge.kind}</title>
            </path>
          );
        })}
        {nodes.map((node) => {
          const selected = node.id === selectedId || node.id === focusId;
          return (
            <g
              key={node.id}
              className={`graph-node-group${selected ? " is-selected" : ""}`}
              transform={`translate(${node.x - NODE_W / 2}, ${node.y - NODE_H / 2})`}
              onClick={() => onSelect?.(node.id)}
              role="button"
              tabIndex={0}
            >
              <rect width={NODE_W} height={NODE_H} rx="10" />
              <text x="14" y="22" className="graph-node-name">
                {truncate(node.name, 18)}
              </text>
              <text x="14" y="38" className="graph-node-meta">
                {node.kind}
              </text>
              <title>
                {node.name} · {node.path}
              </title>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function truncate(value: string, max: number) {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}
