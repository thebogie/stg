use anyhow::Result;
use arangors::Connection;
use clap::Parser;
use log::{error, info, warn};

mod place_id_migration;

#[derive(Parser, Debug)]
#[command(name = "migrate-place-ids")]
#[command(about = "Migrate venue place_ids using Google Places API")]
struct Args {
    /// ArangoDB endpoint URL
    #[arg(long, env = "ARANGO_URL", default_value = "http://localhost:8529")]
    arango_url: String,
    
    /// ArangoDB database name
    #[arg(long, env = "ARANGO_DB", default_value = "test")]
    arango_db: String,
    
    /// ArangoDB username
    #[arg(long, env = "ARANGO_USERNAME", default_value = "test")]
    arango_username: String,
    
    /// ArangoDB password
    #[arg(long, env = "ARANGO_PASSWORD", default_value = "test")]
    arango_password: String,
    
    /// Google Places API URL
    #[arg(long, env = "GOOGLEMAP_API_URL", default_value = "https://maps.googleapis.com/maps/api")]
    google_api_url: String,
    
    /// Google Places API key
    #[arg(long, env = "GOOGLE_LOCATION_API")]
    google_api_key: String,
    
    /// Dry run - don't actually update the database
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    let args = Args::parse();
    
    info!("🚀 Starting venue place_id migration");
    info!("📊 ArangoDB: {} (database: {})", args.arango_url, args.arango_db);
    info!("🌐 Google Places API: {}", args.google_api_url);
    
    if args.dry_run {
        warn!("⚠️  DRY RUN MODE - No changes will be made to the database");
    }
    
    // Connect to ArangoDB
    let conn = Connection::establish_basic_auth(
        &args.arango_url,
        &args.arango_username,
        &args.arango_password,
    ).await?;
    
    let db = conn.db(&args.arango_db).await?;
    info!("✅ Connected to ArangoDB database: {}", args.arango_db);
    
    // Run the migration
    match place_id_migration::migrate_venues_place_id(
        &db,
        &args.google_api_url,
        &args.google_api_key,
    ).await {
        Ok(_) => {
            info!("✅ Place ID migration completed successfully");
        },
        Err(e) => {
            error!("❌ Place ID migration failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
