use std::collections::HashSet;

use codeatlas_domain::{
    Language, Relationship, RelationshipKind, RepositoryIndex, SymbolId, SymbolKind,
};
use serde::{Deserialize, Serialize};

use crate::classify::{is_route_path, is_test_path};
use crate::{ArchitectureKind, ArchitectureLayer, CodeGraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchitectureGroup {
    Frontend,
    Backend,
    Infrastructure,
    Other,
}

impl ArchitectureGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::Backend => "backend",
            Self::Infrastructure => "infrastructure",
            Self::Other => "other",
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::Frontend => 0,
            Self::Backend => 1,
            Self::Infrastructure => 2,
            Self::Other => 3,
        }
    }

    fn layer_label(kind: ArchitectureKind) -> &'static str {
        match kind {
            ArchitectureKind::Api => "Controllers",
            ArchitectureKind::Services => "Services",
            ArchitectureKind::Repositories => "Repositories",
            ArchitectureKind::Database => "Database",
            ArchitectureKind::Workers => "Queue",
            ArchitectureKind::Components => "Components",
            ArchitectureKind::Hooks => "Hooks",
            ArchitectureKind::Pages => "Pages",
            ArchitectureKind::External => "External APIs",
            ArchitectureKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureGroupNode {
    pub name: String,
    pub group: ArchitectureGroup,
    pub file_count: u64,
    pub layers: Vec<ArchitectureLayer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureOutline {
    pub groups: Vec<ArchitectureGroupNode>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoHealth {
    pub file_count: u64,
    pub line_count: u64,
    pub language_count: u64,
    pub symbol_count: u64,
    pub dependency_count: u64,
    pub api_endpoint_count: u64,
    pub test_file_count: u64,
    pub test_symbol_count: u64,
    pub architecture_mapped: f32,
    pub complexity: f32,
    pub test_presence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolRef {
    pub id: SymbolId,
    pub name: String,
    pub kind: String,
    pub path: String,
    pub relation: String,
}

impl CodeGraph {
    pub fn architecture_outline(&self) -> ArchitectureOutline {
        let layers = self.architecture();
        let mut groups = [
            ArchitectureGroup::Frontend,
            ArchitectureGroup::Backend,
            ArchitectureGroup::Infrastructure,
            ArchitectureGroup::Other,
        ]
        .into_iter()
        .map(|group| ArchitectureGroupNode {
            name: group_title(group).into(),
            group,
            file_count: 0,
            layers: Vec::new(),
        })
        .collect::<Vec<_>>();

        for layer in layers {
            if layer.file_count == 0 {
                continue;
            }
            if let Some(node) = groups
                .iter_mut()
                .find(|node| node.group == layer.kind.group())
            {
                node.file_count += layer.file_count;
                node.layers.push(layer);
            }
        }
        groups.retain(|node| node.file_count > 0);
        groups.sort_by_key(|node| node.group.sort_key());

        let summary = interpret(&groups);
        ArchitectureOutline { groups, summary }
    }

    pub fn health(&self, index: &RepositoryIndex) -> RepoHealth {
        let file_count = index.files.len() as u64;
        let language_count = index
            .files
            .iter()
            .map(|file| file.language)
            .collect::<HashSet<Language>>()
            .len() as u64;
        let test_file_count = index
            .files
            .iter()
            .filter(|file| is_test_path(&file.path))
            .count() as u64;
        let test_symbol_count = index
            .symbols
            .iter()
            .filter(|symbol| {
                self.file_path(symbol.file_id)
                    .is_some_and(is_test_path)
            })
            .count() as u64;
        let api_endpoint_count = index
            .symbols
            .iter()
            .filter(|symbol| {
                matches!(
                    symbol.kind,
                    SymbolKind::Function | SymbolKind::Method | SymbolKind::Component
                ) && self.file_path(symbol.file_id).is_some_and(is_route_path)
            })
            .count() as u64;
        let mapped = index
            .files
            .iter()
            .filter(|file| ArchitectureKind::from_path(&file.path) != ArchitectureKind::Other)
            .count() as u64;
        let architecture_mapped = if file_count == 0 {
            0.0
        } else {
            mapped as f32 / file_count as f32
        };
        let test_presence = if file_count == 0 {
            0.0
        } else {
            test_file_count as f32 / file_count as f32
        };

        RepoHealth {
            file_count,
            line_count: index.line_count,
            language_count,
            symbol_count: index.symbols.len() as u64,
            dependency_count: index.stats().dependency_count,
            api_endpoint_count,
            test_file_count,
            test_symbol_count,
            architecture_mapped,
            complexity: structural_complexity(self, index),
            test_presence,
        }
    }

    pub fn outgoing_refs(&self, id: SymbolId) -> Vec<SymbolRef> {
        self.refs_from(self.outgoing.get(&id).into_iter().flatten(), true)
    }

    pub fn incoming_refs(&self, id: SymbolId) -> Vec<SymbolRef> {
        self.refs_from(self.incoming.get(&id).into_iter().flatten(), false)
    }

    fn refs_from<'a>(
        &self,
        rels: impl Iterator<Item = &'a Relationship>,
        outgoing: bool,
    ) -> Vec<SymbolRef> {
        let mut refs = Vec::new();
        let mut seen = HashSet::new();
        for rel in rels {
            if matches!(rel.kind, RelationshipKind::Contains) {
                continue;
            }
            let target_id = if outgoing { rel.target } else { rel.source };
            if !seen.insert((target_id, rel.kind)) {
                continue;
            }
            let Some(symbol) = self.symbols.get(&target_id) else {
                continue;
            };
            let Some(path) = self.file_path(symbol.file_id) else {
                continue;
            };
            refs.push(SymbolRef {
                id: symbol.id,
                name: symbol.name.clone(),
                kind: symbol.kind.as_str().to_string(),
                path: path.to_string(),
                relation: rel.kind.as_str().to_string(),
            });
        }
        refs.sort_by(|a, b| a.name.cmp(&b.name));
        refs
    }
}

fn structural_complexity(graph: &CodeGraph, index: &RepositoryIndex) -> f32 {
    let callables = index
        .symbols
        .iter()
        .filter(|symbol| matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method))
        .collect::<Vec<_>>();
    if callables.is_empty() {
        return 0.0;
    }
    let fanout = callables
        .iter()
        .map(|symbol| {
            graph
                .outgoing
                .get(&symbol.id)
                .map(|rels| {
                    rels.iter()
                        .filter(|rel| rel.kind == RelationshipKind::Calls)
                        .count()
                })
                .unwrap_or(0) as f32
        })
        .sum::<f32>()
        / callables.len() as f32;
    let span = callables
        .iter()
        .map(|symbol| symbol.end_line.saturating_sub(symbol.start_line).max(1) as f32)
        .sum::<f32>()
        / callables.len() as f32;
    ((fanout / 6.0).min(1.0) + (span / 60.0).min(1.0)) / 2.0
}

fn group_title(group: ArchitectureGroup) -> &'static str {
    match group {
        ArchitectureGroup::Frontend => "Frontend",
        ArchitectureGroup::Backend => "Backend",
        ArchitectureGroup::Infrastructure => "Infrastructure",
        ArchitectureGroup::Other => "Other",
    }
}

fn interpret(groups: &[ArchitectureGroupNode]) -> String {
    if groups.is_empty() {
        return "No architecture layers were detected from file paths.".into();
    }
    let mut parts = Vec::new();
    for group in groups {
        let layers = group
            .layers
            .iter()
            .map(|layer| {
                format!(
                    "{} ({})",
                    ArchitectureGroup::layer_label(layer.kind),
                    layer.file_count
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!(
            "{} — {} file{}: {layers}",
            group.name,
            group.file_count,
            if group.file_count == 1 { "" } else { "s" }
        ));
    }
    format!(
        "Inferred from paths, not an LLM. {}",
        parts.join(". ")
    )
}
