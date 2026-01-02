use actix_web::{web, HttpResponse};
use serde::Deserialize;
use crate::third_party::google::timezone::GoogleTimezoneService;

#[derive(Deserialize)]
pub struct ResolveQuery {
	pub lat: Option<f64>,
	pub lng: Option<f64>,
	pub place_id: Option<String>,
}

pub fn configure_routes(cfg: &mut web::ServiceConfig, google_api_url: String, google_api_key: String) {
	let service = web::Data::new(GoogleTimezoneService::new(google_api_url, google_api_key));
	cfg.service(
		web::scope("/api/timezone")
			.app_data(service.clone())
			.route(
				"/resolve",
				web::get().to(|query: web::Query<ResolveQuery>, svc: web::Data<GoogleTimezoneService>| async move {
					let tz = if let Some(place_id) = &query.place_id {
						// Use place_id if provided
						svc.infer_timezone_from_place_id(place_id).await
					} else if let (Some(lat), Some(lng)) = (query.lat, query.lng) {
						// Fall back to coordinates
						svc.infer_timezone_from_coordinates(lat, lng).await
					} else {
						"UTC".to_string()
					};
					Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"timezone": tz})))
				}),
			),
	);
}


