use codeatlas_domain::{Language, RelationshipKind, SymbolKind};
use tree_sitter::{Node, Parser};

use crate::error::ParseError;
use crate::parsed::{ExtractedCall, ExtractedHeritage, ExtractedImport, ExtractedSymbol, ParsedFile};
use crate::walk::{ancestor_of_kind, field, line_range, named_field, text, walk_preorder};
use crate::LanguageParser;

pub struct PythonParser;

impl LanguageParser for PythonParser {
    fn language(&self) -> Language {
        Language::Python
    }

    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|error| ParseError::Language(error.to_string()))?;
        let tree = parser.parse(source, None).ok_or(ParseError::ParseFailed)?;
        Ok(extract(tree.root_node(), source))
    }
}

fn extract(root: Node<'_>, source: &str) -> ParsedFile {
    let mut out = ParsedFile::default();
    walk_preorder(root, &mut |node| match node.kind() {
        "function_definition" => push_function(node, source, &mut out),
        "class_definition" => push_class(node, source, &mut out),
        "import_statement" => push_import(node, source, &mut out),
        "import_from_statement" => push_import_from(node, source, &mut out),
        "call" => push_call(node, source, &mut out),
        "assignment" => push_constant(node, source, &mut out),
        _ => {}
    });
    out
}

fn push_function(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let kind = if ancestor_of_kind(node, &["class_definition"]).is_some() {
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
        container_name: ancestor_of_kind(node, &["class_definition"])
            .and_then(|class| named_field(class, source)),
    });
}

fn push_class(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name: name.clone(),
        kind: SymbolKind::Class,
        start_line,
        end_line,
        container_name: None,
    });

    if let Some(superclasses) = field(node, "superclasses") {
        let mut cursor = superclasses.walk();
        for child in superclasses.named_children(&mut cursor) {
            let parent = match child.kind() {
                "identifier" => text(child, source).to_string(),
                "attribute" => field(child, "attribute")
                    .map(|attr| text(attr, source).to_string())
                    .unwrap_or_else(|| text(child, source).to_string()),
                _ => continue,
            };
            out.heritage.push(ExtractedHeritage {
                child: name.clone(),
                parent,
                kind: RelationshipKind::Extends,
            });
        }
    }
}

fn push_import(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let (module, name) = match child.kind() {
            "dotted_name" => {
                let raw = text(child, source);
                (raw.to_string(), raw.rsplit('.').next().unwrap_or(raw).to_string())
            }
            "aliased_import" => {
                let module = field(child, "name")
                    .map(|name| text(name, source).to_string())
                    .unwrap_or_default();
                let alias = field(child, "alias")
                    .map(|alias| text(alias, source).to_string())
                    .unwrap_or_else(|| {
                        module.rsplit('.').next().unwrap_or(&module).to_string()
                    });
                (module, alias)
            }
            _ => continue,
        };
        if !module.is_empty() {
            out.imports.push(ExtractedImport {
                module,
                names: vec![name],
            });
        }
    }
}

fn push_import_from(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let module = field(node, "module_name")
        .map(|module| text(module, source).to_string())
        .unwrap_or_default();
    if module.is_empty() {
        return;
    }

    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "dotted_name" if Some(child) != field(node, "module_name") => {
                names.push(text(child, source).to_string());
            }
            "aliased_import" => {
                if let Some(alias) = field(child, "alias").or_else(|| field(child, "name")) {
                    names.push(text(alias, source).to_string());
                }
            }
            _ => {}
        }
    }

    out.imports.push(ExtractedImport { module, names });
}

fn push_call(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(func) = field(node, "function") else {
        return;
    };
    let name = match func.kind() {
        "identifier" => text(func, source).to_string(),
        "attribute" => field(func, "attribute")
            .map(|attr| text(attr, source).to_string())
            .unwrap_or_default(),
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

fn push_constant(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    if ancestor_of_kind(node, &["function_definition", "class_definition"]).is_some() {
        return;
    }
    let Some(left) = field(node, "left") else {
        return;
    };
    if left.kind() != "identifier" {
        return;
    }
    let name = text(left, source);
    if !is_constant_name(name) {
        return;
    }
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name: name.to_string(),
        kind: SymbolKind::Constant,
        start_line,
        end_line,
        container_name: None,
    });
}

fn is_constant_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        && name.chars().any(|c| c.is_ascii_uppercase())
}
