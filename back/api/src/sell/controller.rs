//! HTTP handlers for Sell a Game workflow.

use crate::player::repository::PlayerRepositoryImpl;
use crate::sell::automation;
use crate::sell::image::upload_content_type_allowed;
use crate::sell::image::SellPhotoVariant;
use crate::sell::preferences_repository::SellPreferencesRepositoryImpl;
use crate::sell::repository::SellListingRepositoryImpl;
use crate::surreal_helpers::{canonical_id_from_http_path_param, record_id_to_key};
use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use shared::dto::sell_listing::{
    AiExtractionResultDto, AutomationResultRequest, BggMatchRequest, SellListingDto,
    SellListingPhotoDto, UpdateSellListingDraftRequest,
};
use shared::dto::sell_preferences::{
    playwright_job_status, BggAutomateRequest, BggAutomateResponse, SellPreferencesDto,
};
use validator::Validate;

async fn viewer_player_id(
    req: &HttpRequest,
    player_repo: &PlayerRepositoryImpl,
) -> Result<String, HttpResponse> {
    let email = req.extensions().get::<String>().cloned().ok_or_else(|| {
        HttpResponse::Unauthorized().json(json!({ "error": "Authentication required" }))
    })?;
    let player = player_repo
        .find_by_email_for_auth(email.as_str())
        .await
        .ok_or_else(|| HttpResponse::Unauthorized().json(json!({ "error": "user_not_found" })))?;
    Ok(player.id)
}

fn can_access_listing(dto: &SellListingDto, viewer_id: &str) -> bool {
    record_id_to_key(&dto.seller_id, "player") == record_id_to_key(viewer_id, "player")
}

async fn load_listing_for_viewer(
    listing_id: &str,
    viewer_id: &str,
    repo: &SellListingRepositoryImpl,
) -> Result<SellListingDto, HttpResponse> {
    let listing = repo
        .find_by_id(listing_id)
        .await
        .ok_or_else(|| HttpResponse::NotFound().json(json!({ "error": "listing not found" })))?;
    let dto = SellListingDto::from(&listing);
    if !can_access_listing(&dto, viewer_id) {
        return Err(HttpResponse::Forbidden().json(json!({ "error": "forbidden" })));
    }
    Ok(dto)
}

fn listing_key_param(param: &str) -> String {
    record_id_to_key(
        &canonical_id_from_http_path_param("sell_listing", param),
        "sell_listing",
    )
}

#[post("")]
pub async fn create_listing_handler(
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match repo.create_listing(&viewer_id).await {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[get("")]
pub async fn list_listings_handler(
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let listings = repo.list_by_seller(&viewer_id).await;
    HttpResponse::Ok().json(listings)
}

#[get("/{listing_id}")]
pub async fn get_listing_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let param = path.into_inner();
    let id = format!("sell_listing/{}", listing_key_param(&param));
    match load_listing_for_viewer(&id, &viewer_id, &repo).await {
        Ok(mut dto) => {
            let photos = repo.list_photos(&id).await;
            dto.photos = photos
                .iter()
                .map(|p| {
                    let mut pd = SellListingPhotoDto::from(p);
                    let pk = record_id_to_key(&p.id, "sell_listing_photo");
                    let lk = listing_key_param(&param);
                    pd.preview_url = Some(format!("/api/sell/listings/{lk}/photos/{pk}"));
                    pd
                })
                .collect();
            HttpResponse::Ok().json(dto)
        }
        Err(r) => r,
    }
}

#[put("/{listing_id}/photos")]
pub async fn upload_photo_handler(
    path: web::Path<String>,
    body: web::Bytes,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let param = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&param));
    let content_type = req
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg");
    if !upload_content_type_allowed(content_type) {
        return HttpResponse::BadRequest().json(json!({
            "error": "Content-Type must be an image"
        }));
    }
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo
        .add_photo(&listing_id, content_type, &body)
        .await
    {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[get("/{listing_id}/photos/{photo_id}")]
pub async fn get_photo_handler(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let (listing_param, photo_param) = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&listing_param));
    let photo_id = format!(
        "sell_listing_photo/{}",
        record_id_to_key(
            &canonical_id_from_http_path_param("sell_listing_photo", &photo_param),
            "sell_listing_photo",
        )
    );
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.read_photo_bytes(&listing_id, &photo_id, SellPhotoVariant::Thumb) {
        Some((bytes, mime)) => HttpResponse::Ok()
            .content_type(mime)
            .insert_header(("Cache-Control", "private, max-age=300"))
            .body(bytes),
        None => HttpResponse::NotFound().json(json!({ "error": "photo not found" })),
    }
}

