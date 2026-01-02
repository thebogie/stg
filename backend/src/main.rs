use actix_web::{App, HttpServer, web};
use backend::player::session::RedisSessionStore;
use backend::third_party::BGGService;
use backend::config::BGGConfig;
use log::error;
use arangors::client::reqwest::ReqwestClient;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging with debug level for better visibility
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    // Load configuration from environment variables
    let config = match backend::config::Config::load() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    // Initialize Redis client
    let redis_client = match redis::Client::open(config.redis.url.clone()) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Redis client: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()));
        }
    };
    let redis_data = web::Data::new(redis_client.clone());
    let session_store = web::Data::new(RedisSessionStore { client: redis_client.clone() });
    let redis_client_for_ratings = redis_client.clone();

    // Initialize ArangoDB connection with root credentials
    let conn = match arangors::Connection::establish_basic_auth(
        &config.database.url,
        &config.database.root_username,
        &config.database.root_password
    ).await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to connect to ArangoDB: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()));
        }
    };

    let db = match conn.db(&config.database.name).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to get ArangoDB database: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()));
        }
    };

    let player_repo = web::Data::new(backend::player::repository::PlayerRepositoryImpl { db: db.clone() });

    // Initialize venue repository with Google Places API if configured
    let google_config = if let Some(api_key) = &config.google.location_api_key {
        log::info!("Google Places API configured with URL: {}", config.google.api_url);
        Some((config.google.api_url.clone(), api_key.clone()))
    } else {
        log::warn!("Google Places API not configured - no API key provided");
        None
    };
    let venue_repo = web::Data::new(backend::venue::repository::VenueRepositoryImpl::new(db.clone(), google_config.clone()));

    // Initialize game repository with BGG service
    let bgg_service = BGGService::new_with_config(&BGGConfig {
        api_url: config.bgg.api_url.clone(),
        api_token: config.bgg.api_token.clone(),
    });
    log::info!("BGG API configured with URL: {}", config.bgg.api_url);
    if config.bgg.api_token.is_some() {
        log::info!("BGG API token configured (Bearer authentication enabled)");
    } else {
        log::warn!("BGG API token not configured - requests will be unauthenticated");
    }

    let game_repo = web::Data::new(backend::game::repository::GameRepositoryImpl::new_with_bgg(db.clone(), bgg_service));

    // Initialize contest repository
    let contest_repo = web::Data::new(backend::contest::repository::ContestRepositoryImpl::new_with_google_config(db.clone(), google_config));

    // Initialize client analytics components
    let client_analytics_repo = backend::client_analytics::repository::ClientAnalyticsRepositoryImpl::<ReqwestClient>::new(db.clone());
    let client_analytics_usecase = backend::client_analytics::usecase::ClientAnalyticsUseCaseImpl::<_, ReqwestClient>::new(client_analytics_repo);
    let client_analytics_controller = web::Data::new(
        backend::client_analytics::controller::ClientAnalyticsController::new(
            client_analytics_usecase,
            db.clone(),
        )
    );

    // Initialize ratings scheduler
    let ratings_repo = backend::ratings::repository::RatingsRepository::new(db.clone());
    let ratings_usecase = backend::ratings::usecase::RatingsUsecase::new(ratings_repo);
    let mut ratings_scheduler = backend::ratings::scheduler::RatingsScheduler::new(ratings_usecase.clone());
    
    // Start the ratings scheduler in the background
    if let Err(e) = ratings_scheduler.start().await {
        log::error!("Failed to start ratings scheduler: {}", e);
    } else {
        log::info!("Glicko2 ratings scheduler started successfully");
    }

    // Analytics components will be initialized in the route configuration

    // Start HTTP server
    log::info!("Starting server on {}:{}", config.server.host, config.server.port);

    HttpServer::new(move || {
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(redis_data.clone())
            .app_data(player_repo.clone())
            .app_data(venue_repo.clone())
            .app_data(game_repo.clone())
            .app_data(contest_repo.clone())
            .app_data(session_store.clone())
            .service(backend::health::health_check)
            .service(backend::health::detailed_health_check)
            .service(backend::health::scheduler_health_check)
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
                    .service(backend::player::controller::logout_handler_prod)
                    .service(backend::player::controller::search_players_handler)
                    .service(backend::player::controller::search_players_db_handler)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware { redis: std::sync::Arc::new(redis_data.get_ref().clone()) })
                            .service(backend::player::controller::me_handler_prod)
                            .service(backend::player::controller::update_email_handler_prod)
                            .service(backend::player::controller::update_handle_handler_prod)
                            .service(backend::player::controller::update_password_handler_prod)
                    )
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware { 
                        redis: std::sync::Arc::new(redis_data.get_ref().clone())
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::search_venues_handler)
                    .service(backend::venue::controller::search_venues_db_handler)
                    .service(backend::venue::controller::search_venues_create_handler)
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler)
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware { 
                        redis: std::sync::Arc::new(redis_data.get_ref().clone())
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::search_games_handler)
                    .service(backend::game::controller::search_games_db_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler)
            )
            
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware { redis: std::sync::Arc::new(redis_data.get_ref().clone()) })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::get_player_game_contests_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler)
            )
            .configure(|cfg| {
                log::debug!("Registering /api/analytics routes");
                backend::analytics::controller::configure_routes(cfg, db.clone(), config.database.clone(), std::sync::Arc::new(redis_data.get_ref().clone()));
            })
            .configure(|cfg| {
                log::debug!("Registering /api/client routes");
                backend::client_analytics::controller::configure_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                );
            })
            .configure(|cfg| {
                log::debug!("Registering enhanced analytics routes");
                backend::client_analytics::controller::configure_enhanced_routes(
                    cfg,
                    client_analytics_controller.clone(),
                    std::sync::Arc::new(redis_data.get_ref().clone()),
                );
            })

            .configure(|cfg| {
                backend::ratings::controller::RatingsController::configure_routes(cfg, db.clone(), ratings_scheduler.clone(), redis_client_for_ratings.clone());
            })
            .configure(|cfg| {
                log::debug!("Registering /api/timezone routes");
                backend::timezone::controller::configure_routes(
                    cfg, 
                    std::env::var("GOOGLEMAP_API_TIMEZONE_URL").unwrap_or_default(),
                    std::env::var("GOOGLE_LOCATION_API").unwrap_or_default()
                );
            })
    })
    .bind((config.server.host.as_str(), config.server.port))?
    .run()
    .await
} 
