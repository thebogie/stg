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
use once_cell::sync::Lazy;

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

fn parse_months_lower(q_lc: &str) -> i64 {
    static RE_LAST_N_MONTHS: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"last\s+(\d+)\s+month").expect("regex"));

    if let Some(c) = RE_LAST_N_MONTHS.captures(q_lc) {
        if let Some(n) = c.get(1).and_then(|m| m.as_str().parse::<i64>().ok()) {
            return n.max(1);
        }
    }
    if q_lc.contains("last month") {
        return 1;
    }
    3
}

fn parse_my_view_tool(question: &str) -> Option<MyViewTool> {
    let q = question.trim();
    let q_lc = q.to_lowercase();

    static RE_BEAT_ME: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r"how many games did\s+@?([a-z0-9_]+)\s+beat\s+me").expect("regex")
    });
    static RE_BETTER_THAN: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r"am i better (?:than|then)\s+@?([a-z0-9_]+)").expect("regex")
    });
    static RE_GAME_CITIES: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r"what city has game\s+(.+?)\s+been played").expect("regex")
    });
    static RE_POPULAR_GAME_CITY: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(r"most popular game in city\s+(.+)$").expect("regex")
    });

    if let Some(c) = RE_BEAT_ME.captures(&q_lc) {
        let opponent_handle_lc = c.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if !opponent_handle_lc.is_empty() {
            let months = parse_months_lower(&q_lc);
            return Some(MyViewTool::HeadToHeadBeatMe {
                opponent_handle_lc,
                months,
            });
        }
    }

    if q_lc.contains("most popular games") && q_lc.contains("month") {
        let months = parse_months_lower(&q_lc);
        return Some(MyViewTool::PopularGamesLastMonths { months });
    }

    if let Some(c) = RE_BETTER_THAN.captures(&q_lc) {
        let opponent_handle = c.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if !opponent_handle.is_empty() {
            return Some(MyViewTool::CompareBetterThan { opponent_handle });
        }
    }

    if q_lc.contains("better glicko")
        || (q_lc.contains("better") && q_lc.contains("rating") && q_lc.contains("than me"))
    {
        return Some(MyViewTool::CountHigherRatedThanMe);
    }

    if let Some(c) = RE_GAME_CITIES.captures(q) {
        let game_query = c.get(1).map(|m| m.as_str().trim()).unwrap_or("").to_string();
        if !game_query.is_empty() {
            return Some(MyViewTool::GameCities { game_query });
        }
    }

    if let Some(c) = RE_POPULAR_GAME_CITY.captures(q) {
        let city = c.get(1).map(|m| m.as_str().trim()).unwrap_or("").to_string();
        if !city.is_empty() {
            return Some(MyViewTool::MostPopularGameInCity { city });
        }
    }

    None
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
    let viewer_key = viewer.id.trim_start_matches("player/").to_string();
    let analytics_repo = AnalyticsRepository::new(db.get_ref().clone(), db_cfg.get_ref().clone());

    let Some(tool) = parse_my_view_tool(q_raw) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error":"unsupported_question",
            "hint":"Try: “How many games did @nick beat me in the last month?” or “Most popular games played in the last 3 months”"
        }));
    };

    let warnings = vec!["My view: uses your account permissions. Handles only.".to_string()];

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
                });
            }
            HttpResponse::Ok().json(AiAskResponse {
                answer: format!("{} has been played in: {}.", resolved_name, list.join(", ")),
                warnings,
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
            })
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

