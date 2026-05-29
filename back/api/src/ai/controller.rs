use crate::ai::client::AiClient;
use actix_web::{post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use shared::models::contest_moderation::moderation_status;

use crate::contest::repository::ContestRepositoryImpl;
use crate::analytics::repository::AnalyticsRepository;
use crate::config::DatabaseConfig;
use crate::db::Db;
use crate::player::repository::PlayerRepository;
use crate::ratings::repository::RatingsRepository;
use serde_json::Value;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarify: Option<AiClarify>,
}

#[derive(Debug, Serialize)]
pub struct AiClarify {
    pub question: String,
    pub choices: Vec<AiClarifyChoice>,
}

#[derive(Debug, Serialize)]
pub struct AiClarifyChoice {
    pub label: String,
    pub question: String,
}

#[derive(Debug, Deserialize)]
pub struct AiAskMyViewRequest {
    pub question: String,
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

#[derive(Debug, Clone)]
enum MyViewTool {
    HeadToHeadBeatMe {
        opponent_handle_lc: String,
        months: i64,
    },
    PlayerWinsLastMonths {
        player_handle_lc: String,
        months: i64,
    },
    PopularGamesLastMonths {
        months: i64,
    },
    CompareBetterThan {
        opponent_handle: String,
    },
    CountHigherRatedThanMe,
    GameCities {
        game_query: String,
    },
    MostPopularGameInCity {
        city: String,
    },
}

fn json_object_from_text(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(s[start..=end].to_string())
}

fn normalize_handle_lc(s: &str) -> Option<String> {
    let h = s.trim().trim_start_matches('@').to_lowercase();
    if h.is_empty() || h.len() > 32 {
        return None;
    }
    if !h
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return None;
    }
    Some(h)
}

fn clamp_months(m: i64) -> i64 {
    m.clamp(1, 24)
}

enum MyViewRouteDecision {
    Tool(MyViewTool),
    Clarify(AiClarify),
}

