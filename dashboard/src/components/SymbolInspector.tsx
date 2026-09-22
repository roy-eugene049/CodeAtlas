import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { getExplain, getImpact } from "../api";
import type { SymbolRef } from "../types";
import { GraphCanvas } from "./GraphCanvas";
import { ImpactReportView } from "./ImpactReport";

type InspectorTab = "explain" | "dependencies" | "impact";

export function SymbolInspector({
  repoId,
  symbolId,
  onSelect,
  onOpenSource,
  initialTab = "explain",
}: {
  repoId: string;
  symbolId: string;
  onSelect: (id: string) => void;
  onOpenSource?: (id: string) => void;
  initialTab?: InspectorTab;
}) {
  const [tab, setTab] = useState<InspectorTab>(initialTab);
  const explain = useQuery({
    queryKey: ["explain", repoId, symbolId],
    queryFn: () => getExplain(repoId, symbolId),
  });
  const impact = useQuery({
    queryKey: ["impact", repoId, symbolId],
    queryFn: () => getImpact(repoId, symbolId),
  });

  const name = explain.data?.symbol.name ?? impact.data?.origin.name ?? "Symbol";
  const kind = explain.data?.symbol.kind ?? impact.data?.origin.kind ?? "";
  const suffix = kind === "function" || kind === "method" ? "()" : "";

  return (
    <section className="inspector">
      <header className="inspector-head">
        <div>
          <h2>
            {name}
            {suffix}
          </h2>
          <p className="muted">{explain.data?.symbol.path ?? impact.data?.origin.path}</p>
        </div>
        <div className="inspector-tabs">
          <div className="segmented">
            {(
              [
                ["explain", "Explain"],
                ["dependencies", "Dependencies"],
                ["impact", "Impact"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                className={tab === id ? "active" : ""}
                onClick={() => setTab(id)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
          {onOpenSource ? (
            <button className="btn" onClick={() => onOpenSource(symbolId)} type="button">
              Open source
            </button>
          ) : null}
        </div>
      </header>

      {explain.error ? <p className="error">{explain.error.message}</p> : null}
      {impact.error ? <p className="error">{impact.error.message}</p> : null}

      {tab === "explain" ? (
        explain.isLoading ? (
          <p className="muted">Reading symbol…</p>
        ) : explain.data ? (
          <ExplainView
            purpose={explain.data.purpose}
            flow={explain.data.flow}
            dependencies={explain.data.dependencies}
            onSelect={onSelect}
          />
        ) : null
      ) : null}

      {tab === "dependencies" ? (
        explain.isLoading ? (
          <p className="muted">Walking outgoing edges…</p>
        ) : (
          <RefList
            title="Dependencies"
            empty="No outgoing calls or imports from this symbol."
            items={explain.data?.dependencies ?? []}
            onSelect={onSelect}
          />
        )
      ) : null}

      {tab === "impact" ? (
        impact.isLoading ? (
          <p className="muted">Calculating blast radius…</p>
        ) : impact.data ? (
          <>
            <article className="ask-answer" style={{ marginBottom: 20 }}>
              <p className="muted">What breaks if I change this?</p>
              <div className="ask-prose">{impact.data.narrative.text}</div>
            </article>
            <div style={{ marginBottom: 24 }}>
              <GraphCanvas
                nodes={impact.data.subgraph.nodes}
                edges={impact.data.subgraph.edges}
                focusId={impact.data.origin.id}
                selectedId={symbolId}
                onSelect={onSelect}
              />
            </div>
            <ImpactReportView impact={impact.data} onSelect={onSelect} />
          </>
        ) : null
      ) : null}
    </section>
  );
}

function ExplainView({
  purpose,
  flow,
  dependencies,
  onSelect,
}: {
  purpose: string;
  flow: string[];
  dependencies: SymbolRef[];
  onSelect: (id: string) => void;
}) {
  return (
    <div className="explain-body">
      <article className="panel">
        <h3>Purpose</h3>
        <p>{purpose}</p>
      </article>
      <article className="panel">
        <h3>Flow</h3>
        {flow.length === 0 ? <p className="muted">No extracted steps.</p> : null}
        <ol className="flow-list">
          {flow.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      </article>
      <article className="panel">
        <h3>Dependencies</h3>
        <RefList
          title=""
          empty="No linked callees."
          items={dependencies}
          onSelect={onSelect}
        />
      </article>
    </div>
  );
}

function RefList({
  title,
  empty,
  items,
  onSelect,
}: {
  title: string;
  empty: string;
  items: SymbolRef[];
  onSelect: (id: string) => void;
}) {
  return (
    <div>
      {title ? <h3>{title}</h3> : null}
      {items.length === 0 ? <p className="muted">{empty}</p> : null}
      <div className="grouped">
        {items.map((item) => (
          <button
            key={`${item.id}-${item.relation}`}
            className="grouped-row"
            onClick={() => onSelect(item.id)}
            type="button"
          >
            <span>
              <strong>{item.name}</strong>
              <div className="muted" style={{ marginTop: 4, fontSize: 13 }}>
                {item.path} · {item.relation}
              </div>
            </span>
            <span className="muted">{item.kind}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
