use crate::api::api_url;
use crate::api::utils::authenticated_post;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AiAskRequest {
    pub question: String,
}

#[derive(Debug, Deserialize, Default)]
struct ApiErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    hint: Option<String>,
}

async fn http_error(resp: gloo_net::http::Response) -> String {
    let status = resp.status();
    let txt = resp.text().await.unwrap_or_default();
    if let Ok(body) = serde_json::from_str::<ApiErrorBody>(&txt) {
        let mut parts = vec![format!("HTTP {}", status)];
        if let Some(e) = body.error {
            parts.push(e);
        }
        if let Some(h) = body.hint {
            parts.push(h);
        }
        if let Some(d) = body.details {
            parts.push(d);
        }
        return parts.join(": ");
    }
    if txt.trim().is_empty() {
        format!("HTTP {}", status)
    } else {
        format!("HTTP {}: {}", status, txt)
    }
}

#[derive(Debug, Deserialize)]
pub struct AiAskResponse {
    pub answer: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub clarify: Option<AiClarify>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AiClarify {
    pub question: String,
    #[serde(default)]
    pub choices: Vec<AiClarifyChoice>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AiClarifyChoice {
    pub label: String,
    pub question: String,
}

pub async fn ai_ask(question: String) -> Result<AiAskResponse, String> {
    let url = api_url("/api/ai/ask");
    let req = authenticated_post(&url)
        .json(&AiAskRequest { question })
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_error(resp).await);
    }
    resp.json::<AiAskResponse>().await.map_err(|e| e.to_string())
}

pub async fn ai_ask_my_view(question: String) -> Result<AiAskResponse, String> {
    let url = api_url("/api/ai/me/ask-my-view");
    let req = authenticated_post(&url)
        .json(&AiAskRequest { question })
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_error(resp).await);
    }
    resp.json::<AiAskResponse>().await.map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct AiSmacktalkRequest {
    pub contest_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiSmacktalkResponse {
    pub lines: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub async fn ai_smacktalk(req: AiSmacktalkRequest) -> Result<AiSmacktalkResponse, String> {
    let url = api_url("/api/ai/smacktalk");
    let http_req = authenticated_post(&url).json(&req).map_err(|e| e.to_string())?;
    let resp = http_req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_error(resp).await);
    }
    resp.json::<AiSmacktalkResponse>()
        .await
        .map_err(|e| e.to_string())
}

