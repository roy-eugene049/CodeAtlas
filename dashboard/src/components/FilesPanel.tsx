import { useQuery } from "@tanstack/react-query";

import { getFiles } from "../api";

export function FilesPanel({ repoId }: { repoId: string }) {
  const files = useQuery({
    queryKey: ["files", repoId],
    queryFn: () => getFiles(repoId),
  });

  if (files.isLoading) return <p className="muted">Loading files…</p>;
  if (files.error) return <p className="error">{files.error.message}</p>;

  return (
    <div className="list">
      {files.data?.map((file) => (
        <article className="file-row" key={file.id}>
          <span>{file.path}</span>
          <span className="badge">{file.language}</span>
        </article>
      ))}
    </div>
  );
}
