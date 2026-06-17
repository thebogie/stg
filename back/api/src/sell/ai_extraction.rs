//! Vision AI extraction for sell listings (Bedrock or dev stub).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use shared::dto::sell_listing::{AiClarifyChoiceDto, AiClarifyDto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionOutput {
    pub title: String,
    pub description: String,
    pub condition_notes: String,
    pub game_name: String,
    pub edition_notes: String,
    pub missing_components: Vec<String>,
    pub bgg_id_candidates: Vec<i32>,
    pub confidence: f64,
    pub questions: Vec<String>,
    pub warnings: Vec<String>,
    pub clarify: Option<AiClarifyDto>,
}

#[derive(Debug, Deserialize)]
struct LlmJson {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    condition_notes: String,
    #[serde(default)]
    game_name: String,
    #[serde(default)]
    edition_notes: String,
    #[serde(default)]
    missing_components: Vec<String>,
    #[serde(default)]
    bgg_id_candidates: Vec<i32>,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    questions: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    clarify_question: Option<String>,
    #[serde(default)]
    clarify_choices: Vec<String>,
}

fn json_object_from_text(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(s[start..=end].to_string())
}

fn parse_llm_json(raw: &str) -> Result<ExtractionOutput, String> {
    let obj = json_object_from_text(raw).ok_or_else(|| "invalid_json".to_string())?;
    let parsed: LlmJson =
        serde_json::from_str(&obj).map_err(|e| format!("parse_error: {e}"))?;

    let clarify = parsed.clarify_question.map(|question| AiClarifyDto {
        question,
        choices: parsed
            .clarify_choices
            .into_iter()
            .map(|label| AiClarifyChoiceDto {
                question: label.clone(),
                label,
            })
            .collect(),
    });

    Ok(ExtractionOutput {
        title: parsed.title,
        description: parsed.description,
        condition_notes: parsed.condition_notes,
        game_name: parsed.game_name,
        edition_notes: parsed.edition_notes,
        missing_components: parsed.missing_components,
        bgg_id_candidates: parsed.bgg_id_candidates,
        confidence: parsed.confidence.clamp(0.0, 1.0),
        questions: parsed.questions,
        warnings: parsed.warnings,
        clarify,
    })
}

const EXTRACTION_PROMPT: &str = r#"You are STG's board-game listing assistant. Analyze the provided game photos and output ONLY valid JSON (no markdown) with these keys:
title, description, condition_notes, game_name, edition_notes, missing_components (array of strings), bgg_id_candidates (array of integers, empty if unknown), confidence (0-1), questions (array of strings for uncertain details), warnings (array of strings), clarify_question (optional string), clarify_choices (optional array of 2-4 short strings).
Describe box, components, cards, inserts, and manual condition. Note missing or damaged parts. If game identity is uncertain, set confidence below 0.7 and provide clarify_question with clarify_choices."#;

/// Extract listing draft from photos. Uses Bedrock when configured, else stub.
pub async fn extract_from_photos(photos: &[(String, Vec<u8>)]) -> Result<ExtractionOutput, String> {
    if photos.is_empty() {
        return Err("no_photos".to_string());
    }

    if bedrock_configured() {
        bedrock_extract(photos).await
    } else {
        stub_extract(photos)
    }
}

fn bedrock_configured() -> bool {
    std::env::var("BEDROCK_MODEL_ID")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
        && std::env::var("AWS_REGION")
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
}

async fn bedrock_extract(photos: &[(String, Vec<u8>)]) -> Result<ExtractionOutput, String> {
    let region = std::env::var("AWS_REGION").map_err(|_| "AWS_REGION not set".to_string())?;
    let model_id =
        std::env::var("BEDROCK_MODEL_ID").map_err(|_| "BEDROCK_MODEL_ID not set".to_string())?;

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region))
        .load()
        .await;
    let client = aws_sdk_bedrockruntime::Client::new(&config);

    let mut content: Vec<serde_json::Value> = Vec::new();
    for (mime, bytes) in photos {
        let media_type = if mime.contains("png") {
            "image/png"
        } else if mime.contains("webp") {
            "image/webp"
        } else {
            "image/jpeg"
        };
        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": B64.encode(bytes)
            }
        }));
    }
    content.push(serde_json::json!({
        "type": "text",
        "text": EXTRACTION_PROMPT
    }));

    let body = serde_json::json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 4096,
        "temperature": 0.2,
        "messages": [{ "role": "user", "content": content }]
    });

    let resp = client
        .invoke_model()
        .model_id(&model_id)
        .content_type("application/json")
        .accept("application/json")
        .body(body.to_string().into_bytes().into())
        .send()
        .await
        .map_err(|e| format!("bedrock_request: {e}"))?;

    let bytes = resp.body().as_ref();
    let parsed: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("bedrock_json: {e}"))?;

    let text = parsed
        .pointer("/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "bedrock_empty_response".to_string())?;

    parse_llm_json(text)
}

fn stub_extract(photos: &[(String, Vec<u8>)]) -> Result<ExtractionOutput, String> {
    let total_kb: u64 = photos.iter().map(|(_, b)| b.len() as u64).sum::<u64>() / 1024;
    Ok(ExtractionOutput {
        title: "Board game for sale".to_string(),
        description: format!(
            "Listing draft from {} photo(s) ({} KB total). Configure BEDROCK_MODEL_ID and AWS_REGION for AI extraction.",
            photos.len(),
            total_kb
        ),
        condition_notes: "Review condition manually — AI extraction not configured.".to_string(),
        game_name: String::new(),
        edition_notes: String::new(),
        missing_components: Vec::new(),
        bgg_id_candidates: Vec::new(),
        confidence: 0.3,
        questions: vec![
            "What is the exact game title?".to_string(),
            "Are all components present?".to_string(),
        ],
        warnings: vec!["Using stub extractor — set BEDROCK_MODEL_ID for vision AI.".to_string()],
        clarify: Some(AiClarifyDto {
            question: "Which game is shown in these photos?".to_string(),
            choices: vec![
                AiClarifyChoiceDto {
                    label: "I know the title".to_string(),
                    question: "Enter the game title on the next step.".to_string(),
                },
                AiClarifyChoiceDto {
                    label: "Not sure".to_string(),
                    question: "Search BGG manually after reviewing photos.".to_string(),
                },
            ],
        }),
    })
}
