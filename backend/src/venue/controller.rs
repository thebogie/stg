use actix_web::{web, HttpResponse, Responder, get, post, put, delete};
use validator::Validate;
use shared::dto::venue::VenueDto;
use crate::venue::usecase::{VenueUseCase, VenueUseCaseImpl};
use crate::venue::repository::{VenueRepository, VenueRepositoryImpl};

pub async fn get_venue_handler_impl<R>(
    path: web::Path<String>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let param = path.into_inner();
    let id = if param.contains('/') { param } else { format!("venue/{}", param) };
    match usecase.get_venue(&id).await {
        Ok(venue) => {
            let venue_dto = VenueDto::from(&venue);
            HttpResponse::Ok().json(venue_dto)
        },
        Err(e) => HttpResponse::NotFound().body(e),
    }
}

#[get("/{id}")]
pub async fn get_venue_handler(
    path: web::Path<String>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    let id_param = path.into_inner();
    let id = if id_param.contains('/') { id_param } else { format!("venue/{}", id_param) };
    match repo.get_venue_with_timezone(&id).await {
        Ok(venue_dto) => HttpResponse::Ok().json(venue_dto),
        Err(e) => HttpResponse::NotFound().body(e),
    }
}

pub async fn get_all_venues_handler_impl<R>(
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    match usecase.get_all_venues().await {
        Ok(venues) => {
            let venue_dtos: Vec<VenueDto> = venues.iter().map(|v| VenueDto::from(v)).collect();
            HttpResponse::Ok().json(venue_dtos)
        },
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("")]
pub async fn get_all_venues_handler(
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    get_all_venues_handler_impl::<VenueRepositoryImpl>(repo).await
}

pub async fn create_venue_handler_impl<R>(
    venue_dto: web::Json<VenueDto>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    if let Err(e) = venue_dto.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    match usecase.create_venue(venue_dto.into_inner()).await {
        Ok(venue) => {
            let venue_dto = VenueDto::from(&venue);
            HttpResponse::Created().json(venue_dto)
        },
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

#[post("")]
pub async fn create_venue_handler(
    venue_dto: web::Json<VenueDto>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    create_venue_handler_impl::<VenueRepositoryImpl>(venue_dto, repo).await
}

pub async fn update_venue_handler_impl<R>(
    path: web::Path<String>,
    venue_dto: web::Json<VenueDto>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    if let Err(e) = venue_dto.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let param = path.into_inner();
    let id = if param.contains('/') { param } else { format!("venue/{}", param) };
    match usecase.update_venue(&id, venue_dto.into_inner()).await {
        Ok(venue) => {
            let venue_dto = VenueDto::from(&venue);
            HttpResponse::Ok().json(venue_dto)
        },
        Err(e) => {
            if e.contains("not found") {
                HttpResponse::NotFound().body(e)
            } else {
                HttpResponse::BadRequest().body(e)
            }
        },
    }
}

#[put("/{id}")]
pub async fn update_venue_handler(
    path: web::Path<String>,
    venue_dto: web::Json<VenueDto>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    update_venue_handler_impl::<VenueRepositoryImpl>(path, venue_dto, repo).await
}

pub async fn delete_venue_handler_impl<R>(
    path: web::Path<String>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let param = path.into_inner();
    let id = if param.contains('/') { param } else { format!("venue/{}", param) };
    match usecase.delete_venue(&id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            if e.contains("not found") {
                HttpResponse::NotFound().body(e)
            } else {
                HttpResponse::InternalServerError().body(e)
            }
        },
    }
}

#[delete("/{id}")]
pub async fn delete_venue_handler(
    path: web::Path<String>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    delete_venue_handler_impl::<VenueRepositoryImpl>(path, repo).await
}

pub async fn search_venues_handler_impl<R>(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let empty_string = String::new();
    let search_query = query.get("query").unwrap_or(&empty_string);
    
    if search_query.is_empty() {
        return HttpResponse::BadRequest().body("Query parameter is required");
    }
    
    match usecase.search_venues_dto(search_query).await {
        Ok(venue_dtos) => {
            HttpResponse::Ok().json(venue_dtos)
        },
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/search")]
pub async fn search_venues_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    search_venues_handler_impl::<VenueRepositoryImpl>(query, repo).await
}

// DB-only alias for clarity
#[get("/db_search")]
pub async fn search_venues_db_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    search_venues_handler_impl::<VenueRepositoryImpl>(query, repo).await
}

// External search for create pages (includes Google Places API)
#[get("/create_search")]
pub async fn search_venues_create_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let empty_string = String::new();
    let search_query = query.get("query").unwrap_or(&empty_string);
    
    if search_query.is_empty() {
        return HttpResponse::BadRequest().body("Query parameter is required");
    }
    
    match usecase.search_venues_dto_with_external(search_query).await {
        Ok(venue_dtos) => {
            HttpResponse::Ok().json(venue_dtos)
        },
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

// Enhanced analytics endpoints
pub async fn get_venue_performance_handler_impl<R>(
    path: web::Path<String>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let venue_id = path.into_inner();
    let id = if venue_id.contains('/') { venue_id } else { format!("venue/{}", venue_id) };
    
    match usecase.get_venue_performance(&id).await {
        Ok(performance) => HttpResponse::Ok().json(performance),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/venues/performance/{venue_id}")]
pub async fn get_venue_performance_handler(
    path: web::Path<String>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    get_venue_performance_handler_impl::<VenueRepositoryImpl>(path, repo).await
}

pub async fn get_player_venue_stats_handler_impl<R>(
    path: web::Path<String>,
    repo: web::Data<R>,
) -> impl Responder
where
    R: VenueRepository + Clone + 'static,
{
    let usecase = VenueUseCaseImpl { repo: repo.get_ref().clone() };
    let player_id = path.into_inner();
    let id = if player_id.contains('/') { player_id } else { format!("player/{}", player_id) };
    
    match usecase.get_player_venue_stats(&id).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/venues/player-stats/{player_id:.*}")]
pub async fn get_player_venue_stats_handler(
    path: web::Path<String>,
    repo: web::Data<VenueRepositoryImpl>,
) -> impl Responder {
    get_player_venue_stats_handler_impl::<VenueRepositoryImpl>(path, repo).await
}