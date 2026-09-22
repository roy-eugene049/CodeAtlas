import { useNavigate, useParams } from "@tanstack/react-router";

import { AskPanel } from "../components/AskPanel";

export function AskPage() {
  const { id } = useParams({ from: "/repositories/$id/ai" });
  const navigate = useNavigate();
  return (
    <AskPanel
      repoId={id}
      onOpenCitation={(symbol) =>
        void navigate({
          to: "/repositories/$id/files",
          params: { id },
          search: { symbol },
        })
      }
    />
  );
}
