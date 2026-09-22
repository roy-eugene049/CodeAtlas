use codeatlas_ai::{AiError, LanguageClient};

/// HTTP lives here, not in the AI crate.
pub struct OpenAiClient {
    api_key: String,
    model: String,
}

impl OpenAiClient {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CODEATLAS_OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .ok()
            .filter(|key| !key.is_empty())?;
        let model = std::env::var("CODEATLAS_OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".to_string());
        Some(Self { api_key, model })
    }
}

impl LanguageClient for OpenAiClient {
    fn complete(&self, system: &str, user: &str) -> Result<String, AiError> {
        let body = serde_json::json!({
            "model": self.model,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });
        let response = ureq::post("https://api.openai.com/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|error| AiError::Model(error.to_string()))?;
        let payload: serde_json::Value = response
            .into_json()
            .map_err(|error| AiError::Model(error.to_string()))?;
        Ok(payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}")
            .to_string())
    }
}
