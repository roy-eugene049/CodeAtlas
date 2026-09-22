import { useQuery } from "@tanstack/react-query";
import { Link, Outlet, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { listRepositories } from "../api";
import { CommandPalette } from "../components/CommandPalette";

const NAV = [
  { to: "/dashboard", label: "Home" },
  { to: "/repositories", label: "Repositories" },
] as const;

const REPO_NAV = [
  { to: "/repositories/$id/overview", label: "Overview" },
  { to: "/repositories/$id/files", label: "Files" },
  { to: "/repositories/$id/graph", label: "Graph" },
  { to: "/repositories/$id/search", label: "Search" },
  { to: "/repositories/$id/ai", label: "Ask" },
] as const;

export function AppShell() {
  const params = useParams({ strict: false }) as { id?: string };
  const navigate = useNavigate();
  const repoId = params.id;
  const [paletteOpen, setPaletteOpen] = useState(false);
  const repos = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const current = repos.data?.find((repo) => repo.id === repoId);

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
    <div className="app-shell">
      <header className="topbar">
        <Link to="/dashboard" className="brand">
          CodeAtlas
          {current ? <span>{current.name}</span> : null}
        </Link>
        <button className="search-trigger" onClick={() => setPaletteOpen(true)} type="button">
          Search
          <span>⌘K</span>
        </button>
      </header>
      <div className="workspace">
        <aside className="sidebar">
          <p className="nav-section">Library</p>
          {NAV.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className="nav-button"
              activeProps={{ className: "nav-button active" }}
              activeOptions={{ exact: true }}
            >
              {item.label}
            </Link>
          ))}
          {repoId ? (
            <>
              <p className="nav-section">Repository</p>
              {REPO_NAV.map((item) => (
                <Link
                  key={item.to}
                  to={item.to}
                  params={{ id: repoId }}
                  className="nav-button"
                  activeProps={{ className: "nav-button active" }}
                >
                  {item.label}
                </Link>
              ))}
            </>
          ) : null}
          {(repos.data ?? []).length > 0 ? (
            <>
              <p className="nav-section">Indexed</p>
              {(repos.data ?? []).map((repo) => (
                <Link
                  key={repo.id}
                  to="/repositories/$id/overview"
                  params={{ id: repo.id }}
                  className={repo.id === repoId ? "repo-option active" : "repo-option"}
                >
                  {repo.name}
                </Link>
              ))}
            </>
          ) : null}
        </aside>
        <main className="main">
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
