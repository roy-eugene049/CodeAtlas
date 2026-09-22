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
    <div>
      <h1 className="mb-4 text-3xl font-semibold">Search</h1>
      <input
        className="mb-4 w-full max-w-xl rounded-lg border border-[var(--line)] bg-[var(--bg)] px-3 py-2"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="How does authentication work?"
      />
      <div className="mb-4 flex gap-2">
        {FILTERS.map((item) => (
          <button
            key={item}
            className={`rounded-full border px-3 py-1 text-xs capitalize ${
              kind === item ? "border-[var(--accent)] text-[var(--accent)]" : "border-[var(--line)]"
            }`}
            onClick={() => setKind(item)}
            type="button"
          >
            {item}
          </button>
        ))}
      </div>
      <div className="space-y-2">
        {hits.map((hit) => (
          <button
            key={hit.id}
            className="flex w-full items-center justify-between rounded-xl border border-[var(--line)] px-4 py-3 text-left"
            onClick={() =>
              void navigate({
                to: "/repositories/$id/files",
                params: { id },
                search: { symbol: hit.id },
              })
            }
            type="button"
          >
            <div>
              <strong>{hit.name}</strong>
              <div className="text-sm text-[var(--muted)]">{hit.path}</div>
            </div>
            <span className="text-sm text-[var(--accent)]">{Math.round(hit.score * 100)}% relevance</span>
          </button>
        ))}
      </div>
    </div>
  );
}
