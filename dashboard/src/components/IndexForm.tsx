import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { startIndex, watchIndexJob } from "../api";
import type { IndexJob } from "../types";

export function IndexForm() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [source, setSource] = useState("");
  const [job, setJob] = useState<IndexJob>();

  const index = useMutation({
    mutationFn: startIndex,
    onSuccess: (started) => {
      setJob(started);
      const stop = watchIndexJob(started.jobId, (event) => {
        setJob(event);
        if (event.status === "succeeded") {
          void queryClient.invalidateQueries({ queryKey: ["repositories"] });
          void queryClient.invalidateQueries({ queryKey: ["overview"] });
          if (event.repositoryId) {
            void navigate({
              to: "/repositories/$id/overview",
              params: { id: event.repositoryId },
            });
          }
          stop();
        }
        if (event.status === "failed") stop();
      });
    },
  });

  const busy = index.isPending || (job != null && !["succeeded", "failed"].includes(job.status));

  return (
    <div className="space-y-3">
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (source.trim()) index.mutate(source.trim());
        }}
      >
        <input
          className="min-w-0 flex-1 rounded-lg border border-[var(--line)] bg-[var(--bg)] px-3 py-2 text-sm"
          value={source}
          onChange={(event) => setSource(event.target.value)}
          placeholder="https://github.com/user/project or /absolute/path"
        />
        <button
          className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-black disabled:opacity-50"
          disabled={busy}
          type="submit"
        >
          {busy ? "Indexing…" : "Index"}
        </button>
      </form>
      {index.error ? <p className="text-sm text-[#ef8b8b]">{index.error.message}</p> : null}
      {job?.error ? <p className="text-sm text-[#ef8b8b]">{job.error}</p> : null}
      {job?.status === "succeeded" && job.progress.message ? (
        <p className="text-sm text-[var(--accent)]">{job.progress.message}</p>
      ) : null}
      {job && job.status !== "succeeded" ? (
        <ol className="space-y-1 text-sm text-[var(--muted)]">
          <li className={job.progress.filesDiscovered ? "text-[var(--accent)]" : ""}>
            {job.progress.filesDiscovered ? "✓" : "●"}{" "}
            {job.progress.filesDiscovered
              ? `${job.progress.filesDiscovered.toLocaleString()} files discovered`
              : "Discovering files…"}
          </li>
          <li className={job.progress.filesParsed + job.progress.filesSkipped ? "text-[var(--accent)]" : ""}>
            {job.progress.filesParsed + job.progress.filesSkipped ? "✓" : "●"}{" "}
            {job.progress.filesParsed + job.progress.filesSkipped
              ? `${job.progress.filesParsed.toLocaleString()} parsed, ${job.progress.filesSkipped.toLocaleString()} skipped`
              : "Parsing changed files…"}
          </li>
          <li className={job.progress.symbolsExtracted ? "text-[var(--accent)]" : ""}>
            {job.progress.symbolsExtracted ? "✓" : "●"}{" "}
            {job.progress.symbolsExtracted
              ? `${job.progress.symbolsExtracted.toLocaleString()} symbols extracted`
              : "Extracting symbols…"}
          </li>
          <li className={job.status === "embedding" || job.progress.chunksEmbedded ? "text-[var(--accent)]" : ""}>
            {job.progress.chunksEmbedded ? "✓" : "●"} Generating embeddings…
          </li>
        </ol>
      ) : null}
    </div>
  );
}
