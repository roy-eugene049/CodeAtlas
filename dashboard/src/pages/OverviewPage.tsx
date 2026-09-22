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
      <h1 className="mb-1 text-3xl font-semibold">
        {overview.data?.repository.name ?? "Repository"}
      </h1>
      <p className="mb-6 text-[var(--muted)]">{overview.data?.repository.url}</p>
      {overview.data?.aiSummary ? (
        <p className="mb-6 max-w-3xl text-sm leading-6 text-[var(--muted)]">{overview.data.aiSummary}</p>
      ) : null}
      <OverviewPanel overview={overview.data} />
    </div>
  );
}
