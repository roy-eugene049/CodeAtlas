import type { RepositorySummary, WorkstationView } from "../types";

const VIEWS: Array<{ id: WorkstationView; label: string }> = [
  { id: "overview", label: "Overview" },
  { id: "files", label: "Files" },
  { id: "symbols", label: "Symbols" },
  { id: "graph", label: "Graph" },
  { id: "search", label: "Search" },
  { id: "ai", label: "AI" },
  { id: "insights", label: "Impact" },
  { id: "evolution", label: "Evolution" },
];

interface Props {
  view: WorkstationView;
  onView: (view: WorkstationView) => void;
  repositories: RepositorySummary[];
  selectedId?: string;
  onSelect: (id: string) => void;
}

export function Sidebar({ view, onView, repositories, selectedId, onSelect }: Props) {
  return (
    <aside className="sidebar">
      {VIEWS.map((item) => (
        <button
          key={item.id}
          className={`nav-button${view === item.id ? " active" : ""}`}
          onClick={() => onView(item.id)}
          type="button"
        >
          {item.label}
        </button>
      ))}
      <div style={{ height: 18 }} />
      {repositories.map((repo) => (
        <button
          key={repo.id}
          className={`repo-option${selectedId === repo.id ? " active" : ""}`}
          onClick={() => onSelect(repo.id)}
          type="button"
        >
          {repo.name}
        </button>
      ))}
    </aside>
  );
}
