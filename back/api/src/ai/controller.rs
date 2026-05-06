use crate::ai::client::AiClient;
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use shared::models::contest_moderation::moderation_status;

use crate::contest::repository::ContestRepositoryImpl;
use crate::db::Db;

#[derive(Debug, Deserialize)]
pub struct AiAskRequest {
    pub question: String,
    #[serde(default)]
    pub scope: Option<AiScope>,
}

#[derive(Debug, Deserialize, Default)]
pub struct AiScope {
    pub contest_id: Option<String>,
    pub game_id: Option<String>,
    pub venue_id: Option<String>,
    pub since_days: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AiAskResponse {
    pub answer: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiSmacktalkRequest {
    pub contest_id: String,
    #[serde(default)]
    pub target: Option<String>, // winner|runner_up|everyone
    #[serde(default)]
    pub style: Option<String>, // short|medium|set_3
    #[serde(default)]
    pub intensity: Option<String>, // low|med|high
}

#[derive(Debug, Serialize)]
pub struct AiSmacktalkResponse {
    pub lines: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

fn normalize_contest_id(id: &str) -> String {
    if id.contains('/') {
        id.to_string()
    } else {
        format!("contest/{}", id)
    }
}

#[post("/ask")]
pub async fn ask_handler(req: web::Json<AiAskRequest>) -> impl Responder {
    let q = req.question.trim();
    if q.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error":"question_required"}));
    }
    if q.chars().count() > 500 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error":"question_too_long","max_chars":500}));
    }

    // MVP: prompt-only (no DB tool calls yet). We still declare the public-only boundary.
    let prompt = format!(
        "You are STG's assistant. You have access only to PUBLIC (approved) contest data.\n\
Rules:\n\
- Do not use or request emails or other private info.\n\
- Do not compare raw scores across different games; scores are game-specific.\n\
- Be concise.\n\n\
Question: {q}\n"
    );

    let client = AiClient::from_env();
    match client.generate(&prompt).await {
        Ok(answer) => HttpResponse::Ok().json(AiAskResponse {
            answer,
            warnings: vec!["Public data only (approved contests).".to_string()],
        }),
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({
            "error":"llm_unavailable",
            "details": e.to_string()
        })),
    }
}

#[post("/smacktalk")]
pub async fn smacktalk_handler(
    req: web::Json<AiSmacktalkRequest>,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
) -> impl Responder {
    let contest_id = normalize_contest_id(req.contest_id.trim());

    let Some(contest) = repo
        .find_details_by_id_using(&contest_id, db.get_ref())
        .await
    else {
        return HttpResponse::NotFound().json(serde_json::json!({"error":"contest_not_found"}));
    };

    // Public-only boundary: only approved contests.
    if contest.moderation_status != moderation_status::APPROVED {
        return HttpResponse::NotFound().json(serde_json::json!({"error":"contest_not_found"}));
    }

    // Minimal context. Do not include emails or internal ids.
    let game_names = contest
        .games
        .iter()
        .map(|g| g.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let venue_name = contest.venue.display_name.clone();
    let outcomes = contest
        .outcomes
        .iter()
        .map(|o| format!("{} place={} result={}", o.handle, o.place, o.result))
        .collect::<Vec<_>>()
        .join("\n");

    let target = req.target.clone().unwrap_or_else(|| "everyone".to_string());
    let style = req.style.clone().unwrap_or_else(|| "set_3".to_string());
    let intensity = req.intensity.clone().unwrap_or_else(|| "med".to_string());

    let prompt = format!(
        "You are STG's smacktalk generator.\n\
Generate EXACTLY ONE line of competitive banter based ONLY on the provided public contest context.\n\
\n\
Tone: R-rated profanity allowed, but STRICTLY forbid hate/slurs, threats, sexual content, doxxing, or attacks on identity/appearance/protected traits.\n\
Hard rules: no emails, no real names. Player handles are allowed.\n\
Output format: plain text ONLY (no JSON, no quotes, no markdown).\n\n\
Venue: {venue_name}\n\
Games: {game_names}\n\
Outcomes:\n{outcomes}\n\n\
Target: {target}\n\
Style: {style}\n\
Intensity: {intensity}\n",
        venue_name = venue_name,
        game_names = game_names,
        outcomes = outcomes,
        target = target,
        style = style,
        intensity = intensity
    );

    let client = AiClient::from_env();
    let raw = match client.generate(&prompt).await {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::BadGateway().json(serde_json::json!({
                "error":"llm_unavailable",
                "details": e.to_string()
            }))
        }
    };

    let one = raw
        .lines()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .trim_matches('"')
        .to_string();

    let mut out = AiSmacktalkResponse {
        lines: vec![one],
        warnings: vec![],
    };

    out.warnings
        .push("Public data only (approved contests).".to_string());
    HttpResponse::Ok().json(out)
}

