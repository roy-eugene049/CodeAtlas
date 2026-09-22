use codeatlas_domain::{RepositoryIndex, SemanticUnit, Symbol, SymbolId, SymbolKind};
use codeatlas_graph::{CodeGraph, ImpactReport, SymbolRef};
use serde::{Deserialize, Serialize};

use crate::answer::{AnswerMode, Citation};
use crate::embed::tokenize;
use crate::error::AiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolExplanation {
    pub purpose: String,
    pub flow: Vec<String>,
    pub dependencies: Vec<SymbolRef>,
    pub dependents: Vec<SymbolRef>,
    pub citations: Vec<Citation>,
    pub mode: AnswerMode,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactNarrative {
    pub text: String,
    pub citations: Vec<Citation>,
    pub mode: AnswerMode,
}

pub fn explain_symbol(
    symbol_id: SymbolId,
    index: &RepositoryIndex,
) -> Result<SymbolExplanation, AiError> {
    let symbol = index
        .symbols
        .iter()
        .find(|symbol| symbol.id == symbol_id)
        .ok_or(AiError::EmptyContext)?;
    let unit = index
        .units
        .iter()
        .find(|unit| unit.symbol_id == symbol_id);
    let graph = CodeGraph::build(index);
    let dependencies = graph.outgoing_refs(symbol_id);
    let dependents = graph.incoming_refs(symbol_id);
    let text = unit.map(|unit| unit.text.as_str()).unwrap_or("");
    let purpose = purpose_of(symbol, text);
    let flow = flow_of(text, &dependencies);
    let citations = unit
        .map(|unit| vec![citation_from_unit(unit)])
        .unwrap_or_default();
    let rendered = render_explanation(symbol, &purpose, &flow, &dependencies);
    Ok(SymbolExplanation {
        purpose,
        flow,
        dependencies,
        dependents,
        citations,
        mode: AnswerMode::Retrieved,
        text: rendered,
    })
}

pub fn explain_impact(report: &ImpactReport, index: &RepositoryIndex) -> ImpactNarrative {
    let mut text = format!(
        "Changing {}() can affect {} symbols across {} files.\n\n- {} API endpoint{}\n- {} UI component{}\n- {} test suite{}\n",
        report.origin.name,
        report.symbol_count,
        report.file_count,
        report.routes.len(),
        plural(report.routes.len()),
        report.components.len(),
        plural(report.components.len()),
        report.test_file_count,
        plural(report.test_file_count as usize)
    );
    if !report.direct.is_empty() {
        text.push_str("\nDirect dependents:\n");
        for symbol in report.direct.iter().take(8) {
            text.push_str(&format!("→ {} ({})\n", symbol.name, symbol.path));
        }
    }
    if !report.routes.is_empty() {
        text.push_str("\nAPI endpoints that sit on this path:\n");
        for symbol in report.routes.iter().take(6) {
            text.push_str(&format!("→ {} ({})\n", symbol.name, symbol.path));
        }
    }
    if !report.tests.is_empty() {
        text.push_str("\nTests that exercise this symbol:\n");
        for symbol in report.tests.iter().take(6) {
            text.push_str(&format!("→ {} ({})\n", symbol.name, symbol.path));
        }
    }

    let mut citations = Vec::new();
    if let Some(unit) = index
        .units
        .iter()
        .find(|unit| unit.symbol_id == report.origin.id)
    {
        citations.push(citation_from_unit(unit));
    }
    for symbol in report
        .direct
        .iter()
        .chain(report.routes.iter())
        .chain(report.components.iter())
        .take(6)
    {
        if let Some(unit) = index.units.iter().find(|unit| unit.symbol_id == symbol.id) {
            citations.push(citation_from_unit(unit));
        }
    }

    ImpactNarrative {
        text,
        citations,
        mode: AnswerMode::Retrieved,
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn purpose_of(symbol: &Symbol, text: &str) -> String {
    if let Some(comment) = leading_comment(text) {
        return comment;
    }
    let words = tokenize(&symbol.name);
    if words.is_empty() {
        return format!("The {} {}.", symbol.kind, symbol.name);
    }
    match symbol.kind {
        SymbolKind::Function | SymbolKind::Method => {
            let verb = inflect_verb(&words[0]);
            let rest = words[1..].join(" ");
            if rest.is_empty() {
                format!("{verb} this unit of work.")
            } else {
                format!("{verb} a {rest}.")
            }
        }
        SymbolKind::Class | SymbolKind::Struct | SymbolKind::Component => {
            format!("Defines {}.", symbol.name)
        }
        _ => format!("The {} {}.", symbol.kind, symbol.name),
    }
}

fn flow_of(text: &str, dependencies: &[SymbolRef]) -> Vec<String> {
    let mut steps = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(comment) = trimmed
            .strip_prefix("//")
            .or_else(|| trimmed.strip_prefix("#"))
            .or_else(|| trimmed.strip_prefix("*"))
        {
            let comment = comment.trim().trim_start_matches('/').trim();
            if comment.len() > 3 {
                steps.push(comment.to_string());
            }
        }
    }
    if steps.is_empty() {
        for dep in dependencies.iter().filter(|dep| dep.relation == "calls") {
            steps.push(format!("Calls {}", dep.name));
        }
    }
    steps.into_iter().take(8).collect()
}

fn leading_comment(text: &str) -> Option<String> {
    for line in text.lines().take(6) {
        let trimmed = line.trim();
        if trimmed.starts_with("export ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("def ")
            || trimmed.contains('(')
        {
            break;
        }
        if let Some(comment) = trimmed.strip_prefix("//").or_else(|| trimmed.strip_prefix("#"))
        {
            let comment = comment.trim();
            if comment.len() > 3 {
                return Some(comment.to_string());
            }
        }
    }
    None
}

fn inflect_verb(token: &str) -> String {
    let word = if token.is_empty() {
        return "Runs".into();
    } else {
        let mut chars = token.chars();
        match chars.next() {
            Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
            None => return "Runs".into(),
        }
    };
    if word.ends_with('s') || word.ends_with('x') || word.ends_with("ch") {
        format!("{word}es")
    } else if word.ends_with('y') && word.len() > 1 {
        format!("{}ies", &word[..word.len() - 1])
    } else if word.ends_with('e') {
        format!("{word}s")
    } else {
        format!("{word}s")
    }
}

fn render_explanation(
    symbol: &Symbol,
    purpose: &str,
    flow: &[String],
    dependencies: &[SymbolRef],
) -> String {
    let mut out = format!("Purpose\n{purpose}\n");
    if !flow.is_empty() {
        out.push_str("\nFlow\n");
        for (index, step) in flow.iter().enumerate() {
            out.push_str(&format!("{}. {step}\n", index + 1));
        }
    }
    if !dependencies.is_empty() {
        out.push_str("\nDependencies\n");
        for dep in dependencies {
            out.push_str(&format!("→ {}\n", dep.name));
        }
    }
    let _ = symbol;
    out
}

fn citation_from_unit(unit: &SemanticUnit) -> Citation {
    Citation::from_unit(unit)
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        Language, Relationship, RelationshipKind, Repository, RepositoryIndex, SemanticUnit,
        SourceFile, Symbol, SymbolKind,
    };
    use codeatlas_graph::{ImpactReport, ImpactSymbol, ImpactTreeNode};

    use super::*;

    #[test]
    fn explain_lists_purpose_flow_and_linked_dependencies() {
        let repo = Repository::new("pay", "u", "main", "sha");
        let file = SourceFile::new(repo.id, "src/services/payment.ts", Language::TypeScript, 40, "h");
        let handle = Symbol::new(file.id, "handlePayment", SymbolKind::Function, 1, 12);
        let stripe = Symbol::new(file.id, "StripeClient", SymbolKind::Class, 20, 24);
        let unit = SemanticUnit {
            symbol_id: handle.id,
            repository_id: repo.id,
            file_id: file.id,
            name: handle.name.clone(),
            kind: handle.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: 1,
            end_line: 12,
            text: "export function handlePayment() {\n  // Validates payment details\n  // Calls Stripe\n  charge();\n  // Persists transaction\n}".into(),
        };
        let index = RepositoryIndex {
            repository: repo,
            files: vec![file],
            symbols: vec![handle.clone(), stripe.clone()],
            relationships: vec![Relationship::new(
                handle.id,
                stripe.id,
                RelationshipKind::Calls,
            )],
            units: vec![unit],
            line_count: 12,
        };
        let explanation = explain_symbol(handle.id, &index).expect("explain");
        assert!(explanation.purpose.to_ascii_lowercase().contains("payment"));
        assert!(explanation.flow.iter().any(|step| step.contains("Stripe")));
        assert!(explanation
            .dependencies
            .iter()
            .any(|dep| dep.name == "StripeClient" && dep.id == stripe.id));
        assert!(explanation.text.contains("Purpose"));
        assert!(explanation.text.contains("Dependencies"));
    }

    #[test]
    fn impact_narrative_uses_counted_graph_results() {
        let origin = ImpactSymbol {
            id: SymbolId::new(),
            name: "updateUser".into(),
            kind: "method".into(),
            file_id: codeatlas_domain::FileId::new(),
            path: "src/services/user.ts".into(),
            depth: 0,
        };
        let route = ImpactSymbol {
            id: SymbolId::new(),
            name: "userHandler".into(),
            kind: "function".into(),
            file_id: codeatlas_domain::FileId::new(),
            path: "src/api/users.ts".into(),
            depth: 1,
        };
        let report = ImpactReport {
            origin: origin.clone(),
            file_count: 8,
            symbol_count: 17,
            direct: vec![route.clone()],
            indirect: vec![],
            tests: vec![],
            test_file_count: 2,
            routes: vec![route],
            components: vec![origin.clone(), origin.clone(), origin.clone(), origin],
            files: vec![],
            tree: ImpactTreeNode {
                path: "src/services/user.ts".into(),
                file_id: codeatlas_domain::FileId::new(),
                roles: vec![],
                children: vec![],
            },
            subgraph: codeatlas_graph::GraphView {
                nodes: vec![],
                edges: vec![],
            },
        };
        let index = RepositoryIndex {
            repository: Repository::new("d", "u", "main", "sha"),
            files: vec![],
            symbols: vec![],
            relationships: vec![],
            units: vec![],
            line_count: 0,
        };
        let narrative = explain_impact(&report, &index);
        assert!(narrative.text.contains("17 symbols"));
        assert!(narrative.text.contains("8 files"));
        assert!(narrative.text.contains("1 API"));
        assert!(narrative.text.contains("2 test suites"));
    }
}
