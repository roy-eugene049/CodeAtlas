import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useState } from "react";

import { searchSymbols } from "../api";

const FILTERS = ["all", "files", "symbols", "function", "component", "api"] as const;

function matchesFilter(
  hit: { kind: string; path: string; source: string; name: string },
  filter: (typeof FILTERS)[number],
) {
  if (filter === "all" || filter === "symbols") return true;
  if (filter === "files") return hit.source === "fileName" || hit.source === "path";
  if (filter === "api") {
    return hit.path.includes("/api/") || hit.name.toLowerCase().includes("handler");
  }
  return hit.kind === filter;
}

export function SearchPage() {
  const { id } = useParams({ from: "/repositories/$id/search" });
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<(typeof FILTERS)[number]>("all");
  const results = useQuery({
    queryKey: ["search", id, query],
    queryFn: () => searchSymbols(id, query),
    enabled: query.trim().length > 0,
  });
  const hits = (results.data ?? []).filter((hit) => matchesFilter(hit, kind));

  return (
    <div className="mx-auto max-w-2xl">
      <header className="page-header">
        <h1>Search</h1>
        <p>Symbols ranked by name, path, and retrieved meaning.</p>
      </header>
      <input
        className="field"
        style={{ fontSize: 20, padding: "14px 16px", marginBottom: 16 }}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="How does authentication work?"
      />
      <div className="segmented" style={{ marginBottom: 20 }}>
        {FILTERS.map((item) => (
          <button
            key={item}
            className={kind === item ? "active" : ""}
            onClick={() => setKind(item)}
            type="button"
          >
            {item}
          </button>
        ))}
      </div>
      <div className="grouped">
        {hits.map((hit) => (
          <button
            key={hit.id}
            className="grouped-row"
            onClick={() =>
              void navigate({
                to: "/repositories/$id/files",
                params: { id },
                search: { symbol: hit.id },
              })
            }
            type="button"
          >
            <span>
              <strong>{hit.name}</strong>
              <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
                {hit.path}
              </div>
            </span>
            <span className="muted">{Math.round(hit.score * 100)}%</span>
          </button>
        ))}
      </div>
    </div>
  );
}
