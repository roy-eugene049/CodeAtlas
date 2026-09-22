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
      <input
        className="field"
        style={{ marginBottom: 16 }}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="handlePayment, UserService.updateUser"
      />
      {hits.data && hits.data.length > 0 ? (
        <div className="grouped" style={{ marginBottom: 24 }}>
          {hits.data.slice(0, 10).map((symbol) => (
            <button
              key={symbol.id}
              className="grouped-row"
              onClick={() => {
                onSelect(symbol.id);
                setQuery("");
              }}
              type="button"
            >
              <span>
                <strong>{symbol.name}</strong>
                <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
                  {symbol.path}
                </div>
              </span>
              <span className="muted">{symbol.kind}</span>
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
