import { useQuery } from "@tanstack/react-query";
import { Link, Outlet, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { listRepositories } from "../api";
import { CommandPalette } from "../components/CommandPalette";

const NAV = [
  { to: "/dashboard", label: "Dashboard" },
  { to: "/repositories", label: "Repositories" },
] as const;

const REPO_NAV = [
  { to: "/repositories/$id/overview", label: "Overview" },
  { to: "/repositories/$id/files", label: "Files" },
  { to: "/repositories/$id/graph", label: "Graph" },
  { to: "/repositories/$id/search", label: "Search" },
  { to: "/repositories/$id/ai", label: "AI" },
  { to: "/repositories/$id/insights", label: "Insights" },
] as const;

export function AppShell() {
  const params = useParams({ strict: false }) as { id?: string };
  const navigate = useNavigate();
  const repoId = params.id;
  const [paletteOpen, setPaletteOpen] = useState(false);
  const repos = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--text,#e7edf5)]">
      <header className="flex items-center justify-between border-b border-[var(--line)] px-5 py-3">
        <Link to="/dashboard" className="text-sm font-semibold tracking-[0.18em]">
          CODEATLAS
          <span className="ml-2 text-[11px] tracking-normal text-[var(--muted)]">
            code intelligence
          </span>
        </Link>
        <button
          className="rounded-lg border border-[var(--line)] px-3 py-1.5 text-sm text-[var(--muted)]"
          onClick={() => setPaletteOpen(true)}
          type="button"
        >
          Search codebase <span className="ml-2 text-xs">⌘K</span>
        </button>
      </header>
      <div className="grid min-h-[calc(100vh-56px)] grid-cols-[220px_1fr]">
        <aside className="flex flex-col gap-1 border-r border-[var(--line)] p-3">
          {NAV.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className="rounded-lg px-3 py-2 text-sm text-[var(--muted)] hover:bg-[#1a2029]"
              activeProps={{ className: "rounded-lg px-3 py-2 text-sm bg-[#1a2029] text-white" }}
              activeOptions={{ exact: item.to !== "/repositories" }}
            >
              {item.label}
            </Link>
          ))}
          {repoId
            ? REPO_NAV.map((item) => (
                <Link
                  key={item.to}
                  to={item.to}
                  params={{ id: repoId }}
                  className="rounded-lg px-3 py-2 text-sm text-[var(--muted)] hover:bg-[#1a2029]"
                  activeProps={{ className: "rounded-lg px-3 py-2 text-sm bg-[#1a2029] text-white" }}
                >
                  {item.label}
                </Link>
              ))
            : null}
          <div className="mt-4 space-y-1">
            {(repos.data ?? []).map((repo) => (
              <Link
                key={repo.id}
                to="/repositories/$id/overview"
                params={{ id: repo.id }}
                className="block rounded-lg px-3 py-2 text-sm text-[var(--muted)] hover:bg-[#1a2029] [&.active]:text-white"
              >
                {repo.name}
              </Link>
            ))}
          </div>
        </aside>
        <main className="px-8 py-7">
          <Outlet />
        </main>
      </div>
      {paletteOpen && (repoId || repos.data?.[0]?.id) ? (
        <CommandPalette
          repoId={repoId ?? repos.data?.[0]?.id ?? ""}
          onClose={() => setPaletteOpen(false)}
          onSelect={(symbol) => {
            const id = repoId ?? repos.data?.[0]?.id;
            setPaletteOpen(false);
            if (!id) return;
            void navigate({
              to: "/repositories/$id/files",
              params: { id },
              search: { symbol },
            });
          }}
        />
      ) : null}
    </div>
  );
}
