//! UI-independent code graph: nodes, edges, traversal, dependencies, impact.
//!
//! `dependencies`, `dependents`, `impact`, and `related` are the public query API.

mod architecture;
mod classify;
mod impact;

use std::collections::{HashMap, HashSet, VecDeque};

use codeatlas_domain::{
    FileId, Relationship, RelationshipKind, RepositoryIndex, SourceFile, Symbol, SymbolId,
};
use serde::{Deserialize, Serialize};

pub use architecture::{
    ArchitectureGroup, ArchitectureGroupNode, ArchitectureOutline, RepoHealth, SymbolRef,
};
pub use classify::ImpactRole;
pub use impact::{ImpactReport, ImpactSymbol, ImpactTreeNode, ImpactedFile};

#[derive(Debug, Clone)]
pub struct CodeGraph {
    pub(crate) symbols: HashMap<SymbolId, Symbol>,
    pub(crate) files: HashMap<FileId, SourceFile>,
    pub(crate) outgoing: HashMap<SymbolId, Vec<Relationship>>,
    pub(crate) incoming: HashMap<SymbolId, Vec<Relationship>>,
}

impl CodeGraph {
    pub fn build(index: &RepositoryIndex) -> Self {
        let mut outgoing: HashMap<SymbolId, Vec<Relationship>> = HashMap::new();
        let mut incoming: HashMap<SymbolId, Vec<Relationship>> = HashMap::new();

        for relationship in &index.relationships {
            outgoing
                .entry(relationship.source)
                .or_default()
                .push(relationship.clone());
            incoming
                .entry(relationship.target)
                .or_default()
                .push(relationship.clone());
        }

        Self {
            symbols: index
                .symbols
                .iter()
                .map(|symbol| (symbol.id, symbol.clone()))
                .collect(),
            files: index
                .files
                .iter()
                .map(|file| (file.id, file.clone()))
                .collect(),
            outgoing,
            incoming,
        }
    }

    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    pub fn dependencies(&self, id: SymbolId) -> Vec<SymbolRef> {
        self.outgoing_refs(id)
    }

    pub fn dependents(&self, id: SymbolId) -> Vec<SymbolRef> {
        self.incoming_refs(id)
    }

    pub fn related(&self, id: SymbolId) -> Vec<SymbolRef> {
        let mut refs = self.dependents(id);
        refs.extend(self.dependencies(id));
        refs.sort_by(|a, b| a.name.cmp(&b.name));
        refs.dedup_by(|a, b| a.id == b.id && a.relation == b.relation);
        refs
    }

