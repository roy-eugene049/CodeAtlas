import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { searchSymbols } from "../api";
import { SymbolInspector } from "./SymbolInspector";

interface Props {
  repoId: string;
  selectedSymbolId?: string;
  onSelect: (id: string) => void;
}

export function InsightsPanel({ repoId, selectedSymbolId, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const hits = useQuery({
    queryKey: ["search", repoId, query],
    queryFn: () => searchSymbols(repoId, query),
    enabled: query.trim().length > 0,
  });

  return (
    <div>
      <p className="muted">
        Select a symbol. Explain reads the unit. Dependencies and impact walk the graph — then AI
        narrates the counts.
      </p>
      <form className="index-form" onSubmit={(event) => event.preventDefault()}>
        <input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="handlePayment, UserService.updateUser"
        />
      </form>
      {hits.data && hits.data.length > 0 ? (
        <div className="list" style={{ marginTop: 12 }}>
          {hits.data.slice(0, 10).map((symbol) => (
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

      {!selectedSymbolId ? <p className="empty">Pick a symbol to explain it.</p> : null}
      {selectedSymbolId ? (
        <div style={{ marginTop: 24 }}>
          <SymbolInspector
            key={selectedSymbolId}
            repoId={repoId}
            symbolId={selectedSymbolId}
            onSelect={onSelect}
            initialTab="explain"
          />
        </div>
      ) : null}
    </div>
  );
}
