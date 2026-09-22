use codeatlas_domain::{is_sensitive_path, looks_like_secret, SemanticUnit, SourceFile, Symbol};

const MAX_SNIPPET_CHARS: usize = 2500;
const MAX_SNIPPET_LINES: usize = 80;

pub fn build_units(
    files: &[SourceFile],
    symbols: &[Symbol],
    sources: &[(String, String)],
) -> Vec<SemanticUnit> {
    let sources: std::collections::HashMap<&str, &str> = sources
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect();

    let mut units = Vec::new();
    for symbol in symbols {
        let Some(file) = files.iter().find(|file| file.id == symbol.file_id) else {
            continue;
        };
        if is_sensitive_path(&file.path) {
            continue;
        }
        let Some(source) = sources.get(file.path.as_str()) else {
            continue;
        };
        let text = snippet(source, symbol.start_line, symbol.end_line);
        if text.trim().is_empty() || looks_like_secret(&text) {
            continue;
        }
        units.push(SemanticUnit {
            symbol_id: symbol.id,
            repository_id: file.repository_id,
            file_id: file.id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: symbol.start_line,
            end_line: symbol.end_line,
            text,
        });
    }
    units
}

pub fn snippet(source: &str, start_line: u32, end_line: u32) -> String {
    let start = start_line.max(1) as usize;
    let end = end_line.max(start_line) as usize;
    let taken = source
        .lines()
        .skip(start.saturating_sub(1))
        .take((end + 1).saturating_sub(start).min(MAX_SNIPPET_LINES))
        .collect::<Vec<_>>()
        .join("\n");
    if taken.chars().count() <= MAX_SNIPPET_CHARS {
        taken
    } else {
        taken.chars().take(MAX_SNIPPET_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_is_symbol_span_not_the_file() {
        let source = "line1\nfunction handleFailure() {\n  retry();\n}\nline5\n";
        let text = snippet(source, 2, 4);
        assert!(text.contains("handleFailure"));
        assert!(!text.contains("line1"));
        assert!(!text.contains("line5"));
    }
}
