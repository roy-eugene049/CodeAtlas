import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { askQuestion } from "../api";
import type { Citation } from "../types";
import { AnswerText } from "./AnswerText";

export function AskPanel({
  repoId,
  onOpenCitation,
}: {
  repoId: string;
  onOpenCitation: (symbolId: string) => void;
}) {
  const [question, setQuestion] = useState("");
  const ask = useMutation({
    mutationFn: (value: string) => askQuestion(repoId, value),
  });

  return (
    <section className="ask-workspace">
      <header>
        <h2>Ask CodeAtlas</h2>
        <p className="muted">
          Retrieval first: symbol units, then the graph, then a ranked pack. The model never sees
          the whole repository.
        </p>
      </header>
      <form
        className="ask-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (question.trim()) ask.mutate(question.trim());
        }}
      >
        <textarea
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          placeholder="How does authentication work?"
          rows={3}
        />
        <button className="primary" disabled={ask.isPending} type="submit">
          {ask.isPending ? "Retrieving…" : "Ask"}
        </button>
      </form>
      {ask.error ? <p className="error">{ask.error.message}</p> : null}
      {ask.data ? (
        <article className="ask-answer">
          <p className="muted">
            {confidenceLabel(ask.data.confidence)} · {ask.data.intent} · {ask.data.mode}
          </p>
          <p className="muted">{ask.data.retrievalSummary}</p>
          <AnswerText
            text={ask.data.answer}
            citations={ask.data.citations}
            onOpen={onOpenCitation}
          />
          <div className="citation-list">
            {ask.data.citations.map((citation) => (
              <CitationCard
                key={`${citation.symbolId}-${citation.startLine}`}
                citation={citation}
                onOpen={onOpenCitation}
              />
            ))}
          </div>
        </article>
      ) : null}
    </section>
  );
}

function CitationCard({
  citation,
  onOpen,
}: {
  citation: Citation;
  onOpen: (symbolId: string) => void;
}) {
  return (
    <button className="citation-card" onClick={() => onOpen(citation.symbolId)} type="button">
      <div className="citation-card-top">
        <strong>
          {citation.path}:{citation.startLine}-{citation.endLine}
        </strong>
      </div>
      <span className="muted">{citation.symbol}</span>
    </button>
  );
}

function confidenceLabel(value: "high" | "medium" | "low") {
  if (value === "high") return "High confidence";
  if (value === "medium") return "Medium confidence";
  return "Low confidence";
}
