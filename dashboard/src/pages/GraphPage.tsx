import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import { getExplain, getGraph } from "../api";
import { FlowGraph } from "../components/FlowGraph";
import { SymbolInspector } from "../components/SymbolInspector";

export function GraphPage() {
  const { id } = useParams({ from: "/repositories/$id/graph" });
  const { symbol } = useSearch({ from: "/repositories/$id/graph" });
  const navigate = useNavigate();
  const graph = useQuery({
    queryKey: ["graph", id, symbol],
    queryFn: () => getGraph(id, symbol),
  });
  const explain = useQuery({
    queryKey: ["explain", id, symbol],
    queryFn: () => getExplain(id, symbol!),
    enabled: Boolean(symbol),
  });
  const onSelect = (next: string) => {
    void navigate({ to: "/repositories/$id/graph", params: { id }, search: { symbol: next } });
  };
  const openSource = (next: string) => {
    void navigate({
      to: "/repositories/$id/files",
      params: { id },
      search: { symbol: next },
    });
  };

  return (
    <div>
      <header className="page-header">
        <h1>Graph</h1>
        <p>Imports, calls, and references. Click a node to inspect it.</p>
      </header>
      {graph.data ? (
        <FlowGraph
          nodes={graph.data.nodes}
          edges={graph.data.edges}
          selectedId={symbol}
          onSelect={onSelect}
        />
      ) : (
        <p className="text-[var(--muted)]">Loading graph…</p>
      )}
      {symbol && explain.data ? (
        <div className="panel mt-4 flex flex-wrap items-center justify-between gap-4">
          <div>
            <div style={{ fontSize: 22, fontWeight: 590, letterSpacing: "-0.02em" }}>
              {explain.data.symbol.name}
            </div>
            <div className="muted" style={{ marginTop: 4 }}>
              {explain.data.dependencies.length} dependencies · {explain.data.dependents.length}{" "}
              dependents
            </div>
          </div>
          <div className="flex gap-2">
            <button
              className="btn"
              onClick={() => document.getElementById("graph-inspector")?.scrollIntoView()}
              type="button"
            >
              Explain
            </button>
            <button className="btn btn-primary" onClick={() => openSource(symbol)} type="button">
              Open source
            </button>
          </div>
        </div>
      ) : null}
      {symbol ? (
        <div className="mt-6" id="graph-inspector">
          <SymbolInspector
            repoId={id}
            symbolId={symbol}
            onSelect={onSelect}
            onOpenSource={openSource}
          />
        </div>
      ) : null}
    </div>
  );
}
