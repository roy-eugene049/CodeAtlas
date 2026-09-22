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
    <div className="mx-auto max-w-3xl space-y-8">
      <div>
        <h1 className="text-3xl font-semibold">CodeAtlas</h1>
        <p className="mt-2 text-[var(--muted)]">
          Index a repository, then explore it as a system: files, symbols, graph, search, and
          answers that cite source locations.
        </p>
      </div>
      <ol className="grid gap-2 text-sm text-[var(--muted)] sm:grid-cols-1">
        {LOOP.map((step, index) => (
          <li key={step} className="flex gap-3">
            <span className="text-[var(--accent)]">{index + 1}.</span>
            <span>{step}</span>
          </li>
        ))}
      </ol>
      <section className="rounded-2xl border border-[var(--line)] bg-[var(--raised)] p-5">
        <h2 className="mb-3 text-lg font-medium">Index a repository</h2>
        <IndexForm />
      </section>
      {(repos.data ?? []).length > 0 ? (
        <section className="space-y-3">
          <h2 className="text-lg font-medium">Repositories</h2>
          <div className="grid gap-3">
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
                  {repo.defaultBranch} · {repo.commitSha.slice(0, 8)} ·{" "}
                  {repo.lineCount.toLocaleString()} lines
                </p>
              </Link>
            ))}
          </div>
        </section>
      ) : (
        <p className="text-sm text-[var(--muted)]">
          After indexing, open Overview, Files, Graph, Search, and Ask.
        </p>
      )}
    </div>
  );
}
