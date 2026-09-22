use tree_sitter::Node;

pub fn text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

pub fn line_range(node: Node<'_>) -> (u32, u32) {
    (
        node.start_position().row as u32 + 1,
        node.end_position().row as u32 + 1,
    )
}

pub fn field<'a>(node: Node<'a>, name: &str) -> Option<Node<'a>> {
    node.child_by_field_name(name)
}

pub fn ancestor_of_kind<'a>(mut node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if kinds.contains(&parent.kind()) {
            return Some(parent);
        }
        node = parent;
    }
    None
}

pub fn named_field(node: Node<'_>, source: &str) -> Option<String> {
    field(node, "name").map(|name| text(name, source).to_string())
}

pub fn is_pascal_case(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

pub fn contains_kind(node: Node<'_>, kinds: &[&str]) -> bool {
    if kinds.contains(&node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if contains_kind(child, kinds) {
            return true;
        }
    }
    false
}

pub fn walk_preorder(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_preorder(child, visit);
    }
}
