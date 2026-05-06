use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct AiClient {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
pub enum AiClientError {
    #[error("LLM request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("LLM returned empty response")]
    EmptyResponse,
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

impl AiClient {
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://ollama:11434".to_string());
        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());
        let timeout_ms: u64 = std::env::var("LLM_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30_000);

        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .expect("reqwest client build");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            http,
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, AiClientError> {
        let url = format!("{}/api/generate", self.base_url);
        let req = OllamaGenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };
        let resp: OllamaGenerateResponse = self.http.post(url).json(&req).send().await?.json().await?;
        let trimmed = resp.response.trim().to_string();
        if trimmed.is_empty() {
            return Err(AiClientError::EmptyResponse);
        }
        Ok(trimmed)
    }
}

