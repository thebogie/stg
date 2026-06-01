//! HTTP handlers for contest thumbnails (stored as WebP).

use crate::contest::image::{
    delete_image_file, enrich_contest_dto, image_file_exists, read_image_file, write_image_atomic,
    ImageVariant,
};
use crate::contest::repository::ContestRepositoryImpl;
use crate::db::Db;
use crate::player::repository::PlayerRepositoryImpl;
use crate::surreal_helpers::{canonical_id_from_http_path_param, record_id_to_key};
use actix_web::{delete, get, put, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use shared::dto::contest::ContestDto;

fn admin_email_from_env(email: &str) -> bool {
    let list = match std::env::var("ADMIN_EMAILS") {
        Ok(s) if !s.trim().is_empty() => s,
        _ => return false,
    };
    let email_lower = email.trim().to_lowercase();
    list.split(',')
        .any(|e| e.trim().to_lowercase() == email_lower)
}

fn contest_is_visible_to_viewer(
    dto: &ContestDto,
    viewer_player_id: &str,
    viewer_is_admin: bool,
) -> bool {
    use shared::models::contest_moderation::moderation_status;
    let status = dto.moderation_status.as_str();
    if status.is_empty() || status == moderation_status::APPROVED {
        return true;
    }
    if viewer_is_admin {
        return true;
    }
    let creator_key = record_id_to_key(&dto.creator_id, "player");
    let viewer_key = record_id_to_key(viewer_player_id, "player");
    !creator_key.is_empty() && creator_key == viewer_key
}

async fn viewer_context(
    req: &HttpRequest,
    player_repo: &PlayerRepositoryImpl,
) -> Result<(String, String, bool), HttpResponse> {
    let email = req.extensions().get::<String>().cloned().ok_or_else(|| {
        HttpResponse::Unauthorized().json(json!({ "error": "Authentication required" }))
    })?;
    let player = player_repo.find_by_email_for_auth(email.as_str()).await.ok_or_else(|| {
        HttpResponse::Unauthorized().json(json!({ "error": "user_not_found" }))
    })?;
    let is_admin = player.is_admin || admin_email_from_env(&email);
    Ok((email, player.id, is_admin))
}

async fn load_contest_for_image(
    contest_id: &str,
    repo: &ContestRepositoryImpl,
    db: &Db,
) -> Result<ContestDto, HttpResponse> {
    let id = if contest_id.contains('/') {
        contest_id.to_string()
    } else {
        format!("contest/{}", contest_id)
    };
    repo.find_details_by_id_using(&id, db)
        .await
        .ok_or_else(|| {
            HttpResponse::NotFound().json(json!({ "error": "Contest not found" }))
        })
}

fn can_mutate_image(dto: &ContestDto, viewer_id: &str, is_admin: bool) -> bool {
    if is_admin {
        return true;
    }
    record_id_to_key(&dto.creator_id, "player") == record_id_to_key(viewer_id, "player")
}

fn upload_content_type_allowed(content_type: &str) -> bool {
    let ct = content_type.split(';').next().unwrap_or(content_type).trim();
    ct.starts_with("image/")
        || ct.eq_ignore_ascii_case("application/octet-stream")
}

/// Upload or replace contest thumbnail (JPEG/PNG/WebP body; stored as resized WebP).
#[put("/{contest_id}/image")]
pub async fn upload_contest_image_handler(
    path: web::Path<String>,
    body: web::Bytes,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
) -> impl Responder {
    let param = path.into_inner();
    let contest_key = record_id_to_key(&canonical_id_from_http_path_param("contest", &param), "contest");
    if contest_key.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "Invalid contest id" }));
    }

    let content_type = req
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !upload_content_type_allowed(content_type) {
        return HttpResponse::BadRequest().json(json!({
            "error": "Content-Type must be an image (e.g. image/jpeg, image/png, image/webp)"
        }));
    }

    let (_, viewer_id, is_admin) = match viewer_context(&req, &repo.player_usecase.repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };

    let mut dto = match load_contest_for_image(&format!("contest/{}", contest_key), &repo, &db).await
    {
        Ok(d) => d,
        Err(r) => return r,
    };

    if !can_mutate_image(&dto, &viewer_id, is_admin) {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }

    if let Err(e) = write_image_atomic(&contest_key, &body) {
        return HttpResponse::BadRequest().json(json!({ "error": e }));
    }

    if let Err(e) = repo.set_contest_has_image(&contest_key, true, db.get_ref()).await {
        log::error!("set_contest_has_image: {}", e);
        delete_image_file(&contest_key);
        return HttpResponse::InternalServerError().json(json!({ "error": e }));
    }

    dto.has_image = true;
    enrich_contest_dto(&mut dto);
    HttpResponse::Ok().json(dto)
}

/// Serve contest detail image (~512px edge WebP) for lightbox/hover; falls back to thumb if missing.
#[get("/{contest_id}/image/detail")]
pub async fn get_contest_image_detail_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
) -> impl Responder {
    serve_contest_image(path, req, repo, db, ImageVariant::Detail).await
}

/// Serve contest list thumbnail (WebP, or legacy PNG).
#[get("/{contest_id}/image")]
pub async fn get_contest_image_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
) -> impl Responder {
    serve_contest_image(path, req, repo, db, ImageVariant::Thumb).await
}

async fn serve_contest_image(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
    variant: ImageVariant,
) -> HttpResponse {
    let param = path.into_inner();
    let contest_key = record_id_to_key(&canonical_id_from_http_path_param("contest", &param), "contest");
    if contest_key.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "Invalid contest id" }));
    }

    let (_, viewer_id, is_admin) = match viewer_context(&req, &repo.player_usecase.repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };

    let dto = match load_contest_for_image(&format!("contest/{}", contest_key), &repo, &db).await {
        Ok(d) => d,
        Err(r) => return r,
    };

    if !contest_is_visible_to_viewer(&dto, &viewer_id, is_admin) {
        return HttpResponse::NotFound().json(json!({ "error": "Contest not found" }));
    }

    if !dto.has_image && !image_file_exists(&contest_key) {
        return HttpResponse::NotFound().json(json!({ "error": "No image for this contest" }));
    }

    let (bytes, mime) = match read_image_file(&contest_key, variant) {
        Some(v) => v,
        None => {
            return HttpResponse::NotFound().json(json!({ "error": "No image for this contest" }))
        }
    };

    HttpResponse::Ok()
        .content_type(mime)
        .insert_header(("Cache-Control", "public, max-age=86400"))
        .body(bytes)
}

/// Remove contest thumbnail.
#[delete("/{contest_id}/image")]
pub async fn delete_contest_image_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<ContestRepositoryImpl>,
    db: web::Data<Db>,
) -> impl Responder {
    let param = path.into_inner();
    let contest_key = record_id_to_key(&canonical_id_from_http_path_param("contest", &param), "contest");
    if contest_key.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "Invalid contest id" }));
    }

    let (_, viewer_id, is_admin) = match viewer_context(&req, &repo.player_usecase.repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };

    let dto = match load_contest_for_image(&format!("contest/{}", contest_key), &repo, &db).await {
        Ok(d) => d,
        Err(r) => return r,
    };

    if !can_mutate_image(&dto, &viewer_id, is_admin) {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }

    delete_image_file(&contest_key);
    if let Err(e) = repo.set_contest_has_image(&contest_key, false, db.get_ref()).await {
        log::error!("clear has_image: {}", e);
        return HttpResponse::InternalServerError().json(json!({ "error": e }));
    }

    HttpResponse::NoContent().finish()
}
