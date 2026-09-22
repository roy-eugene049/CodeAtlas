use std::collections::HashMap;

use codeatlas_domain::{is_sensitive_path, RepositoryIndex, SemanticUnit, SymbolId};
use codeatlas_graph::CodeGraph;

use crate::embed::{cosine, query_tokens, expanded_query, Embedder};
use crate::error::AiError;
use crate::intent::QueryIntent;

const SEMANTIC_K: usize = 16;
const SYMBOL_K: usize = 8;
const FINAL_K: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalSource {
    Semantic,
    Symbol,
    Graph,
}

#[derive(Debug, Clone)]
pub struct ScoredUnit {
    pub unit: SemanticUnit,
    pub score: f32,
    pub source: RetrievalSource,
}

pub fn retrieve(
    question: &str,
    intent: QueryIntent,
    index: &RepositoryIndex,
    embeddings: &[(SymbolId, Vec<f32>)],
    embedder: &impl Embedder,
) -> Result<Vec<ScoredUnit>, AiError> {
    let query_vec = embedder
        .embed(&[expanded_query(question)])
        .map_err(|error| AiError::Embedding(error.to_string()))?
        .into_iter()
        .next()
        .ok_or(AiError::EmptyContext)?;

    let units: HashMap<SymbolId, &SemanticUnit> = index
        .units
        .iter()
        .filter(|unit| !is_sensitive_path(&unit.path))
        .map(|unit| (unit.symbol_id, unit))
        .collect();

    let mut scored: HashMap<SymbolId, ScoredUnit> = HashMap::new();

    let mut semantic = embeddings
        .iter()
        .filter_map(|(id, vector)| {
            let unit = units.get(id)?;
            Some((
                *id,
                cosine(&query_vec, vector),
                (*unit).clone(),
            ))
        })
        .collect::<Vec<_>>();
    semantic.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (id, score, unit) in semantic.into_iter().take(SEMANTIC_K) {
        scored.insert(
            id,
            ScoredUnit {
                unit,
                score: score + name_bonus(question, &index.units.iter().find(|u| u.symbol_id == id).map(|u| u.name.as_str()).unwrap_or("")),
                source: RetrievalSource::Semantic,
            },
        );
    }

    let tokens = query_tokens(question);
    let mut lexical = index
        .units
        .iter()
        .filter(|unit| !is_sensitive_path(&unit.path))
        .map(|unit| {
            let name_hit = name_bonus(question, &unit.name);
            let token_hit = tokens
                .iter()
                .filter(|token| {
                    unit.name.to_ascii_lowercase().contains(token.as_str())
                        || unit.text.to_ascii_lowercase().contains(token.as_str())
                })
                .count() as f32
                / tokens.len().max(1) as f32;
            (unit, name_hit + token_hit)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect::<Vec<_>>();
    lexical.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (unit, score) in lexical.into_iter().take(SYMBOL_K) {
        scored
            .entry(unit.symbol_id)
            .and_modify(|existing| existing.score += score * 0.35)
            .or_insert(ScoredUnit {
                unit: unit.clone(),
                score: score * 0.8,
                source: RetrievalSource::Symbol,
            });
    }

    let graph = CodeGraph::build(index);
    let mut seed_rank = scored
        .iter()
        .map(|(id, unit)| (*id, unit.score))
        .collect::<Vec<_>>();
    seed_rank.sort_by(|a, b| b.1.total_cmp(&a.1));
    let seeds: Vec<SymbolId> = seed_rank
        .into_iter()
        .take(3)
        .map(|(id, _)| id)
        .collect();
    for seed in seeds {
        let mut neighbor_ids = graph
            .neighbors(seed, None)
            .into_iter()
            .map(|symbol| symbol.id)
            .collect::<Vec<_>>();
        neighbor_ids.extend(graph.incoming(seed, None).into_iter().map(|symbol| symbol.id));
        for neighbor_id in neighbor_ids {
            if scored.contains_key(&neighbor_id) {
                continue;
            }
            if let Some(unit) = units.get(&neighbor_id) {
                scored.insert(
                    neighbor_id,
                    ScoredUnit {
                        unit: (*unit).clone(),
                        score: 0.35,
                        source: RetrievalSource::Graph,
                    },
                );
            }
        }
    }

    if matches!(intent, QueryIntent::Impact) {
        let origin_id = scored
            .values()
            .max_by(|left, right| left.score.total_cmp(&right.score))
            .map(|origin| origin.unit.symbol_id);
        if let Some(origin_id) = origin_id {
            if let Some(report) = graph.impact(origin_id) {
                for symbol in report.direct.iter().chain(report.components.iter()) {
                    if let Some(unit) = units.get(&symbol.id) {
                        scored.entry(symbol.id).or_insert(ScoredUnit {
                            unit: (*unit).clone(),
                            score: 0.3,
                            source: RetrievalSource::Graph,
                        });
                    }
                }
            }
        }
    }

    let mut ranked = scored.into_values().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    let keep = match intent {
        QueryIntent::Locate => 5,
        _ => FINAL_K,
    };
    ranked.truncate(keep);

    if ranked.is_empty() {
        return Err(AiError::EmptyContext);
    }
    Ok(ranked)
}

fn name_bonus(question: &str, name: &str) -> f32 {
    let q = question.to_ascii_lowercase();
    let n = name.to_ascii_lowercase();
    if n.len() >= 4 && q.contains(&n) {
        return 0.6;
    }
    let query = query_tokens(question);
    let name_tokens = crate::embed::tokenize(name);
    if name_tokens
        .iter()
        .any(|token| token.len() >= 4 && query.iter().any(|item| item == token))
    {
        0.45
    } else {
        0.0
    }
}