async fn route_my_view_tool_llm(question: &str) -> Result<MyViewRouteDecision, String> {
    #[derive(Debug, Deserialize)]
    struct RouterOut {
        tool: String,
        #[serde(default)]
        args: Value,
        #[serde(default)]
        confidence: Option<f64>,
    }

    let prompt = format!(
        "You are STG's \"My view\" tool router.\n\
You MUST select exactly ONE tool from the allowlist below and output ONLY valid JSON.\n\
No markdown. No explanations. No extra keys.\n\
\n\
Allowed tools:\n\
- head_to_head_beat_me {{ opponent_handle: string, months: integer }}\n\
- player_wins {{ player_handle: string, months: integer }}\n\
- popular_games_last_months {{ months: integer }}\n\
- compare_better_than {{ opponent_handle: string }}\n\
- count_higher_rated_than_me {{}}\n\
- game_cities {{ game_query: string }}\n\
- most_popular_game_in_city {{ city: string }}\n\
\n\
You must also include a numeric confidence between 0 and 1.\n\
If you are unsure between two or more tools, respond with tool=\"clarify\" and provide args.question (one sentence) and args.options (2-4 short strings).\n\
\n\
Rules:\n\
- If the user says \"last month\" => months=1.\n\
- If they say \"last N months\" => months=N.\n\
- If months is missing but a timeframe is implied, choose months=3.\n\
- Handles must be returned WITHOUT the leading @.\n\
- If you cannot confidently map the question to a tool, output tool=\"clarify\".\n\
\n\
Question: {q}\n",
        q = question.trim()
    );

    let client = AiClient::from_env();
    let raw = client.generate(&prompt).await.map_err(|e| e.to_string())?;
    let Some(obj) = json_object_from_text(&raw) else {
        return Err("router_invalid_json".to_string());
    };
    let out: RouterOut = serde_json::from_str(&obj).map_err(|_| "router_invalid_json".to_string())?;

    // Semantic consistency check: if the user explicitly asked "beat/beaten me",
    // never execute a "total wins" tool. Instead, force a clarify with two choices.
    // This keeps the LLM flexible for phrasing while preventing confident misroutes.
    let q_lc = question.trim().to_lowercase();
    let asked_beat_me = q_lc.contains("beat me") || q_lc.contains("beaten me");

    let tool = out.tool.trim().to_lowercase();
    let confidence = out.confidence.unwrap_or(0.0);

    if asked_beat_me && tool == "player_wins" {
        // Attempt to extract the opponent handle from the question.
        static RE_OPP: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
            regex::Regex::new(r"@?([a-z0-9_-]+)\s+(?:beat|beaten)\s+me").expect("regex")
        });
        let opp = RE_OPP
            .captures(&q_lc)
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .unwrap_or("them");

        // Try to keep the user's requested months, if we can.
        static RE_LAST_N_MONTHS: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new(r"last\s+(\d+)\s+month").expect("regex"));
        let months = if let Some(c) = RE_LAST_N_MONTHS.captures(&q_lc) {
            c.get(1)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(3)
        } else if q_lc.contains("last month") {
            1
        } else {
            3
        };

        return Ok(MyViewRouteDecision::Clarify(AiClarify {
            question: format!(
                "Quick clarification: do you mean head‑to‑head ({} beat you) or {}’s total wins?",
                format!("@{}", opp),
                format!("@{}", opp)
            ),
            choices: vec![
                AiClarifyChoice {
                    label: "Head‑to‑head vs me".to_string(),
                    question: format!(
                        "How many times has @{} beaten me in the last {} months?",
                        opp, months
                    ),
                },
                AiClarifyChoice {
                    label: format!("@{} total wins", opp),
                    question: format!(
                        "How many contests did @{} win in the last {} months?",
                        opp, months
                    ),
                },
            ],
        }));
    }

    // Force clarify when low confidence, regardless of tool.
    if confidence < 0.65 {
        let q = out
            .args
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("Quick clarification: what exactly do you mean?")
            .trim()
            .to_string();
        let mut choices = Vec::new();
        if let Some(arr) = out.args.get("choices").and_then(|v| v.as_array()) {
            for c in arr.iter().take(4) {
                let label = c.get("label").and_then(|v| v.as_str()).unwrap_or("").trim();
                let question = c
                    .get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if !label.is_empty() && !question.is_empty() && question.len() <= 500 {
                    choices.push(AiClarifyChoice {
                        label: label.to_string(),
                        question: question.to_string(),
                    });
                }
            }
        }
        // Fallback to "options" strings if present.
        if choices.is_empty() {
            if let Some(arr) = out.args.get("options").and_then(|v| v.as_array()) {
                for (i, o) in arr.iter().take(4).enumerate() {
                    if let Some(s) = o.as_str().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        choices.push(AiClarifyChoice {
                            label: format!("Option {}", i + 1),
                            question: s.to_string(),
                        });
                    }
                }
            }
        }
        return Ok(MyViewRouteDecision::Clarify(AiClarify {
            question: q,
            choices,
        }));
    }

    match tool.as_str() {
        "head_to_head_beat_me" => {
            let opp = out
                .args
                .get("opponent_handle")
                .and_then(|v| v.as_str())
                .and_then(normalize_handle_lc)
                .ok_or_else(|| "router_invalid_args".to_string())?;
            let months = out
                .args
                .get("months")
                .and_then(|v| v.as_i64())
                .unwrap_or(3);
            Ok(MyViewRouteDecision::Tool(MyViewTool::HeadToHeadBeatMe {
                opponent_handle_lc: opp,
                months: clamp_months(months),
            }))
        }
        "player_wins" => {
            let h = out
                .args
                .get("player_handle")
                .and_then(|v| v.as_str())
                .and_then(normalize_handle_lc)
                .ok_or_else(|| "router_invalid_args".to_string())?;
            let months = out
                .args
                .get("months")
                .and_then(|v| v.as_i64())
                .unwrap_or(3);
            Ok(MyViewRouteDecision::Tool(MyViewTool::PlayerWinsLastMonths {
                player_handle_lc: h,
                months: clamp_months(months),
            }))
        }
        "popular_games_last_months" => {
            let months = out
                .args
                .get("months")
                .and_then(|v| v.as_i64())
                .unwrap_or(3);
            Ok(MyViewRouteDecision::Tool(MyViewTool::PopularGamesLastMonths {
                months: clamp_months(months),
            }))
        }
        "compare_better_than" => {
            let opp = out
                .args
                .get("opponent_handle")
                .and_then(|v| v.as_str())
                .and_then(normalize_handle_lc)
                .ok_or_else(|| "router_invalid_args".to_string())?;
            Ok(MyViewRouteDecision::Tool(MyViewTool::CompareBetterThan {
                opponent_handle: opp,
            }))
        }
        "count_higher_rated_than_me" => Ok(MyViewRouteDecision::Tool(MyViewTool::CountHigherRatedThanMe)),
        "game_cities" => {
            let game_query = out
                .args
                .get("game_query")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.len() <= 80)
                .ok_or_else(|| "router_invalid_args".to_string())?;
            Ok(MyViewRouteDecision::Tool(MyViewTool::GameCities { game_query }))
        }
        "most_popular_game_in_city" => {
            let city = out
                .args
                .get("city")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.len() <= 80)
                .ok_or_else(|| "router_invalid_args".to_string())?;
            Ok(MyViewRouteDecision::Tool(MyViewTool::MostPopularGameInCity { city }))
        }
        "clarify" => {
            let q = out
                .args
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("Quick clarification: what exactly do you mean?")
                .trim()
                .to_string();
            let mut choices = Vec::new();
            if let Some(arr) = out.args.get("choices").and_then(|v| v.as_array()) {
                for c in arr.iter().take(4) {
                    let label = c.get("label").and_then(|v| v.as_str()).unwrap_or("").trim();
                    let question = c
                        .get("question")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if !label.is_empty() && !question.is_empty() && question.len() <= 500 {
                        choices.push(AiClarifyChoice {
                            label: label.to_string(),
                            question: question.to_string(),
                        });
                    }
                }
            }
            Ok(MyViewRouteDecision::Clarify(AiClarify {
                question: q,
                choices,
            }))
        }
        _ => Err("router_unknown_tool".to_string()),
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
            clarify: None,
        }),
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({
            "error":"llm_unavailable",
            "details": e.to_string()
        })),
    }
}

