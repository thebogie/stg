use actix_web::{post, web, HttpResponse, HttpRequest, get, HttpMessage, put};

use shared::models::player::PlayerLogin;
use crate::player::usecase::{PlayerUseCase, PlayerUseCaseImpl};
use crate::player::repository::{PlayerRepositoryImpl, PlayerRepository};
use crate::player::error::PlayerError;
use crate::error::ApiError;
use uuid::Uuid;
use shared::dto::player::{LoginResponse, PlayerDto, CreatePlayerRequest, UpdateEmailRequest, UpdateHandleRequest, UpdatePasswordRequest, UpdateResponse};
use log::{info, error, warn};
use crate::player::session::SessionStore;

pub async fn login_handler_impl<R, S>(
    login: web::Json<PlayerLogin>,
    session_store: web::Data<S>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
    S: SessionStore + 'static,
{
    let email = login.email.clone();
    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };

    match usecase.login(login.into_inner()).await {
        Ok(player) => {
            let session_id = Uuid::new_v4().to_string();
            match session_store.set_session(&session_id, &player.email).await {
                Ok(_) => {
                    let player_dto = PlayerDto::from(&player);
                    let response = LoginResponse { 
                        player: player_dto,
                        session_id: session_id.clone(),
                    };
                    info!("Player {} logged in successfully, session {} created", player.email, session_id);
                    Ok(HttpResponse::Ok().json(response))
                },
                Err(e) => {
                    let err_msg = format!("Session store error: {}", e);
                    error!("Session store error during login for {}: {}", player.email, e);
                    Err(PlayerError::SessionError(err_msg).into())
                },
            }
        },
        Err(PlayerError::NotFound) => {
            info!("Login attempt for non-existent player: {}", email);
            Err(PlayerError::NotFound.into())
        },
        Err(PlayerError::InvalidPassword) => {
            info!("Invalid password attempt for player: {}", email);
            Err(PlayerError::InvalidPassword.into())
        },
        Err(e) => {
            error!("Unexpected login error for {}: {}", email, e);
            Err(e.into())
        },
    }
}

#[post("/login")]
pub async fn login_handler_prod(
    req: HttpRequest,
    login: web::Json<PlayerLogin>,
    session_store: web::Data<crate::player::session::RedisSessionStore>,
    repo: web::Data<PlayerRepositoryImpl>,
    redis_client: web::Data<redis::Client>,
) -> Result<HttpResponse, ApiError> {
    // Basic rate limiting: 10 attempts per 5 minutes per IP+email
    if let Some(peer) = req.peer_addr() {
        let ip = peer.ip().to_string();
        let key = format!("login:{}:{}", ip, login.email);
        if let Ok(mut conn) = redis_client.get_async_connection().await {
            let _: () = redis::cmd("INCR").arg(&key).query_async(&mut conn).await.unwrap_or(());
            let ttl: i64 = redis::cmd("TTL").arg(&key).query_async(&mut conn).await.unwrap_or(-1);
            if ttl < 0 { let _: () = redis::cmd("EXPIRE").arg(&key).arg(300).query_async(&mut conn).await.unwrap_or(()); }
            let count: i64 = redis::cmd("GET").arg(&key).query_async(&mut conn).await.unwrap_or(0);
            if count > 10 {
                return Ok(HttpResponse::TooManyRequests().json(serde_json::json!({
                    "error": "Too Many Requests",
                    "message": "Too many login attempts. Please try again later."
                })));
            }
        }
    }

    // Inline the login_impl logic so we can set cookies
    let email = login.email.clone();
    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };
    match usecase.login(login.into_inner()).await {
        Ok(player) => {
            let session_id = uuid::Uuid::new_v4().to_string();
                            match session_store.set_session(&session_id, &player.email).await {
                    Ok(_) => {
                        let player_dto = PlayerDto::from(&player);
                        let response = LoginResponse { player: player_dto, session_id: session_id.clone() };
                        // No cookies - frontend will use Authorization header
                        Ok(HttpResponse::Ok().json(response))
                    },
                Err(e) => {
                    let err_msg = format!("Session store error: {}", e);
                    error!("Session store error during login for {}: {}", player.email, e);
                    Err(PlayerError::SessionError(err_msg).into())
                }
            }
        },
        Err(PlayerError::NotFound) => {
            info!("Login attempt for non-existent player: {}", email);
            Err(PlayerError::NotFound.into())
        },
        Err(PlayerError::InvalidPassword) => {
            info!("Invalid password attempt for player: {}", email);
            Err(PlayerError::InvalidPassword.into())
        },
        Err(e) => {
            error!("Unexpected login error for {}: {}", email, e);
            Err(e.into())
        },
    }
}