#[get("/{listing_id}/photos/{photo_id}/detail")]
pub async fn get_photo_detail_handler(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let (listing_param, photo_param) = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&listing_param));
    let photo_id = format!(
        "sell_listing_photo/{}",
        record_id_to_key(
            &canonical_id_from_http_path_param("sell_listing_photo", &photo_param),
            "sell_listing_photo",
        )
    );
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.read_photo_bytes(&listing_id, &photo_id, SellPhotoVariant::Detail) {
        Some((bytes, mime)) => HttpResponse::Ok()
            .content_type(mime)
            .insert_header(("Cache-Control", "private, max-age=300"))
            .body(bytes),
        None => HttpResponse::NotFound().json(json!({ "error": "photo not found" })),
    }
}

#[delete("/{listing_id}/photos/{photo_id}")]
pub async fn delete_photo_handler(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let (listing_param, photo_param) = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&listing_param));
    let photo_id = format!(
        "sell_listing_photo/{}",
        record_id_to_key(
            &canonical_id_from_http_path_param("sell_listing_photo", &photo_param),
            "sell_listing_photo",
        )
    );
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.delete_photo(&listing_id, &photo_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({ "ok": true })),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[post("/{listing_id}/checkpoint/{checkpoint}")]
pub async fn approve_checkpoint_handler(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let (listing_param, checkpoint) = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&listing_param));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo
        .approve_checkpoint(&listing_id, &checkpoint, &viewer_id)
        .await
    {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[post("/{listing_id}/extract")]
pub async fn extract_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.run_extraction(&listing_id).await {
        Ok((listing, clarify)) => {
            HttpResponse::Ok().json(AiExtractionResultDto { listing, clarify })
        }
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[put("/{listing_id}/draft")]
pub async fn update_draft_handler(
    path: web::Path<String>,
    body: web::Json<UpdateSellListingDraftRequest>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.update_draft(&listing_id, body.into_inner()).await {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[put("/{listing_id}/bgg-match")]
pub async fn bgg_match_handler(
    path: web::Path<String>,
    body: web::Json<BggMatchRequest>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    let inner = body.into_inner();
    match repo
        .set_bgg_match(&listing_id, inner.bgg_id, &inner.game_name)
        .await
    {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[post("/{listing_id}/automate")]
pub async fn automate_handler(
    path: web::Path<String>,
    body: web::Json<BggAutomateRequest>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    prefs_repo: web::Data<SellPreferencesRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
    redis_client: web::Data<redis::Client>,
) -> impl Responder {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": "validation_failed",
            "details": e.to_string(),
        }));
    }
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }

    let prefs = prefs_repo.get_or_default(&viewer_id).await;
    let payload = match repo.build_export(&listing_id, &prefs).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({ "error": e })),
    };

    let username = body.bgg_username.clone();
    let password = body.bgg_password.clone();

    // Remember username only — password is never stored.
    let mut prefs_to_save = prefs.clone();
    prefs_to_save.bgg_username = Some(username.clone());
    let _ = prefs_repo.upsert(&viewer_id, prefs_to_save).await;

    if crate::sell::playwright_queue::is_queue_mode() {
        match automation::enqueue_bgg_automation(
            redis_client.get_ref(),
            &listing_id,
            &viewer_id,
            &payload,
            &username,
            &password,
        )
        .await
        {
            Ok(job_id) => HttpResponse::Accepted().json(BggAutomateResponse {
                listing_id: listing_id.clone(),
                status: playwright_job_status::QUEUED.to_string(),
                message: "BGG automation queued — filling form on BoardGameGeek.".to_string(),
                job_id: Some(job_id),
            }),
            Err(e) => HttpResponse::BadGateway().json(json!({ "error": e })),
        }
    } else {
        let result = automation::run_bgg_automation_local(&payload, &username, &password).await;
        match result {
            Ok(message) => {
                let _ = repo
                    .record_automation_result(&listing_id, true, None, None, false)
                    .await;
                HttpResponse::Ok().json(BggAutomateResponse {
                    listing_id: listing_id.clone(),
                    status: "bgg_preview".to_string(),
                    message,
                    job_id: None,
                })
            }
            Err(e) => {
                let _ = repo
                    .record_automation_result(&listing_id, false, None, Some(e.clone()), false)
                    .await;
                HttpResponse::BadGateway().json(json!({ "error": e }))
            }
        }
    }
}

