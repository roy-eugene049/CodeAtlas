import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { listRepositories } from "../api";
import { IndexForm } from "../components/IndexForm";

export function RepositoriesPage() {
  const repos = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  return (
    <div className="mx-auto max-w-2xl">
      <header className="page-header">
        <h1>Repositories</h1>
        <p>Add a local path or Git URL. Reindex uses the commit range.</p>
      </header>
      <section className="panel">
        <IndexForm />
      </section>
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
                {repo.defaultBranch} · {repo.commitSha.slice(0, 8)}
              </div>
            </span>
            <span className="muted">{repo.lineCount.toLocaleString()} lines</span>
          </Link>
        ))}
      </div>
    </div>
  );
}
