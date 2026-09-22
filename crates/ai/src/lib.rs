//! Prompt construction, retrieved context, LLM completion, and citations.
//!
//! This crate has no HTTP client. The API layer supplies a `LanguageClient`.
//! Streaming and tool calls stay as surfaces until that phase is built.

mod answer;
mod context;
mod embed;
mod engine;
mod error;
mod evolution;
mod inspect;
mod intent;
mod retrieve;
mod summary;

pub use answer::{
    citation_key, resolve_citation_markers, retrieval_summary, AnswerMode, Citation,
    ContextConfidence, DelegatedModel, ExtractiveModel, FallbackModel, GroundedAnswer,
    LanguageClient,
};
pub use inspect::{explain_impact, explain_symbol, ImpactNarrative, SymbolExplanation};
pub use embed::{cosine, HashedEmbedder, Embedder};
pub use engine::ContextEngine;
pub use evolution::answer_evolution;
pub use error::AiError;
pub use intent::{detect_intent, QueryIntent};
pub use summary::summarize_repository;

pub fn embed_units(
    embedder: &impl Embedder,
    units: &[codeatlas_domain::SemanticUnit],
) -> Result<Vec<(codeatlas_domain::SymbolId, Vec<f32>)>, AiError> {
    codeatlas_search::embed_units(embedder, units).map_err(|error| AiError::Embedding(error.to_string()))
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        Language, Relationship, RelationshipKind, Repository, RepositoryIndex, SemanticUnit,
        SourceFile, Symbol, SymbolKind,
    };

    use super::*;

    fn payment_index() -> RepositoryIndex {
        let repo = Repository::new("pay", "https://example.com/pay", "main", "sha");
        let file = SourceFile::new(repo.id, "src/services/payment.ts", Language::TypeScript, 80, "h");
        let auth = SourceFile::new(repo.id, "src/auth/AuthProvider.tsx", Language::TypeScript, 40, "a");
        let handle = Symbol::new(file.id, "handleFailure", SymbolKind::Method, 10, 16);
        let process = Symbol::new(file.id, "processPayment", SymbolKind::Method, 2, 5);
        let provider = Symbol::new(auth.id, "AuthProvider", SymbolKind::Component, 1, 12);
        let handle_unit = SemanticUnit {
            symbol_id: handle.id,
            repository_id: repo.id,
            file_id: file.id,
            name: handle.name.clone(),
            kind: handle.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: handle.start_line,
            end_line: handle.end_line,
            text: "handleFailure(error) {\n  scheduleRetry();\n  setTransactionStatus('failed');\n  dispatchNotification();\n}".into(),
        };
        let process_unit = SemanticUnit {
            symbol_id: process.id,
            repository_id: repo.id,
            file_id: file.id,
            name: process.name.clone(),
            kind: process.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: process.start_line,
            end_line: process.end_line,
            text: "processPayment(amount) { return charge(amount); }".into(),
        };
        let auth_unit = SemanticUnit {
            symbol_id: provider.id,
            repository_id: repo.id,
            file_id: auth.id,
            name: provider.name.clone(),
            kind: provider.kind,
            path: auth.path.clone(),
            language: auth.language,
            start_line: provider.start_line,
            end_line: provider.end_line,
            text: "export function AuthProvider() { useSession(); }".into(),
        };
        RepositoryIndex {
            repository: repo,
            files: vec![file, auth],
            symbols: vec![handle.clone(), process.clone(), provider.clone()],
            relationships: vec![Relationship::new(
                handle.id,
                process.id,
                RelationshipKind::Calls,
            )],
            units: vec![handle_unit, process_unit, auth_unit],
            line_count: 40,
        }
    }

    #[test]
    fn failed_payments_retrieve_handle_failure_and_cite_it() {
        let index = payment_index();
        let embedder = HashedEmbedder::default();
        let embeddings = embed_units(&embedder, &index.units).expect("embed");
        let engine = ContextEngine::new(embedder, ExtractiveModel);
        let answer = engine
            .ask("Where do we handle failed payments?", &index, &embeddings)
            .expect("ask");
        assert!(answer.text.contains("handleFailure"));
        assert!(
            answer.text.contains("src/services/payment.ts:10-16")
                || answer.citations.iter().any(|citation| citation.location() == "src/services/payment.ts:10-16"),
            "markers should resolve to path:start-end, got {}",
            answer.text
        );
        assert!(answer.retrieval_summary.contains("relevant symbols"));
        assert!(answer
            .citations
            .iter()
            .any(|citation| citation.path.ends_with("payment.ts") && citation.start_line == 10));
        assert!(!answer.text.to_ascii_lowercase().contains("authprovider"));
        assert_eq!(answer.mode, AnswerMode::Retrieved);
    }

    #[test]
    fn authentication_question_retrieves_auth_symbols() {
        let index = payment_index();
        let embeddings = embed_units(&HashedEmbedder::default(), &index.units).expect("embed");
        let engine = ContextEngine::new(HashedEmbedder::default(), ExtractiveModel);
        let answer = engine
            .ask("How does authentication work?", &index, &embeddings)
            .expect("ask");
        assert_eq!(
            answer.citations.first().map(|citation| citation.symbol.as_str()),
            Some("AuthProvider"),
            "auth question should lead with AuthProvider, got {:?}",
            answer.citations.iter().map(|c| &c.symbol).collect::<Vec<_>>()
        );
        assert!(answer.text.contains("AuthProvider"));
    }
}
