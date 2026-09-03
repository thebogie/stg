use actix_web::{web, App, HttpServer};
use backend::config::LoggingConfig;
use backend::db::{connect_surreal, Db};
use backend::error::ApiError;
use backend::observability::events::log_shutdown;
use backend::observability::{init_observability, AppSpanGuard};
use backend::player::session::RedisSessionStore;
use backend::third_party::BGGService;
use utoipa::OpenApi;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => log_shutdown("SIGINT"),
        _ = terminate => log_shutdown("SIGTERM"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let logging = LoggingConfig::from_env();
    let _obs: AppSpanGuard = init_observability(&logging).map_err(std::io::Error::other)?;

    let config = match backend::config::Config::load() {
        Ok(config) => config,
        Err(e) => {
            backend::observability::events::log_config_error(&e.to_string());
            return Err(std::io::Error::other(e.to_string()));
        }
    };

    backend::observability::events::log_startup(
        &config.logging,
        &config.server.host,
        config.server.port,
    );

    // Initialize Redis client
    let redis_client = match redis::Client::open(config.redis.url.clone()) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(event = "redis.connect.error", error.message = %e, "failed to create Redis client");
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                e.to_string(),
            ));
        }
    };
    let redis_data = web::Data::new(redis_client.clone());
    let database_config_data = web::Data::new(config.database.clone());
    let session_store = web::Data::new(RedisSessionStore {
        client: redis_client.clone(),
    });
    let redis_client_for_ratings = redis_client.clone();

    let db: Db = match connect_surreal(&config.database).await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(event = "db.connect.error", error.message = %e, "SurrealDB connection failed");
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                e.to_string(),
            ));
        }
    };

    if let Err(e) = backend::contest::image::ensure_image_dir() {
        log::warn!("Could not create contest image directory: {}", e);
    }
    if let Err(e) = backend::sell::image::ensure_image_dir() {
        log::warn!("Could not create sell image directory: {}", e);
    }

    // Initialize Redis cache for repositories
    use backend::cache::{CacheTTL, RedisCache};
    use std::sync::Arc;
    let game_cache = Arc::new(RedisCache::new(
        redis_client.clone(),
        "stg:cache:game".to_string(),
        CacheTTL::game(),
    ));
    let venue_cache = Arc::new(RedisCache::new(
        redis_client.clone(),
        "stg:cache:venue".to_string(),
        CacheTTL::venue(),
    ));
    let player_cache = Arc::new(RedisCache::new(
        redis_client.clone(),
        "stg:cache:player".to_string(),
        CacheTTL::player(),
    ));
    log::info!("Redis cache initialized for games, venues, and players");

    // IMPORTANT: SurrealDB scope (NS/DB) does not reliably persist across pooled connections,
    // so repositories must set scope per-query for writes/reads to hit the expected database.
    let mut player_repo_impl = backend::player::repository::PlayerRepositoryImpl::new_with_cache(
        db.clone(),
        player_cache.clone(),
    );
    player_repo_impl.ns = Some(config.database.ns.clone());
    player_repo_impl.db_name = Some(config.database.name.clone());
    let player_repo_arc = std::sync::Arc::new(player_repo_impl.clone());
    let player_repo = web::Data::from(player_repo_arc.clone());

    // Initialize venue repository with Google Places API if configured
    let google_config = if let Some(api_key) = &config.google.location_api_key {
        log::info!(
            "Google Places API configured with URL: {}",
            config.google.api_url
        );
        Some((config.google.api_url.clone(), api_key.clone()))
    } else {
        log::warn!("Google Places API not configured - no API key provided");
        None
    };
    let venue_repo = web::Data::new(
        backend::venue::repository::VenueRepositoryImpl::new_with_cache_and_scope(
            db.clone(),
            google_config.clone(),
            venue_cache.clone(),
            config.database.ns.clone(),
            config.database.name.clone(),
        ),
    );

    // Initialize game repository with BGG service
    let bgg_service = BGGService::new_with_config(&config.bgg);
    log::info!("BGG API configured with URL: {}", config.bgg.api_url);
    if config.bgg.api_token.is_some() {
        log::info!("BGG API token configured (Bearer authentication enabled)");
    } else {
        log::warn!("BGG API token not configured - requests will be unauthenticated");
    }

    let game_repo = web::Data::new(
        backend::game::repository::GameRepositoryImpl::new_with_bgg_and_cache_and_scope(
            db.clone(),
            bgg_service,
            game_cache.clone(),
            config.database.ns.clone(),
            config.database.name.clone(),
        ),
    );

    // Initialize contest repository (with NS/DB scope so CREATE/INSERT run in the app's namespace/database).
    // Share the scoped player repo so create_contest can resolve the creator by session email (embedded
    // PlayerRepositoryImpl::new() would query the wrong NS/DB and return user_not_found).
    let contest_repo = web::Data::new(
        backend::contest::repository::ContestRepositoryImpl::new_with_google_config_and_player_repo(
            db.clone(),
            google_config,
            Some(player_repo_impl.clone()),
            Some(config.database.ns.clone()),
            Some(config.database.name.clone()),
        ),
    );

    // Initialize client analytics components
    let client_analytics_repo =
        backend::client_analytics::repository::ClientAnalyticsRepositoryImpl::new(
            db.clone(),
            config.database.clone(),
        );
    let client_analytics_usecase =
        backend::client_analytics::usecase::ClientAnalyticsUseCaseImpl::new(client_analytics_repo);
    let client_analytics_controller = web::Data::new(
        backend::client_analytics::controller::ClientAnalyticsController::new(
            client_analytics_usecase,
            db.clone(),
        ),
    );

    // Initialize ratings scheduler
    let ratings_repo = backend::ratings::repository::RatingsRepository::new(db.clone());
    let ratings_usecase = backend::ratings::usecase::RatingsUsecase::new(ratings_repo);
    let mut ratings_scheduler =
        backend::ratings::scheduler::RatingsScheduler::new(ratings_usecase.clone());

    // Start the ratings scheduler in the background
    if let Err(e) = ratings_scheduler.start().await {
        log::error!("Failed to start ratings scheduler: {}", e);
    } else {
        log::info!("Glicko2 ratings scheduler started successfully");
    }

    // Store scheduler in web::Data for health checks
    let scheduler_data = web::Data::new(ratings_scheduler.clone());

    // Sell listing repository and image cleanup scheduler
    let sell_listing_repo = web::Data::new(
        backend::sell::repository::SellListingRepositoryImpl::new_with_scope(
            db.clone(),
            config.database.ns.clone(),
            config.database.name.clone(),
        ),
    );
    let sell_prefs_repo = web::Data::new(
        backend::sell::preferences_repository::SellPreferencesRepositoryImpl::new_with_scope(
            db.clone(),
            config.database.ns.clone(),
            config.database.name.clone(),
        ),
    );
    let mut sell_cleanup_scheduler = backend::sell::scheduler::SellImageCleanupScheduler::new(
        backend::sell::repository::SellListingRepositoryImpl::new_with_scope(
            db.clone(),
            config.database.ns.clone(),
            config.database.name.clone(),
        ),
    );
    if let Err(e) = sell_cleanup_scheduler.start().await {
        log::error!("Failed to start sell image cleanup scheduler: {}", e);
    } else {
        log::info!("Sell listing image cleanup scheduler started");
    }

    // Analytics components will be initialized in the route configuration

    // Start HTTP server
    log::info!(
        "Starting server on {}:{}",
        config.server.host,
        config.server.port
    );

    // Store database in web::Data for health checks
    let db_data = web::Data::new(db.clone());

    // Initialize metrics (this will initialize the global instance)
    let metrics = match backend::metrics::Metrics::new() {
        Ok(m) => {
            log::info!("Metrics initialized successfully");
            let metrics_arc = std::sync::Arc::new(m);
            // Set global metrics instance
            backend::metrics::Metrics::set_global(metrics_arc.clone());
            metrics_arc
        }
        Err(e) => {
            log::error!("Failed to initialize metrics: {}", e);
            return Err(std::io::Error::other(format!(
                "Failed to initialize metrics: {}",
                e
            )));
        }
    };
    let metrics_data = web::Data::new(metrics.clone());

    let mut ratings_scheduler_shutdown = ratings_scheduler.clone();
    let mut sell_cleanup_scheduler_shutdown = sell_cleanup_scheduler.clone();

    let server = HttpServer::new(move || {
        // Configure JSON error handler to always return JSON (not HTML)
        let json_config = actix_web::web::JsonConfig::default()
            .limit(256 * 1024)
            .error_handler(|err, _req| {
                // Convert JSON deserialization errors to JSON responses
                let error = ApiError::bad_request(&format!("Invalid JSON: {}", err));
                error.into()
            });

        App::new()
            .wrap(backend::middleware::Logger::with_metrics(metrics.clone()))
            .wrap(backend::middleware::SecurityHeaders)
            .wrap(backend::middleware::cors_middleware())
            .app_data(metrics_data.clone())
            .app_data(json_config)
            .app_data(redis_data.clone())
            .app_data(database_config_data.clone())
            .app_data(db_data.clone())
            .app_data(scheduler_data.clone())
            .app_data(player_repo.clone())
            .app_data(venue_repo.clone())
            .app_data(game_repo.clone())
            .app_data(contest_repo.clone())
            .app_data(sell_listing_repo.clone())
            .app_data(sell_prefs_repo.clone())
            .app_data(session_store.clone())
            .service(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui/{_:.*}").url(
                "/api-docs/openapi.json",
                <backend::openapi::ApiDoc as OpenApi>::openapi(),
            ))
            .service(backend::health::health_check)
            .service(backend::health::surreal_db_check)
            .service(backend::health::detailed_health_check)
            .service(backend::health::scheduler_health_check)
            .service(backend::health::version_info)
            .service(backend::health::version_info_root)
            .service(backend::health::metrics_endpoint)
            .service(
                web::scope("/api/ai")
                    .service(backend::ai::controller::ask_handler)
                    .service(backend::ai::controller::smacktalk_handler)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                            })
                            .service(backend::ai::controller::ask_my_view_handler),
                    ),
            )
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
                    .service(backend::player::controller::logout_handler_prod)
                    .service(backend::player::controller::search_players_handler)
                    .service(backend::player::controller::search_players_db_handler)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                            })
                            .service(backend::player::controller::me_handler_prod)
                            .service(backend::player::controller::update_email_handler_prod)
                            .service(backend::player::controller::update_handle_handler_prod)
                            .service(backend::player::controller::update_password_handler_prod),
                    ),
            )
            .service(
                web::scope("/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
                    .service(backend::player::controller::logout_handler_prod)
                    .service(backend::player::controller::search_players_handler)
                    .service(backend::player::controller::search_players_db_handler)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                            })
                            .service(backend::player::controller::me_handler_prod)
                            .service(backend::player::controller::update_email_handler_prod)
                            .service(backend::player::controller::update_handle_handler_prod)
                            .service(backend::player::controller::update_password_handler_prod),
                    ),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::search_venues_handler)
                    .service(backend::venue::controller::search_venues_db_handler)
                    .service(backend::venue::controller::search_venues_create_handler)
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler),
            )
            .service(
                web::scope("/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::search_venues_handler)
                    .service(backend::venue::controller::search_venues_db_handler)
                    .service(backend::venue::controller::search_venues_create_handler)
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::search_games_handler)
                    .service(backend::game::controller::search_games_db_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            )
            .service(
                web::scope("/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::search_games_handler)
                    .service(backend::game::controller::search_games_db_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(web::PayloadConfig::default().limit(
                        backend::contest::image::CONTEST_IMAGE_UPLOAD_MAX_BYTES,
                    ))
                    .app_data(player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::update_contest_handler)
                    .service(backend::contest::controller::get_player_game_contests_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::image_handlers::get_contest_image_detail_handler)
                    .service(backend::contest::image_handlers::get_contest_image_handler)
                    .service(backend::contest::image_handlers::upload_contest_image_handler)
                    .service(backend::contest::image_handlers::delete_contest_image_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                                player_repo: player_repo_arc.clone(),
                            })
                            .app_data(contest_repo.clone())
                            .service(backend::contest::controller::list_pending_contests_handler)
                            .service(backend::contest::controller::approve_contest_handler)
                            .service(backend::contest::controller::reject_contest_handler)
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            )
            .service(
                web::scope("/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(web::PayloadConfig::default().limit(
                        backend::contest::image::CONTEST_IMAGE_UPLOAD_MAX_BYTES,
                    ))
                    .app_data(player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::update_contest_handler)
                    .service(backend::contest::controller::get_player_game_contests_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::image_handlers::get_contest_image_detail_handler)
                    .service(backend::contest::image_handlers::get_contest_image_handler)
                    .service(backend::contest::image_handlers::upload_contest_image_handler)
                    .service(backend::contest::image_handlers::delete_contest_image_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                                player_repo: player_repo_arc.clone(),
                            })
                            .app_data(contest_repo.clone())
                            .service(backend::contest::controller::list_pending_contests_handler)
                            .service(backend::contest::controller::approve_contest_handler)
                            .service(backend::contest::controller::reject_contest_handler)
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            )
            .service(
                web::scope("/api/sell/preferences")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(player_repo.clone())
                    .service(backend::sell::controller::get_preferences_handler)
                    .service(backend::sell::controller::put_preferences_handler),
            )
            .service(
                web::scope("/sell/preferences")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(player_repo.clone())
                    .service(backend::sell::controller::get_preferences_handler)
                    .service(backend::sell::controller::put_preferences_handler),
            )
            .service(
                web::scope("/api/sell/listings")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(web::PayloadConfig::default().limit(
                        backend::sell::image::max_upload_bytes(),
                    ))
                    .app_data(player_repo.clone())
                    .app_data(sell_prefs_repo.clone())
                    .app_data(redis_data.clone())
                    .service(backend::sell::controller::create_listing_handler)
                    .service(backend::sell::controller::list_listings_handler)
                    .service(backend::sell::controller::get_listing_handler)
                    .service(backend::sell::controller::upload_photo_handler)
                    .service(backend::sell::controller::get_photo_detail_handler)
                    .service(backend::sell::controller::get_photo_handler)
                    .service(backend::sell::controller::delete_photo_handler)
                    .service(backend::sell::controller::approve_checkpoint_handler)
                    .service(backend::sell::controller::extract_handler)
                    .service(backend::sell::controller::update_draft_handler)
                    .service(backend::sell::controller::bgg_match_handler)
                    .service(backend::sell::controller::export_listing_handler)
                    .service(backend::sell::controller::automate_handler)
                    .service(backend::sell::controller::automate_job_status_handler)
                    .service(backend::sell::controller::automation_result_handler)
                    .service(backend::sell::controller::cancel_listing_handler),
            )
            .service(
                web::scope("/sell/listings")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(redis_data.get_ref().clone()),
                    })
                    .app_data(web::PayloadConfig::default().limit(
                        backend::sell::image::max_upload_bytes(),
                    ))
                    .app_data(player_repo.clone())
                    .app_data(sell_prefs_repo.clone())
                    .app_data(redis_data.clone())
                    .service(backend::sell::controller::create_listing_handler)
                    .service(backend::sell::controller::list_listings_handler)
                    .service(backend::sell::controller::get_listing_handler)
                    .service(backend::sell::controller::upload_photo_handler)
                    .service(backend::sell::controller::get_photo_detail_handler)
                    .service(backend::sell::controller::get_photo_handler)
                    .service(backend::sell::controller::delete_photo_handler)
                    .service(backend::sell::controller::approve_checkpoint_handler)
                    .service(backend::sell::controller::extract_handler)
                    .service(backend::sell::controller::update_draft_handler)
                    .service(backend::sell::controller::bgg_match_handler)
                    .service(backend::sell::controller::export_listing_handler)
                    .service(backend::sell::controller::automate_handler)
                    .service(backend::sell::controller::automate_job_status_handler)
                    .service(backend::sell::controller::automation_result_handler)
                    .service(backend::sell::controller::cancel_listing_handler),
            )
            .configure(|cfg| {
                log::debug!("Registering /api/analytics routes");
                backend::analytics::controller::configure_routes(
                    cfg,
                    db.clone(),
                    config.database.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "/api",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /analytics routes (Trunk proxy)");
                backend::analytics::controller::configure_routes(
                    cfg,
                    db.clone(),
                    config.database.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /api/client routes");
                backend::client_analytics::controller::configure_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "/api",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /client routes (Trunk proxy)");
                backend::client_analytics::controller::configure_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering enhanced analytics routes at /api");
                backend::client_analytics::controller::configure_enhanced_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "/api",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering enhanced analytics routes (Trunk proxy)");
                backend::client_analytics::controller::configure_enhanced_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /api/ratings routes");
                backend::ratings::controller::RatingsController::configure_routes(
                    cfg,
                    db.clone(),
                    ratings_scheduler.clone(),
                    redis_client_for_ratings.clone(),
                    "/api",
                    player_repo_arc.clone(),
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /ratings routes (Trunk proxy)");
                backend::ratings::controller::RatingsController::configure_routes(
                    cfg,
                    db.clone(),
                    ratings_scheduler.clone(),
                    redis_client_for_ratings.clone(),
                    "",
                    player_repo_arc.clone(),
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /api/timezone routes");
                backend::timezone::controller::configure_routes(
                    cfg,
                    std::env::var("GOOGLEMAP_API_TIMEZONE_URL").unwrap_or_default(),
                    std::env::var("GOOGLE_LOCATION_API").unwrap_or_default(),
                    "/api",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /api/admin routes");
                backend::admin::controller::configure_routes(
                    cfg,
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "/api",
                    player_repo_arc.clone(),
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /timezone routes (Trunk proxy)");
                backend::timezone::controller::configure_routes(
                    cfg,
                    std::env::var("GOOGLEMAP_API_TIMEZONE_URL").unwrap_or_default(),
                    std::env::var("GOOGLE_LOCATION_API").unwrap_or_default(),
                    "",
                );
            })
            .configure(|cfg| {
                log::debug!("Registering /admin routes (Trunk proxy)");
                backend::admin::controller::configure_routes(
                    cfg,
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                    "",
                    player_repo_arc.clone(),
                );
            })
    })
    .workers(config.server.workers)
    .bind((config.server.host.as_str(), config.server.port))?
    .run();

    let server_handle = server.handle();

    tokio::select! {
        res = server => res?,
        _ = shutdown_signal() => {
            ratings_scheduler_shutdown.stop();
            sell_cleanup_scheduler_shutdown.stop();
            server_handle.stop(true).await;
        }
    }

    Ok(())
}
