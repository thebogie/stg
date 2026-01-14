//! Export production ArangoDB data for test data seeding
//!
//! This script exports all collections from the production ArangoDB database
//! to a JSON file that can later be sanitized and used in tests.

use anyhow::{Context, Result};
use arangors::{AqlQuery, Connection, Database};
use chrono::Utc;
use clap::Parser;
use log::{info, warn};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ArangoDB connection URL (e.g., http://localhost:8529)
    #[arg(short, long, env = "ARANGO_URL")]
    arango_url: String,

    /// Database name
    #[arg(short, long, env = "ARANGO_DB", default_value = "_system")]
    database: String,

    /// ArangoDB username
    #[arg(short, long, env = "ARANGO_USERNAME", default_value = "root")]
    username: String,

    /// ArangoDB password
    #[arg(short, long, env = "ARANGO_PASSWORD")]
    password: String,

    /// Output file path
    #[arg(short, long, default_value = "dump.json")]
    output: PathBuf,

    /// Collections to export (default: all)
    #[arg(short, long)]
    collections: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("Connecting to ArangoDB at {}", args.arango_url);
    // Use basic auth with provided credentials
    let conn = Connection::establish_basic_auth(&args.arango_url, &args.username, &args.password)
        .await
        .context("Failed to connect to ArangoDB with provided credentials")?;

    let db: Database<_> = conn
        .db(&args.database)
        .await
        .context("Failed to access database")?;

    info!("Fetching collections from database {}", args.database);

    // List collections using AQL query
    use arangors::AqlQuery;
    let list_query = AqlQuery::builder()
        .query("FOR c IN COLLECTIONS FILTER !STARTS_WITH(c.name, '_') RETURN c.name")
        .build();

    let collections: Vec<String> = db
        .aql_query(list_query)
        .await
        .context("Failed to list collections")?;

    let collections_to_export: Vec<String> = if let Some(specified) = args.collections {
        specified
    } else {
        collections
    };

    info!("Exporting {} collections", collections_to_export.len());

    let mut dump = json!({
        "database": args.database,
        "exported_at": Utc::now().to_rfc3339(),
        "collections": {}
    });

    for collection_name in &collections_to_export {
        info!("Exporting collection: {}", collection_name);

        match export_collection(&db, collection_name).await {
            Ok(documents) => {
                let count = documents.len();
                info!("Exported {} documents from {}", count, collection_name);
                dump["collections"][collection_name] = json!(documents);
            }
            Err(e) => {
                warn!("Failed to export collection {}: {}", collection_name, e);
            }
        }
    }

    info!("Writing dump to {}", args.output.display());
    let mut file = File::create(&args.output).context("Failed to create output file")?;

    serde_json::to_writer_pretty(&mut file, &dump).context("Failed to write JSON to file")?;

    file.flush()?;

    info!("✅ Export completed successfully!");
    info!("📁 Output file: {}", args.output.display());
    info!("⚠️  Remember to sanitize PII before using in tests!");

    Ok(())
}

async fn export_collection(
    db: &Database<arangors::client::reqwest::ReqwestClient>,
    collection_name: &str,
) -> Result<Vec<Value>> {
    let query_str = format!("FOR doc IN {} RETURN doc", collection_name);
    let query = AqlQuery::builder().query(&query_str).build();

    let result: Vec<Value> = db
        .aql_query(query)
        .await
        .with_context(|| format!("Failed to query collection {}", collection_name))?;

    Ok(result)
}
