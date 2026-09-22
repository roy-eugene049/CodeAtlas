import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { searchSymbols } from "../api";

export function CommandPalette({
  repoId,
  onClose,
  onSelect,
}: {
  repoId: string;
  onClose: () => void;
  onSelect: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const results = useQuery({
    queryKey: ["search", repoId, query],
    queryFn: () => searchSymbols(repoId, query),
    enabled: query.trim().length > 0,
  });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="overlay" onClick={onClose} role="presentation">
      <div className="palette" onClick={(event) => event.stopPropagation()} role="dialog">
        <input
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search"
        />
        {results.data?.map((symbol) => (
          <button
            className="grouped-row"
            key={symbol.id}
            onClick={() => {
              onSelect(symbol.id);
              onClose();
            }}
            type="button"
          >
            <span>
              <strong>{symbol.name}</strong>
              <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
                {symbol.path}
              </div>
            </span>
            <span className="muted">
              {Math.round(symbol.score * 100)}% · {symbol.kind}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
