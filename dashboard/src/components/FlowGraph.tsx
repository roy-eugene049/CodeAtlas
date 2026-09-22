import {
  Background,
  Controls,
  type Edge,
  type Node,
  ReactFlow,
} from "@xyflow/react";
import { useMemo, useState } from "react";

import type { GraphEdge, GraphNode } from "../types";

import "@xyflow/react/dist/style.css";

const EDGE_FILTERS = ["imports", "calls", "references", "dependencies"] as const;
const NODE_FILTERS = ["file", "symbol", "service", "component", "api", "database"] as const;

function nodeBucket(node: GraphNode): (typeof NODE_FILTERS)[number] {
  const path = node.path.toLowerCase();
  const kind = node.kind.toLowerCase();
  if (path.includes("/db/") || path.includes("/database/") || kind.includes("repository")) {
    return "database";
  }
  if (path.includes("/api/") || kind.includes("handler") || kind.includes("route")) {
    return "api";
  }
  if (kind === "component") return "component";
  if (kind === "class" || path.includes("/services/")) return "service";
  if (kind === "file") return "file";
  return "symbol";
}

function edgeMatches(kind: string, filters: string[]) {
  if (filters.includes(kind)) return true;
  if (filters.includes("dependencies") && (kind === "imports" || kind === "calls")) return true;
  return false;
}

export function FlowGraph({
  nodes,
  edges,
  selectedId,
  onSelect,
}: {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedId?: string;
  onSelect: (id: string) => void;
}) {
  const [edgeKinds, setEdgeKinds] = useState<string[]>(["imports", "calls"]);
  const [nodeKinds, setNodeKinds] = useState<string[]>([]);
  const visibleNodes = useMemo(
    () =>
      nodes.filter((node) => nodeKinds.length === 0 || nodeKinds.includes(nodeBucket(node))),
    [nodes, nodeKinds],
  );
  const visibleIds = useMemo(() => new Set(visibleNodes.map((node) => node.id)), [visibleNodes]);
  const flowNodes: Node[] = useMemo(
    () =>
      visibleNodes.map((node) => ({
        id: node.id,
        position: { x: node.x * 1.4 + 400, y: node.y + 200 },
        data: { label: `${node.name}\n${nodeBucket(node)}` },
        style: {
          border: node.id === selectedId ? "1px solid #7dd3c0" : "1px solid #232a34",
          background: "#12161c",
          color: "#e7edf5",
          fontSize: 12,
          width: 168,
        },
      })),
    [visibleNodes, selectedId],
  );
  const flowEdges: Edge[] = useMemo(
    () =>
      edges
        .filter(
          (edge) =>
            visibleIds.has(edge.source) &&
            visibleIds.has(edge.target) &&
            (edgeKinds.length === 0 || edgeMatches(edge.kind, edgeKinds)),
        )
        .map((edge) => ({
          id: `${edge.source}-${edge.target}-${edge.kind}`,
          source: edge.source,
          target: edge.target,
          label: edge.kind,
        })),
    [edges, edgeKinds, visibleIds],
  );

  return (
    <div>
      <div className="mb-3 flex flex-wrap gap-2">
        {EDGE_FILTERS.map((kind) => (
          <FilterChip
            key={kind}
            label={kind}
            active={edgeKinds.includes(kind)}
            onClick={() => toggle(setEdgeKinds, kind)}
          />
        ))}
        {NODE_FILTERS.map((kind) => (
          <FilterChip
            key={kind}
            label={kind}
            active={nodeKinds.includes(kind)}
            onClick={() => toggle(setNodeKinds, kind)}
          />
        ))}
      </div>
      <div className="h-[520px] overflow-hidden rounded-2xl border border-[var(--line)]">
        <ReactFlow
          nodes={flowNodes}
          edges={flowEdges}
          colorMode="dark"
          fitView
          onNodeClick={(_, node) => onSelect(node.id)}
          proOptions={{ hideAttribution: true }}
        >
          <Background color="#232a34" />
          <Controls />
        </ReactFlow>
      </div>
    </div>
  );
}

function FilterChip({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`rounded-full border px-3 py-1 text-xs capitalize ${
        active ? "border-[var(--accent)] text-[var(--accent)]" : "border-[var(--line)] text-[var(--muted)]"
      }`}
      onClick={onClick}
      type="button"
    >
      {label}
    </button>
  );
}

function toggle(set: (update: (current: string[]) => string[]) => void, value: string) {
  set((current) =>
    current.includes(value) ? current.filter((item) => item !== value) : [...current, value],
  );
}
