import type {
  AskResponse,
  ExplainResponse,
  FileContents,
  FileResponse,
  GitEvolution,
  GraphResponse,
  ImpactResponse,
  IndexJob,
  OverviewResponse,
  RepositorySummary,
  SearchHit,
  SymbolKind,
  SymbolResponse,
} from "./types";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => ({ error: response.statusText }))) as {
      error?: string;
    };
    throw new Error(body.error ?? "Request failed");
  }
  return response.json() as Promise<T>;
}

export function listRepositories() {
  return request<RepositorySummary[]>("/api/repositories");
}

export async function startIndex(source: string) {
  const response = await fetch("/api/repositories", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ source }),
  });
  if (!response.ok && response.status !== 202) {
    const body = (await response.json().catch(() => ({ error: response.statusText }))) as {
      error?: string;
    };
    throw new Error(body.error ?? "Request failed");
  }
  return (await response.json()) as IndexJob;
}

export function watchIndexJob(jobId: string, onEvent: (event: IndexJob) => void) {
  const source = new EventSource(`/api/jobs/${jobId}/events`);
  source.onmessage = (message) => {
    const data = JSON.parse(message.data) as {
      jobId?: string;
      repositoryId?: string;
      status?: IndexJob["status"];
      stage?: IndexJob["status"];
      message?: string;
      filesDiscovered?: number;
      filesParsed?: number;
      filesSkipped?: number;
      symbolsExtracted?: number;
      relationshipsBuilt?: number;
      chunksEmbedded?: number;
      durationMs?: number;
      error?: string;
      progress?: IndexJob["progress"];
    };
    const stage = data.progress?.stage ?? data.status ?? data.stage ?? "queued";
    onEvent({
      jobId: data.jobId ?? jobId,
      repositoryId: data.repositoryId,
      status: stage,
      progress: data.progress ?? {
        stage,
        message: data.message ?? "",
        filesDiscovered: data.filesDiscovered ?? 0,
        filesParsed: data.filesParsed ?? 0,
        filesSkipped: data.filesSkipped ?? 0,
        symbolsExtracted: data.symbolsExtracted ?? 0,
        relationshipsBuilt: data.relationshipsBuilt ?? 0,
        chunksEmbedded: data.chunksEmbedded ?? 0,
        durationMs: data.durationMs ?? 0,
      },
      error: data.error,
    });
  };
  return () => source.close();
}

export function getOverview(id: string) {
  return request<OverviewResponse>(`/api/repositories/${id}/overview`);
}

export function getFiles(id: string) {
  return request<FileResponse[]>(`/api/repositories/${id}/files`);
}

export function getFileContents(id: string, path: string) {
  const params = new URLSearchParams({ path });
  return request<FileContents>(`/api/repositories/${id}/contents?${params}`);
}

export function getSymbols(id: string, query?: string, kind?: SymbolKind) {
  const params = new URLSearchParams();
  if (query) params.set("q", query);
  if (kind) params.set("kind", kind);
  const suffix = params.size ? `?${params}` : "";
  return request<SymbolResponse[]>(`/api/repositories/${id}/symbols${suffix}`);
}

export function getGraph(id: string, focus?: string, depth = 2) {
  const params = new URLSearchParams();
  if (focus) params.set("focus", focus);
  if (depth !== 2) params.set("depth", String(depth));
  const suffix = params.size ? `?${params}` : "";
  return request<GraphResponse>(`/api/repositories/${id}/graph${suffix}`);
}

export function getImpact(repoId: string, symbolId: string) {
  return request<ImpactResponse>(`/api/repositories/${repoId}/symbols/${symbolId}/impact`);
}

export function getSymbol(symbolId: string) {
  return request<SymbolResponse>(`/api/symbols/${symbolId}`);
}

export function getSymbolImpact(symbolId: string) {
  return request<ImpactResponse>(`/api/symbols/${symbolId}/impact`);
}

export function getExplain(repoId: string, symbolId: string) {
  return request<ExplainResponse>(`/api/repositories/${repoId}/symbols/${symbolId}/explain`);
}

export function getEvolution(repoId: string) {
  return request<GitEvolution>(`/api/repositories/${repoId}/evolution`);
}

export function askQuestion(repoId: string, question: string) {
  return request<AskResponse>(`/api/repositories/${repoId}/ask`, {
    method: "POST",
    body: JSON.stringify({ question }),
  });
}

export function searchSymbols(id: string, query: string) {
  const params = new URLSearchParams({ q: query });
  return request<SearchHit[]>(`/api/repositories/${id}/search?${params}`);
}
