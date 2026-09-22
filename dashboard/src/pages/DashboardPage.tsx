import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { listRepositories } from "../api";
import { IndexForm } from "../components/IndexForm";

const LOOP = [
  "Index a Git repository or local path",
  "Explore files and symbols",
  "Read the dependency graph",
  "Search the codebase",
  "Ask a question and open the cited source",
];

export function DashboardPage() {
  const repos = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });

  return (
    <div className="mx-auto max-w-2xl">
      <header className="page-header">
        <h1>CodeAtlas</h1>
        <p>Index a repository, then explore it as a system.</p>
      </header>
      <ol className="grouped mb-8">
        {LOOP.map((step, index) => (
          <li key={step} className="grouped-row">
            <span>{step}</span>
            <span className="muted">{index + 1}</span>
          </li>
        ))}
      </ol>
      <p className="section-label">Index a repository</p>
      <section className="panel">
        <IndexForm />
      </section>
      {(repos.data ?? []).length > 0 ? (
        <>
          <p className="section-label">Repositories</p>
          <div className="grouped">
            {(repos.data ?? []).map((repo) => (
              <Link
                key={repo.id}
                to="/repositories/$id/overview"
                params={{ id: repo.id }}
                className="grouped-row"
              >
                <span>
                  <strong>{repo.name}</strong>
                  <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
                    {repo.url}
                  </div>
                </span>
                <span className="muted">{repo.lineCount.toLocaleString()}</span>
              </Link>
            ))}
          </div>
        </>
      ) : null}
    </div>
  );
}
