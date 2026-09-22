use codeatlas_domain::{FileId, SemanticUnit, SymbolId};
use serde::{Deserialize, Serialize};

use crate::context::ContextPack;
use crate::embed::tokenize;
use crate::error::AiError;
use crate::intent::QueryIntent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnswerMode {
    Retrieved,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextConfidence {
    High,
    Medium,
    Low,
}

impl ContextConfidence {
    pub fn from_counts(symbols: usize, files: usize) -> Self {
        if symbols >= 4 && files >= 3 {
            Self::High
        } else if symbols >= 2 && files >= 2 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            Self::High => "High confidence",
            Self::Medium => "Medium confidence",
            Self::Low => "Low confidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub file_id: FileId,
    pub symbol_id: SymbolId,
    pub symbol: String,
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub excerpt: String,
}

impl Citation {
    pub fn from_unit(unit: &SemanticUnit) -> Self {
        Self {
            file_id: unit.file_id,
            symbol_id: unit.symbol_id,
            symbol: unit.name.clone(),
            path: unit.path.clone(),
            start_line: unit.start_line,
            end_line: unit.end_line,
            excerpt: unit.text.lines().take(8).collect::<Vec<_>>().join("\n"),
        }
    }

    pub fn key(&self) -> String {
        citation_key(self.symbol_id)
    }

    pub fn location(&self) -> String {
        format!("{}:{}-{}", self.path, self.start_line, self.end_line)
    }
}

pub fn citation_key(symbol_id: SymbolId) -> String {
    symbol_id.to_string().replace('-', "").chars().take(8).collect()
}

pub fn retrieval_summary(symbols: usize, files: usize) -> String {
    format!("CodeAtlas found {symbols} relevant symbols across {files} files.")
}

/// Replace `[citation:abc123]` with `src/auth/AuthService.ts:18-42`.
pub fn resolve_citation_markers(text: &str, citations: &[Citation]) -> String {
    let mut out = text.to_string();
    for citation in citations {
        let marker = format!("[citation:{}]", citation.key());
        out = out.replace(&marker, &citation.location());
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundedAnswer {
    pub text: String,
    pub citations: Vec<Citation>,
    pub mode: AnswerMode,
    pub intent: QueryIntent,
    pub confidence: ContextConfidence,
    pub retrieval_summary: String,
}

impl GroundedAnswer {
    pub fn assemble(
        text: String,
        citations: Vec<Citation>,
        mode: AnswerMode,
        intent: QueryIntent,
        confidence: ContextConfidence,
        retrieval_summary: String,
    ) -> Self {
        Self {
            text: resolve_citation_markers(&text, &citations),
            citations,
            mode,
            intent,
            confidence,
            retrieval_summary,
        }
    }
}

pub trait AnswerModel: Send + Sync {
    fn answer(&self, pack: &ContextPack) -> Result<GroundedAnswer, AiError>;
}

pub struct ExtractiveModel;

impl AnswerModel for ExtractiveModel {
    fn answer(&self, pack: &ContextPack) -> Result<GroundedAnswer, AiError> {
        let citations = citations_from(pack);
        let lead = pack.units.first().ok_or(AiError::EmptyContext)?;
        let handles = handles_from(&lead.unit.text);
        let mut text = match pack.intent {
            QueryIntent::Explain => explain(pack),
            _ => format!(
                "{}{}\n\n{}:{}",
                lead.unit.name,
                if matches!(
                    lead.unit.kind,
                    codeatlas_domain::SymbolKind::Function | codeatlas_domain::SymbolKind::Method
                ) {
                    "()"
                } else {
                    ""
                },
                lead.unit.path,
                lead.unit.start_line
            ),
        };
        if !handles.is_empty() {
            text.push_str("\n\nHandles:\n");
            for item in handles {
                text.push_str(&format!("- {item}\n"));
            }
        }
        Ok(GroundedAnswer::assemble(
            text,
            citations,
            AnswerMode::Retrieved,
            pack.intent,
            pack.confidence(),
            pack.retrieval_summary(),
        ))
    }
}

fn explain(pack: &ContextPack) -> String {
    let names = pack
        .units
        .iter()
        .take(4)
        .map(|unit| unit.unit.name.as_str())
        .collect::<Vec<_>>();
    let start = names.first().copied().unwrap_or("the retrieved symbols");
    let rest = if names.len() > 1 {
        format!(", which connects to {}", names[1..].join(", "))
    } else {
        String::new()
    };
    let rel = pack
        .relationships
        .first()
        .map(|rel| format!(" Key relationship: {rel}."))
        .unwrap_or_default();
    let markers = pack
        .units
        .iter()
        .take(4)
        .map(|unit| format!("[citation:{}]", citation_key(unit.unit.symbol_id)))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{start} is the primary entry{rest}.{rel} {markers}")
}

fn handles_from(text: &str) -> Vec<String> {
    let mut comments = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(comment) = trimmed.strip_prefix("//").or_else(|| trimmed.strip_prefix("#")) {
            let comment = comment.trim();
            if comment.len() > 3 {
                comments.push(comment.to_string());
            }
        }
    }
    if !comments.is_empty() {
        comments.dedup();
        return comments.into_iter().take(6).collect();
    }

    let mut items = Vec::new();
    for token in tokenize(text) {
        if token.len() > 4
            && (token.contains("fail")
                || token.contains("retry")
                || token.contains("notify")
                || token.contains("status")
                || token.contains("transaction")
                || token.contains("stripe")
                || token.contains("session")
                || token.contains("auth"))
        {
            items.push(token);
        }
    }
    items.sort();
    items.dedup();
    items.into_iter().take(6).collect()
}

fn citations_from(pack: &ContextPack) -> Vec<Citation> {
    let lead_path = pack.units.first().map(|unit| unit.unit.path.as_str());
    pack.units
        .iter()
        .filter(|scored| {
            pack.intent != QueryIntent::Locate
                || lead_path.is_some_and(|path| scored.unit.path == path)
        })
        .take(4)
        .map(|scored| Citation::from_unit(&scored.unit))
        .collect()
}

/// Transport-agnostic completion. The API crate owns HTTP.
pub trait LanguageClient: Send + Sync {
    fn complete(&self, system: &str, user: &str) -> Result<String, AiError>;
}

pub struct DelegatedModel {
    client: std::sync::Arc<dyn LanguageClient>,
}

impl DelegatedModel {
    pub fn new(client: std::sync::Arc<dyn LanguageClient>) -> Self {
        Self { client }
    }
}

impl AnswerModel for DelegatedModel {
    fn answer(&self, pack: &ContextPack) -> Result<GroundedAnswer, AiError> {
        let system = "Answer using only the provided codebase context. Cite sources as [citation:KEY] using the keys shown in context. Never invent keys. If the context is insufficient, say so.";
        let user = format!("Question: {}\n\n{}", pack.question, pack.render());
        let content = self.client.complete(system, &user)?;
        let parsed: serde_json::Value =
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));
        let text = parsed["text"]
            .as_str()
            .unwrap_or(&content)
            .to_string();
        Ok(GroundedAnswer::assemble(
            text,
            citations_from(pack),
            AnswerMode::Model,
            pack.intent,
            pack.confidence(),
            pack.retrieval_summary(),
        ))
    }
}

pub struct FallbackModel {
    primary: Option<DelegatedModel>,
    fallback: ExtractiveModel,
}

impl FallbackModel {
    pub fn extractive() -> Self {
        Self {
            primary: None,
            fallback: ExtractiveModel,
        }
    }

    pub fn with_client(client: std::sync::Arc<dyn LanguageClient>) -> Self {
        Self {
            primary: Some(DelegatedModel::new(client)),
            fallback: ExtractiveModel,
        }
    }
}

impl Default for FallbackModel {
    fn default() -> Self {
        Self::extractive()
    }
}

impl AnswerModel for FallbackModel {
    fn answer(&self, pack: &ContextPack) -> Result<GroundedAnswer, AiError> {
        if let Some(model) = &self.primary {
            if let Ok(answer) = model.answer(pack) {
                if !answer.text.trim().is_empty() {
                    return Ok(answer);
                }
            }
        }
        self.fallback.answer(pack)
    }
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{Language, SemanticUnit, SymbolKind};

    use super::*;

    #[test]
    fn rewrites_citation_markers_to_locations() {
        let unit = SemanticUnit {
            symbol_id: SymbolId::new(),
            repository_id: codeatlas_domain::RepositoryId::new(),
            file_id: FileId::new(),
            name: "AuthService".into(),
            kind: SymbolKind::Class,
            path: "src/auth/AuthService.ts".into(),
            language: Language::TypeScript,
            start_line: 18,
            end_line: 42,
            text: "export class AuthService {}".into(),
        };
        let citation = Citation::from_unit(&unit);
        let text = format!("See [citation:{}]", citation.key());
        assert_eq!(
            resolve_citation_markers(&text, &[citation]),
            "See src/auth/AuthService.ts:18-42"
        );
    }
}
