use codeatlas_domain::{RelationshipKind, SymbolKind};
use tree_sitter::Node;

use crate::parsed::{ExtractedCall, ExtractedHeritage, ExtractedImport, ExtractedSymbol, ParsedFile};
use crate::walk::{
    ancestor_of_kind, contains_kind, field, is_pascal_case, line_range, named_field, text, walk_preorder,
};

const JSX_KINDS: &[&str] = &["jsx_element", "jsx_self_closing_element", "jsx_fragment"];

pub fn extract_ecma(root: Node<'_>, source: &str, jsx_file: bool) -> ParsedFile {
    let mut out = ParsedFile::default();
    walk_preorder(root, &mut |node| match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            push_function(node, source, jsx_file, &mut out);
        }
        "method_definition" => push_method(node, source, &mut out),
        "class_declaration" | "abstract_class_declaration" => {
            push_class(node, source, jsx_file, &mut out);
        }
        "interface_declaration" => push_named(node, source, SymbolKind::Interface, &mut out),
        "enum_declaration" => push_named(node, source, SymbolKind::Enum, &mut out),
        "lexical_declaration" | "variable_declaration" => {
            push_declarators(node, source, jsx_file, &mut out);
        }
        "import_statement" => push_import(node, source, &mut out),
        "call_expression" => push_call(node, source, &mut out),
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

fn push_function(node: Node<'_>, source: &str, jsx_file: bool, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let body = field(node, "body").unwrap_or(node);
    let kind = function_kind(&name, body, jsx_file);
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name,
        kind,
        start_line,
        end_line,
        container_name: container_name(node, source),
    });
}

fn push_method(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(name_node) = field(node, "name") else {
        return;
    };
    let name = text(name_node, source);
    if name == "constructor" {
        return;
    }
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name: name.to_string(),
        kind: SymbolKind::Method,
        start_line,
        end_line,
        container_name: container_name(node, source),
    });
}

fn push_class(node: Node<'_>, source: &str, jsx_file: bool, out: &mut ParsedFile) {
    let Some(name) = named_field(node, source) else {
        return;
    };
    let kind = if is_component_class(node, source, jsx_file, &name) {
        SymbolKind::Component
    } else {
        SymbolKind::Class
    };
    let (start_line, end_line) = line_range(node);
    out.symbols.push(ExtractedSymbol {
        name: name.clone(),
        kind,
        start_line,
        end_line,
        container_name: None,
    });
    push_heritage(&name, node, source, out);
}

fn push_heritage(child: &str, node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let mut cursor = node.walk();
    for child_node in node.children(&mut cursor) {
        match child_node.kind() {
            "class_heritage" => {
                if let Some(parent) = heritage_name(child_node, source) {
                    out.heritage.push(ExtractedHeritage {
                        child: child.to_string(),
                        parent,
                        kind: RelationshipKind::Extends,
                    });
                }
            }
            "implements_clause" => {
                let mut impl_cursor = child_node.walk();
                for impl_node in child_node.children(&mut impl_cursor) {
                    if matches!(
                        impl_node.kind(),
                        "type_identifier" | "identifier" | "nested_type_identifier" | "generic_type"
                    ) {
                        out.heritage.push(ExtractedHeritage {
                            child: child.to_string(),
                            parent: type_name(impl_node, source),
                            kind: RelationshipKind::Implements,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn heritage_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        return Some(type_name(child, source));
    }
    None
}

fn type_name(node: Node<'_>, source: &str) -> String {
    match node.kind() {
        "generic_type" => field(node, "name")
            .map(|inner| type_name(inner, source))
            .unwrap_or_else(|| text(node, source).to_string()),
        "member_expression" | "nested_type_identifier" => field(node, "property")
            .or_else(|| field(node, "name"))
            .map(|inner| text(inner, source).to_string())
            .unwrap_or_else(|| text(node, source).to_string()),
        _ => text(node, source).to_string(),
    }
}

fn push_declarators(node: Node<'_>, source: &str, jsx_file: bool, out: &mut ParsedFile) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }
        let Some(name) = named_field(child, source) else {
            continue;
        };
        let value = field(child, "value");
        let kind = match value {
            Some(value) if is_function_value(value) => function_kind(&name, value, jsx_file),
            Some(_) if is_constant_name(&name) => SymbolKind::Constant,
            _ => SymbolKind::Variable,
        };
        let (start_line, end_line) = line_range(child);
        out.symbols.push(ExtractedSymbol {
            name,
            kind,
            start_line,
            end_line,
            container_name: container_name(child, source),
        });
    }
}

fn is_function_value(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "generator_function"
    ) || (node.kind() == "call_expression" && contains_kind(node, &["arrow_function", "function_expression"]))
}

fn function_kind(name: &str, body: Node<'_>, jsx_file: bool) -> SymbolKind {
    if is_pascal_case(name) && (jsx_file || contains_kind(body, JSX_KINDS)) {
        SymbolKind::Component
    } else {
        SymbolKind::Function
    }
}

fn is_component_class(node: Node<'_>, source: &str, jsx_file: bool, name: &str) -> bool {
    if !is_pascal_case(name) {
        return false;
    }
    if jsx_file {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            if let Some(parent) = heritage_name(child, source) {
                return parent.ends_with("Component") || parent == "PureComponent";
            }
        }
    }
    false
}

fn container_name(node: Node<'_>, source: &str) -> Option<String> {
    ancestor_of_kind(node, &["class_declaration", "abstract_class_declaration", "class"])
        .and_then(|class| named_field(class, source))
}

fn push_import(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let source_node = field(node, "source");
    let module = source_node
        .map(|node| unquote(text(node, source)))
        .unwrap_or_default();
    if module.is_empty() {
        return;
    }

    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_import_names(child, source, &mut names);
    }

    out.imports.push(ExtractedImport { module, names });
}

fn collect_import_names(node: Node<'_>, source: &str, names: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "identifier_name" => {
            let name = text(node, source);
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
        "import_specifier" => {
            let name = field(node, "alias")
                .or_else(|| field(node, "name"))
                .map(|node| text(node, source).to_string());
            if let Some(name) = name {
                names.push(name);
            }
        }
        "namespace_import" => {
            if let Some(name) = named_field(node, source) {
                names.push(name);
            }
        }
        "import_clause" | "named_imports" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_import_names(child, source, names);
            }
        }
        _ => {}
    }
}

fn push_call(node: Node<'_>, source: &str, out: &mut ParsedFile) {
    let Some(func) = field(node, "function") else {
        return;
    };
    let name = match func.kind() {
        "identifier" => text(func, source).to_string(),
        "member_expression" => field(func, "property")
            .map(|property| text(property, source).to_string())
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

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

fn is_constant_name(name: &str) -> bool {
    name.len() > 1
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
}
