import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";

import { getOverview } from "../api";
import { OverviewPanel } from "../components/OverviewPanel";

export function OverviewPage() {
  const { id } = useParams({ from: "/repositories/$id/overview" });
  const overview = useQuery({
    queryKey: ["overview", id],
    queryFn: () => getOverview(id),
  });
  return (
    <div>
      <header className="page-header">
        <h1>{overview.data?.repository.name ?? "Repository"}</h1>
        <p>{overview.data?.repository.url ?? (overview.isLoading ? "Loading…" : "")}</p>
      </header>
      {overview.data?.aiSummary ? (
        <p className="muted" style={{ maxWidth: 640, marginBottom: 28, fontSize: 17, lineHeight: 1.45 }}>
          {overview.data.aiSummary}
        </p>
      ) : null}
      {overview.isLoading ? <p className="muted">Loading overview…</p> : <OverviewPanel overview={overview.data} />}
    </div>
  );
}
