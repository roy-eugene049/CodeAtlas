import type { Citation } from "../types";

const LOCATION = /([A-Za-z0-9_./\\-]+):(\d+)-(\d+)/g;

export function AnswerText({
  text,
  citations,
  onOpen,
}: {
  text: string;
  citations: Citation[];
  onOpen: (symbolId: string) => void;
}) {
  const nodes: Array<{ key: string; text: string; citation?: Citation }> = [];
  let last = 0;
  for (const match of text.matchAll(LOCATION)) {
    const index = match.index ?? 0;
    if (index > last) {
      nodes.push({ key: `t-${last}`, text: text.slice(last, index) });
    }
    const location = match[0];
    const citation = citations.find(
      (item) => `${item.path}:${item.startLine}-${item.endLine}` === location,
    );
    nodes.push({ key: location + index, text: location, citation });
    last = index + location.length;
  }
  if (last < text.length) {
    nodes.push({ key: `t-${last}`, text: text.slice(last) });
  }

  return (
    <div className="ask-prose">
      {nodes.map((node) =>
        node.citation ? (
          <button
            key={node.key}
            className="citation-inline"
            onClick={() => onOpen(node.citation!.symbolId)}
            type="button"
          >
            {node.text}
          </button>
        ) : (
          <span key={node.key}>{node.text}</span>
        ),
      )}
    </div>
  );
}