pub async fn register_handler_impl<R>(
    registration: web::Json<CreatePlayerRequest>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let email = registration.email.clone();
    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };

    match usecase.register(registration.into_inner()).await {
        Ok(player) => {
            let player_dto = PlayerDto::from(&player);
            info!("Player {} registered successfully", email);
            Ok(HttpResponse::Created().json(player_dto))
        },
        Err(PlayerError::AlreadyExists) => {
            info!("Registration attempt for existing email: {}", email);
            Err(PlayerError::AlreadyExists.into())
        },
        Err(e) => {
            error!("Unexpected registration error for {}: {}", email, e);
            Err(e.into())
        },
    }
}

#[post("/register")]
pub async fn register_handler_prod(
    registration: web::Json<CreatePlayerRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    register_handler_impl::<PlayerRepositoryImpl>(registration, repo).await
}

pub async fn logout_handler<S: SessionStore + 'static>(
    req: HttpRequest,
    session_store: web::Data<S>,
) -> Result<HttpResponse, ApiError> {
    // Get session ID from Authorization header
    let session_id = req.headers().get("Authorization")
        .and_then(|auth_header| {
            auth_header.to_str().ok()
                .and_then(|header_str| {
                    if header_str.starts_with("Bearer ") {
                        Some(header_str[7..].trim().to_string())
                    } else {
                        None
                    }
                })
        });

    let session_id = match session_id {
        Some(sid) => sid,
        None => {
            warn!("Logout attempt without Authorization header from IP: {}", 
                req.peer_addr().map(|addr| addr.ip().to_string()).unwrap_or_else(|| "unknown".to_string()));
            return Err(ApiError::bad_request("Missing Authorization header"));
        },
    };

    // Log logout attempt with session ID, IP, and user agent
    let peer_ip = req.peer_addr().map(|addr| addr.ip().to_string()).unwrap_or_else(|| "unknown".to_string());
    let user_agent = req.headers()
        .get("User-Agent")
        .and_then(|ua| ua.to_str().ok())
        .unwrap_or("unknown");
    info!("Logout attempt for session {} from IP: {} with User-Agent: {}", session_id, peer_ip, user_agent);

            match session_store.delete_session(&session_id).await {
            Ok(_) => {
                info!("Player logged out successfully, session {} terminated from IP: {} with User-Agent: {}", session_id, peer_ip, user_agent);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Logged out successfully",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })))
            },
        Err(e) => {
            let err_msg = format!("Session store error: {}", e);
            error!("Session store error during logout for session {}: {}", session_id, e);
            Err(PlayerError::SessionError(err_msg).into())
        },
    }
}

#[post("/logout")]
pub async fn logout_handler_prod(
    req: HttpRequest,
    session_store: web::Data<crate::player::session::RedisSessionStore>,
) -> Result<HttpResponse, ApiError> {
    logout_handler(req, session_store).await
}

pub async fn me_handler_impl<R>(
    req: HttpRequest,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return Err(ApiError::unauthorized("Not authenticated")),
    };

    let player = match repo.find_by_email(&email).await {
        Some(player) => player,
        None => return Err(PlayerError::NotFound.into()),
    };

    let player_dto = PlayerDto::from(&player);
    Ok(HttpResponse::Ok().json(player_dto))
}

#[get("")]
pub async fn me_handler_prod(
    req: HttpRequest,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    me_handler_impl::<PlayerRepositoryImpl>(req, repo).await
}

pub async fn search_players_handler_impl<R>(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let empty_string = String::new();
    let search_query = query.get("query").unwrap_or(&empty_string);

    if search_query.is_empty() {
        return Err(ApiError::bad_request("Query parameter is required"));
    }

    let players = repo.search_players(search_query).await;
    // Always return 200 OK with an empty list if no players found
    let player_dtos: Vec<PlayerDto> = players.iter().map(|p| PlayerDto::from(p)).collect();
    Ok(HttpResponse::Ok().json(player_dtos))
}

