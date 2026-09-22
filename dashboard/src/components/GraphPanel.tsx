import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { getGraph, searchSymbols } from "../api";
import { GraphCanvas } from "./GraphCanvas";
import { SymbolInspector } from "./SymbolInspector";

interface Props {
  repoId: string;
  selectedSymbolId?: string;
  onSelect: (id: string) => void;
}

export function GraphPanel({ repoId, selectedSymbolId, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const graph = useQuery({
    queryKey: ["graph", repoId, selectedSymbolId],
    queryFn: () => getGraph(repoId, selectedSymbolId),
  });
  const hits = useQuery({
    queryKey: ["search", repoId, query],
    queryFn: () => searchSymbols(repoId, query),
    enabled: query.trim().length > 0,
  });

  if (graph.isLoading) return <p className="muted">Loading graph…</p>;
  if (graph.error) return <p className="error">{graph.error.message}</p>;

  return (
    <div className="graph-workstation">
      <form className="index-form" onSubmit={(event) => event.preventDefault()}>
        <input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Query the graph — handlePayment, login, useAuth"
        />
      </form>
      {hits.data && hits.data.length > 0 ? (
        <div className="list" style={{ margin: "12px 0 20px" }}>
          {hits.data.slice(0, 8).map((symbol) => (
            <button
              key={symbol.id}
              className="symbol-row"
              onClick={() => {
                onSelect(symbol.id);
                setQuery("");
              }}
              type="button"
            >
              <div>
                <strong>{symbol.name}</strong>
                <div className="muted">{symbol.path}</div>
              </div>
              <span className="badge">{symbol.kind}</span>
            </button>
          ))}
        </div>
      ) : null}

      <GraphCanvas
        nodes={graph.data?.nodes ?? []}
        edges={graph.data?.edges ?? []}
        focusId={graph.data?.focus ?? selectedSymbolId}
        selectedId={selectedSymbolId}
        onSelect={onSelect}
      />

      {selectedSymbolId ? (
        <div style={{ marginTop: 28 }}>
          <SymbolInspector
            key={selectedSymbolId}
            repoId={repoId}
            symbolId={selectedSymbolId}
            onSelect={onSelect}
          />
        </div>
      ) : (
        <p className="muted" style={{ marginTop: 20 }}>
          Click a function. Explain, dependencies, and impact are all graph-backed.
        </p>
      )}
    </div>
  );
}
