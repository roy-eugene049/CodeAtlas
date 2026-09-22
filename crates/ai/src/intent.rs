#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryIntent {
    Locate,
    Explain,
    Impact,
    Evolution,
    Search,
}

pub fn detect_intent(question: &str) -> QueryIntent {
    let q = question.to_ascii_lowercase();
    if q.contains("hotspot")
        || q.contains("frequently")
        || q.contains("contributor")
        || q.contains("contributors")
        || q.contains("who wrote")
        || q.contains("authors")
        || q.contains("commits")
        || q.contains("branches")
        || (q.contains("change") && (q.contains("file") || q.contains("most") || q.contains("often")))
    {
        QueryIntent::Evolution
    } else if q.contains("where") || q.contains("which file") || q.contains("handle") || q.contains("find")
    {
        QueryIntent::Locate
    } else if q.contains("impact") || q.contains("depend") || q.contains("break") || q.contains("affect")
    {
        QueryIntent::Impact
    } else if q.contains("how") || q.contains("explain") || q.contains("work") || q.contains("does")
    {
        QueryIntent::Explain
    } else {
        QueryIntent::Search
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locates_failed_payment_questions() {
        assert_eq!(
            detect_intent("Where do we handle failed payments?"),
            QueryIntent::Locate
        );
    }

    #[test]
    fn explains_architecture_questions() {
        assert_eq!(
            detect_intent("How does authentication work?"),
            QueryIntent::Explain
        );
    }

    #[test]
    fn evolution_questions_win_over_locate() {
        assert_eq!(
            detect_intent("Which files change most frequently?"),
            QueryIntent::Evolution
        );
    }
}