#[get("/search")]
pub async fn search_players_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    search_players_handler_impl::<PlayerRepositoryImpl>(query, repo).await
}

// DB-only alias for clarity
#[get("/db_search")]
pub async fn search_players_db_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    search_players_handler_impl::<PlayerRepositoryImpl>(query, repo).await
}

pub async fn update_email_handler_impl<R>(
    req: HttpRequest,
    update_request: web::Json<UpdateEmailRequest>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return Err(ApiError::unauthorized("Not authenticated")),
    };

    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };
    
    match usecase.update_email(&email, &update_request.email, &update_request.password).await {
        Ok(player) => {
            let player_dto = PlayerDto::from(&player);
            let response = UpdateResponse {
                message: "Email updated successfully".to_string(),
                player: player_dto,
            };
            info!("Player {} updated email to {}", email, update_request.email);
            Ok(HttpResponse::Ok().json(response))
        },
        Err(PlayerError::InvalidPassword) => {
            info!("Invalid password for email update attempt by {}", email);
            Err(PlayerError::InvalidPassword.into())
        },
        Err(PlayerError::AlreadyExists) => {
            info!("Email update attempt with existing email {} by {}", update_request.email, email);
            Err(PlayerError::AlreadyExists.into())
        },
        Err(e) => {
            error!("Unexpected error updating email for {}: {}", email, e);
            Err(e.into())
        },
    }
}

#[put("/email")]
pub async fn update_email_handler_prod(
    req: HttpRequest,
    update_request: web::Json<UpdateEmailRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    update_email_handler_impl(req, update_request, repo).await
}

pub async fn update_handle_handler_impl<R>(
    req: HttpRequest,
    update_request: web::Json<UpdateHandleRequest>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return Err(ApiError::unauthorized("Not authenticated")),
    };

    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };
    
    match usecase.update_handle(&email, &update_request.handle, &update_request.password).await {
        Ok(player) => {
            let player_dto = PlayerDto::from(&player);
            let response = UpdateResponse {
                message: "Handle updated successfully".to_string(),
                player: player_dto,
            };
            info!("Player {} updated handle to {}", email, update_request.handle);
            Ok(HttpResponse::Ok().json(response))
        },
        Err(PlayerError::InvalidPassword) => {
            info!("Invalid password for handle update attempt by {}", email);
            Err(PlayerError::InvalidPassword.into())
        },
        Err(PlayerError::AlreadyExists) => {
            info!("Handle update attempt with existing handle {} by {}", update_request.handle, email);
            Err(PlayerError::AlreadyExists.into())
        },
        Err(e) => {
            error!("Unexpected error updating handle for {}: {}", email, e);
            Err(e.into())
        },
    }
}

#[put("/handle")]
pub async fn update_handle_handler_prod(
    req: HttpRequest,
    update_request: web::Json<UpdateHandleRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    update_handle_handler_impl(req, update_request, repo).await
}

pub async fn update_password_handler_impl<R>(
    req: HttpRequest,
    update_request: web::Json<UpdatePasswordRequest>,
    repo: web::Data<R>,
) -> Result<HttpResponse, ApiError>
where
    R: PlayerRepository + Clone + 'static,
{
    let email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return Err(ApiError::unauthorized("Not authenticated")),
    };

    let usecase = PlayerUseCaseImpl { repo: repo.get_ref().clone() };
    
    match usecase.update_password(&email, &update_request.current_password, &update_request.new_password).await {
        Ok(player) => {
            let player_dto = PlayerDto::from(&player);
            let response = UpdateResponse {
                message: "Password updated successfully".to_string(),
                player: player_dto,
            };
            info!("Player {} updated password", email);
            Ok(HttpResponse::Ok().json(response))
        },
        Err(PlayerError::InvalidPassword) => {
            info!("Invalid current password for password update attempt by {}", email);
            Err(PlayerError::InvalidPassword.into())
        },
        Err(e) => {
            error!("Unexpected error updating password for {}: {}", email, e);
            Err(e.into())
        },
    }
}

#[put("/password")]
pub async fn update_password_handler_prod(
    req: HttpRequest,
    update_request: web::Json<UpdatePasswordRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> Result<HttpResponse, ApiError> {
    update_password_handler_impl(req, update_request, repo).await
}
