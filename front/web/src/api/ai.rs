use crate::api::api_url;
use crate::api::utils::authenticated_post;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AiAskRequest {
    pub question: String,
}

#[derive(Debug, Deserialize)]
pub struct AiAskResponse {
    pub answer: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub async fn ai_ask(question: String) -> Result<AiAskResponse, String> {
    let url = api_url("/api/ai/ask");
    let req = authenticated_post(&url)
        .json(&AiAskRequest { question })
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<AiSmacktalkResponse>()
        .await
        .map_err(|e| e.to_string())
}

