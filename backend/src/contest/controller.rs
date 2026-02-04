use crate::contest::repository::{ContestRepository, ContestRepositoryImpl};
use crate::player::repository::PlayerRepository;
use actix_web::HttpMessage;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use shared::dto::contest::ContestDto;
use validator::Validate;

#[post("")]
pub async fn create_contest_handler(
    contest: web::Json<ContestDto>,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
) -> impl Responder {
    // Validate input without logging sensitive payload data
    if let Err(e) = contest.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }

    // Extract creator information from authenticated user
    let creator_id = match req.extensions().get::<String>() {
        Some(email) => {
            // Look up the player by email to get the actual player ID
            match repo.player_usecase.repo.find_by_email(email).await {
                Some(player) => player.id,
                None => {
                    log::error!("Authenticated user {} not found in player database", email);
                    return HttpResponse::Unauthorized().json(serde_json::json!({
                        "error": "user_not_found",
                        "details": "Authenticated user not found in player database"
                    }));
                }
            }
        }
        None => {
            log::error!("No authenticated user found for contest creation");
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "not_authenticated",
                "details": "Authentication required to create contests"
            }));
        }
    };

    log::info!("Contest creation requested by player: {}", creator_id);
    match repo.create_contest(contest.into_inner(), creator_id).await {
        Ok(created) => {
            log::info!("Contest created successfully");
            HttpResponse::Ok().json(created)
        }
        Err(e) => {
            log::error!("Contest creation failed: {}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

#[get("/{contest_id}")]
pub async fn get_contest_handler(
    path: web::Path<String>,
    repo: web::Data<ContestRepositoryImpl>,
) -> impl Responder {
    let contest_param = path.into_inner();

    // Normalize contest_id to full ID if it's just a key
    let contest_id = if contest_param.contains('/') {
        contest_param
    } else {
        format!("contest/{}", contest_param)
    };

    log::info!("Fetching contest details for ID: {}", contest_id);

    match repo.find_details_by_id(&contest_id).await {
        Some(contest_details) => {
            log::info!("Contest details found");
            HttpResponse::Ok().json(contest_details)
        }
        None => {
            log::warn!("Contest not found");
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Contest not found"
            }))
        }
    }
}

#[get("/player/{player_id}/game/{game_id}")]
pub async fn get_player_game_contests_handler(
    path: web::Path<(String, String)>,
    repo: web::Data<ContestRepositoryImpl>,
) -> impl Responder {
    let (player_param, game_id) = path.into_inner();

    // Convert keys to full ArangoDB IDs for internal use
    let player_id = if player_param.contains('/') {
        player_param
    } else {
        format!("player/{}", player_param)
    };

    let game_id = if game_id.contains('/') {
        game_id
    } else {
        format!("game/{}", game_id)
    };

    log::info!(
        "Fetching contests for player {} and game {}",
        player_id,
        game_id
    );

    match repo
        .find_contests_by_player_and_game(&player_id, &game_id)
        .await
    {
        Ok(contests) => {
            log::info!(
                "Found {} contests for player {} and game {}",
                contests.len(),
                player_id,
                game_id
            );
            HttpResponse::Ok().json(contests)
        }
        Err(e) => {
            log::error!(
                "Failed to fetch contests for player {} and game {}: {}",
                player_id,
                game_id,
                e
            );
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch contests"
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct ContestSearchQuery {
    pub q: Option<String>,
    pub start_from: Option<String>,
    pub start_to: Option<String>,
    pub stop_from: Option<String>,
    pub stop_to: Option<String>,
    pub venue_id: Option<String>,
    pub game_ids: Option<String>, // csv
    pub sort_by: Option<String>,  // start|stop|created_at
    pub sort_dir: Option<String>, // asc|desc
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub scope: Option<String>,     // mine|my_venues|my_games|all
    pub player_id: Option<String>, // fallback if auth not plumbed
}

pub async fn search_contests_handler_impl(
    query: web::Query<ContestSearchQuery>,
    repo: web::Data<ContestRepositoryImpl>,
    player_repo: web::Data<crate::player::repository::PlayerRepositoryImpl>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let q = query.q.clone().unwrap_or_default();
    let sort_by = query.sort_by.clone().unwrap_or_else(|| "start".into());
    let sort_dir = query.sort_dir.clone().unwrap_or_else(|| "desc".into());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let requested_scope = query.scope.clone().unwrap_or_else(|| "mine".into());

    // If query.player_id is provided, use it for filtering (searching for a specific player's contests)
    // Otherwise, use authenticated user's player_id for scope filtering
    let (filter_player_id, scope_player_id, effective_scope) =
        if let Some(query_player_id) = &query.player_id {
            // User is searching for a specific player's contests
            // Check if the provided value is an email (contains '@') or a player ID
            let normalized_id = if query_player_id.contains('@') {
                // It's an email - look up the player by email to get the actual player ID
                match player_repo.find_by_email(query_player_id).await {
                    Some(player) => {
                        log::info!("Found player by email '{}': {}", query_player_id, player.id);
                        player.id
                    }
                    None => {
                        log::warn!("Player not found for email: {}", query_player_id);
                        // Return empty to indicate no filter (will return empty results)
                        String::new()
                    }
                }
            } else if query_player_id.contains('/') {
                // Already in ArangoDB format (player/xxx)
                query_player_id.clone()
            } else {
                // Assume it's a player ID without the prefix
                format!("player/{}", query_player_id)
            };

            // If we couldn't find the player, don't filter (will return empty results)
            let filter_id = if normalized_id.is_empty() {
                None
            } else {
                Some(normalized_id)
            };

            // When filtering by a specific player, always use "all" scope
            (filter_id, String::new(), "all".to_string())
        } else {
            // No specific player filter, use authenticated user's player_id for scope
            let auth_player_id = if let Some(email) = req.extensions().get::<String>() {
                // Look up the player by email to get the actual player ID
                match player_repo.find_by_email(email).await {
                    Some(player) => player.id,
                    None => {
                        log::warn!("Player not found for email: {}", email);
                        String::new()
                    }
                }
            } else {
                String::new()
            };
            // If there's no player context, force scope to 'all' to avoid 400s and allow browsing
            let effective_scope = if auth_player_id.is_empty() {
                "all".to_string()
            } else {
                requested_scope
            };
            (None, auth_player_id, effective_scope)
        };
    let venue_id = query.venue_id.clone();
    let game_ids: Vec<String> = query
        .game_ids
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_else(|| vec![]);

    match repo
        .search_contests(
            &q,
            query.start_from.as_deref(),
            query.start_to.as_deref(),
            query.stop_from.as_deref(),
            query.stop_to.as_deref(),
            venue_id.as_deref(),
            &game_ids,
            &sort_by,
            &sort_dir,
            page,
            page_size,
            &effective_scope,
            &scope_player_id,
            filter_player_id.as_deref(),
        )
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            log::error!("search_contests failed: {}. Returning empty result set.", e);
            HttpResponse::Ok().json(json!({
                "items": [],
                "total": 0,
                "page": page,
                "page_size": page_size
            }))
        }
    }
}

#[get("/search")]
pub async fn search_contests_handler(
    query: web::Query<ContestSearchQuery>,
    repo: web::Data<ContestRepositoryImpl>,
    player_repo: web::Data<crate::player::repository::PlayerRepositoryImpl>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    search_contests_handler_impl(query, repo, player_repo, req).await
}
