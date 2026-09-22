import { useNavigate, useParams, useSearch } from "@tanstack/react-router";

import { Explorer } from "../components/Explorer";

export function FilesPage() {
  const { id } = useParams({ from: "/repositories/$id/files" });
  const search = useSearch({ from: "/repositories/$id/files" });
  const navigate = useNavigate();
  return (
    <div>
      <header className="page-header">
        <h1>Files</h1>
        <p>Source on the left, editor in the middle, symbols on the right.</p>
      </header>
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
