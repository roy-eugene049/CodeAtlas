use crate::error::SearchError;

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, SearchError>;
    fn dimensions(&self) -> usize;
}

/// Local hashed bag-of-tokens embedder. Swap this without changing callers.
pub struct HashedEmbedder {
    dimensions: usize,
}

impl HashedEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions: dimensions.max(32),
        }
    }
}

impl Default for HashedEmbedder {
    fn default() -> Self {
        Self::new(256)
    }
}

impl Embedder for HashedEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, SearchError> {
        Ok(texts.iter().map(|text| self.embed_one(text)).collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl HashedEmbedder {
    fn embed_one(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; self.dimensions];
        for token in tokenize(text) {
            let bucket = hash_token(&token) as usize % self.dimensions;
            vector[bucket] += 1.0;
        }
        normalize(&mut vector);
        vector
    }
}

pub fn cosine(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, tokens: &mut Vec<String>| {
        if current.len() >= 2 {
            tokens.push(std::mem::take(current));
        } else {
            current.clear();
        }
    };

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && !current.is_empty() {
                flush(&mut current, &mut tokens);
            }
            current.push(ch.to_ascii_lowercase());
        } else {
            flush(&mut current, &mut tokens);
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

pub fn query_tokens(question: &str) -> Vec<String> {
    let mut tokens = tokenize(question);
    let mut extras = Vec::new();
    for token in &tokens {
        if token.len() >= 8 {
            extras.push(token[..4].to_string());
        }
        match token.as_str() {
            "authentication" | "authorize" | "authorized" | "login" => {
                extras.extend(["auth", "login", "session"].map(String::from));
            }
            "payments" | "payment" | "billing" | "checkout" => {
                extras.extend(["pay", "payment", "stripe", "transaction"].map(String::from));
            }
            "failed" | "failure" | "failing" => {
                extras.extend(["fail", "failure", "retry"].map(String::from));
            }
            _ => {}
        }
    }
    tokens.extend(extras);
    tokens.sort();
    tokens.dedup();
    tokens
}

pub fn expanded_query(question: &str) -> String {
    format!("{} {}", question, query_tokens(question).join(" "))
}

fn hash_token(token: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in token.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector.iter_mut() {
            *value /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_payment_texts_rank_above_auth() {
        let embedder = HashedEmbedder::default();
        let vectors = embedder
            .embed(&[
                "Where do we handle failed payments?".into(),
                "handleFailure stripe retry transaction notification".into(),
                "AuthProvider validates a session cookie".into(),
            ])
            .expect("embed");
        assert!(cosine(&vectors[0], &vectors[1]) > cosine(&vectors[0], &vectors[2]));
    }
}
