import { useQuery } from "@tanstack/react-query";

import { getEvolution } from "../api";

export function EvolutionPanel({ repoId }: { repoId: string }) {
  const evolution = useQuery({
    queryKey: ["evolution", repoId],
    queryFn: () => getEvolution(repoId),
  });

  if (evolution.isLoading) return <p className="muted">Reading git history…</p>;
  if (evolution.error) return <p className="error">{evolution.error.message}</p>;
  const data = evolution.data;
  if (!data || (data.commits.length === 0 && data.hotspots.length === 0)) {
    return <p className="empty">No git history was collected for this repository.</p>;
  }

  return (
    <div className="evolution">
      <p className="muted">
        Evolution is counted from git log, not guessed. Secrets and .env paths are dropped.
      </p>
      <section className="stats health-stats">
        <article className="stat">
          <span className="muted">Commits</span>
          <strong>{data.commits.length}</strong>
        </article>
        <article className="stat">
          <span className="muted">Branches</span>
          <strong>{data.branches.length}</strong>
        </article>
        <article className="stat">
          <span className="muted">Authors</span>
          <strong>{data.authors.length}</strong>
        </article>
        <article className="stat">
          <span className="muted">Hotspots</span>
          <strong>{data.hotspots.length}</strong>
        </article>
      </section>

      <section className="panel">
        <h2>Which files change most frequently?</h2>
        <div className="list">
          {data.hotspots.slice(0, 20).map((hotspot) => (
            <article className="symbol-row" key={hotspot.path}>
              <div>
                <strong>{hotspot.path}</strong>
                <div className="muted">
                  Changed {hotspot.changeCount} times · {hotspot.contributorCount} contributors
                </div>
              </div>
              <span className="badge">{hotspot.changeCount}</span>
            </article>
          ))}
        </div>
      </section>

      <div className="evolution-grid">
        <section className="panel">
          <h3>Authors</h3>
          <div className="list">
            {data.authors.map((author) => (
              <article className="symbol-row" key={author.email}>
                <div>
                  <strong>{author.name}</strong>
                  <div className="muted">{author.email}</div>
                </div>
                <span className="badge">{author.commitCount}</span>
              </article>
            ))}
          </div>
        </section>
        <section className="panel">
          <h3>Branches</h3>
          <div className="list">
            {data.branches.map((branch) => (
              <article className="symbol-row" key={branch.name}>
                <div>
                  <strong>
                    {branch.name}
                    {branch.isDefault ? " · default" : ""}
                  </strong>
                  <div className="muted">{branch.sha.slice(0, 12)}</div>
                </div>
              </article>
            ))}
          </div>
        </section>
      </div>

      <section className="panel">
        <h3>Commits</h3>
        <div className="list">
          {data.commits.slice(0, 20).map((commit) => (
            <article className="symbol-row" key={commit.sha}>
              <div>
                <strong>{commit.subject}</strong>
                <div className="muted">
                  {commit.sha.slice(0, 8)} · {commit.authorName} · {commit.filesChanged} files
                </div>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
