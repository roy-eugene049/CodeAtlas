use codeatlas_domain::{GitEvolution, RepositoryIndex};

use crate::answer::{
    retrieval_summary, AnswerMode, Citation, ContextConfidence, GroundedAnswer,
};
use crate::error::AiError;
use crate::intent::QueryIntent;

pub fn answer_evolution(
    question: &str,
    evolution: &GitEvolution,
    index: &RepositoryIndex,
) -> Result<GroundedAnswer, AiError> {
    if evolution.hotspots.is_empty() && evolution.commits.is_empty() {
        return Err(AiError::EmptyContext);
    }
    let q = question.to_ascii_lowercase();
    let text = if q.contains("author") || q.contains("who wrote") || q.contains("contributor") {
        render_authors(evolution)
    } else if q.contains("branch") {
        render_branches(evolution)
    } else if q.contains("commit") && !q.contains("file") {
        render_commits(evolution)
    } else {
        render_hotspots(evolution)
    };
    let citations = citations_for_hotspots(evolution, index);
    let files = citations
        .iter()
        .map(|citation| citation.path.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let symbols = citations.len();
    Ok(GroundedAnswer::assemble(
        text,
        citations,
        AnswerMode::Retrieved,
        QueryIntent::Evolution,
        ContextConfidence::from_counts(symbols, files),
        retrieval_summary(symbols, files),
    ))
}

fn render_hotspots(evolution: &GitEvolution) -> String {
    let mut out = String::from("Files that change most frequently\n\n");
    for hotspot in evolution.hotspots.iter().take(8) {
        out.push_str(&format!(
            "{}\nChanged {} times\n{} contributors\n\n",
            hotspot.path, hotspot.change_count, hotspot.contributor_count
        ));
    }
    if evolution.hotspots.is_empty() {
        out.push_str("No file history was collected for this repository.\n");
    }
    out
}

fn render_authors(evolution: &GitEvolution) -> String {
    let mut out = String::from("Authors\n\n");
    for author in evolution.authors.iter().take(12) {
        out.push_str(&format!(
            "{} <{}>\n{} commits · {} file touches\n\n",
            author.name, author.email, author.commit_count, author.file_count
        ));
    }
    out
}

fn render_branches(evolution: &GitEvolution) -> String {
    let mut out = String::from("Branches\n\n");
    for branch in &evolution.branches {
        out.push_str(&format!(
            "{}{}\n{}\n\n",
            branch.name,
            if branch.is_default { " (default)" } else { "" },
            &branch.sha[..branch.sha.len().min(12)]
        ));
    }
    out
}

fn render_commits(evolution: &GitEvolution) -> String {
    let mut out = String::from("Recent commits\n\n");
    for commit in evolution.commits.iter().take(12) {
        out.push_str(&format!(
            "{}  {}\n{} · {} files\n\n",
            &commit.sha[..commit.sha.len().min(8)],
            commit.subject,
            commit.author_name,
            commit.files_changed
        ));
    }
    out
}

fn citations_for_hotspots(evolution: &GitEvolution, index: &RepositoryIndex) -> Vec<Citation> {
    evolution
        .hotspots
        .iter()
        .take(4)
        .filter_map(|hotspot| {
            let symbol = index
                .symbols
                .iter()
                .find(|symbol| {
                    index
                        .files
                        .iter()
                        .any(|file| file.id == symbol.file_id && file.path == hotspot.path)
                })?;
            Some(Citation {
                file_id: symbol.file_id,
                symbol_id: symbol.id,
                symbol: symbol.name.clone(),
                path: hotspot.path.clone(),
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                excerpt: format!(
                    "Changed {} times · {} contributors",
                    hotspot.change_count, hotspot.contributor_count
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        CommitTouch, GitEvolution, Language, Repository, RepositoryIndex, SourceFile, Symbol,
        SymbolKind,
    };

    use super::*;

    #[test]
    fn frequent_files_answer_cites_hotspot_path() {
        let repo = Repository::new("d", "u", "main", "sha");
        let file = SourceFile::new(repo.id, "src/auth/session.ts", Language::TypeScript, 10, "h");
        let symbol = Symbol::new(file.id, "loadSession", SymbolKind::Function, 1, 8);
        let index = RepositoryIndex {
            repository: repo,
            files: vec![file],
            symbols: vec![symbol],
            relationships: vec![],
            units: vec![],
            line_count: 8,
        };
        let evolution = GitEvolution::assemble(
            vec![
                CommitTouch {
                    sha: "a".into(),
                    author_name: "Ada".into(),
                    author_email: "ada@ex.com".into(),
                    authored_at: 1,
                    subject: "wip".into(),
                    paths: vec!["src/auth/session.ts".into()],
                },
                CommitTouch {
                    sha: "b".into(),
                    author_name: "Ben".into(),
                    author_email: "ben@ex.com".into(),
                    authored_at: 2,
                    subject: "wip".into(),
                    paths: vec!["src/auth/session.ts".into()],
                },
            ],
            vec![],
        );
        let answer = answer_evolution(
            "Which files change most frequently?",
            &evolution,
            &index,
        )
        .expect("answer");
        assert!(answer.text.contains("src/auth/session.ts"));
        assert!(answer.text.contains("Changed 2 times"));
        assert!(answer.text.contains("2 contributors"));
        assert_eq!(answer.citations[0].path, "src/auth/session.ts");
    }
}
