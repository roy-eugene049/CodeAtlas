import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { listRepositories } from "../api";
import { IndexForm } from "../components/IndexForm";

export function RepositoriesPage() {
  const repos = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  return (
    <div>
      <h1 className="mb-4 text-3xl font-semibold">Repositories</h1>
      <section className="mb-6 max-w-3xl">
        <IndexForm />
      </section>
      <div className="grid gap-3 md:grid-cols-2">
        {(repos.data ?? []).map((repo) => (
          <Link
            key={repo.id}
            to="/repositories/$id/overview"
            params={{ id: repo.id }}
            className="rounded-2xl border border-[var(--line)] bg-[var(--raised)] p-5 hover:border-[var(--accent)]"
          >
            <strong>{repo.name}</strong>
            <p className="mt-1 truncate text-sm text-[var(--muted)]">{repo.url}</p>
            <p className="mt-2 text-xs text-[var(--muted)]">
              {repo.defaultBranch} · {repo.commitSha.slice(0, 8)} · {repo.lineCount.toLocaleString()} lines
            </p>
          </Link>
        ))}
      </div>
    </div>
  );
}
