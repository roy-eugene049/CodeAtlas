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
          className="field"
          value={source}
          onChange={(event) => setSource(event.target.value)}
          placeholder="Git URL or local path"
        />
        <button className="btn btn-primary" disabled={busy} type="submit">
          {busy ? "Indexing…" : "Index"}
        </button>
      </form>
      {index.error ? <p className="error">{index.error.message}</p> : null}
      {job?.error ? <p className="error">{job.error}</p> : null}
      {job?.status === "succeeded" && job.progress.message ? (
        <p className="muted">{job.progress.message}</p>
      ) : null}
      {job && job.status !== "succeeded" ? (
        <ol className="muted" style={{ margin: 0, paddingLeft: 18, fontSize: 13 }}>
          <li>
            {job.progress.filesDiscovered
              ? `${job.progress.filesDiscovered.toLocaleString()} files`
              : "Discovering files"}
          </li>
          <li>
            {job.progress.filesParsed + job.progress.filesSkipped
              ? `${job.progress.filesParsed.toLocaleString()} parsed`
              : "Parsing"}
          </li>
          <li>
            {job.progress.symbolsExtracted
              ? `${job.progress.symbolsExtracted.toLocaleString()} symbols`
              : "Extracting symbols"}
          </li>
        </ol>
      ) : null}
    </div>
  );
}
