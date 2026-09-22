import type { OverviewResponse } from "../types";

const LAYER_LABEL: Record<string, string> = {
  api: "Controllers",
  services: "Services",
  repositories: "Repositories",
  database: "Database",
  workers: "Queue",
  components: "Components",
  hooks: "Hooks",
  pages: "Pages",
  external: "External APIs",
  other: "Other",
};

export function OverviewPanel({ overview }: { overview?: OverviewResponse }) {
  if (!overview) {
    return <p className="empty">Index a repository to see structure, symbols, and architecture.</p>;
  }

  const health = overview.health;
  const stats = [
    ["Files", health.fileCount],
    ["Lines", health.lineCount],
    ["Languages", health.languageCount],
    ["Symbols", health.symbolCount],
    ["Dependencies", health.dependencyCount],
    ["API endpoints", health.apiEndpointCount],
  ] as const;

  return (
    <>
      <section className="panel health-panel">
        <h2>Codebase health</h2>
        <p className="muted">Counts come from the index. Bars are ratios, not estimated scores.</p>
        <section className="stats health-stats">
          {stats.map(([label, value]) => (
            <article className="stat" key={label}>
              <span className="muted">{label}</span>
              <strong>{Number(value ?? 0).toLocaleString()}</strong>
            </article>
          ))}
        </section>
        <div className="health-meters">
          <Meter
            label="Architecture"
            value={health.architectureMapped}
            detail={`${Math.round(health.architectureMapped * 100)}% of files mapped to a known layer`}
          />
          <Meter
            label="Complexity"
            value={health.complexity}
            detail="Mean call fan-out and symbol span among functions"
          />
          <Meter
            label="Test presence"
            value={health.testPresence}
            detail={`${health.testFileCount} test files / ${health.fileCount} files · ${health.testSymbolCount} test symbols`}
          />
        </div>
      </section>
      {overview.languages.length > 0 ? (
        <>
          <p className="section-label">Languages</p>
          <div className="grouped" style={{ marginBottom: 16 }}>
            {overview.languages
              .slice()
              .sort((a, b) => (b.percent ?? 0) - (a.percent ?? 0))
              .map((item) => (
                <div className="grouped-row" key={item.language}>
                  <span className="capitalize">{item.language}</span>
                  <span className="muted">{Math.round((item.percent ?? 0) * 100)}%</span>
                </div>
              ))}
          </div>
        </>
      ) : null}
      <section className="panel">
        <h2>Architecture</h2>
        <p className="muted">{overview.architectureTree.summary}</p>
        <div className="architecture-tree">
          {overview.architectureTree.groups.map((group) => (
            <article className="arch-group" key={group.group}>
              <div className="arch-group-head">
                <strong>{group.name}</strong>
                <span className="muted">{group.fileCount} files</span>
              </div>
              <ul>
                {group.layers.map((layer) => (
                  <li key={layer.kind}>
                    <span>{LAYER_LABEL[layer.kind] ?? layer.kind}</span>
                    <span className="muted">{layer.fileCount}</span>
                  </li>
                ))}
              </ul>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}

function Meter({ label, value, detail }: { label: string; value: number; detail: string }) {
  const pct = Math.round(Math.min(1, Math.max(0, value)) * 100);
  return (
    <div className="meter">
      <div className="meter-head">
        <strong>{label}</strong>
        <span className="muted">{pct}%</span>
      </div>
      <div className="meter-track">
        <div className="meter-fill" style={{ width: `${pct}%` }} />
      </div>
      <p className="muted">{detail}</p>
    </div>
  );
}
