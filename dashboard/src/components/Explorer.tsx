import Editor from "@monaco-editor/react";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { getFileContents, getFiles, getSymbols } from "../api";
import type { FileResponse } from "../types";
import { SymbolInspector } from "./SymbolInspector";

const LANG: Record<string, string> = {
  typescript: "typescript",
  javascript: "javascript",
  rust: "rust",
  python: "python",
};

export function Explorer({
  repoId,
  onSelect,
  selectedSymbolId,
}: {
  repoId: string;
  selectedSymbolId?: string;
  onSelect: (id: string) => void;
}) {
  const files = useQuery({ queryKey: ["files", repoId], queryFn: () => getFiles(repoId) });
  const [path, setPath] = useState<string>();
  const contents = useQuery({
    queryKey: ["contents", repoId, path],
    queryFn: () => getFileContents(repoId, path!),
    enabled: Boolean(path),
  });
  const symbols = useQuery({
    queryKey: ["symbols", repoId],
    queryFn: () => getSymbols(repoId),
  });
  const selected = useMemo(
    () => (symbols.data ?? []).find((symbol) => symbol.id === selectedSymbolId),
    [symbols.data, selectedSymbolId],
  );
  const fileSymbols = useMemo(
    () => (symbols.data ?? []).filter((symbol) => symbol.path === path),
    [symbols.data, path],
  );

  useEffect(() => {
    if (selected?.path && selected.path !== path) {
      setPath(selected.path);
    }
  }, [selected?.path, path]);

  return (
    <div className="grid h-[70vh] grid-cols-[220px_minmax(0,1fr)_320px] overflow-hidden rounded-2xl border border-[var(--line)]">
      <aside className="overflow-auto border-r border-[var(--line)] bg-[var(--raised)] p-3 text-sm">
        {groupedFiles(files.data ?? []).map((group) => (
          <div key={group.dir} className="mb-3">
            <p className="mb-1 px-2 text-[11px] uppercase tracking-wide text-[var(--muted)]">
              {group.dir}
            </p>
            {group.files.map((file) => (
              <button
                key={file.id}
                className={`mb-1 block w-full truncate rounded px-2 py-1 text-left ${
                  path === file.path ? "bg-[#1a2029] text-white" : "text-[var(--muted)]"
                }`}
                onClick={() => setPath(file.path)}
                type="button"
              >
                {file.path.slice(group.dir === "." ? 0 : group.dir.length + 1) || file.path}
              </button>
            ))}
          </div>
        ))}
      </aside>
      <div className="min-w-0">
        {path ? (
          <Editor
            height="100%"
            theme="vs-dark"
            language={LANG[contents.data?.language ?? "typescript"] ?? "plaintext"}
            value={contents.data?.content ?? ""}
            options={{ readOnly: true, minimap: { enabled: false }, fontSize: 13 }}
          />
        ) : (
          <p className="p-6 text-sm text-[var(--muted)]">Select a file.</p>
        )}
      </div>
      <aside className="overflow-auto border-l border-[var(--line)] p-3">
        <p className="mb-2 text-xs uppercase tracking-wide text-[var(--muted)]">Intelligence</p>
        <div className="mb-4 space-y-1">
          {fileSymbols.map((symbol) => (
            <button
              key={symbol.id}
              className="block w-full rounded px-2 py-1 text-left text-sm hover:bg-[#1a2029]"
              onClick={() => onSelect(symbol.id)}
              type="button"
            >
              {symbol.name}
              <span className="ml-2 text-xs text-[var(--muted)]">{symbol.kind}</span>
            </button>
          ))}
        </div>
        {selectedSymbolId ? (
          <SymbolInspector
            repoId={repoId}
            symbolId={selectedSymbolId}
            onSelect={onSelect}
          />
        ) : (
          <p className="text-sm text-[var(--muted)]">Symbol, dependencies, AI explain.</p>
        )}
      </aside>
    </div>
  );
}

function groupedFiles(files: FileResponse[]) {
  const groups = new Map<string, FileResponse[]>();
  for (const file of files) {
    const slash = file.path.lastIndexOf("/");
    const dir = slash === -1 ? "." : file.path.slice(0, slash);
    const list = groups.get(dir) ?? [];
    list.push(file);
    groups.set(dir, list);
  }
  return [...groups.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([dir, files]) => ({ dir, files }));
}
