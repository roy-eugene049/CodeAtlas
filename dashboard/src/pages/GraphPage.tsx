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
      <h1 className="mb-4 text-3xl font-semibold">Graph explorer</h1>
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
        <div className="mt-4 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-[var(--line)] bg-[var(--raised)] px-5 py-4">
          <div>
            <div className="text-lg font-medium">{explain.data.symbol.name}</div>
            <div className="text-sm text-[var(--muted)]">
              Dependencies: {explain.data.dependencies.length} · Dependents:{" "}
              {explain.data.dependents.length}
            </div>
          </div>
          <div className="flex gap-2">
            <button
              className="rounded-lg border border-[var(--line)] px-3 py-1.5 text-sm"
              onClick={() => document.getElementById("graph-inspector")?.scrollIntoView()}
              type="button"
            >
              Explain
            </button>
            <button
              className="rounded-lg border border-[var(--line)] px-3 py-1.5 text-sm"
              onClick={() => document.getElementById("graph-inspector")?.scrollIntoView()}
              type="button"
            >
              Impact
            </button>
            <button
              className="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-sm text-[#08211c]"
              onClick={() => openSource(symbol)}
              type="button"
            >
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
