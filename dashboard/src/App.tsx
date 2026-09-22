import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { getOverview, startIndex, watchIndexJob, listRepositories } from "./api";
import type { IndexJob, WorkstationView } from "./types";
import { AskPanel } from "./components/AskPanel";
import { EvolutionPanel } from "./components/EvolutionPanel";
import { CommandPalette } from "./components/CommandPalette";
import { FilesPanel } from "./components/FilesPanel";
import { GraphPanel } from "./components/GraphPanel";
import { InsightsPanel } from "./components/InsightsPanel";
import { OverviewPanel } from "./components/OverviewPanel";
import { SearchPanel } from "./components/SearchPanel";
import { Sidebar } from "./components/Sidebar";
import { SymbolsPanel } from "./components/SymbolsPanel";

export function App() {
  const queryClient = useQueryClient();
  const [repoId, setRepoId] = useState<string>();
  const [view, setView] = useState<WorkstationView>("overview");
  const [source, setSource] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [selectedSymbolId, setSelectedSymbolId] = useState<string>();
  const [indexJob, setIndexJob] = useState<IndexJob>();

  const openImpact = (id: string, nextView: WorkstationView = "insights") => {
    setSelectedSymbolId(id);
    setView(nextView);
  };

  const repos = useQuery({
    queryKey: ["repositories"],
    queryFn: listRepositories,
  });

  useEffect(() => {
    if (!repoId && repos.data?.[0]) {
      setRepoId(repos.data[0].id);
    }
  }, [repoId, repos.data]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen(true);
        setView("search");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const overview = useQuery({
    queryKey: ["overview", repoId],
    queryFn: () => getOverview(repoId!),
    enabled: Boolean(repoId),
  });

  const index = useMutation({
    mutationFn: startIndex,
    onSuccess: (job) => {
      setIndexJob(job);
      const stop = watchIndexJob(job.jobId, (event) => {
        setIndexJob(event);
        if (event.status === "succeeded") {
          if (event.repositoryId) {
            setRepoId(event.repositoryId);
          }
          setView("overview");
          void queryClient.invalidateQueries({ queryKey: ["repositories"] });
          void queryClient.invalidateQueries({ queryKey: ["overview"] });
          stop();
        }
        if (event.status === "failed") {
          stop();
        }
      });
    },
  });

  const selected = useMemo(
    () => repos.data?.find((repo) => repo.id === repoId),
    [repoId, repos.data],
  );

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          CODEATLAS
          <span>code intelligence workstation</span>
        </div>
        <button className="search-trigger" onClick={() => setPaletteOpen(true)} type="button">
          Search codebase
          <span>⌘K</span>
        </button>
      </header>
      <div className="workspace">
        <Sidebar
          view={view}
          onView={setView}
          repositories={repos.data ?? []}
          selectedId={repoId}
          onSelect={setRepoId}
        />
        <main className="main">
          <section className="hero">
            <div>
              <h1>{overview.data?.repository.name ?? selected?.name ?? "Index a repository"}</h1>
              <p>
                {overview.data?.repository.url ??
                  "Give CodeAtlas a local path or git URL. It clones, parses, and builds a symbol graph."}
              </p>
              <form
                className="index-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  if (source.trim()) {
                    index.mutate(source.trim());
                  }
                }}
              >
                <input
                  value={source}
                  onChange={(event) => setSource(event.target.value)}
                  placeholder="https://github.com/user/project or /absolute/path"
                />
                <button
                  className="primary"
                  disabled={index.isPending || (indexJob != null && !["succeeded", "failed"].includes(indexJob.status))}
                  type="submit"
                >
                  {indexJob && !["succeeded", "failed"].includes(indexJob.status) ? "Indexing…" : "Index"}
                </button>
              </form>
              {index.error ? <p className="error">{index.error.message}</p> : null}
              {indexJob?.error ? <p className="error">{indexJob.error}</p> : null}
              {indexJob ? <IndexProgressList job={indexJob} /> : null}
            </div>
          </section>

          {view === "overview" ? <OverviewPanel overview={overview.data} /> : null}
          {view === "files" && repoId ? <FilesPanel repoId={repoId} /> : null}
          {view === "symbols" && repoId ? (
            <SymbolsPanel repoId={repoId} onSelect={(id) => openImpact(id)} />
          ) : null}
          {view === "graph" && repoId ? (
            <GraphPanel
              repoId={repoId}
              selectedSymbolId={selectedSymbolId}
              onSelect={(id) => openImpact(id, "graph")}
            />
          ) : null}
          {view === "search" && repoId ? (
            <SearchPanel repoId={repoId} onSelect={(id) => openImpact(id)} />
          ) : null}
          {view === "ai" && repoId ? (
            <AskPanel repoId={repoId} onOpenCitation={(id) => openImpact(id, "graph")} />
          ) : null}
          {view === "insights" && repoId ? (
            <InsightsPanel
              repoId={repoId}
              selectedSymbolId={selectedSymbolId}
              onSelect={(id) => openImpact(id, "insights")}
            />
          ) : null}
          {view === "evolution" && repoId ? <EvolutionPanel repoId={repoId} /> : null}
        </main>
      </div>
      {paletteOpen && repoId ? (
        <CommandPalette
          repoId={repoId}
          onClose={() => setPaletteOpen(false)}
          onSelect={(id) => openImpact(id)}
        />
      ) : null}
    </div>
  );
}

function IndexProgressList({ job }: { job: IndexJob }) {
  const steps = [
    {
      done: job.progress.filesDiscovered > 0,
      label:
        job.progress.filesDiscovered > 0
          ? `${job.progress.filesDiscovered.toLocaleString()} files discovered`
          : "Discovering files…",
    },
    {
      done: job.progress.filesParsed > 0 || job.progress.filesSkipped > 0,
      label:
        job.progress.filesParsed + job.progress.filesSkipped > 0
          ? `${job.progress.filesParsed.toLocaleString()} files parsed${
              job.progress.filesSkipped
                ? `, ${job.progress.filesSkipped.toLocaleString()} unchanged skipped`
                : ""
            }`
          : "Parsing files…",
    },
    {
      done: job.progress.symbolsExtracted > 0,
      label:
        job.progress.symbolsExtracted > 0
          ? `${job.progress.symbolsExtracted.toLocaleString()} symbols extracted`
          : "Extracting symbols…",
    },
    {
      done: job.progress.relationshipsBuilt > 0 || job.status === "embedding" || job.status === "persisting" || job.status === "succeeded",
      label: "Dependency graph built",
    },
    {
      done: job.progress.chunksEmbedded > 0 || job.status === "succeeded",
      active: job.status === "embedding",
      label:
        job.progress.chunksEmbedded > 0
          ? `${job.progress.chunksEmbedded.toLocaleString()} embeddings stored`
          : "Generating embeddings…",
    },
  ];

  return (
    <ol className="index-progress">
      {steps.map((step) => (
        <li key={step.label} className={step.done ? "done" : "active"}>
          <span>{step.done ? "✓" : "●"}</span>
          {step.label}
        </li>
      ))}
      {job.status === "failed" ? <li className="failed">Index failed</li> : null}
    </ol>
  );
}
