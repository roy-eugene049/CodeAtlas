use codeatlas_domain::{Language, RelationshipKind, SymbolKind};
use tree_sitter::{Node, Parser};

use crate::error::ParseError;
use crate::parsed::{ExtractedCall, ExtractedHeritage, ExtractedImport, ExtractedSymbol, ParsedFile};
use crate::walk::{ancestor_of_kind, field, line_range, named_field, text, walk_preorder};
use crate::LanguageParser;

pub struct RustParser;

impl LanguageParser for RustParser {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|error| ParseError::Language(error.to_string()))?;
        let tree = parser.parse(source, None).ok_or(ParseError::ParseFailed)?;
        Ok(extract(tree.root_node(), source))
    }
}

fn extract(root: Node<'_>, source: &str) -> ParsedFile {
    let mut out = ParsedFile::default();
    walk_preorder(root, &mut |node| match node.kind() {
        "function_item" => push_function(node, source, &mut out),
        "struct_item" => push_named(node, source, SymbolKind::Struct, &mut out),
        "enum_item" => push_named(node, source, SymbolKind::Enum, &mut out),
        "trait_item" => push_named(node, source, SymbolKind::Trait, &mut out),
        "const_item" | "static_item" => push_named(node, source, SymbolKind::Constant, &mut out),
        "use_declaration" => push_use(node, source, &mut out),
        "call_expression" => push_call(node, source, &mut out),
        "impl_item" => push_impl(node, source, &mut out),
        _ => {}
    });
    out
}

fn push_named(node: Node<'_>, source: &str, kind: SymbolKind, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name,
        kind,
        start_line,
        end_line,
        container_name: container_name(node, source),
    });
}

fn push_function(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let kind = if ancestor_of_kind(node, &["impl_item", "trait_item"]).is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name,
        kind,
        start_line,
        end_line,
        container_name: container_name(node, source),
    });
}

fn container_name(node: Node<'_>, source: &str) -> Option<String> {
    if let Some(impl_node) = ancestor_of_kind(node, &["impl_item"]) {
        return field(impl_node, "type").map(|ty| type_name(ty, source));
    }
    if let Some(trait_node) = ancestor_of_kind(node, &["trait_item"]) {
        return named_field(trait_node, source);
    }
    None
}

fn type_name(node: Node<'_>, source: &str) -> String {
    if let Some(name) = named_field(node, source) {
        return name;
    }
    match node.kind() {
        "generic_type" | "scoped_type_identifier" => field(node, "type")
            .or_else(|| field(node, "name"))
            .map(|inner| type_name(inner, source))
            .unwrap_or_else(|| text(node, source).to_string()),
        _ => text(node, source).to_string(),
    }
}

fn push_use(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let raw = text(node, source)
        .trim()
        .trim_start_matches("pub use ")
        .trim_start_matches("use ")
        .trim_end_matches(';')
        .trim();

    if let Some((module, rest)) = raw.split_once("::{") {
        let names = rest
            .trim_end_matches('}')
            .split(',')
            .filter_map(import_name)
            .collect::<Vec<_>>();
        if !names.is_empty() {
            out.imports.push(ExtractedImport {
                module: module.to_string(),
                names,
            });
        }
        return;
    }

    let (module, name) = match raw.rsplit_once("::") {
        Some((module, name)) => (module.to_string(), name.to_string()),
        None => (raw.to_string(), raw.to_string()),
    };
    if let Some(name) = import_name(&name) {
        out.imports.push(ExtractedImport {
            module,
            names: vec![name],
        });
    }
}

fn import_name(raw: &str) -> Option<String> {
    let name = raw.split(" as ").next().unwrap_or(raw).trim();
    if name.is_empty() || name == "self" || name == "*" {
        None
    } else {
        Some(name.to_string())
    }
}

fn push_call(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(func) = field(node, "function") else {
        return;
    };
    let name = match func.kind() {
        "identifier" => text(func, source).to_string(),
        "field_expression" => field(func, "field")
            .map(|field| text(field, source).to_string())
            .unwrap_or_default(),
        "scoped_identifier" => field(func, "name")
            .map(|name| text(name, source).to_string())
            .unwrap_or_else(|| text(func, source).to_string()),
        _ => return,
    };
    if name.is_empty() {
        return;
    }
    out.calls.push(ExtractedCall {
        callee: name,
        line: node.start_position().row as u32 + 1,
    });
}

fn push_impl(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(ty) = field(node, "type") else {
        return;
    };
    let Some(trait_node) = field(node, "trait") else {
        return;
    };
    out.heritage.push(ExtractedHeritage {
        child: type_name(ty, source),
        parent: type_name(trait_node, source),
        kind: RelationshipKind::Implements,
    });
}
