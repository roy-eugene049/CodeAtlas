use std::collections::{HashMap, VecDeque};

use codeatlas_domain::{FileId, RelationshipKind, Symbol, SymbolId};
use serde::{Deserialize, Serialize};

use crate::classify::{is_component_symbol, is_route_path, is_test_path, roles_for, ImpactRole};
use crate::{CodeGraph, GraphView};

const DEPENDENT_KINDS: &[RelationshipKind] = &[
    RelationshipKind::Calls,
    RelationshipKind::Imports,
    RelationshipKind::References,
    RelationshipKind::Extends,
    RelationshipKind::Implements,
];

const MAX_DEPTH: u32 = 16;
const MAX_NODES: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactSymbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: String,
    pub file_id: FileId,
    pub path: String,
    pub depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactedFile {
    pub path: String,
    pub file_id: FileId,
    pub symbol_count: u64,
    pub roles: Vec<ImpactRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactTreeNode {
    pub path: String,
    pub file_id: FileId,
    pub roles: Vec<ImpactRole>,
    pub children: Vec<ImpactTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactReport {
    pub origin: ImpactSymbol,
    pub file_count: u64,
    pub symbol_count: u64,
    pub direct: Vec<ImpactSymbol>,
    pub indirect: Vec<ImpactSymbol>,
    pub tests: Vec<ImpactSymbol>,
    pub test_file_count: u64,
    pub routes: Vec<ImpactSymbol>,
    pub components: Vec<ImpactSymbol>,
    pub files: Vec<ImpactedFile>,
    pub tree: ImpactTreeNode,
    pub subgraph: GraphView,
}

impl CodeGraph {
    pub fn impact(&self, origin_id: SymbolId) -> Option<ImpactReport> {
        let origin = self.symbols.get(&origin_id)?;
        let depths = self.dependent_depths(origin_id);
        let mut reached = depths
            .into_iter()
            .filter(|(id, _)| *id != origin_id)
            .filter_map(|(id, depth)| self.to_impact_symbol(id, depth))
            .collect::<Vec<_>>();
        reached.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.name.cmp(&b.name)));

        let direct = reached
            .iter()
            .filter(|symbol| symbol.depth == 1)
            .cloned()
            .collect::<Vec<_>>();
        let indirect = reached
            .iter()
            .filter(|symbol| symbol.depth > 1)
            .cloned()
            .collect::<Vec<_>>();

        let tests = reached
            .iter()
            .filter(|symbol| is_test_path(&symbol.path))
            .cloned()
            .collect::<Vec<_>>();
        let routes = reached
            .iter()
            .filter(|symbol| is_route_path(&symbol.path))
            .cloned()
            .collect::<Vec<_>>();
        let components = reached
            .iter()
            .filter(|symbol| {
                self.symbols
                    .get(&symbol.id)
                    .is_some_and(|raw| is_component_symbol(raw, &symbol.path))
            })
            .cloned()
            .collect::<Vec<_>>();

        let test_file_count = tests
            .iter()
            .map(|symbol| symbol.path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;
        let files = self.impacted_files(origin, &reached);
        let tree = impact_tree(origin, &files, self.file_path(origin.file_id));
        let subgraph = self.neighborhood(origin_id, 2);

        Some(ImpactReport {
            origin: self.to_impact_symbol(origin_id, 0)?,
            file_count: files.len() as u64,
            symbol_count: reached.len() as u64,
            direct,
            indirect,
            tests,
            test_file_count,
            routes,
            components,
            files,
            tree,
            subgraph,
        })
    }

    fn dependent_depths(&self, origin: SymbolId) -> HashMap<SymbolId, u32> {
        let mut depths = HashMap::new();
        let mut queue = VecDeque::new();
        depths.insert(origin, 0);
        queue.push_back(origin);

        while let Some(current) = queue.pop_front() {
            let depth = depths[&current];
            if depth >= MAX_DEPTH || depths.len() >= MAX_NODES {
                continue;
            }
            for rel in self.incoming.get(&current).into_iter().flatten() {
                if !DEPENDENT_KINDS.contains(&rel.kind) {
                    continue;
                }
                if depths.contains_key(&rel.source) {
                    continue;
                }
                depths.insert(rel.source, depth + 1);
                queue.push_back(rel.source);
            }
        }
        depths
    }

    fn impacted_files(&self, origin: &Symbol, reached: &[ImpactSymbol]) -> Vec<ImpactedFile> {
        let mut grouped: HashMap<FileId, Vec<&Symbol>> = HashMap::new();
        grouped.entry(origin.file_id).or_default().push(origin);
        for item in reached {
            if let Some(symbol) = self.symbols.get(&item.id) {
                grouped.entry(symbol.file_id).or_default().push(symbol);
            }
        }

        let mut files = grouped
            .into_iter()
            .filter_map(|(file_id, symbols)| {
                let path = self.file_path(file_id)?.to_string();
                Some(ImpactedFile {
                    path: path.clone(),
                    file_id,
                    symbol_count: symbols.len() as u64,
                    roles: roles_for(&path, &symbols),
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files
    }

    fn to_impact_symbol(&self, id: SymbolId, depth: u32) -> Option<ImpactSymbol> {
        let symbol = self.symbols.get(&id)?;
        Some(ImpactSymbol {
            id: symbol.id,
            name: symbol.name.clone(),
            kind: symbol.kind.as_str().to_string(),
            file_id: symbol.file_id,
            path: self.file_path(symbol.file_id)?.to_string(),
            depth,
        })
    }

    pub(crate) fn file_path(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|file| file.path.as_str())
    }
}

fn impact_tree(origin: &Symbol, files: &[ImpactedFile], origin_path: Option<&str>) -> ImpactTreeNode {
    let origin_path = origin_path.unwrap_or("").to_string();
    let origin_file = files
        .iter()
        .find(|file| file.file_id == origin.file_id)
        .cloned()
        .unwrap_or_else(|| ImpactedFile {
            path: origin_path.clone(),
            file_id: origin.file_id,
            symbol_count: 1,
            roles: Vec::new(),
        });

    let mut children = files
        .iter()
        .filter(|file| file.file_id != origin.file_id)
        .map(|file| ImpactTreeNode {
            path: file.path.clone(),
            file_id: file.file_id,
            roles: file.roles.clone(),
            children: Vec::new(),
        })
        .collect::<Vec<_>>();
    children.sort_by(|a, b| a.path.cmp(&b.path));

    ImpactTreeNode {
        path: origin_file.path,
        file_id: origin_file.file_id,
        roles: origin_file.roles,
        children,
    }
}