    pub fn neighbors(&self, id: SymbolId, kind: Option<RelationshipKind>) -> Vec<&Symbol> {
        self.outgoing
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|rel| kind.is_none_or(|wanted| rel.kind == wanted))
            .filter_map(|rel| self.symbols.get(&rel.target))
            .collect()
    }

    pub fn incoming(&self, id: SymbolId, kind: Option<RelationshipKind>) -> Vec<&Symbol> {
        self.incoming
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|rel| kind.is_none_or(|wanted| rel.kind == wanted))
            .filter_map(|rel| self.symbols.get(&rel.source))
            .collect()
    }

    pub fn architecture(&self) -> Vec<ArchitectureLayer> {
        let mut layers: HashMap<ArchitectureKind, HashSet<FileId>> = HashMap::new();
        for file in self.files.values() {
            let kind = ArchitectureKind::from_path(&file.path);
            layers.entry(kind).or_default().insert(file.id);
        }

        let mut result = layers
            .into_iter()
            .map(|(kind, files)| ArchitectureLayer {
                kind,
                file_count: files.len() as u64,
            })
            .collect::<Vec<_>>();
        result.sort_by_key(|layer| layer.kind.sort_key());
        result
    }

    pub fn view(&self) -> GraphView {
        self.view_ids(self.symbols.keys().copied())
    }

    pub fn neighborhood(&self, origin: SymbolId, depth: u8) -> GraphView {
        let mut ids = HashSet::new();
        let mut queue = VecDeque::new();
        ids.insert(origin);
        queue.push_back((origin, 0u8));

        while let Some((current, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }
            for rel in self.outgoing.get(&current).into_iter().flatten() {
                if ids.insert(rel.target) {
                    queue.push_back((rel.target, current_depth + 1));
                }
            }
            for rel in self.incoming.get(&current).into_iter().flatten() {
                if ids.insert(rel.source) {
                    queue.push_back((rel.source, current_depth + 1));
                }
            }
        }

        self.layout(self.view_ids(ids.into_iter()), Some(origin))
    }

    pub fn compact_view(&self, limit: usize) -> GraphView {
        let mut ranked = self
            .symbols
            .keys()
            .map(|id| {
                let degree = self.outgoing.get(id).map(Vec::len).unwrap_or(0)
                    + self.incoming.get(id).map(Vec::len).unwrap_or(0);
                (*id, degree)
            })
            .filter(|(_, degree)| *degree > 0)
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let focus = ranked.first().map(|(id, _)| *id);
        let ids = ranked.into_iter().take(limit).map(|(id, _)| id);
        self.layout(self.view_ids(ids), focus)
    }

    fn view_ids(&self, ids: impl IntoIterator<Item = SymbolId>) -> GraphView {
        let ids = ids.into_iter().collect::<HashSet<_>>();
        let nodes = ids
            .iter()
            .filter_map(|id| self.symbols.get(id))
            .map(|symbol| GraphNode::from_symbol(symbol, self.file_path(symbol.file_id)))
            .collect();
        let edges = self
            .outgoing
            .values()
            .flatten()
            .filter(|rel| ids.contains(&rel.source) && ids.contains(&rel.target))
            .map(GraphEdge::from_relationship)
            .collect();
        GraphView { nodes, edges }
    }

    fn layout(&self, mut view: GraphView, focus: Option<SymbolId>) -> GraphView {
        let Some(focus) = focus.or_else(|| view.nodes.first().map(|node| node.id)) else {
            return view;
        };

        let mut layers: HashMap<SymbolId, i32> = HashMap::new();
        let mut queue = VecDeque::new();
        layers.insert(focus, 0);
        queue.push_back(focus);

        while let Some(current) = queue.pop_front() {
            let layer = layers[&current];
            for rel in self.outgoing.get(&current).into_iter().flatten() {
                if layers.contains_key(&rel.target) {
                    continue;
                }
                if view.nodes.iter().any(|node| node.id == rel.target) {
                    layers.insert(rel.target, layer + 1);
                    queue.push_back(rel.target);
                }
            }
            for rel in self.incoming.get(&current).into_iter().flatten() {
                if layers.contains_key(&rel.source) {
                    continue;
                }
                if view.nodes.iter().any(|node| node.id == rel.source) {
                    layers.insert(rel.source, layer - 1);
                    queue.push_back(rel.source);
                }
            }
        }

        let mut by_layer: HashMap<i32, Vec<SymbolId>> = HashMap::new();
        for node in &view.nodes {
            by_layer
                .entry(*layers.get(&node.id).unwrap_or(&0))
                .or_default()
                .push(node.id);
        }
        for ids in by_layer.values_mut() {
            ids.sort_by_key(|id| {
                self.symbols
                    .get(id)
                    .map(|symbol| symbol.name.clone())
                    .unwrap_or_default()
            });
        }

        const X_GAP: f64 = 188.0;
        const Y_GAP: f64 = 118.0;
        for node in &mut view.nodes {
            let layer = *layers.get(&node.id).unwrap_or(&0);
            let row = by_layer.get(&layer).map(Vec::as_slice).unwrap_or(&[]);
            let index = row.iter().position(|id| *id == node.id).unwrap_or(0);
            let width = row.len().max(1) as f64 * X_GAP;
            node.layer = layer;
            node.x = (index as f64 * X_GAP) - width / 2.0 + X_GAP / 2.0;
            node.y = f64::from(layer) * Y_GAP;
        }
        view
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchitectureKind {
    Api,
    Services,
    Repositories,
    Database,
    Workers,
    Components,
    Hooks,
    Pages,
    External,
    Other,
}

impl ArchitectureKind {
    pub fn from_path(path: &str) -> Self {
        let lower = path.replace('\\', "/").to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        if contains_segment(&lower, &["page", "pages"])
            || matches!(name, "page.tsx" | "page.jsx" | "page.ts")
        {
            Self::Pages
        } else if contains_segment(
            &lower,
            &["api", "handler", "handlers", "routes", "http", "controller", "controllers"],
        ) {
            Self::Api
        } else if contains_segment(&lower, &["service", "services"]) {
            Self::Services
        } else if contains_segment(&lower, &["repository", "repositories"]) || name.contains("repository")
        {
            Self::Repositories
        } else if contains_segment(&lower, &["db", "database", "storage"]) {
            Self::Database
        } else if contains_segment(&lower, &["worker", "workers", "job", "jobs", "queue"]) {
            Self::Workers
        } else if contains_segment(&lower, &["component", "components"]) {
            Self::Components
        } else if contains_segment(&lower, &["hook", "hooks"]) {
            Self::Hooks
        } else if contains_segment(&lower, &["client", "clients", "external", "integration", "integrations"])
        {
            Self::External
        } else {
            Self::Other
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::Api => 0,
            Self::Services => 1,
            Self::Repositories => 2,
            Self::Database => 3,
            Self::Workers => 4,
            Self::Components => 5,
            Self::Hooks => 6,
            Self::Pages => 7,
            Self::External => 8,
            Self::Other => 9,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Services => "services",
            Self::Repositories => "repositories",
            Self::Database => "database",
            Self::Workers => "workers",
            Self::Components => "components",
            Self::Hooks => "hooks",
            Self::Pages => "pages",
            Self::External => "external",
            Self::Other => "other",
        }
    }

    pub fn group(self) -> ArchitectureGroup {
        match self {
            Self::Components | Self::Hooks | Self::Pages => ArchitectureGroup::Frontend,
            Self::Api | Self::Services | Self::Repositories => ArchitectureGroup::Backend,
            Self::Database | Self::Workers | Self::External => ArchitectureGroup::Infrastructure,
            Self::Other => ArchitectureGroup::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureLayer {
    pub kind: ArchitectureKind,
    pub file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphView {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: SymbolId,
    pub name: String,
    pub kind: String,
    pub file_id: FileId,
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub layer: i32,
}

impl GraphNode {
    fn from_symbol(symbol: &Symbol, path: Option<&str>) -> Self {
        Self {
            id: symbol.id,
            name: symbol.name.clone(),
            kind: symbol.kind.as_str().to_string(),
            file_id: symbol.file_id,
            path: path.unwrap_or("").to_string(),
            x: 0.0,
            y: 0.0,
            layer: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: SymbolId,
    pub target: SymbolId,
    pub kind: String,
}

impl GraphEdge {
    fn from_relationship(rel: &Relationship) -> Self {
        Self {
            source: rel.source,
            target: rel.target,
            kind: rel.kind.as_str().to_string(),
        }
    }
}

pub(crate) fn contains_segment(path: &str, needles: &[&str]) -> bool {
    path.split(|c| c == '/' || c == '\\' || c == '.' || c == '-' || c == '_')
        .any(|segment| needles.contains(&segment))
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        Language, Relationship, RelationshipKind, Repository, RepositoryIndex, SourceFile, Symbol,
        SymbolKind,
    };

    use super::*;

    struct AuthGraph {
        index: RepositoryIndex,
        login: SymbolId,
        use_auth: SymbolId,
        provider: SymbolId,
        dashboard: SymbolId,
        test: SymbolId,
        route: SymbolId,
        repository: SymbolId,
    }

    fn auth_graph() -> AuthGraph {
        let repo = Repository::new("demo", "https://example.com/demo", "main", "sha");
        let service_file = SourceFile::new(
            repo.id,
            "src/services/AuthService.ts",
            Language::TypeScript,
            40,
            "s",
        );
        let hook_file = SourceFile::new(repo.id, "src/hooks/useAuth.ts", Language::TypeScript, 20, "h");
        let provider_file = SourceFile::new(
            repo.id,
            "src/components/AuthProvider.tsx",
            Language::TypeScript,
            30,
            "p",
        );
        let dashboard_file = SourceFile::new(
            repo.id,
            "src/components/Dashboard.tsx",
            Language::TypeScript,
            24,
            "d",
        );
        let test_file = SourceFile::new(
            repo.id,
            "src/services/AuthService.test.ts",
            Language::TypeScript,
            16,
            "t",
        );
        let route_file = SourceFile::new(repo.id, "src/api/session.ts", Language::TypeScript, 12, "r");
        let repo_file = SourceFile::new(repo.id, "src/db/UserRepository.ts", Language::TypeScript, 18, "u");

        let login = Symbol::new(service_file.id, "login", SymbolKind::Method, 12, 20);
        let use_auth = Symbol::new(hook_file.id, "useAuth", SymbolKind::Function, 1, 8);
        let provider = Symbol::new(provider_file.id, "AuthProvider", SymbolKind::Component, 1, 30);
        let dashboard = Symbol::new(dashboard_file.id, "Dashboard", SymbolKind::Component, 1, 20);
        let test = Symbol::new(test_file.id, "logs_in_a_user", SymbolKind::Function, 1, 10);
        let route = Symbol::new(route_file.id, "sessionHandler", SymbolKind::Function, 1, 8);
        let repository = Symbol::new(repo_file.id, "findUser", SymbolKind::Method, 4, 8);

        AuthGraph {
            login: login.id,
            use_auth: use_auth.id,
            provider: provider.id,
            dashboard: dashboard.id,
            test: test.id,
            route: route.id,
            repository: repository.id,
            index: RepositoryIndex {
                repository: repo,
                files: vec![
                    service_file,
                    hook_file,
                    provider_file,
                    dashboard_file,
                    test_file,
                    route_file,
                    repo_file,
                ],
                symbols: vec![
                    login.clone(),
                    use_auth.clone(),
                    provider.clone(),
                    dashboard.clone(),
                    test.clone(),
                    route.clone(),
                    repository.clone(),
                ],
                relationships: vec![
                    Relationship::new(use_auth.id, login.id, RelationshipKind::Calls),
                    Relationship::new(provider.id, use_auth.id, RelationshipKind::Calls),
                    Relationship::new(dashboard.id, provider.id, RelationshipKind::Imports),
                    Relationship::new(test.id, login.id, RelationshipKind::Calls),
                    Relationship::new(route.id, login.id, RelationshipKind::Calls),
                    Relationship::new(login.id, repository.id, RelationshipKind::Calls),
                ],
                units: vec![],
                line_count: 160,
            },
        }
    }

    #[test]
    fn neighbors_follow_relationship_kind() {
        let fixture = auth_graph();
        let graph = CodeGraph::build(&fixture.index);
        let calls = graph.neighbors(fixture.use_auth, Some(RelationshipKind::Calls));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "login");
        let deps = graph.dependencies(fixture.use_auth);
        assert!(deps.iter().any(|symbol| symbol.name == "login"));
        let dependents = graph.dependents(fixture.login);
        assert!(dependents.iter().any(|symbol| symbol.name == "useAuth"));
        let related = graph.related(fixture.login);
        assert!(related.iter().any(|symbol| symbol.name == "useAuth"));
        assert!(related.iter().any(|symbol| symbol.name == "findUser"));
    }

    #[test]
    fn architecture_clusters_files_by_path() {
        let fixture = auth_graph();
        let graph = CodeGraph::build(&fixture.index);
        let layers = graph.architecture();
        assert!(layers.iter().any(|layer| layer.kind == ArchitectureKind::Api));
        assert!(layers
            .iter()
            .any(|layer| layer.kind == ArchitectureKind::Services));
        let outline = graph.architecture_outline();
        assert!(outline
            .groups
            .iter()
            .any(|group| group.group == crate::ArchitectureGroup::Frontend));
        assert!(outline
            .groups
            .iter()
            .any(|group| group.group == crate::ArchitectureGroup::Backend));
        assert!(outline.summary.contains("Inferred from paths"));
        let health = graph.health(&fixture.index);
        assert_eq!(health.file_count, 7);
        assert_eq!(health.language_count, 1);
        assert!(health.api_endpoint_count >= 1);
        assert!(health.test_file_count >= 1);
        assert!((0.0..=1.0).contains(&health.architecture_mapped));
        assert!((0.0..=1.0).contains(&health.complexity));
        assert!((0.0..=1.0).contains(&health.test_presence));
        assert!(health.test_presence > 0.0);
    }

    #[test]
    fn impact_walks_direct_and_indirect_dependents() {
        let fixture = auth_graph();
        let graph = CodeGraph::build(&fixture.index);
        let impact = graph.impact(fixture.login).expect("impact");

        assert_eq!(impact.origin.name, "login");
        assert!(impact.direct.iter().any(|symbol| symbol.id == fixture.use_auth));
        assert!(impact.direct.iter().any(|symbol| symbol.id == fixture.test));
        assert!(impact.direct.iter().any(|symbol| symbol.id == fixture.route));
        assert!(impact.indirect.iter().any(|symbol| symbol.id == fixture.provider));
        assert!(impact.indirect.iter().any(|symbol| symbol.id == fixture.dashboard));
        assert!(!impact.direct.iter().any(|symbol| symbol.id == fixture.repository));
        assert!(impact.tests.iter().any(|symbol| symbol.id == fixture.test));
        assert!(impact.routes.iter().any(|symbol| symbol.id == fixture.route));
        assert!(impact.components.iter().any(|symbol| symbol.id == fixture.provider));
        assert!(impact.file_count >= 6);
        assert_eq!(impact.tree.path, "src/services/AuthService.ts");
        assert!(impact
            .tree
            .children
            .iter()
            .any(|child| child.path.ends_with("AuthProvider.tsx")));
    }

    #[test]
    fn neighborhood_includes_dependents_and_dependencies() {
        let fixture = auth_graph();
        let graph = CodeGraph::build(&fixture.index);
        let view = graph.neighborhood(fixture.login, 2);
        let ids = view.nodes.iter().map(|node| node.id).collect::<HashSet<_>>();
        assert!(ids.contains(&fixture.login));
        assert!(ids.contains(&fixture.use_auth));
        assert!(ids.contains(&fixture.repository));
        assert!(ids.contains(&fixture.provider));
        assert!(view.nodes.iter().any(|node| node.id == fixture.login && node.layer == 0));
        assert!(view.nodes.iter().any(|node| node.id == fixture.repository && node.y > 0.0));
        assert!(view.nodes.iter().any(|node| node.id == fixture.use_auth && node.y < 0.0));
    }
}
