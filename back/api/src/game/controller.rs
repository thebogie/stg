use crate::game::repository::{GameRepository, GameRepositoryImpl};
use crate::game::usecase::{GameUseCase, GameUseCaseImpl};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use shared::dto::game::GameDto;
use validator::Validate;

pub async fn get_game_handler_impl<R>(path: web::Path<String>, repo: web::Data<R>) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let param = path.into_inner();
    let id = if param.contains('/') {
        param
    } else {
        format!("game/{}", param)
    };
    match usecase.get_game(&id).await {
        Ok(game) => {
            let game_dto = GameDto::from(&game);
            HttpResponse::Ok().json(game_dto)
        }
        Err(e) => HttpResponse::NotFound().body(e),
    }
}

#[get("/{id:[^/]+|game/[^/]+}")]
pub async fn get_game_handler(
    path: web::Path<String>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    get_game_handler_impl::<GameRepositoryImpl>(path, repo).await
}

pub async fn get_all_games_handler_impl<R>(repo: web::Data<R>) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.get_all_games().await {
        Ok(games) => {
            let game_dtos: Vec<GameDto> = games.iter().map(|g| GameDto::from(g)).collect();
            HttpResponse::Ok().json(game_dtos)
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("")]
pub async fn get_all_games_handler(repo: web::Data<GameRepositoryImpl>) -> impl Responder {
    get_all_games_handler_impl::<GameRepositoryImpl>(repo).await
}

pub async fn create_game_handler_impl<R>(
    game_dto: web::Json<GameDto>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    if let Err(e) = game_dto.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.create_game(game_dto.into_inner()).await {
        Ok(game) => {
            let game_dto = GameDto::from(&game);
            HttpResponse::Created().json(game_dto)
        }
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

#[post("")]
pub async fn create_game_handler(
    game_dto: web::Json<GameDto>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    create_game_handler_impl::<GameRepositoryImpl>(game_dto, repo).await
}

pub async fn update_game_handler_impl<R>(
    path: web::Path<String>,
    game_dto: web::Json<GameDto>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    if let Err(e) = game_dto.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let param = path.into_inner();
    let id = if param.contains('/') {
        param
    } else {
        format!("game/{}", param)
    };
    match usecase.update_game(&id, game_dto.into_inner()).await {
        Ok(game) => {
            let game_dto = GameDto::from(&game);
            HttpResponse::Ok().json(game_dto)
        }
        Err(e) => {
            if e.contains("not found") {
                HttpResponse::NotFound().body(e)
            } else {
                HttpResponse::BadRequest().body(e)
            }
        }
    }
}

#[put("/{id:[^/]+|game/[^/]+}")]
pub async fn update_game_handler(
    path: web::Path<String>,
    game_dto: web::Json<GameDto>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    update_game_handler_impl::<GameRepositoryImpl>(path, game_dto, repo).await
}

pub async fn delete_game_handler_impl<R>(
    path: web::Path<String>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let param = path.into_inner();
    let id = if param.contains('/') {
        param
    } else {
        format!("game/{}", param)
    };
    match usecase.delete_game(&id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            if e.contains("not found") {
                HttpResponse::NotFound().body(e)
            } else {
                HttpResponse::InternalServerError().body(e)
            }
        }
    }
}

#[delete("/{id:[^/]+|game/[^/]+}")]
pub async fn delete_game_handler(
    path: web::Path<String>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    delete_game_handler_impl::<GameRepositoryImpl>(path, repo).await
}

pub async fn search_games_handler_impl<R>(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let empty_string = String::new();
    let search_query = query.get("query").unwrap_or(&empty_string);

    if search_query.is_empty() {
        return HttpResponse::BadRequest().body("Query parameter is required");
    }

    match usecase.search_games_dto(search_query).await {
        Ok(game_dtos) => HttpResponse::Ok().json(game_dtos),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/search")]
pub async fn search_games_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    search_games_handler_impl::<GameRepositoryImpl>(query, repo).await
}

// DB-only alias for clarity
#[get("/db_search")]
pub async fn search_games_db_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    let empty_string = String::new();
    let search_query = query.get("query").unwrap_or(&empty_string);
    if search_query.is_empty() {
        return HttpResponse::BadRequest().body("Query parameter is required");
    }
    let game_dtos = repo.search_db_only_dto(search_query).await;
    HttpResponse::Ok().json(game_dtos)
}

// Enhanced analytics endpoints
pub async fn get_game_recommendations_handler_impl<R>(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let player_id = path.into_inner();
    let id = if player_id.contains('/') {
        player_id
    } else {
        format!("player/{}", player_id)
    };

    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<i32>().ok())
        .unwrap_or(5);

    match usecase.get_game_recommendations(&id, limit).await {
        Ok(recommendations) => HttpResponse::Ok().json(recommendations),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/games/recommendations/{player_id:.*}")]
pub async fn get_game_recommendations_handler(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    get_game_recommendations_handler_impl::<GameRepositoryImpl>(path, query, repo).await
}

pub async fn get_similar_games_handler_impl<R>(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let game_id = path.into_inner();
    let id = if game_id.contains('/') {
        game_id
    } else {
        format!("game/{}", game_id)
    };

    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<i32>().ok())
        .unwrap_or(5);

    match usecase.get_similar_games(&id, limit).await {
        Ok(similar_games) => HttpResponse::Ok().json(similar_games),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/games/similar/{game_id}")]
pub async fn get_similar_games_handler(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    get_similar_games_handler_impl::<GameRepositoryImpl>(path, query, repo).await
}

pub async fn get_popular_games_handler_impl<R>(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: GameRepository + Clone + 'static,
{
    let usecase = GameUseCaseImpl {
        repo: repo.get_ref().clone(),
    };

    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<i32>().ok())
        .unwrap_or(10);

    match usecase.get_popular_games(limit).await {
        Ok(popular_games) => HttpResponse::Ok().json(popular_games),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/games/popular")]
pub async fn get_popular_games_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<GameRepositoryImpl>,
) -> impl Responder {
    get_popular_games_handler_impl::<GameRepositoryImpl>(query, repo).await
}
