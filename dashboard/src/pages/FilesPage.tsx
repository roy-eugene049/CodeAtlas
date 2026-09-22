import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import { Explorer } from "../components/Explorer";

export function FilesPage() {
  const { id } = useParams({ from: "/repositories/$id/files" });
  const search = useSearch({ from: "/repositories/$id/files" });
  const navigate = useNavigate();
  return (
    <div>
      <h1 className="mb-4 text-3xl font-semibold">Code explorer</h1>
      <Explorer
        repoId={id}
        selectedSymbolId={search.symbol}
        onSelect={(symbol) =>
          void navigate({
            to: "/repositories/$id/files",
            params: { id },
            search: { symbol },
          })
        }
      />
    </div>
  );
}
