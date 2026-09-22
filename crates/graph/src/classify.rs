use codeatlas_domain::{Symbol, SymbolKind};
use serde::{Deserialize, Serialize};

use crate::contains_segment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactRole {
    Test,
    Route,
    Component,
    Other,
}

impl ImpactRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Route => "route",
            Self::Component => "component",
            Self::Other => "other",
        }
    }
}

pub fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/fixtures/") {
        return false;
    }
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    normalized.contains("/test/")
        || normalized.contains("/tests/")
        || normalized.contains("/__tests__/")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.ends_with("_test.rs")
        || name.ends_with("_test.py")
        || name.starts_with("test_")
}

pub fn is_route_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    contains_segment(&normalized, &["route", "routes", "pages", "handlers", "handler"])
        || normalized.contains("/api/")
        || matches!(
            name,
            "page.tsx" | "page.jsx" | "page.ts" | "route.ts" | "route.tsx" | "layout.tsx"
        )
}

pub fn is_component_symbol(symbol: &Symbol, path: &str) -> bool {
    symbol.kind == SymbolKind::Component
        || contains_segment(path, &["component", "components"])
        || ((path.ends_with(".tsx") || path.ends_with(".jsx"))
            && symbol
                .name
                .chars()
                .next()
                .is_some_and(char::is_uppercase))
}

pub fn roles_for(path: &str, symbols: &[&Symbol]) -> Vec<ImpactRole> {
    let mut roles = Vec::new();
    if is_test_path(path) {
        roles.push(ImpactRole::Test);
    }
    if is_route_path(path) {
        roles.push(ImpactRole::Route);
    }
    if symbols.iter().any(|symbol| is_component_symbol(symbol, path)) {
        roles.push(ImpactRole::Component);
    }
    if roles.is_empty() {
        roles.push(ImpactRole::Other);
    }
    roles
}
