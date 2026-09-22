import type { ImpactResponse, ImpactSymbol } from "../types";

export function ImpactReportView({
  impact,
  onSelect,
}: {
  impact: ImpactResponse;
  onSelect?: (id: string) => void;
}) {
  const buckets = [
    ["Direct dependents", impact.direct],
    ["Indirect dependents", impact.indirect],
    ["Tests", impact.tests],
    ["Routes", impact.routes],
    ["UI components", impact.components],
  ] as const;

  return (
    <section className="impact">
      <header className="impact-hero">
        <div>
          <h2>
            {impact.origin.name}
            {impact.origin.kind === "method" ? "()" : ""}
          </h2>
          <p className="muted">{impact.origin.path}</p>
        </div>
        <article className="stat">
          <span className="muted">Potential impact</span>
          <strong>{impact.symbolCount}</strong>
          <span className="muted">
            symbols · {impact.fileCount} files · {impact.routes.length} APIs ·{" "}
            {impact.components.length} UI · {impact.testFileCount} tests
          </span>
        </article>
      </header>

      <div className="impact-buckets">
        {buckets.map(([label, items]) => (
          <article className="panel" key={label}>
            <h3>
              {label} <span className="muted">{items.length}</span>
            </h3>
            <div className="grouped">
              {items.length === 0 ? <p className="muted" style={{ padding: 12 }}>None found</p> : null}
              {items.map((item) => (
                <SymbolHit key={item.id} symbol={item} onSelect={onSelect} />
              ))}
            </div>
          </article>
        ))}
      </div>

      <article className="panel">
        <h3>Impacted files</h3>
        <ul className="impact-tree">
          <li>
            <span>{impact.tree.path}</span>
            {impact.tree.children.length > 0 ? (
              <ul>
                {impact.tree.children.map((child) => (
                  <li key={child.fileId}>
                    {child.path}
                    {child.roles.length > 0 ? (
                      <span className="badge">{child.roles.join(" · ")}</span>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : null}
          </li>
        </ul>
      </article>
    </section>
  );
}

function SymbolHit({
  symbol,
  onSelect,
}: {
  symbol: ImpactSymbol;
  onSelect?: (id: string) => void;
}) {
  return (
    <button className="grouped-row" onClick={() => onSelect?.(symbol.id)} type="button">
      <span>
        <strong>{symbol.name}</strong>
        <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
          {symbol.path}
        </div>
      </span>
      <span className="muted">{symbol.kind}</span>
    </button>
  );
}