/// Authenticated "my view" Ask STG: answers using the viewer's permissions (no emails).
#[post("/ask-my-view")]
pub async fn ask_my_view_handler(
    req: web::Json<AiAskMyViewRequest>,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
    db_cfg: web::Data<DatabaseConfig>,
    http_req: HttpRequest,
) -> impl Responder {
    let q_raw = req.question.trim();
    if q_raw.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error":"question_required"}));
    }
    if q_raw.chars().count() > 500 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error":"question_too_long","max_chars":500}));
    }

    // Viewer identity (session email) -> player record
    let Some(email) = http_req.extensions().get::<String>().cloned() else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error":"Authentication required"}));
    };
    let Some(viewer) = repo
        .player_usecase
        .repo
        .find_by_email_for_auth(email.as_str())
        .await
    else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error":"user_not_found"}));
    };
    let _viewer_handle = viewer.handle.clone();
    let viewer_key = viewer
        .id
        .trim_start_matches("player/")
        .trim_start_matches("player:")
        .to_string();
    let analytics_repo = AnalyticsRepository::new(db.get_ref().clone(), db_cfg.get_ref().clone());

    let warnings = vec!["My view: uses your account permissions. Handles only.".to_string()];
    let decision = match route_my_view_tool_llm(q_raw).await {
        Ok(d) => d,
        Err(e) => {
            return HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "I can help, but I need a quick clarification.\n\n{}",
                    e.trim()
                ),
                warnings,
                clarify: None,
            });
        }
    };

    match decision {
        MyViewRouteDecision::Clarify(clarify) => {
            return HttpResponse::Ok().json(AiAskResponse {
                answer: clarify.question.clone(),
                warnings,
                clarify: Some(clarify),
            });
        }
        MyViewRouteDecision::Tool(tool) => {
            let mut warnings = warnings;
            warnings.push(format!("routed_tool={:?}", tool));

            match tool {
        MyViewTool::HeadToHeadBeatMe {
            opponent_handle_lc,
            months,
        } => {
            let since_days: i64 = months * 30;
            let beats = match analytics_repo
                .count_opponent_beats_me_since_days_for_viewer(
                    &viewer_key,
                    &opponent_handle_lc,
                    since_days,
                )
                .await
            {
                Ok(n) => n,
                Err(shared::SharedError::NotFound(_)) => {
                    return HttpResponse::NotFound()
                        .json(serde_json::json!({"error":"player_not_found"}))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"analytics_failed","details":e.to_string()}))
                }
            };
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "@{} beat you in {} contest(s) in the last {} month(s).",
                    opponent_handle_lc, beats, months
                ),
                warnings,
                clarify: None,
            })
        }
        MyViewTool::PlayerWinsLastMonths {
            player_handle_lc,
            months,
        } => {
            let since_days: i64 = months * 30;
            let wins = match analytics_repo
                .count_player_wins_since_days_for_viewer(&viewer_key, &player_handle_lc, since_days)
                .await
            {
                Ok(n) => n,
                Err(shared::SharedError::NotFound(_)) => {
                    return HttpResponse::NotFound()
                        .json(serde_json::json!({"error":"player_not_found"}))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"analytics_failed","details":e.to_string()}))
                }
            };
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "@{} won {} contest(s) in the last {} month(s).",
                    player_handle_lc, wins, months
                ),
                warnings,
                clarify: None,
            })
        }
        MyViewTool::PopularGamesLastMonths { months } => {
            let since_days: i64 = months * 30;
            match analytics_repo
                .get_top_games_since_days_for_viewer(&viewer_key, since_days, 5)
                .await
            {
                Ok(rows) if rows.is_empty() => HttpResponse::Ok().json(AiAskResponse {
                    answer: format!("No games found in the last {} month(s).", months),
                    warnings,
                    clarify: None,
                }),
                Ok(rows) => {
                    let summary = rows
                        .into_iter()
                        .map(|(name, plays)| format!("{} ({})", name, plays))
                        .collect::<Vec<_>>()
                        .join(", ");
                    HttpResponse::Ok().json(AiAskResponse {
                        answer: format!(
                            "Most popular games in the last {} month(s): {}",
                            months, summary
                        ),
                        warnings,
                        clarify: None,
                    })
                }
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error":"analytics_failed","details":e.to_string()})),
            }
        }
        MyViewTool::CompareBetterThan { opponent_handle } => {
            let Some(other_player) = repo
                .player_usecase
                .repo
                .find_by_handle(&opponent_handle)
                .await
            else {
                return HttpResponse::NotFound().json(serde_json::json!({"error":"player_not_found"}));
            };
            let other_key = other_player
                .id
                .trim_start_matches("player/")
                .trim_start_matches("player:")
                .to_string();
            let my_rating = analytics_repo
                .get_player_rating_latest(&format!("player/{}", viewer_key))
                .await
                .ok()
                .flatten()
                .map(|(r, _rd, _gp)| r)
                .unwrap_or(1200.0);
            let their_rating = analytics_repo
                .get_player_rating_latest(&format!("player/{}", other_key))
                .await
                .ok()
                .flatten()
                .map(|(r, _rd, _gp)| r)
                .unwrap_or(1200.0);
            let verdict = if my_rating > their_rating {
                "Yes"
            } else if (my_rating - their_rating).abs() < 1.0 {
                "About the same"
            } else {
                "Not yet"
            };
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "{} — your global rating is {:.0} vs @{} at {:.0}.",
                    verdict, my_rating, opponent_handle, their_rating
                ),
                warnings,
                clarify: None,
            })
        }
        MyViewTool::CountHigherRatedThanMe => {
            let my_rating = analytics_repo
                .get_player_rating_latest(&format!("player/{}", viewer_key))
                .await
                .ok()
                .flatten()
                .map(|(r, _rd, _gp)| r)
                .unwrap_or(1200.0);
            let ratings_repo = RatingsRepository::new(db.get_ref().clone());
            let better = match ratings_repo
                .count_higher_latest_ratings("global", my_rating)
                .await
            {
                Ok(n) => n,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"ratings_failed","details":e.to_string()}))
                }
            };
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "{} player(s) have a higher global rating than you (your rating ≈ {:.0}).",
                    better, my_rating
                ),
                warnings,
                clarify: None,
            })
        }
        MyViewTool::GameCities { game_query } => {
            let (resolved_name, list) = match analytics_repo
                .get_cities_for_game_for_viewer(&viewer_key, &game_query, 12)
                .await
            {
                Ok(v) => v,
                Err(shared::SharedError::NotFound(_)) => {
                    return HttpResponse::NotFound()
                        .json(serde_json::json!({"error":"game_not_found"}))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"analytics_failed","details":e.to_string()}))
                }
            };
            if list.is_empty() {
                return HttpResponse::Ok().json(AiAskResponse {
                    answer: format!(
                        "I couldn’t find any visible contests that played {}.",
                        resolved_name
                    ),
                    warnings,
                    clarify: None,
                });
            }
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!("{} has been played in: {}.", resolved_name, list.join(", ")),
                warnings,
                clarify: None,
            })
        }
        MyViewTool::MostPopularGameInCity { city } => {
            let maybe_top = match analytics_repo
                .get_most_popular_game_in_city_for_viewer(&viewer_key, &city)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"analytics_failed","details":e.to_string()}))
                }
            };
            let Some((top_name, top_count)) = maybe_top else {
                return HttpResponse::NotFound().json(serde_json::json!({"error":"city_not_found"}));
            };
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!(
                    "Most popular game in {}: {} ({} plays).",
                    city, top_name, top_count
                ),
                warnings,
                clarify: None,
            })
        }
            }
        }
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

