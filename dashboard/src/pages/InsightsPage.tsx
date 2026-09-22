import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import { InsightsPanel } from "../components/InsightsPanel";

export function InsightsPage() {
  const { id } = useParams({ from: "/repositories/$id/insights" });
  const { symbol } = useSearch({ from: "/repositories/$id/insights" });
  const navigate = useNavigate();
  return (
    <InsightsPanel
      repoId={id}
      selectedSymbolId={symbol}
      onSelect={(next) =>
        void navigate({
          to: "/repositories/$id/insights",
          params: { id },
          search: { symbol: next },
        })
      }
    />
  );
}
