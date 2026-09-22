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
          placeholder="Search symbols, then open the file"
        />
        {results.data?.map((symbol) => (
          <button
            className="symbol-row"
            key={symbol.id}
            onClick={() => {
              onSelect(symbol.id);
              onClose();
            }}
            style={{ border: 0, borderRadius: 0, width: "100%" }}
            type="button"
          >
            <div>
              <strong>{symbol.name}</strong>
              <div className="muted">{symbol.path}</div>
            </div>
            <span className="badge">
              {Math.round(symbol.score * 100)}% · {symbol.kind}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
