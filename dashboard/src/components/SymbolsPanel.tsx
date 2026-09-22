import { useQuery } from "@tanstack/react-query";

import { getSymbols } from "../api";

export function SymbolsPanel({
  repoId,
  onSelect,
}: {
  repoId: string;
  onSelect: (id: string) => void;
}) {
  const symbols = useQuery({
    queryKey: ["symbols", repoId],
    queryFn: () => getSymbols(repoId),
  });

  if (symbols.isLoading) return <p className="muted">Loading symbols…</p>;
  if (symbols.error) return <p className="error">{symbols.error.message}</p>;

  return (
    <div className="list">
      {symbols.data?.map((symbol) => (
        <button className="symbol-row" key={symbol.id} onClick={() => onSelect(symbol.id)} type="button">
          <div>
            <strong>{symbol.name}</strong>
            <div className="muted">
              {symbol.path} · lines {symbol.startLine}–{symbol.endLine}
            </div>
          </div>
          <span className="badge">{symbol.kind}</span>
        </button>
      ))}
    </div>
  );
}
