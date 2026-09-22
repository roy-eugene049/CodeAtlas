/// Paths that must never be indexed, embedded, or sent to an LLM.
pub fn is_sensitive_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name == "id_rsa"
        || name == "id_ed25519"
        || name == "credentials"
        || name == "credentials.json"
        || name == "secret.json"
        || name.contains("private") && name.ends_with(".key")
        || lower.contains("/.git/")
        || lower.starts_with("secrets/")
        || lower.contains("/secrets/")
}

/// Detect assignment-style secrets before they reach an external model.
pub fn looks_like_secret(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    upper.contains("BEGIN PRIVATE KEY")
        || upper.contains("BEGIN RSA PRIVATE KEY")
        || upper.contains("AWS_SECRET_ACCESS_KEY")
        || upper.contains("API_KEY=")
        || upper.contains("SECRET=")
        || upper.contains("PASSWORD=")
        || upper.contains("PRIVATE_KEY")
}

/// Replace secret values so retrieved context is safe to send to an LLM.
pub fn redact_secrets(text: &str) -> String {
    text.lines().map(redact_line).collect::<Vec<_>>().join("\n")
}

fn redact_line(line: &str) -> String {
    let upper = line.to_ascii_uppercase();
    if !(upper.contains("API_KEY=")
        || upper.contains("SECRET=")
        || upper.contains("PASSWORD=")
        || upper.contains("PRIVATE_KEY")
        || upper.contains("BEGIN PRIVATE KEY")
        || upper.contains("AWS_SECRET_ACCESS_KEY"))
    {
        return line.to_string();
    }
    if let Some(index) = line.find('=') {
        return format!("{}[REDACTED]", &line[..=index]);
    }
    if let Some(index) = line.find(':') {
        return format!("{} [REDACTED]", &line[..=index]);
    }
    "[REDACTED]".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_env_and_key_files() {
        assert!(is_sensitive_path(".env"));
        assert!(is_sensitive_path("config/.env.local"));
        assert!(is_sensitive_path("certs/prod.pem"));
        assert!(is_sensitive_path("secrets/token"));
        assert!(!is_sensitive_path("src/auth/session.ts"));
    }

    #[test]
    fn detects_assignment_secrets() {
        assert!(looks_like_secret("API_KEY=abc"));
        assert!(looks_like_secret("SECRET=shh"));
        assert!(looks_like_secret("PASSWORD=hunter2"));
        assert!(looks_like_secret("-----BEGIN PRIVATE KEY-----"));
        assert!(!looks_like_secret("export function login() {}"));
    }

    #[test]
    fn redacts_values_but_keeps_keys() {
        let redacted = redact_secrets("const x = 1;\nAPI_KEY=super-secret\nok");
        assert!(redacted.contains("API_KEY=[REDACTED]"));
        assert!(!redacted.contains("super-secret"));
        assert!(redacted.contains("const x = 1;"));
    }
}
