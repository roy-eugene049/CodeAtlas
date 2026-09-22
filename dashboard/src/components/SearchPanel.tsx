import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { searchSymbols } from "../api";

export function SearchPanel({
  repoId,
  onSelect,
}: {
  repoId: string;
  onSelect: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const results = useQuery({
    queryKey: ["search", repoId, query],
    queryFn: () => searchSymbols(repoId, query),
    enabled: query.trim().length > 0,
  });

  return (
    <>
      <form className="index-form" onSubmit={(event) => event.preventDefault()}>
        <input
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Find a symbol"
        />
      </form>
      <div className="list" style={{ marginTop: 16 }}>
        {results.data?.map((symbol) => (
          <button className="symbol-row" key={symbol.id} onClick={() => onSelect(symbol.id)} type="button">
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
    </>
  );
}