#[get("/{listing_id}/automate/{job_id}/status")]
pub async fn automate_job_status_handler(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
    redis_client: web::Data<redis::Client>,
) -> impl Responder {
    let (listing_param, job_id) = path.into_inner();
    let listing_id = format!("sell_listing/{}", listing_key_param(&listing_param));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }

    match crate::sell::playwright_queue::get_job_status(
        redis_client.get_ref(),
        &job_id,
        &listing_id,
        &viewer_id,
    )
    .await
    {
        Ok(status) => {
            let terminal = status.status == playwright_job_status::COMPLETED
                || status.status == playwright_job_status::FAILED;
            if terminal {
                if let Ok(true) =
                    crate::sell::playwright_queue::try_mark_finalized(redis_client.get_ref(), &job_id)
                        .await
                {
                    let success = status.status == playwright_job_status::COMPLETED;
                    let err = status.error.clone();
                    let _ = repo
                        .record_automation_result(&listing_id, success, None, err, false)
                        .await;
                }
            }
            HttpResponse::Ok().json(status)
        }
        Err(e) => {
            if e == "forbidden" {
                HttpResponse::Forbidden().json(json!({ "error": e }))
            } else {
                HttpResponse::NotFound().json(json!({ "error": e }))
            }
        }
    }
}

#[get("/{listing_id}/export")]
pub async fn export_listing_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    prefs_repo: web::Data<SellPreferencesRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    let prefs = prefs_repo.get_or_default(&viewer_id).await;
    match repo.build_export(&listing_id, &prefs).await {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[post("/{listing_id}/automation/result")]
pub async fn automation_result_handler(
    path: web::Path<String>,
    body: web::Json<AutomationResultRequest>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    let inner = body.into_inner();
    match repo
        .record_automation_result(
            &listing_id,
            inner.success,
            inner.bgg_listing_url,
            inner.error_message,
            inner.submitted_on_bgg,
        )
        .await
    {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[delete("/{listing_id}")]
pub async fn cancel_listing_handler(
    path: web::Path<String>,
    req: HttpRequest,
    repo: web::Data<SellListingRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let listing_id = format!("sell_listing/{}", listing_key_param(&path.into_inner()));
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if load_listing_for_viewer(&listing_id, &viewer_id, &repo)
        .await
        .is_err()
    {
        return HttpResponse::Forbidden().json(json!({ "error": "forbidden" }));
    }
    match repo.cancel_listing(&listing_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({ "ok": true })),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

#[get("")]
pub async fn get_preferences_handler(
    req: HttpRequest,
    prefs_repo: web::Data<SellPreferencesRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let mut dto = prefs_repo.get_or_default(&viewer_id).await;
    if !prefs_repo.has_preferences(&viewer_id).await {
        dto.updated_at = None;
    }
    HttpResponse::Ok().json(dto)
}

#[put("")]
pub async fn put_preferences_handler(
    body: web::Json<SellPreferencesDto>,
    req: HttpRequest,
    prefs_repo: web::Data<SellPreferencesRepositoryImpl>,
    player_repo: web::Data<PlayerRepositoryImpl>,
) -> impl Responder {
    let viewer_id = match viewer_player_id(&req, &player_repo).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match prefs_repo.upsert(&viewer_id, body.into_inner()).await {
        Ok(dto) => HttpResponse::Ok().json(dto),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}
