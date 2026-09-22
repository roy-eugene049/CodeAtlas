use codeatlas_domain::{GitEvolution, RepositoryIndex, SymbolId};

use crate::answer::{AnswerModel, GroundedAnswer};
use crate::context::ContextPack;
use crate::embed::Embedder;
use crate::error::AiError;
use crate::evolution::answer_evolution;
use crate::inspect::{explain_impact, explain_symbol, ImpactNarrative, SymbolExplanation};
use crate::intent::detect_intent;
use crate::retrieve::retrieve;
use codeatlas_graph::ImpactReport;

pub struct ContextEngine<E, M> {
    embedder: E,
    model: M,
}

impl<E: Embedder, M: AnswerModel> ContextEngine<E, M> {
    pub fn new(embedder: E, model: M) -> Self {
        Self { embedder, model }
    }

    pub fn ask(
        &self,
        question: &str,
        index: &RepositoryIndex,
        embeddings: &[(SymbolId, Vec<f32>)],
    ) -> Result<GroundedAnswer, AiError> {
        let question = question.trim();
        if question.is_empty() {
            return Err(AiError::EmptyContext);
        }
        let intent = detect_intent(question);
        let units = retrieve(question, intent, index, embeddings, &self.embedder)?;
        let pack = ContextPack::build(question, intent, index, units);
        if pack.units.is_empty() {
            return Err(AiError::EmptyContext);
        }
        let mut answer = self.model.answer(&pack)?;
        answer.citations.retain(|citation| {
            pack.units.iter().any(|unit| {
                unit.unit.symbol_id == citation.symbol_id && unit.unit.path == citation.path
            })
        });
        answer.confidence = pack.confidence();
        answer.retrieval_summary = pack.retrieval_summary();
        answer.text = crate::answer::resolve_citation_markers(&answer.text, &answer.citations);
        Ok(answer)
    }

    pub fn explain_symbol(
        &self,
        symbol_id: SymbolId,
        index: &RepositoryIndex,
    ) -> Result<SymbolExplanation, AiError> {
        explain_symbol(symbol_id, index)
    }

    pub fn explain_impact(
        &self,
        report: &ImpactReport,
        index: &RepositoryIndex,
    ) -> ImpactNarrative {
        explain_impact(report, index)
    }

    pub fn ask_evolution(
        &self,
        question: &str,
        evolution: &GitEvolution,
        index: &RepositoryIndex,
    ) -> Result<GroundedAnswer, AiError> {
        answer_evolution(question, evolution, index)
    }
}

impl ContextEngine<crate::embed::HashedEmbedder, crate::answer::FallbackModel> {
    pub fn production() -> Self {
        Self::new(
            crate::embed::HashedEmbedder::default(),
            crate::answer::FallbackModel::default(),
        )
    }
}
